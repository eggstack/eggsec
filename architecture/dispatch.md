# Dispatch Layer

Deep dive: how requests become engine executions once policy has approved them.

Parent overview: [overview.md](overview.md). Related: [runtime_bridge.md](runtime_bridge.md), [cli_commands.md](cli_commands.md), [config.md](config.md).

> **Corrections (2026-10-06, verified against source).** `TaskKind` 29→**30**
> variants (`Resume` added; routing table extended with row 10). `TaskResult`
> 34→**35** total variants (34 data + `Error`). `CanonicalOperationRequest`
> 29→**30** variants. Fixed the file-layout line counts
> (`canonical_execution.rs` 2265→2387, `mod.rs` 311→315, `executors/scanner.rs`
> 84→95, `executors/recon.rs` 61→66), re-pointed `execute_approved_execution()`
> 755→772 and the `runtime_contract_closure` test count 16→17, and replaced the
> "34 vs 29 TaskKind gap" note with the accurate current mapping. All
> enforcement invariants below were re-checked against `tool/dispatcher.rs` and
> `runtime_bridge/bundle.rs` and are unchanged.

## Role

`crates/eggsec/src/dispatch/` is the engine's **frontend-neutral execution layer**.
Phase 1 convergence: the single production owner for
`(canonical operation ID + canonical typed request + ApprovedOperation) → engine
executor` is `canonical_execution.rs` (`execute_approved_execution` /
`execute_approved` / `execute_canonical`).
It performs **no authorization of its own beyond binding verification** — scope/policy
decisions were made upstream by `EnforcementContext::evaluate()` (see [config.md](config.md));
the boundary re-verifies request/approval binding at executor entry and checks
feature availability in one layer.

```
CLI adapter (route + canonical request + approval)
TUI shallow adapter (TaskKind → canonical request) ─┐
Daemon bundle (approval + canonical request) ───────┤→ execute_approved() → execute_canonical() → worker → TaskResult
REST/MCP/gRPC/Agent (EnforcedDispatcher, canonical validation) ─┘
```

The public entry points are:

- `execute_approved_execution()` (`canonical_execution.rs:772`) — the scope-bearing canonical boundary. Takes an `ApprovedExecution` bundle (token + scope snapshot from `approve_execution()`/`approve_manual_execution()`); strict surfaces (daemon runtime bridge) dispatch only through this path.
- `execute_approved()` (`canonical_execution.rs`) — the canonical boundary. Requires
  exact operation/target/descriptor binding (`matches_descriptor`), validates through
  canonical contracts, checks features, emits frontend-neutral `ExecutionEvent`s.
- `execute_canonical()` (`canonical_execution.rs`) — the single-owner executor match
  over `CanonicalOperationRequest`. `dispatch_inner()` and the TUI dispatcher both
  delegate here; neither owns a second mapping.
- `dispatch_task()` (`mod.rs`) — creates per-task progress + result channels, calls
  `dispatch_inner()`, and forwards the result. Returns `(progress_rx, result_rx)`.
- `dispatch_inner()` (`mod.rs`) — legacy manual shim over `execute_canonical`
  (`TaskKind → CanonicalOperationRequest`, `FeatureUnavailable → Ok(Error)` for
  backward compat). New code uses `execute_approved` or the runtime-bridge bundle.

## File Layout

| File | Lines | Contents |
|------|-------|----------|
| `canonical_execution.rs` | 2387 | `execute_approved_execution()` — scope-bearing bundle entry; `execute_approved()` — binding/feature checks + single-owner routing; `execute_canonical()` — the executor match; `CanonicalOperationRequest` (30-variant typed enum), `ExecutionEvent`/`ExecutionSink` (bounded, coalescing progress, never-drop findings/terminal), `executor_route_for()`, `is_feature_available()`; unit tests |
| `mod.rs` | 315 | `dispatch_task()` — channel creation + forwarding; `dispatch_inner()` — legacy manual shim delegating to `execute_canonical`; unit tests |
| `types.rs` | 156 | `TaskResult` enum (34 typed variants + `Error`), `GraphQlResults`, `OAuthResults`, `NseResults`, `TracerouteHopResult`, `ReconOptions`, `send_progress()` helper |
| `executor.rs` | 64 | `OperationExecutor` trait (object-safe: no generic self, no generic associated types), `ExecutionOutput` enum (`Success`/`FeatureUnavailable`/`Failed`) |
| `executors/mod.rs` | 43 | `build_default_registry()` — registers 5 always-compiled + 2 feature-gated adapters |
| `executors/registry.rs` | 140 | `ExecutorRegistry` — maps operation IDs to `Box<dyn OperationExecutor>`, panics on duplicate registration |
| `executors/scanner.rs` | 95 | `ScannerExecutor` — `scan-ports`, `scan-endpoints`, `fingerprint` |
| `executors/recon.rs` | 66 | `ReconExecutor` — `recon`, `pipeline` |
| `executors/waf.rs` | 57 | `WafExecutor` — `waf-detect`, `waf-bypass`, `waf-stress` |
| `executors/fuzz.rs` | 97 | `FuzzExecutor` — `fuzz`, `graphql`, `oauth` |
| `executors/network.rs` | 91 | `NetworkExecutor` — `load-test`, `stress-test`, `packet`, `auth-test` |
| `executors/nse.rs` | 50 | `NseExecutor` — `nse` (`#[cfg(feature = "nse")]`) |
| `executors/db_pentest.rs` | 55 | `DbPentestExecutor` — `db-pentest` (`#[cfg(feature = "db-pentest")]`) |

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

## Canonical Routing (single owner)

`execute_canonical()` (`canonical_execution.rs`) matches `CanonicalOperationRequest`
(converted exhaustively from `TaskKind` via `from_task_kind`, or from CLI adapters).
Each arm normalizes through canonical contracts, then delegates to the corresponding
domain worker. `dispatch_inner()` no longer owns a `TaskKind` match; it converts and
delegates. The `TaskKind` variants (30 total, defined at `eggsec-runtime/src/request.rs:64`)
route as follows:

> **34 metadata vs 30 TaskKind gap.** The operation catalog holds 34 canonical
> operations but `TaskKind` has 30 variants, and `CanonicalOperationRequest` has
> 30. The mapping is deliberately many-to-one, so the variant counts do not have
> to match: `TaskKind::operation_id()` (`crates/eggsec-runtime/src/request.rs`)
> collapses `PacketCapture`/`PacketTraceroute`/`PacketSend` → `packet`,
> `Wireless`/`WirelessActive` → `wireless`, and `Resume` → `pipeline`. So the 30
> `TaskKind` variants cover only 26 distinct catalog IDs. The remaining **8**
> catalog operations — `waf-bypass`, `wireless-deauth`, `mobile-static`,
> `mobile-dynamic`, `remote`, `search`, `evasion`, `postex` — have no
> `TaskKind`/`CanonicalOperationRequest` arm and are reachable only via
> CLI/manual and protocol adapters (e.g. `--waf-bypass`, MCP tool call).
> Growth in either registry without the other is drift — check both when adding
> operations.

| # | TaskKind | Worker Call | Feature Gate |
|---|----------|-------------|-------------|
| 1 | `LoadTest` | `network::run_load_test_with_scope` | always |
| 2 | `StressTest` | `network::run_stress_test` | always |
| 3 | `PortScan` | `scanner::run_port_scan` | always |
| 4 | `EndpointScan` | `scanner::run_endpoint_scan` | always |
| 5 | `Fingerprint` | `scanner::run_fingerprint` | always |
| 6 | `Fuzz` | `fuzzer::run_fuzz` | always |
| 7 | `Waf` | `fuzzer::run_waf` | always |
| 8 | `WafStress` | `fuzzer::run_waf_stress` | always |
| 9 | `Pipeline` | `recon::run_pipeline` | always |
| 10 | `Resume` | `recon::run_pipeline_resume` | always |
| 11 | `Recon` | `recon::run_recon` | always |
| 12 | `PacketCapture` | `network::run_packet_capture` | always |
| 13 | `PacketTraceroute` | `network::run_packet_traceroute` | always |
| 14 | `PacketSend` | `network::run_packet_send` | always |
| 15 | `GraphQl` | `api::run_graphql` | always |
| 16 | `OAuth` | `api::run_oauth` | always |
| 17 | `AuthTest` | `auth::run_auth_task` | always |
| 18 | `Nse` | `api::run_nse` | `nse` |
| 19 | `Hunt` | `security::run_hunt_task` | `advanced-hunting` |
| 20 | `Browser` | `security::run_browser_task` | `headless-browser` |
| 21 | `Compliance` | `security::run_compliance_task` | `compliance` |
| 22 | `Storage` | `security::run_storage_task` | `database` |
| 23 | `Integrations` | `security::run_integrations_task` | `external-integrations` |
| 24 | `Workflow` | `security::run_workflow_task` | `finding-workflow` |
| 25 | `Vuln` | `security::run_vuln_task` | `vuln-management` |
| 26 | `Wireless` | `security::run_wireless_task` | `wireless` |
| 27 | `WirelessActive` | `security::run_wireless_active_task` | `wireless-advanced` |
| 28 | `DbPentest` | `db_pentest::run_db_pentest_task` | `db-pentest` |
| 29 | `Intercept` | `intercept::run_intercept_task` | `web-proxy` |
| 30 | `C2` | `c2::run_c2_task` | `c2` |

Feature-gated families without their feature return typed
`ExecutionError::FeatureUnavailable` at the boundary (one predictable layer);
the legacy `dispatch_inner` shim converts that to `Ok(TaskResult::Error(..))` for
backward compatibility. This is **never a silent no-op** — the caller always
receives an explicit error.

## Executor Adapters (retained, non-owning)

`executors/mod.rs:25` (`build_default_registry()`) registers adapters used by the tool/registry path.
Phase 1 completion record: the `ExecutorRegistry` (`OperationExecutor` trait objects)
is **retained** as the tool-path adapter registry (built + tested, used by
`EngineServices` composition), but it is **not** a second runtime dispatch owner:
runtime/embedded execution routes through `execute_canonical`, not the registry.
Unifying the tool path onto `execute_canonical` is tracked future work; validation
already converges via shared canonical contracts (`validate_tool_request_params`).

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

The `ExecutorRegistry` (`executors/registry.rs:9`) uses `rustc_hash::FxHashMap` for operation→executor mapping. It panics on duplicate operation ID registration (`registry.rs:33`).

## Interaction With Enforcement

Paths reach the single executor owner post-authorization:

### 1. Manual Surfaces (CLI/TUI)

CLI handlers classify once via `commands::route::route_for_commands`, convert to
canonical requests, enforce via `ctx.evaluate_and_enforce_operation(descriptor)`,
and render outcomes (CLI adapter owns rendering, not executor selection).
See [cli_commands.md](cli_commands.md). TUI actions enforce via `EnforcementFacade`
(exact binding), then `TuiTaskDispatcher` converts `TaskKind → canonical request →
execute_canonical` (shallow adapter, no second mapping).

```
CLI handler
    → route_for_commands() classify once
    → canonical request conversion
    → ctx.evaluate_and_enforce_operation(descriptor)
    → ApprovedOperation → execute_approved() → outcome → CLI rendering
TUI action
    → EnforcementFacade (exact matches_descriptor binding)
    → TuiTaskDispatcher: TaskKind → canonical → execute_canonical()
    → envelope (dispatch::task_result_envelope, single owner) + typed TaskResult rendering
```

### 2. Strict Protocol Surfaces (REST/MCP/gRPC/Agent)

> Phase D: adapters hold `tool::service::EngineServices` (`OperationCatalog`,
> `CheckedExecutor`, `PreflightService`) via `with_services`, not concrete
> `ToolRegistry`/`ToolDispatcher`. `CheckedExecutor` exposes only
> `dispatch_checked`; MCP uses the narrow `mcp::bridge::McpEngineBridge`, the
> agent uses `agent::services::AgentExecutionService`. Authorization stays in
> `EnforcementContext`; see `architecture/api_extraction_boundary.md`.


Route through `EnforcedDispatcher::dispatch_checked()` in `tool/dispatcher.rs:336`, which calls `validate_request_binding()` (`tool/dispatcher.rs:81`) to verify the `ApprovedOperation` token binding (tool ↔ canonical operation, target agreement). Fails closed before any executor runs.

```
REST/MCP/gRPC/Agent handler
    → EnforcementContext::evaluate()
    → ApprovedOperation
    → EnforcedDispatcher::dispatch_checked(approved, request)
        → validate_request_binding() [tool/dispatcher.rs:81]
        → ToolDispatcher::dispatch(request)
```

Callers include:
- `tool/protocol/rest.rs:864`
- `tool/protocol/grpc.rs:737,887`
- `tool/protocol/mcp/handlers/server.rs:695`

### 3. Daemon/Runtime Surfaces

`runtime_bridge::approve_run_request_bundle()` converts `RunRequest` → descriptor, obtains an `ApprovedOperation`, and the resulting `ApprovedRunRequest` executes through the canonical boundary via `dispatch_approved_runtime_request()`. See [runtime_bridge.md](runtime_bridge.md).

```
Daemon/Runtime
    → approve_run_request_bundle()
    → ApprovedRunRequest (token + request coupled)
    → dispatch_approved_runtime_request()
        → re-resolve descriptor, exact matches_descriptor check
        → CanonicalOperationRequest::from_task_kind()
        → execute_approved() (binding re-checked at entry)
```

## TaskResult Variants

`TaskResult` (`types.rs:80`) is a typed enum with 35 variants total (34 data variants + `Error`). Feature-gated variants compile out with their features:

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

1. **Executors are policy-free**: no `LoadedScope`/`EnforcementContext` access below this layer. Binding is re-verified at entry (`matches_descriptor`), but authorization decisions stay upstream.
2. **Every spawned task path carries timeout wrappers** per workspace convention (AGENTS.md lesson).
3. **Unsupported feature-gated families fail with typed `FeatureUnavailable`** at the boundary, never silently no-op (legacy shim maps to `TaskResult::Error`).
4. **The TUI must not host its own dispatch code** — tab UIs call into this module via the shallow `TuiTaskDispatcher` adapter (architecture guards 18, 85 enforced).
5. **`dispatch_inner` must only be invoked from manual surfaces** (CLI/TUI `ManualPermissive` context). Strict surfaces must never call it directly — route through `EnforcementContext::evaluate()` and `EnforcedDispatcher::dispatch_checked()`. New code prefers `execute_approved`.
6. **`dispatch_task` wraps errors in `TaskResult::Error`** — callers always receive a result through the channel, never a dropped request.
7. **Canonical IDs only at the boundary**: aliases resolve in surface adapters (`CommandRoute`, Clap tree, `TaskKind` mappings); `execute_approved` requires exact operation equality.
8. **Progress may coalesce; findings/terminal outcomes never drop** (`ExecutionSink` bounded 100, explicit loss counter).

## Bug Sweep

| Severity | File:Line | Issue |
|----------|-----------|-------|
| FIXED | `executors/registry.rs:9` | Previously used `std::collections::HashMap`; now uses `rustc_hash::FxHashMap` (`operation_to_executor: FxHashMap<String, usize>`), consistent with workspace convention. |
| INFO | `eggsec-runtime/src/request.rs:538,598` | `TaskKind::Resume` has capability name `"resume"` but operation identity `"pipeline"`. Intentional (a runtime can advertise pipeline support without resume), but the split is easy to misread when reconciling `RuntimeCapabilities::task_kinds` against catalog IDs. |
| LOW | `runtime_bridge/executor.rs` | Spawned `progress_forwarder` tokio task has no explicit timeout wrapper. However, it is bounded by the `dispatch_approved_runtime_request` lifetime and `CancellationToken` — the task drains on success and is aborted when the shared `race_with_cancel` reports cancellation. Acceptable but not explicit. |

## Testing

Boundary tests live in three mandatory-path suites:

- **`canonical_execution` unit tests** — canonical IDs (no aliases), packet-family sharing, `from_task_kind` exhaustiveness, executor routes, sink coalescing/drop-counting, binding rejections, envelope kind stability.
- **`crates/eggsec/tests/canonical_dispatch_ownership.rs`** (26 tests) — per-family CLI/runtime/tool normalization equivalence, identity/binding/route agreement, feature-gate consistency, error/outcome classification, `CommandRoute` ownership, daemon-bundle cancel race.
- **`crates/eggsec/tests/runtime_contract_closure.rs`** (18 tests, Phase 3) — surface round-trips, wire-identity agreement across all 30 `TaskKind` variants, wire JSON stability, no-target family failures, stable envelope kinds, embedded/daemon seam equivalence, approval-binding regression. Test count 17→18 on 2026-10-06: `TaskKind::Resume` was added to `all_task_kinds()` (the helper covered only 29 of 30 variants while the pin also read 29), and `enumerated_task_kind_wire_tags_match_the_enum` was added to stop the variant-name list drifting from the enum.
- **`dispatch/mod.rs` tests** — channel plumbing, legacy shim behavior, executor-registry coverage (retained adapter registry).
- **`runtime_bridge/bundle.rs` tests** — anti-tamper checks (operation/target mismatch).
- **TUI `task_runtime`/`task_dispatcher` tests** — embedded cancel contract (shared primitive).

## See Also

- [runtime_bridge.md](runtime_bridge.md) — Daemon/runtime entry point into dispatch
- [cli_commands.md](cli_commands.md) — CLI/TUI manual entry points
- [config.md](config.md) — `EnforcementContext`, `ExecutionPolicy`, `OperationMetadata`
- [tool/dispatcher.rs](../crates/eggsec/src/tool/dispatcher.rs) — `EnforcedDispatcher::dispatch_checked()` for strict surfaces
- [overview.md](overview.md) — System-wide architecture, enforcement model

## Phase 3 Closure Record (2026-09-11)

Starting SHA `585b4fec` (Phases 0–2 executed). Removed the last duplicate
semantic mappings: engine `operation_id_for_task_kind`/`target_for_task_kind`
`TaskKind`-arm tables (now thin delegates to the single wire-side `TaskKind` match;
daemon-executor and TUI private `TaskResult`→envelope matches (now the single
engine-owned `dispatch::task_result_envelope`); ad-hoc `cancel.cancelled()`
selects in both runtime adapters (now `race_with_cancel` by construction).
`RuntimeSurface` documented as a wire DTO with an exhaustive bidirectional
bridge (`Unknown` rejected). Retained (boundary conversions, not duplicate
ownership): `TaskKind` wire enum + params (serialization compat);
`CanonicalOperationRequest::from_task_kind` (typed engine conversion);
`ExecutionSink` legacy `(u64, u64)` bridge for domain workers (residual polish
when a worker changes its progress contract); large hotspot files with
rationale (see the Phase 3 plan closure record).

## Phase 1 Completion Record (2026-09-10)

Starting SHA `d4723af0` (Phase 0 executed); final SHA recorded in the plan file.
Removed: `dispatch_inner`'s `TaskKind` match (moved to `execute_canonical`);
registry-bridge prelude in `handle_command` (replaced by single `route_for_commands`
classification); TUI direct `dispatch_inner` call (now `execute_canonical` shallow
adapter); daemon direct `dispatch_inner` call (now `execute_approved` via bundle).
Retained (legitimate boundary conversions, not duplicate ownership): `TaskKind` and
CLI `Commands` enums (wire/UI representations); `Commands → CommandRoute` match
(compiler-enforced boundary conversion); `TaskKind → CanonicalOperationRequest`
conversion (explicit, exhaustive); `ExecutorRegistry` adapter registry (tool-path
composition, validation converges via shared canonical contracts); helper/lifecycle
routes (explicit non-operation); compatibility aliases at input/wire boundaries only.

*Last verified against source: 2026-09-11 (Phase 3 closure); counts/cites re-verified 2026-09-22 (systematic review); file sizes and `TaskKind`/`TaskResult` counts re-verified 2026-09-25; all counts, cites and the metadata↔`TaskKind` mapping re-verified 2026-10-06 (see Corrections note)*
