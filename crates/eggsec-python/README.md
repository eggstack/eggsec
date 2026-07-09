# eggsec-python

Python bindings for the [Eggsec](https://github.com/anomalyco/eggsec) security assessment engine.

## Status

**Phase C** — Async client, endpoint discovery, and service fingerprinting.

### Features

- **Port scanning** — scoped TCP port scans with service detection
- **Endpoint discovery** — HTTP path probing with status classification
- **Service fingerprinting** — banner analysis and service identification
- **Sync API** — blocking calls with GIL release during I/O
- **Async API** — non-blocking `AsyncClient` and convenience functions
- **Scope enforcement** — all operations validated against authorization scope

## Architecture

Eggsec is a Rust-native security assessment engine. These bindings use [PyO3](https://pyo3.rs) and [maturin](https://github.com/PyO3/maturin) to expose the engine as a Python-native library.

This is a **host-language binding**, not an internal plugin runtime. The Rust engine is compiled into a Python extension module.

## Installation (development)

```bash
cd crates/eggsec-python
python -m venv .venv
source .venv/bin/activate
pip install maturin pytest
maturin develop
python -c "import eggsec; print(eggsec.__version__)"
pytest
```

## Quick Start

```python
import eggsec

# Define scope
scope = eggsec.Scope.allow_hosts(["127.0.0.1"])

# Port scan
result = eggsec.scan_ports("127.0.0.1", [22, 80, 443], scope)
for port in result.open_ports:
    print(f"  {port.port}: {port.service}")

# Endpoint discovery
config = eggsec.EndpointScanConfig(
    base_url="http://127.0.0.1",
    endpoints=["/", "/admin", "/login"],
)
result = eggsec.scan_endpoints(config, scope)
print(f"Found {result.found} endpoints")

# Service fingerprinting
result = eggsec.fingerprint_services("127.0.0.1", [22, 80, 443], scope)
for svc in result.services:
    print(f"  {svc.port}: {svc.service} {svc.version or ''}")

# Async
future = eggsec.async_scan_ports("127.0.0.1", [80], scope)
for result in future:
    if result is not None:
        print(result)
```

## API Overview

### Classes

| Class | Description |
|-------|-------------|
| `Scope` | Authorization scope (frozen, factory methods) |
| `Client` | Sync scan client with scope enforcement |
| `AsyncClient` | Async scan client (context manager) |
| `EndpointScanConfig` | Endpoint discovery configuration |
| `EndpointScanResult` | Endpoint scan results |
| `EndpointFinding` | Individual endpoint finding |
| `FingerprintScanResult` | Fingerprint scan results |
| `ServiceFingerprintResult` | Individual service fingerprint |
| `PortScanResult` | Port scan results |
| `PortRange` | Port list helpers |
| `TimingPreset` | Scan timing profiles |
| `PyFuture` | Pollable async future |

### Functions

| Function | Description |
|----------|-------------|
| `scan_ports()` | Sync port scan |
| `async_scan_ports()` | Async port scan |
| `scan_endpoints()` | Sync endpoint scan |
| `async_scan_endpoints()` | Async endpoint scan |
| `fingerprint_services()` | Sync service fingerprinting |
| `async_fingerprint_services()` | Async service fingerprinting |
| `features()` | Available feature flags |
| `has_feature()` | Check a feature flag |
| `build_info()` | Build metadata |

### Exceptions

- `EggsecError` — base for all errors
- `ConfigError` — configuration errors
- `ScopeError` — scope parsing errors
- `EnforcementError` — scope violations
- `NetworkError` — network errors
- `ScanError` — scan failures
- `TimeoutError` — timeouts
- `FeatureUnavailableError` — missing features
- `SerializationError` — serialization errors
- `InternalError` — internal engine errors

## Documentation

- [Quick Start](../../docs/python/quickstart.md)
- [Sync API Reference](../../docs/python/sync-api.md)
- [Async API Reference](../../docs/python/async-api.md)
- [Endpoint Discovery](../../docs/python/endpoint-discovery.md)
- [Service Fingerprinting](../../docs/python/service-fingerprinting.md)
- [Scanner Guide](../../docs/python/scanner.md)
- [Scope & Safety](../../docs/python/scope-and-safety.md)

## License

MIT
