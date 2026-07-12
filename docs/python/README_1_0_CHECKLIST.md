# eggsec-python 1.0 Readiness Checklist

Workstream G12 — Python API Milestone G completeness review.

---

## A. API Audit

### Core

| Symbol | Type | Stability |
|--------|------|-----------|
| `Engine` | class | stable |
| `AsyncEngine` | class | stable |
| `Scope` | class | stable |
| `Client` | class | stable |
| `AsyncClient` | class | stable |
| `EggsecConfig` | class | stable |
| `SensitiveString` | class (frozen) | stable |
| `HttpConfig` | class (frozen) | stable |
| `ScanConfig` | class (frozen) | stable |
| `OutputConfig` | class (frozen) | stable |
| `ReconApiConfig` | class (frozen) | stable |
| `ReconConfig` | class (frozen) | stable |
| `ProxyConfigEntry` | class (frozen) | stable |
| `AllowedWorker` | class (frozen) | stable |
| `RemoteConfig` | class (frozen) | stable |
| `AiConfig` | class (frozen) | stable |
| `SearchConfig` | class (frozen) | stable |
| `PathsConfig` | class (frozen) | stable |
| `CacheConfig` | class (frozen) | stable |
| `AlertChannelConfig` | class (frozen) | stable |

**Count:** 20 classes/functions. All stable.

### Scanning

| Symbol | Type | Stability |
|--------|------|-----------|
| `scan_ports` / `async_scan_ports` | function | stable |
| `scan_endpoints` / `async_scan_endpoints` | function | stable |
| `fingerprint_services` / `async_fingerprint_services` | function | stable |
| `PortScanResult` | class | stable |
| `OpenPort` | class | stable |
| `ScanStats` | class | stable |
| `PortRange` | class | stable |
| `TimingPreset` | class | stable |
| `EndpointScanConfig` | class | stable |
| `EndpointFinding` | class | stable |
| `EndpointScanStats` | class | stable |
| `EndpointScanResult` | class | stable |
| `FingerprintEvidence` | class | stable |
| `FingerprintConfidence` | class | stable |
| `ServiceFingerprintResult` | class | stable |
| `FingerprintScanResult` | class | stable |

**Count:** 16 (8 functions, 12 classes). All stable.

### Recon

| Symbol | Type | Stability |
|--------|------|-----------|
| `recon_dns` / `async_recon_dns` | function | stable |
| `inspect_tls` / `async_inspect_tls` | function | stable |
| `detect_technology` / `async_detect_technology` | function | stable |
| `DnsRecordSet` | class | stable |
| `MxRecord` | class | stable |
| `SoaRecord` | class | stable |
| `TlsCertificateInfo` | class | stable |
| `TlsInspectionResult` | class | stable |
| `SslIssue` | class | stable |
| `TechStack` | class | stable |
| `TechDetectionResult` | class | stable |
| `ConsolidatedReconConfig` | class | stable |
| `ReconModuleResult` | class | stable |
| `ConsolidatedReconReport` | class | stable |
| `run_consolidated_recon` / `async_run_consolidated_recon` | function | stable |

**Count:** 15 (8 functions, 11 classes). All stable.

### WAF

| Symbol | Type | Stability |
|--------|------|-----------|
| `detect_waf` / `async_detect_waf` | function | stable |
| `validate_waf` / `async_validate_waf` | function | stable |
| `fuzz_http` / `async_fuzz_http` | function | stable |
| `generate_fuzz_payloads` | function | stable |
| `WafDetectionResult` | class | stable |
| `BypassResult` | class | stable |
| `WafScanResult` | class | stable |
| `Payload` | class | stable |
| `FuzzResult` | class | stable |
| `FuzzSession` | class | stable |
| `FuzzConfig` | class | stable |

**Count:** 11 (7 functions, 6 classes). All stable.

### Findings

| Symbol | Type | Stability |
|--------|------|-----------|
| `Severity` | enum (frozen) | stable |
| `Evidence` | class (frozen) | stable |
| `Finding` | class | stable |
| `FindingSet` | class | stable |
| `Report` | class | stable |
| `Confidence` | enum (Confirmed, High, Medium, Low, Informational) | stable |
| `FindingType` | enum (Vulnerability, Misconfiguration, InformationLeak, PolicyViolation, ScanResult, ServiceDetection, AssetDiscovery, FuzzResult, WafDetection) | stable |
| `EvidenceKind` | enum (Screenshot, HttpRequest, HttpResponse, Header, BodySnippet, Certificate, DnsRecord, FilePath, LogLine, PortState, Timing, Banner, Diff) | stable |
| `AffectedAsset` | class | stable |
| `FindingLocation` | class | stable |
| `VersionedEvidence` | class | stable |
| `VersionedFinding` | class | stable |
| `FINDING_SCHEMA_VERSION` | constant | stable |

**Count:** 13. All stable.

### Pipeline

| Symbol | Type | Stability |
|--------|------|-----------|
| `PipelineStep` | class | stable |
| `StepResult` | class | stable |
| `PipelineResult` | class | stable |
| `Pipeline` | class | stable |
| `AsyncPipeline` | class | stable |
| `PlanStep` | class | stable |
| `ScanPlan` | class | stable |
| `Checkpoint` | class | stable |
| `CheckpointStore` | class | stable |

**Count:** 9. All stable.

### Enforcement

| Symbol | Type | Stability |
|--------|------|-----------|
| `EnforcementContext` | class | stable |
| `ExecutionPolicy` | class | stable |
| `ManualOverride` | class | stable |
| `ExecutionSurface` | class | stable |
| `ExecutionProfile` | class | stable |
| `PolicyDecision` | class | stable |
| `EnforcementOutcome` | class | stable |
| `ApprovedOperation` | class | stable |
| `OperationRegistry` | class | stable |
| `OperationMetadataView` | class | stable |
| `OperationDescriptor` | class | stable |
| `OperationRisk` | enum | stable |
| `OperationMode` | enum | stable |
| `IntendedUse` | enum | stable |
| `Capability` | enum | stable |
| `DenialClass` | enum | stable |
| `TargetPolicyKind` | enum | stable |
| `DomainDescriptor` | class (frozen) | stable |
| `DomainRegistry` | class | stable |

**Count:** 19. All stable.

### Events

| Symbol | Type | Stability |
|--------|------|-----------|
| `EventEnvelope` | class | stable |
| `EventStream` | class | stable |
| `ExecutionHandle` | class | stable |
| `ExecutionEvent` | class | stable |
| `EventLog` | class | stable |
| `CancellationToken` | class | stable |
| `PlanningEvent` | class | stable |
| `PreflightEvent` | class | stable |
| `StageLifecycleEvent` | class | stable |
| `ProgressEvent` | class | stable |
| `FindingEvent` | class | stable |
| `ArtifactEvent` | class | stable |
| `CancellationEvent` | class | stable |
| `FailureEvent` | class | stable |
| `CompletionEvent` | class | stable |
| `wrap_event` | function | stable |
| `event_stream_from_legacy` | function | stable |
| `EVENT_SCHEMA_VERSION` | constant | stable |

**Count:** 18. All stable.

### Callbacks

| Symbol | Type | Stability |
|--------|------|-----------|
| `AuditSink` | class | stable |
| `FindingSink` | class | stable |
| `ArtifactSink` | class | stable |
| `ProgressSink` | class | stable |
| `EventConsumer` | class | stable |
| `AsyncCallback` | class | stable |
| `CallbackScheduler` | class | stable |
| `BackpressureChannel` | class | stable |

**Count:** 8. All stable.

### Buffers

The buffer support types (`BinaryBuffer`, `ArtifactMeta`, `LazyArtifact`,
`PaginatedResults`) are registered in `_core` but are not re-exported in
the top-level `eggsec` namespace. They are internal types accessible via
`eggsec._core`. They are not part of the public API.

**Count:** 0 public (4 internal). Internal only.

### Introspection

| Symbol | Type | Stability |
|--------|------|-----------|
| `api_surface` | function | stable |
| `api_surface_version` | function | stable |
| `features` | function | stable |
| `has_feature` | function | stable |
| `feature_matrix` | function | stable |
| `build_info` | function | stable |
| `OperationRegistry` | class | stable |
| `DomainRegistry` | class | stable |

**Count:** 8. All stable.

### Audit / Preflight

| Symbol | Type | Stability |
|--------|------|-----------|
| `validate_scope` | function | stable |
| `preflight_operation` | function | stable |
| `preflight_with_descriptor` | function | stable |
| `audit_event_from_enforcement` | function | stable |
| `audit_event_from_preflight` | function | stable |
| `emit_audit_event` | function | stable |
| `LoadedScope` | class | stable |
| `ScopeSource` | class | stable |
| `ScopeRule` | class | stable |
| `ScopeExplanation` | class | stable |
| `ScopeValidation` | class | stable |
| `PreflightResult` | class | stable |
| `AuditOutcome` | class | stable |
| `ManualOverrideAudit` | class | stable |
| `ScopeAudit` | class | stable |
| `EnforcementAuditEvent` | class | stable |

**Count:** 16. All stable.

### Introspection (version/constants)

| Symbol | Type | Stability |
|--------|------|-----------|
| `__version__` | constant | stable |
| `__version_info__` | constant | stable |
| `__schema_version__` | constant | stable |
| `__protocol_version__` | constant | stable |
| `__abi_version__` | constant | stable |
| `SCHEMA_VERSION` | constant | stable |
| `PROTOCOL_VERSION` | constant | stable |
| `ABI_VERSION` | constant | stable |

**Count:** 8. All stable.

### Deprecated

| Symbol | Type | Stability |
|--------|------|-----------|
| `deprecated_warning` | function | deprecated |
| `DeprecatedWarning` | class | deprecated |

**Count:** 2. Deprecated (replaced by `DeprecationWarning` directly).

### Experimental namespace

The `eggsec.experimental` subpackage exists but exports nothing (`__all__ = []`). No APIs are currently in the experimental namespace.

### Summary

| Category | Count | Status |
|----------|-------|--------|
| Core | 20 | stable |
| Scanning | 16 | stable |
| Recon | 15 | stable |
| WAF | 11 | stable |
| Findings | 13 | stable |
| Pipeline | 9 | stable |
| Enforcement | 19 | stable |
| Events | 18 | stable |
| Callbacks | 8 | stable |
| Buffers | 0 | internal only |
| Introspection | 8 | stable |
| Audit/Preflight | 16 | stable |
| Version constants | 8 | stable |
| Deprecated | 2 | deprecated |
| **Total** | **167** | |

---

## B. Naming Consistency Review

### Conventions verified

- **CamelCase classes**: All classes use CamelCase (e.g., `PortScanResult`, `EndpointScanConfig`).
- **snake_case functions**: All functions use snake_case (e.g., `scan_ports`, `recon_dns`).
- **Prefix conventions**:
  - `scan_*` for scanning operations (scan_ports, scan_endpoints, scan_docker_image, etc.)
  - `detect_*` for detection operations (detect_waf, detect_technology, detect_escape_risks)
  - `recon_*` for recon operations (recon_dns)
  - `async_*` for async counterparts (async_scan_ports, async_recon_dns, etc.)
  - `validate_*` for validation (validate_waf, validate_scope)
  - `fuzz_*` for fuzzing (fuzz_http)
  - `run_*` for long-running operations (run_consolidated_recon, run_traceroute)
  - `load_test_*` for load testing (load_test_http)

### Inconsistencies found

1. **`DomainDescriptorPy`** is registered in `_core` as `DomainDescriptorPy` but re-exported as `DomainDescriptor` in `__init__.py`. The `api_surface()` function registers it as `DomainDescriptorPy` (not `DomainDescriptor`). This is a naming inconsistency: the api_surface key uses the internal Py-suffixed name while the public symbol uses the clean name. Other Py-suffixed classes (e.g., `ExecutionSurfacePy`) are NOT in api_surface at all, making this the only case of the internal name leaking.

### Naming conventions are otherwise consistent

No other inconsistencies found. All public function names follow snake_case, all class names follow CamelCase, and prefix conventions are applied uniformly.

---

## C. Exception Hierarchy

```
Exception
  EggsecError                    (base for all eggsec errors)
    ConfigError                  (configuration validation failures)
    ScopeError                   (scope rule violations)
    EnforcementError             (enforcement/policy denials)
    NetworkError                 (HTTP, DNS, connection failures)
    ScanError                    (scan execution failures)
    TimeoutError                 (operation timeouts)
    FeatureUnavailableError      (feature not compiled in)
    SerializationError           (JSON/parse failures)
    InternalError                (internal engine errors)
```

**Total:** 10 exception types (1 base + 9 subtypes).

### Coverage

- `ConfigError`: raised for configuration and validation errors (mapped from `EggsecError::Config`, `EggsecError::Validation`).
- `ScopeError`: available but not directly mapped from engine errors (scope violations use `EnforcementError`).
- `EnforcementError`: raised for scope violations and invalid targets.
- `NetworkError`: raised for HTTP, DNS, rate limiting, address parse failures.
- `ScanError`: raised for scan failures, payloads, IO, proxy, recon, fingerprint, load test errors.
- `TimeoutError`: raised for operation timeouts.
- `FeatureUnavailableError`: available for feature-gated operation rejection (used in Python-side guards).
- `SerializationError`: raised for JSON/parse errors.
- `InternalError`: raised for internal engine errors.

All error paths from the Rust engine are mapped to Python exceptions via `engine_error_to_pyerr()` in `src/error.rs:17-51`.

---

## D. Type Consistency

### to_dict() / to_json() / __repr__

Verified on the following core types (all present in release hardening tests):

| Type | to_dict | to_json | __repr__ |
|------|---------|---------|----------|
| Finding | yes | yes | yes |
| CvssScore | yes | yes | yes |
| EventEnvelope | yes | yes | yes |
| ExecutionEvent | yes | yes | yes |
| CancellationToken | yes | yes | yes |
| ExecutionHandle | yes | yes | yes |
| DomainDescriptor | yes | -- | yes |
| EventStream | -- | -- | -- |

### Frozen DTOs with __hash__ and __eq__

| Type | frozen | __hash__ | __eq__ |
|------|--------|----------|--------|
| Severity | yes | yes (derive) | yes (derive) |
| Evidence | yes | -- | yes (derive) |
| SensitiveString | yes | yes (manual) | yes (manual) |
| HttpConfig | yes | -- | -- |
| ScanConfig | yes | -- | -- |
| Scope | yes | -- | -- |
| DomainDescriptor | yes | -- | -- |
| EventEnvelope | -- | -- | -- |

Note: Many frozen DTOs derive `PartialEq`/`Hash` from Rust's `#[derive]` but don't expose `__hash__` to Python. Only `Severity` and `SensitiveString` explicitly expose Python-level `__hash__`.

### Context manager support on session types

| Type | __enter__ | __exit__ |
|------|-----------|----------|
| ExecutionHandle | yes | yes |
| Client | -- | -- |
| AsyncClient | -- | -- |

`ExecutionHandle` supports context manager protocol. `Client` and `AsyncClient` do not (they are not resource-holders that require cleanup).

---

## E. Feature Behavior

### feature_matrix() coverage

The `feature_matrix()` function (in `src/features.rs:88-289`) returns 32 features:

**Always available (10):**
core, scanner, async-api, endpoint-discovery, service-fingerprinting, waf-detection, waf-validation, http-fuzzing, load-testing, findings-reporting

**Feature-gated (22):**
websocket, git-secrets, sbom, db-pentest, db-pentest-mongodb, db-pentest-redis, web-proxy, mobile, mobile-dynamic, packet-inspection, stress-testing, nse, container, daemon-client, headless-browser, advanced-hunting, compliance, wireless, evasion, postex, c2, ai-integration

**Note:** The `features()` function returns only 24 features (10 always + 14 feature-gated), while `feature_matrix()` returns all 32. The difference is that `feature_matrix()` includes headless-browser, advanced-hunting, compliance, wireless, evasion, postex, c2, and ai-integration which are defined only in `feature_matrix()`, not in `features()`.

### has_feature() coverage

`has_feature()` in `src/features.rs:53-83` handles the same 24 names as `features()`. The 8 additional features in `feature_matrix()` (headless-browser, advanced-hunting, compliance, wireless, evasion, postex, c2, ai-integration) return `false` from `has_feature()` for unknown names.

### Feature-gated imports are graceful

Feature-gated symbols in `__init__.py` use `try/except AttributeError` blocks, so missing features don't cause import errors. Verified for: websocket, git-secrets, sbom, db-pentest, mobile, packet-inspection, stress-testing, nse, daemon-client, container, proxy, web-proxy, headless-browser, advanced-hunting, compliance, wireless, evasion, postex, c2, ai-integration.

---

## F. Documentation Coverage

### docs/python/ files

| File | Status | Purpose |
|------|--------|---------|
| `index.md` | exists | Package overview |
| `quickstart.md` | exists | Getting started guide |
| `installation.md` | exists | Install instructions |
| `scope-and-safety.md` | exists | Scope enforcement details |
| `scanner.md` | exists | Port scanning guide |
| `endpoint-discovery.md` | exists | Endpoint discovery guide |
| `service-fingerprinting.md` | exists | Service fingerprinting guide |
| `recon.md` | exists | Reconnaissance guide |
| `waf.md` | exists | WAF detection guide |
| `reports.md` | exists | Findings and reporting guide |
| `sync-api.md` | exists | Sync API patterns |
| `async-api.md` | exists | Async API patterns |
| `api-reference.md` | exists | Full API reference (2355 lines) |
| `packaging.md` | exists | Distribution/packaging notes |
| `namespace.md` | exists | Namespace and stability policy |
| `versioning.md` | exists | Versioning and governance |
| `events.md` | exists | Event protocol documentation |
| `callbacks.md` | exists | Callbacks and sinks documentation |

### api-reference.md coverage

The `api-reference.md` file is 2355 lines and covers all public functions and classes organized by category (Module-level functions, Classes, Enums, Exceptions). It includes parameter tables, return types, and raise conditions.

### Guides exist for major workflows

All major workflows have dedicated guides: scanning, recon, WAF, findings, events, callbacks, sync/async patterns, configuration, packaging, and versioning.

---

## G. Migration Path

### Changes from pre-Milestone-G API

Milestone G added the following without breaking existing APIs:

1. **G1**: `DomainDescriptor`, `DomainRegistry` (new classes, additive)
2. **G2**: `EventEnvelope`, `EventStream`, 9 typed event payloads, `wrap_event`, `EVENT_SCHEMA_VERSION`, `event_stream_from_legacy` (new classes/functions, additive)
3. **G3**: `AuditSink`, `FindingSink`, `ArtifactSink`, `ProgressSink`, `EventConsumer`, `AsyncCallback`, `CallbackScheduler`, `BackpressureChannel` (new classes, additive)
4. **G4**: pathlib support, datetime conversion, hash/eq on frozen DTOs, context managers on `ExecutionHandle`, async iterators (`EventStreamAsyncIterator`, `FindingStreamAsyncIterator`), pickle support (enhancements, non-breaking)
5. **G5**: `BinaryBuffer`, `ArtifactMeta`, `LazyArtifact`, `PaginatedResults` (new classes, additive)
6. **G6**: `api_surface()`, `feature_matrix()`, `DeprecatedWarning`, `deprecated_warning()`, `experimental` namespace (new introspection, additive)
7. **G7**: `SCHEMA_VERSION`, `PROTOCOL_VERSION`, `ABI_VERSION`, `api_surface_version()` (new constants/functions, additive)
8. **G8**: Documentation additions (non-breaking)
9. **G9**: Wheel profiles documented (non-breaking)
10. **G10**: Release hardening tests (test additions only)
11. **G11**: Performance gate tests (test additions only)

### Backward-incompatible changes

None. All Milestone G additions are purely additive. The `deprecated_warning` function is marked deprecated in `api_surface()` but remains importable and functional.

### Deprecated APIs

| API | Replacement |
|-----|-------------|
| `deprecated_warning(msg)` | `warnings.warn(msg, DeprecationWarning)` directly |
| `DeprecatedWarning` class | Use `DeprecationWarning` from stdlib |

---

## H. Security Semantics

### Scope enforcement

Documented in `docs/python/scope-and-safety.md`. Scope enforcement is mandatory for all automated surfaces (MCP, Agent, CI). Manual surfaces (CLI/TUI) support overrides via `ManualOverride`. `EnforcementContext::evaluate()` is the mandatory pre-dispatch gate.

### SensitiveString handling

`SensitiveString` (`src/config_model.rs:8-53`):
- `__repr__` returns `"SensitiveString([REDACTED])"`
- `__str__` returns `"[REDACTED]"`
- The actual secret is only accessible via `expose_secret()`
- The class is frozen (immutable after creation)

### Secrets in repr/str

All `SensitiveString` representations are redacted. No other types handle secrets directly. `proxy_auth` in `HttpConfig` is stored as `Option<String>` (not `SensitiveString`) and is not included in repr — this is a known limitation.

---

## I. Packaging Readiness

### Wheel profiles

Documented in `docs/python/versioning.md` (section 7) and `docs/python/packaging.md` (section "Wheel profiles"):

| Profile | Features | System Deps |
|---------|----------|-------------|
| default | core, scanner, async-api, endpoint-discovery, service-fingerprinting, waf-detection, waf-validation, http-fuzzing, load-testing, findings-reporting | None |
| full-no-system | default + websocket, git-secrets, sbom, container | None |
| full | everything | Varies |

### Python version support

| Version | Support |
|---------|---------|
| 3.9 | Minimum supported |
| 3.10 | Supported |
| 3.11 | Supported |
| 3.12 | Fully tested (primary CI target) |
| 3.13 | Supported (forward-compatible) |

### Platform support

| Platform | Architecture | Status |
|----------|-------------|--------|
| Linux | x86_64 | Primary (CI tested) |
| Linux | aarch64 | Supported |
| macOS | arm64 (Apple Silicon) | Primary (CI tested) |
| macOS | x86_64 (Intel) | Supported |
| Windows | x86_64 | Experimental (no CI) |

---

## J. CI Coverage

### Test files and purpose

| File | Purpose |
|------|---------|
| `test_import.py` | Basic import verification |
| `test_smoke.py` | Smoke tests for core functionality |
| `test_scope.py` | Scope creation and validation |
| `test_scan_ports.py` | Port scanning functionality |
| `test_endpoint.py` | Endpoint discovery |
| `test_fingerprint.py` | Service fingerprinting |
| `test_dto.py` | Data transfer object construction |
| `test_async.py` | Async API contract |
| `test_policy_equivalence.py` | Execution policy defaults and equivalence |
| `test_enforcement.py` | Enforcement context and evaluation |
| `test_ergonomics.py` | pathlib, datetime, hash/eq ergonomics |
| `test_callbacks.py` | Callback sink types |
| `test_buffer.py` | BinaryBuffer and PaginatedResults |
| `test_milestone_c.py` | Milestone C (consolidated recon, GraphQL, OAuth, auth, browser, hunt) |
| `test_milestone_e.py` | Milestone E (versioned findings, artifacts, workflow, repository, baselines, reporting, compliance, integrations, migration) |
| `test_milestone_f.py` | Milestone F (wireless, evasion, postex, c2, distributed, notifications, AI) |
| `test_release_hardening.py` | G10: 117 tests covering export parity, API surface snapshot, import profiles, sync/async parity, cancellation/leak/shutdown, policy equivalence, serialization, deprecation, version metadata, event schema, callback contracts |
| `test_performance_gates.py` | G11: 13 performance benchmarks with regression budgets |
| `test_wheel_profiles.py` | Wheel profile validation |

### Release hardening tests exist

Yes. `test_release_hardening.py` contains 117 tests across 11 test classes covering: runtime/stub export parity, API surface snapshot, import profiles, sync/async contract parity, cancellation/leak/shutdown, policy equivalence, serialization compatibility, deprecation warnings, version metadata, event schema, and callback contracts.

### Performance budget tests exist

Yes. `test_performance_gates.py` contains 13 performance benchmarks with regression budgets defined in `performance_budgets.json`. Tests measure import overhead, Scope creation, `api_surface()` call, `feature_matrix()` call, EventEnvelope creation, `has_feature()` calls, `features()` call, `build_info()` call, CancellationToken operations, and Finding serialization.
