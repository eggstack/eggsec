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
- **Sync and async APIs** -- `Engine` blocks the calling thread;
  `AsyncEngine` returns Python `awaitables` via `PyFuture`.
- **Typed stubs** -- a `py.typed` marker and `.pyi` stubs are included for
  IDE autocompletion and static type checking.

## What the package provides

| Category | Sync API | Async API |
|---|---|---|
| Port scanning | `scan_ports()` / `Engine.run_port_scan()` | `async_scan_ports()` / `AsyncEngine.async_run_port_scan()` |
| Endpoint discovery | `scan_endpoints()` / `Engine.run_endpoint_scan()` | `async_scan_endpoints()` / `AsyncEngine.async_run_endpoint_scan()` |
| Service fingerprinting | `fingerprint_services()` / `Engine.run_fingerprint()` | `async_fingerprint_services()` / `AsyncEngine.async_run_fingerprint()` |
| DNS recon | `recon_dns()` / `Engine.run_recon_dns()` | `async_recon_dns()` / `AsyncEngine.async_run_recon_dns()` |
| TLS inspection | `inspect_tls()` / `Engine.run_tls_inspect()` | `async_inspect_tls()` / `AsyncEngine.async_run_tls_inspect()` |
| Technology detection | `detect_technology()` / `Engine.run_tech_detect()` | `async_detect_technology()` / `AsyncEngine.async_run_tech_detect()` |
| WAF detection | `detect_waf()` / `Engine.run_waf_detect()` | `async_detect_waf()` / `AsyncEngine.async_run_waf_detect()` |
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
| Consolidated recon | Yes | Milestone C |
| GraphQL security | Yes | Milestone C |
| OAuth/OIDC security | Yes | Milestone C |
| Auth assessment | Yes | Milestone C |
| Tool abstraction & JSON Schemas | Yes | Release 5 Phase A |
| NSE script metadata | No | Requires `nse` feature |
| Packet inspection / traceroute | No | Requires `packet-inspection` feature |
| Web proxy interception | No | Requires `web-proxy` feature |
| Mobile dynamic analysis | No | Requires `mobile` + `mobile-dynamic` features |
| Database pentest | No | Requires `db-pentest` feature |
| Daemon client | No | Requires `daemon-client` feature |
| Stress testing | No | Requires `stress-testing` feature |
| Headless browser | No | Requires `headless-browser` feature |
| Advanced hunting | No | Requires `advanced-hunting` feature |
| SBOM generation | No | Requires `sbom` feature |
| WebSocket testing | No | Requires `websocket` feature |

Use `eggsec.features()` and `eggsec.has_feature(name)` to check what is
available in your installed wheel at runtime.

## Comparison with other surfaces

| | Python bindings | CLI | TUI | REST API |
|---|---|---|---|---|
| Interface | `import eggsec` | Shell commands | Terminal UI | HTTP endpoints |
| Scope enforcement | `Scope` class + `EnforcementError` | `--target` flags | Scope config file | `ApprovedOperation` tokens |
| Async support | `AsyncEngine` + `asyncio` | N/A | N/A | N/A |
| Report formats | `to_dict()`, `to_json()`, `to_rows()`, `Report.write_*()` | `--format json\|sarif\|...` | Interactive | JSON responses |
| Feature parity | Core scanner, recon, WAF, fingerprint, Milestone C & D types | Full (all features) | Full (all features) | Depends on build |
| GIL behavior | Released during I/O | N/A | N/A | N/A |
| Embeddability | High -- import from any Python script | Requires subprocess | Standalone process | Requires HTTP client |
| Installation | `pip install eggsec` | Binary or `cargo install` | Binary or `cargo install` | Docker / binary |

---

## Documentation

### 1. Installation & Wheel Profiles

Build from source with maturin, install pre-built wheels, and understand
which features are compiled into each wheel profile. Covers development
installs, release builds, and PyPI publishing.

- [Installation](installation.md) -- build from source, development setup
- [Packaging & Release](packaging.md) -- wheel builds, PyPI publishing, versioning

### 2. Capability & Maturity Discovery

Discover what your installed wheel can do at runtime. Query feature flags,
API surface versions, domain maturity levels, and stability classifications
to write resilient code that adapts to the build profile.

- [Domain Maturity](domain-maturity.md) -- the twenty-two-operation stable core boundary
- [Stability Classifications](STABILITY_CLASSIFICATIONS.md) -- per-symbol stability mapping

### 3. Stable Operations (Engine & AsyncEngine)

The twenty-two-operation stable core: port scanning, endpoint discovery,
fingerprinting, DNS, TLS, technology detection, WAF, and promoted domain
operations. Covers both synchronous `Engine` and asynchronous `AsyncEngine`
dispatch paths.

- [Sync API](sync-api.md) -- `Engine` walkthrough with examples
- [Async API](async-api.md) -- `AsyncEngine` walkthrough with examples

### 4. Scope, Policy, Confirmation & Audit

Scope enforcement is the mandatory pre-dispatch gate. Understand how
targets are validated, how policy decisions are made, and how audit events
are emitted for every operation. Covers `EnforcementContext`, confirmation
prompts, and audit trails.

- [Scope & Safety](scope-and-safety.md) -- scope enforcement model and safety guarantees

### 5. Events, Callbacks, Cancellation & Timeouts

Observability and control flow for scan execution. Subscribe to typed
events, push findings via sinks, cancel long-running pipelines, and set
per-operation timeouts.

- [Events](events.md) -- `EventEnvelope`, typed event payloads, progress tracking
- [Callbacks](callbacks.md) -- `AuditSink`, `FindingSink`, `ProgressSink`

### 6. Pipelines & Checkpoints

Chain multi-step assessments into resumable pipelines with dependency
graphs, parallel groups, retry policies, and checkpoint persistence.
Pipelines survive interruptions and can resume from the last saved state.

- [Checkpoint & Resume](checkpoints-resume.md) -- pipeline state persistence
- [Pipeline Schema](PIPELINE_SCHEMA.md) -- pipeline definition format

### 7. Low-Level Networking & Protocol Probes

Programmable TCP/UDP sessions, DNS/TLS/HTTP one-shot probes, security-oriented
HTTP clients, and WebSocket sessions. These types are provisional -- scope-checked
and policy-gated but not part of the stable-core operation registry.

- [Network Programmability](network-programmability.md) -- transport, probes, HTTP client, WebSocket

### 8. Tools & JSON Schemas

The tool abstraction layer provides a unified `ToolDescriptor` for every
operation, deterministic JSON Schema generation, and registry-driven
dispatch. Use this to build custom UIs, validate requests, or discover
operation metadata programmatically.

- [Tool Abstraction Layer](tools.md) -- `ToolRegistry`, `ToolDescriptor`, `SchemaGenerator`
- [Tool Core Binding Map](TOOL_CORE_BINDING_MAP.md) -- machine-readable operation-to-type mapping

### 9. Repositories, Artifacts & Reporting

Persistent storage for findings, assessments, and artifacts. Content-addressed
artifact stores with deduplication, SQLite-backed finding repositories with
query and deduplication, and report generation in JSON/SARIF/JUnit/HTML/CSV/MD.

- [Reports](reports.md) -- `Finding`, `FindingSet`, `Report`
- [Repositories](repositories.md) -- `SqliteFindingRepository`, `SqliteAssessmentRepository`
- [Artifact Stores](artifact-stores.md) -- `ContentAddressedArtifactStore`

### 10. Daemon Execution

Run scans through the persistent daemon process for session continuity,
reconnect/replay semantics, and long-running assessments. The daemon client
is provisional until transport parity is complete.

- [Daemon Parity](daemon-parity.md) -- daemon execution model and protocol

### 11. Provisional Managed Sessions

Browser and mobile session lifecycle types for dynamic analysis. These are
provisional -- the underlying operations (`mobile-dynamic`, `headless-browser`)
require feature flags and are not part of the stable-core contract.

- [Browser Session Architecture](BROWSER_SESSION_ARCHITECTURE.md) -- headless browser session types
- [Mobile Session Lifecycle](mobile-session-lifecycle.md) -- mobile app session types

### 12. Experimental Domains

Features that may change or be removed without notice. These live in the
`eggsec.experimental` namespace and require explicit feature flags.

- [NSE Runtime Architecture](NSE_RUNTIME_ARCHITECTURE.md) -- NSE/Lua integration
- [Interception Proxy Architecture](INTERCEPTION_PROXY_ARCHITECTURE.md) -- MITM proxy subsystem

### 13. API Reference & Compatibility

Complete class and function reference, semantic versioning policy, and
migration guidance across releases.

- [API Reference](api-reference.md) -- complete class/function reference
- [Versioning](versioning.md) -- semver policy, schema management, stability guarantees
- [Migration Guide](MIGRATION_GUIDE.md) -- upgrade instructions across releases
