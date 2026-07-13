# eggsec

Python bindings for the [Eggsec](https://github.com/sugarwookie/eggsec) security assessment engine.

## Status

**Scoped pre-1.0 release candidate** — Not yet published to PyPI. The
stable-core compatibility boundary is the ten operations listed in
[`docs/python/domain-maturity.md`](../../docs/python/domain-maturity.md).
See `RELEASE_CHECKLIST.md` for publication gates and
`docs/python/README_1_0_CHECKLIST.md` for the remaining 1.0 readiness work.

The stable guarantee covers local `Engine` and `AsyncEngine` execution only.
The optional daemon client is retained for integration work but is explicitly
provisional until daemon request/result parity, reconnect/replay, and
cancellation semantics are closed in a follow-up milestone.

### Stability Classifications

Operations and types are classified according to WS9 criteria:

| Level | Criteria | Examples |
|-------|----------|----------|
| **Stable-core** | Canonical registry, mandatory policy gate, typed payload, structured error, audit decision, and sync/async contract coverage | the ten operations in `domain-maturity.md`, plus their core DTOs |
| **Provisional** | Public API shape is useful, but common-engine parity, deterministic fixtures, or transport/schema coverage is incomplete | consolidated recon, GraphQL, OAuth, auth, database, NSE, daemon, pipeline/configuration surfaces |
| **Experimental** | Platform-sensitive, hazardous, provider-dependent, or subject to substantial change | wireless, evasion, postex, C2, browser, mobile dynamic, proxy, packet, distributed, and AI domains |
| **Internal** | No compatibility guarantee, not top-level exported | `deprecated_warning` |

Use `eggsec.api_surface()` to inspect the stability of any exported name at runtime.

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

For release validation, run `bash scripts/validate_python_release_candidate.sh`
from the repository root. It exercises loopback TCP/HTTP/TLS fixtures,
checkpoint persistence, wheel smoke tests, and architecture guards.

### Included Features (default wheel)

- Port scanning with service detection
- Endpoint discovery and HTTP path probing
- Service fingerprinting and banner analysis
- Passive recon (DNS, TLS inspection, technology detection)
- WAF detection
- Findings and reporting (JSON, Markdown)
- Sync and async APIs
- Scope enforcement
- Mandatory policy gate and structured dispatch audit records for stable-core operations
- Versioned `OperationError` payloads with compatibility `error_message`
- Versioned event envelopes with monotonic sequence numbers
- Backpressure delivery statistics with reliable lifecycle-event handling
- Domain maturity introspection via `domain_maturity()`
- Policy, configuration, and execution context (provisional until common-engine parity closes)
- Consolidated reconnaissance, GraphQL, OAuth/OIDC, and authentication assessment (provisional)
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
- Domain registry and operation introspection (Milestone G)
- Versioned event protocol with typed payloads (Milestone G)
- Callback/sink contracts: AuditSink, FindingSink, etc. (Milestone G)
- Python-native ergonomics: pathlib, datetime, hash/eq, context managers, pickle (Milestone G)
- Binary buffer protocol: BinaryBuffer, LazyArtifact, PaginatedResults (Milestone G)
- API surface introspection and feature matrix (Milestone G)
- Performance benchmarks and regression gates (Milestone G)
- 1.0 readiness checklist and stability classifications (Milestone G)

The default wheel is a scoped stable-core release candidate, not a claim that
all importable modules are stable. Use `eggsec.api_surface()` and
`eggsec.domain_maturity()` before selecting a domain for compatibility-sensitive
automation.

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
| `OperationError` | Versioned structured failure payload |
| `DispatchAuditEvent` | Stable-core dispatch decision record |
| `EventDeliveryStats` | Event delivery and drop counters |
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

| Function | Stability | Description |
|----------|-----------|-------------|
| `scan_ports()` / `async_scan_ports()` | stable | Port scanning |
| `scan_endpoints()` / `async_scan_endpoints()` | stable | Endpoint discovery |
| `fingerprint_services()` / `async_fingerprint_services()` | stable | Service fingerprinting |
| `recon_dns()` / `async_recon_dns()` | stable | DNS reconnaissance |
| `inspect_tls()` / `async_inspect_tls()` | stable | TLS certificate inspection |
| `detect_technology()` / `async_detect_technology()` | stable | Technology stack detection |
| `detect_waf()` / `async_detect_waf()` | stable | WAF detection |
| `validate_waf()` / `async_validate_waf()` | stable | WAF bypass validation (requires scope) |
| `fuzz_http()` / `async_fuzz_http()` | stable | HTTP fuzzing (requires scope) |
| `load_test_http()` / `async_load_test_http()` | stable | HTTP load testing (requires scope) |
| `features()` | stable | Available feature flags |
| `has_feature()` | stable | Check a feature flag |
| `build_info()` | stable | Build metadata |
| `preflight_operation()` | stable | Pre-dispatch policy preview |
| `validate_scope()` | stable | Scope validation |
| `audit_event_from_enforcement()` | stable | Create audit event from enforcement outcome |
| `audit_event_from_preflight()` | stable | Create audit event from preflight result |
| `run_consolidated_recon()` / `async_run_consolidated_recon()` | provisional | Consolidated multi-module reconnaissance |
| `graphql_test()` / `async_graphql_test()` | provisional | GraphQL security assessment |
| `oauth_discover_endpoints()` | provisional | Discover OAuth/OIDC endpoints |
| `oauth_test()` / `async_oauth_test()` | provisional | OAuth/OIDC security assessment |
| `auth_test()` / `async_auth_test()` | provisional | Authentication security assessment |
| `browser_test()` / `async_browser_test()` | experimental | Headless browser assessment (feature-gated) |
| `hunt_test()` / `async_hunt_test()` | experimental | Advanced vulnerability hunting (feature-gated) |
| `nse_list_scripts()` | provisional | List available NSE scripts (feature: `nse`) |
| `nse_get_script_metadata()` | provisional | Get NSE script metadata (feature: `nse`) |
| `run_traceroute()` / `async_run_traceroute()` | provisional | Traceroute (feature: `packet-inspection`) |
| `traceroute()` | provisional | Traceroute shorthand (feature: `packet-inspection`) |
| `list_mobile_devices()` | experimental | List connected mobile devices (feature: `mobile`) |
| `dynamic_mobile_analysis()` | experimental | Dynamic mobile analysis (feature: `mobile`) |
| `db_list_drivers()` | provisional | List available database drivers (feature: `db-pentest`) |
| `db_get_capabilities()` | provisional | Get DB driver capabilities (feature: `db-pentest`) |
| `db_run_with_config()` | provisional | Run DB pentest with config (feature: `db-pentest`) |
| `wireless_scan()` / `async_wireless_scan()` | experimental | WiFi scanning (feature: `wireless`) |
| `evasion_scan()` / `async_evasion_scan()` | experimental | Evasion detection (feature: `evasion`) |
| `postex_scan()` / `async_postex_scan()` | experimental | Post-exploitation (feature: `postex`) |
| `c2_scan()` / `async_c2_scan()` | experimental | C2 simulation (feature: `c2`) |
| `ai_analyze_finding()` / `async_ai_analyze_finding()` | experimental | AI finding analysis (feature: `ai-integration`) |

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

## Engine API Documentation

### Accessing Operation Results

Every `Engine.run_*()` method and convenience function returns an `OperationResult` with a typed payload:

```python
from eggsec import Engine, Scope, PortScanRequest

scope = Scope.allow_hosts(["127.0.0.1"])
engine = Engine(scope)

# Via engine method
result = engine.run_port_scan(PortScanRequest("127.0.0.1", ports="22,80,443"))
if result.status.name() == "Completed":
    # Access the typed payload directly
    payload = result.payload  # PortScanResult
    for port in payload.open_ports:
        print(f"  {port.port}: {port.service}")

# Via convenience function
result = eggsec.scan_ports("127.0.0.1", [22, 80, 443], scope)
for port in result.open_ports:
    print(f"  {port.port}: {port.service}")
```

### Canonical Operation IDs

Each operation has a canonical ID used by `OperationRegistry` and the enforcement model:

| Operation ID | Python Function | Request Type | Result Type |
|-------------|-----------------|--------------|-------------|
| `scan-ports` | `scan_ports()` | `PortScanRequest` | `PortScanResult` |
| `scan-endpoints` | `scan_endpoints()` | `EndpointScanRequest` | `EndpointScanResult` |
| `fingerprint-services` | `fingerprint_services()` | `FingerprintRequest` | `FingerprintScanResult` |
| `recon` | `recon_dns()` | `ReconDnsRequest` | `DnsRecordSet` |
| `tls-inspect` | `inspect_tls()` | `TlsInspectRequest` | `TlsInspectionResult` |
| `tech-detect` | `detect_technology()` | `TechDetectRequest` | `TechDetectionResult` |
| `waf-detect` | `detect_waf()` | `WafDetectRequest` | `WafDetectionResult` |
| `waf-validate` | `validate_waf()` | `WafValidateRequest` | `WafScanResult` |
| `http-fuzz` | `fuzz_http()` | `FuzzRequest` | `FuzzSession` |
| `load-test` | `load_test_http()` | `LoadTestRequest` | `LoadTestResult` |

Look up operation metadata at runtime:

```python
from eggsec import OperationRegistry

op = OperationRegistry.find("scan-ports")
print(op.operation_id)         # "scan-ports"
print(op.default_risk.name)    # "safe-active"
print(op.supported_surfaces)   # ["cli", "tui", "mcp", "rest"]
```

### Execution Context and Preflight

Before dispatching an operation, use the preflight path to preview the policy decision:

```python
from eggsec import (
    EnforcementContext, ExecutionPolicy, ExecutionSurface,
    LoadedScope, OperationRegistry,
)

scope = LoadedScope.default_empty()
policy = ExecutionPolicy.default()

# Create enforcement context
ctx = EnforcementContext.manual_permissive(policy, scope)

# Look up operation and build descriptor
op = OperationRegistry.find("scan-ports")
desc = op.descriptor_for_target("example.com")

# Preview the decision (no side effects)
outcome = ctx.evaluate(desc)
print(outcome.outcome_type)  # "allow", "confirm", or "deny"

# Approve (generates audit token)
approved = ctx.approve(ExecutionSurface.CLI_MANUAL, desc)
```

### Event Guarantees and Progress

The event protocol provides typed, versioned events:

```python
from eggsec import EventStream, EventEnvelope

# Events are delivered in monotonic sequence order
# Each envelope contains: event_type, payload, sequence_id, correlation_id

# Guaranteed event types:
# - pipeline.started / pipeline.completed / pipeline.failure
# - step.started / step.completed / step.failed
# - finding.discovered
# - artifact.created
# - cancellation.requested
# - progress.updated (where supported)

# Progress events are NOT guaranteed for all operations.
# Operations that support progress: scan-ports, load-test, http-fuzz
# Operations without progress: recon-dns, tls-inspect, tech-detect
```

### Cancellation and Partial Results

Pipelines and engine operations support cooperative cancellation:

```python
from eggsec import Pipeline, CancellationToken, Engine, Scope

engine = Engine(Scope.allow_hosts(["127.0.0.1"]))
pipeline = Pipeline("my-scan")

# Set a cancellation token
token = CancellationToken()
pipeline.set_cancel_token(token)

# Add steps...

# Run in background and cancel when needed
result = pipeline.run(engine)

# Or cancel from another thread
token.cancel("User requested abort")

# Partial results are preserved in PipelineResult.step_results
# even when cancellation occurs mid-pipeline
```

### Feature Compiled vs Runtime-Ready

Some features are compiled (available when the feature flag is enabled) while others are runtime-ready (fully validated and tested):

```python
import eggsec

# Check what's compiled in
features = eggsec.features()
print(features["db-pentest"])  # True if compiled with feature

# Check stability via api_surface
surface = eggsec.api_surface()
print(surface["db_probe"]["stability"])  # "provisional"
print(surface["scan_ports"]["stability"])  # "stable"
```

### Migration: Convenience Functions to Engine Requests

Convenience functions wrap the engine with simplified signatures. For full control, use the engine directly:

```python
# Before: convenience function (simplified signature)
result = eggsec.scan_ports("127.0.0.1", [80, 443], scope)

# After: engine request (full control over all parameters)
from eggsec import Engine, Scope, PortScanRequest

scope = Scope.allow_hosts(["127.0.0.1"])
engine = Engine(scope, mode="manual", concurrency=200, timeout_ms=10000)

request = PortScanRequest(
    "127.0.0.1",
    ports="1-1024",
    timeout_ms=30000,
)
result = engine.run_port_scan(request)

# Or use the generic dispatch
from eggsec import OperationRequest

request = OperationRequest(
    operation="scan_ports",
    target="127.0.0.1",
    timeout_ms=30000,
    metadata={"ports": "1-1024"},
)
result = engine.run(request)
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
- [Events](../../docs/python/events.md)
- [Callbacks](../../docs/python/callbacks.md)
- [Versioning](../../docs/python/versioning.md)
- [Namespaces](../../docs/python/namespace.md)
- [Stability Classifications](../../docs/python/STABILITY_CLASSIFICATIONS.md)
- [1.0 Readiness Checklist](../../docs/python/README_1_0_CHECKLIST.md)

## Safety

All operations enforce authorization scope. Scans only target hosts and ports explicitly allowed in the scope configuration. See [Scope & Safety](../../docs/python/scope-and-safety.md) for details.

## Daemon Client (feature: `daemon-client`)

The Python daemon client delegates to the Rust daemon protocol and is retained
for integration work, but it is **provisional** for the scoped 0.x release.
Wire compatibility alone is not the stable contract: request/result parity,
reconnect and replay behavior, cancellation, checkpoint portability, and
event ordering still require a dedicated follow-up gate.

**Wire format:** JSON lines over Unix socket. Protocol version: `DAEMON_PROTOCOL_VERSION = 1`.

**Task kinds** use `eggsec_runtime::TaskKind` serde format: `{"kind": "PortScan", "params": {"target": "10.0.0.1"}}`.

### Session lifecycle

```python
import eggsec

# Connect to daemon
client = eggsec.daemon_connect("/tmp/eggsec.sock")

# Declare client type
eggsec.async_daemon_declare_client(client, kind="cli", label="my-script")

# Create session
resp = eggsec.async_daemon_create_session(client, surface="cli_manual")
session_id = resp.message.split("=", 1)[1]  # parse session_id from response

# Submit a task
task_json = '{"kind": "PortScan", "params": {"target": "10.0.0.1", "ports": [22, 80]}}'
resp = eggsec.async_daemon_submit_task(client, session_id, task_json)

# Manage tasks
eggsec.async_daemon_cancel_active(client, session_id)
eggsec.async_daemon_approve_policy(client, session_id, task_id, approved=True)

# Persisted sessions
eggsec.async_daemon_list_persisted_sessions(client)
eggsec.async_daemon_get_persisted_snapshot(client, session_id)

# Cleanup
eggsec.async_daemon_close_session(client, session_id)
client.close()
```

### Simplified DTOs

Python DTOs (`DaemonResponsePy`, `SessionSummaryPy`, etc.) are **API convenience** wrappers, not wire format changes. The actual JSON on the wire matches the Rust daemon protocol exactly.

## License

MIT
