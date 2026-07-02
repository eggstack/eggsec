# Phase 3 Plan: Worker Dispatch Migration

## Goal

Move task execution dispatch out of `eggsec-tui` and into the frontend-neutral runtime/engine layer. After this phase, `eggsec-tui` should no longer own how assessment tasks are executed. It should translate tab inputs into runtime `RunRequest` values, submit them, and render events.

This phase removes the largest architectural blocker to daemon mode: the current TUI-local worker dispatcher.

## Current Problem

`eggsec-tui/src/workers/runner.rs` defines the effective execution dispatcher for interactive tasks. It matches over `TaskConfig` and calls worker modules for load tests, stress tests, port scans, endpoint scans, fingerprinting, fuzzing, WAF tasks, recon, packet operations, GraphQL/OAuth, NSE, security tasks, DB pentest, proxy intercept, and C2.

Because this dispatch lives in the TUI crate, any daemon, CLI client, web frontend, desktop frontend, mobile frontend, or codegg integration would either need to depend on `eggsec-tui` or duplicate the dispatch logic. That is the wrong dependency direction.

## Desired End State for This Phase

Runtime accepts frontend-neutral `RunRequest`/`TaskKind` values and dispatches them to engine operations. The TUI does not have a `workers` execution subsystem except for temporary conversion helpers if absolutely needed.

`eggsec-runtime` must not import TUI tab modules, Ratatui, crossterm, or TUI component types.

## Dispatch Ownership Options

There are two viable ownership models:

### Preferred: Runtime owns orchestration, engine owns capability functions

`eggsec-runtime` owns task lifecycle and dispatch orchestration. It calls public async functions in `eggsec` for the actual work. If current callable functions are only exposed through CLI handlers or TUI workers, extract those functions into engine modules first.

This gives the cleanest daemon model.

### Acceptable Transitional: Runtime owns dispatcher, reuses moved worker modules

Move the existing TUI worker modules into `eggsec-runtime` with minimal code changes, then remove TUI dependencies from them incrementally. This is acceptable if direct engine APIs are not yet clean enough.

Avoid leaving moved workers dependent on TUI tab result structs.

## Types That Must Move or Be Replaced

The following TUI-local concepts should be replaced with runtime/engine-neutral types:

- `TaskConfig` -> `eggsec_runtime::TaskKind` / `RunRequest`
- `TaskResult` -> `eggsec_runtime::TaskOutcome` / typed result enum
- `TracerouteHopResult` -> runtime or engine packet result type
- `crate::tabs::graphql::GraphQlResults` -> neutral GraphQL result type
- `crate::tabs::oauth::OAuthResults` -> neutral OAuth result type
- `crate::tabs::nse::NseResults` -> neutral NSE result type if feature enabled
- `crate::tabs::recon::ReconOptions` -> neutral recon options type

If a type is domain-specific and useful outside the TUI, prefer moving it to `eggsec` or an appropriate domain crate. If it is runtime/protocol-specific, keep it in `eggsec-runtime`.

## Feature Gating Requirements

Preserve the current feature behavior:

- `stress-testing` gates low-level stress/packet send paths as appropriate.
- `packet-inspection` gates packet capture/traceroute/crafting support as appropriate.
- `nse` gates NSE tasks.
- `advanced-hunting`, `headless-browser`, `compliance`, `database`, `external-integrations`, `finding-workflow`, `vuln-management`, `wireless`, `wireless-advanced`, `db-pentest`, `web-proxy`, and `c2` remain correctly gated.

Runtime capability discovery should know which task kinds are available under the current build features. If a frontend submits an unavailable task, runtime should return a structured unsupported-capability error rather than panicking or silently mapping to dashboard behavior.

## Implementation Steps

1. Inventory all functions currently called by `eggsec-tui/src/workers/runner.rs`.
2. For each task category, identify whether the real engine function already exists in `eggsec` or whether the TUI worker contains unique orchestration logic.
3. Move unique orchestration logic into `eggsec-runtime` or `eggsec` depending on whether it is lifecycle/runtime logic or domain engine logic.
4. Replace TUI-local option/result structs with neutral equivalents.
5. Implement a runtime dispatcher:

```rust
pub async fn dispatch_task(
    request: RunRequest,
    events: RuntimeEventSink,
    cancel: CancellationToken,
) -> Result<TaskOutcome, RuntimeError>
```

6. Translate progress updates into `RuntimeEvent::TaskProgress`.
7. Translate final results into typed or generic `TaskOutcome` values.
8. Update the Phase 2 runtime executor to call the runtime dispatcher instead of the TUI compatibility executor.
9. Update TUI task launch code to build `RunRequest`/`TaskKind` directly.
10. Remove or deprecate `eggsec-tui/src/workers` execution modules.
11. Keep any remaining TUI-only translation helpers outside the runtime path and mark them as temporary if they must remain.

## Suggested Migration Order

Start with simple, low-risk task families:

1. Load test
2. Port scan
3. Endpoint scan
4. Fingerprint
5. Recon

Then move protocol/security tasks:

6. GraphQL
7. OAuth
8. Auth test
9. WAF/WAF stress
10. Fuzz

Then move feature-gated or higher-risk tasks:

11. Packet capture/traceroute/send
12. Stress testing
13. NSE
14. Browser/compliance/storage/integrations/workflow/vuln/wireless
15. DB pentest/web proxy/C2

This order reduces the chance of breaking feature-gated surfaces early.

## Files Likely to Change

Runtime:

- `crates/eggsec-runtime/src/dispatcher.rs`
- `crates/eggsec-runtime/src/request.rs`
- `crates/eggsec-runtime/src/event.rs`
- `crates/eggsec-runtime/src/outcome.rs`
- `crates/eggsec-runtime/src/capabilities.rs`
- `crates/eggsec-runtime/Cargo.toml`

TUI:

- `crates/eggsec-tui/src/workers/runner.rs`
- `crates/eggsec-tui/src/workers/*.rs`
- `crates/eggsec-tui/src/app/task_runtime.rs`
- `crates/eggsec-tui/src/app/state_update.rs`
- `crates/eggsec-tui/src/tabs/*`

Engine:

- `crates/eggsec/src/loadtest/*`
- `crates/eggsec/src/scanner/*`
- `crates/eggsec/src/recon/*`
- `crates/eggsec/src/fuzzer/*`
- `crates/eggsec/src/waf/*`
- `crates/eggsec/src/commands/fuzz_convert.rs`
- feature-gated modules as needed

## Non-goals

Do not add daemon transport yet.

Do not redesign every engine API. Extract the minimum clean callable functions required to move dispatch out of the TUI.

Do not convert all outcomes to perfect typed domain models if that creates large churn. Generic JSON/text outcomes are acceptable temporarily if documented.

Do not add multi-task support unless Phase 2 already introduced it cleanly.

## Validation

Run at minimum:

```bash
cargo check -p eggsec-runtime
cargo test -p eggsec-runtime
cargo check -p eggsec-tui
cargo test -p eggsec-tui
cargo check -p eggsec-cli
```

Run feature smoke checks where practical:

```bash
cargo check -p eggsec-tui --features stress-testing,packet-inspection
cargo check -p eggsec-tui --features rest-api
cargo check -p eggsec-tui --features db-pentest,web-proxy
cargo check -p eggsec-cli --features full
```

Manual smoke tests:

- Launch embedded TUI.
- Run load test against a controlled/local endpoint.
- Run port scan against a controlled/local target.
- Run recon against a safe test target.
- Run GraphQL/OAuth dry/safe tests if available.
- Confirm cancellation still works.
- Confirm feature-gated tabs either work or report unavailable capabilities cleanly.

## Acceptance Criteria

- Runtime dispatch can execute representative tasks without importing TUI modules.
- TUI no longer owns the canonical execution match over task kinds.
- `eggsec-tui/src/workers` is removed or reduced to temporary compatibility code with clear TODOs.
- Task progress and completion flow through runtime events.
- Feature-gated capabilities remain gated and report unavailable status cleanly.
