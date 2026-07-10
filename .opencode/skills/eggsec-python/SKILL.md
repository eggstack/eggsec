---
name: eggsec-python
description: "Python bindings for Eggsec via PyO3/maturin - use when working with Python integration, maturin builds, type stubs, or Python-side API usage."
---

# Eggsec Python Bindings Skill

Python bindings for the Eggsec security assessment engine via PyO3/maturin.

## Overview

The `eggsec-python` crate provides Python-native bindings over the Rust engine. It is a host-language binding (not an internal plugin runtime) that wraps `eggsec` and `eggsec-core` via PyO3. The GIL is released during network I/O.

**Status**: Experimental (0.1.0). Default wheel includes: core binding, scanner, endpoint discovery, service fingerprinting, recon, WAF detection, reporting, and policy/configuration/execution context (Milestone B).

## Directory Structure

```
crates/eggsec-python/
├── Cargo.toml              # PyO3 cdylib crate
├── pyproject.toml           # maturin build config
├── src/
│   ├── lib.rs               # PyModule definition, class/function registration
│   ├── client.rs            # Sync Client class
│   ├── async_client.rs      # AsyncClient class (tokio-backed)
│   ├── scope.rs             # Scope enforcement (allow_hosts, allow_cidrs)
│   ├── scanner.rs           # scan_ports, scan_endpoints, fingerprint_services
│   ├── recon.rs             # recon_dns, inspect_tls, detect_technology
│   ├── waf.rs               # detect_waf
│   ├── endpoint.rs          # EndpointScanConfig, EndpointFinding, EndpointScanResult
│   ├── fingerprint.rs       # FingerprintEvidence, ServiceFingerprintResult
│   ├── finding.rs           # Severity, Evidence, Finding, FindingSet, Report
│   ├── dto.rs               # PortScanResult, OpenPort, ScanStats, PortRange, TimingPreset
│   ├── error.rs             # Python exception hierarchy
│   ├── features.rs          # features(), has_feature()
│   ├── version.rs           # build_info()
│   ├── runtime_sync.rs      # Sync blocking wrapper
│   ├── runtime_async.rs     # Async runtime (PyFuture)
│   ├── config_model.rs      # SensitiveString, EggsecConfig, config sub-models
│   ├── scope_eval.rs        # LoadedScope, ScopeSource, ScopeRule, validate_scope()
│   ├── operation_metadata.rs # OperationRegistry, OperationDescriptor, OperationRisk, Capability
│   ├── execution_context.rs # EnforcementContext, ExecutionSurface, ExecutionProfile
│   ├── authorization.rs     # ExecutionPolicy, ManualOverride
│   ├── preflight.rs         # PreflightResult, preflight_operation()
│   └── audit.rs             # EnforcementAuditEvent, AuditOutcome, emit_audit_event()
├── python/
│   └── eggsec/
│       ├── __init__.py      # Re-exports all public API
│       ├── __init__.pyi     # Type stubs
│       ├── py.typed         # PEP 561 marker
│       └── *.pyi            # Per-module type stubs
└── tests/
    ├── test_import.py
    ├── test_scope.py
    ├── test_scan_ports.py
    ├── test_dto.py
    ├── test_endpoint.py
    ├── test_fingerprint.py
    ├── test_async.py
    ├── test_smoke.py
    └── test_policy_equivalence.py
```

## Build Commands

```bash
# Development build (installs into active venv)
cd crates/eggsec-python
maturin develop

# Release wheel
maturin build --release

# Develop with features (future use)
maturin develop --features <feature>
```

Requires Python >= 3.9 and `maturin>=1.5`.

## Feature Flags

The Python crate mirrors engine features via Cargo features:

```bash
# Default (no extra features)
maturin develop

# With specific features
maturin develop --features db-pentest
maturin develop --features web-proxy
maturin develop --features nse
maturin develop --features mobile

# All features without system dependencies
maturin develop --features full-no-system
```

| Python Feature | Engine Feature | System Dep | Notes |
|----------------|----------------|------------|-------|
| `websocket` | `websocket` | none | WebSocket security testing |
| `git-secrets` | `git-secrets` | none | Git secret detection |
| `sbom` | `sbom` | none | SBOM generation |
| `db-pentest` | `db-pentest` | none (drivers) | Database pentest (requires `eggsec-db-lab`) |
| `db-pentest-mongodb` | `db-pentest-mongodb` | none | MongoDB pentest |
| `db-pentest-redis` | `db-pentest-redis` | none | Redis pentest |
| `web-proxy` | `web-proxy` | none | Web proxy MITM (requires `eggsec-web-proxy`) |
| `mobile` | `mobile` | none | APK/IPA static analysis |
| `mobile-dynamic` | `mobile-dynamic` | ADB + device | Android dynamic testing |
| `packet-inspection` | `packet-inspection` | `libpcap-dev` | Packet capture |
| `stress-testing` | `stress-testing` | none | Stress testing (raw sockets) |
| `nse` | `nse` | `libssl-dev` | Nmap NSE scripts (requires `eggsec-nse`) |
| `container` | `container` | none | K8s/Docker scanning |
| `daemon-client` | — | none | Daemon session access |
| `full-no-system` | — | none | Aggregate: `websocket`, `git-secrets`, `sbom`, `container` |

## Test Commands

```bash
# Python-side tests
pytest crates/eggsec-python/tests/

# Rust-side tests
cargo test -p eggsec-python

# Policy equivalence tests (Milestone B)
pytest crates/eggsec-python/tests/test_policy_equivalence.py
```

## API Surface

### Classes

| Class | Purpose |
|-------|---------|
| `Scope` | Target/port authorization (frozen). Use `Scope.allow_hosts()` or `Scope.allow_cidrs()`. |
| `Client` | Sync scan client. Releases GIL during I/O. |
| `AsyncClient` | Async scan client (tokio-backed). Returns `PyFuture` objects. |
| `PyFuture` | Awaitable wrapper for async Rust futures. |
| `EggsecConfig` | Full configuration model. Use `EggsecConfig.load()` or `EggsecConfig.default()`. |
| `LoadedScope` | Enriched scope with source tracking. Use `LoadedScope.default_empty()` or `LoadedScope.explicit(...)`. |
| `OperationRegistry` | Static registry of operation metadata. Use `OperationRegistry.all_operations()`, `.find()`, `.find_by_tool_id()`. |
| `OperationMetadataView` | Read-only view of an operation's metadata. Use `.descriptor_for_target()` to get a writable `OperationDescriptor`. |
| `OperationDescriptor` | Writable descriptor for a specific target. Required by `EnforcementContext.evaluate()`. |
| `EnforcementContext` | Policy evaluation gate. Use `manual_permissive()`, `mcp_strict()`, `agent_strict()`, `ci_strict()`, or `for_surface()`. |
| `ExecutionPolicy` | Risk-level policy config. Use `ExecutionPolicy.default()` or `ExecutionPolicy.from_config()`. |
| `ExecutionSurface` | Surface identifier. Static constants: `CLI_MANUAL`, `TUI_MANUAL`, `MCP_SERVER`, `SECURITY_AGENT`, `CI`, etc. |
| `ExecutionProfile` | Profile for enforcement. Constants: `manual_permissive`, `mcp_strict`, `agent_strict`, `ci_strict`. |
| `ManualOverride` | Override flags for manual surfaces (reason, allow_high_risk, etc.). |
| `ApprovedOperation` | Authorization token from `EnforcementContext.approve()`. Contains audit event ID. |
| `PolicyDecision` | Result of policy evaluation (allowed, denied, warnings, confirmation required). |
| `EnforcementOutcome` | Rich outcome from `evaluate()` (outcome_type, decision, warnings, confirmation_classes). |
| `PreflightResult` | Pre-dispatch preview (outcome, suggested CLI flags, scope status, risk level). |
| `SensitiveString` | Zeroized secret wrapper. Use `SensitiveString.new("value")`, `.expose_secret()` to read. |
| `EnforcementAuditEvent` | Audit trail entry with event_id, timestamp, surface, outcome, scope info, policy hash. |
| `ScopeValidation` | Result of `validate_scope()` (valid, errors, warnings, target/exclusion counts). |
| `AlertChannelConfig` | Alert channel config. Use `.webhook()`, `.email()`, `.slack()`, `.pagerduty()` static constructors. |

### Functions

| Function | Sync/Async | Purpose |
|----------|-----------|---------|
| `scan_ports` / `async_scan_ports` | Both | TCP port scanning |
| `scan_endpoints` / `async_scan_endpoints` | Both | Hidden endpoint discovery |
| `fingerprint_services` / `async_fingerprint_services` | Both | Service fingerprinting |
| `recon_dns` / `async_recon_dns` | Both | DNS enumeration |
| `inspect_tls` / `async_inspect_tls` | Both | TLS certificate inspection |
| `detect_technology` / `async_detect_technology` | Both | Technology stack detection |
| `detect_waf` / `async_detect_waf` | Both | WAF detection |
| `validate_waf` / `async_validate_waf` | Both | WAF bypass validation (requires scope) |
| `fuzz_http` / `async_fuzz_http` | Both | HTTP fuzzing (requires scope) |
| `load_test_http` / `async_load_test_http` | Both | HTTP load testing (requires scope) |
| `features` | Sync | List available features |
| `has_feature` | Sync | Check if a feature is compiled in |
| `build_info` | Sync | Build metadata |
| `preflight_operation` | Sync | Pre-dispatch policy preview by operation ID |
| `preflight_with_descriptor` | Sync | Pre-dispatch policy preview with explicit descriptor |
| `validate_scope` | Sync | Validate a `LoadedScope` (returns errors/warnings) |
| `audit_event_from_enforcement` | Sync | Create `EnforcementAuditEvent` from enforcement outcome |
| `audit_event_from_preflight` | Sync | Create `EnforcementAuditEvent` from preflight result |
| `emit_audit_event` | Sync | Emit an audit event (logging/sink) |

### Exceptions

| Exception | Parent |
|-----------|--------|
| `EggsecError` | `Exception` |
| `ConfigError` | `EggsecError` |
| `ScopeError` | `EggsecError` |
| `EnforcementError` | `EggsecError` |
| `NetworkError` | `EggsecError` |
| `ScanError` | `EggsecError` |
| `TimeoutError` | `EggsecError` |
| `FeatureUnavailableError` | `EggsecError` |
| `SerializationError` | `EggsecError` |
| `InternalError` | `EggsecError` |

## Common Patterns

### Scope Creation

```python
from eggsec import Scope

# Allow specific hosts
scope = Scope.allow_hosts(["example.com", "10.0.0.0/8"])

# Allow CIDR ranges
scope = Scope.allow_cidrs(["192.168.0.0/16"])
```

### Sync Client Usage

```python
from eggsec import Client, Scope

scope = Scope.allow_hosts(["example.com"])
client = Client(scope, mode="manual", concurrency=100, timeout_ms=5000)

result = client.scan_ports("example.com", [80, 443, 8080])
for port in result.open_ports:
    print(f"Port {port.port} is {port.state}")
```

### Async Client Usage

```python
import asyncio
from eggsec import AsyncClient, Scope

async def main():
    scope = Scope.allow_hosts(["example.com"])
    client = AsyncClient(scope)

    future = client.scan_ports("example.com", [80, 443])
    result = await future
    print(result)

asyncio.run(main())
```

### Standalone Functions (No Client)

```python
from eggsec import scan_ports, Scope

scope = Scope.allow_hosts(["example.com"])
result = scan_ports(scope, "example.com", [80, 443, 8080])
```

### Finding/Report Access

```python
from eggsec import Severity

# Results include FindingSet with typed findings
for finding in result.findings:
    if finding.severity >= Severity.HIGH:
        print(f"Critical: {finding.title}")
```

### EnforcementContext Usage

```python
from eggsec import (
    EnforcementContext, ExecutionPolicy, ExecutionSurface,
    LoadedScope, ManualOverride, OperationRegistry,
)

scope = LoadedScope.default_empty()
policy = ExecutionPolicy.default()

# CLI manual surface — operator-directed, supports overrides
ctx = EnforcementContext.manual_permissive(policy, scope)

# MCP/REST surface — strict, no overrides
ctx = EnforcementContext.mcp_strict(policy, scope)

# Agent surface — explicit scope, no overrides
ctx = EnforcementContext.agent_strict(policy, scope)

# CI surface — hard enforcement
ctx = EnforcementContext.ci_strict(policy, scope)

# Custom surface
ctx = EnforcementContext.for_surface(ExecutionSurface.GRPC_API, policy, scope)

# Evaluate an operation
op = OperationRegistry.find("port_scan")
desc = op.descriptor_for_target("example.com")
outcome = ctx.evaluate(desc)
print(outcome.outcome_type)  # "allow", "confirm", or "deny"

# Approve (strict surfaces require this before dispatch)
approved = ctx.approve(ExecutionSurface.MCP_SERVER, desc)

# Manual override (CLI/TUI only)
override = ManualOverride(reason="testing", allow_high_risk=True)
approved = ctx.approve_manual(ExecutionSurface.CLI_MANUAL, desc, override)
```

### OperationRegistry Discovery

```python
from eggsec import OperationRegistry

# List all registered operations
all_ops = OperationRegistry.all_operations()
for op in all_ops:
    print(f"{op.operation_id}: {op.label} (risk={op.risk.name})")

# Find by operation ID
op = OperationRegistry.find("port_scan")
if op:
    print(op.description)

# Find by tool ID (MCP/REST tool name)
op = OperationRegistry.find_by_tool_id("eggsec.port_scan")

# Get a descriptor for a specific target (mutable copy)
desc = op.descriptor_for_target("192.168.1.1")
```

### Preflight Evaluation

```python
from eggsec import preflight_operation, preflight_with_descriptor

# Quick preview by operation ID
result = preflight_operation("port_scan", target="example.com")
print(result.outcome)           # "allow", "confirm", "deny"
print(result.suggested_cli_flags)

# With explicit descriptor
desc = op.descriptor_for_target("example.com")
result = preflight_with_descriptor(desc)
print(result.scope_status)
print(result.risk_level)
```

### Audit Event Creation

```python
from eggsec import (
    audit_event_from_enforcement, audit_event_from_preflight, emit_audit_event,
)

# From enforcement outcome
event = audit_event_from_enforcement(
    surface="CLI_MANUAL",
    operation_id="port_scan",
    target="example.com",
    allowed=True,
    denied=False,
    confirmed=False,
    override_ignored=False,
    decision_summary="Auto-approved: passive risk",
    confirmation_classes=[],
    manual_override_reason=None,
    scope_source="config_file",
    scope_path=None,
    allow_rule_count=3,
    exclusion_rule_count=0,
    explicit_manifest=True,
    policy_hash="abc123",
)
emit_audit_event(event)

# From preflight
event = audit_event_from_preflight(
    surface="MCP_SERVER",
    operation_id="port_scan",
    target="example.com",
    allowed=True,
    denied=False,
    decision_summary="Scope validated",
    confirmation_classes=[],
    scope_source="config_file",
    scope_path=None,
    allow_rule_count=3,
    exclusion_rule_count=0,
    explicit_manifest=True,
    policy_hash="abc123",
)
```

### Configuration Loading

```python
from eggsec import EggsecConfig, SensitiveString

# Load from default path
config = EggsecConfig.load()

# Load from custom path
config = EggsecConfig.load("/etc/eggsec/config.toml")

# Validate
errors = config.validate()
if errors:
    print("Config errors:", errors)

# Access sub-configs
print(config.http.timeout_secs)
print(config.scan.default_concurrency)
print(config.output.format)

# SensitiveString handling
secret = SensitiveString.new("api-key-123")
print(secret.expose_secret())  # "api-key-123"
print(secret.is_empty())       # False
```

### Scope Evaluation

```python
from eggsec import LoadedScope, validate_scope

# Default empty scope
scope = LoadedScope.default_empty()

# Check targets
print(scope.is_target_allowed("example.com"))
print(scope.is_port_allowed(80))
print(scope.is_excluded("10.0.0.1"))

# Get explanation
explanation = scope.explain("example.com")
print(explanation.allowed, explanation.reason)

# Validate
result = validate_scope(scope)
print(result.valid, result.errors, result.warnings)
```

## Type Stubs

Full type stubs are included in the wheel:
- `python/eggsec/__init__.pyi` — top-level stubs
- `python/eggsec/*.pyi` — per-module stubs (client, scope, dto, endpoint, fingerprint, finding, recon, waf, config_model, scope_eval, operation_metadata, execution_context, authorization, preflight, audit, etc.)
- `python/eggsec/py.typed` — PEP 561 marker for type checker support

**Naming convention**: Some Rust modules export types with `Py` suffixes internally (e.g., `ExecutionSurfacePy`, `EnforcementContextPy`). The `__init__.py` re-exports these under clean names (`ExecutionSurface`, `EnforcementContext`). Type stubs use the clean names.

## Documentation

See `docs/python/` for user-facing guides:
- `quickstart.md` — getting started
- `installation.md` — install options
- `scope-and-safety.md` — scope enforcement details
- `scanner.md` — port scanning
- `endpoint-discovery.md` — endpoint discovery
- `service-fingerprinting.md` — service fingerprinting
- `recon.md` — reconnaissance (DNS, TLS, tech detection)
- `waf.md` — WAF detection
- `reports.md` — findings and reporting
- `sync-api.md` / `async-api.md` — API patterns
- `api-reference.md` — full API reference
- `packaging.md` — distribution/packaging notes

## CI

Python binding tests run in `test.yml` GitHub Actions workflow alongside Rust tests.

## Key Files

| File | Purpose |
|------|---------|
| `src/lib.rs` | PyModule definition, all class/function registration |
| `src/client.rs` | `Client` class — sync scanning |
| `src/async_client.rs` | `AsyncClient` class — async scanning |
| `src/scope.rs` | `Scope` class — target authorization |
| `src/error.rs` | Python exception hierarchy |
| `src/config_model.rs` | `SensitiveString`, `EggsecConfig`, config sub-models |
| `src/scope_eval.rs` | `LoadedScope`, `ScopeSource`, `ScopeRule`, `validate_scope()` |
| `src/operation_metadata.rs` | `OperationRegistry`, `OperationDescriptor`, `OperationRisk`, `Capability` |
| `src/execution_context.rs` | `EnforcementContext`, `ExecutionSurface`, `ExecutionProfile` |
| `src/authorization.rs` | `ExecutionPolicy`, `ManualOverride` |
| `src/preflight.rs` | `PreflightResult`, `preflight_operation()` |
| `src/audit.rs` | `EnforcementAuditEvent`, `AuditOutcome`, `emit_audit_event()` |
| `python/eggsec/__init__.py` | Public API re-exports |
| `python/eggsec/__init__.pyi` | Top-level type stubs |
| `pyproject.toml` | maturin build configuration |

## Common Tasks

### Adding a New Python Function
1. Implement Rust function in appropriate `src/*.rs` file with `#[pyfunction]`
2. Register in `src/lib.rs` via `m.add_function(wrap_pyfunction!(...)?)`
3. Re-export in `python/eggsec/__init__.py`
4. Add type stub in `python/eggsec/*.pyi`
5. Add tests in `tests/`

### Adding a New Python Class
1. Implement with `#[pyclass]` and `#[pymethods]` in `src/*.rs`
2. Register in `src/lib.rs` via `m.add_class::<T>()`
3. Re-export in `python/eggsec/__init__.py`
4. Add type stub
5. Add tests

## Known Limitations

- **Async bridge**: Hand-rolled `PyFuture` wrapper, not `pyo3-async-runtimes`. The `AsyncClient` spawns a tokio task and polls from Python's event loop via `PyFuture`. This works but lacks integration with Python's native `asyncio` cancellation propagation.
- **GIL release**: GIL is released during network I/O (blocking calls use `py.allow_threads()`), but CPU-bound Rust work holds the GIL.
- **Feature parity**: Not all engine features are exposed to Python. Feature-gated modules (e.g., `fuzzer`, `loadtest`, `stress`) require explicit `--features` at build time.
- **Type stubs**: Generated manually, not auto-generated from Rust source. Keep `python/eggsec/*.pyi` in sync with `src/` changes.
