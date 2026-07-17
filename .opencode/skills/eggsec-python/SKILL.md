---
name: eggsec-python
description: "Python bindings for Eggsec via PyO3/maturin - use when working with Python integration, maturin builds, type stubs, or Python-side API usage."
---

# Eggsec Python Bindings Skill

Python bindings for the Eggsec security assessment engine via PyO3/maturin.

## Overview

The `eggsec-python` crate provides Python-native bindings over the Rust engine. It is a host-language binding (not an internal plugin runtime) that wraps `eggsec` and `eggsec-core` via PyO3. The GIL is released during network I/O.

**Status**: Scoped pre-1.0 release candidate (0.1.0). The stable-core
boundary is the twenty-two-operation `StableOperation` registry. Stable-core paths
share the mandatory policy/audit gate, typed payloads, `OperationError`, and
governed event delivery. Milestone C/E/G and feature-gated domains remain
provisional or experimental until they satisfy the graduation checklist in
`docs/python/domain-maturity.md`.
The release guarantee is local `Engine`/`AsyncEngine` only; daemon-client
execution remains provisional pending transport parity.

Release 1/2 closure pass completed 2026-07-14 (1977 passed, 89 skipped).
Release 1-4 operational correction pass completed 2026-07-15 (2969 passed, 0 failed).

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
│   ├── runtime_async.rs     # Async runtime (PyFuture, shared OnceLock<Runtime>)
│   ├── config_model.rs      # SensitiveString, EggsecConfig, config sub-models
│   ├── scope_eval.rs        # LoadedScope, ScopeSource, ScopeRule, validate_scope()
│   ├── operation_metadata.rs # OperationRegistry, OperationDescriptor, OperationRisk, Capability
│   ├── execution_context.rs # EnforcementContext, ExecutionSurface, ExecutionProfile
│   ├── authorization.rs     # ExecutionPolicy, ManualOverride
│   ├── preflight.rs         # PreflightResult, preflight_operation()
│   ├── audit.rs             # EnforcementAuditEvent, AuditOutcome, emit_audit_event()
│   ├── consolidated_recon.rs # ConsolidatedReconConfig, run_consolidated_recon
│   ├── graphql.rs           # GraphQLFuzzer, GraphQLTestConfig, graphql_test
│   ├── oauth.rs             # OAuthFuzzer, OAuthTestConfig, oauth_test
│   ├── auth_assess.rs       # AuthTestConfig, AuthTestReport, auth_test
│   ├── browser_assess.rs    # BrowserTestConfig, BrowserTestReport, browser_test (feature-gated)
│   ├── hunt.rs              # HuntTestConfig, HuntReport, hunt_test (feature-gated)
│   ├── finding_schema.rs    # Confidence, FindingType, VersionedFinding, VersionedEvidence (Milestone E)
│   ├── artifact.rs          # MilestoneArtifact, ArtifactReference, ArtifactStore (Milestone E)
│   ├── vuln_record.rs       # CvssScore, VulnerabilityRecord, RemediationRecord (Milestone E)
│   ├── workflow.rs          # FindingState, WorkflowTransition, Suppression, FindingWorkflow (Milestone E)
│   ├── repository.rs        # FindingRepository, Assessment, AssessmentRepository (Milestone E)
│   ├── correlation.rs       # FindingCorrelation, FindingDiff, AssessmentDiff, BaselineComparator (Milestone E)
│   ├── reporting.rs         # FindingReporter, SeveritySummary, ReportEnvelope (Milestone E)
│   ├── compliance.rs        # ComplianceFramework, ComplianceControl, ComplianceMapper (feature-gated, Milestone E)
│   ├── integration.rs       # IntegrationType, PublicationRecord, ExternalIntegration (Milestone E)
│   └── migration.rs         # SchemaVersion, MigrationResult, FindingMigration (Milestone E)
├── python/
│   └── eggsec/
│       ├── __init__.py      # Re-exports all public API (stable core only)
│       ├── _feature_guard.py # Feature availability detection (Phase C)
│       ├── net/             # Network types: Target, TcpSession, HttpClient, etc. (provisional)
│       ├── sessions/        # Session types: MobileSession, BrowserSession, etc. (provisional)
│       ├── storage/         # Repository types: FindingRepository, ArtifactStore, etc. (provisional)
│       ├── reporting/       # Reporter types: FindingReporter, StreamingReporter, etc. (provisional)
│       ├── daemon/          # Daemon client and parity types (provisional)
│       ├── experimental/    # Experimental types: wireless, evasion, postex, C2, hunt, AI, stress
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
    ├── test_policy_equivalence.py
    └── test_golden_contract.py  # Phase B+C architecture guard tests
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
| `headless-browser` | `headless-browser` | `headless-chrome` | Headless browser testing (DOM XSS, SPA routes) |
| `advanced-hunting` | `advanced-hunting` | none | Advanced vulnerability hunting (attack chains, business logic, race conditions) |
| `compliance` | `compliance` | none | Compliance mapping and reporting (OWASP, HIPAA, PCI, SOC2) |
| `daemon-client` | — | none | Daemon session access |
| `full-no-system` | — | none | Aggregate: `websocket`, `git-secrets`, `sbom`, `container` |

## Test Commands

```bash
# Python-side tests (run from the workspace root)
pytest crates/eggsec-python/tests/ crates/eggsec-python/python/tests/

# Rust-side tests
cargo test -p eggsec-python

# Policy equivalence tests (Milestone B)
pytest crates/eggsec-python/tests/test_policy_equivalence.py

# Release-closure validation from the workspace root
bash scripts/validate_python_release_candidate.sh
```

The release fixture suite covers all twenty-two stable operations using managed
loopback services and must not be converted into conditional skips. Set
`EGGSEC_ALLOW_LOOPBACK_FIXTURE=1` only for that explicit fixture harness or
installed-wheel smoke test. The normal resolver and policy gate remain
unchanged for callers. The first-release contract is local `Engine` and
`AsyncEngine`; daemon-client execution is provisional.

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
| `PipelineCheckpoint` | Versioned checkpoint with compatibility identity fields and redacted step results. |
| `CheckpointStore` | Atomic in-memory or file-backed checkpoint persistence. |
| `SensitiveString` | Zeroized secret wrapper. Use `SensitiveString.new("value")`, `.expose_secret()` to read. |
| `EnforcementAuditEvent` | Audit trail entry with event_id, timestamp, surface, outcome, scope info, policy hash. |
| `ScopeValidation` | Result of `validate_scope()` (valid, errors, warnings, target/exclusion counts). |
| `AlertChannelConfig` | Alert channel config. Use `.webhook()`, `.email()`, `.slack()`, `.pagerduty()` static constructors. |
| `ConsolidatedReconConfig` | Config for consolidated recon. Toggle modules: `run_dns`, `run_ssl`, `run_tech_detect`, etc. |
| `ReconModuleResult` | Single module result: `module`, `success`, `data_json`, `error`. |
| `ConsolidatedReconReport` | Aggregated recon report with per-module results. |
| `GraphQLVulnerability` | Enum: `Introspection`, `QueryInjection`, `DepthLimitBypass`, etc. |
| `GraphQLTestResult` | Single test result: `vulnerability`, `success`, `query`, `severity`. |
| `GraphQLType` | GraphQL schema type: `name`, `kind`, `fields`, `input_fields`. |
| `GraphQLField` | GraphQL field: `name`, `type_name`, `args`, `is_deprecated`. |
| `GraphQLArg` | GraphQL argument: `name`, `type_name`, `default_value`. |
| `GraphQLInputField` | GraphQL input field: `name`, `type_name`, `default_value`. |
| `GraphQLSchema` | Full introspection schema: `query_type`, `mutation_type`, `types`. |
| `GraphQLTestConfig` | Config for GraphQL tests: `enable_introspection`, `enable_depth_bypass`, etc. |
| `OAuthVulnerability` | Enum: `RedirectUriValidation`, `StateParameterMissing`, etc. |
| `OAuthEndpointKind` | Enum: `OidcDiscovery`, `Authorize`, `Token`, `UserInfo`, `Jwks`, `Revoke`. |
| `OAuthEndpoint` | Discovered endpoint: `url`, `kind`. |
| `OAuthTestResult` | Single test result: `vulnerability`, `success`, `endpoint`, `severity`. |
| `OAuthTestConfig` | Config for OAuth tests: `client_id`, `redirect_uri`, `issuer_url`, etc. |
| `AuthTestType` | Enum: `BruteForce`, `CredentialStuffing`, `AccountLockout`, etc. |
| `AuthFinding` | Auth finding: `test_type`, `severity`, `title`, `description`, `recommendation`. |
| `AuthTestConfig` | Config for auth tests: `max_attempts`, `concurrency`, `usernames`, `passwords`. |
| `AuthTestReport` | Aggregated auth report with per-test-type results and findings. |
| `XssSource` | Enum: `Url`, `Fragment`, `PostMessage`, `Storage`, etc. |
| `XssSink` | Enum: `InnerHtml`, `Eval`, `DocumentWrite`, `Location`, etc. |
| `DomXssFinding` | DOM XSS finding: `id`, `source`, `sink`, `severity`, `evidence`. |
| `DiscoveryMethod` | Enum: `LinkExtraction`, `ApiDiscovery`, `RouteBruteForce`, `HistoryApi`. |
| `SpaRoute` | SPA route: `path`, `method`, `parameters`, `discovered_via`. |
| `ClientIssueType` | Enum: `InsecureStorage`, `WeakCrypto`, `CorsMisconfig`, etc. |
| `ClientIssue` | Client-side issue: `id`, `issue_type`, `severity`, `location`, `description`. |
| `BrowserTestConfig` | Config for browser tests: `check_dom_xss`, `discover_spa_routes`, etc. |
| `BrowserTestReport` | Browser scan report with DOM XSS, SPA routes, client issues. |
| `ChainType` | Enum: `AuthenticationBypass`, `PrivilegeEscalation`, `DataExfiltration`, etc. |
| `ChainStep` | Attack chain step: `order`, `description`, `evidence`, `severity`. |
| `AttackChain` | Multi-step attack chain: `id`, `name`, `chain_type`, `steps`, `severity`. |
| `FlawType` | Enum: `BusinessLogicBypass`, `RaceCondition`, `InputValidation`, etc. |
| `BusinessLogicFlaw` | Business logic flaw: `id`, `flaw_type`, `severity`, `evidence`. |
| `RaceType` | Enum: `DoubleSpend`, `TimeOfCheck`, `ConcurrentModification`, etc. |
| `RaceCondition` | Race condition finding: `id`, `race_type`, `severity`, `endpoint`. |
| `BypassType` | Enum: `VerticalPrivilegeEscalation`, `HorizontalPrivilegeEscalation`, etc. |
| `AuthzBypass` | Authorization bypass: `id`, `bypass_type`, `severity`, `endpoint`, `evidence`. |
| `SessionIssueType` | Enum: `Fixation`, `TokenLeakage`, `InsecureCookie`, etc. |
| `SessionIssue` | Session issue: `id`, `issue_type`, `severity`, `evidence`. |
| `HuntTestConfig` | Config for hunt: `check_attack_chains`, `check_business_logic`, etc. |
| `HuntReport` | Hunt report with chains, business logic, race, authz, session findings. |
| `Confidence` | Enum: `Certain`, `High`, `Medium`, `Low`, `None`. |
| `FindingType` | Enum: `Vulnerability`, `Misconfiguration`, `InformationLeak`, `PolicyViolation`, `Custom`. |
| `EvidenceKind` | Enum: `Screenshot`, `Log`, `PacketCapture`, `HttpRequestResponse`, `CommandLine`, `Artifact`, `Custom`. |
| `AffectedAsset` | Asset: `asset_type`, `identifier`, `details`. |
| `FindingLocation` | Location: `file_path`, `line`, `column`, `url`, `parameter`. |
| `VersionedEvidence` | Evidence with `schema_version`, `kind`, `data`, `collected_at`. |
| `VersionedFinding` | Finding with `schema_version`, `id`, `title`, `severity`, `confidence`, `finding_type`, `evidence`, `location`, `assets`, `remediation`. |
| `MilestoneArtifact` | Stored artifact: `id`, `name`, `kind`, `mime_type`, `size_bytes`, `content_hash`, `path`, `created_at`. |
| `ArtifactReference` | Reference: `artifact_id`, `finding_id`, `role`. |
| `ArtifactStore` | Artifact store: `store(artifact)`, `get(id)`, `list()`, `delete(id)`. |
| `CvssScore` | CVSS v3.1: `base_score`, `temporal_score`, `environmental_score`, `vector_string`. |
| `VulnerabilityRecord` | Vuln record: `cve_id`, `cvss`, `description`, `references`, `published_at`. |
| `RemediationRecord` | Remediation: `finding_id`, `summary`, `steps`, `references`, `effort`. |
| `FindingState` | Enum: `Open`, `Triaged`, `InProgress`, `Resolved`, `Dismissed`, `Reopened`. |
| `WorkflowTransition` | Transition: `from_state`, `to_state`, `actor`, `timestamp`, `reason`. |
| `Suppression` | Suppression: `finding_id`, `reason`, `expires_at`, `suppressed_by`. |
| `FindingWorkflow` | Workflow: `transition(finding_id, to_state, ...)`, `history(finding_id)`. |
| `FindingRepository` | Repository: `save(finding)`, `get(id)`, `query(filters)`, `count()`. |
| `Assessment` | Assessment: `id`, `name`, `target`, `started_at`, `completed_at`, `finding_ids`, `metadata`. |
| `AssessmentRepository` | Repository: `save(assessment)`, `get(id)`, `list()`. |
| `FindingCorrelation` | Correlation: `finding_ids`, `correlation_type`, `confidence`, `description`. |
| `FindingDiff` | Diff: `finding_id`, `added_fields`, `removed_fields`, `changed_fields`. |
| `AssessmentDiff` | Diff: `assessment_id`, `new_findings`, `resolved_findings`, `changed_findings`. |
| `BaselineComparator` | Comparator: `compare(baseline, current)`, `summary(diff)`. |
| `FindingReporter` | Reporter: `generate(findings, format)`, `write(path)`. |
| `SeveritySummary` | Summary: `critical`, `high`, `medium`, `low`, `info`, `total`. |
| `ReportEnvelope` | Envelope: `report_id`, `generated_at`, `schema_version`, `format`, `summary`, `finding_count`. |
| `ComplianceFramework` | Framework: `id`, `name`, `version`, `controls` (feature: `compliance`). |
| `ComplianceControl` | Control: `id`, `framework_id`, `title`, `description`, `severity` (feature: `compliance`). |
| `ComplianceMapping` | Mapping: `finding_id`, `control_id`, `match_type`, `confidence` (feature: `compliance`). |
| `ComplianceResult` | Result: `framework_id`, `compliant_count`, `non_compliant_count`, `mappings` (feature: `compliance`). |
| `ControlAssessment` | Assessment: `control_id`, `status`, `findings`, `evidence` (feature: `compliance`). |
| `ComplianceReport` | Report: `framework`, `results`, `control_assessments`, `generated_at` (feature: `compliance`). |
| `ComplianceMapper` | Mapper: `map_findings(findings, framework)`, `assess(findings, framework)` (feature: `compliance`). |
| `IntegrationType` | Enum: `Jira`, `GitHub`, `GitLab`, `Slack`, `Webhook`, `Custom`. |
| `PublicationRecord` | Record: `finding_id`, `integration_type`, `external_id`, `url`, `published_at`. |
| `RetryPolicy` | Policy: `max_retries`, `backoff_ms`, `timeout_ms`. |
| `PublicationPolicy` | Policy: `integration_type`, `auto_publish`, `retry`, `filter_severity`. |
| `ExternalIntegration` | Integration: `publish(finding)`, `list_publications()`, `status()`. |
| `SchemaVersion` | Version: `major`, `minor`, `patch`, `is_compatible(other)`. |
| `MigrationResult` | Result: `success`, `migrated_count`, `errors`, `warnings`. |
| `FindingMigration` | Migration: `migrate(findings, from_version, to_version)`, `register_transform(version, fn)`. |

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
| `run_consolidated_recon` / `async_run_consolidated_recon` | Both | Consolidated multi-module reconnaissance |
| `graphql_test` / `async_graphql_test` | Both | GraphQL security assessment (introspection, injection, batching) |
| `oauth_discover_endpoints` | Sync | Discover OAuth/OIDC endpoints from issuer URL |
| `oauth_test` / `async_oauth_test` | Both | OAuth/OIDC security assessment (redirect, state, scope, PKCE) |
| `auth_test` / `async_auth_test` | Both | Authentication security assessment (brute force, lockout, MFA, etc.) |
| `scan_git_secrets` / `async_scan_git_secrets` | Both | Git secrets scanning |
| `generate_sbom` / `async_generate_sbom` | Both | SBOM generation (CycloneDX, SPDX) |
| `nse_run` / `async_nse_run` | Both | Execute NSE scripts (feature: `nse`) |
| `db_probe` / `async_db_probe` | Both | Database security probe (feature: `db-pentest`) |
| `scan_docker_image` / `async_scan_docker_image` | Both | Docker image security scanning (feature: `container`) |
| `scan_kubernetes` / `async_scan_kubernetes` | Both | Kubernetes manifest scanning (feature: `container`) |
| `analyze_apk` / `async_analyze_apk` | Both | Android APK static analysis (feature: `mobile`) |
| `analyze_ipa` / `async_analyze_ipa` | Both | iOS IPA static analysis (feature: `mobile`) |
| `browser_test` / `async_browser_test` | Both | Headless browser assessment (DOM XSS, SPA, client checks) — feature-gated |
| `hunt_test` / `async_hunt_test` | Both | Advanced vulnerability hunting (chains, business logic, race, authz) — feature-gated |

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

### Consolidated Reconnaissance

```python
from eggsec import ConsolidatedReconConfig, run_consolidated_recon

# Run all available modules
config = ConsolidatedReconConfig()
report = run_consolidated_recon("example.com", config)
print(f"Completed {sum(1 for m in report.results if m.success)}/{len(report.results)} modules")

# Selective modules
config = ConsolidatedReconConfig(
    run_dns=True, run_ssl=True, run_tech_detect=True,
    run_subdomain=False, run_whois=False,
    run_cors=False, run_wayback=False,
    run_js_analysis=False, run_content=False, run_email=False,
)
report = run_consolidated_recon("example.com", config)
for module in report.results:
    if module.success:
        print(f"{module.module}: {module.data_json[:100]}")
```

### GraphQL Security Assessment

```python
from eggsec import GraphQLTestConfig, graphql_test

config = GraphQLTestConfig(
    endpoint="https://example.com/graphql",
    enable_introspection=True,
    enable_depth_bypass=True,
    enable_alias_overload=True,
    max_depth=5,
)
report = graphql_test("https://example.com", config)
for result in report.results:
    if result.success:
        print(f"[{result.severity}] {result.vulnerability}: {result.description}")
```

### OAuth/OIDC Assessment

```python
from eggsec import oauth_discover_endpoints, OAuthTestConfig, oauth_test

# Discover endpoints
endpoints = oauth_discover_endpoints("https://auth.example.com")
for ep in endpoints:
    print(f"{ep.kind}: {ep.url}")

# Run tests
config = OAuthTestConfig(
    client_id="test-client",
    redirect_uri="https://example.com/callback",
    issuer_url="https://auth.example.com",
    enable_redirect_test=True,
    enable_scope_test=True,
    enable_state_test=True,
)
report = oauth_test("https://auth.example.com", config)
for result in report.results:
    if result.success:
        print(f"[{result.severity}] {result.vulnerability}")
```

### Authentication Assessment

```python
from eggsec import AuthTestConfig, auth_test

config = AuthTestConfig(
    max_attempts=5,
    concurrency=10,
    timeout_secs=30,
    stop_on_lockout=True,
    usernames=["admin", "user"],
    passwords=["password123", "admin"],
)
report = auth_test("https://example.com", config)
for finding in report.findings:
    print(f"[{finding.severity}] {finding.title}: {finding.description}")
    print(f"  Recommendation: {finding.recommendation}")
```

### Git Secrets Scanning

```python
from eggsec import scan_git_secrets

result = scan_git_secrets("/path/to/repo")
for secret in result.secrets:
    print(f"[{secret.severity}] {secret.title}: {secret.file_path}:{secret.line}")
```

### SBOM Generation

```python
from eggsec import generate_sbom, SbomFormat

result = generate_sbom("/path/to/project", format=SbomFormat.CYCLONEDX)
print(f"Generated SBOM with {result.component_count} components")
```

### NSE Script Execution

```python
from eggsec import nse_run, NseRunRequest

request = NseRunRequest(
    scripts=["http-headers", "ssl-cert"],
    target="example.com",
    port=443,
)
report = nse_run(request)
for result in report.results:
    print(f"Script: {result.script}, Output: {result.output[:100]}")
```

### Database Probe

```python
from eggsec import db_probe, DbProbeRequest

request = DbProbeRequest(
    host="127.0.0.1",
    port=5432,
    database="labdb",
    user="labuser",
    checks=["auth", "config", "extensions"],
)
result = db_probe(request)
for finding in result.findings:
    print(f"[{finding.severity}] {finding.title}")
```

### Docker Image Scanning

```python
from eggsec import scan_docker_image

result = scan_docker_image("nginx:latest")
for vuln in result.vulnerabilities:
    print(f"[{vuln.severity}] {vuln.cve}: {vuln.description}")
```

### APK Analysis

```python
from eggsec import analyze_apk

result = analyze_apk("/path/to/app.apk")
print(f"Package: {result.package_name}")
print(f"Permissions: {len(result.permissions)}")
for finding in result.findings:
    print(f"[{finding.severity}] {finding.title}")
```

### Pipeline with Dependencies and Parallel Groups

```python
from eggsec import Pipeline, PipelineStep

pipeline = Pipeline("advanced-scan")

# Define steps with dependencies
step1 = PipelineStep("recon", operation="recon_dns", target="example.com")
step2 = PipelineStep("port-scan", operation="scan_ports", target="example.com")
step3 = PipelineStep("fingerprint", operation="fingerprint_services", target="example.com",
                     depends_on=["port-scan"])
step4 = PipelineStep("fuzz", operation="fuzz_http", target="https://example.com",
                     depends_on=["fingerprint"])

# Add parallel group (recon and port-scan run concurrently)
pipeline.add_step(step1)
pipeline.add_step(step2)
pipeline.add_step(step3)  # waits for step2
pipeline.add_step(step4)  # waits for step3

# Configure retry and failure policy
pipeline.set_retry_policy(max_retries=2, backoff_ms=1000)
pipeline.set_failure_policy("continue-on-error")  # or "fail-fast"

result = pipeline.run(engine)
```

### Headless Browser Assessment

```python
from eggsec import BrowserTestConfig, browser_test

config = BrowserTestConfig(
    check_dom_xss=True,
    discover_spa_routes=True,
    check_client_security=True,
    timeout_ms=30000,
)
report = browser_test("https://example.com", config)
for finding in report.dom_xss:
    print(f"DOM XSS: {finding.source} -> {finding.sink} [{finding.severity}]")
for route in report.spa_routes:
    print(f"SPA Route: {route.path} ({route.method})")
for issue in report.client_issues:
    print(f"Client Issue: {issue.issue_type} [{issue.severity}]")
```

### Advanced Vulnerability Hunting

```python
from eggsec import HuntTestConfig, hunt_test

config = HuntTestConfig(
    check_attack_chains=True,
    check_business_logic=True,
    check_race_conditions=True,
    check_authz_bypass=True,
    check_session=True,
    concurrency=10,
)
report = hunt_test("https://example.com", config)
for chain in report.attack_chains:
    print(f"Attack Chain: {chain.name} [{chain.severity}]")
    for step in chain.steps:
        print(f"  Step {step.order}: {step.description}")
for flaw in report.business_logic:
    print(f"Business Logic: {flaw.flaw_type} [{flaw.severity}]")
for race in report.race_conditions:
    print(f"Race Condition: {race.race_type} [{race.severity}]")
for bypass in report.authz_bypasses:
    print(f"AuthZ Bypass: {bypass.bypass_type} [{bypass.severity}]")
```

### Pipeline Features

The pipeline supports advanced orchestration:

- **Step dependencies**: declare prerequisite steps that must complete before a step runs
- **Parallel execution groups**: run independent steps concurrently
- **Retry policy**: configurable retry count and backoff for transient failures
- **Failure policy**: choose between `fail-fast` (abort on first failure) and `continue-on-error` (collect all results)

### Milestone E: Findings, Reporting, Storage, and Integrations

#### Creating Versioned Findings

```python
from eggsec import (
    Confidence, FindingType, EvidenceKind, Severity,
    VersionedFinding, VersionedEvidence, AffectedAsset, FindingLocation,
)

evidence = VersionedEvidence(
    kind=EvidenceKind.HTTP_REQUEST_RESPONSE,
    data="HTTP/1.1 200 OK\nX-Powered-By: PHP/7.4",
    collected_at="2026-07-10T12:00:00Z",
)

finding = VersionedFinding(
    title="Information Disclosure via X-Powered-By Header",
    severity=Severity.MEDIUM,
    confidence=Confidence.HIGH,
    finding_type=FindingType.INFORMATION_LEAK,
    evidence=[evidence],
    location=FindingLocation(url="https://example.com/"),
    assets=[AffectedAsset(asset_type="host", identifier="example.com")],
)
print(f"Finding: {finding.title} (confidence={finding.confidence})")
```

#### Artifact Storage

```python
from eggsec import ArtifactStore, MilestoneArtifact

store = ArtifactStore()

artifact = MilestoneArtifact(
    name="capture.pcap",
    mime_type="application/octet-stream",
    content=b"...",
)
stored = store.store(artifact)
print(f"Artifact ID: {stored.id}")

# Retrieve
retrieved = store.get(stored.id)
```

#### Finding Workflow

```python
from eggsec import FindingWorkflow, FindingState

workflow = FindingWorkflow()

# Transition a finding through its lifecycle
workflow.transition("find-001", FindingState.TRIAGED, actor="analyst", reason="Confirmed valid")
workflow.transition("find-001", FindingState.IN_PROGRESS, actor="analyst", reason="Working on fix")

# View history
for transition in workflow.history("find-001"):
    print(f"{transition.from_state} -> {transition.to_state} by {transition.actor}")
```

#### Finding Repository and Assessment

```python
from eggsec import FindingRepository, Assessment, AssessmentRepository

repo = FindingRepository()
assessment_repo = AssessmentRepository()

# Save findings
repo.save(finding)

# Create assessment
assessment = Assessment(
    name="Q3 2026 Pentest",
    target="example.com",
    finding_ids=[finding.id],
)
assessment_repo.save(assessment)
```

#### Baseline Comparison

```python
from eggsec import BaselineComparator

comparator = BaselineComparator()

# Compare two assessments
diff = comparator.compare(baseline_assessment, current_assessment)
print(f"New findings: {len(diff.new_findings)}")
print(f"Resolved: {len(diff.resolved_findings)}")
print(f"Changed: {len(diff.changed_findings)}")
```

#### Finding Reporting

```python
from eggsec import FindingReporter, SeveritySummary

reporter = FindingReporter()

# Generate report from findings
envelope = reporter.generate(findings, format="json")
print(f"Report {envelope.report_id}: {envelope.finding_count} findings")
print(f"Generated at: {envelope.generated_at}")

# Write to file
reporter.write(findings, "output/report.json", format="json")
```

#### Compliance Mapping (feature: `compliance`)

```python
from eggsec import ComplianceMapper, ComplianceFramework

mapper = ComplianceMapper()

# Map findings to a compliance framework
framework = ComplianceFramework.load("owasp-top-10")
result = mapper.map_findings(findings, framework)

print(f"Compliant: {result.compliant_count}, Non-compliant: {result.non_compliant_count}")
for mapping in result.mappings:
    print(f"  Finding {mapping.finding_id} -> Control {mapping.control_id} ({mapping.match_type})")
```

#### External Integration and Publication

```python
from eggsec import ExternalIntegration, PublicationPolicy, RetryPolicy

integration = ExternalIntegration(integration_type="jira")
policy = PublicationPolicy(
    integration_type="jira",
    auto_publish=True,
    retry=RetryPolicy(max_retries=3, backoff_ms=1000),
    filter_severity="HIGH",
)

# Publish a finding
record = integration.publish(finding, policy=policy)
print(f"Published to: {record.url}")
```

#### Finding Migration

```python
from eggsec import FindingMigration, SchemaVersion

migration = FindingMigration()

# Migrate findings from v1 to current schema
target_version = SchemaVersion(major=2, minor=0, patch=0)
result = migration.migrate(old_findings, from_version=SchemaVersion(major=1, minor=0, patch=0), to_version=target_version)
print(f"Migrated {result.migrated_count} findings, errors: {len(result.errors)}")
```

## Milestone G — Extensibility and API Stabilization

### G1: Domain Registry and Operation Introspection

`DomainDescriptor` groups operations under capability domains. `DomainRegistry` provides read-only access to all registered domains. `OperationRegistry` gains enhanced methods for domain-scoped queries.

- `DomainDescriptor`: domain ID, label, required feature, category, operations list
- `DomainRegistry.all_domains()`, `.find(domain_id)`
- `OperationRegistry.find_by_domain(domain_id)`, `.domains()`

### G2: Event Protocol

Versioned event protocol with typed payloads. `EventEnvelope` wraps all events with schema version, sequence, timestamp, and kind. `EventStream` provides an async iterator over events.

- `EventEnvelope`: schema_version, sequence, timestamp, kind, payload
- `EventStream`: async iterator yielding `EventEnvelope` instances
- Typed payloads: `PlanningEvent`, `PreflightEvent`, `ProgressEvent`, `FindingEvent`, `ArtifactEvent`, `CompletionEvent`, `FailureEvent`, `CancellationEvent`

### G3: Callback and Sink Contracts

Finalized interfaces for extensibility points. Callbacks are guarded to prevent user exceptions from destabilizing Rust execution.

- `AuditSink`: receives `EnforcementAuditEvent` records
- `FindingSink`: receives `VersionedFinding` records
- `ArtifactSink`: receives `MilestoneArtifact` records
- `ProgressConsumer`: receives progress updates
- `AsyncCallback`: generic async callback wrapper
- `CallbackScheduler`: manages callback registration and invocation
- `BackpressureChannel`: bounded channel with backpressure for high-volume event streams

### G4: Python-Native Ergonomics

Consistent Python conventions across all types.

- `pathlib.Path` accepted for file paths (in addition to `str`)
- Python `datetime` converted to/from Rust `OffsetDateTime`
- `__hash__` and `__eq__` on immutable DTOs
- Context manager support (`__enter__`/`__exit__`) for session-oriented types
- Pickle support for versioned, secret-free DTOs only
- Stable `__repr__` with redacted secrets

### G5: Binary Buffer Protocol

Efficient binary data handling without unnecessary copies.

- `BinaryBuffer`: zero-copy buffer protocol (`memoryview` compatible) for packet and binary artifact data
- `LazyArtifact`: deferred artifact loading (loads content on access)
- `PaginatedResults`: iterator-based pagination for large result sets

### G6: Namespace and Import Stability

Finalized package layout and deprecation policy.

- `api_surface()`: returns list of all public API names with stability classifications
- `feature_matrix()`: returns dict of feature flags and their status
- `DeprecatedWarning`: emitted when using deprecated APIs
- `experimental`: namespace marker for pre-stability APIs

### G7: Versioning and Governance

Machine-readable version and schema metadata.

- `API_VERSION`, `SCHEMA_VERSION`, `ABI_VERSION` constants
- `api_surface_version()`: returns current API version tuple
- Version metadata in events, results, findings, and daemon messages

### G10: Release Hardening

Extended CI test suite covering:

- Runtime/stub export parity
- API-surface snapshots
- Minimal and feature-rich import tests
- Sync/async contract parity
- Cancellation/leak/shutdown tests
- Policy-equivalence tests
- Serialization compatibility fixtures
- Documentation build and link checks
- Wheel smoke tests
- Deprecation warning tests

### G11: Performance Budgets

Benchmark suite tracking:

- Engine startup time
- Repeated-call overhead
- Python/Rust transition cost
- Event delivery latency
- Large-result serialization throughput
- Packet-stream backpressure
- Callback overhead
- Async concurrency scaling

Regression budgets enforced in CI.

### G12: 1.0 Readiness Checklist

Final public API audit covering:

- Naming consistency
- Exception hierarchy completeness
- Type consistency across stubs and runtime
- Feature behavior documentation
- Migration path from pre-1.0
- Security semantics documentation
- Packaging matrix validation

See `docs/python/README_1_0_CHECKLIST.md` and `docs/python/STABILITY_CLASSIFICATIONS.md`.

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

## Release-readiness contracts

- `Engine` and `AsyncEngine` dispatch only the canonical twenty-two-operation
  `StableOperation` set (historical aliases are accepted for compatibility).
- `OperationResult.error` is an `OperationError`; use `error_message` only for
  legacy string consumers. `raise_for_status()` maps its `kind` to the
  documented Eggsec exception classes.
- `Engine.audit_events()` and `AsyncEngine.audit_events()` expose the
  structured allow/deny decisions emitted by the mandatory policy gate.
- `EventEnvelope.sequence` is monotonic within a producer stream.
  `BackpressureChannel.stats()` reports emitted, delivered, dropped, queue
  depth, and terminal-delivery counters; lifecycle/finding/artifact events are
  reliable within the process queue while progress is best effort.
- `domain_maturity()` is the authoritative whole-domain release boundary;
  feature availability does not promote a provisional or experimental domain.

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
| `src/consolidated_recon.rs` | `ConsolidatedReconConfig`, `run_consolidated_recon()` |
| `src/graphql.rs` | `GraphQLTestConfig`, `GraphQLSchema`, `graphql_test()` |
| `src/oauth.rs` | `OAuthTestConfig`, `OAuthEndpoint`, `oauth_test()` |
| `src/auth_assess.rs` | `AuthTestConfig`, `AuthTestReport`, `auth_test()` |
| `src/browser_assess.rs` | `BrowserTestConfig`, `BrowserTestReport`, `browser_test()` (feature-gated) |
| `src/hunt.rs` | `HuntTestConfig`, `HuntReport`, `hunt_test()` (feature-gated) |
| `src/finding_schema.rs` | `Confidence`, `FindingType`, `VersionedFinding`, `VersionedEvidence` (Milestone E) |
| `src/artifact.rs` | `MilestoneArtifact`, `ArtifactReference`, `ArtifactStore` (Milestone E) |
| `src/vuln_record.rs` | `CvssScore`, `VulnerabilityRecord`, `RemediationRecord` (Milestone E) |
| `src/workflow.rs` | `FindingState`, `WorkflowTransition`, `Suppression`, `FindingWorkflow` (Milestone E) |
| `src/repository.rs` | `FindingRepository`, `Assessment`, `AssessmentRepository` (Milestone E) |
| `src/correlation.rs` | `FindingCorrelation`, `FindingDiff`, `AssessmentDiff`, `BaselineComparator` (Milestone E) |
| `src/reporting.rs` | `FindingReporter`, `SeveritySummary`, `ReportEnvelope` (Milestone E) |
| `src/compliance.rs` | `ComplianceFramework`, `ComplianceControl`, `ComplianceMapper` (feature-gated, Milestone E) |
| `src/integration.rs` | `IntegrationType`, `PublicationRecord`, `ExternalIntegration` (Milestone E) |
| `src/migration.rs` | `SchemaVersion`, `MigrationResult`, `FindingMigration` (Milestone E) |
| `src/network.rs` | `TargetPy`, `ConnectionConfigPy`, `TimeoutConfigPy`, `RetryPolicyPy`, timing/evidence/transcript types (Release 2) |
| `src/transport.rs` | `TcpSessionPy`, `UdpSocketPy` managed sessions (Release 2) |
| `src/probes.rs` | `dns_query()`, `tls_probe()`, `http_probe()` one-shot probes (Release 2) |
| `src/http_client.rs` | `HttpClientPy`, `AsyncHttpClientPy` security-oriented HTTP client (Release 2) |
| `src/websocket.rs` | `WebSocketSessionPy`, `AsyncWebSocketSessionPy`, `websocket_assess()` (Release 2, feature-gated) |
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

- **Async bridge**: Hand-rolled `PyFuture` wrapper, not `pyo3-async-runtimes`. A process-global shared Tokio runtime (via `OnceLock`) ensures stateful async resources survive across chained awaits. The `AsyncClient` spawns tasks on the shared runtime and polls from Python's event loop via `PyFuture`. Lacks integration with Python's native `asyncio` cancellation propagation.
- **GIL release**: GIL is released during network I/O (blocking calls use `py.allow_threads()`), but CPU-bound Rust work holds the GIL.
- **Feature parity**: Not all engine features are exposed to Python. Feature-gated modules (e.g., `fuzzer`, `loadtest`, `stress`) require explicit `--features` at build time.
- **Type stubs**: Generated manually, not auto-generated from Rust source. Keep `python/eggsec/*.pyi` in sync with `src/` changes.

## Release 2: Network Programmability (Provisional Stability)

Release 2 network, transport, and probe types are now registered in the API
surface with provisional stability. They are scope-checked and policy-gated
but not yet part of the stable-core operation registry.

### Module Locations

| Module | Rust Source | Purpose |
|--------|------------|---------|
| `eggsec.network` | `src/network.rs` | Target resolution, connection config, timing, evidence, transcripts |
| `eggsec.transport` | `src/transport.rs` | Managed TCP/UDP sessions (context managers) |
| `eggsec.probes` | `src/probes.rs` | One-shot DNS, TLS, HTTP probes |
| `eggsec.http_client` | `src/http_client.rs` | Security-oriented HTTP client (sync/async) |
| `eggsec.websocket` | `src/websocket.rs` | WebSocket sessions and assessment |

### Convention Reminders

Release 2 types follow existing eggsec-python conventions:

- **Frozen pyclasses**: All DTOs (`TargetPy`, `ConnectionConfigPy`,
  `TimeoutConfigPy`, `RetryPolicyPy`, `SocketEndpointPy`, `ConnectionTimingPy`,
  `ConnectionMetadataPy`, `NetworkEvidencePy`, `TranscriptEntryPy`,
  `NetworkTranscriptPy`) are `#[pyclass(frozen)]`. Use constructor methods or
  factory functions — never mutate after construction.
- **`to_dict()` / `to_json()`**: Every serializable type exposes both methods.
  `to_dict()` returns a Python `dict`; `to_json()` returns a JSON string.
- **Context managers**: `TcpSessionPy`, `UdpSocketPy`, `WebSocketSessionPy`,
  and `AsyncWebSocketSessionPy` implement `__enter__`/`__exit__` (or async
  equivalents). Always use `with` blocks to ensure cleanup.
- **Scope enforcement**: All network operations validate targets against
  `LoadedScope` before making contact. Out-of-scope targets raise
  `EnforcementError`.
- **`_require_packet_inspection` decorator**: Handles both class and method
  decoration. When applied to a class, it gates all methods requiring the
  `packet-inspection` feature. When applied to a single method, it gates only
  that method.
- **Redaction**: `HttpClientPy` and `AsyncHttpClientPy` automatically redact
  sensitive headers (`Authorization`, `Cookie`, etc.) from transcripts and
  evidence.

### How to Add New Network Types

1. Implement the Rust struct in the appropriate `src/*.rs` file with
   `#[pyclass(frozen)]` and `#[pymethods]`.
2. Register in `src/lib.rs` via `m.add_class::<T>()`.
3. Re-export in `python/eggsec/__init__.py`.
4. Add a type stub in `python/eggsec/*.pyi`.
5. Add tests in `tests/` (use loopback fixtures; set
   `EGGSEC_ALLOW_LOOPBACK_FIXTURE=1` only for explicit fixture harnesses).
6. If the type is a new operation (not just a DTO), register it in the
   `OperationRegistry` and ensure it passes through `EnforcementContext`.

### Feature Flags

| Python Feature | Engine Feature | Notes |
|----------------|----------------|-------|
| `websocket` | `websocket` | WebSocket sessions and assessment; marker feature, no system deps |
| `packet-inspection` | `packet-inspection` | Raw packet injection; requires `libpcap-dev`; experimental |

Network programmability types in `eggsec.network`, `eggsec.transport`,
`eggsec.probes`, and `eggsec.http_client` are included in the default wheel
(no feature flag required). `eggsec.websocket` and raw packet injection
require their respective feature flags.

## Release 4: Stateful and Remote Execution

Release 4 adds session-oriented state management, daemon protocol parity,
SQLite-backed persistence, content-addressed artifact storage, and streaming
reporting. These types enable long-running assessments with resumable state,
durable findings, and incremental report generation.

### New Types

#### Session Contract

Core abstractions for stateful, resumable assessment sessions.

| Type | Purpose |
|------|---------|
| `SessionState` | Enum: `Created`, `Running`, `Paused`, `Completed`, `Failed`, `Cancelled` |
| `SessionIdentity` | Session ID, name, target, creation time, owner |
| `SessionStats` | Elapsed time, operations executed, findings collected, bytes transferred |
| `SessionCloseMode` | Enum: `Graceful`, `Immediate`, `Rollback` |
| `SessionEvent` | Lifecycle event: `Started`, `Progress`, `Finding`, `Artifact`, `Completed`, `Failed` |
| `SessionEventStream` | Async iterator yielding `SessionEvent` records for a session |
| `SessionCapabilities` | Declared capabilities: max concurrency, timeout, supported operation kinds |

#### Mobile Session

Stateful mobile device analysis sessions (feature: `mobile`).

| Type | Purpose |
|------|---------|
| `MobileDeviceDescriptor` | Device ID, OS version, manufacturer, model, connection type |
| `MobileDeviceCapabilities` | Supported features: ADB, screen capture, logcat, package management |
| `MobileSessionConfig` | Session config: device target, timeout, operation set, artifact output dir |
| `MobileSessionState` | Current device state: screen on/off, foreground app, battery, storage |
| `MobileSessionStats` | Per-session stats: apps analyzed, permissions extracted, findings collected |
| `MobileSession` | Sync session with context manager; manages device lifecycle |
| `AsyncMobileSession` | Async session with async context manager |
| `MobileDeviceRegistry` | Registry of discovered/connected devices |

#### Browser Session

Stateful headless browser assessment sessions (feature: `headless-browser`).

| Type | Purpose |
|------|---------|
| `BrowserCapabilities` | Supported features: navigation, JS execution, screenshots, network capture |
| `BrowserSessionState` | Current state: URL, ready state, JS errors, cookies, storage |
| `BrowserSessionConfig` | Session config: base URL, viewport, timeouts, permitted origins |
| `BrowserSessionStats` | Per-session stats: pages visited, DOM snapshots, findings collected |
| `BrowserNavigationEvent` | Navigation event: URL, method, status, redirect chain |
| `BrowserConsoleEvent` | Console event: level, message, source, timestamp |
| `BrowserNetworkEvent` | Network event: request/response, status, headers, timing |
| `BrowserDomSnapshot` | DOM snapshot: HTML, title, forms, links, scripts, storage |
| `BrowserFormInfo` | Form metadata: action, method, fields |
| `BrowserFormField` | Field: name, type, required, autocomplete, value |
| `BrowserLinkInfo` | Link: href, text, rel, target |
| `BrowserStorageInfo` | Storage: localStorage, sessionStorage, indexedDB keys |
| `BrowserCookieInfo` | Cookie: name, value, domain, path, expires, secure, httponly |
| `BrowserSession` | Sync session with context manager |
| `AsyncBrowserSession` | Async session with async context manager |

#### Daemon Parity

Protocol and reliability types for daemon session access.

| Type | Purpose |
|------|---------|
| `DaemonProtocolVersion` | Protocol version: major, minor, compatibility |
| `IdempotencyKey` | Unique key for idempotent operation submission |
| `DaemonSubmissionResult` | Submission result: task ID, accepted, queue position |
| `ReconnectOptions` | Reconnect config: max retries, backoff, session resume |
| `ReplayCursor` | Cursor for replaying missed events from a session |
| `ReplayResult` | Replay result: events replayed, gap detected, cursor advanced |
| `CancellationRequest` | Cancellation: task ID, reason, graceful flag |
| `CancellationResult` | Result: cancelled, in_progress, already_terminal |
| `TaskArtifactDescriptor` | Artifact descriptor: task ID, artifact ID, kind, path |
| `EventReplayInfo` | Replay info: sequence range, event kinds, completeness |
| `DaemonHealthDetail` | Health: uptime, active sessions, memory, CPU, version |

#### SQLite Repository

Durable persistence for findings and assessments.

| Type | Purpose |
|------|---------|
| `SqliteFindingRepository` | SQLite-backed finding repository with query, filter, count |
| `SqliteAssessmentRepository` | SQLite-backed assessment repository |
| `SqliteMigration` | Schema migration runner for SQLite databases |
| `MigrationResult` | Migration result: success, version, applied migrations, errors |

#### Content-Addressed Store

Immutable, deduplicated artifact storage.

| Type | Purpose |
|------|---------|
| `ContentAddressedArtifactStore` | CAS store keyed by content hash |
| `DirectoryArtifactStore` | Directory-backed artifact store |
| `ArtifactInfo` | Artifact metadata: hash, size, mime type, created at |
| `ArtifactData` | Artifact content: bytes or path reference |
| `IntegrityResult` | Integrity check: hash match, corruption detected |
| `ArtifactQuery` | Query: hash prefix, mime type, size range, date range |

#### Streaming Reporter

Incremental report generation and diffing.

| Type | Purpose |
|------|---------|
| `StreamingReportConfig` | Config: format, buffer size, flush interval, output path |
| `StreamingReporter` | Incremental reporter: `write_finding()`, `finalize()`, `summary()` |
| `ReportSummary` | Summary: total findings, severity breakdown, elapsed time |
| `StreamingDiffReporter` | Diff reporter: compare two streaming report runs |
| `FindingDiffResult` | Diff: added, removed, changed findings |
| `DiffReportSummary` | Diff summary: net changes, regression count |
| `ReportManifest` | Manifest: report ID, schema version, finding count, checksum |

### Feature Flags

| Feature | Types Enabled | Notes |
|---------|---------------|-------|
| `mobile` | `MobileDeviceDescriptor`, `MobileDeviceCapabilities`, `MobileSessionConfig`, `MobileSessionState`, `MobileSessionStats`, `MobileSession`, `AsyncMobileSession`, `MobileDeviceRegistry` | Requires ADB for dynamic sessions |
| `headless-browser` | `BrowserCapabilities`, `BrowserSessionState`, `BrowserSessionConfig`, `BrowserSessionStats`, `BrowserNavigationEvent`, `BrowserConsoleEvent`, `BrowserNetworkEvent`, `BrowserDomSnapshot`, `BrowserFormInfo`, `BrowserFormField`, `BrowserLinkInfo`, `BrowserStorageInfo`, `BrowserCookieInfo`, `BrowserSession`, `AsyncBrowserSession` | Requires headless-chrome |
| *(always available)* | All session contract, daemon parity, SQLite repository, content-addressed store, and streaming reporter types | No feature flag required |

### Common Patterns

#### Creating a Session with Context Manager

```python
from eggsec import MobileSession, MobileSessionConfig

config = MobileSessionConfig(
    device_id="emulator-5554",
    timeout_secs=300,
    output_dir="/tmp/artifacts",
)

# Sync — cleanup on exit
with MobileSession(config) as session:
    result = session.analyze_apk("/path/to/app.apk")
    print(f"Session stats: {session.stats}")
    for event in session.events():
        print(f"Event: {event}")

# Async variant
from eggsec import AsyncMobileSession

async with AsyncMobileSession(config) as session:
    result = await session.analyze_apk("/path/to/app.apk")
    async for event in session.events():
        print(f"Event: {event}")
```

#### Using SQLite Repository

```python
from eggsec import SqliteFindingRepository, SqliteAssessmentRepository, VersionedFinding

# Open (creates if missing)
repo = SqliteFindingRepository("/data/eggsec-findings.db")

# Save findings
repo.save(finding)

# Query
results = repo.query(severity="HIGH", finding_type="vulnerability")
print(f"Found {repo.count()} total, {len(results)} matching")

# Assessment persistence
assess_repo = SqliteAssessmentRepository("/data/eggsec-findings.db")
assess_repo.save(assessment)

# Schema migration
from eggsec import SqliteMigration

migration = SqliteMigration("/data/eggsec-findings.db")
result = migration.run()
print(f"Migrated to version {result.version}, applied {len(result.applied)} migrations")
```

#### Content-Addressed Artifact Storage

```python
from eggsec import ContentAddressedArtifactStore, ArtifactData

store = ContentAddressedArtifactStore("/data/artifacts")

# Store content — key is the SHA-256 hash
artifact_id = store.store(ArtifactData(content=b"..."), mime_type="application/octet-stream")
print(f"Stored as: {artifact_id}")

# Retrieve
info = store.get(artifact_id)
print(f"Size: {info.size_bytes}, hash: {info.hash}")

# Integrity check
result = store.verify_integrity(artifact_id)
print(f"Intact: {result.hash_match}")

# Query by prefix or mime type
matches = store.query(mime_type="application/json")
```

#### Streaming Reporting

```python
from eggsec import StreamingReporter, StreamingReportConfig

config = StreamingReportConfig(
    format="json",
    buffer_size=100,
    flush_interval_secs=10,
    output_path="/reports/scan.json",
)

reporter = StreamingReporter(config)

# Write findings incrementally as they arrive
for finding in scan_results:
    reporter.write_finding(finding)

# Finalize and get summary
summary = reporter.finalize()
print(f"Total: {summary.total_findings}")
print(f"Critical: {summary.critical}, High: {summary.high}")

# Diff two reports
from eggsec import StreamingDiffReporter

diff_reporter = StreamingDiffReporter()
diff = diff_reporter.compare(baseline_report, current_report)
print(f"Added: {len(diff.added)}, Removed: {len(diff.removed)}")
```

### Key Files

| File | Purpose |
|------|---------|
| `src/session.rs` | `SessionState`, `SessionIdentity`, `SessionStats`, `SessionEvent`, `SessionEventStream`, `SessionCapabilities` |
| `src/mobile_session.rs` | `MobileSession`, `AsyncMobileSession`, `MobileDeviceDescriptor`, `MobileDeviceCapabilities`, `MobileSessionConfig`, `MobileSessionState`, `MobileSessionStats`, `MobileDeviceRegistry` (feature: `mobile`) |
| `src/browser_session.rs` | `BrowserSession`, `AsyncBrowserSession`, `BrowserCapabilities`, `BrowserSessionState`, `BrowserSessionConfig`, `BrowserSessionStats`, browser event/snapshot types (feature: `headless-browser`) |
| `src/daemon_parity.rs` | `DaemonProtocolVersion`, `IdempotencyKey`, `DaemonSubmissionResult`, `ReconnectOptions`, `ReplayCursor`, `ReplayResult`, `CancellationRequest`, `CancellationResult`, `TaskArtifactDescriptor`, `EventReplayInfo`, `DaemonHealthDetail` |
| `src/sqlite_repository.rs` | `SqliteFindingRepository`, `SqliteAssessmentRepository`, `SqliteMigration`, `MigrationResult` |
| `src/content_addressed_store.rs` | `ContentAddressedArtifactStore`, `DirectoryArtifactStore`, `ArtifactInfo`, `ArtifactData`, `IntegrityResult`, `ArtifactQuery` |
| `src/streaming_reporter.rs` | `StreamingReportConfig`, `StreamingReporter`, `ReportSummary`, `StreamingDiffReporter`, `FindingDiffResult`, `DiffReportSummary`, `ReportManifest` |

## Release 5 Phase A: Tool-Core and Schema Integration

Release 5 Phase A exposes `eggsec-tool-core` types to Python, providing a
deterministic tool abstraction for all 22 stable operations. The tool layer
gives every operation a unified request/response contract, JSON Schema
generation, and a registry-driven invocation path.

### New Types

| Type | Purpose |
|------|---------|
| `ToolTargetType` | Target classification enum (url, domain, ip, cidr, file) |
| `ToolAuthType` | Authentication type enum (none, basic, bearer, api_key, oauth2) |
| `ToolResponseType` | Execution status enum (success, partial_success, failed, timeout, scope_violation, cancelled) |
| `ToolFindingType` | Finding classification enum (vulnerability, information, weakness, etc.) |
| `ToolSeverity` | Severity level enum (critical, high, medium, low, info, none) |
| `ToolErrorType` | Error classification enum (validation, authentication, network, etc.) |
| `ToolPortState` | Port scan state enum (open, closed, filtered) |
| `ToolStreamEventType` | Stream event type enum (progress, finding, result, error) |
| `ToolScope` | Execution scope (allowed/excluded patterns, IPs) |
| `ToolTarget` | Scanning target (type + value + optional scope) |
| `ToolRequestOptions` | Request options (timeout, concurrency, proxy, stealth, SSL) |
| `ToolAuthConfig` | Auth configuration (type + credentials, redacted in repr) |
| `ToolRequest` | Execution request (tool, target, params, options) |
| `ToolResponseMetadata` | Response metadata (timing, counts, duration) |
| `ToolFinding` | Security finding (type, severity, title, location, evidence) |
| `ToolError` | Structured error (code, message, type, recoverable, retry) |
| `ToolResponse` | Execution response (status, results, metadata, errors, findings) |
| `ToolProgressUpdate` | Streaming progress (stage, percentage, items found) |
| `ToolStreamEvent` | Typed stream event (progress, finding, result, error) |
| `ToolPortData` | Port scan result for a single port |
| `ToolEndpointData` | Discovered endpoint |
| `ToolTechnologyData` | Detected technology |
| `ToolRateLimitConfig` | Rate limit configuration (RPM, concurrency, burst) |
| `ToolRateLimitStatus` | Current rate limit state |
| `ToolExecutionEntry` | Execution history record |
| `ToolDescriptor` | Tool metadata (ID, label, target types, schemas, risk) |
| `ToolRegistry` | Static registry for tool lookup |
| `SchemaGenerator` | JSON Schema generation for tool types |

### Key Files

| File | Purpose |
|------|---------|
| `src/tool_core.rs` | All tool-core Python bindings (enums, structs, conversion helpers) |

### Common Patterns

#### Tool Registry Discovery

```python
from eggsec import ToolRegistry

# List all registered tools
tools = ToolRegistry.all_tools()
for t in tools:
    print(f"{t.tool_id}: {t.label} (risk={t.risk})")

# Find by tool ID
desc = ToolRegistry.find("scan_ports")
print(desc.description)
print(desc.target_types)  # [ToolTargetType.ip(), ToolTargetType.domain()]

# Find by operation ID
desc = ToolRegistry.find_by_operation("scan-ports")
```

#### Schema Generation

```python
from eggsec import SchemaGenerator

# Request schema for an operation
schema = SchemaGenerator.request_schema("scan_ports")
# Returns JSON Schema dict

# Response schema
schema = SchemaGenerator.response_schema("scan_ports")

# Full manifest (all 22 operations)
manifest = SchemaGenerator.full_manifest()
```

#### Tool Invocation via Engine

```python
from eggsec import Engine, Scope, ToolRequest, ToolTarget, ToolRequestOptions

scope = Scope.allow_hosts(["127.0.0.1"])
engine = Engine(scope)

target = ToolTarget.ip("127.0.0.1")
request = ToolRequest.new(
    tool="scan_ports",
    target=target,
    params={"ports": [22, 80, 443]},
    options=ToolRequestOptions.new(timeout_ms=5000),
)

response = engine.invoke_tool(request)
if response.is_success():
    for finding in response.findings:
        print(f"[{finding.severity}] {finding.title}")
```

#### Async Tool Invocation

```python
import asyncio
from eggsec import AsyncEngine, Scope, ToolRequest, ToolTarget

async def main():
    scope = Scope.allow_hosts(["127.0.0.1"])
    engine = AsyncEngine(scope)

    target = ToolTarget.ip("127.0.0.1")
    request = ToolRequest.new(
        tool="scan_ports",
        target=target,
        params={"ports": [22, 80, 443]},
    )

    response = await engine.async_invoke_tool(request)
    print(response.status)

asyncio.run(main())
```

#### Using Tool-Core Types Directly

```python
from eggsec import (
    ToolTargetType, ToolAuthType, ToolResponseType, ToolFindingType,
    ToolSeverity, ToolErrorType, ToolPortState, ToolStreamEventType,
    ToolScope, ToolTarget, ToolRequestOptions, ToolAuthConfig,
    ToolRequest, ToolFinding, ToolError, ToolResponse,
    ToolPortData, ToolEndpointData, ToolTechnologyData,
    ToolRateLimitConfig, ToolRateLimitStatus, ToolExecutionEntry,
)

# Build a target
target = ToolTarget.url("https://example.com")

# Build request options
options = ToolRequestOptions.new(
    timeout_ms=10000,
    concurrency=5,
    stealth=True,
    verify_ssl=False,
)

# Build a request
request = ToolRequest.new(
    tool="scan_ports",
    target=target,
    params={"ports": [80, 443]},
    options=options,
)

# Access request properties
print(request.id)       # auto-generated UUID
print(request.tool)     # "scan_ports"
print(request.target)   # ToolTarget
print(request.params)   # {"ports": [80, 443]}

# Create findings
finding = ToolFinding(
    id="find-001",
    finding_type=ToolFindingType.vulnerability(),
    severity=ToolSeverity.high(),
    title="Open port with default credentials",
    description="Port 22 accepts root:root",
    location="127.0.0.1:22",
    evidence="SSH-2.0-OpenSSH_8.9",
    cve_ids=["CVE-2023-1234"],
    remediation="Disable root login",
)

# Create errors
error = ToolError(
    code="E_TIMEOUT",
    message="Connection timed out",
    error_type=ToolErrorType.timeout(),
    recoverable=True,
    retry_after_ms=5000,
)

# Rate limiting
config = ToolRateLimitConfig.strict()
status = ToolRateLimitStatus.new(
    tokens_available=45.0,
    requests_this_minute=15,
    requests_per_minute=60,
    concurrent_available=3,
    concurrent_limit=5,
    concurrent_in_use=2,
)
```

## Release 5 Phase B: Registry Dispatch Architecture

Release 5 Phase B completes registry and dispatch convergence. A single
authoritative `OperationExecutorDescriptor` registry replaces duplicated
sync/async dispatch and parallel metadata inventories.

### Architecture Overview

The generic dispatch lifecycle (`pre_dispatch_lifecycle` → `execute_operation`
→ `post_dispatch_hooks`) replaces per-operation match arms in both sync and
async engines.

```
Engine.run(request) / AsyncEngine.run(request)
  └─ OperationExecutorRegistry::execute()
       └─ dispatch()
            ├─ pre_dispatch_lifecycle()     ← shared with async
            │    ├─ planning event
            │    ├─ scope/feature validation
            │    ├─ preflight event
            │    ├─ cancel check
            │    └─ deadline check
            ├─ execute_operation()          ← per-op match, typed params
            └─ post_dispatch_hooks()        ← finding/artifact events
```

### Key Components

| Component | File | Purpose |
|-----------|------|---------|
| `OperationExecutorDescriptor` | `src/operation_registry.rs` | Single source of truth for per-operation metadata |
| `pre_dispatch_lifecycle()` | `src/dispatch_helpers.rs` | Shared planning, validation, cancel/deadline checks |
| `execute_operation()` | `src/engine.rs` / `src/async_engine.rs` | Per-operation parameter extraction and inner call |
| `post_dispatch_hooks()` | `src/engine.rs` | Finding/artifact event emission after dispatch |
| Per-op executors | `src/operation_executors.rs` | Normalization of generic requests into typed parameters |

### Adding a New Operation

To add a new stable operation:

1. **Register in `StableOperation` enum** (`src/operation_registry.rs`):
   - Add variant to `StableOperation`
   - Add entry in `StableOperation::ALL`
   - Add to `from_operation()` match for descriptor metadata
   - Add to `id()`, `parse()`, `feature_required()`, `aliases()` methods

2. **Add `OperationExecutorDescriptor`** in `from_operation()`:
   - Set risk classification, feature gate, confirmation behavior
   - Set schema IDs, daemon task kind, finding/artifact hook flags

3. **Add `execute_operation` arm** in `src/engine.rs`:
   - Extract typed parameters from `request.metadata`
   - Delegate to the inner method (e.g., `run_*_inner()`)

4. **Add `execute_operation_async` arm** in `src/async_engine.rs`:
   - Same parameter extraction as sync
   - Delegate to the async method (e.g., `run_*_async()`)

5. **Add `post_dispatch_hooks` arm** (if finding/artifact hooks needed):
   - Emit finding events based on result payload

6. **Add daemon task kind** in `dispatch_helpers::operation_request_to_daemon_task()`:
   - Add JSON params for the operation

7. **Add tests**:
   - Add to `tests/test_golden_contract.py` canonical operation list
   - Add Rust-side tests in `src/operation_registry.rs`

### Architecture Guard Tests

The golden contract test suite (`tests/test_golden_contract.py`) enforces:

- One descriptor per operation (no duplicates)
- Unique operation IDs across all 22 operations
- Alias non-collision (no two operations share an alias)
- Schema identity (ToolRegistry descriptors match OperationExecutorDescriptor)
- Feature gate metadata consistency
- Risk classification completeness

Run via: `pytest crates/eggsec-python/tests/test_golden_contract.py`

### Key Invariants

1. **`OperationExecutorDescriptor` is the single source of truth** — no
   parallel metadata inventories in engine or async engine.
2. **Generic dispatch lifecycle is shared** — `pre_dispatch_lifecycle()`
   handles identical pre-dispatch steps for both sync and async.
3. **Typed methods are thin delegates** — `Engine.run_port_scan()` etc.
   remain as `#[pymethods]` for backward compatibility but delegate to the
   same inner methods used by generic dispatch.
4. **Generated inventories derive from registry** — capability manifests,
   tool descriptors, feature maps, and daemon parity metadata are all
   derived from `OperationExecutorDescriptor`, not hand-maintained.

## Release 5 Phase C: Namespace and Maturity Governance

Phase C reorganizes the Python package into intentional submodules by capability ownership.

### Package Structure Changes

- **Stable top-level surface**: Only engine, 22 operations, config, events, and core DTOs remain at `eggsec.__init__`
- **`eggsec._feature_guard`**: Feature availability detection with structured `FeatureUnavailableError` and install guidance
- **`eggsec.net`**: Network types (Target, TcpSession, HttpClient, WebSocket) — provisional
- **`eggsec.sessions`**: Session types (MobileSession, BrowserSession, DatabaseSession, ProxySession) — provisional
- **`eggsec.storage`**: Repository types (FindingRepository, AssessmentRepository, ArtifactStore) — provisional
- **`eggsec.reporting`**: Reporter types (FindingReporter, StreamingReporter, BaselineComparator) — provisional
- **`eggsec.daemon`**: Daemon client and parity contracts — provisional
- **`eggsec.experimental`**: Wireless, evasion, postex, C2, hunt, AI, stress types

### Key Changes

- **Canonical naming**: Py-suffixed names (e.g., `TargetPy`) are deprecated in favor of clean names (e.g., `Target`)
- **Backward compatibility**: All existing import paths continue to work; Py-suffixed names emit `DeprecationWarning`
- **Import safety**: `import eggsec` does not initialize experimental, browser, database, or other heavy dependencies
- **Namespace governance**: 27 new architecture guard tests enforce submodule boundaries and stability classifications

### Migration

```python
# Before (flat namespace)
from eggsec import TargetPy, TcpSessionPy, HttpClientPy

# After (organized submodules, recommended)
from eggsec.net import Target, TcpSession, HttpClient

# Backward-compatible (deprecated, still works)
from eggsec import TargetPy
```

## Phase D: Python Ergonomics

### Context Managers
All callback/sink classes support `with` statements for automatic cleanup:
```python
with AuditSink(lambda e: print(e)) as sink:
    # use sink
    pass
# sink is automatically closed
```

Affected classes: `AuditSink`, `FindingSink`, `ArtifactSink`, `ProgressSink`, `EventConsumer`, `AsyncCallback`, `CallbackScheduler`

### Enum Ergonomics
- All enums support `from_str()` which raises `ValueError` for unknown values
- `ExecutionStatus` has full `__eq__`, `__hash__`, `__str__`, `from_str` support
- All `eq_int` enums have `__hash__` for use in sets/dicts

### Serialization Round-Trip
`OperationError`, `ExecutionStats`, and `Artifact` support full round-trip:
```python
err = OperationError(kind="network", code="timeout", message="timed out")
d = err.to_dict()
restored = OperationError.from_dict(d)  # == err in value
j = err.to_json()
restored = OperationError.from_json(j)  # == err in value
```

### Testing
Run Phase D tests: `pytest crates/eggsec-python/tests/test_phase_d_ergonomics.py`

## Phase E: Packaging, Documentation, and Executable Examples

### Package Metadata Corrections
- Repository URLs aligned to workspace (`eggstack/eggsec`)
- Author metadata corrected to match workspace
- Development status updated to `4 - Beta`
- `Operating System :: POSIX` classifier added

### Wheel Profiles
Machine-readable manifest at `crates/eggsec-python/wheel-profiles.json`. Nine profiles:
- **core** — default wheel, no optional features, 14 stable operations
- **full-no-system** — websocket + git-secrets + sbom + container
- **nse** — NSE script execution (requires libssl-dev)
- **db-pentest** — database pentesting
- **web-proxy** — web proxy MITM
- **mobile** — mobile app static analysis
- **packet-inspection** — packet capture (requires libpcap-dev)
- **headless-browser** — browser security testing (requires Chromium)
- **full** — all non-default features

### Enhanced Diagnostics
`build_info()` now returns: `schema_version`, `protocol_version`, `abi_version`, `python_version`, `compiled_features` (list), `wheel_profile`.

New standalone function: `eggsec.wheel_profile()` → `"core"` | `"full-no-system"` | `"custom"`.

### Workflow-Oriented Documentation
Python docs landing page (`docs/python/index.md`) reorganized into 13 workflow sections:
1. Installation & Wheel Profiles
2. Capability & Maturity Discovery
3. Stable Operations (Engine & AsyncEngine)
4. Scope, Policy, Confirmation & Audit
5. Events, Callbacks, Cancellation & Timeouts
6. Pipelines & Checkpoints
7. Low-Level Networking & Protocol Probes
8. Tools & JSON Schemas
9. Repositories, Artifacts & Reporting
10. Daemon Execution
11. Provisional Managed Sessions
12. Experimental Domains
13. API Reference & Compatibility

### Executable Examples (6 new, all deterministic/local)
- `port_scan_loopback.py` — sync+async port scan with in-process TCP server
- `engine_capability_discovery.py` — feature/version/maturity introspection
- `policy_preflight.py` — enforcement context and policy gate walkthrough
- `cancellation_timeout.py` — cancellation token and timeout handling
- `finding_repository.py` — finding storage, query, and baseline comparison
- `tool_descriptor_schema.py` — tool registry and JSON Schema generation
