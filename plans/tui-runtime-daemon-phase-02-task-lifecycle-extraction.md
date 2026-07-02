# Phase 2 Plan: Task Lifecycle Extraction

## Goal

Move task lifecycle ownership out of `eggsec-tui::App` and into the new frontend-neutral runtime layer introduced in Phase 1. This phase should preserve current embedded TUI behavior while changing ownership of task IDs, task handles, timeout policy, progress/result channels, cancellation, and active-task bookkeeping.

The TUI should become a client of an embedded runtime rather than the owner of task execution mechanics.

## Current Problem

`eggsec-tui` currently owns task lifecycle in `App::spawn_task` and related `TaskState` handling. It creates progress/result channels, constructs a `TaskRunner`, spawns Tokio tasks, stores abort handles, applies a hard 300-second timeout, receives progress/result values, and clears runtime state on completion.

This blocks daemonization because the runtime cannot exist independently of terminal UI state. It also prevents multiple frontend clients from observing or controlling the same task lifecycle.

## Desired End State for This Phase

A runtime object owns active tasks. The TUI calls runtime submission/cancellation APIs and consumes runtime events. The TUI may still translate its existing tab inputs into the current TUI `TaskConfig` shape temporarily, but lifecycle mechanics should move out of `App`.

The existing TUI should still support the same single-active-task behavior unless multi-task support is explicitly added later. Preserve the current UX before expanding semantics.

## Runtime API to Add

Add an embedded runtime type in `eggsec-runtime`, for example:

```rust
pub struct Runtime {
    // internal sessions/tasks/event senders
}

impl Runtime {
    pub fn new(config: RuntimeConfig) -> Self;
    pub fn create_session(&self, options: SessionOptions) -> Result<SessionId>;
    pub fn submit(&self, session_id: SessionId, request: RunRequest) -> Result<TaskId>;
    pub fn cancel(&self, session_id: SessionId, task_id: TaskId) -> Result<()>;
    pub fn snapshot(&self, session_id: SessionId) -> Result<SessionSnapshot>;
    pub fn subscribe(&self, session_id: SessionId) -> Result<RuntimeEventReceiver>;
}
```

Use Tokio channels internally. Keep the first implementation simple. It may support one session and one active task if that matches current TUI behavior, but the public API should not prevent multiple sessions later.

## Compatibility Strategy

This phase can bridge to existing TUI worker execution rather than immediately moving every worker. There are two acceptable approaches:

1. **Temporary compatibility adapter:** `eggsec-tui` submits a runtime request, but the runtime calls a boxed executor supplied by the TUI. This moves lifecycle ownership first while worker dispatch remains TUI-local until Phase 3.
2. **Early internal executor:** Move enough of `TaskRunner` into runtime now to avoid an adapter. This is more invasive and may be better deferred to Phase 3.

Prefer option 1 if implementation risk is high. The goal of Phase 2 is lifecycle extraction, not full worker migration.

A compatibility trait could be:

```rust
#[async_trait]
pub trait RuntimeTaskExecutor: Send + Sync + 'static {
    async fn execute(
        &self,
        task_id: TaskId,
        request: RunRequest,
        events: RuntimeEventSink,
        cancel: CancellationToken,
    ) -> Result<TaskOutcome, RuntimeError>;
}
```

In Phase 2, the TUI can provide an executor that calls the existing worker path. In Phase 3, that executor moves into runtime/engine.

## Task Timeout Policy

The current TUI hard-codes a 300-second timeout. Move this into `RuntimeConfig`:

```rust
pub struct RuntimeConfig {
    pub default_task_timeout: Option<Duration>,
    pub max_active_tasks_per_session: usize,
}
```

Preserve the current 300-second default for embedded TUI compatibility unless existing config says otherwise. Emit a `TaskFailed` or `TaskCancelled`/`TimedOut` event with a clear reason on timeout.

## Cancellation Model

Use `tokio_util::sync::CancellationToken` if acceptable, or Tokio abort handles as an implementation detail. The runtime API should expose cancellation by `SessionId` and `TaskId`; frontends should not hold raw task handles.

Current `Ctrl-C` TUI stop behavior should call runtime cancellation. Current tab state should be updated from runtime events.

## Event Delivery

When a task is submitted, runtime should emit:

- `TaskQueued`
- `TaskStarted`
- progress events as available
- exactly one terminal event: `TaskCompleted`, `TaskFailed`, or `TaskCancelled`

If using a compatibility executor, the executor may translate legacy progress/result channels into runtime events.

## TUI Changes

Update `App` and `TaskState` so that:

- `TaskState` stores `Option<TaskId>`, initiating tab, started time, and subscription state.
- `TaskState` no longer stores raw `JoinHandle`s or abort handles.
- `App::spawn_task` becomes `App::submit_task_to_runtime` or a wrapper around runtime submission.
- `App::stop` calls runtime cancellation.
- `App::update` drains runtime events rather than directly draining task progress/result receivers where possible.

If full event reducer work is deferred to Phase 4, this phase may translate runtime events back into the old result/progress path as a temporary bridge. Keep the bridge small and documented.

## Files Likely to Change

Runtime side:

- `crates/eggsec-runtime/Cargo.toml`
- `crates/eggsec-runtime/src/lib.rs`
- `crates/eggsec-runtime/src/runtime.rs`
- `crates/eggsec-runtime/src/session.rs`
- `crates/eggsec-runtime/src/event.rs`
- `crates/eggsec-runtime/src/error.rs`

TUI side:

- `crates/eggsec-tui/Cargo.toml`
- `crates/eggsec-tui/src/app/mod.rs`
- `crates/eggsec-tui/src/app/state.rs`
- `crates/eggsec-tui/src/app/task_runtime.rs`
- `crates/eggsec-tui/src/app/state_update.rs`
- `crates/eggsec-tui/src/workers/runner.rs`

## Non-goals

Do not add daemon transport.

Do not implement multi-client fanout beyond what is needed for embedded TUI.

Do not fully migrate all worker dispatch unless the compatibility adapter makes that easy.

Do not change the manual-vs-agent enforcement semantics.

Do not change the visible TUI behavior intentionally.

## Implementation Steps

1. Add `Runtime`, `RuntimeConfig`, `SessionOptions`, event sink/receiver types, and in-memory session/task maps to `eggsec-runtime`.
2. Add task submission and cancellation APIs.
3. Add a task executor trait or equivalent compatibility seam.
4. Implement a single-active-task policy matching current TUI behavior.
5. Move timeout handling into runtime.
6. Add unit tests for task submission, start event, progress event, completion event, failed task event, timeout, and cancellation.
7. Add `eggsec-runtime` dependency to `eggsec-tui`.
8. Add a TUI compatibility executor that wraps existing worker execution if needed.
9. Change `App::spawn_task` to submit to runtime and store `TaskId` instead of task handles.
10. Change `App::stop`/`stop_with_message` to call runtime cancellation and then update the appropriate tab state from event/result handling.
11. Keep temporary bridging code clearly named and documented for Phase 3/4 removal.

## Validation

Run:

```bash
cargo check -p eggsec-runtime
cargo test -p eggsec-runtime
cargo check -p eggsec-tui
cargo test -p eggsec-tui
cargo check -p eggsec-cli
```

Feature smoke checks:

```bash
cargo check -p eggsec-tui --features stress-testing,packet-inspection
cargo check -p eggsec-cli --features rest-api
```

Manual smoke checks if possible:

- Launch TUI.
- Start a simple recon or port scan task.
- Confirm progress updates.
- Switch tabs during task execution.
- Cancel a task.
- Confirm task timeout behavior remains sensible.

## Acceptance Criteria

- Runtime owns task handles, timeout, and cancellation.
- TUI no longer stores raw task `JoinHandle`s or abort handles.
- TUI task launch/cancel behavior remains user-compatible.
- Runtime emits canonical task lifecycle events.
- Tests cover lifecycle behavior without constructing Ratatui/crossterm terminal state.
