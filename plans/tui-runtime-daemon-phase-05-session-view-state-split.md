# Phase 5 Plan: Session/View State Split

## Goal

Split canonical runtime session state from local TUI view state. By the end of this phase, runtime sessions should be constructible, queryable, and testable without creating a Ratatui terminal or `eggsec-tui::App`. The TUI should become a view/controller around a runtime session handle.

This phase completes the first major architecture milestone: Eggsec can run embedded in the TUI using a frontend-neutral runtime, and the remaining work toward daemon mode becomes transport and persistence rather than core disentanglement.

## Current Problem

The TUI `App` struct currently owns too many concerns:

- current tab
- input mode
- session manager
- theme manager
- tab store
- HTTP/global options
- history
- overlays
- search and quick switch state
- task state
- command palette
- redraw flags
- bookmarks
- theme load state
- enforcement facade

Some of these are purely visual. Others are canonical runtime/session state. Daemon mode requires a session model that can exist without terminal rendering and can later be observed by multiple clients.

## Desired End State for This Phase

Introduce a clear split:

### Runtime/session-owned state

- `SessionId`
- config snapshot
- loaded scope
- execution surface
- enforcement context or enforcement session binding
- active task records
- completed task records
- task progress/status
- task outcomes or result records
- audit events
- runtime capabilities
- artifact references, if already present
- session metadata such as created/updated timestamps

### TUI/view-owned state

- current tab
- input mode
- tab input fields
- focus areas
- overlays and notifications
- command palette
- quick switch and search UI state
- theme state
- scroll offsets
- bookmarks
- Ratatui layout caches
- rendered result buffers if not yet moved to neutral records

The TUI may keep formatted result buffers for display, but runtime should own the canonical task record and completion status.

## Session API

Add or complete a runtime session model:

```rust
pub struct RuntimeSession {
    pub id: SessionId,
    // internal canonical state
}

impl RuntimeSession {
    pub fn snapshot(&self) -> SessionSnapshot;
    pub fn execution_surface(&self) -> RuntimeSurface;
    pub fn capabilities(&self) -> RuntimeCapabilities;
    pub fn active_tasks(&self) -> Vec<TaskSnapshot>;
    pub fn completed_tasks(&self) -> Vec<TaskSnapshot>;
}
```

The public API can be exposed through `Runtime` rather than direct mutable session access, but tests should be able to create and inspect session state without TUI.

## TUI App Refactor

Rename or conceptually narrow `App` into a TUI view state object. This does not need a full type rename if that creates churn, but its fields should become clearly separated.

Add something like:

```rust
pub struct RuntimeBinding {
    pub session_id: SessionId,
    pub runtime: RuntimeHandle,
    pub events: RuntimeEventReceiver,
}
```

Then `App` owns `RuntimeBinding` plus view state. Avoid storing canonical task data directly in `App` except where retained for temporary compatibility.

## Enforcement State

Currently the TUI creates enforcement state as `ExecutionSurface::TuiManual`. In the new split, session creation should bind the execution surface and loaded scope to the runtime session. The TUI can still display enforcement prompts and warnings, but canonical enforcement configuration should be session/runtime-owned.

Requirements:

- TUI embedded sessions use TUI manual surface.
- CLI manual sessions use CLI manual or strict surface.
- Future daemon agent/MCP sessions use strict surfaces.
- Manual overrides and approvals remain auditable.

If the existing TUI enforcement facade is not ready to move entirely, introduce a temporary bridge. Document it clearly and ensure strict surfaces cannot use TUI-only permissive paths.

## History and Session Restore

Decide where history belongs. Recommended split:

- Runtime owns canonical task history and task records.
- TUI owns view history formatting and local UI navigation state.

Existing TUI session restore should keep working. If migrating session files now is too large, keep TUI session restore for view state and add runtime snapshot restore later. Do not silently drop current session restore behavior.

## Snapshot Hydration

Add support for hydrating TUI view state from `SessionSnapshot`. This is needed for later daemon attach mode.

For Phase 5, snapshot hydration can be partial:

- active task list
- completed task summaries
- progress/status
- capabilities

Full result reconstruction can come later if current tab display buffers are not yet runtime-owned.

## Files Likely to Change

Runtime:

- `crates/eggsec-runtime/src/session.rs`
- `crates/eggsec-runtime/src/runtime.rs`
- `crates/eggsec-runtime/src/event.rs`
- `crates/eggsec-runtime/src/capabilities.rs`
- `crates/eggsec-runtime/src/error.rs`

TUI:

- `crates/eggsec-tui/src/app/mod.rs`
- `crates/eggsec-tui/src/app/state.rs`
- `crates/eggsec-tui/src/app/runner.rs`
- `crates/eggsec-tui/src/app/task_runtime.rs`
- `crates/eggsec-tui/src/app/state_update.rs`
- `crates/eggsec-tui/src/session/*`
- `crates/eggsec-tui/src/tabs/history.rs`
- `crates/eggsec-tui/src/tabs/dashboard.rs`

Engine/enforcement:

- `crates/eggsec/src/config/*`
- `crates/eggsec/src/audit.rs`
- `crates/eggsec/src/commands/handlers/mod.rs`

## Implementation Steps

1. Define canonical runtime session state and snapshot APIs.
2. Move active/completed task records into runtime session state.
3. Bind execution surface, loaded scope, and policy context to runtime session creation.
4. Add a `RuntimeBinding` or equivalent to TUI `App`.
5. Remove canonical active task metadata from TUI `TaskState`; keep only view-specific metadata such as initiating tab and local display pause state.
6. Update TUI initialization to create an embedded runtime session.
7. Update TUI event reducer to use runtime snapshot/task records where appropriate.
8. Add partial snapshot hydration for future daemon attach.
9. Preserve existing TUI session restore for view state.
10. Add tests that create runtime sessions and submit/query tasks without TUI.
11. Add tests that construct TUI around an existing runtime session binding.

## Non-goals

Do not implement daemon transport.

Do not require full durable persistence yet.

Do not require multi-client support yet.

Do not remove all TUI formatted result buffers if doing so would create excessive churn.

Do not weaken enforcement separation between manual and strict surfaces.

## Validation

Run:

```bash
cargo check -p eggsec-runtime
cargo test -p eggsec-runtime
cargo check -p eggsec-tui
cargo test -p eggsec-tui
cargo check -p eggsec-cli
```

Feature checks:

```bash
cargo check -p eggsec-tui --features stress-testing,packet-inspection
cargo check -p eggsec-cli --features rest-api
```

Manual smoke checks:

- Launch TUI in embedded mode.
- Confirm settings/theme/session view behavior still works.
- Start and complete a task.
- Cancel a task.
- Restart TUI and confirm existing restore behavior is not regressed.
- Confirm enforcement prompts/warnings still appear for manual TUI operations.

## Acceptance Criteria

- Runtime session state can be created and inspected without TUI.
- TUI owns view state but not canonical task lifecycle state.
- Execution surface and loaded scope are bound to runtime session state.
- TUI embedded behavior remains compatible with current usage.
- Snapshot APIs exist for later daemon attach mode.
- No runtime module depends on Ratatui, crossterm, or TUI tab types.
