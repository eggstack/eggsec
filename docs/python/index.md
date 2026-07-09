# Python Bindings

Native Python bindings for the Eggsec security assessment engine, built with
[PyO3](https://pyo3.rs) and [maturin](https://github.com/PyO3/maturin).

## Overview

The `eggsec` Python package provides a **host-language binding** over the
Rust engine. It compiles the full Eggsec core into a Python extension module
(`eggsec._core`) with zero Python runtime dependencies. The engine runs
entirely in Rust; only the binding shim lives on the Python side.

Key characteristics:

- **No subprocess overhead** -- the engine is called directly via PyO3 FFI,
  not by shelling out to the CLI.
- **GIL released during I/O** -- all network operations release the GIL so
  other Python threads can run concurrently.
- **Scope enforcement** -- every scan target is validated against a `Scope`
  before any network request is made. Scope violations raise
  `EnforcementError`.
- **Sync and async APIs** -- `Client` blocks the calling thread;
  `AsyncClient` returns Python `awaitables` via `PyFuture`.
- **Typed stubs** -- a `py.typed` marker and `.pyi` stubs are included for
  IDE autocompletion and static type checking.

## What the package provides

| Category | Sync API | Async API |
|---|---|---|
| Port scanning | `scan_ports()` / `Client.scan_ports()` | `async_scan_ports()` / `AsyncClient.scan_ports()` |
| Endpoint discovery | `scan_endpoints()` / `Client.scan_endpoints()` | `async_scan_endpoints()` / `AsyncClient.scan_endpoints()` |
| Service fingerprinting | `fingerprint_services()` / `Client.fingerprint_services()` | `async_fingerprint_services()` / `AsyncClient.fingerprint_services()` |
| DNS recon | `recon_dns()` / `Client.recon_dns()` | `async_recon_dns()` / `AsyncClient.recon_dns()` |
| TLS inspection | `inspect_tls()` / `Client.inspect_tls()` | `async_inspect_tls()` / `AsyncClient.inspect_tls()` |
| Technology detection | `detect_technology()` / `Client.detect_technology()` | `async_detect_technology()` / `AsyncClient.detect_technology()` |
| WAF detection | `detect_waf()` / `Client.detect_waf()` | `async_detect_waf()` / `AsyncClient.detect_waf()` |
| Findings & reporting | `Finding`, `FindingSet`, `Report` | (same classes) |
| Scope enforcement | `Scope` | (same class) |
| Feature introspection | `features()`, `has_feature()`, `build_info()` | (same functions) |

## Feature availability

The Python bindings compile the engine with a **default feature set**. The
table below shows what is available out of the box and what requires
additional configuration.

| Feature | Default wheel | Notes |
|---|---|---|
| Port scanning | Yes | |
| Endpoint discovery | Yes | |
| Service fingerprinting | Yes | |
| DNS recon | Yes | |
| TLS inspection | Yes | |
| Technology detection | Yes | |
| WAF detection | Yes | |
| Findings & reporting | Yes | |
| Scope enforcement | Yes | |
| NSE (Nmap scripts) | No | Requires `nse` feature at build time |
| Stress testing | No | Requires `stress-testing` feature |
| Packet inspection | No | Requires `packet-inspection` feature |
| Headless browser | No | Requires `headless-browser` feature |
| Database persistence | No | Requires `database` feature |
| Cloud integration | No | Requires `cloud` feature |
| SBOM generation | No | Requires `sbom` feature |
| WebSocket testing | No | Requires `websocket` feature |

Use `eggsec.features()` and `eggsec.has_feature(name)` to check what is
available in your installed wheel at runtime.

## Comparison with other surfaces

| | Python bindings | CLI | TUI | REST API |
|---|---|---|---|---|
| Interface | `import eggsec` | Shell commands | Terminal UI | HTTP endpoints |
| Scope enforcement | `Scope` class + `EnforcementError` | `--target` flags | Scope config file | `ApprovedOperation` tokens |
| Async support | `AsyncClient` + `asyncio` | N/A | N/A | N/A |
| Report formats | `to_dict()`, `to_json()`, `to_rows()`, `Report.write_*()` | `--format json\|sarif\|...` | Interactive | JSON responses |
| Feature parity | Core scanner, recon, WAF, fingerprint | Full (all features) | Full (all features) | Depends on build |
| GIL behavior | Released during I/O | N/A | N/A | N/A |
| Embeddability | High -- import from any Python script | Requires subprocess | Standalone process | Requires HTTP client |
| Installation | `pip install eggsec` | Binary or `cargo install` | Binary or `cargo install` | Docker / binary |

## Documentation

| Document | Description |
|---|---|
| [Installation](installation.md) | Build from source, development setup |
| [Packaging & Release](packaging.md) | Wheel builds, PyPI publishing, versioning |
| [Quickstart](quickstart.md) | First scan in 5 lines of Python |
| [Scope & Safety](scope-and-safety.md) | Scope enforcement model and safety guarantees |
| [Sync API](sync-api.md) | `Client` walkthrough with examples |
| [Async API](async-api.md) | `AsyncClient` walkthrough with examples |
| [Port Scanning](scanner.md) | `scan_ports`, `PortScanResult`, `PortRange` |
| [Endpoint Discovery](endpoint-discovery.md) | `scan_endpoints`, `EndpointScanResult` |
| [Service Fingerprinting](service-fingerprinting.md) | `fingerprint_services`, `FingerprintScanResult` |
| [Recon](recon.md) | DNS, TLS, technology detection |
| [WAF Detection](waf.md) | `detect_waf`, `WafDetectionResult` |
| [Reports](reports.md) | `Finding`, `FindingSet`, `Report` |
| [API Reference](api-reference.md) | Complete class/function reference |
