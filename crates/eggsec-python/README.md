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
- Consolidated reconnaissance (Milestone C)
- GraphQL security assessment (Milestone C)
- OAuth/OIDC security assessment (Milestone C)
- Authentication security assessment (Milestone C)
- NSE script metadata and sandbox policy inspection (Milestone D)
- Packet filter and flow record types (Milestone D)
- Traceroute API (Milestone D)
- Interception proxy config and captured exchanges (Milestone D)
- Mobile device listing and dynamic analysis config (Milestone D)
- Daemon capabilities, task handles, and session summaries (Milestone D)
- Database driver enumeration, capability descriptors, and credential providers (Milestone D)
- Context manager support for session-oriented types (Milestone D)
- Typed findings schema with confidence, evidence kinds, and versioned findings (Milestone E)
- Artifact storage and reference tracking (Milestone E)
- CVSS scoring, vulnerability records, and remediation tracking (Milestone E)
- Finding workflow with states, transitions, and suppression (Milestone E)
- Finding repository with assessments (Milestone E)
- Finding correlation, diffing, and baseline comparison (Milestone E)
- Finding reporting with severity summaries and report envelopes (Milestone E)
- Compliance mapping and reporting (feature: `compliance`) (Milestone E)
- External integrations with publication policies (Milestone E)
- Schema versioning and finding migration (Milestone E)

### Not Included (default wheel)

The following require building from source with feature flags:

- Nmap NSE/Lua script execution (feature: `nse`)
- Live packet capture sessions (feature: `packet-inspection`)
- Raw packet crafting and transmission (feature: `packet-inspection`)
- Stress testing / DoS simulation (feature: `stress-testing`)
- Web proxy MITM interception (feature: `web-proxy`)
- Mobile dynamic analysis with ADB/Frida (feature: `mobile-dynamic`)
- Headless browser automation (feature: `headless-browser`)
- Advanced vulnerability hunting (feature: `advanced-hunting`)
- Database pentest execution (feature: `db-pentest`)
- Daemon client connections (feature: `daemon-client`)
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
| `ConsolidatedReconConfig` | Config for consolidated recon (toggle modules) |
| `ReconModuleResult` | Single module result from consolidated recon |
| `ConsolidatedReconReport` | Aggregated consolidated recon report |
| `GraphQLVulnerability` | GraphQL vulnerability enum |
| `GraphQLTestResult` | GraphQL test result |
| `GraphQLSchema` | GraphQL introspection schema |
| `GraphQLTestConfig` | GraphQL test configuration |
| `OAuthVulnerability` | OAuth vulnerability enum |
| `OAuthEndpoint` | Discovered OAuth/OIDC endpoint |
| `OAuthTestConfig` | OAuth test configuration |
| `AuthTestType` | Authentication test type enum |
| `AuthFinding` | Authentication finding |
| `AuthTestConfig` | Authentication test configuration |
| `AuthTestReport` | Aggregated auth test report |
| `BrowserTestConfig` | Browser test configuration (feature-gated) |
| `BrowserTestReport` | Browser scan report (feature-gated) |
| `DomXssFinding` | DOM XSS finding (feature-gated) |
| `SpaRoute` | SPA route discovery (feature-gated) |
| `ClientIssue` | Client-side security issue (feature-gated) |
| `HuntTestConfig` | Hunt test configuration (feature-gated) |
| `HuntReport` | Advanced hunt report (feature-gated) |
| `AttackChain` | Multi-step attack chain (feature-gated) |
| `BusinessLogicFlaw` | Business logic flaw (feature-gated) |
| `RaceCondition` | Race condition finding (feature-gated) |
| `AuthzBypass` | Authorization bypass (feature-gated) |
| `SessionIssue` | Session security issue (feature-gated) |
| `NseScriptMetadata` | NSE script metadata (feature: `nse`) |
| `NseSandboxPolicy` | NSE sandbox configuration (feature: `nse`) |
| `NseTargetContext` | NSE target context (feature: `nse`) |
| `PacketFilter` | Packet capture filter (feature: `packet-inspection`) |
| `FlowRecord` | Network flow record (feature: `packet-inspection`) |
| `LiveCaptureResult` | Live capture result (feature: `packet-inspection`) |
| `TracerouteConfig` | Traceroute configuration (feature: `packet-inspection`) |
| `TracerouteHop` | Single traceroute hop (feature: `packet-inspection`) |
| `TracerouteResult` | Traceroute result (feature: `packet-inspection`) |
| `InterceptConfig` | Intercept proxy config (feature: `web-proxy`) |
| `CapturedExchange` | Captured HTTP exchange (feature: `web-proxy`) |
| `InterceptSessionResult` | Intercept session result (feature: `web-proxy`) |
| `MobileDevice` | Connected mobile device (feature: `mobile`) |
| `DynamicMobileConfig` | Dynamic analysis config (feature: `mobile`) |
| `DynamicMobileReport` | Dynamic analysis report (feature: `mobile`) |
| `DaemonCapabilities` | Daemon transport capabilities (feature: `daemon-client`) |
| `TaskHandle` | Submitted task handle (feature: `daemon-client`) |
| `TaskStatus` | Task execution status (feature: `daemon-client`) |
| `DaemonEvent` | Daemon event stream entry (feature: `daemon-client`) |
| `SessionSummary` | Session summary (feature: `daemon-client`) |
| `TransportMetadata` | Transport metadata (feature: `daemon-client`) |
| `DbDriverInfo` | Database driver info (feature: `db-pentest`) |
| `DbCapability` | Database capability descriptor (feature: `db-pentest`) |
| `DbCredentialProvider` | Credential provider (feature: `db-pentest`) |
| `DbSessionConfig` | Database session config (feature: `db-pentest`) |
| `Confidence` | Finding confidence level enum |
| `FindingType` | Finding type classification enum |
| `EvidenceKind` | Evidence kind enum (Screenshot, Log, PacketCapture, etc.) |
| `AffectedAsset` | Affected asset descriptor |
| `FindingLocation` | Location of finding within target |
| `VersionedEvidence` | Evidence with schema version |
| `VersionedFinding` | Finding with schema version |
| `MilestoneArtifact` | Stored artifact (renamed from Artifact to avoid collision) |
| `ArtifactReference` | Reference to a stored artifact |
| `ArtifactStore` | Artifact storage manager |
| `CvssScore` | CVSS v3.1 score components |
| `VulnerabilityRecord` | CVE/vulnerability reference record |
| `RemediationRecord` | Remediation guidance record |
| `FindingState` | Workflow state for a finding |
| `WorkflowTransition` | State transition event |
| `Suppression` | Finding suppression rule |
| `FindingWorkflow` | Workflow engine for findings |
| `FindingRepository` | Persistence layer for findings |
| `Assessment` | Assessment session record |
| `AssessmentRepository` | Persistence layer for assessments |
| `FindingCorrelation` | Correlation between related findings |
| `FindingDiff` | Diff between two findings |
| `AssessmentDiff` | Diff between two assessments |
| `BaselineComparator` | Baseline comparison engine |
| `FindingReporter` | Report generator for findings |
| `SeveritySummary` | Aggregated severity counts |
| `ReportEnvelope` | Report metadata envelope |
| `ComplianceFramework` | Compliance framework descriptor (feature: `compliance`) |
| `ComplianceControl` | Control within a framework (feature: `compliance`) |
| `ComplianceMapping` | Mapping between finding and control (feature: `compliance`) |
| `ComplianceResult` | Compliance evaluation result (feature: `compliance`) |
| `ControlAssessment` | Assessment of a specific control (feature: `compliance`) |
| `ComplianceReport` | Aggregated compliance report (feature: `compliance`) |
| `ComplianceMapper` | Maps findings to compliance controls (feature: `compliance`) |
| `IntegrationType` | External integration type enum |
| `PublicationRecord` | Record of a published finding |
| `RetryPolicy` | Retry configuration for publications |
| `PublicationPolicy` | Policy governing external publication |
| `ExternalIntegration` | External service integration manager |
| `SchemaVersion` | Finding schema version descriptor |
| `MigrationResult` | Result of a schema migration |
| `FindingMigration` | Finding schema migration engine |

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
| `run_consolidated_recon()` / `async_run_consolidated_recon()` | Consolidated multi-module reconnaissance |
| `graphql_test()` / `async_graphql_test()` | GraphQL security assessment |
| `oauth_discover_endpoints()` | Discover OAuth/OIDC endpoints |
| `oauth_test()` / `async_oauth_test()` | OAuth/OIDC security assessment |
| `auth_test()` / `async_auth_test()` | Authentication security assessment |
| `browser_test()` / `async_browser_test()` | Headless browser assessment (feature-gated) |
| `hunt_test()` / `async_hunt_test()` | Advanced vulnerability hunting (feature-gated) |
| `nse_list_scripts()` | List available NSE scripts (feature: `nse`) |
| `nse_get_script_metadata()` | Get NSE script metadata (feature: `nse`) |
| `run_traceroute()` / `async_run_traceroute()` | Traceroute (feature: `packet-inspection`) |
| `traceroute()` / `async_traceroute()` | Traceroute shorthand (feature: `packet-inspection`) |
| `list_mobile_devices()` | List connected mobile devices (feature: `mobile`) |
| `dynamic_mobile_analysis()` | Dynamic mobile analysis (feature: `mobile`) |
| `db_list_drivers()` | List available database drivers (feature: `db-pentest`) |
| `db_get_capabilities()` | Get DB driver capabilities (feature: `db-pentest`) |
| `db_run_with_config()` | Run DB pentest with config (feature: `db-pentest`) |

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
