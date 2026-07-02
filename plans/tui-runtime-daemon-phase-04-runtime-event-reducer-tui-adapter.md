# Phase 4 Plan: Runtime Event Reducer and TUI Adapter

## Goal

Replace direct task-result-to-tab mutation with a runtime event reducer in the TUI. After Phase 3, runtime owns task dispatch. This phase makes the TUI consume neutral runtime events and update local view state through an explicit adapter boundary.

The TUI should render runtime state, not own canonical task state.

## Current Problem

The current `App::update` and `App::handle_result` path drains task progress/result channels and directly mutates specific tab structs. For example, a port scan result mutates `self.tabs.scan_ports`, a load test mutates `self.tabs.load`, a fingerprint result mutates `self.tabs.fingerprint`, and WAF results mutate WAF tab state.

This creates three architectural problems:

1. Results are associated with tab fields rather than task IDs and sessions.
2. A non-TUI frontend cannot consume the same task events without duplicating TUI routing logic.
3. Switching tabs or later supporting multiple tasks/frontends becomes harder because the result path assumes a local TUI application model.

## Desired End State for This Phase

Runtime emits `RuntimeEvent` values. `eggsec-tui` has a dedicated adapter/reducer that maps those events into TUI view state. The mapping from task ID to initiating tab is explicit and local to the TUI frontend.

The runtime remains unaware of tabs, Ratatui, input buffers, overlays, or rendered result buffers.

## TUI Adapter Design

Add a module such as:

```text
crates/eggsec-tui/src/runtime_adapter/
  mod.rs
  reducer.rs
  task_map.rs
  result_format.rs
```

Or if keeping module count lower:

```text
crates/eggsec-tui/src/app/runtime_adapter.rs
```

Suggested types:

```rust
pub struct TuiRuntimeAdapter {
    task_tabs: HashMap<TaskId, Tab>,
}

impl TuiRuntimeAdapter {
    pub fn register_task(&mut self, task_id: TaskId, tab: Tab);
    pub fn apply_event(&mut self, app: &mut App, event: RuntimeEvent);
}
```

Avoid putting event application directly into the runtime crate. The reducer is frontend-specific.

## Event Routing Rules

- `TaskQueued`: register task/tab relationship if not already registered; set tab state to queued/running as appropriate.
- `TaskStarted`: mark the initiating tab as running.
- `TaskProgress`: update the initiating tab's progress gauge.
- `TaskLog`: append to a runtime log panel or tab result buffer if useful; otherwise route to notification/history.
- `PolicyDecisionRequired`: show confirmation overlay for manual TUI sessions. Do not auto-approve.
- `TaskCompleted`: format outcome for the initiating tab and mark completed.
- `TaskFailed`: set tab error for the initiating tab.
- `TaskCancelled`: set tab idle/cancelled state and show notification if appropriate.
- `Audit`: append to history/audit view if one exists; otherwise log.
- `Snapshot`: hydrate view state when attaching to existing session later.

## Result Formatting

Separate result formatting from event routing. The reducer should decide which tab receives an outcome; a formatter should convert a `TaskOutcome` or typed result into the existing tab display format.

For early implementation, it is acceptable to call existing `tab.set_results(...)` methods from the reducer, but keep that in TUI adapter code. Do not let runtime call tab methods.

If Phase 3 used generic `TaskOutcome::Json`, add formatting helpers that deserialize based on task kind or outcome type. If Phase 3 added typed outcomes, match on typed outcomes.

## Task-to-Tab Mapping

When the TUI submits a task, it should register:

```rust
task_id -> initiating Tab
```

This is important because the current tab may change while the task runs. Existing tests already check that stop/result behavior targets the task tab rather than current tab; preserve and expand that coverage.

If multiple tasks are not supported yet, keep the map anyway. It prevents future churn.

## Policy Prompt Handling

For `RuntimeEvent::PolicyDecisionRequired`, the TUI should show the existing confirmation overlay or an equivalent manual approval UI. The approval response should be sent back to runtime in later daemon/client phases. In this phase, if approval plumbing is not yet fully runtime-mediated, keep a compatibility bridge but document it.

Do not weaken strict surfaces. If runtime session surface is MCP/agent/CI, confirmation should be denied or represented as a hard policy failure according to existing enforcement semantics.

## App Update Loop Changes

Change `App::update` so that it drains runtime events rather than legacy progress/result receivers. The update loop should become roughly:

```rust
while let Some(event) = runtime_event_rx.try_recv() {
    self.runtime_adapter.apply_event(self, event);
    dirty = true;
}
```

If temporary legacy channels still exist, isolate them behind a clearly named compatibility function and remove them before phase completion if possible.

## Files Likely to Change

- `crates/eggsec-tui/src/app/state_update.rs`
- `crates/eggsec-tui/src/app/task_runtime.rs`
- `crates/eggsec-tui/src/app/state.rs`
- `crates/eggsec-tui/src/app/mod.rs`
- `crates/eggsec-tui/src/tabs/mod.rs`
- `crates/eggsec-tui/src/tabs/*`
- `crates/eggsec-tui/src/runtime_adapter/*` or equivalent new module
- `crates/eggsec-runtime/src/event.rs`
- `crates/eggsec-runtime/src/outcome.rs`
- `crates/eggsec-runtime/src/session.rs`

## Non-goals

Do not add daemon transport.

Do not redesign all tab rendering.

Do not force all tab result buffers into runtime state yet; Phase 5 handles deeper session/view split.

Do not make runtime aware of TUI tabs.

## Implementation Steps

1. Add `TuiRuntimeAdapter` with a `TaskId -> Tab` mapping.
2. Register task-tab mapping when TUI submits a runtime task.
3. Add `apply_event` handling for queued, started, progress, completed, failed, cancelled, policy, audit, and snapshot events.
4. Move existing direct result handling from `App::handle_result` into reducer/formatter functions.
5. Ensure results target the initiating tab, not the current tab.
6. Update `App::update` to drain runtime events as the primary path.
7. Remove or quarantine legacy direct progress/result receiver handling.
8. Add tests for event routing.
9. Add tests for tab switch during task execution.
10. Add tests for failed/cancelled task event rendering.

## Suggested Tests

- Submit task from Recon tab, switch to Dashboard, deliver `TaskCompleted`, verify Recon tab receives result.
- Submit task from Port Scan tab, deliver progress event, verify Port Scan progress updates.
- Deliver `TaskFailed` for a task whose initiating tab is not current, verify correct tab error.
- Deliver unknown task event, verify warning/logging but no panic.
- Deliver snapshot event, verify adapter can hydrate known task state or safely ignore unsupported fields.
- Deliver policy prompt event under TUI manual surface, verify confirmation UI path is invoked.

## Validation

Run:

```bash
cargo check -p eggsec-runtime
cargo test -p eggsec-runtime
cargo check -p eggsec-tui
cargo test -p eggsec-tui
cargo check -p eggsec-cli
```

Manual smoke checks:

- Launch TUI.
- Start a task and switch tabs before completion.
- Confirm result appears on the initiating tab.
- Cancel a task and confirm the initiating tab is updated.
- Trigger or simulate an error and confirm error placement.
- Confirm redraw behavior remains responsive.

## Acceptance Criteria

- Runtime events are the canonical task update path for the TUI.
- TUI result routing is isolated in an adapter/reducer module.
- Runtime does not depend on TUI tabs or rendering types.
- Results/progress/errors target task IDs and initiating tabs rather than current tab.
- Existing TUI behavior remains stable.
