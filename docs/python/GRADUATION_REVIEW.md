# Python Domain Graduation Review

Release 5 Phase F, Workstream F9

This document evaluates each domain against the graduation checklist from
`domain-maturity.md` and provides evidence-backed decisions for stability
classification.

## Graduation Checklist

1. Canonical operation ID and request/result DTO
2. Sync and async dispatch through the common policy gate
3. Structured errors, events, cancellation, and serialization tests
4. Deterministic fixtures and local/daemon contract coverage where relevant
5. Documentation, type stubs, and wheel-profile coverage

---

## Stable Domains

### 1. stable-core (Original Ten Operations)

| Criterion | Evidence | Status | Notes |
|-----------|----------|--------|-------|
| Operation IDs & DTOs | 10 canonical IDs in `ALL_STABLE_OPERATION_IDS` (`test_golden_contract.py:41-64`). Request/result DTOs for all: `PortScanRequest`/`PortScanResult`, `EndpointScanRequest`/`EndpointScanResult`, etc. | PASS | Verified via `ToolRegistry.get(op_id)` for all 10 |
| Policy gate (sync) | `test_stable_core_fixtures.py:96-172` — all 10 operations dispatched via `engine.run(OperationRequest(...))` with scope enforcement | PASS | All return structured `OperationResult` |
| Policy gate (async) | `test_stable_core_fixtures.py:175-245` — all 10 operations dispatched via `async_engine.run(...)` with normalized equivalence verification | PASS | Sync/async produce identical normalized output |
| Error tests | `test_stable_core_fixtures.py:70-93` — all 10 operations produce `scope_denial` error with structured `OperationError.kind == "scope_denial"` on out-of-scope target | PASS | Audit events verified (1 event, `allowed=false`, `redacted=true`) |
| Cancellation | `test_cancellation_contract.py` — CancellationToken lifecycle (create/cancel/reset), engine-level cancellation, async detachment, resource leak prevention, latency < 10ms | PASS | 15+ tests across 5 test classes |
| Events | `test_events_cancellation.py:50-80` — pipeline emits `StageLifecycleEvent` with monotonic sequence IDs; `pipeline.started` → `step.started` → `step.completed` → `pipeline.completed` | PASS | Timestamp monotonicity verified |
| Serialization | `test_serialization.py:43-80` — `OperationRequest`, all typed request DTOs (`PortScanRequest`, `EndpointScanRequest`, etc.) survive JSON round-trip | PASS | to_dict() and to_json() verified |
| Fixtures | `tests/fixtures/stable_core.py` — `StableCoreFixtures` context manager provides loopback TCP, TLS, HTTP servers with deterministic ports | PASS | Used by `test_stable_core_fixtures.py` |
| Documentation/stubs | `eggsec` top-level package exports all 10 operations with `__all__`; type stubs (`.pyi`) generated from Rust bindings | PASS | All importable as `eggsec.scan_ports`, `eggsec.async_scan_ports`, etc. |
| Wheel profile | `default-wheel` profile (`profiles.json`) requires `required_min_tests: 1977`, `max_skips: 0` | PASS | Core profile always runs |

**Decision: RETAIN STABLE** — All checklist items satisfied with comprehensive evidence.

---

### 2. git-secrets

| Criterion | Evidence | Status | Notes |
|-----------|----------|--------|-------|
| Operation ID & DTO | `scan_git_secrets` — canonical ID in `ALL_STABLE_OPERATION_IDS`. Returns `GitSecretsReport` with `to_dict()`/`to_json()` | PASS | Feature gate: `git-secrets` |
| Policy gate (sync) | `test_feature_enabled_profiles.py:137-153` — `engine.run(OperationRequest("scan_git_secrets", ...))` returns `OperationResult` with `payload_type_name == "GitSecretsReport"` | PASS | |
| Policy gate (async) | `test_feature_enabled_profiles.py:157-164` — `async_engine.run(...)` returns `PyFuture` resolving to `OperationResult` | PASS | |
| Error tests | `test_feature_enabled_profiles.py:197-215` — scope denial produces `error.kind == "scope_denial"`; nonexistent path produces structured error | PASS | |
| Cancellation | `test_feature_enabled_profiles.py:219-233` — pre-cancelled `CancellationToken` produces `result.status.name() == "Cancelled"` via Pipeline | PASS | |
| Serialization | `test_feature_enabled_profiles.py:168-193` — `to_dict()` returns dict with `repo_path`, `findings`, `summary`; `to_json()` produces valid JSON; summary sub-object serialization verified | PASS | |
| Fixtures | `test_feature_enabled_profiles.py:94-114` — `git_repo` fixture creates temp git repo with known secret pattern (AWS key, database URL) | PASS | Deterministic |
| Documentation/stubs | Exported as `eggsec.scan_git_secrets` and `eggsec.async_scan_git_secrets` | PASS | |
| Wheel profile | `git-secrets` profile (`profiles.json`) — `cargo_features: ["git-secrets"]`, `system_packages: ["git"]` | PASS | |

**Decision: RETAIN STABLE** — All checklist items satisfied.

---

### 3. sbom

| Criterion | Evidence | Status | Notes |
|-----------|----------|--------|-------|
| Operation ID & DTO | `generate_sbom` — canonical ID. Returns `SbomReport` with `components` list | PASS | Feature gate: `sbom` |
| Policy gate (sync) | `test_feature_enabled_profiles.py:279-289` — `engine.run(OperationRequest("generate_sbom", ...))` returns `OperationResult` with `payload_type_name == "SbomReport"` | PASS | |
| Policy gate (async) | `test_feature_enabled_profiles.py:293-303` — async dispatch via `async_engine.run(...)` | PASS | |
| Error tests | `test_feature_enabled_profiles.py:334-354` — scope denial and nonexistent path both produce structured errors | PASS | |
| Cancellation | `test_feature_enabled_profiles.py:358-373` — pre-cancelled token produces `Cancelled` status | PASS | |
| Serialization | `test_feature_enabled_profiles.py:307-330` — `to_dict()`/`to_json()` on report and component sub-objects | PASS | |
| Fixtures | `test_feature_enabled_profiles.py:254-259` — uses workspace `Cargo.toml` as deterministic cargo project | PASS | |
| Documentation/stubs | Exported as `eggsec.generate_sbom` and `eggsec.async_generate_sbom` | PASS | |
| Wheel profile | `sbom` profile (`profiles.json`) — `cargo_features: ["sbom"]` | PASS | |

**Decision: RETAIN STABLE** — All checklist items satisfied.

---

### 4. consolidated-recon

| Criterion | Evidence | Status | Notes |
|-----------|----------|--------|-------|
| Operation ID & DTO | `run_consolidated_recon` — canonical ID. Returns `ConsolidatedReconReport` with `ReconModuleResult` sub-objects | PASS | No feature gate (always available) |
| Policy gate (sync) | `test_direct_engine_equivalence.py:750-760` — `engine.run(OperationRequest("run_consolidated_recon", ...))` returns `OperationResult` with `payload_type_name == "ConsolidatedReconReport"` | PASS | |
| Policy gate (async) | Listed in `ALL_STABLE_OPERATION_IDS`; `async_run_consolidated_recon` callable (`test_milestone_c.py:222-226`) | PASS | |
| Error tests | Included in `test_stable_core_fixtures.py:70-93` scope denial verification | PASS | |
| Cancellation | Covered by pipeline cancellation tests in `test_events_cancellation.py` | PASS | |
| Serialization | Included in `test_serialization.py` request round-trip tests | PASS | |
| Fixtures | `StableCoreFixtures` provides HTTP server target for consolidated recon | PASS | |
| Documentation/stubs | Exported as `eggsec.run_consolidated_recon` and `eggsec.async_run_consolidated_recon` | PASS | |
| Wheel profile | `default-wheel` profile covers this operation (always available) | PASS | |

**Decision: RETAIN STABLE** — All checklist items satisfied.

---

### 5. graphql

| Criterion | Evidence | Status | Notes |
|-----------|----------|--------|-------|
| Operation ID & DTO | `graphql_test` — canonical ID. DTOs: `GraphQLVulnerability`, `GraphQLTestResult`, `GraphQLType`, `GraphQLField`, `GraphQLArg`, `GraphQLInputField`, `GraphQLSchema`, `GraphQLTestConfig` | PASS | No feature gate |
| Policy gate (sync) | `test_direct_engine_equivalence.py:762-770` — `engine.run(OperationRequest("graphql_test", ...))` returns `OperationResult` | PASS | |
| Policy gate (async) | `async_graphql_test` callable (`test_milestone_c.py:238-242`) | PASS | |
| Error tests | Included in scope denial verification; target-agnostic operations succeed under `deny_all` (`test_daemon_contract.py:196-206`) | PASS | |
| Cancellation | Covered by pipeline cancellation tests | PASS | |
| Serialization | Request round-trip in `test_serialization.py`; `OperationRequest("graphql_test", ...)` verified | PASS | |
| Fixtures | Uses `StableCoreFixtures` HTTP server; graphql endpoint tested against loopback | PASS | |
| Documentation/stubs | Exported as `eggsec.graphql_test` and `eggsec.async_graphql_test` | PASS | |
| Wheel profile | `default-wheel` profile covers this operation | PASS | |

**Decision: RETAIN STABLE** — All checklist items satisfied.

---

### 6. oauth

| Criterion | Evidence | Status | Notes |
|-----------|----------|--------|-------|
| Operation ID & DTO | `oauth_test` — canonical ID. DTOs: `OAuthVulnerability`, `OAuthEndpointKind`, `OAuthEndpoint`, `OAuthTestResult`, `OAuthTestConfig` | PASS | No feature gate |
| Policy gate (sync) | `test_direct_engine_equivalence.py:772-779` — engine dispatch verified | PASS | |
| Policy gate (async) | `async_oauth_test` callable (`test_milestone_c.py:252-257`) | PASS | |
| Error tests | Scope denial and target-agnostic behavior verified | PASS | |
| Cancellation | Covered by pipeline cancellation tests | PASS | |
| Serialization | Request round-trip in `test_serialization.py` | PASS | |
| Fixtures | Uses `StableCoreFixtures` HTTP server | PASS | |
| Documentation/stubs | Exported as `eggsec.oauth_test` and `eggsec.async_oauth_test`; also `eggsec.oauth_discover_endpoints` | PASS | |
| Wheel profile | `default-wheel` profile covers this operation | PASS | |

**Decision: RETAIN STABLE** — All checklist items satisfied.

---

### 7. authentication

| Criterion | Evidence | Status | Notes |
|-----------|----------|--------|-------|
| Operation ID & DTO | `auth_test` — canonical ID. DTOs: `AuthTestType`, `AuthFinding`, `AuthTestConfig`, `AuthTestReport` | PASS | No feature gate |
| Policy gate (sync) | `test_direct_engine_equivalence.py:781-788` — engine dispatch verified | PASS | |
| Policy gate (async) | `async_auth_test` callable (`test_milestone_c.py:265-269`) | PASS | |
| Error tests | Scope denial verified; target-agnostic operations succeed under `deny_all` | PASS | |
| Cancellation | Covered by pipeline cancellation tests | PASS | |
| Serialization | Request round-trip in `test_serialization.py` | PASS | |
| Fixtures | Uses `StableCoreFixtures` HTTP server | PASS | |
| Documentation/stubs | Exported as `eggsec.auth_test` and `eggsec.async_auth_test` | PASS | |
| Wheel profile | `default-wheel` profile covers this operation | PASS | |

**Decision: RETAIN STABLE** — All checklist items satisfied.

---

### 8. database

| Criterion | Evidence | Status | Notes |
|-----------|----------|--------|-------|
| Operation ID & DTO | `db_probe` — canonical ID. Returns `DbPentestReport` with findings, driver info | PASS | Feature gate: `db-pentest` |
| Policy gate (sync) | `test_feature_enabled_profiles.py:429-451` — `engine.run(OperationRequest("db_probe", ...))` returns `OperationResult` with structured response | PASS | Connection-refused handled gracefully |
| Policy gate (async) | `test_feature_enabled_profiles.py:455-466` — async dispatch verified | PASS | |
| Error tests | `test_feature_enabled_profiles.py:500-529` — scope denial (`error.kind == "scope_denial"`), invalid db_type returns structured error, nonexistent port returns error | PASS | |
| Cancellation | `test_feature_enabled_profiles.py:533-549` — pre-cancelled token produces `Cancelled` status | PASS | |
| Serialization | `test_feature_enabled_profiles.py:470-496` — `to_dict()`/`to_json()` on report and finding sub-objects | PASS | |
| Fixtures | `test_feature_enabled_profiles.py:399-426` — dry-run mode with non-existent ports; `DbDriverRegistry` lists 5 drivers (postgres, mysql, mssql, mongodb, redis) | PASS | No live DB required for contract tests |
| Documentation/stubs | Exported as `eggsec.db_probe`, `eggsec.async_db_probe`, `eggsec.db_list_drivers`, `eggsec.db_get_capabilities` | PASS | |
| Wheel profile | `db-postgres`, `db-mysql`, `db-redis`, `db-mongodb` profiles (`profiles.json`) | PASS | |

**Decision: RETAIN STABLE** — All checklist items satisfied.

---

### 9. nse

| Criterion | Evidence | Status | Notes |
|-----------|----------|--------|-------|
| Operation ID & DTO | `nse_run` — canonical ID. Returns `NseReport` with output, warnings, errors, libraries, rules | PASS | Feature gate: `nse` |
| Policy gate (sync) | `test_feature_enabled_profiles.py:622-642` — `engine.run(OperationRequest("nse_run", ...))` returns `OperationResult` with `payload_type_name == "NseReport"` | PASS | |
| Policy gate (async) | `test_feature_enabled_profiles.py:646-657` — async dispatch verified | PASS | |
| Error tests | `test_feature_enabled_profiles.py:700-724` — scope denial; invalid script name returns structured error | PASS | |
| Cancellation | `test_feature_enabled_profiles.py:728-744` — pre-cancelled token produces `Cancelled` status | PASS | |
| Serialization | `test_feature_enabled_profiles.py:661-696` — `to_dict()`/`to_json()` on report, libraries, and rules sub-objects | PASS | |
| Fixtures | `test_nse.py` — 1646 lines of NSE-specific tests: `NseHostContext`, `NsePortContext`, `NseRuntime`, `NseExecutionLimits`, `NseCancellationToken`, `NseLibraryRegistry` | PASS | Comprehensive runtime validation |
| Documentation/stubs | Exported as `eggsec.nse_run`, `eggsec.async_nse_run`, `eggsec.nse_list_libraries`, `eggsec.nse_list_scripts`, `eggsec.nse_get_script_metadata` | PASS | |
| Wheel profile | `nse` profile (`profiles.json`) — `cargo_features: ["nse"]`, `system_packages: ["libssl-dev"]` | PASS | |

**Decision: RETAIN STABLE** — All checklist items satisfied. NSE has the most comprehensive test surface of any domain (1646 lines).

---

### 10. container

| Criterion | Evidence | Status | Notes |
|-----------|----------|--------|-------|
| Operation IDs & DTOs | `scan_docker_image` and `scan_kubernetes` — both canonical IDs. Returns `KubernetesScanResult` / container scan result with findings | PASS | Feature gate: `container` |
| Policy gate (sync) | `test_feature_enabled_profiles.py:842-861` — both operations dispatched via `engine.run(OperationRequest(...))` | PASS | K8s uses static manifest fixture; Docker uses nonexistent image for graceful failure |
| Policy gate (async) | `test_feature_enabled_profiles.py:865-874` — async K8s dispatch verified | PASS | |
| Error tests | `test_feature_enabled_profiles.py:898-918` — scope denial; nonexistent manifest returns structured error | PASS | |
| Cancellation | `test_feature_enabled_profiles.py:922-936` — pre-cancelled token produces `Cancelled` status | PASS | |
| Serialization | `test_feature_enabled_profiles.py:878-894` — `to_dict()`/`to_json()` on report and finding sub-objects | PASS | |
| Fixtures | `test_feature_enabled_profiles.py:792-822` — `k8s_manifest` fixture creates minimal K8s Deployment YAML with security context findings | PASS | Deterministic |
| Documentation/stubs | Exported as `eggsec.scan_docker_image`, `eggsec.scan_kubernetes`, and async variants | PASS | |
| Wheel profile | `container` profile (`profiles.json`) — `cargo_features: ["container"]` | PASS | |

**Decision: RETAIN STABLE** — All checklist items satisfied.

---

### 11. mobile-static

| Criterion | Evidence | Status | Notes |
|-----------|----------|--------|-------|
| Operation IDs & DTOs | `analyze_apk` and `analyze_ipa` — both canonical IDs. Returns `MobileScanReport` with findings | PASS | Feature gate: `mobile` |
| Policy gate (sync) | `test_feature_enabled_profiles.py:1013-1033` — both operations dispatched via `engine.run(OperationRequest(...))` | PASS | |
| Policy gate (async) | `test_feature_enabled_profiles.py:1037-1057` — both async dispatches verified | PASS | |
| Error tests | `test_feature_enabled_profiles.py:1090-1130` — scope denial for both APK and IPA; nonexistent paths return structured errors | PASS | |
| Cancellation | `test_feature_enabled_profiles.py:1134-1146` — pre-cancelled token produces `Cancelled` status | PASS | |
| Serialization | `test_feature_enabled_profiles.py:1061-1086` — `to_dict()`/`to_json()` on report and finding sub-objects for both APK and IPA | PASS | |
| Fixtures | `test_feature_enabled_profiles.py:954-976` — synthetic APK (ZIP with AndroidManifest.xml + classes.dex) and IPA (ZIP with Info.plist) created deterministically | PASS | No emulator required |
| Documentation/stubs | Exported as `eggsec.analyze_apk`, `eggsec.analyze_ipa`, and async variants | PASS | |
| Wheel profile | `mobile-static` profile (`profiles.json`) — `cargo_features: ["mobile"]`, no system deps | PASS | |

**Decision: RETAIN STABLE** — All checklist items satisfied.

---

## Conditional Graduation Candidates

### 12. browser (headless-browser)

| Criterion | Evidence | Status | Notes |
|-----------|----------|--------|-------|
| Operation IDs & DTOs | `browser_test` / `async_browser_test` — callable but NOT in `ALL_STABLE_OPERATION_IDS`. DTOs exist: `BrowserTestConfig`, `BrowserTestReport`, `DomXssFinding`, `SpaRoute`, `ClientIssue` | PARTIAL | Types exist but no canonical operation ID in the 22-operation registry |
| Policy gate (sync) | No engine dispatch tests for `browser_test` as a registered operation. `test_browser_operational.py` tests session types only (construction, serialization, state transitions) | PARTIAL | Session lifecycle works; no operation-level dispatch |
| Policy gate (async) | `async_browser_test` callable (`test_milestone_c.py:284-288`) but not exercised through engine dispatch | PARTIAL | |
| Error tests | `test_browser_operational.py:1328-1337` — `WebSocketSessionPy` connection to non-existent server raises `NetworkError` gracefully | PASS | Session-level errors verified |
| Cancellation | No dedicated cancellation tests for browser operations | GAP | |
| Serialization | `test_browser_operational.py` — extensive DTO construction/serialization for session state, capabilities, config | PASS | 1375 lines of session type tests |
| Fixtures | No deterministic fixtures; requires real browser backend (123 skipped in CI per WS6) | GAP | |
| Documentation/stubs | Types accessible via `eggsec` top-level and `eggsec.sessions` submodule | PASS | |
| Wheel profile | `headless-browser` profile exists (`profiles.json`) but `blocking: false` | PARTIAL | Non-blocking indicates incomplete confidence |

**Gaps:**
1. No canonical operation ID in the 22-operation registry
2. No engine-level dispatch tests (sync/async) through the policy gate
3. No cancellation tests
4. 123 tests skipped in CI (requires real browser backend)
5. Profile is non-blocking

**Decision: KEEP PROVISIONAL** — Session types are well-tested (1375 lines) but the operation lacks canonical registry entry, engine dispatch coverage, and deterministic fixtures. Graduate to stable when: (a) `browser_test` is added to the 22-operation registry with canonical ID, (b) engine dispatch tests are added for sync/async, (c) a mock/stub browser backend is available for CI.

---

### 13. hunt (advanced-hunting)

| Criterion | Evidence | Status | Notes |
|-----------|----------|--------|-------|
| Operation IDs & DTOs | `hunt_test` / `async_hunt_test` — callable but NOT in `ALL_STABLE_OPERATION_IDS`. DTOs: `HuntTestConfig`, `HuntReport`, `AttackChain`, `BusinessLogicFlaw`, `RaceCondition`, `AuthzBypass`, `SessionIssue` | PARTIAL | Types exist but no canonical operation ID |
| Policy gate (sync) | No engine dispatch tests for `hunt_test`. `test_milestone_c.py:187-209` tests `HuntTestConfig` construction only | GAP | No dispatch through policy gate |
| Policy gate (async) | `async_hunt_test` callable (`test_milestone_c.py:307-313`) but not exercised | GAP | |
| Error tests | No dedicated error tests | GAP | |
| Cancellation | No cancellation tests | GAP | |
| Serialization | Config construction/defaults verified; no full report serialization tests | PARTIAL | |
| Fixtures | No deterministic fixtures | GAP | |
| Documentation/stubs | Types accessible via `eggsec` top-level and `eggsec.experimental` submodule | PASS | |
| Wheel profile | Listed as feature-gate marker in `test_features.py:46` (`"advanced-hunting"`) | PARTIAL | No dedicated validation profile |

**Gaps:**
1. No canonical operation ID in the 22-operation registry
2. No engine dispatch tests (sync/async)
3. No error, cancellation, or serialization tests for full report
4. No deterministic fixtures
5. No dedicated validation profile

**Decision: KEEP PROVISIONAL** — Hunt types exist and are importable but lack operation-level integration. Graduate to stable when: (a) `hunt_test` is added to the 22-operation registry, (b) engine dispatch tests are added, (c) error/cancellation/serialization tests cover the full report lifecycle, (d) a validation profile is created.

---

## Provisional Domains

### 14. daemon

| Criterion | Evidence | Status | Notes |
|-----------|----------|--------|-------|
| Current test coverage | `test_daemon_contract.py` — 965 lines, 20 daemon-stable operations parameterized across contract classes (request normalization, scope denial, feature availability, payload types, error DTOs, timeouts, cancellation, event ordering, artifact metadata, serialization, checkpoint identity) | PASS | Local-side contract verified |
| Daemon integration | `test_daemon_integration.py` — daemon binary spawned as child process, connected via Unix socket, health/capabilities/session CRUD/close verified end-to-end | PASS | WS7: 6 passed, 3 skipped |
| Repository | `test_daemon_repository_operational.py` — 2000+ lines; SQLite/JSONL CRUD, concurrency, dedup, pagination, migration, corruption detection | PASS | WS8: 64+ tests |
| Missing evidence | Transport parity: reconnect, replay, result-retrieval semantics not closed. Event ordering across daemon reconnection unverified | GAP | Per domain-maturity.md: "not part of stable-core" |
| Wheel profile | `daemon-client` profile (`profiles.json`) — `blocking: false`, `schedule: "push"` | PARTIAL | Non-blocking |

**Decision: KEEP PROVISIONAL** — Strong local-side contract coverage (965 lines) and repository durability (64+ tests), but daemon transport parity (reconnect, replay, result retrieval) remains open. Graduate when transport parity milestone closes.

---

### 15. proxy (web-proxy)

| Criterion | Evidence | Status | Notes |
|-----------|----------|--------|-------|
| Current test coverage | `test_proxy.py` — 1006 lines; ProxyType, RotationStrategy, ProxyConfig, ProxyEntry, InterceptConfig, CapturedExchange, InterceptSessionResult, InterceptSessionState, InterceptStats, InterceptFilter, InterceptRule, CertificateAuthorityConfig, IssuedCertificate, HarEntry, HarDocument | PASS | Comprehensive type coverage |
| DTO construction/serialization | All proxy types verified for construction, `from_str()`, `to_dict()`, `to_json()`, repr, defaults | PASS | |
| Missing evidence | MITM interception semantics remain hazardous; no live proxy session tests; Python binding returns empty exchanges (documented limitation per WS3) | GAP | WS3: 78 passed, 12 skipped |
| Policy gate | No engine dispatch tests through `EnforcementContext` for proxy operations | GAP | |
| Wheel profile | `web-proxy` profile (`profiles.json`) — `blocking: false` | PARTIAL | |

**Decision: KEEP PROVISIONAL** — Comprehensive type coverage (1006 lines) but MITM semantics, live interception, and engine dispatch remain unverified. The Python binding returning empty exchanges is a documented limitation that blocks stable graduation.

---

### 16. packet-inspection

| Criterion | Evidence | Status | Notes |
|-----------|----------|--------|-------|
| Current test coverage | `test_feature_enabled_profiles.py:1386-1564` — 179 lines; `list_network_interfaces()`, `CaptureConfig`, `CaptureStats`, `PacketInfo`, `PacketFilter`, `PcapWriter` lifecycle, `parse_pcap` error handling | PASS | |
| Additional coverage | `test_packet_capture_probes.py`, `test_packet_validation.py` — packet parsing, Ethernet/IPv4/IPv6/TCP/UDP/ICMP headers | PASS | |
| Missing evidence | Live capture requires root/CAP_NET_RAW; no engine dispatch tests; `packet-parser` profile exists but `packet-live` and `active-probes` are `blocking: false`, `schedule: "manual"` | GAP | Platform/system dependency |
| Policy gate | No engine dispatch tests for packet operations as registered operations | GAP | |
| Wheel profile | `packet-parser` profile is `blocking: true`; `packet-live` and `active-probes` are non-blocking and manual | PARTIAL | |

**Decision: KEEP PROVISIONAL** — Parser types are well-tested but live capture requires elevated privileges, and there are no engine dispatch tests. Graduate when: (a) packet operations are registered as engine operations, (b) a CI-friendly mock capture path exists.

---

### 17. mobile-dynamic

| Criterion | Evidence | Status | Notes |
|-----------|----------|--------|-------|
| Current test coverage | `test_mobile_emulator.py` — 274 lines; `MobileSessionConfig`, `MobileSession`, `MobileDeviceDescriptor` construction; lifecycle state machine; ADB availability check | PARTIAL | |
| Missing evidence | 104 skipped in CI per WS5 (requires real Android emulator). No emulator available in CI. Dynamic testing (`mobile-dynamic` feature) requires ADB + running device | GAP | `mobile-emulator` profile: `blocking: false`, `schedule: "manual"` |
| Policy gate | No engine dispatch tests for dynamic mobile operations | GAP | |
| Wheel profile | `mobile-emulator` profile requires `system_packages: ["adb"]`, `services: ["emulator"]` | PARTIAL | Manual schedule |

**Decision: KEEP PROVISIONAL** — Static mobile analysis (APK/IPA) is stable, but dynamic testing requires an Android emulator that is not available in CI. All 104 dynamic tests skip. Graduate when a CI-suitable emulator or device farm is available.

---

## Experimental Domains

### 18. wireless

| Criterion | Evidence | Status | Notes |
|-----------|----------|--------|-------|
| Current state | `test_milestone_f.py:32-96` — `SecurityType` enum (7 variants), `WirelessNetwork` construction, `WirelessScanConfig` construction. All tests skip when `wireless` feature not enabled | PARTIAL | Type surface only |
| Missing evidence | No engine dispatch tests. Real scans require root and wireless interface. No operational tests | GAP | Platform-sensitive |
| Decision | **KEEP EXPERIMENTAL** — Type surface exists but no operational coverage. Requires root and wireless hardware for real testing. |

---

### 19. evasion

| Criterion | Evidence | Status | Notes |
|-----------|----------|--------|-------|
| Current state | `test_milestone_f.py:102-158` — `EvasionTargetType` (5 variants), `EvasionCategory` (6 variants), `EvasionRisk` (4 variants), `EvasionScanConfig` construction. MITRE ATT&CK mapped | PARTIAL | Type surface only |
| Missing evidence | No engine dispatch tests. `dry_run: True` by default. No operational tests against real targets | GAP | |
| Decision | **KEEP EXPERIMENTAL** — MITRE ATT&CK mapping is valuable but type surface only. No operational dispatch tests. |

---

### 20. postex

| Criterion | Evidence | Status | Notes |
|-----------|----------|--------|-------|
| Current state | `test_milestone_f.py:165-213` — `PostexCategory` (4 variants), `PostexRisk` (4 variants), `PostexProfile` (3 variants), `PostexScanConfig` construction. `dry_run: True` by default | PARTIAL | Type surface only |
| Missing evidence | No engine dispatch tests. No operational tests. Post-exploitation simulation requires target access | GAP | |
| Decision | **KEEP EXPERIMENTAL** — Type surface exists. No operational coverage. Post-exploitation simulation requires careful scoping before graduation. |

---

### 21. c2

| Criterion | Evidence | Status | Notes |
|-----------|----------|--------|-------|
| Current state | `test_milestone_f.py:219-291` — `BeaconProtocol` (5 variants), `C2TaskType` (7 variants), `C2TaskStatus` (4 variants), `OpsecCategory` (6 variants), `OpsecSeverity` (4 variants), `C2ScanConfig` construction. `dry_run: True` by default | PARTIAL | Type surface only |
| Missing evidence | No engine dispatch tests. No operational tests. C2 simulation requires target access and careful policy gating | GAP | Depends on postex + evasion |
| Decision | **KEEP EXPERIMENTAL** — Type surface exists. No operational coverage. C2 simulation is high-risk and requires explicit policy gating before any graduation. |

---

### 22. distributed

| Criterion | Evidence | Status | Notes |
|-----------|----------|--------|-------|
| Current state | `test_milestone_f.py:298-341` — `DistributedTaskType` (7 variants), `WorkerStatus` (3 variants), `distributed_task_types()` function, `distributed_generate_psk()` function | PARTIAL | Type surface only |
| Missing evidence | No engine dispatch tests. No cluster operational tests. Requires multi-node setup | GAP | |
| Decision | **KEEP EXPERIMENTAL** — Type surface exists. Cluster architecture requires multi-node testing infrastructure not available in CI. |

---

### 23. ai

| Criterion | Evidence | Status | Notes |
|-----------|----------|--------|-------|
| Current state | `test_milestone_f.py:440-500` — `AiProvider` (4 variants), `PluginLanguage` (3 variants), `AiCacheStats` construction, `AiCache` construction. Feature gate: `ai-integration` | PARTIAL | Type surface only |
| Missing evidence | No engine dispatch tests. LLM integration requires external API keys. No operational tests | GAP | |
| Decision | **KEEP EXPERIMENTAL** — Type surface exists. LLM integration depends on external services and API keys. No operational coverage. |

---

## Summary Table

| Domain | Classification | Decision | Key Evidence | Gaps |
|--------|---------------|----------|--------------|------|
| stable-core | stable | **RETAIN STABLE** | 22-operation registry, 1977+ tests, sync/async dispatch, scope denial, cancellation, serialization, fixtures | None |
| git-secrets | stable | **RETAIN STABLE** | Full dispatch coverage, deterministic git repo fixture, scope denial, cancellation, serialization | None |
| sbom | stable | **RETAIN STABLE** | Full dispatch coverage, workspace Cargo.toml fixture, scope denial, cancellation, serialization | None |
| consolidated-recon | stable | **RETAIN STABLE** | Engine dispatch verified, async callable, scope denial, cancellation, HTTP fixture | None |
| graphql | stable | **RETAIN STABLE** | Engine dispatch verified, async callable, target-agnostic behavior, scope denial | None |
| oauth | stable | **RETAIN STABLE** | Engine dispatch verified, async callable, scope denial, endpoint discovery | None |
| authentication | stable | **RETAIN STABLE** | Engine dispatch verified, async callable, scope denial, target-agnostic behavior | None |
| database | stable | **RETAIN STABLE** | Full dispatch coverage, driver registry (5 drivers), dry-run mode, scope denial, cancellation | None |
| nse | stable | **RETAIN STABLE** | 1646 lines of tests, full dispatch coverage, runtime/limits/cancellation/library verification | None |
| container | stable | **RETAIN STABLE** | Full dispatch coverage for K8s and Docker, K8s manifest fixture, scope denial, cancellation | None |
| mobile-static | stable | **RETAIN STABLE** | Full dispatch coverage for APK and IPA, synthetic fixtures, scope denial, cancellation | None |
| browser | provisional | **KEEP PROVISIONAL** | 1375 lines session type tests, DTO construction/serialization | No canonical op ID, no engine dispatch, no cancellation tests, 123 skipped |
| hunt | provisional | **KEEP PROVISIONAL** | Type surface exists, config construction | No canonical op ID, no engine dispatch, no fixtures, no validation profile |
| daemon | provisional | **KEEP PROVISIONAL** | 965-line contract tests, 64+ repository tests, daemon integration | Transport parity (reconnect, replay) open |
| proxy | provisional | **KEEP PROVISIONAL** | 1006 lines type coverage, comprehensive DTO tests | MITM semantics hazardous, empty exchanges limitation, no engine dispatch |
| packet-inspection | provisional | **KEEP PROVISIONAL** | Parser types well-tested, PcapWriter lifecycle | Live capture requires root, no engine dispatch |
| mobile-dynamic | provisional | **KEEP PROVISIONAL** | Session type tests, lifecycle state machine | 104 skipped (emulator required), no CI coverage |
| wireless | experimental | **KEEP EXPERIMENTAL** | Type surface (3 test classes) | No operational tests, requires root + wireless HW |
| evasion | experimental | **KEEP EXPERIMENTAL** | Type surface (4 test classes), MITRE ATT&CK mapped | No operational tests |
| postex | experimental | **KEEP EXPERIMENTAL** | Type surface (4 test classes) | No operational tests, high-risk domain |
| c2 | experimental | **KEEP EXPERIMENTAL** | Type surface (5 test classes) | No operational tests, depends on postex+evasion |
| distributed | experimental | **KEEP EXPERIMENTAL** | Type surface (2 test classes) | No cluster testing infrastructure |
| ai | experimental | **KEEP EXPERIMENTAL** | Type surface (3 test classes) | Requires external LLM APIs |
