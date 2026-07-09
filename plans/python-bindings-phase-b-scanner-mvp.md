# Phase B Plan: Scanner MVP

Status: Executed

## Objective

Expose Eggsec's first real Python-facing capability: scoped, synchronous TCP port scanning with stable result DTOs and JSON/dict serialization. This phase should prove that Python can call the Rust engine directly, receive structured results, and enforce scope semantics without going through the CLI.

The MVP should be useful for basic scripts while staying narrow enough to keep the binding API stable.

## Dependencies

This phase assumes Phase A is complete:

- `crates/eggsec-python` exists.
- `import eggsec` works after local maturin build.
- Python exception classes exist.
- `features()` and `has_feature()` exist.
- The crate is not coupled to CLI/TUI.

## Public Python API

Add top-level convenience API:

```python
result = eggsec.scan_ports(
    target="example.com",
    ports=[22, 80, 443],
    concurrency=100,
    timeout_ms=5000,
    scope=eggsec.Scope.allow_hosts(["example.com"]),
)
```

Add client-oriented API:

```python
client = eggsec.Client(
    scope=eggsec.Scope.allow_hosts(["example.com"]),
    mode="manual",
)

result = client.scan_ports(
    "example.com",
    ports=[22, 80, 443],
    concurrency=100,
    timeout_ms=5000,
)
```

Expose initial classes:

```python
Scope
Client
PortRange
PortScanConfig
PortScanResult
OpenPort
ScanStats
TimingPreset
```

Keep the top-level `scan_ports` as a convenience wrapper that creates an ephemeral client internally.

## Rust implementation shape

Add source modules:

```text
crates/eggsec-python/src/
  client.rs
  scope.rs
  scanner.rs
  dto.rs
  runtime_sync.rs
```

The sync runtime should be managed carefully. Avoid creating a new Tokio runtime for every low-level operation if the `Client` path can reuse one. Top-level convenience functions can create a temporary runtime if needed, but client-based scans should reuse runtime state.

The scanner binding should call existing Rust scanner primitives directly, not shell out to the `eggsec` binary.

Use `py.allow_threads(...)` or equivalent PyO3 patterns to avoid holding the GIL while Rust performs network I/O.

## Scope model

Implement a minimal Python `Scope` wrapper.

Required constructors:

```python
Scope.allow_hosts(hosts: list[str])
Scope.allow_cidrs(cidrs: list[str])
Scope.deny_all()
```

Optional if easy:

```python
Scope.from_file(path: str)
```

Do not add unrestricted scope by default. If an unrestricted manual scope is needed later, it must have an explicit name and acknowledgement.

The first scope implementation can convert into existing Eggsec scope/config enforcement types through the engine's current config model or runtime bridge. Do not create a separate permissive Python-only enforcement path.

## Mode model

Add `mode` to `Client`:

```python
mode="manual"
mode="automation"
```

In this phase, it is acceptable for `automation` to be implemented conservatively and to reject manual override-like behavior even if not all runtime-surface variants are plumbed yet.

Invalid modes should raise `ValueError`.

Scope violations should raise `EnforcementError`.

## DTO behavior

`PortScanResult` should expose:

```python
result.target
result.open_ports
result.scanned_ports
result.elapsed_ms
result.stats
result.to_dict()
result.to_json()
```

`OpenPort` should expose:

```python
port.port
port.protocol
port.service
port.banner
port.confidence
```

If the Rust result type does not contain all fields, expose only what exists and do not invent data. Keep field names stable and documented.

`PortRange` should support:

```python
PortRange.list([22, 80, 443])
PortRange.range(1, 1024)
PortRange.top_100()
PortRange.top_1000()
```

If top port lists are not already available, implement only `list` and `range` now and defer named lists.

## Documentation

Add:

```text
docs/python/quickstart.md
docs/python/sync-api.md
docs/python/scanner.md
docs/python/scope-and-safety.md
examples/python/basic_port_scan.py
examples/python/scan_to_json.py
```

The quickstart should show a scoped scan against localhost or an explicitly authorized host. Avoid public third-party targets in examples unless clearly documented as placeholders.

The safety doc should explain that Python library mode is not the same as NSE/Lua script execution and not the same as shelling out to CLI.

## Tests

Add Python tests:

```text
crates/eggsec-python/python/tests/test_import.py
crates/eggsec-python/python/tests/test_scope.py
crates/eggsec-python/python/tests/test_scan_ports.py
crates/eggsec-python/python/tests/test_serialization.py
```

Recommended test strategy:

1. Start a local TCP listener in Python on `127.0.0.1` using an ephemeral port.
2. Call `eggsec.scan_ports("127.0.0.1", ports=[port], scope=Scope.allow_hosts(["127.0.0.1"]))`.
3. Assert the port is reported open.
4. Scan a closed local port and assert no false open result.
5. Attempt a target outside scope and assert `EnforcementError`.
6. Assert `result.to_dict()` and `result.to_json()` are valid.

Add Rust tests for conversion code where practical.

## Validation commands

Run:

```bash
cargo check -p eggsec-python
cd crates/eggsec-python
maturin develop
pytest python/tests
python ../../examples/python/basic_port_scan.py
```

If the repo uses a different test directory layout, adjust paths but keep the same coverage.

## Acceptance criteria

Python can perform a scoped synchronous TCP port scan through Rust engine calls.

The binding does not shell out to the CLI.

`Client.scan_ports(...)` and `eggsec.scan_ports(...)` both work.

Results expose stable Python attributes and serialize to dict/JSON.

Scope-denied targets raise `EnforcementError`.

Invalid user arguments raise Python-native exceptions such as `ValueError` or a specific Eggsec exception.

The GIL is not held for the full duration of network scans.

Docs and examples show basic scanner usage.

## Out of scope

Async APIs, endpoint discovery, service fingerprinting, WAF detection, recon, reporting, PyPI release, NSE, packet inspection, stress testing, and major tool expansion are out of scope for this phase.
