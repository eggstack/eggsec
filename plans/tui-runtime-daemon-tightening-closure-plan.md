# TUI Runtime/Daemon Architecture Tightening and Closure Plan

## Purpose

This plan closes the first runtime/TUI decoupling milestone after the implementation of the first five roadmap phases. The repo is now in a much better architectural state: `eggsec-runtime` exists, runtime owns task lifecycle, dispatch has moved out of `eggsec-tui`, the TUI has a runtime event reducer, and runtime sessions now own execution-surface/scope-bound task state.

The remaining work is not to add daemon transport yet. The goal is to tighten the seams introduced during migration so the next daemon phase does not fossilize temporary compatibility bridges into a public API.

## Current State Summary

The current shape appears to be:

- `eggsec-runtime` owns `SessionId`, `TaskId`, `RunRequest`, `TaskKind`, `RuntimeEvent`, `TaskStatus`, `TaskProgress`, `TaskOutcome`, `Runtime`, `RuntimeSession`, `SessionSnapshot`, `SessionScope`, and lifecycle APIs.
- `eggsec::dispatch` owns the canonical task dispatch path and the moved task result/domain structs that previously lived under `eggsec-tui/src/workers`.
- `eggsec-tui/src/workers` has been removed.
- TUI tabs now build `RunRequest`/`TaskKind` instead of TUI-local `TaskConfig` values.
- `TuiRuntimeAdapter` maps `TaskId` to initiating `Tab` and applies runtime lifecycle events to TUI state.
- Runtime sessions bind execution surface and scope metadata.
- Snapshot hydration preserves completed task snapshots.

This is the correct overall direction. The closure pass should focus on making the boundary precise, truthful, and test-protected.

## Primary Risks to Close

### 1. Typed result compatibility bridge still exists

The Phase 4 adapter made runtime lifecycle events canonical, but typed domain results still appear to flow through legacy `progress_rx`/`result_rx` compatibility channels for rich tab rendering. That means runtime events are canonical for lifecycle but not yet canonical for domain result delivery.

This is acceptable during migration, but it should be resolved before daemon transport work. A daemon client cannot consume TUI-local result channels.

### 2. Capability metadata may overstate runtime behavior

Runtime capability defaults should describe current behavior, not roadmap intent. In particular, if the runtime still enforces a single-active-task policy, `supports_multiple_active_tasks` must be false. If daemon transports do not exist yet, transport capability lists should not advertise working `unix-socket`, `stdio`, WebSocket, or other transports unless implemented.

### 3. Engine/runtime dependency direction needs a conscious decision

`eggsec::dispatch` now uses `eggsec-runtime` protocol types. This is workable for the migration, but it creates a tighter coupling between the engine crate and the runtime protocol. Before daemon/public protocol work, decide whether to keep this dependency intentionally or extract shared DTOs into a smaller protocol crate.

### 4. Plan/audit trail should be preserved

Some early plan files may have been removed after implementation. If that was intentional, archive or status files should replace them. The repo has used `plans/` as an implementation handoff/audit trail, so deletion makes later review harder.

### 5. Feature-gated dispatch paths need focused verification

Dispatch moved across crate boundaries. The riskiest regressions will be in feature-gated surfaces, especially `stress-testing`, `packet-inspection`, `nse`, `db-pentest`, `web-proxy`, `wireless`, `wireless-advanced`, and `c2`. The closure pass should add explicit compile/test coverage and architecture guards for these paths.

## Non-goals

Do not add daemon transport in this pass.

Do not add WebSocket/SSE/gRPC yet.

Do not introduce multi-client semantics yet.

Do not make broad UI/UX changes.

Do not redesign all engine APIs unless needed to close a boundary leak.

Do not weaken execution-surface enforcement. TUI/CLI manual behavior and MCP/agent/CI strict behavior must remain separate.

## Workstream 1: Eliminate or Formalize the Typed Result Bridge

### Goal

Make the runtime event stream sufficient for non-TUI frontends to receive task completion data, or explicitly mark remaining TUI-only result delivery as local rendering compatibility with a clear removal path.

### Current issue

`TuiRuntimeAdapter` receives runtime lifecycle events, but typed domain results may still be delivered through `result_rx` as `eggsec::dispatch::TaskResult`. That means runtime `TaskCompleted` events may carry `TaskOutcome::Empty` or generic output while the actual useful data flows through a TUI-only side channel.

### Preferred target

Task completion should use a typed or structured runtime outcome path:

```rust
RuntimeEvent::TaskCompleted {
    session_id,
    task_id,
    outcome: TaskOutcome,
}
```

Where `TaskOutcome` can carry:

- a typed dispatch result enum, if the type can live in a shared crate;
- a JSON payload with a stable `kind` discriminator;
- an artifact reference plus summary;
- structured text/log output for simple tasks.

### Recommended approach

Use a conservative two-step migration.

First, add a stable result envelope to `eggsec-runtime`:

```rust
pub struct TaskResultEnvelope {
    pub kind: String,
    pub summary: Option<String>,
    pub payload: serde_json::Value,
    pub artifacts: Vec<ArtifactRef>,
}

pub struct ArtifactRef {
    pub id: String,
    pub kind: String,
    pub path: Option<String>,
    pub mime_type: Option<String>,
    pub summary: Option<String>,
}
```

Then add a `TaskOutcome::Result(TaskResultEnvelope)` variant. Avoid moving every typed result into `eggsec-runtime` immediately if that creates dependency bloat.

Second, update `eggsec::dispatch` or the TUI dispatcher adapter so that each completed task returns a meaningful `TaskOutcome::Result(...)` alongside any temporary typed result channel. The TUI can continue to use typed `TaskResult` for rich rendering temporarily, but tests should confirm the runtime outcome is not empty for supported tasks.

### Minimal acceptable closure

If full outcome migration is too large for this pass, document the compatibility bridge explicitly and add an architectural guard/TODO with a tracked plan. Also ensure every `TaskCompleted` has at least a non-empty summary or result kind so daemon work has a usable baseline.

### Implementation steps

1. Inspect `crates/eggsec-tui/src/app/task_dispatcher.rs`, `task_runtime.rs`, and `state_update.rs` for remaining `progress_rx`/`result_rx` paths.
2. Inspect `crates/eggsec/src/dispatch/types.rs` and `dispatch/mod.rs` for all `TaskResult` variants.
3. Add `TaskResultEnvelope` and `ArtifactRef` to `eggsec-runtime` if not already present.
4. Add `TaskOutcome::Result(TaskResultEnvelope)` or equivalent.
5. Add conversion helpers from representative `eggsec::dispatch::TaskResult` values to runtime result envelopes.
6. Ensure `TuiExecutor` returns meaningful `TaskOutcome` from dispatch.
7. Keep TUI typed rendering if needed, but make it a rendering optimization rather than the only useful result path.
8. Add tests proving `TaskCompleted` includes a useful outcome for representative tasks.

### Acceptance criteria

- Runtime `TaskCompleted` events contain useful outcome data for representative task kinds.
- The compatibility channel is either removed or explicitly isolated as TUI rendering compatibility.
- No daemon-necessary result data exists only in `eggsec-tui`.
- Tests cover at least port scan, recon, load test, GraphQL/OAuth if easy, and one feature-gated task if practical.

## Workstream 2: Runtime Capability Truthfulness

### Goal

Make `RuntimeCapabilities` a truthful representation of what is currently implemented under the active build features.

### Problems to check

- `supports_multiple_active_tasks` should match `RuntimeConfig::max_active_tasks_per_session` and actual runtime behavior.
- `supports_multiple_sessions` should reflect whether runtime really supports independent sessions in the current implementation. If tests prove multiple sessions work, mark it true; otherwise false.
- `transports` should not list daemon transports that do not exist yet.
- Feature-gated task capabilities should be present only when the feature is compiled.
- Capability names should map deterministically to `TaskKind` variants and tab specs where possible.

### Implementation steps

1. Review `crates/eggsec-runtime/src/capabilities.rs`.
2. Add `Runtime::capabilities()` or `RuntimeSession::capabilities()` if capability reporting currently uses only a static default.
3. Make active-task support derive from config:

```rust
supports_multiple_active_tasks = max_active_tasks_per_session > 1
```

4. Remove unimplemented transports from default capability output. Use `embedded` or `in-process` if that is the only current transport.
5. Gate task capabilities with `#[cfg(feature = ...)]` where applicable, or move capability construction to the engine/TUI layer that knows feature availability.
6. Add tests for default capabilities and configured multi-active behavior.
7. Add tests ensuring unsupported feature-gated task kinds are not advertised under default features.

### Acceptance criteria

- Runtime capabilities do not advertise unimplemented transports.
- Multi-active support is accurate.
- Feature-gated task capabilities are accurate under default and selected features.
- Capability names are stable and documented.

## Workstream 3: Engine/Runtime Dependency Boundary Decision

### Goal

Decide and document whether `eggsec` should depend on `eggsec-runtime`, or whether shared protocol DTOs should move to a smaller crate before daemon API work.

### Current shape

`eggsec::dispatch` appears to consume `eggsec-runtime::RunRequest`/`TaskKind` directly. This simplified the migration, but it also means the engine crate depends on runtime protocol types.

### Acceptable options

#### Option A: Keep `eggsec -> eggsec-runtime` intentionally

This is acceptable if `eggsec-runtime` remains lightweight and protocol/DTO-oriented. Document that `eggsec-runtime` is not just a daemon host; it is the shared runtime protocol crate. Keep it free of transport dependencies.

Required guardrails:

- `eggsec-runtime` must not depend on `eggsec`.
- `eggsec-runtime` must not depend on `eggsec-tui`.
- `eggsec-runtime` must not depend on Ratatui/crossterm.
- transport dependencies must not enter `eggsec-runtime`; use `eggsec-daemon` or transport crates later.

#### Option B: Extract `eggsec-protocol` or `eggsec-runtime-core`

Move DTOs such as `RunRequest`, `TaskKind`, `RuntimeEvent`, `TaskOutcome`, `SessionId`, and `TaskId` into a smaller crate. Then `eggsec-runtime` owns lifecycle and depends on the protocol crate, while `eggsec` depends only on the protocol crate.

This is cleaner but more churn. It may be the better choice before publishing daemon APIs.

### Recommended closure for now

Do not refactor immediately unless dependency checks show a cycle risk. Instead, document Option A as the current intentional boundary and add architecture guards that prevent `eggsec-runtime` from pulling in TUI, transport, or engine-heavy dependencies.

### Implementation steps

1. Inspect `crates/eggsec-runtime/Cargo.toml` for dependency creep.
2. Inspect `crates/eggsec/Cargo.toml` and dispatch modules for runtime usage.
3. Update `architecture/overview.md` and `architecture/tui.md` to describe the chosen dependency direction.
4. Add or update architecture guard scripts to fail if `eggsec-runtime` imports `eggsec_tui`, `ratatui`, `crossterm`, `axum`, `tonic`, `tokio-tungstenite`, or daemon transport modules.
5. Add a note that daemon transport must live outside `eggsec-runtime`.

### Acceptance criteria

- Dependency direction is documented.
- Architecture guards enforce the chosen boundary.
- `eggsec-runtime` remains free of TUI and transport dependencies.
- Any future protocol-extraction option is documented as a later refactor, not an ambiguous gap.

## Workstream 4: Feature-Gated Dispatch Verification

### Goal

Ensure moving workers into `eggsec::dispatch` did not silently break feature-gated task paths.

### Implementation steps

1. Add compile checks or CI matrix entries for representative features:

```bash
cargo check -p eggsec-runtime
cargo test -p eggsec-runtime
cargo check -p eggsec-tui
cargo test -p eggsec-tui
cargo check -p eggsec-cli
cargo check -p eggsec-tui --features stress-testing,packet-inspection
cargo check -p eggsec-tui --features nse
cargo check -p eggsec-tui --features db-pentest
cargo check -p eggsec-tui --features web-proxy
cargo check -p eggsec-tui --features wireless,wireless-advanced
cargo check -p eggsec-cli --features rest-api
```

2. If `full` is expected to compile in normal dev environments, add:

```bash
cargo check -p eggsec-cli --features full
```

If full requires system dependencies, document why it is excluded.

3. Add tests for `RunRequest` construction from representative feature-gated tabs where possible.
4. Add tests that unsupported feature-gated task kinds return structured unsupported errors rather than panics.
5. Add a static grep guard that `eggsec-tui/src/workers` remains absent.
6. Add a static grep guard that TUI code does not recreate a canonical `match TaskKind` execution dispatcher.

### Acceptance criteria

- Feature-gated compile checks pass or documented exclusions exist.
- Unsupported task kinds fail cleanly.
- TUI workers directory remains removed.
- Canonical dispatch remains outside `eggsec-tui`.

## Workstream 5: TUI Runtime Adapter Closure

### Goal

Ensure `TuiRuntimeAdapter` is robust enough to be the long-lived frontend adapter pattern.

### Issues to check

- Unknown task events should not panic.
- Terminal events should unregister task mappings exactly once.
- Events should route by `TaskId`, not current tab.
- Lazy session/task registration should not race with early runtime events.
- Snapshot hydration should not duplicate completed tasks after reconnect-like flows.
- Policy decision events should be handled deliberately rather than silently ignored if manual prompts are expected later.

### Implementation steps

1. Review `crates/eggsec-tui/src/app/runtime_adapter/mod.rs`.
2. Add tests for duplicate terminal events, unknown task IDs, task started before explicit registration, and task completion after current tab changes.
3. Add a test for snapshot hydration preserving completed tasks without duplicating live completed tasks.
4. If `PolicyDecisionRequired` is currently ignored, add a visible TODO and test that it does not panic. If practical, route it to the existing confirmation overlay for TUI manual sessions.
5. Ensure `TaskCancelled` and `TaskFailed` clear active UI state consistently.

### Acceptance criteria

- Runtime adapter has regression tests for routing and race-prone event orderings.
- Event handling is deterministic and non-panicking.
- Policy events are either routed or explicitly documented as a future closure item.

## Workstream 6: Plan and Documentation Audit Trail

### Goal

Keep the implementation audit trail usable.

### Implementation steps

1. Inspect `plans/` for current runtime/daemon plan files.
2. If Phase 1/2 plan files were removed intentionally, either restore them or create a summary file such as:

```text
plans/tui-runtime-daemon-completed-phase-index.md
```

3. Mark each completed phase with commit SHA, completion status, and known follow-up items.
4. Ensure the roadmap still exists and points to the current state.
5. Update `architecture/tui.md` with the current post-Phase-5 architecture, including which compatibility bridges remain.
6. Update `AGENTS.md` or local skill guidance to reflect the current preferred paths for adding new task types.

### Acceptance criteria

- A future contributor can trace plan -> implementation -> remaining gap.
- Completed plans are not silently lost.
- Docs name the typed-result bridge if it remains.
- Docs specify where new task kinds, dispatch logic, runtime DTOs, and TUI rendering adapters belong.

## Workstream 7: Architecture Guard Additions

### Goal

Prevent regression to TUI-owned execution or transport creep in runtime.

### Suggested guards

Add checks to existing architecture guard scripts or create a dedicated script. Suggested assertions:

1. `crates/eggsec-tui/src/workers` must not exist.
2. `eggsec-runtime` must not import `ratatui`, `crossterm`, or `eggsec_tui`.
3. `eggsec-runtime` must not import daemon transport crates such as `axum`, `tonic`, `tokio_tungstenite`, or `tower`.
4. `eggsec-tui` must not define a canonical `TaskConfig` enum.
5. `eggsec-tui` must not define a canonical `TaskResult` enum.
6. Any `match TaskKind` execution dispatcher in `eggsec-tui` should be rejected unless it is clearly a view/request builder and not execution.
7. Runtime capability defaults must not advertise unimplemented transports.

### Acceptance criteria

- Architecture guard script catches TUI worker reintroduction.
- Architecture guard script catches TUI/transport dependency leaks into runtime.
- Guards are included in the documented validation command set.

## Recommended Implementation Order

1. Audit remaining compatibility channels and document exact current flow.
2. Fix runtime capability truthfulness.
3. Add architecture guards for no TUI workers, no TUI deps in runtime, and no unimplemented transports.
4. Add adapter race/unknown-event tests.
5. Add result envelope or, at minimum, non-empty runtime outcome summaries.
6. Add feature-gated dispatch compile/test coverage.
7. Restore/archive completed plan audit trail.
8. Update architecture docs and handoff guidance.

## Validation Checklist

Run the following where practical:

```bash
cargo fmt --all --check
cargo check -p eggsec-runtime
cargo test -p eggsec-runtime
cargo check -p eggsec
cargo test --lib -p eggsec
cargo check -p eggsec-tui
cargo test -p eggsec-tui
cargo check -p eggsec-cli
```

Feature checks:

```bash
cargo check -p eggsec-tui --features stress-testing,packet-inspection
cargo check -p eggsec-tui --features nse
cargo check -p eggsec-tui --features db-pentest
cargo check -p eggsec-tui --features web-proxy
cargo check -p eggsec-tui --features wireless,wireless-advanced
cargo check -p eggsec-cli --features rest-api
```

Architecture checks:

```bash
./scripts/check-architecture-guards.sh
```

If the repo has make targets for these, prefer the project-standard targets.

## Final Acceptance Criteria

This closure pass is complete when:

- Runtime/TUI lifecycle ownership is documented and guard-protected.
- TUI workers remain removed.
- Runtime capability output is truthful.
- The typed result bridge is either eliminated or explicitly isolated with useful runtime outcomes available.
- Feature-gated dispatch paths compile under representative feature sets.
- Runtime sessions preserve execution surface and scope metadata.
- Snapshot hydration is non-lossy for completed task records.
- `eggsec-runtime` remains free of TUI and transport dependencies.
- The plan/audit trail is restored or indexed.

Only after these criteria are met should the next roadmap phase start daemon transport work.
