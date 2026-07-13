# Retries and Failure Policy

This guide covers configuring retry behavior and failure policies for
pipeline steps.

## RetryPolicy

`RetryPolicy` controls how failed steps are retried before giving up.

```python
from eggsec import RetryPolicy

retry = RetryPolicy(
    max_attempts=3,              # Total attempts including the first (default: 1)
    retryable_errors=None,       # List of error kinds to retry (default: all)
    backoff_ms=1000,             # Initial backoff delay (default: 1000)
    max_delay_ms=30000,          # Maximum delay cap (default: 30000)
    jitter=True,                 # Add random jitter (default: True)
)
```

### Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_attempts` | `int` | `1` | Total attempts (1 = no retry) |
| `retryable_errors` | `list[str]?` | `None` (all errors) | Error kinds to retry. Empty/None means retry all. |
| `backoff_ms` | `int` | `1000` | Initial backoff delay in milliseconds |
| `max_delay_ms` | `int` | `30000` | Maximum delay cap in milliseconds |
| `jitter` | `bool` | `True` | Whether to add random jitter to delays |

### Retryable Error Kinds

Error kinds are extracted from `OperationError.kind` or `ExecutionStatus`:

| Error Kind | Description |
|------------|-------------|
| `timeout` | Operation timed out |
| `network` | Network connectivity issue |
| `scope_denial` | Target not in scope |
| `internal` | Unexpected internal error |
| `unknown` | Unclassified error |

When `retryable_errors` is empty or `None`, all errors are retried. When
explicitly set, only matching error kinds are retried.

### Example: Retry on Network Errors Only

```python
retry = RetryPolicy(
    max_attempts=3,
    retryable_errors=["network", "timeout"],
    backoff_ms=2000,
)
```

### Example: No Retry

```python
# Default — max_attempts=1 means no retry
retry = RetryPolicy()  # or RetryPolicy(max_attempts=1)
```

## Exponential Backoff with Jitter

The delay for each retry attempt is computed as:

```
base = backoff_ms * 2^min(attempt, 10)
capped = min(base, max_delay_ms)
delay = capped - jitter_range + random(0, 2 * jitter_range)
```

Where `jitter_range = capped / 4` when jitter is enabled.

### Delay Table (backoff_ms=1000, max_delay_ms=30000, jitter=True)

| Attempt | Base (ms) | Capped (ms) | Approximate Delay Range |
|---------|-----------|-------------|------------------------|
| 1 | 1000 | 1000 | 750 – 1250 |
| 2 | 2000 | 2000 | 1500 – 2500 |
| 3 | 4000 | 4000 | 3000 – 5000 |
| 4 | 8000 | 8000 | 6000 – 10000 |
| 5 | 16000 | 16000 | 12000 – 20000 |
| 6 | 32000 | 30000 | 22500 – 30000 |
| 7+ | 64000+ | 30000 | 22500 – 30000 |

### Without Jitter

When `jitter=False`, the delay is exactly `min(base, max_delay_ms)`:

| Attempt | Delay (ms) |
|---------|------------|
| 1 | 1000 |
| 2 | 2000 |
| 3 | 4000 |
| 4 | 8000 |
| 5 | 16000 |
| 6 | 30000 |

## FailurePolicy

`FailurePolicy` determines pipeline behavior when a step fails after
exhausting retries.

```python
from eggsec import FailurePolicy

# Stop on first failure (default)
policy = FailurePolicy.StopPipeline

# Continue regardless of failures
policy = FailurePolicy.Continue

# Continue but skip steps that depend on the failed step
policy = FailurePolicy.SkipDependents
```

### Variants

| Variant | Behavior |
|---------|----------|
| `StopPipeline` | Halt the entire pipeline on the first step failure |
| `Continue` | Execute all remaining steps regardless of failures |
| `SkipDependents` | Continue execution but skip steps that depend (directly or transitively) on the failed step |

### StopPipeline (Default)

```python
from eggsec import Pipeline, FailurePolicy, OperationRequest

pipeline = Pipeline("strict", failure_policy=FailurePolicy.StopPipeline)
pipeline.add_step("step-a", OperationRequest("recon", "example.com"))
pipeline.add_step("step-b", OperationRequest("tls-inspect", "example.com"),
                  dependencies=["step-a"])

# If step-a fails, step-b is never executed
# PipelineResult.status == Failed
```

### Continue

```python
pipeline = Pipeline("lenient", failure_policy=FailurePolicy.Continue)
pipeline.add_step("step-a", OperationRequest("recon", "example.com"))
pipeline.add_step("step-b", OperationRequest("tls-inspect", "example.com"))

# If step-a fails, step-b still executes
# PipelineResult.status == Failed (if any step failed)
# Both step results are available
```

### SkipDependents

```python
pipeline = Pipeline("smart", failure_policy=FailurePolicy.SkipDependents)
pipeline.add_step("recon", OperationRequest("recon", "example.com"))
pipeline.add_step("tls", OperationRequest("tls-inspect", "example.com"),
                  dependencies=["recon"])
pipeline.add_step("fingerprint", OperationRequest("fingerprint-services", "10.0.0.1"))

# If recon fails:
#   - tls is skipped (depends on recon)
#   - fingerprint still runs (no dependency on recon)
# PipelineResult.status == Failed
```

## Combined Retry + Failure Policy

When both `retry_policy` and `failure_policy` are set, retries execute first,
then the failure policy determines pipeline behavior:

```python
from eggsec import Pipeline, RetryPolicy, FailurePolicy, OperationRequest

pipeline = Pipeline(
    "resilient",
    retry_policy=RetryPolicy(
        max_attempts=3,
        retryable_errors=["network", "timeout"],
        backoff_ms=1000,
    ),
    failure_policy=FailurePolicy.SkipDependents,
)

pipeline.add_step("recon", OperationRequest("recon", "example.com"))
pipeline.add_step("tls", OperationRequest("tls-inspect", "example.com"),
                  dependencies=["recon"])
pipeline.add_step("tech", OperationRequest("tech-detect", "example.com"),
                  dependencies=["recon"])
```

Execution flow for each step:

1. Attempt 1: Execute the step
2. If failed and retryable: Wait backoff delay, then retry
3. Repeat up to `max_attempts`
4. If still failed: Apply `failure_policy`

### Global vs Per-Step Retry

The retry policy set on the `Pipeline` applies to all steps. Per-step retry
is not currently supported. To apply different retry behavior, use multiple
pipelines or set `max_attempts=1` on the pipeline and handle retries manually.

## Serialization

Both `RetryPolicy` and `FailurePolicy` support dict/JSON conversion:

```python
retry = RetryPolicy(max_attempts=3, backoff_ms=500)
print(retry.to_dict())
# {'max_attempts': 3, 'retryable_errors': [], 'backoff_ms': 500, 'max_delay_ms': 30000, 'jitter': True}

print(retry.to_json())
# '{"max_attempts":3,"retryable_errors":[],"backoff_ms":500,"max_delay_ms":30000,"jitter":true}'

policy = FailurePolicy.Continue
print(policy.to_dict())
# {'type': 'Continue', 'value': 1}
```
