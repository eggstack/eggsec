# eggsec

Python bindings for the [Eggsec](https://github.com/sugarwookie/eggsec) security assessment engine.

## Status

**Phase E** — Packaging, PyPI, type stubs, and documentation hardening.

## Installation

```bash
pip install eggsec
```

### Supported Platforms

| Platform | Architecture |
|----------|-------------|
| macOS | arm64 (Apple Silicon) |
| macOS | x86_64 |
| Linux | x86_64 (manylinux) |
| Linux | aarch64 (manylinux) |

Prebuilt wheels are available — no Rust toolchain required.

### Included Features (default wheel)

- Port scanning with service detection
- Endpoint discovery and HTTP path probing
- Service fingerprinting and banner analysis
- Passive recon (DNS, TLS inspection, technology detection)
- WAF detection
- Findings and reporting (JSON, Markdown)
- Sync and async APIs
- Scope enforcement

### Not Included (default wheel)

The following require building from source with feature flags:

- Nmap NSE/Lua compatibility
- Raw packet inspection
- Stress testing / DoS simulation
- Headless browser automation
- Database pentest native drivers
- Wireless tooling
- Cloud SDK-heavy features

## Quick Start

```python
import eggsec

# Check available features
print(eggsec.features())

# Define scope
scope = eggsec.Scope.allow_hosts(["127.0.0.1"])

# Port scan
result = eggsec.scan_ports("127.0.0.1", [22, 80, 443], scope)
for port in result.open_ports:
    print(f"  {port.port}: {port.service}")

# Passive recon
dns = eggsec.recon_dns("example.com")
print(dns.a)

tls = eggsec.inspect_tls("example.com")
print(tls.certificate.subject)

# WAF detection
waf = eggsec.detect_waf("https://example.com")
if waf.detected:
    print(f"WAF: {waf.waf_name}")

# Reporting
report = eggsec.Report()
report.add_result(result)
report.write_json("scan_report.json")
```

## API Overview

### Classes

| Class | Description |
|-------|-------------|
| `Scope` | Authorization scope (frozen, factory methods) |
| `Client` | Sync client with scope enforcement |
| `AsyncClient` | Async client (context manager) |
| `PortScanResult` | Port scan results |
| `EndpointScanResult` | Endpoint scan results |
| `FingerprintScanResult` | Fingerprint results |
| `DnsRecordSet` | DNS recon results |
| `TlsInspectionResult` | TLS inspection results |
| `TechDetectionResult` | Technology detection |
| `WafDetectionResult` | WAF detection results |
| `Finding` | Individual security finding |
| `Report` | Aggregated findings report |
| `Severity` | Finding severity enum |

### Functions

| Function | Description |
|----------|-------------|
| `scan_ports()` / `async_scan_ports()` | Port scanning |
| `scan_endpoints()` / `async_scan_endpoints()` | Endpoint discovery |
| `fingerprint_services()` / `async_fingerprint_services()` | Service fingerprinting |
| `recon_dns()` / `async_recon_dns()` | DNS reconnaissance |
| `inspect_tls()` / `async_inspect_tls()` | TLS certificate inspection |
| `detect_technology()` / `async_detect_technology()` | Technology stack detection |
| `detect_waf()` / `async_detect_waf()` | WAF detection |
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

## Typing

This package ships `py.typed` and `.pyi` type stubs for full IDE support.

## Documentation

- [Installation](../../docs/python/installation.md)
- [Quick Start](../../docs/python/quickstart.md)
- [Sync API Reference](../../docs/python/sync-api.md)
- [Async API Reference](../../docs/python/async-api.md)
- [Scanner Guide](../../docs/python/scanner.md)
- [Scope & Safety](../../docs/python/scope-and-safety.md)
- [Endpoint Discovery](../../docs/python/endpoint-discovery.md)
- [Service Fingerprinting](../../docs/python/service-fingerprinting.md)
- [Recon](../../docs/python/recon.md)
- [WAF Detection](../../docs/python/waf.md)
- [Reports](../../docs/python/reports.md)
- [Packaging & Release](../../docs/python/packaging.md)

## Safety

All operations enforce authorization scope. Scans only target hosts and ports explicitly allowed in the scope configuration. See [Scope & Safety](../../docs/python/scope-and-safety.md) for details.

## License

MIT
