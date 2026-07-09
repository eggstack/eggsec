# Phase C Plan: Async API, Endpoint Discovery, and Service Fingerprinting

## Objective

Extend the Python library from a synchronous port-scan MVP into a practical scripting API with asyncio support, client runtime reuse, endpoint discovery, and service fingerprinting.

This phase should make Eggsec useful in Python automation pipelines while preserving the Rust engine's async-first design.

## Dependencies

This phase assumes Phase B is complete:

- `Client` and `Scope` exist.
- Sync `scan_ports` works.
- Result DTOs serialize to dict/JSON.
- Scope-denied operations raise `EnforcementError`.
- Local Python tests and examples exist.

## Public Python API additions

Add `AsyncClient`:

```python
client = eggsec.AsyncClient(
    scope=eggsec.Scope.allow_hosts(["example.com"]),
    mode="manual",
)

result = await client.scan_ports(
    "example.com",
    ports=[22, 80, 443],
    concurrency=256,
)
```

Add top-level async convenience functions:

```python
await eggsec.async_scan_ports(...)
await eggsec.async_scan_endpoints(...)
await eggsec.async_fingerprint_services(...)
```

Add sync and async endpoint discovery:

```python
result = client.scan_endpoints(
    base_url="https://example.com",
    endpoints=["admin", "login", "api/v1/users"],
    concurrency=32,
    timeout_ms=5000,
    include_404=False,
    verify_tls=True,
)
```

Add sync and async service fingerprinting:

```python
result = client.fingerprint_services(
    target="example.com",
    ports=[22, 80, 443],
    timeout_ms=2000,
)
```

Expose DTOs:

```python
EndpointScanConfig
EndpointScanResult
EndpointFinding
ServiceFingerprintResult
ServiceFingerprint
FingerprintEvidence
FingerprintConfidence
```

Use exact names that fit the existing Rust types, but prefer stable Python naming over exposing internal module names.

## Async runtime design

Use `pyo3-async-runtimes` or equivalent to bridge Rust futures to Python awaitables.

Avoid blocking the Python event loop. Async functions must not call sync wrappers internally.

Avoid nested Tokio runtime panics. `AsyncClient` should use the async bridge; `Client` can use a managed runtime internally.

Define cancellation behavior. Python task cancellation should attempt to cancel the Rust future. If cancellation cannot fully abort all underlying network work, document the limitation and ensure no panic or memory leak occurs.

## Client lifecycle

`Client` should own or share a sync runtime handle.

`AsyncClient` should not unnecessarily create a new runtime per call.

Both clients should support:

```python
client.close()
```

If feasible, also support context manager protocols:

```python
with eggsec.Client(scope=scope) as client:
    result = client.scan_ports(...)
```

```python
async with eggsec.AsyncClient(scope=scope) as client:
    result = await client.scan_ports(...)
```

If context managers add too much PyO3 surface area, defer them but document lifecycle behavior.

## Endpoint discovery binding

Bind the existing endpoint scanner through Python-facing config and result DTOs.

Required parameters:

```python
base_url: str
endpoints: list[str]
concurrency: int = 20
timeout_ms: int = 30000
include_404: bool = False
verify_tls: bool = True
```

Validate URL inputs early and return clear Python errors.

Scope checks should apply to the base URL host before scanning.

Do not expose spoofing or advanced stress-related endpoint scan options in this phase unless they are already safe and clearly scoped.

## Service fingerprinting binding

Bind service fingerprinting after port scanning and endpoint discovery are stable.

Required parameters:

```python
target: str
ports: list[int]
timeout_ms: int = 2000
```

Return evidence-bearing structured results. Preserve confidence values if present in Rust. Do not convert nuanced confidence values into bare booleans.

## DTO serialization

Every new result object should implement:

```python
to_dict()
to_json()
```

For endpoint results, include:

```python
base_url
endpoints_found
results
elapsed_ms
stats
```

For endpoint entries, include:

```python
url
path
status_code
content_length
content_type
redirect_location
interesting
```

Only expose fields that are actually available from Rust results. Do not invent data.

For fingerprinting results, include:

```python
target
services
evidence
confidence
elapsed_ms
```

## Documentation and examples

Add:

```text
docs/python/async-api.md
docs/python/endpoint-discovery.md
docs/python/fingerprinting.md
examples/python/async_multi_target_scan.py
examples/python/endpoint_discovery.py
examples/python/service_fingerprint.py
```

Examples should include:

- scanning several targets concurrently with `AsyncClient`
- endpoint discovery against a local HTTP fixture or clearly scoped staging URL
- service fingerprinting after a port scan

## Tests

Add Python tests:

```text
test_async_scan_ports.py
test_endpoint_discovery.py
test_service_fingerprinting.py
test_client_lifecycle.py
test_async_cancellation.py
```

Recommended fixtures:

- local TCP listener for open-port checks
- local HTTP server with known paths for endpoint discovery
- local banner-emitting TCP server for fingerprint evidence if feasible

Cancellation test:

- start an async scan with a long timeout
- cancel the Python task
- assert cancellation does not panic, leak, or leave the event loop blocked

## Validation commands

Run:

```bash
cargo check -p eggsec-python
cd crates/eggsec-python
maturin develop
pytest python/tests
python ../../examples/python/async_multi_target_scan.py
python ../../examples/python/endpoint_discovery.py
python ../../examples/python/service_fingerprint.py
```

If async runtime integration requires feature flags, document them in the local development guide.

## Acceptance criteria

`AsyncClient` works inside an existing Python asyncio event loop.

Top-level async convenience functions return awaitables.

Sync APIs remain functional after async support is added.

Endpoint discovery works against a local HTTP fixture.

Service fingerprinting returns structured evidence and confidence where available.

Client runtime reuse avoids creating unnecessary Tokio runtimes per call.

Cancellation behavior is tested and documented.

All new DTOs serialize to dict/JSON.

Scope checks apply to endpoint and fingerprinting targets.

## Out of scope

Reporting, WAF detection, passive recon, PyPI publication, wheel CI, NSE, fuzzing, load testing, raw packet features, stress tools, and broad major-tool expansion are out of scope for this phase.
