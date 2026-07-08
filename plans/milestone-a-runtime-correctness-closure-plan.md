# Milestone A: Runtime Correctness Closure Plan

## Objective

Close correctness gaps in `eggsec-runtime` before the daemon becomes a real execution backend. This milestone should make runtime task lifecycle behavior deterministic, auditable, and suitable for daemon-backed frontends.

The runtime is intended to be frontend-neutral infrastructure. Bugs here will affect CLI daemon mode, TUI daemon mode, future desktop/mobile clients, and strict programmatic execution. Keep this milestone focused on lifecycle semantics, not on wiring real Eggsec scan execution.

## Current Findings

The current runtime exposes `SessionOptions.task_timeout`, but `Runtime::create_session()` and `Runtime::create_session_with_scope()` ignore the options argument, and `Runtime::submit()` uses only `RuntimeConfig.default_task_timeout`. This creates an API/behavior mismatch.

Single-active-task replacement currently removes prior active task records after cancelling them. That emits a cancellation event but loses the terminal task record from normal session history. A still-running spawned executor can later finish and attempt to write final state, but the record is gone. This prevents overwrite by accident, not by explicit stale-state protection.

`RuntimeSession::snapshot()` currently sets `created_at_secs` using elapsed monotonic seconds, while session summaries use epoch-style creation seconds. The field name is ambiguous and poor for persisted snapshots.

Runtime event sends use `let _ =` broadly. This is probably fine for ordinary no-subscriber UI events, but the behavior should be explicit and observable for unexpected channel failures.

## Files and Areas to Inspect

- `crates/eggsec-runtime/src/runtime.rs`
- `crates/eggsec-runtime/src/session.rs`
- `crates/eggsec-runtime/src/event.rs`
- `crates/eggsec-runtime/src/error.rs`
- `crates/eggsec-runtime/src/lib.rs`
- Any runtime unit tests in the above files
- Any daemon persistence code that consumes `SessionSnapshot`, especially `crates/eggsec-daemon/src/store/` and `crates/eggsec-daemon/src/host.rs`

## Work Item A1: Honor Per-session Timeout Options

### Desired behavior

`SessionOptions.task_timeout` should override `RuntimeConfig.default_task_timeout` for tasks submitted to that session. If the session override is `None`, the runtime should use the global default. If both are `None`, no timeout applies.

### Implementation guidance

Add session-level options to runtime state. Prefer storing the resolved or original options in `RuntimeSession` so the session snapshot model can later expose useful metadata if needed. A lightweight field such as `task_timeout: Option<Duration>` or `options: SessionOptions` is acceptable, but keep serializable snapshots separate unless Duration serialization is intentionally designed.

Update:

- `RuntimeSession::new(...)`
- `RuntimeSession::with_scope(...)`
- `Runtime::create_session(...)`
- `Runtime::create_session_with_scope(...)`
- `Runtime::submit(...)`

During submit, compute timeout by looking up the session first:

```text
let timeout_duration = session.task_timeout_override().or(state.config.default_task_timeout)
```

Do not silently ignore `SessionOptions` anymore. Remove underscore-prefixed `_options` usage.

### Tests

Add tests covering:

- session with shorter timeout than runtime default times out according to session setting;
- session with no override uses runtime default;
- two sessions in the same runtime can use different timeout behavior;
- session timeout of `None` can disable a global timeout if that behavior is intentionally chosen. If `None` means “no override,” add a richer options type later rather than overloading `None`.

Important: decide and document whether `SessionOptions.task_timeout = None` means “no override” or “explicitly no timeout.” The current type cannot distinguish these. The simplest first pass is “None means no override.” If explicit timeout disabling is needed, introduce an enum or additional boolean later.

## Work Item A2: Preserve Cancelled/Replaced Task History

### Desired behavior

When a new task replaces an existing active task under the single-active-task policy, the old task should remain in session history as terminal `Cancelled`. It should not be removed from the task map before the session snapshot can preserve it.

### Implementation guidance

In `Runtime::submit()`, replace the current remove-and-cancel logic with in-place terminal transition:

- collect active task IDs;
- for each active task, set `TaskStatus::Cancelled`;
- take and cancel the abort token;
- clear the join handle if appropriate, or keep it only as an internal non-snapshot handle until completion;
- set `last_error` or cancellation reason if the model supports it;
- emit `TaskCancelled` event;
- increment session generation.

Avoid deleting task records as part of replacement. If unbounded task history is a concern, add a separate retention policy later; do not solve it by dropping terminal state in this milestone.

### Tests

Add or update tests covering:

- `cancel_replaces_existing_task` should assert that task 1 is present in completed tasks with `Cancelled`, and task 2 is active/running or completed depending on executor timing;
- replacing multiple active tasks, if possible, terminalizes all previous active tasks;
- cancelled task snapshots preserve task kind, request summary, and last error/reason where available.

## Work Item A3: Add Stale Completion Guards

### Desired behavior

A task that has been cancelled, timed out, or superseded must not later overwrite its terminal status when its spawned executor future returns.

### Implementation guidance

Add an explicit guard before writing final status in the spawned task. Options:

1. Check the current task status and only write final status if it is still non-terminal.
2. Add a task generation/epoch to `TaskRecord` and capture it at spawn; only write if the captured generation matches.
3. Combine both: generation protects replacement semantics and terminal-status check protects cancellation.

The minimal safe implementation is:

```text
if let Some(task) = session.tasks.get_mut(&task_id) {
    if task.status.is_terminal() {
        return;
    }
    task.status = final_status;
    ...
}
```

However, if task IDs are unique and records are retained, a terminal-status guard may be enough for now. A generation guard is more robust if future behavior allows task record reuse, which it currently should not.

Also ensure timeout behavior cancels the token and marks the final status as `TimedOut`, and that a later executor return does not turn it into `Completed`.

### Tests

Add a controlled executor that waits on a channel or cancellation token and returns after the runtime has already cancelled the task. Assert that the final snapshot remains `Cancelled` or `TimedOut`, not `Completed`.

## Work Item A4: Clarify Snapshot Timestamp Semantics

### Desired behavior

Snapshot timestamps should be unambiguous and useful after persistence/recovery.

### Implementation options

Preferred:

- Rename `SessionSnapshot.created_at_secs` to `created_at_epoch_secs`.
- Add `age_secs` only if frontends need age directly.
- Keep `SessionSummary.created_at_secs` or rename it consistently to `created_at_epoch_secs`.

If renaming is too disruptive for this pass, add a new field and deprecate the old one internally, then migrate daemon persistence tests.

### Files likely affected

- `crates/eggsec-runtime/src/session.rs`
- `crates/eggsec-daemon/src/store/*`
- `crates/eggsec-daemon/src/protocol.rs` tests
- Any tests constructing `SessionSnapshot` manually

### Tests

Add roundtrip tests proving:

- snapshot creation time is epoch-style and stable over repeated snapshots;
- age, if exposed, increases or is computed separately;
- persisted and hydrated snapshots do not reinterpret age as creation time.

## Work Item A5: Make Event-send Failure Semantics Explicit

### Desired behavior

Best-effort UI event delivery should not crash the runtime when there are no subscribers. Unexpected failures should be observable in tracing or return values where appropriate.

### Implementation guidance

Add a small helper for runtime event emission, possibly on `RuntimeEventSink` and within `Runtime`:

```text
fn emit_event(tx: &broadcast::Sender<RuntimeEvent>, event: RuntimeEvent, criticality: EventCriticality)
```

For normal events, log at trace/debug if there are no subscribers. For policy/audit-critical events, consider warning on send failure or routing through a more reliable persistence/audit path in later milestones.

Do not over-engineer reliable event delivery in this milestone. The goal is to remove ambiguous silent suppression and make the expected behavior explicit.

### Tests

Add tests only where practical. Broadcast send without receivers returns an error in some cases; tests should assert the helper handles this without panicking.

## Validation Commands

Run at minimum:

```bash
cargo test -p eggsec-runtime
cargo test -p eggsec-daemon
cargo check --workspace --all-targets
```

If workspace checks are too heavy locally, run the targeted package tests first and note any unrelated failures.

## Acceptance Checklist

- [ ] `SessionOptions.task_timeout` is no longer ignored.
- [ ] Replacement cancellation preserves terminal history.
- [ ] Stale executor completion cannot overwrite terminal cancellation/timeout.
- [ ] Snapshot timestamp semantics are unambiguous.
- [ ] Event-send error behavior is documented in code and at least minimally observable.
- [ ] Runtime unit tests cover the changed lifecycle behavior.
- [ ] Daemon tests still compile after snapshot field changes.

## Handoff Notes

Keep this milestone dependency-light. Do not wire real Eggsec scan execution yet. The outcome should be a more reliable runtime substrate for daemon and frontend work.
