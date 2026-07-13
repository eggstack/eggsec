# Cancellation and Timeout

Pipelines and engine operations support cooperative cancellation and
per-step timeout configuration.

## CancellationToken

`CancellationToken` enables cooperative cancellation from any thread:

```python
from eggsec import CancellationToken

token = CancellationToken()

# Check cancellation
print(token.is_cancelled())  # False

# Request cancellation with a reason
token.cancel("User requested abort")
print(token.is_cancelled())  # True
print(token.reason())        # "User requested abort"
```

### API

| Method | Description |
|--------|-------------|
| `CancellationToken()` | Create a new token |
| `cancel(reason=None)` | Request cancellation with optional reason |
| `is_cancelled()` | Check if cancellation has been requested |
| `reason()` | Get the cancellation reason (if any) |

### Serialization

```python
token = CancellationToken()
token.cancel("timeout")

print(token.to_dict())
# {'cancelled': True, 'reason': 'timeout'}

print(token.to_json())
# '{"cancelled":true,"reason":"timeout"}'
```

## Pipeline Cancellation

Attach a `CancellationToken` to a pipeline:

```python
from eggsec import Pipeline, CancellationToken, OperationRequest, Engine, Scope

scope = Scope.allow_hosts(["10.0.0.1"])
engine = Engine(scope)

pipeline = Pipeline("long-scan")
token = CancellationToken()
pipeline.set_cancel_token(token)

pipeline.add_step("recon", OperationRequest("recon", "example.com"))
pipeline.add_step("tls", OperationRequest("tls-inspect", "example.com"))

# Run in background or with a watchdog
result = pipeline.run(engine)
```

### Cancellation Checkpoints

Cancellation is checked **between dependency groups**, not mid-step. Once a
step starts, it runs to completion (or until its own timeout fires).

```
[Step A] -- check cancel -- [Step B] -- check cancel -- [Step C]
                                ^
                          Cancellation checked here
```

### Cancellation Behavior

When cancellation is detected:

1. The current group completes (steps already dispatched finish)
2. No new groups are started
3. `PipelineResult.status` becomes `Cancelled`
4. Partial results are preserved in `PipelineResult.step_results`
5. A `cancellation.requested` event is emitted

```python
import threading
from eggsec import Pipeline, CancellationToken, OperationRequest, Engine, Scope

token = CancellationToken()

def cancel_after_delay():
    import time
    time.sleep(5)
    token.cancel("Timeout exceeded")

# Start cancellation watcher
t = threading.Thread(target=cancel_after_delay, daemon=True)
t.start()

pipeline = Pipeline("watched-scan")
pipeline.set_cancel_token(token)
pipeline.add_step("recon", OperationRequest("recon", "example.com"))
pipeline.add_step("tls", OperationRequest("tls-inspect", "example.com"))

result = pipeline.run(engine)

if result.status.name() == "Cancelled":
    print(f"Pipeline cancelled: {result.status.reason}")
    # Partial results still available
    for step in result.step_results:
        print(f"  {step.step_name}: {step.status.name()}")
```

### Async Pipeline Cancellation

```python
import asyncio
from eggsec import AsyncPipeline, CancellationToken, AsyncEngine, Scope, OperationRequest

async def main():
    scope = Scope.allow_hosts(["10.0.0.1"])
    engine = AsyncEngine(scope)

    token = CancellationToken()
    pipeline = AsyncPipeline("async-scan")
    pipeline.set_cancel_token(token)
    pipeline.add_step("recon", OperationRequest("recon", "example.com"))

    # Cancel after 10 seconds
    async def cancel_later():
        await asyncio.sleep(10)
        token.cancel("Async timeout")

    asyncio.create_task(cancel_later())

    result = await pipeline.run(engine)
    print(f"Status: {result.status.name()}")

asyncio.run(main())
```

## Per-Step Timeout

Each step can have its own timeout:

```python
from eggsec import Pipeline, OperationRequest

pipeline = Pipeline("timed-scan")

# Step with 5-second timeout
pipeline.add_step("quick-recon", OperationRequest("recon", "example.com"),
                  timeout_ms=5000)

# Step with 2-minute timeout
pipeline.add_step("deep-scan", OperationRequest("scan-endpoints", "https://example.com"),
                  timeout_ms=120000)

# Step with no timeout (uses engine default)
pipeline.add_step("fingerprint", OperationRequest("fingerprint-services", "10.0.0.1"))
```

### Timeout Behavior

When a step times out:

1. The step status becomes `Timeout` with `elapsed_ms`
2. The step is marked as failed
3. The failure policy determines whether the pipeline continues
4. If retry is configured, the timeout error is retried per the retry policy

```python
from eggsec import Pipeline, RetryPolicy, FailurePolicy, OperationRequest

pipeline = Pipeline(
    "timeout-demo",
    retry_policy=RetryPolicy(max_attempts=2, retryable_errors=["timeout"]),
    failure_policy=FailurePolicy.SkipDependents,
)

pipeline.add_step("reliable", OperationRequest("recon", "example.com"),
                  timeout_ms=5000)
pipeline.add_step("depends", OperationRequest("tls-inspect", "example.com"),
                  dependencies=["reliable"])
```

If `reliable` times out, it is retried once. If it still fails, `depends`
is skipped (per `SkipDependents`).

## Engine-Level Timeout

The engine itself can have a global timeout that applies to all operations:

```python
from eggsec import Engine, Scope

# Engine with 30-second default timeout
engine = Engine(Scope.allow_hosts(["10.0.0.1"]), timeout_ms=30000)

# Engine with no global timeout
engine = Engine(Scope.allow_hosts(["10.0.0.1"]))
```

Per-step timeouts override the engine-level timeout for that specific step.

## Cancellation Event Emissions

Cancellation emits typed events through the pipeline event stream:

| Event | When |
|-------|------|
| `cancellation.requested` | When `token.cancel()` is called |
| `pipeline.failure` | When pipeline stops due to cancellation |

Events are delivered in monotonic sequence order through the
`PipelineResult.events` list.

```python
result = pipeline.run(engine)

for event in result.events:
    print(f"{event.event_type}: {event.payload}")
    # cancellation.requested: {"reason": "User requested abort"}
    # pipeline.failure: {"kind": "step_failure", "message": "...", "recoverable": false}
```

## Behavior on Cancellation

### Partial Results

When a pipeline is cancelled, `PipelineResult.step_results` contains all
step results collected before cancellation:

```python
result = pipeline.run(engine)

if result.status.name() == "Cancelled":
    completed = [s for s in result.step_results if s.is_success()]
    failed = [s for s in result.step_results if not s.is_success()]

    print(f"Completed: {len(completed)}")
    print(f"Incomplete: {len(failed)}")

    # Access partial results
    for step in completed:
        if step.result:
            print(f"  {step.step_name}: {step.result.payload}")
```

### Cleanup

The pipeline does not perform automatic cleanup of partial state. If
checkpointing is enabled, the checkpoint is **not** deleted on cancellation
(it remains for potential resume). On successful completion, the checkpoint
is deleted.

### Thread Safety

`CancellationToken` is thread-safe (uses `AtomicBool` and `Mutex` internally).
You can call `cancel()` from any thread without synchronization.

## Complete Example: Watchdog Pattern

```python
import threading
from eggsec import (
    Pipeline, CancellationToken, RetryPolicy, FailurePolicy,
    Engine, Scope, OperationRequest, create_checkpoint_store,
)

def run_with_watchdog(timeout_seconds=300):
    scope = Scope.allow_hosts(["10.0.0.1"])
    engine = Engine(scope)

    token = CancellationToken()
    store = create_checkpoint_store("/tmp/checkpoint.json")

    pipeline = Pipeline(
        "watched-assessment",
        retry_policy=RetryPolicy(max_attempts=2, backoff_ms=2000),
        failure_policy=FailurePolicy.SkipDependents,
        max_concurrency=2,
    )
    pipeline.set_cancel_token(token)
    pipeline.set_checkpoint_store(store)

    pipeline.add_step("recon", OperationRequest("recon", "example.com"))
    pipeline.add_step("tls", OperationRequest("tls-inspect", "example.com"),
                      parallel_group="passive")
    pipeline.add_step("tech", OperationRequest("tech-detect", "example.com"),
                      parallel_group="passive")
    pipeline.add_step("fingerprint", OperationRequest("fingerprint-services", "10.0.0.1"),
                      dependencies=["recon"])

    # Watchdog thread
    def watchdog():
        import time
        time.sleep(timeout_seconds)
        token.cancel(f"Watchdog timeout ({timeout_seconds}s)")

    t = threading.Thread(target=watchdog, daemon=True)
    t.start()

    result = pipeline.run(engine)

    if result.status.name() == "Cancelled":
        print(f"Pipeline cancelled after {timeout_seconds}s")
    elif result.is_success():
        print("Pipeline completed successfully")
    else:
        print("Pipeline failed")

    return result
```
