# eggsec

Python bindings for the [Eggsec](https://github.com/sugarwookie/eggsec) security assessment engine.

## Status

**Experimental / Alpha** — Pre-release. Not yet published to PyPI. See `RELEASE_CHECKLIST.md` for publication gates.

## Installation

```bash
# Development build (requires Rust toolchain)
cd crates/eggsec-python
maturin develop

# From source wheel
maturin build --release
pip install target/wheels/eggsec-*.whl
```

### Supported Platforms

| Platform | Architecture | Status |
|----------|-------------|--------|
| macOS | arm64 (Apple Silicon) | Supported (from source) |
| macOS | x86_64 | Supported (from source) |
| Linux | x86_64 (manylinux) | Supported (from source) |
| Linux | aarch64 (manylinux) | Supported (from source) |
| Windows | x86_64 | Not currently built |

Prebuilt wheels are **not yet available on PyPI**. Build from source using maturin.

### Included Features (default wheel)

- Port scanning with service detection
- Endpoint discovery and HTTP path probing
- Service fingerprinting and banner analysis
- Passive recon (DNS, TLS inspection, technology detection)
- WAF detection
- Findings and reporting (JSON, Markdown)
- Sync and async APIs
- Scope enforcement
- Policy, configuration, and execution context (Milestone B)

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
waf = eggsec.detect_waf("https://example.com", scope)
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
| `EggsecConfig` | Full configuration model (load, save, validate) |
| `LoadedScope` | Enriched scope with source tracking and validation |
| `OperationRegistry` | Operation metadata discovery (all operations, find by ID) |
| `EnforcementContext` | Policy evaluation gate (manual, MCP, agent, CI surfaces) |
| `ExecutionPolicy` | Risk-level policy configuration |
| `ExecutionSurface` | Execution surface identification (CLI, TUI, MCP, agent, etc.) |
| `PreflightResult` | Pre-dispatch policy preview |
| `EnforcementAuditEvent` | Audit trail for enforcement decisions |

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
| `validate_waf()` / `async_validate_waf()` | WAF bypass validation (requires scope) |
| `fuzz_http()` / `async_fuzz_http()` | HTTP fuzzing (requires scope) |
| `load_test_http()` / `async_load_test_http()` | HTTP load testing (requires scope) |
| `features()` | Available feature flags |
| `has_feature()` | Check a feature flag |
| `build_info()` | Build metadata |
| `preflight_operation()` | Pre-dispatch policy preview |
| `validate_scope()` | Scope validation |
| `audit_event_from_enforcement()` | Create audit event from enforcement outcome |
| `audit_event_from_preflight()` | Create audit event from preflight result |

### Policy, Configuration & Execution Context

Milestone B adds Python bindings for the engine's enforcement model, configuration system, and operation metadata registry. These are always available (no feature flags required).

| Module | Key Types |
|--------|-----------|
| `config_model` | `EggsecConfig`, `SensitiveString`, `HttpConfig`, `ScanConfig`, `OutputConfig`, `ReconConfig`, `AlertChannelConfig` |
| `scope_eval` | `LoadedScope`, `ScopeSource`, `ScopeRule`, `ScopeValidation`, `validate_scope()` |
| `operation_metadata` | `OperationRegistry`, `OperationMetadataView`, `OperationDescriptor`, `OperationRisk`, `Capability` |
| `execution_context` | `EnforcementContext`, `ExecutionSurface`, `ExecutionProfile`, `PolicyDecision`, `ApprovedOperation` |
| `authorization` | `ExecutionPolicy`, `ManualOverride` |
| `preflight` | `PreflightResult`, `preflight_operation()`, `preflight_with_descriptor()` |
| `audit` | `EnforcementAuditEvent`, `AuditOutcome`, `ManualOverrideAudit`, `ScopeAudit` |

#### Quick example: enforcement workflow

```python
from eggsec import (
    EnforcementContext, ExecutionPolicy, ExecutionSurface,
    OperationRegistry, LoadedScope, ManualOverride,
)

# 1. Load scope and policy
scope = LoadedScope.default_empty()
policy = ExecutionPolicy.default()

# 2. Create enforcement context for a CLI manual session
ctx = EnforcementContext.manual_permissive(policy, scope)

# 3. Look up an operation
op = OperationRegistry.find("port_scan")

# 4. Build a descriptor for a specific target
desc = op.descriptor_for_target("example.com")

# 5. Evaluate — preview the decision
outcome = ctx.evaluate(desc)
print(outcome.outcome_type)     # "allow" or "confirm"
print(outcome.is_allowed)       # True

# 6. Approve (generates audit token)
approved = ctx.approve(ExecutionSurface.CLI_MANUAL, desc)
print(approved.audit_event_id)  # audit trail identifier
```

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
