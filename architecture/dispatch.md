# Dispatch Layer

Deep dive: how requests become engine executions once policy has approved them.

Parent overview: [overview.md](overview.md). Related: [runtime_bridge.md](runtime_bridge.md), [cli_commands.md](cli_commands.md), [config.md](config.md).

## Role

`crates/eggsec/src/dispatch/` is the engine's **frontend-neutral execution layer**. It converts a `TaskKind` request into a call to the right engine module and returns a typed `TaskResult`. It performs **no authorization of its own** — every executor receives an already-evaluated `OperationDescriptor`, and all scope/policy decisions were made upstream by `EnforcementContext::evaluate()` (see [config.md](config.md)).

```
CLI handler ──────────────────────────────────────┐
TUI (via TuiTaskDispatcher) ──────────────────────┤
REST/MCP/gRPC/Agent (EnforcedDispatcher::dispatch_checked) ──┤→ dispatch_inner() → worker → engine → TaskResult
Daemon/Runtime (ApprovedRunRequest bundle) ───────┘
```

The two public entry points are:

- `dispatch_task()` (`mod.rs:63`) — creates per-task progress + result channels, calls `dispatch_inner()`, and forwards the result. Returns `(progress_rx, result_rx)` for the caller to consume.
- `dispatch_inner()` (`mod.rs:101–388`) — the `TaskKind` router. Returns `TaskResult` directly. Marked `#[doc(hidden)]` but `pub` because `eggsec-tui`'s dispatcher and `runtime_bridge` are sanctioned callers.

## File Layout

| File | Lines | Contents |
|------|-------|----------|
| `mod.rs` | 519 | `dispatch_task()` (`:63`) — channel creation + forwarding; `dispatch_inner()` (`:101–388`) — `TaskKind` router (29 match arms covering all task kinds, feature-gated kinds fall through to explicit `TaskResult::Error` when compiled out); unit tests |
| `types.rs` | 156 | `TaskResult` enum (27 typed variants + `Error`), `GraphQlResults`, `OAuthResults`, `NseResults`, `TracerouteHopResult`, `ReconOptions`, `send_progress()` helper |
| `executor.rs` | 64 | `OperationExecutor` trait (object-safe: no generic self, no generic associated types), `ExecutionOutput` enum (`Success`/`FeatureUnavailable`/`Failed`) |
| `executors/mod.rs` | 43 | `build_default_registry()` — registers 5 always-compiled + 2 feature-gated adapters |
| `executors/registry.rs` | 140 | `ExecutorRegistry` — maps operation IDs to `Box<dyn OperationExecutor>`, panics on duplicate registration |
| `executors/scanner.rs` | 83 | `ScannerExecutor` — `scan-ports`, `scan-endpoints`, `fingerprint` |
| `executors/recon.rs` | 61 | `ReconExecutor` — `recon`, `pipeline` |
| `executors/waf.rs` | 57 | `WafExecutor` — `waf-detect`, `waf-bypass`, `waf-stress` |
| `executors/fuzz.rs` | 97 | `FuzzExecutor` — `fuzz`, `graphql`, `oauth` |
| `executors/network.rs` | 90 | `NetworkExecutor` — `load-test`, `stress-test`, `packet`, `auth-test` |
| `executors/nse.rs` | 50 | `NseExecutor` — `nse` (`#[cfg(feature = "nse")]`) |
| `executors/db_pentest.rs` | 54 | `DbPentestExecutor` — `db-pentest` (`#[cfg(feature = "db-pentest")]`) |

### Domain Worker Files

These are the actual task implementations invoked by `dispatch_inner()`:

| Worker | Feature Gate | Module Path |
|--------|-------------|-------------|
| `scanner.rs` | always | `dispatch::scanner` |
| `recon.rs` | always | `dispatch::recon` |
| `fuzzer.rs` | always | `dispatch::fuzzer` |
| `network.rs` | always | `dispatch::network` |
| `auth.rs` | always | `dispatch::auth` |
| `api.rs` | always | `dispatch::api` (includes `run_nse` behind `#[cfg(feature = "nse")]`) |
| `security.rs` | `advanced-hunting`, `compliance`, `database`, `external-integrations`, `finding-workflow`, `vuln-management`, `headless-browser`, `wireless` | `dispatch::security` |
| `c2.rs` | `c2` | `dispatch::c2` |
| `db_pentest.rs` | `db-pentest` | `dispatch::db_pentest` |
| `intercept.rs` | `web-proxy` | `dispatch::intercept` |

## TaskKind Routing

`dispatch_inner()` (`mod.rs:101–388`) matches all 29 `TaskKind` variants (defined at `eggsec-runtime/src/request.rs:53–83`). Each arm extracts parameters from the variant's payload struct and delegates to the corresponding domain worker:

| # | TaskKind | Worker Call | Feature Gate |
|---|----------|-------------|-------------|
| 1 | `LoadTest` | `network::run_load_test` | always |
| 2 | `StressTest` | `network::run_stress_test` | always |
| 3 | `PortScan` | `scanner::run_port_scan` | always |
| 4 | `EndpointScan` | `scanner::run_endpoint_scan` | always |
| 5 | `Fingerprint` | `scanner::run_fingerprint` | always |
| 6 | `Fuzz` | `fuzzer::run_fuzz` | always |
| 7 | `Waf` | `fuzzer::run_waf` | always |
| 8 | `WafStress` | `fuzzer::run_waf_stress` | always |
| 9 | `Pipeline` | `recon::run_pipeline` | always |
| 10 | `Recon` | `recon::run_recon` | always |
| 11 | `PacketCapture` | `network::run_packet_capture` | always |
| 12 | `PacketTraceroute` | `network::run_packet_traceroute` | always |
| 13 | `PacketSend` | `network::run_packet_send` | always |
| 14 | `GraphQl` | `api::run_graphql` | always |
| 15 | `OAuth` | `api::run_oauth` | always |
| 16 | `AuthTest` | `auth::run_auth_task` | always |
| 17 | `Nse` | `api::run_nse` | `nse` |
| 18 | `Hunt` | `security::run_hunt_task` | `advanced-hunting` |
| 19 | `Browser` | `security::run_browser_task` | `headless-browser` |
| 20 | `Compliance` | `security::run_compliance_task` | `compliance` |
| 21 | `Storage` | `security::run_storage_task` | `database` |
| 22 | `Integrations` | `security::run_integrations_task` | `external-integrations` |
| 23 | `Workflow` | `security::run_workflow_task` | `finding-workflow` |
| 24 | `Vuln` | `security::run_vuln_task` | `vuln-management` |
| 25 | `Wireless` | `security::run_wireless_task` | `wireless` |
| 26 | `WirelessActive` | `security::run_wireless_active_task` | `wireless-advanced` |
| 27 | `DbPentest` | `db_pentest::run_db_pentest_task` | `db-pentest` |
| 28 | `Intercept` | `intercept::run_intercept_task` | `web-proxy` |
| 29 | `C2` | `c2::run_c2_task` | `c2` |

The final arm (`_ =>`) at `mod.rs:383` catches feature-gated variants compiled out and returns `TaskResult::Error("Unsupported task kind")` with a `tracing::warn!`. This is **never a silent no-op** — the caller always receives an explicit error.

## Executor Adapters

`executors/mod.rs:25` (`build_default_registry()`) registers adapters used by the tool/registry path:

| Executor | Feature | Operation IDs | Delegates To |
|----------|---------|---------------|-------------|
| `ScannerExecutor` | always | `scan-ports`, `scan-endpoints`, `fingerprint` | `dispatch::scanner` |
| `ReconExecutor` | always | `recon`, `pipeline` | `dispatch::recon` |
| `WafExecutor` | always | `waf-detect`, `waf-bypass`, `waf-stress` | `dispatch::fuzzer` |
| `FuzzExecutor` | always | `fuzz`, `graphql`, `oauth` | `dispatch::fuzzer`, `dispatch::api` |
| `NetworkExecutor` | always | `load-test`, `stress-test`, `packet`, `auth-test` | `dispatch::network`, `dispatch::auth` |
| `NseExecutor` | `nse` | `nse` | `dispatch::api` |
| `DbPentestExecutor` | `db-pentest` | `db-pentest` | `dispatch::db_pentest` |

All adapters implement the `OperationExecutor` trait (`executor.rs:26`), which is **object-safe**: no generic self parameters, no associated types with generic bounds. Executors are stored as `Box<dyn OperationExecutor>` in the `ExecutorRegistry`.

The `OperationExecutor` trait provides:

- `operation_ids()` → canonical operation IDs this executor handles
- `metadata()` → `OperationMetadata` references (currently returns `&[]` for all adapters)
- `execute_async()` → primary async execution path
- `execute_sync()` → optional blocking path (default: returns `Failed`)
- `can_handle()` → checks membership in `operation_ids()`

The `ExecutorRegistry` (`executors/registry.rs:9`) uses `std::collections::HashMap` for operation→executor mapping. It panics on duplicate operation ID registration (`registry.rs:33`).

## Interaction With Enforcement

Three distinct paths reach engine functions, all post-authorization:

### 1. Manual Surfaces (CLI/TUI)

CLI/TUI handlers in `crates/eggsec/src/commands/handlers/` call `ctx.evaluate_and_enforce_operation(descriptor)` themselves, then invoke engine functions directly (not via `dispatch_inner`). See [cli_commands.md](cli_commands.md).

```
CLI/TUI handler
    → ctx.evaluate_and_enforce_operation(descriptor)
    → ApprovedOperation
    → engine function directly
```

### 2. Strict Protocol Surfaces (REST/MCP/gRPC/Agent)

Route through `EnforcedDispatcher::dispatch_checked()` in `tool/dispatcher.rs:258`, which calls `validate_request_binding()` to verify the `ApprovedOperation` token binding (tool ↔ canonical operation, target agreement). Fails closed before any executor runs.

```
REST/MCP/gRPC/Agent handler
    → EnforcementContext::evaluate()
    → ApprovedOperation
    → EnforcedDispatcher::dispatch_checked(approved, request)
        → validate_request_binding() [tool/dispatcher.rs:263]
        → ToolDispatcher::dispatch(request)
```

Callers include:
- `tool/protocol/rest.rs:821`
- `tool/protocol/grpc.rs:711,861`
- `tool/protocol/mcp/handlers/server.rs:650`

### 3. Daemon/Runtime Surfaces

`runtime_bridge::approve_run_request_bundle()` converts `RunRequest` → descriptor, obtains an `ApprovedOperation`, and the resulting `ApprovedRunRequest` executes through this dispatch layer via `dispatch_approved_runtime_request()`. See [runtime_bridge.md](runtime_bridge.md).

```
Daemon/Runtime
    → approve_run_request_bundle()
    → ApprovedRunRequest (token + request coupled)
    → dispatch_approved_runtime_request()
        → re-resolve descriptor, validate operation + target match
        → dispatch_inner(request, progress_tx)
```

## TaskResult Variants

`TaskResult` (`types.rs:80`) is a typed enum with 27 data variants + `Error`. Feature-gated variants compile out with their features:

| Variant | Feature | Source Type |
|---------|---------|------------|
| `LoadTest` | always | `loadtest::metrics::LoadTestResults` |
| `PortScan` | always | `scanner::PortScanResults` |
| `EndpointScan` | always | `scanner::EndpointScanResults` |
| `Fingerprint` | always | `scanner::FingerprintResults` |
| `WafDetection` | always | `waf::WafDetectionResult` |
| `WafBypass` | always | `waf::WafDetectionResult` + `Vec<waf::BypassResult>` |
| `WafStress` | always | `Vec<waf::BypassResult>` |
| `Pipeline` | always | `pipeline::PipelineReport` |
| `Fuzz` | always | `fuzzer::engine::FuzzSession` |
| `Recon` | always | `recon::FullReconResult` |
| `PacketCapture` | always | inline struct |
| `PacketTraceroute` | always | `Vec<TracerouteHopResult>` |
| `PacketSend` | always | inline struct |
| `GraphQl` | always | `GraphQlResults` |
| `OAuth` | always | `OAuthResults` |
| `Auth` | always | `auth::AuthTestReport` |
| `StressTest` | `stress-testing` | inline struct |
| `Nse` | `nse` | `NseResults` |
| `Hunt` | `advanced-hunting` | `hunt::HuntReport` |
| `Browser` | `headless-browser` | `browser::BrowserReport` |
| `Compliance` | `compliance` | `compliance::ComplianceReport` |
| `Storage` (+ `ListScans`, `ListFindings`) | `database` | — |
| `Integrations` (+ variants) | `external-integrations` | — |
| `Workflow` | `finding-workflow` | `workflow::WorkflowReport` |
| `Vuln` | `vuln-management` | `vuln::VulnAssessment` |
| `Wireless` | `wireless` | `wireless::WirelessScanResult` |
| `WirelessActive` | `wireless-advanced` | `wireless::active::ActiveWirelessAttackResult` |
| `DbPentest` | `db-pentest` | `db_pentest::DbPentestReport` |
| `Intercept` | `web-proxy` | `proxy::intercept::types::InterceptSession` |
| `C2` | `c2` | `c2::C2Report` |
| `Error` | always | `String` |

## Invariants

1. **Executors are policy-free**: no `LoadedScope`/`EnforcementContext` access below this layer. The `dispatch_inner()` function (`mod.rs:94–98`) documents this explicitly.
2. **Every spawned task path carries timeout wrappers** per workspace convention (AGENTS.md lesson).
3. **Unsupported feature-gated task kinds fail with explicit `TaskResult::Error`**, never silently no-op (`mod.rs:383–386`).
4. **The TUI must not host its own dispatch code** — tab UIs call into this module (architecture guard enforced).
5. **`dispatch_inner` must only be invoked from manual surfaces** (CLI/TUI `ManualPermissive` context). Strict surfaces must never call it directly — route through `EnforcementContext::evaluate()` and `EnforcedDispatcher::dispatch_checked()` (`mod.rs:94–99`).
6. **`dispatch_task` wraps errors in `TaskResult::Error`** — callers always receive a result through the channel, never a dropped request (`mod.rs:77–81`).

## Bug Sweep

| Severity | File:Line | Issue |
|----------|-----------|-------|
| LOW | `executors/registry.rs:12` | Uses `std::collections::HashMap` instead of `FxHashMap` for operation→executor mapping. Not performance-critical (built once, queried per-dispatch) but inconsistent with workspace convention (`AGENTS.md` key patterns). |
| LOW | `runtime_bridge/executor.rs:344` | Spawned `progress_forwarder` tokio task has no explicit timeout wrapper. However, it is bounded by the `dispatch_approved_runtime_request` lifetime and `CancellationToken` — the task will be aborted via `progress_forwarder.abort()` on cancellation (`:357`). Acceptable but not explicit. |

## Testing

The dispatch module has three test groups in `mod.rs:390–518`:

- **`dispatch_task_port_scan_returns_receivers`** (`:396`) — verifies channel plumbing works for a port scan request.
- **`dispatch_inner_returns_task_result_for_error_case`** (`:419`) — proves `dispatch_inner` returns `TaskResult` (not `()`) using an unreachable target.
- **`executor_registry_covers_core_operations`** (`:454`) — verifies all 15 core operation IDs have registered executors.
- **`executor_registry_feature_gated_operations`** (`:486`) — verifies `nse` and `db-pentest` when features are enabled.
- **`executor_registry_no_duplicates`** (`:504`) — ensures no duplicate operation IDs across executors.

The `runtime_bridge/bundle.rs` tests (`:118–313`) verify dispatch-level anti-tamper checks (operation mismatch, target mismatch).

## See Also

- [runtime_bridge.md](runtime_bridge.md) — Daemon/runtime entry point into dispatch
- [cli_commands.md](cli_commands.md) — CLI/TUI manual entry points
- [config.md](config.md) — `EnforcementContext`, `ExecutionPolicy`, `OperationMetadata`
- [tool/dispatcher.rs](../crates/eggsec/src/tool/dispatcher.rs) — `EnforcedDispatcher::dispatch_checked()` for strict surfaces
- [overview.md](overview.md) — System-wide architecture, enforcement model

*Last verified against source: 2026-08-25*
