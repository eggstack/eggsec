# Registry & Dispatch Architecture Audit

**Workstream 4 — Registry and Dispatch Architecture Tightening**

> **Status: Phase B Complete** — Release 5 Phase B (2026-07-16) implemented
> the target architecture described in this document. The 22-arm match in
> both `Engine::dispatch()` and `AsyncEngine::dispatch_async()` has been
> replaced by a shared `pre_dispatch_lifecycle()` → `execute_operation()` →
> `post_dispatch_hooks()` flow. `OperationExecutorDescriptor` is the single
> source of truth for per-operation metadata. See `src/dispatch_helpers.rs`,
> `src/operation_executors.rs`, and `src/generated_inventories.rs`.

This document audits the current registry and dispatch architecture in the eggsec
Python bindings, identifies duplication between sync and async engines, and
proposes a target architecture to eliminate redundant match arms.

---

## 1. Current State Analysis

### 1.1 Source File Inventory

| File | Lines | Purpose |
|------|-------|---------|
| `crates/eggsec-python/src/engine.rs` | 3313 | Sync `Engine` with typed methods + generic dispatch |
| `crates/eggsec-python/src/async_engine.rs` | 1948 | Async `AsyncEngine` with typed methods + generic dispatch |
| `crates/eggsec-python/src/operation_registry.rs` | 762 | `StableOperation` enum, `OperationExecutorRegistry` facade |
| `crates/eggsec-python/src/engine_state.rs` | 416 | `EngineState` shared by both engines; pre-dispatch gate |

### 1.2 The 22-Arm Match in `Engine::dispatch()` (engine.rs:487–1741)

The `dispatch()` method at `engine.rs:440` receives an `OperationRequest` and
performs:

1. **Planning event emission** (lines 455–463)
2. **Pre-dispatch validation** (line 466)
3. **Preflight event emission** (lines 471–479)
4. **Deadline computation** (lines 482–485)
5. **`StableOperation::parse()`** (lines 487–490)
6. **22-arm `match operation`** (lines 492–1741)

The 22 arms are:

| # | Variant | Lines | Feature Gate | Has Finding/Artifact Hooks |
|---|---------|-------|--------------|---------------------------|
| 1 | `ScanPorts` | 493–593 | — | Finding: open ports |
| 2 | `ScanEndpoints` | 595–679 | — | Finding: endpoints found |
| 3 | `FingerprintServices` | 680–770 | — | Finding: services identified |
| 4 | `ReconDns` | 771–803 | — | None |
| 5 | `InspectTls` | 804–860 | — | Finding: TLS issues |
| 6 | `DetectTechnology` | 861–893 | — | None |
| 7 | `DetectWaf` | 894–926 | — | None |
| 8 | `LoadTest` | 927–980 | — | None |
| 9 | `ValidateWaf` | 981–1013 | — | None |
| 10 | `FuzzHttp` | 1014–1075 | — | Finding: fuzz issues |
| 11 | `ScanGitSecrets` | 1076–1138 | `git-secrets` | Finding: secrets found |
| 12 | `GenerateSbom` | 1139–1204 | `sbom` | Artifact: SBOM output |
| 13 | `RunConsolidatedRecon` | 1205–1316 | — | Artifact: recon output |
| 14 | `GraphqlTest` | 1317–1371 | — | None |
| 15 | `OauthTest` | 1372–1447 | — | None |
| 16 | `AuthTest` | 1448–1479 | — | None |
| 17 | `DbProbe` | 1480–1529 | `db-pentest` | None |
| 18 | `NseRun` | 1530–1577 | `nse` | None |
| 19 | `ScanDockerImage` | 1578–1615 | `container` | None |
| 20 | `ScanKubernetes` | 1616–1659 | `container` | None |
| 21 | `AnalyzeApk` | 1660–1697 | `mobile` | None |
| 22 | `AnalyzeIpa` | 1698–1735 | `mobile` | None |

### 1.3 The 22-Arm Match in `AsyncEngine::dispatch_async()` (async_engine.rs:486–897)

The `dispatch_async()` method at `async_engine.rs:408` mirrors the sync version:

1. **Planning event emission** (lines 421–431) — uses `Python::with_gil` for PyObjects
2. **Pre-dispatch validation** (line 434)
3. **Preflight event emission** (lines 437–447)
4. **Deadline computation** (lines 450–453)
5. **`check_cancel!` macro** (lines 463–484) — replaces per-arm boilerplate
6. **22-arm `match operation`** (lines 486–897)

Key difference: the async version uses a `check_cancel!` macro (lines 463–484) to
reduce cancellation boilerplate, while the sync version has the full cancellation
check inline in each arm (~13 lines × 22 arms ≈ 286 lines of pure duplication).

### 1.4 The 22 `run_*_inner()` Methods (engine.rs:1743–3312)

Each operation has a dedicated `run_*_inner()` method that:
- Enforces scope (target + port)
- Emits "operation.started" event
- Calls the actual engine function (via `runtime_sync::block_on`)
- Converts the result to a Python DTO
- Emits "operation.completed" or "operation.failed" event
- Wraps in `OperationResult` via `operation_ok()` or `operation_err()`

| Method | Lines | Scope Enforcement | Event Hooks |
|--------|-------|-------------------|-------------|
| `run_port_scan_inner` | 1743–1845 | target + ports | started/completed/failed |
| `run_endpoint_scan_inner` | 1847–1946 | host from URL | started/completed/failed |
| `run_fingerprint_inner` | 1948–2046 | target + ports | started/completed/failed |
| `run_recon_dns_inner` | 2048–2156 | target | started/completed/failed |
| `run_tls_inspect_inner` | 2158–2266 | target | started/completed/failed |
| `run_tech_detect_inner` | 2268–2374 | host from URL | started/completed/failed |
| `run_waf_detect_inner` | 2376–2469 | host from URL | started/completed/failed |
| `run_load_test_inner` | 2471–2571 | host from URL | started/completed/failed |
| `run_waf_validate_inner` | 2573–2657 | host from URL | started/completed/failed |
| `run_fuzz_inner` | 2659–2759 | host from URL | started/completed/failed |
| `run_git_secrets_inner` | 2761–2804 | target (via pre_dispatch) | none |
| `run_sbom_inner` | 2806–2867 | target (via pre_dispatch) | none |
| `run_consolidated_recon_inner` | 2869–2964 | target (via pre_dispatch) | none |
| `run_graphql_inner` | 2966–3012 | target (via pre_dispatch) | none |
| `run_oauth_inner` | 3014–3060 | target (via pre_dispatch) | none |
| `run_auth_test_inner` | 3062–3091 | target (via pre_dispatch) | none |
| `run_db_probe_inner` | 3093–3140 | target (via pre_dispatch) | none |
| `run_nse_inner` | 3142–3178 | target (via pre_dispatch) | none |
| `run_docker_image_inner` | 3180–3209 | target (via pre_dispatch) | none |
| `run_kubernetes_inner` | 3211–3252 | target (via pre_dispatch) | none |
| `run_apk_inner` | 3254–3282 | target (via pre_dispatch) | none |
| `run_ipa_inner` | 3284–3312 | target (via pre_dispatch) | none |

### 1.5 The 22 `run_*_async()` Methods (async_engine.rs:899–1947)

Each async method mirrors its sync counterpart but spawns via
`runtime_async::spawn_async` and returns a `PyFuture`. These methods also
duplicate scope enforcement, "operation.started" event emission, and deadline/
cancellation checks internally.

| Method | Lines |
|--------|-------|
| `run_port_scan_async` | 899–993 |
| `run_endpoint_scan_async` | 995–1072 |
| `run_fingerprint_async` | 1074–1152 |
| `run_recon_dns_async` | 1154–1250 |
| `run_tls_inspect_async` | 1252–1343 |
| `run_tech_detect_async` | 1345–1432 |
| `run_waf_detect_async` | 1434–1513 |
| `run_load_test_async` | 1515–1573 |
| `run_waf_validate_async` | 1575–1606 |
| `run_fuzz_async` | 1608–1661 |
| `run_git_secrets_async` | 1663–1684 |
| `run_sbom_async` | 1686–1722 |
| `run_consolidated_recon_async` | 1724–1794 |
| `run_graphql_async` | 1796–1818 |
| `run_oauth_async` | 1820–1844 |
| `run_auth_test_async` | 1846–1852 |
| `run_db_probe_async` | 1854–1878 |
| `run_nse_async` | 1880–1889 |
| `run_docker_image_async` | 1891–1901 |
| `run_kubernetes_async` | 1903–1925 |
| `run_apk_async` | 1927–1936 |
| `run_ipa_async` | 1938–1947 |

### 1.6 The 10 Typed Methods

Both `Engine` and `AsyncEngine` expose 10 typed `#[pymethods]` for backward
compatibility. These provide a direct path that bypasses the generic `run()`:

| Typed Method | Engine (sync) | AsyncEngine |
|--------------|---------------|-------------|
| `run_port_scan` | engine.rs:189–214 | async_engine.rs:172–183 |
| `run_endpoint_scan` | engine.rs:217–226 | async_engine.rs:186–199 |
| `run_fingerprint` | engine.rs:229–239 | async_engine.rs:202–209 |
| `run_recon_dns` | engine.rs:242–251 | async_engine.rs:212–217 |
| `run_tls_inspect` | engine.rs:254–263 | async_engine.rs:220–225 |
| `run_tech_detect` | engine.rs:266–275 | async_engine.rs:228–233 |
| `run_waf_detect` | engine.rs:278–287 | async_engine.rs:236–241 |
| `run_load_test` | engine.rs:290–299 | async_engine.rs:244–261 |
| `run_waf_validate` | engine.rs:302–311 | async_engine.rs:264–269 |
| `run_fuzz` | engine.rs:314–323 | async_engine.rs:272–290 |

The typed methods call `pre_dispatch_validate` then delegate to the
corresponding `run_*_inner()` (sync) or `run_*_async()` (async). They do NOT
go through the generic dispatch match.

### 1.7 The Registry Facade (operation_registry.rs)

The `OperationExecutorRegistry` is a stateless facade:

- **`execute()`** (lines 289–328): Parses operation ID → checks feature gate →
  delegates to `Engine::dispatch()`
- **`execute_async()`** (lines 330–350): Same flow → delegates to
  `AsyncEngine::dispatch_async()`
- **`list()`** (line 352): Returns all 22 operation IDs
- **`get()`** (line 359): Returns `OperationInfo` for an ID
- **`descriptor_for()`** (lines 379–402): Returns `OperationExecutorDescriptor`
  with risk, feature, confirmation metadata

The `OperationExecutorDescriptor` (lines 12–26) bundles:
- `operation: StableOperation`
- `risk: OperationRisk`
- `feature_required: Option<&'static str>`
- `confirmation_required: bool`
- `confirmation_message: Option<&'static str>`
- `intended_uses: Vec<IntendedUse>`

### 1.8 Helper Function Duplication

Both `engine.rs` and `async_engine.rs` independently define identical helper
functions:

| Function | engine.rs | async_engine.rs |
|----------|-----------|-----------------|
| `extract_host_from_url` | lines 37–44 | lines 22–29 |
| `parse_ports_string` | lines 48–90 | lines 32–69 |
| `operation_ok` | lines 93–112 | lines 72–91 |
| `operation_err` | lines 115–117 | lines 94–96 |
| `operation_err_for` | lines 119–133 | lines 98–112 |

These are copy-pasted with no shared module.

---

## 2. Architecture Diagram

### 2.1 Current Dispatch Flow

```
Python caller
  │
  ├─ Engine.run(request)  ─────────────────────────────────────────────┐
  │    └─ OperationExecutorRegistry::execute()                         │
  │         ├─ StableOperation::parse(id)                              │
  │         ├─ Feature gate check                                      │
  │         └─ Engine::dispatch(py, request, cancel_token)  ─────────┐ │
  │              ├─ Emit: operation.planning                          │ │
  │              ├─ EngineState::pre_dispatch_validate()              │ │
  │              ├─ Emit: operation.preflight                         │ │
  │              ├─ StableOperation::parse(&op)                       │ │
  │              └─ match operation {           ◄── 22 ARM MATCH      │ │
  │                   ScanPorts =>                                    │ │
  │                     ├─ Cancel check                               │ │
  │                     ├─ Deadline check                             │ │
  │                     ├─ Metadata extraction                        │ │
  │                     ├─ Engine::run_port_scan_inner() ───────────┐ │ │
  │                     │    ├─ enforce_target/enforce_port          │ │ │
  │                     │    ├─ Emit: operation.started              │ │ │
  │                     │    ├─ runtime_sync::block_on(             │ │ │
  │                     │    │    eggsec::scanner::scan_ports())    │ │ │
  │                     │    ├─ Convert to PortScanResult            │ │ │
  │                     │    ├─ Emit: operation.completed            │ │ │
  │                     │    └─ operation_ok(stats, payload)        │ │ │
  │                     └─ Emit finding if open ports > 0            │ │ │
  │                   ScanEndpoints =>                                │ │
  │                     ├─ Cancel check                               │ │
  │                     ├─ ... same pattern ...                      │ │
  │                   ... (22 arms total) ...                         │ │
  │                 }                                                 │ │
  └───────────────────────────────────────────────────────────────────┘ │
                                                                       │
  Engine.run_port_scan(request)  (typed, bypasses match) ──────────────┘
       └─ pre_dispatch_validate → run_port_scan_inner()

  AsyncEngine.run(request)  ──────────────────────────────────────────┐
       └─ OperationExecutorRegistry::execute_async()                  │
            └─ AsyncEngine::dispatch_async(request, cancel_token)     │
                 └─ match operation {        ◄── 22 ARM MATCH (dup!) │
                      ScanPorts =>                                    │
                        ├─ check_cancel!()                            │
                        ├─ Metadata extraction                        │
                        └─ run_port_scan_async() ─────────────────┐   │
                             ├─ enforce_target/enforce_port        │   │
                             ├─ Emit: operation.started            │   │
                             ├─ runtime_async::spawn_async(async { │   │
                             │    eggsec::scanner::scan_ports()    │   │
                             │    ... convert result ...            │   │
                             │    Ok(operation_ok(...))            │   │
                             │ })                                   │   │
                             └─ PyFuture                           │   │
                 }                                                  │   │
  ──────────────────────────────────────────────────────────────────┘   │
                                                                       │
  AsyncEngine.run_port_scan(request) (typed, bypasses match) ──────────┘
       └─ pre_dispatch_validate → run_port_scan_async()
```

### 2.2 Shared State

```
EngineState (Arc)
  ├─ scope: Scope
  ├─ mode: String
  ├─ concurrency: usize
  ├─ timeout_ms: u64
  ├─ config: PyEggsecConfig
  ├─ registry: OperationExecutorRegistry  ← stateless facade
  ├─ event_tx: Option<EventSender>
  ├─ enforcement: EnforcementContext
  └─ audit_events: Arc<Mutex<Vec<DispatchAuditEvent>>>

Both Engine and AsyncEngine hold Arc<EngineState>.
```

---

## 3. Current Duplication Assessment

### 3.1 Duplicated Match Arms (Sync vs Async)

All 22 operations are duplicated between `Engine::dispatch()` and
`AsyncEngine::dispatch_async()`. Each arm performs the same logical steps:

1. Cancellation check (sync: inline, async: `check_cancel!` macro)
2. Metadata extraction from `request.metadata` HashMap
3. Delegation to `run_*_inner()` or `run_*_async()`
4. Optional finding/artifact event emission (sync only; async has fewer hooks)

**Scale of duplication:**

| Component | Sync (engine.rs) | Async (async_engine.rs) | Total |
|-----------|------------------|------------------------|-------|
| Match arms | ~1250 lines (492–1741) | ~410 lines (486–897) | ~1660 lines |
| run_*_inner methods | ~1570 lines (1743–3312) | — | ~1570 lines |
| run_*_async methods | — | ~1050 lines (899–1947) | ~1050 lines |
| Helper functions | ~50 lines (×2 files) | ~50 lines | ~100 lines |
| **Total dispatch logic** | **~2870 lines** | **~1510 lines** | **~4380 lines** |

### 3.2 Operations with Identical Dispatch Pattern

The following operations follow a nearly identical pattern in both sync and
async — metadata extraction → inner call → result conversion:

| Operation | Metadata Extracted | Finding Hook | Artifact Hook |
|-----------|-------------------|--------------|---------------|
| `ScanPorts` | `ports` | open_ports > 0 | — |
| `ScanEndpoints` | `endpoints` | endpoints_found > 0 | — |
| `FingerprintServices` | `ports` | services_identified > 0 | — |
| `ReconDns` | — | — | — |
| `InspectTls` | — | tls.issues > 0 | — |
| `DetectTechnology` | — | — | — |
| `DetectWaf` | — | — | — |
| `LoadTest` | `requests`, `concurrency`, `method` | — | — |
| `ValidateWaf` | — | — | — |
| `FuzzHttp` | `payload_type`, `threads` | issues > 0 | — |
| `ScanGitSecrets` | `repo_path`, `max_commits` | findings > 0 | — |
| `GenerateSbom` | `project_path`, `ecosystem`, `format` | — | always |
| `RunConsolidatedRecon` | 12 boolean config fields | — | always |
| `GraphqlTest` | 4 config fields | — | — |
| `OauthTest` | 8+ config fields | — | — |
| `AuthTest` | — | — | — |
| `DbProbe` | `db_type`, `username`, `password`, `database`, `port` | — | — |
| `NseRun` | `scripts`, `script_args` | — | — |
| `ScanDockerImage` | `image` | — | — |
| `ScanKubernetes` | `api_server`, `token`, `timeout_secs` | — | — |
| `AnalyzeApk` | `apk_path` | — | — |
| `AnalyzeIpa` | `ipa_path` | — | — |

### 3.3 Operations with Unique Dispatch Logic

These operations have non-trivial metadata extraction that makes their dispatch
arms longer than average:

1. **`RunConsolidatedRecon`** (engine.rs:1205–1316): 12 boolean config fields
   parsed from metadata, each with `and_then(|s| s.parse()).unwrap_or(true)`.
   ~60 lines of metadata extraction alone.

2. **`OauthTest`** (engine.rs:1372–1447): 8+ config fields including optional
   strings and booleans. ~40 lines of metadata extraction.

3. **`LoadTest`** (engine.rs:927–980): 3 numeric fields parsed from metadata.

4. **`FuzzHttp`** (engine.rs:1014–1075): 2 fields + finding emission logic.

### 3.4 Key Structural Differences Between Sync and Async

| Aspect | Sync (`dispatch`) | Async (`dispatch_async`) |
|--------|-------------------|--------------------------|
| Cancellation check | Inline ~13 lines per arm | `check_cancel!` macro |
| Finding/artifact hooks | Present in 6 operations | Absent (simplified) |
| Python GIL | Held throughout | Released; `Python::with_gil` for events |
| Scope enforcement | In `run_*_inner()` | In `run_*_async()` |
| Error return type | `OperationResult` | `PyResult<PyFuture>` |

---

## 4. Target Architecture Recommendation

### 4.1 The "Registry-Owned Executor Descriptor" Pattern

Each operation should be represented by a **single `OperationExecutor`**
struct that owns all dispatch logic for both sync and async paths:

```rust
/// Complete executor descriptor for a stable operation.
///
/// Replaces the 22-arm match in both sync and async dispatch.
pub struct OperationExecutor {
    /// The stable operation identity.
    pub operation: StableOperation,
    /// Risk classification (from current OperationExecutorDescriptor).
    pub risk: OperationRisk,
    /// Feature flag required (if any).
    pub feature_required: Option<&'static str>,
    /// Confirmation behavior.
    pub confirmation_required: bool,
    pub confirmation_message: Option<&'static str>,
    /// Intended use categories.
    pub intended_uses: Vec<IntendedUse>,

    // --- Dispatch logic ---

    /// Validate and convert the generic OperationRequest into operation-specific
    /// parameters. Returns the extracted parameters or an error.
    pub request_converter: fn(&OperationRequest) -> PyResult<Box<dyn Any>>,

    /// Execute the operation synchronously, blocking with GIL.
    pub sync_executor: fn(
        py: Python<'_>,
        state: &EngineState,
        params: &dyn Any,
        cancel_token: Option<CancellationToken>,
        deadline: Option<Instant>,
    ) -> OperationResult,

    /// Execute the operation asynchronously, returning a PyFuture.
    pub async_executor: fn(
        state: &EngineState,
        params: &dyn Any,
        cancel_token: Option<CancellationToken>,
        deadline: Option<Instant>,
    ) -> PyResult<PyFuture>,

    /// Emit finding events after successful execution (optional).
    pub finding_hook: Option<fn(
        py: Python<'_>,
        state: &EngineState,
        result: &OperationResult,
    )>,

    /// Emit artifact events after successful execution (optional).
    pub artifact_hook: Option<fn(
        py: Python<'_>,
        state: &EngineState,
        result: &OperationResult,
    )>,
}
```

### 4.2 How This Reduces the Match to a Small Top-Level Dispatch

With the descriptor pattern, both `Engine::dispatch()` and
`AsyncEngine::dispatch_async()` collapse to ~30 lines:

```rust
// Engine::dispatch() — simplified
pub(crate) fn dispatch(
    &self,
    py: Python<'_>,
    request: OperationRequest,
    cancel_token: Option<CancellationToken>,
) -> OperationResult {
    let op = request.operation.clone();
    let target = request.target.clone();

    // Planning event
    self.state.emit_event(/* planning event */);

    // Pre-dispatch validation
    if let Err(e) = self.state.pre_dispatch_validate(&op, &target) {
        return operation_err(e.to_string());
    }

    // Preflight event
    self.state.emit_event(/* preflight event */);

    // Deadline
    let deadline = request.timeout_ms
        .or(Some(self.state.timeout_ms))
        .map(|ms| Instant::now() + Duration::from_millis(ms));

    // Registry lookup (O(1) via const array or HashMap)
    let executor = self.state.registry.executor_for(&op)
        .ok_or_else(|| operation_err(format!("Unknown operation: {}", op)))?;

    // Cancel check
    check_cancel!(cancel_token, &op);

    // Convert request → params
    let params = (executor.request_converter)(&request)?;

    // Execute
    let result = (executor.sync_executor)(py, &self.state, &*params, cancel_token, deadline);

    // Hook: findings
    if let Some(hook) = executor.finding_hook {
        hook(py, &self.state, &result);
    }

    // Hook: artifacts
    if let Some(hook) = executor.artifact_hook {
        hook(py, &self.state, &result);
    }

    result
}
```

The async version is identical except it calls `executor.async_executor`
and wraps in `spawn_async`.

### 4.3 Operation Registration Table

All 22 operations register in a single table:

```rust
pub fn build_executor_table() -> Vec<OperationExecutor> {
    vec![
        OperationExecutor {
            operation: StableOperation::ScanPorts,
            risk: OperationRisk::SafeActive,
            feature_required: None,
            confirmation_required: false,
            confirmation_message: None,
            intended_uses: vec![IntendedUse::WebAssessment],
            request_converter: |req| {
                // Extract ports from metadata, parse, validate
                let ports_str = req.metadata.get("ports")
                    .cloned().unwrap_or_else(|| "1-1024".to_string());
                let ports = parse_ports_string(&ports_str)?;
                Ok(Box::new(PortScanParams { target: req.target.clone(), ports }))
            },
            sync_executor: sync_port_scan,
            async_executor: async_port_scan,
            finding_hook: Some(|py, state, result| {
                // Emit finding if open_ports > 0
            }),
            artifact_hook: None,
        },
        // ... 21 more entries
    ]
}
```

### 4.4 Eliminated Code

| Before | After | Savings |
|--------|-------|---------|
| 22-arm match in `Engine::dispatch()` (~1250 lines) | ~30 lines dispatch loop | **~1220 lines** |
| 22-arm match in `AsyncEngine::dispatch_async()` (~410 lines) | ~30 lines dispatch loop | **~380 lines** |
| 22 `run_*_inner()` methods (~1570 lines) | 22 executor functions (move to per-op modules) | **0 lines** (reorganized) |
| 22 `run_*_async()` methods (~1050 lines) | 22 executor functions (move to per-op modules) | **0 lines** (reorganized) |
| Duplicate helpers (~100 lines across 2 files) | Shared `utils.rs` module | **~80 lines** |
| **Total net reduction** | | **~1680 lines** |

---

## 5. Refactoring Roadmap

### Phase 1: Extract Common Dispatch Pattern (No Behavior Change)

**Goal**: Eliminate helper duplication and introduce the `check_cancel!`
macro in the sync engine.

**Changes**:
1. Create `crates/eggsec-python/src/utils.rs` with shared functions:
   - `extract_host_from_url()`
   - `parse_ports_string()`
   - `operation_ok()`
   - `operation_err()`
   - `operation_err_for()`
2. Add `check_cancel!` macro in `engine.rs` (matching async_engine.rs:463–484)
3. Replace inline cancellation blocks in all 22 arms of `Engine::dispatch()`
   with `check_cancel!()` calls

**Files modified**: `engine.rs`, `async_engine.rs`, `lib.rs` (new module)

**Risk**: Low — mechanical extraction of identical code.

### Phase 2: Create OperationExecutor Trait/Struct

**Goal**: Define the `OperationExecutor` struct and registry integration.

**Changes**:
1. Create `crates/eggsec-python/src/operation_executor.rs` with:
   - `OperationExecutor` struct
   - `OperationExecutorTable` (Vec or HashMap keyed by `StableOperation`)
   - `build_executor_table()` function
2. Extend `OperationExecutorRegistry` to hold the table
3. Add `executor_for(&self, op: &StableOperation) -> Option<&OperationExecutor>`

**Files modified**: `operation_executor.rs` (new), `operation_registry.rs`

**Risk**: Medium — new abstraction layer; no behavior change yet.

### Phase 3: Migrate Operations One at a Time

**Goal**: Move each operation's dispatch logic into an executor function.

**Migration order** (simplest first):
1. `ReconDns` — simplest metadata extraction, no finding/artifact hooks
2. `DetectTechnology` — same pattern
3. `DetectWaf` — same pattern
4. `InspectTls` — finding hook for TLS issues
5. `ValidateWaf` — simple
6. `AuthTest` — no metadata extraction
7. `ScanDockerImage` — simple metadata
8. `ScanKubernetes` — 3 metadata fields
9. `AnalyzeApk` — simple metadata
10. `AnalyzeIpa` — simple metadata
11. `DbProbe` — 5 metadata fields
12. `NseRun` — 2 metadata fields
13. `ScanPorts` — ports parsing + finding hook
14. `ScanEndpoints` — endpoints parsing + finding hook
15. `FingerprintServices` — ports + finding hook
16. `LoadTest` — 3 numeric fields
17. `FuzzHttp` — 2 fields + finding hook
18. `ScanGitSecrets` — 2 fields + finding hook
19. `GenerateSbom` — 3 fields + artifact hook
20. `RunConsolidatedRecon` — 12 fields + artifact hook
21. `GraphqlTest` — 4 fields
22. `OauthTest` — 8+ fields

**For each migration**:
1. Create `executor_fn_sync()` and `executor_fn_async()` functions
2. Create `request_converter()` function
3. Register in `build_executor_table()`
4. Remove the arm from both match statements
5. Run full test suite

**Files modified**: `engine.rs`, `async_engine.rs`, per-operation modules

**Risk**: Medium — must preserve exact behavior for each operation.

### Phase 4: Remove Duplicated Match Arms

**Goal**: Replace both 22-arm matches with the generic dispatch loop.

**Changes**:
1. Replace `Engine::dispatch()` match with generic loop over executor table
2. Replace `AsyncEngine::dispatch_async()` match with generic loop
3. Remove the 22 inline typed methods from `Engine` and `AsyncEngine`
   (or keep as thin wrappers that delegate to executor table)
4. Verify all 22 operations produce identical `OperationResult` for same inputs

**Files modified**: `engine.rs`, `async_engine.rs`

**Risk**: High — behavior regression risk; requires comprehensive testing.

---

## 6. Risk Assessment

### 6.1 What Could Break During Refactoring

| Risk | Severity | Mitigation |
|------|----------|------------|
| Operation ordering changes | Medium | Executor table preserves `StableOperation::ALL` ordering |
| Feature-gated operations missing | High | `#[cfg(feature = "...")]` must wrap individual executors, not just table entries |
| Finding/artifact hooks lost | Medium | Explicit `finding_hook`/`artifact_hook` fields; test with event subscription |
| Metadata extraction diverges | High | Each request_converter must be tested against typed method behavior |
| Cancel/deadline semantics change | High | Cancel check must remain at dispatch level, not inside executor |
| Public API breakage | Critical | Typed methods (`run_port_scan` etc.) must remain; wrapper approach preserves them |
| GIL handling in async | High | Async executors must not hold GIL during I/O; use `Python::with_gil` only for events |

### 6.2 How to Preserve Public API Compatibility

The 10 typed methods (`run_port_scan`, `run_endpoint_scan`, etc.) are part of
the Python public API. They must remain as `#[pymethods]` on both `Engine` and
`AsyncEngine`. The refactoring should:

1. Keep typed methods as thin wrappers
2. Have them call `(executor.request_converter)()` then `(executor.sync_executor)()`
3. Or simply keep them as-is, since they bypass the generic dispatch entirely

The generic `run()` method (which goes through the registry) is the primary
refactoring target. The typed methods can remain unchanged.

### 6.3 How to Preserve Operation Ordering

`StableOperation::ALL` defines the canonical ordering (used for
`list_operations()`). The executor table must preserve this order:

```rust
pub fn build_executor_table() -> Vec<OperationExecutor> {
    // Must match StableOperation::ALL ordering
    StableOperation::ALL.iter().map(|&op| match op {
        StableOperation::ScanPorts => port_scan_executor(),
        StableOperation::ScanEndpoints => endpoint_scan_executor(),
        // ...
    }).collect()
}
```

Alternatively, use a `HashMap<StableOperation, OperationExecutor>` and have
`list()` iterate `StableOperation::ALL` for ordering.

### 6.4 How to Preserve Policy and Audit Sequencing

The current dispatch flow is:

1. Planning event → 2. Pre-dispatch validation → 3. Preflight event →
4. Cancel check → 5. Deadline check → 6. Execute → 7. Finding/artifact hooks

This sequence must remain identical. The refactored dispatch loop should:

```rust
// 1. Planning event
emit_planning_event(state, &op, &target);

// 2. Pre-dispatch validation (scope + feature gate + audit)
state.pre_dispatch_validate(&op, &target)?;

// 3. Preflight event
emit_preflight_event(state, &op, &target);

// 4. Cancel check
check_cancel!(cancel_token, &op);

// 5. Deadline check
check_deadline(deadline)?;

// 6. Execute via executor
let result = (executor.sync_executor)(py, state, &*params, cancel_token, deadline);

// 7. Finding/artifact hooks
emit_hooks(executor, py, state, &result);
```

---

## 7. Summary Metrics

| Metric | Current | After Refactoring |
|--------|---------|-------------------|
| Match arms in sync dispatch | 22 | 0 (generic loop) |
| Match arms in async dispatch | 22 | 0 (generic loop) |
| Lines in sync dispatch match | ~1250 | ~30 |
| Lines in async dispatch match | ~410 | ~30 |
| Duplicate helper functions | 5 (×2 files) | 1 (shared module) |
| Cancellation boilerplate (sync) | ~286 lines (22 × 13) | 0 (macro) |
| Total dispatch-related LOC | ~4380 | ~2700 (est.) |
| New abstraction (OperationExecutor) | 0 | ~120 lines |
| **Net reduction** | — | **~1560 lines** |

---

*Generated by Workstream 4 audit. This document describes the current architecture
and a recommended refactoring target. No code was modified.*
