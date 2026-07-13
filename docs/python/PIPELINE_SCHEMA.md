# Pipeline Schema

Pipelines chain multiple operations with dependency ordering, parallel
execution, retry, failure policies, and checkpoint resume.

## Pipeline Structure

```python
from eggsec import Pipeline, RetryPolicy, FailurePolicy

pipeline = Pipeline(
    "assessment-pipeline",
    stop_on_failure=True,          # Stop on first failure (default: True)
    retry_policy=RetryPolicy(      # Optional global retry policy
        max_attempts=3,
        backoff_ms=1000,
        max_delay_ms=30000,
        jitter=True,
    ),
    failure_policy=FailurePolicy.StopPipeline,  # Default
    max_concurrency=4,             # Max parallel steps (default: 1)
)
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `str` | *(required)* | Human-readable pipeline name |
| `stop_on_failure` | `bool` | `True` | Legacy flag; prefer `failure_policy` |
| `retry_policy` | `RetryPolicy?` | `None` | Global retry policy applied to all steps |
| `failure_policy` | `FailurePolicy?` | `StopPipeline` | What happens when a step fails |
| `max_concurrency` | `int` | `1` | Maximum number of steps to execute in parallel |

## PipelineStep Fields

```python
from eggsec import PipelineStep, OperationRequest

step = PipelineStep(
    name="port-scan",
    request=OperationRequest("scan-ports", "10.0.0.1", metadata={"ports": "22,80,443"}),
    condition=None,             # Optional conditional execution
    dependencies=None,          # Optional list of step names that must complete first
    timeout_ms=30000,           # Optional per-step timeout
    parallel_group=None,        # Optional parallel execution group name
)
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | `str` | *(required)* | Unique step identifier |
| `request` | `OperationRequest` | *(required)* | The operation to execute |
| `condition` | `str?` | `None` | Expression evaluated before execution |
| `dependencies` | `list[str]?` | `[]` | Step names that must succeed first |
| `timeout_ms` | `int?` | `None` | Per-step timeout override |
| `parallel_group` | `str?` | `None` | Group name for concurrent execution |

### Fluent Step Addition

Steps can be added via `add_step()` which returns `self` for chaining:

```python
pipeline = Pipeline("recon")
pipeline.add_step("dns", OperationRequest("recon", "example.com")) \
        .add_step("tls", OperationRequest("tls-inspect", "example.com")) \
        .add_step("tech", OperationRequest("tech-detect", "example.com"))
```

## Adding Steps

```python
# Simple addition
pipeline.add_step("dns", OperationRequest("recon", "example.com"))

# With dependencies
pipeline.add_step(
    "fingerpint",
    OperationRequest("fingerprint-services", "10.0.0.1"),
    dependencies=["port-scan"],
)

# With condition
pipeline.add_step(
    "waf-validate",
    OperationRequest("waf-validate", "https://example.com"),
    condition="findings:port-scan > 0",
)

# With timeout
pipeline.add_step(
    "slow-scan",
    OperationRequest("scan-endpoints", "https://example.com"),
    timeout_ms=120000,
)

# With parallel group
pipeline.add_step("dns-recon", OperationRequest("recon", "example.com"),
                  parallel_group="passive-recon")
pipeline.add_step("tls-inspect", OperationRequest("tls-inspect", "example.com"),
                  parallel_group="passive-recon")
```

## OutputRef Model

`OutputRef` references typed output from a completed step for use in
conditional expressions:

```python
from eggsec import OutputRef

ref = OutputRef("port-scan", "open_ports")
print(ref.step_id)  # "port-scan"
print(ref.path)     # "open_ports"
```

| Field | Type | Description |
|-------|------|-------------|
| `step_id` | `str` | Name of the source step |
| `path` | `str` | JSON path into the step result |

OutputRef is used in conditional expressions to reference previous step
results. See **Conditional Execution Syntax** below.

## Dependency Resolution and Topological Sort

Dependencies form a directed acyclic graph (DAG). The pipeline engine:

1. **Validates** all dependency references exist (raises `ValueError` otherwise)
2. **Detects cycles** via Kahn's algorithm (raises `ValueError` on cycles)
3. **Sorts** steps topologically, ensuring each step runs only after its
   dependencies complete successfully
4. **Groups** steps by `parallel_group` for concurrent execution

```
port-scan
    |
    v
fingerprint  ──────>  waf-validate
    |
    v
consolidated-recon
```

If a dependency fails, all downstream steps are skipped automatically.

## Parallel Groups

Steps with the same `parallel_group` name execute concurrently, bounded by
`max_concurrency`:

```python
from eggsec import Pipeline, OperationRequest

pipeline = Pipeline("recon", max_concurrency=4)

# These three steps share the "recon" group and run in parallel
pipeline.add_step("dns", OperationRequest("recon", "example.com"),
                  parallel_group="recon")
pipeline.add_step("tls", OperationRequest("tls-inspect", "example.com"),
                  parallel_group="recon")
pipeline.add_step("tech", OperationRequest("tech-detect", "example.com"),
                  parallel_group="recon")

# This step waits for all "recon" steps to finish
pipeline.add_step("fingerprint", OperationRequest("fingerprint-services", "10.0.0.1"),
                  dependencies=["dns", "tls", "tech"])
```

### Execution Order Rules

1. Steps without `parallel_group` or `dependencies` execute in insertion order
2. Steps in the same `parallel_group` execute concurrently
3. Steps with `dependencies` wait for all named dependencies to succeed
4. `max_concurrency` limits how many steps in a parallel group run at once
5. The GIL limits true concurrency in sync pipelines; use `AsyncPipeline` for
   I/O-bound parallelism

## Conditional Execution Syntax

Conditions are string expressions evaluated before a step runs. The step is
skipped if the condition evaluates to `false`.

### Comparison Operators

| Operator | Meaning | Example |
|----------|---------|---------|
| `==` | Equal | `status:step_id == success` |
| `!=` | Not equal | `status:step_id != success` |
| `>` | Greater than | `findings:step_id > 0` |
| `<` | Less than | `findings:step_id < 10` |
| `>=` | Greater or equal | `findings:step_id >= 1` |
| `<=` | Less or equal | `findings:step_id <= 5` |

### Reference Prefixes

| Prefix | Meaning |
|--------|---------|
| `status:step_id` | Execution status of the named step (`success`, `failed`, `cancelled`, `skipped`) |
| `findings:step_id` | Number of findings in the named step's result |

### Examples

```python
# Only run if port-scan found open ports
pipeline.add_step("fingerprint", request,
                  condition="findings:port-scan > 0")

# Only run if a previous step succeeded
pipeline.add_step("deep-scan", request,
                  dependencies=["recon"],
                  condition="status:recon == success")

# Skip if too many findings (quality gate)
pipeline.add_step("report", request,
                  condition="findings:deep-scan < 100")
```

## Running Pipelines

### Sync

```python
from eggsec import Pipeline, Engine, Scope, OperationRequest

scope = Scope.allow_hosts(["10.0.0.1"])
engine = Engine(scope)

pipeline = Pipeline("my-scan")
pipeline.add_step("recon", OperationRequest("recon", "example.com"))
pipeline.add_step("tls", OperationRequest("tls-inspect", "example.com"))

result = pipeline.run(engine)

print(result.name)              # "my-scan"
print(result.status.name())     # "Completed" or "Failed"
print(result.total_duration_ms) # Total pipeline duration
print(result.retried_steps)     # Number of steps that were retried
```

### Async

```python
import asyncio
from eggsec import AsyncPipeline, AsyncEngine, Scope, OperationRequest

async def main():
    scope = Scope.allow_hosts(["10.0.0.1"])
    engine = AsyncEngine(scope)

    pipeline = AsyncPipeline("my-scan")
    pipeline.add_step("recon", OperationRequest("recon", "example.com"))
    pipeline.add_step("tls", OperationRequest("tls-inspect", "example.com"))

    result = await pipeline.run(engine)
    print(result.status.name())

asyncio.run(main())
```

## StepResult and PipelineResult

### StepResult

| Field | Type | Description |
|-------|------|-------------|
| `step_name` | `str` | Name of the step |
| `status` | `ExecutionStatus` | `Completed`, `Failed`, `Cancelled`, or `Timeout` |
| `result` | `OperationResult?` | The operation result (if successful or partially) |
| `duration_ms` | `int` | Step execution duration in milliseconds |
| `attempt` | `int` | Which attempt this was (1 = no retry) |

### PipelineResult

| Field | Type | Description |
|-------|------|-------------|
| `name` | `str` | Pipeline name |
| `status` | `ExecutionStatus` | Overall pipeline status |
| `step_results` | `list[StepResult]` | Results for each executed step |
| `total_duration_ms` | `int` | Total pipeline duration |
| `events` | `list[EventEnvelope]` | All emitted events |
| `retried_steps` | `int` | Number of steps that were retried |

### Convenience Methods

```python
result = pipeline.run(engine)

# Check overall success
if result.is_success():
    print("All steps succeeded")

# Iterate step results
for step in result.step_results:
    print(f"{step.step_name}: {step.status.name()} ({step.duration_ms}ms)")

# Convert to dict/JSON
d = result.to_dict()
j = result.to_json()
```
