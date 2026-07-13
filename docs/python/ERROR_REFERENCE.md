# Structured Error Reference

This guide covers the error types, handling patterns, and pipeline error
conditions in the eggsec-python bindings.

## OperationError Structure

Every `OperationResult` that fails contains an `OperationError` DTO:

```python
result = engine.run_port_scan(request)

if result.status.name() == "Failed":
    error = result.error  # OperationError

    print(error.kind)           # "network", "timeout", "scope_denial", etc.
    print(error.message)        # Human-readable error message
    print(error.details)        # Optional additional context dict
    print(error.error_message)  # Compatibility alias for message
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `kind` | `str` | Error classification string |
| `message` | `str` | Human-readable error message |
| `details` | `dict?` | Optional additional context |
| `error_message` | `str` | Compatibility alias; same as `message` |

## Error Types and Their Meanings

### Python Exception Hierarchy

```
EggsecError (base)
  +-- ConfigError
  +-- ScopeError
  +-- EnforcementError
  +-- NetworkError
  +-- ScanError
  +-- TimeoutError
  +-- FeatureUnavailableError
  +-- SerializationError
  +-- InternalError
  +-- CancellationError
```

### Error Kind to Exception Mapping

| `error.kind` | Python Exception | Description |
|-------------|-----------------|-------------|
| `validation` | `ConfigError` | Invalid configuration or request parameters |
| `configuration` | `ConfigError` | Configuration file errors |
| `scope_denial` | `ScopeError` | Target not in authorized scope |
| `policy_denial` | `EnforcementError` | Enforcement policy denied the operation |
| `capability_unavailable` | `EnforcementError` | Required capability not available |
| `privilege_missing` | `EnforcementError` | Insufficient privileges |
| `feature_unavailable` | `FeatureUnavailableError` | Feature not compiled in this build |
| `network` | `NetworkError` | Network connectivity failure |
| `daemon_transport` | `NetworkError` | Daemon transport error |
| `timeout` | `TimeoutError` | Operation exceeded time limit |
| `cancellation` | `CancellationError` | Operation was cancelled |
| `scan` | `ScanError` | Scan execution failure |
| `serialization` | `SerializationError` | JSON/parse error |
| `parsing` | `SerializationError` | Data parsing error |
| *(other)* | `InternalError` | Unexpected internal error |

## Error Handling in Sync Path

### Result-Based Handling

```python
from eggsec import Engine, Scope, PortScanRequest, EggsecError

scope = Scope.allow_hosts(["10.0.0.1"])
engine = Engine(scope)

try:
    result = engine.run_port_scan(PortScanRequest("10.0.0.1", ports="80"))
except EggsecError as e:
    # Raised for configuration/scope/policy errors
    print(f"Engine error: {e}")
else:
    if result.status.name() == "Completed":
        print(result.payload)
    elif result.status.name() == "Failed":
        error = result.error
        if error.kind == "timeout":
            print("Operation timed out")
        elif error.kind == "network":
            print(f"Network error: {error.message}")
        elif error.kind == "scope_denial":
            print(f"Scope violation: {error.message}")
        else:
            print(f"Error ({error.kind}): {error.message}")
```

### Catching Specific Exceptions

```python
from eggsec import (
    NetworkError, TimeoutError, ScopeError,
    EnforcementError, ScanError, ConfigError,
)

try:
    result = engine.run_port_scan(request)
except NetworkError as e:
    print(f"Network: {e}")
except TimeoutError as e:
    print(f"Timeout: {e}")
except ScopeError as e:
    print(f"Scope: {e}")
except EnforcementError as e:
    print(f"Policy: {e}")
except ScanError as e:
    print(f"Scan: {e}")
except ConfigError as e:
    print(f"Config: {e}")
```

## Error Handling in Async Path

The async path follows the same patterns:

```python
import asyncio
from eggsec import AsyncEngine, Scope, PortScanRequest, NetworkError

async def main():
    scope = Scope.allow_hosts(["10.0.0.1"])
    engine = AsyncEngine(scope)

    try:
        result = await engine.run_port_scan(PortScanRequest("10.0.0.1", ports="80"))
    except NetworkError as e:
        print(f"Async network error: {e}")
    else:
        if result.status.name() == "Failed":
            print(f"Error: {result.error.message}")

asyncio.run(main())
```

## Pipeline Errors

Pipeline-level errors are raised as `ValueError` or `PyRuntimeError`:

### Dependency Cycle Detection

```python
pipeline = Pipeline("cyclic")
pipeline.add_step("a", OperationRequest("recon", "example.com"), dependencies=["b"])
pipeline.add_step("b", OperationRequest("recon", "example.com"), dependencies=["a"])

try:
    pipeline.run(engine)
except ValueError as e:
    print(f"Dependency error: {e}")
    # "dependency_error: circular dependency detected among pipeline steps"
```

### Missing Step Reference

```python
pipeline = Pipeline("broken")
pipeline.add_step("a", OperationRequest("recon", "example.com"), dependencies=["nonexistent"])

try:
    pipeline.run(engine)
except ValueError as e:
    print(f"Dependency error: {e}")
    # "dependency_error: step 'a' references non-existent step 'nonexistent'"
```

### Checkpoint Incompatibility

When resuming from a checkpoint that no longer matches the pipeline:

```python
result = pipeline.run(engine)
# If checkpoint is incompatible, raises:
# ValueError: "checkpoint_incompatible: pipeline definition does not match"
# ValueError: "checkpoint_incompatible: target set does not match"
# ValueError: "checkpoint_incompatible: scope does not match"
```

### Checkpoint Missing Result

If a checkpoint references a completed step but the result is missing:

```python
# ValueError: "checkpoint_incompatible: missing result for completed step 'step1'"
```

## Event-Based Error Reporting

Pipeline events carry error information:

```python
result = pipeline.run(engine)

for event in result.events:
    if event.event_type == "pipeline.failure":
        print(f"Pipeline failed: {event.payload}")
    elif event.event_type == "step.failed":
        print(f"Step failed: {event.payload}")
    elif event.event_type == "cancellation.requested":
        print(f"Cancelled: {event.payload}")
```

## OperationError Serialization

`OperationError` is a versioned DTO that can be serialized:

```python
result = engine.run_port_scan(request)

if result.status.name() == "Failed":
    error = result.error

    # To dict
    d = error.to_dict()
    print(d)

    # To JSON
    j = error.to_json()
    print(j)
```

## Common Error Patterns

### Retryable vs Permanent Errors

```python
RETRYABLE_KINDS = {"timeout", "network"}

def should_retry(error):
    return error.kind in RETRYABLE_KINDS

result = engine.run_port_scan(request)
if result.status.name() == "Failed":
    if should_retry(result.error):
        # Retry the operation
        pass
    else:
        # Permanent failure
        pass
```

### Error Aggregation in Pipelines

```python
pipeline = Pipeline("multi", failure_policy=FailurePolicy.Continue)
pipeline.add_step("a", OperationRequest("recon", "example.com"))
pipeline.add_step("b", OperationRequest("tls-inspect", "example.com"))

result = pipeline.run(engine)

# Collect all errors
errors = []
for step in result.step_results:
    if not step.is_success() and step.result and step.result.error:
        errors.append({
            "step": step.step_name,
            "kind": step.result.error.kind,
            "message": step.result.error.message,
        })

if errors:
    print(f"Pipeline had {len(errors)} failures:")
    for err in errors:
        print(f"  {err['step']}: [{err['kind']}] {err['message']}")
```

## Engine Error Mapping (Internal)

The Rust `EggsecError` enum maps to Python exceptions as follows:

| Rust Variant | Python Exception |
|-------------|-----------------|
| `Config` | `ConfigError` |
| `InvalidTarget` | `EnforcementError` |
| `Network` | `NetworkError` |
| `RequestFailed` | `NetworkError` |
| `Timeout` | `TimeoutError` |
| `RateLimited` | `NetworkError` |
| `ScanFailed` | `ScanError` |
| `Payload` | `ScanError` |
| `Output` | `ScanError` |
| `Internal` | `InternalError` |
| `ScopeViolation` | `EnforcementError` |
| `Io` | `ScanError` |
| `HttpStatus` | `NetworkError` |
| `Http` | `NetworkError` |
| `Parse` | `SerializationError` |
| `Validation` | `ConfigError` |
| `AddressParse` | `NetworkError` |
| `Runtime` | `ScanError` |
| `Cancelled` | `ScanError` |
| `Proxy` | `ScanError` |
| `Recon` | `ScanError` |
| `LoadTest` | `ScanError` |
| `Fingerprint` | `ScanError` |
