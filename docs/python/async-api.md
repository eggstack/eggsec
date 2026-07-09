# Async API Reference

The async API allows non-blocking scan operations. The GIL is released during execution, and results are delivered via a polling protocol (`__next__`).

## `AsyncClient`

Async counterpart to `Client`. Same constructor parameters:

```python
import eggsec

scope = eggsec.Scope.allow_hosts(["127.0.0.1"])
client = eggsec.AsyncClient(scope, concurrency=100, timeout_ms=5000)
```

### Methods

#### `client.scan_ports(target, ports, *, concurrency=None, timeout_ms=None) -> PyFuture`

#### `client.scan_endpoints(config) -> PyFuture`

#### `client.fingerprint_services(host, ports, *, concurrency=None, timeout_ms=None) -> PyFuture`

#### `client.validate_waf(url, *, bypass=False, test_type=None) -> PyFuture`

#### `client.fuzz_http(url, payload_type="all", *, method="GET", param=None, concurrency=10, timeout=30) -> PyFuture`

#### `client.load_test_http(url, total_requests, concurrency, timeout_secs, *, method="GET") -> PyFuture`

All return a `PyFuture` that can be polled or awaited via the async context manager.

### Async Context Manager

```python
async with eggsec.AsyncClient(scope) as client:
    future = client.scan_ports("127.0.0.1", [80, 443])
    # In an async context, await the future
    result = await future
```

## `PyFuture`

A pollable future wrapping an async Rust operation. Supports the iterator protocol for polling:

```python
future = client.scan_ports("127.0.0.1", [80])

# Poll until complete
for result in future:
    if result is not None:
        print(result)
```

`PyFuture.__next__()` returns:
- `None` — operation still running
- Raises `StopIteration(result)` — operation completed successfully

## Convenience Functions

Top-level async functions that create an ephemeral `AsyncClient`:

```python
import eggsec

scope = eggsec.Scope.allow_hosts(["127.0.0.1"])

# Async port scan
future = eggsec.async_scan_ports("127.0.0.1", [80, 443], scope)

# Async endpoint scan
config = eggsec.EndpointScanConfig("http://127.0.0.1", ["/", "/admin"])
future = eggsec.async_scan_endpoints(config, scope)

# Async service fingerprinting
future = eggsec.async_fingerprint_services("127.0.0.1", [22, 80, 443], scope)

# Async WAF validation
future = eggsec.async_validate_waf("https://example.com", scope)

# Async HTTP fuzzing
future = eggsec.async_fuzz_http("https://example.com", scope)

# Async load testing
future = eggsec.async_load_test_http("https://example.com", 100, 10, 30, scope)
```

## Thread-Based Async Bridge

The async bridge uses a dedicated background thread with its own Tokio runtime for each operation. The Rust future is polled on this thread, and the result is sent back to Python via a channel. This avoids `pyo3-async-runtimes` compatibility issues while still releasing the GIL.

```python
import threading

def scan_worker():
    future = eggsec.async_scan_ports("10.0.0.1", [22, 80], scope)
    for result in future:
        if result is not None:
            print(f"Result: {result}")

thread = threading.Thread(target=scan_worker)
thread.start()
thread.join()
```

## Error Handling

All async operations raise the same exceptions as their sync counterparts:

```python
try:
    for result in eggsec.async_scan_ports("10.0.0.1", [80], scope):
        if result is not None:
            print(result)
except eggsec.EnforcementError as e:
    print(f"Scope violation: {e}")
except eggsec.NetworkError as e:
    print(f"Network error: {e}")
except eggsec.TimeoutError as e:
    print(f"Timeout: {e}")
```
