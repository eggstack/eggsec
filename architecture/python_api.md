# Python API Deep Dive

`eggsec-python` is a host-language binding over the Rust engine. Its scoped
pre-1.0 stable core is intentionally smaller than the importable package.

## Role & Responsibilities

The crate bridges the Rust engine to Python via PyO3/maturin, exposing:

- **Engine/AsyncEngine**: primary dispatch surface for all 22 stable operations
- **Client/AsyncClient**: convenience wrappers over Engine
- **OperationExecutorRegistry**: canonical registry driving dispatch, feature
  gating, and daemon mapping
- **StableOperation enum**: exactly 22 variants enforced by exhaustiveness
  tests
- **Tool abstraction layer**: `eggsec-tool-core` types bound to Python with
  deterministic `ToolDescriptor` entries per operation
- **Event protocol**: versioned `EventEnvelope` with monotonic sequences,
  backpressure channels, and structured delivery stats
- **Provisional/experimental domains**: network primitives, sessions,
  repositories, proxy, NSE, database, mobile, browser, wireless, evasion,
  postex, C2, AI integration, stress testing

The Python package reorganizes symbols into intentional submodules by
capability ownership (Phase C), with backward-compatible top-level re-exports.

## Build & Install

### Maturin build

```bash
# Development build (installs into active venv)
cd crates/eggsec-python
maturin develop

# Release wheel
maturin build --release
```

### extension-module nuance

The `extension-module` Cargo feature (configured in `pyproject.toml`
`[tool.maturin] features = ["pyo3/extension-module"]`) enables PyO3's
`extension-module` flag, which disables libpython linkage. This is **required**
when building the Python extension via maturin, but **must stay off** for
`cargo test -p eggsec-python` — the test harness is a standalone executable
that needs libpython linked. The feature is therefore not in the `default`
Cargo features; maturin activates it at build time.

### Wheel profiles

| Profile | Command | Features |
|---------|---------|----------|
| `core` | `maturin build --release` | None (no optional features) |
| `full` | `maturin build --release --features full-no-system` | websocket, git-secrets, sbom, container |
| `full-with-system` | `maturin build --release --features websocket,git-secrets,sbom,container,...` | All non-system + system-dependent |

`full-no-system` = all features buildable without system library dependencies.

## Module Layout & Maturity Tiers

### Rust source (104 files in `crates/eggsec-python/src/`)

```
src/
  lib.rs                     # PyO3 #[pymodule] _core — exports 500+ symbols
  engine.rs                  # Engine (sync dispatch)
  async_engine.rs            # AsyncEngine (async dispatch)
  client.rs / async_client.rs# Client/AsyncClient convenience wrappers
  operation_registry.rs      # StableOperation enum (22 variants), registry, descriptors
  operation_executors.rs     # Executor trait wiring
  operation_metadata.rs      # OperationMetadataView, OperationRegistry pyclass
  dispatch_helpers.rs        # pre_dispatch_lifecycle, post_dispatch_hooks, result builders
  engine_state.rs            # Shared EngineState (scope, registry, events, audit)
  runtime_async.rs           # Process-global OnceLock<Runtime>, PyFuture
  runtime_sync.rs            # Separate OnceLock<Runtime>, block_on with GIL release
  error.rs                   # Exception hierarchy (11 types)
  dto.rs, endpoint.rs, fingerprint.rs, recon.rs, waf.rs, ...  # Typed result DTOs
  requests.rs                # OperationRequest and per-op request types
  status.rs                  # OperationResult, OperationError, ExecutionStats
  event_protocol.rs          # EventEnvelope, typed event variants
  event_stream.rs            # EventStream, legacy bridge
  backpressure.rs            # PyBackpressureChannel, EventDeliveryStats
  callbacks.rs               # AuditSink, FindingSink, ArtifactSink, ProgressSink
  async_support.rs           # AsyncCallback, CallbackScheduler
  cancellation.rs            # CancellationToken
  tool_core.rs               # eggsec-tool-core Python bindings
  tool_descriptor.rs         # ToolDescriptorPy, ToolRegistryPy, SchemaGeneratorPy
  domains.rs                 # DomainDescriptor, domain_maturity()
  scope.rs, scope_eval.rs    # Scope, LoadedScope, scope validation
  config_model.rs            # PyEggsecConfig and sub-configs
  execution_context.rs       # EnforcementContext, ApprovedOperation
  authorization.rs           # ExecutionPolicy, ManualOverride
  preflight.rs               # preflight_operation
  audit.rs                   # AuditOutcome, EnforcementAuditEvent
  pipeline.rs                # Pipeline, AsyncPipeline, PipelineStep
  planning.rs                # ScanPlan, PlanStep
  checkpoint.rs, checkpoint_store.rs  # Checkpoint contract
  # Feature-gated modules:
  nse.rs                     # NSE runtime (feature: nse)
  db_pentest.rs              # Database assessment (feature: db-pentest)
  proxy.rs                   # Interception proxy (feature: web-proxy)
  mobile.rs, mobile_session.rs, mobile_convergence.rs  # Mobile (feature: mobile)
  browser_assess.rs, browser_session.rs, browser_events.rs  # Browser (feature: headless-browser)
  container.rs               # Container security (feature: container)
  wireless.rs                # Wireless scanning (feature: wireless)
  evasion.rs, postex.rs, c2.rs  # Post-exploitation (feature-gated)
  packet_inspection.rs       # Packet capture (feature: packet-inspection)
  stress.rs                  # Stress testing (feature: stress-testing)
  websocket.rs               # WebSocket sessions (feature: websocket)
  git_secrets.rs             # Git secrets (feature: git-secrets)
  sbom.rs                    # SBOM generation (feature: sbom)
  ai_postprocess.rs          # AI integration (feature: ai-integration)
  hunt.rs                    # Advanced hunting (feature: advanced-hunting)
  compliance.rs              # Compliance mapping (feature: compliance)
  daemon.rs                  # Daemon client (feature: daemon-client)
```

### Python package layout

```
python/eggsec/
  __init__.py        # Top-level re-exports, feature guard, deprecation aliases
  __init__.pyi       # Type stubs
  _core.cpython-*.so # Compiled native extension
  _feature_guard.py  # Feature introspection
  py.typed           # PEP 561 marker
  *.pyi              # Per-module type stubs (one per Rust source file)
  net/               # Provisional: network types, transport, probes, HTTP client, WebSocket
  sessions/          # Provisional: browser, mobile, database, proxy session types
  storage/           # Provisional: finding/assessment repositories, artifact stores
  reporting/         # Provisional: reporters, streaming output, baselines
  daemon/            # Provisional: daemon client and parity contracts
  experimental/      # Experimental: wireless, evasion, postex, C2, hunt, AI, stress
```

### Maturity tier table

| Tier | Submodule | Contents |
|------|-----------|----------|
| **stable** | `eggsec` (top-level) | Engine, 22 operations, config, events, scope, core DTOs |
| **provisional** | `eggsec.net` | Network types, transport, probes, HTTP client, WebSocket |
| **provisional** | `eggsec.sessions` | Browser, mobile, database, proxy session types |
| **provisional** | `eggsec.storage` | Finding/assessment repositories, artifact stores |
| **provisional** | `eggsec.reporting` | Reporters, streaming output, baselines |
| **provisional** | `eggsec.daemon` | Daemon client and parity contracts |
| **experimental** | `eggsec.experimental` | Wireless, evasion, postex, C2, hunt, AI, stress |

A Cargo feature only controls compilation. It does not promote a domain to
stable-core. See [`domain-maturity.md`](../docs/python/domain-maturity.md).

## Stable Core Contract

### The 22 stable operations

The `StableOperation` enum in `crates/eggsec-python/src/operation_registry.rs`
defines exactly 22 variants. The exhaustive list, with engine ID mapping and
feature requirements:

| # | Python ID | Engine ID | Python method(s) | Feature | Confirmation |
|---|-----------|-----------|-------------------|---------|--------------|
| 1 | `scan_ports` | `scan-ports` | `scan_ports()` / `async_scan_ports()` | — | no |
| 2 | `scan_endpoints` | `scan-endpoints` | `scan_endpoints()` / `async_scan_endpoints()` | — | no |
| 3 | `fingerprint_services` | `fingerprint` | `fingerprint_services()` / `async_fingerprint_services()` | — | no |
| 4 | `recon_dns` | `recon` | `recon_dns()` / `async_recon_dns()` | — | no |
| 5 | `inspect_tls` | `inspect-tls` | `inspect_tls()` / `async_inspect_tls()` | — | no |
| 6 | `detect_technology` | `detect-technology` | `detect_technology()` / `async_detect_technology()` | — | no |
| 7 | `detect_waf` | `waf-detect` | `detect_waf()` / `async_detect_waf()` | — | no |
| 8 | `validate_waf` | `validate-waf` | `validate_waf()` / `async_validate_waf()` | — | no |
| 9 | `fuzz_http` | `fuzz` | `fuzz_http()` / `async_fuzz_http()` | — | yes |
| 10 | `load_test` | `load-test` | `load_test_http()` / `async_load_test_http()` | — | yes |
| 11 | `scan_git_secrets` | `scan-git-secrets` | `scan_git_secrets()` / `async_scan_git_secrets()` | `git-secrets` | no |
| 12 | `generate_sbom` | `generate-sbom` | `generate_sbom()` / `async_generate_sbom()` | `sbom` | no |
| 13 | `run_consolidated_recon` | `run-consolidated-recon` | `run_consolidated_recon()` / `async_run_consolidated_recon()` | — | no |
| 14 | `graphql_test` | `graphql` | `graphql_test()` / `async_graphql_test()` | — | no |
| 15 | `oauth_test` | `oauth` | `oauth_test()` / `async_oauth_test()` | — | no |
| 16 | `auth_test` | `auth-test` | `auth_test()` / `async_auth_test()` | — | no |
| 17 | `db_probe` | `db-pentest` | `db_probe()` / `async_db_probe()` | `db-pentest` | yes |
| 18 | `nse_run` | `nse` | `nse_run()` / `async_nse_run()` | `nse` | yes |
| 19 | `scan_docker_image` | `scan-docker-image` | `scan_docker_image()` / `async_scan_docker_image()` | `container` | no |
| 20 | `scan_kubernetes` | `scan-kubernetes` | `scan_kubernetes()` / `async_scan_kubernetes()` | `container` | no |
| 21 | `analyze_apk` | `mobile-static` | `analyze_apk()` / `async_analyze_apk()` | `mobile` | no |
| 22 | `analyze_ipa` | `mobile-static` | `analyze_ipa()` / `async_analyze_ipa()` | `mobile` | no |

Source: `StableOperation::ALL` at `operation_registry.rs:402-425`.

**Every operation has both sync and async paths.** This is enforced by the
descriptor contract: `OperationExecutorDescriptor` sets `sync_available: true`
and `async_available: true` for all 22 variants (`operation_registry.rs:296-297`),
and the `sync_and_async_callbacks_both_present` test (`operation_registry.rs:1217-1224`)
asserts both flags are true for every operation.

**Historical aliases** are accepted by `StableOperation::parse()` for backward
compatibility: `fingerprint`, `recon`, `tls_inspect`, `tech_detect`,
`waf_detect`, `waf_validate`, `http_fuzz`, `load_test_http`,
`consolidated_recon` (`operation_registry.rs:535-561`).

## Execution Pipeline

### Engine construction

```
Engine(scope, mode="manual", concurrency=100, timeout_ms=5000)
  → EngineState::from_params(scope, mode, concurrency, timeout_ms)
  → Arc<EngineState> (shared with AsyncEngine)
```

`EngineState` holds the shared `OperationExecutorRegistry`, scope, concurrency,
timeout, event channel, and audit log. Both `Engine` and `AsyncEngine` hold
`Arc<EngineState>`.

### Three-phase dispatch lifecycle

```
Engine::run(request)
  → OperationExecutorRegistry::execute(py, id, request, engine)
    → StableOperation::parse(id)         # reject unknown ops
    → feature_required() check           # reject if feature not compiled
  → Engine::dispatch(py, request, cancel_token)
    Phase 1: pre_dispatch_lifecycle      # planning, validation, preflight, cancel, deadline
    Phase 2: execute_operation           # exhaustive match on StableOperation
    Phase 3: post_dispatch_hooks         # finding/artifact events
  → OperationResult
```

Source: `engine.rs:601-637`.

The async path (`AsyncEngine::dispatch_async`) follows the same three phases
but spawns the operation-specific work onto the shared Tokio runtime via
`runtime_async::spawn_async()`, returning a `PyFuture` that Python can `await`.

### Async runtime bridging (tokio ↔ asyncio)

Two separate process-global Tokio runtimes exist:

- **Async runtime** (`runtime_async.rs`): `OnceLock<Runtime>` with 2 worker
  threads. All `PyFuture` instances share this runtime. Converting results back
  to Python objects acquires the GIL via `Python::attach()` from the worker
  thread (`runtime_async.rs:98`).

- **Sync runtime** (`runtime_sync.rs`): Separate `OnceLock<Runtime>` with 2
  worker threads. `block_on()` uses `py.detach()` to release the GIL during I/O
  (`runtime_sync.rs:28`), preventing deadlock when sync wrappers call async
  engine internals.

### Result envelope

Every operation returns `OperationResult`:

```python
OperationResult {
    status: ExecutionStatus,     # Completed() or Failed { error }
    stats: ExecutionStats?,      # elapsed_ms, items_scanned, items_filtered, errors
    artifacts: list[Artifact],
    error: OperationError?,      # kind, code, operation_id, retryable, denial_class, ...
    metadata: dict[str, str],    # includes "policy_decision" and "policy_schema_version"
    payload: Any?,               # typed result DTO (PortScanResult, etc.)
    payload_type: str?,          # type discriminator for payload
    schema_version: str,         # "1.0"
}
```

Successful results include `policy_decision=allow` in metadata. Failed results
carry a structured `OperationError` with kind, code, and retryability.

## Error Handling Conventions

### Exception hierarchy

Defined in `error.rs:3-13`, registered in `lib.rs:145-161`:

```
PyException
  └── EggsecError
        ├── ConfigError          (validation, configuration)
        ├── ScopeError           (scope denial)
        ├── EnforcementError     (policy denial, capability unavailable, privilege missing)
        ├── NetworkError         (network, daemon transport)
        ├── ScanError            (scan failures)
        ├── TimeoutError         (timeouts)
        ├── FeatureUnavailableError (feature not compiled)
        ├── SerializationError   (serialization, parsing)
        ├── InternalError        (fallback)
        └── CancellationError    (cancellation)
```

### Exception mapping

`operation_error_to_pyerr()` (`error.rs:17-32`) maps `OperationError.kind`
to the appropriate Python exception. `engine_error_to_pyerr()` (`error.rs:37-71`)
maps engine `EggsecError` variants to Python exceptions.

### Enum ValueError convention

All public enums raise `ValueError` on unknown strings. `StableOperation::parse()`
returns `None` for unknown IDs, and the registry provides Levenshtein-distance
suggestions (`operation_registry.rs:783-803`):

```
"Unknown operation: scan_port. Did you mean: scan_ports?"
```

### Context-manager convention

All sink/callback classes and session types implement `__enter__`/`__exit__`
(or `__aenter__`/`__aexit__` for async variants) for automatic cleanup. `Engine`
and `AsyncEngine` also implement context managers (`engine.rs:511-524`,
`async_engine.rs:514-527`).

## Type Stubs & DTOs

### Round-trip contract

All DTO classes implement `to_dict()` and `to_json()` for serialization. The
`from_dict()` and `from_json()` methods support deserialization round-trip.
Type stubs (`.pyi` files) are generated for every Rust source file and included
in the wheel via `pyproject.toml` `include`.

### Key DTO families

| Family | Examples | Stability |
|--------|----------|-----------|
| Scan results | `PortScanResult`, `EndpointScanResult`, `FingerprintScanResult` | stable |
| Recon types | `DnsRecordSet`, `TlsInspectionResult`, `TechDetectionResult` | stable |
| Request types | `PortScanRequest`, `FuzzRequest`, `OperationRequest` | stable |
| Common protocol | `OperationResult`, `OperationError`, `ExecutionStats` | stable |
| Finding schema | `VersionedFinding`, `VersionedEvidence` | provisional |
| Event protocol | `EventEnvelope`, `ProgressEvent`, `FindingEvent` | stable |
| Tool-core | `ToolRequest`, `ToolResponse`, `ToolDescriptor` | stable |

## Feature Gating & Extras

### Cargo features (21 features in `Cargo.toml`)

| Feature | Engine passthrough | System deps | Notes |
|---------|-------------------|-------------|-------|
| `extension-module` | `pyo3/extension-module` | — | Required for maturin builds |
| `websocket` | `eggsec/websocket` | — | WebSocket sessions |
| `git-secrets` | `eggsec/git-secrets` | — | Git secrets scanning |
| `sbom` | `eggsec/sbom` | — | SBOM generation |
| `db-pentest` | `eggsec/db-pentest` | drivers | Database assessment |
| `web-proxy` | `eggsec/web-proxy` | — | MITM proxy |
| `mobile` | `eggsec/mobile` | — | APK/IPA static analysis |
| `mobile-dynamic` | `eggsec/mobile-dynamic` | ADB | Android runtime testing |
| `packet-inspection` | `eggsec/packet-inspection` | libpcap-dev | Packet capture |
| `stress-testing` | `eggsec/stress-testing` | — | Raw socket stress |
| `nse` | `eggsec/nse` | libssl-dev | NSE script execution |
| `container` | `eggsec/container` | — | Docker/K8s scanning |
| `daemon-client` | — | — | Daemon IPC (eggsec-daemon, eggsec-runtime) |
| `headless-browser` | `eggsec/headless-browser` | Chromium | Browser testing |
| `advanced-hunting` | `eggsec/advanced-hunting` | — | Attack chain detection |
| `compliance` | — | — | Compliance mapping (no engine passthrough) |
| `wireless` | `eggsec/wireless` | wireless-tools | WiFi scanning |
| `evasion` | `eggsec/evasion` | — | Evasion detection |
| `postex` | `eggsec/postex` | — | Post-exploitation |
| `c2` | `eggsec/c2` | — | C2 simulation |
| `ai-integration` | `eggsec/ai-integration` | — | LLM integration |
| `full-no-system` | — | — | Aggregate: websocket + git-secrets + sbom + container |

### Python extras (`pyproject.toml [project.optional-dependencies]`)

| Extra | Features |
|-------|----------|
| `db-pentest` | db-pentest |
| `web-proxy` | web-proxy |
| `mobile` | mobile |
| `mobile-dynamic` | mobile + mobile-dynamic |
| `packet-inspection` | packet-inspection |
| `stress-testing` | stress-testing |
| `nse` | nse |
| `wireless` | wireless |
| `headless-browser` | headless-browser |
| `full-no-system` | websocket + git-secrets + sbom + container |

### Feature introspection

`eggsec._feature_guard` and `eggsec.features()` / `eggsec.has_feature()` /
`eggsec.feature_matrix()` provide runtime introspection of compiled features.
The `__init__.py` feature guard registers unavailable symbols with structured
error messages including maturity level, install hint, and platform prerequisites.

## Daemon Client (Provisional)

The optional `daemon-client` feature enables routing through `eggsec-daemon`
over a Unix socket. Both `Engine` and `AsyncEngine` support daemon-backed
construction via `Engine.daemon(socket_path, ...)` / `AsyncEngine.daemon(...)`.

When daemon-backed, `Engine::run()` delegates to `run_via_daemon()` which:
1. Creates a session on first dispatch (if none provided)
2. Converts `OperationRequest` to `TaskKind` JSON via registry-driven
   `operation_request_to_daemon_task()` (uses `OperationExecutorDescriptor.daemon_task_kind`)
3. Submits to daemon and converts response to `OperationResult`

The daemon client remains **provisional**. No release documentation should
describe daemon execution as stable until the daemon parity milestone closes:
request normalization, stable operation identity, policy/audit parity,
structured errors, payload schemas, cancellation, timeouts, reconnect/result
retrieval, event replay, and artifact metadata.

## Validation & Testing

### Rust-side tests (`operation_registry.rs`)

~30 architecture guard and contract tests including:

- `canonical_registry_is_exhaustive` (line 837): asserts `StableOperation::ALL.len() == 22`
- `operation_ids_are_unique_and_ordered` (line 847)
- `legacy_aliases_preserve_dispatch_identity` (line 858)
- `test_all_operations_have_descriptors` (line 879)
- `test_all_operations_have_feature_requirements` (line 888)
- `test_feature_gate_consistency` (line 925): verifies feature-gated ops match expected features
- `sync_and_async_callbacks_both_present` (line 1217): every op has both paths
- `test_confirmation_required_operations` (line 979): NseRun, DbProbe, FuzzHttp, LoadTest
- `feature_gated_executors_agree_with_cargo_features` (line 1203)
- `every_stable_operation_has_exactly_one_executor` (line 1132)
- `aliases_do_not_collide` (line 1161)
- `engine_ids_are_kebab_case` / `python_ids_are_snake_case` (line 1315/1330)
- `generated_metadata_is_current` (line 1237): cross-checks `generated_inventories.rs`

### Python-side testing

```bash
pytest crates/eggsec-python/tests/ crates/eggsec-python/python/tests/
```

- Pytest excludes `network`-marked tests by default (`-m 'not network'`)
- Feature-specific tests gated by `maturin develop --features ...`
- 20 validation profiles in `crates/eggsec-python/validation/profiles.json`
- Skip budget enforcement prevents silent test suite erosion

### Validation scripts

| Script | Purpose |
|--------|---------|
| [`scripts/validate_python_profiles.py`](../scripts/validate_python_profiles.py) | Validates profile manifest |
| [`scripts/run_python_profile.py`](../scripts/run_python_profile.py) | Runs a specific profile end-to-end |
| [`scripts/check_python_compatibility.py`](../scripts/check_python_compatibility.py) | Semantic compatibility checker against baseline |
| [`scripts/check-python.sh`](../scripts/check-python.sh) | Unified CI check (one build, all checks) |
| `make check-python` | Builds into `.venv-ci/`, runs all Python CI checks |

## Release Features

### Release 5 Phase A: Tool Integration

All 22 stable operations have `ToolDescriptor` entries in `ToolRegistry`
generated from `OperationMetadata`. `Engine.invoke_tool()` and
`AsyncEngine.async_invoke_tool()` dispatch through the standard
`EnforcementContext` → `EnforcedDispatcher` path.

`SchemaGenerator` produces JSON Schema for request/response types.

### Release 5 Phase F: Compatibility Enforcement

Automated compatibility enforcement via `check_python_compatibility.py`
against a baseline manifest. Severity levels: `breaking`, `warning`, `info`.
Resource budget enforcement prevents module/export/dependency/size bloat.

### Release 4: Common Session Contract

`SessionState`, `SessionIdentity`, `MobileSession`, `BrowserSession` share
a unified lifecycle contract. Daemon parity protocol with idempotent
submission, reconnect/replay, and cancellation propagation.

### Release 2: Network Programmability

Low-level network primitives (`Target`, `TcpSession`, `UdpSocket`,
`HttpClient`, WebSocket) with enforcement posture. All provisional.

## Invariants & Gotchas

1. **Unknown ops raise `ValueError`** (sync) or return `None` from `parse()`.
   Levenshtein suggestions are provided for close matches.

2. **Feature-gated ops raise `FeatureUnavailableError`** when the Cargo feature
   is not compiled. The Python feature guard registers structured error messages.

3. **Confirmation required** for: `NseRun`, `DbProbe`, `FuzzHttp`, `LoadTest`
   (`operation_registry.rs:92-98`).

4. **Sync `block_on()` releases the GIL** via `py.detach()` (`runtime_sync.rs:28`).
   Async `spawn_async()` acquires the GIL from worker threads via `Python::attach()`
   for result conversion (`runtime_async.rs:98`).

5. **Engine and AsyncEngine share `Arc<EngineState>`** — scope, registry, events,
   and audit log are consistent across sync and async paths.

6. **Historical aliases** (`fingerprint`, `recon`, `tls_inspect`, etc.) resolve
   to the same `StableOperation` variant. They are accepted by `Engine.run()`
   for backward compatibility but new code should use canonical IDs.

7. **Python snake_case ↔ Engine kebab-case**: every operation has different
   naming conventions between Python and Rust. The mapping is in
   `StableOperation::to_engine_id()` (`operation_registry.rs:367-391`).

8. **`EGGSEC_ALLOW_LOOPBACK_FIXTURE=1`** enables release fixture tests using
   loopback addresses. Required for deterministic CI testing.

9. **`api_surface()` stability labels** are the source of truth for individual
   symbol stability. `domain_maturity()` describes whole-domain state.

10. **Checkpoint schema version 3**: checkpoints record operation schema,
    target-set hash, scope hash, execution profile, feature-set hash,
    pipeline-definition hash, and artifact-store identity. Resume rejects
    any mismatch.

## Cross-References

- [overview.md](overview.md) — workspace crate index, system architecture
- [ai_agents.md](ai_agents.md) — AI/LLM integration architecture
- [daemon.md](daemon.md) — daemon persistence, session lifecycle, transport
- [dispatch.md](dispatch.md) — engine dispatch mechanics
- [config.md](config.md) — enforcement model, LoadedScope, policy system
- [docs/python/domain-maturity.md](../docs/python/domain-maturity.md) — domain maturity classifications and graduation checklist

*Last verified against source: 2026-08-25*
