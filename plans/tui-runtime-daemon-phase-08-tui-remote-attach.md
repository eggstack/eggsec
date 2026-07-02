# Phase 8 Plan: TUI Remote Attach Mode

## Goal

Allow `eggsec-tui` to run against either the embedded in-process runtime or a local daemon runtime. Embedded mode should remain the default. Remote attach mode should use the same runtime protocol and event model as the local daemon MVP from Phase 7.

This phase turns the TUI into a true frontend over a runtime boundary rather than a terminal application that only happens to embed a runtime.

## Current Baseline

After Phase 7, there should be:

- A local-only daemon hosting `eggsec-runtime`.
- A protocol for session creation, session listing, snapshots, task submission, cancellation, capabilities, health, and event subscription.
- Runtime events and `TaskResultEnvelope` outcomes sufficient for basic non-TUI clients.
- Embedded TUI behavior preserved.

Phase 8 should not change engine execution. It adds a second runtime backend for the TUI.

## Runtime Client Abstraction

Introduce a TUI-side runtime client trait that hides embedded vs daemon transport differences.

Suggested module:

```text
crates/eggsec-tui/src/runtime_client/
  mod.rs
  embedded.rs
  daemon.rs
```

Suggested trait:

```rust
pub trait TuiRuntimeClient: Send + Sync {
    fn capabilities(&self) -> RuntimeClientFuture<RuntimeCapabilities>;
    fn create_session(&self, options: CreateSessionOptions) -> RuntimeClientFuture<SessionId>;
    fn list_sessions(&self) -> RuntimeClientFuture<Vec<SessionSummary>>;
    fn snapshot(&self, session_id: SessionId) -> RuntimeClientFuture<SessionSnapshot>;
    fn submit(&self, session_id: SessionId, request: RunRequest) -> RuntimeClientFuture<TaskId>;
    fn cancel(&self, session_id: SessionId, task_id: TaskId) -> RuntimeClientFuture<()>;
    fn cancel_active(&self, session_id: SessionId) -> RuntimeClientFuture<()>;
    fn subscribe(&self, session_id: SessionId) -> RuntimeClientFuture<RuntimeEventReceiverLike>;
}
```

If async trait usage is already common, use `async_trait`; otherwise use boxed futures to avoid adding unnecessary dependencies.

## Embedded Client

`EmbeddedRuntimeClient` should wrap the existing `Arc<Runtime>` path and preserve current TUI behavior.

Requirements:

- Uses existing `RuntimeBinding` or replaces it cleanly.
- Does not change default launch behavior.
- Preserves typed result channel rendering path for embedded mode if still required.
- Produces the same event stream shape as daemon client.

## Daemon Client

`DaemonRuntimeClient` should talk to the local daemon protocol from Phase 7.

Requirements:

- Connect to configured socket path.
- Send command/response messages with request IDs.
- Provide event subscription stream to TUI.
- Reconnect behavior can be minimal in this phase: show a clear error and allow manual reconnect.
- Snapshot hydration must work on attach.

Do not implement internet-facing remote access. Local daemon only.

## TUI Launch Modes

Add configuration/CLI support for runtime mode.

Potential user-facing options:

```text
eggsec tui --runtime embedded
eggsec tui --runtime daemon --socket <path>
eggsec --tui-runtime daemon --socket <path>
```

Keep default behavior:

```text
eggsec
```

when no command and terminal stdout should still launch embedded TUI unless the user config explicitly says daemon.

## Session Attach UX

Add minimal attach flow.

Behavior:

1. TUI connects to daemon.
2. TUI requests `ListSessions`.
3. If no sessions exist, TUI creates a new manual session with explicit surface.
4. If sessions exist, TUI can either attach to the most recent session or show a basic session picker.
5. TUI calls `GetSnapshot` and hydrates view state.
6. TUI subscribes to runtime events.

For MVP, a simple option is acceptable:

```text
--session <session-id>
--new-session
--attach-latest
```

Interactive session picker can be deferred if it grows too large.

## Snapshot Hydration

Use the Phase 5 snapshot hydration work.

Minimum hydration:

- session surface
- scope metadata
- active tasks
- completed task summaries
- capabilities
- latest known progress/status

Do not attempt full tab-rich rendering from historical typed results unless the envelope payload supports it. Show summaries in history/dashboard if rich tab reconstruction is not available.

## Event Routing

`TuiRuntimeAdapter` should continue to be the only TUI event reducer.

Changes needed:

- It must accept events from either embedded or daemon client.
- It must handle events for tasks that were already active before attach.
- It must not require a local typed result channel for lifecycle state.
- It should display envelope summaries when typed result data is unavailable.

## Typed Result Channel Behavior

Embedded mode may keep typed channels for rich rendering.

Daemon mode cannot depend on typed in-process channels. It must render from `TaskResultEnvelope` and snapshots.

Acceptance target:

- Embedded TUI retains rich existing rendering.
- Daemon-attached TUI displays meaningful summaries and lifecycle state even without typed results.
- Docs clearly state which tabs have full remote rendering and which have summary-level remote rendering.

## Files Likely to Change

- `crates/eggsec-tui/src/app/mod.rs`
- `crates/eggsec-tui/src/app/runner.rs`
- `crates/eggsec-tui/src/app/runtime_adapter/mod.rs`
- `crates/eggsec-tui/src/app/task_runtime.rs`
- `crates/eggsec-tui/src/runtime_client/*`
- `crates/eggsec-tui/src/tabs/dashboard.rs`
- `crates/eggsec-tui/src/tabs/history.rs`
- `crates/eggsec-cli/src/main.rs`
- `crates/eggsec-daemon/src/protocol.rs`
- `architecture/tui.md`

## Implementation Steps

1. Add `TuiRuntimeClient` abstraction.
2. Implement embedded client over current runtime.
3. Implement daemon client over local daemon protocol.
4. Replace direct `RuntimeBinding` usage with a runtime-client binding where possible.
5. Add TUI runtime mode configuration.
6. Add session attach options: new, latest, explicit session ID.
7. Add snapshot hydration for daemon attach.
8. Update `TuiRuntimeAdapter` to handle pre-existing active/completed task records.
9. Add remote-mode rendering fallback for envelope summaries.
10. Preserve embedded mode as default and test it.
11. Add docs and architecture guard notes.

## Tests

Unit tests:

- Embedded client implements expected calls.
- Daemon client serializes expected commands.
- Snapshot hydration from daemon snapshot updates TUI view state.
- Runtime adapter handles unknown/pre-existing tasks without panic.
- Envelope-only completion updates summary/history view.

Integration tests where practical:

- Start local daemon in test.
- Launch a daemon client against it.
- Create session and submit test task.
- Attach TUI client state to existing session snapshot.
- Receive runtime event and update view state.

## Non-Goals

Do not expose daemon over public network.

Do not implement full reconnect/replay semantics.

Do not require every tab to have full rich remote rendering.

Do not remove embedded mode.

Do not make daemon mode default.

## Validation

Run:

```bash
cargo fmt --all --check
cargo check -p eggsec-runtime
cargo test -p eggsec-runtime
cargo check -p eggsec-daemon
cargo test -p eggsec-daemon
cargo check -p eggsec-tui
cargo test -p eggsec-tui
cargo check -p eggsec-cli
./scripts/check-architecture-guards.sh
```

Manual smoke checks:

```text
1. Launch embedded TUI as before.
2. Start daemon locally.
3. Launch TUI in daemon attach mode.
4. Create or attach session.
5. Submit a safe local task.
6. Confirm event/progress/completion render.
7. Cancel a running task.
8. Restart TUI and attach to existing daemon session snapshot.
```

## Acceptance Criteria

- TUI supports embedded runtime and daemon runtime modes.
- Embedded mode remains default and behavior-compatible.
- Daemon attach can create/list/attach sessions.
- Daemon attach can submit/cancel tasks and receive runtime events.
- Snapshot hydration shows active/completed task state.
- Envelope-only results are rendered meaningfully in remote mode.
- No daemon transport dependencies enter `eggsec-runtime`.
