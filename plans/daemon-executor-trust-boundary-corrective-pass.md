# Daemon Executor Trust Boundary Corrective Pass

## Objective

Correct the remaining trust-boundary and lifecycle issues introduced by the initial daemon/frontend separation implementation. The current repo has the right structural pieces: runtime lifecycle fixes, daemon host hardening, a runtime/enforcement bridge, and an `EggsecRuntimeExecutor`. The next pass should make those pieces security-correct and production-credible.

The most important issue is that the real daemon executor currently hardcodes all daemon-executed tasks as `CliManual` with `LoadedScope::default_empty()`. That collapses daemon-backed MCP/agent/REST/gRPC execution into manual permissive behavior and defeats the intended strict-surface design. This pass must make daemon execution use the actual session-bound surface and scope.

## Non-negotiable Invariants

- Daemon-backed execution must not imply manual permissive behavior.
- A session created as `McpServer`, `SecurityAgent`, `RestApi`, `GrpcApi`, or `Ci` must evaluate as strict and fail closed.
- A session created as `CliManual` or `TuiManual` must remain first-class human operator mode, with manual warning/confirmation behavior where intended.
- `RuntimeSurface::Unknown` must not execute. It may be resolved to a configured concrete default at session creation, but it must not reach execution.
- The same runtime request that is approved must be the request that is dispatched.
- Terminal runtime events must describe state that is already visible in `snapshot()`.
- Closing a session should preserve final history; it should not delete useful persisted state by default.

## Current Problems to Fix

1. `EggsecRuntimeExecutor::execute()` ignores the request/session surface and instead forces `RuntimeSurface::CliManual`.
2. `EggsecRuntimeExecutor::execute()` ignores session scope and instead forces `LoadedScope::default_empty()`.
3. The runtime executor receives no execution context beyond task ID, request, event sink, and cancel token.
4. Runtime terminal events are emitted before the task record is updated, so the daemon terminal-event persistence worker may persist stale pre-terminal snapshots.
5. Close-session marks the session closed but does not clearly cancel active tasks, and daemon host deletes the persisted session snapshot instead of preserving a final closed snapshot.
6. The real executor obtains an `ApprovedOperation`, logs it, then dispatches through `dispatch_inner(request, progress_tx)`, which does not require the approval token.
7. The real executor ignores cancellation and does not forward worker progress into `RuntimeEventSink::progress()`.
8. Daemon capabilities remain too broad and are not sufficiently tied to executor mode, feature gates, and safe initial support.
9. Persisted-session listing remains too broad for a future multi-client daemon.

## Files and Areas to Inspect

Core runtime:

- `crates/eggsec-runtime/src/runtime.rs`
- `crates/eggsec-runtime/src/session.rs`
- `crates/eggsec-runtime/src/event.rs`
- `crates/eggsec-runtime/src/capabilities.rs`

Daemon:

- `crates/eggsec-daemon/src/main.rs`
- `crates/eggsec-daemon/src/host.rs`
- `crates/eggsec-daemon/src/server.rs`
- `crates/eggsec-daemon/src/protocol.rs`
- `crates/eggsec-daemon/src/client_registry.rs`
- `crates/eggsec-daemon/src/store/`

Runtime bridge and dispatch:

- `crates/eggsec/src/runtime_bridge/surface.rs`
- `crates/eggsec/src/runtime_bridge/descriptor.rs`
- `crates/eggsec/src/runtime_bridge/manual.rs`
- `crates/eggsec/src/runtime_bridge/executor.rs`
- `crates/eggsec/src/dispatch/mod.rs`

Docs/tests:

- `docs/ARCHITECTURE.md`
- `docs/ARCHITECTURE_INVARIANTS.md`
- `docs/CI_ARCHITECTURE_GUARDS.md`
- existing package tests for `eggsec-runtime`, `eggsec-daemon`, and `eggsec`

## Work Item 1: Add Runtime Execution Context

### Desired behavior

The executor must receive enough context to approve a task according to the session that owns it.

Add a context object in `eggsec-runtime`, for example:

```rust
pub struct RuntimeExecutionContext {
    pub session_id: SessionId,
    pub task_id: TaskId,
    pub surface: RuntimeSurface,
    pub scope: Option<SessionScope>,
    pub request: RunRequest,
}
```

Update `RuntimeTaskExecutor::execute()` to accept this context instead of only `(task_id, request, sink, cancel)`, or add a new trait method and migrate implementations cleanly.

Recommended signature:

```rust
fn execute(
    &self,
    ctx: RuntimeExecutionContext,
    sink: RuntimeEventSink,
    cancel: CancellationToken,
) -> Pin<Box<dyn Future<Output = Result<TaskOutcome, RuntimeError>> + Send + 'static>>;
```

The runtime should populate `ctx.surface` and `ctx.scope` from the owning `RuntimeSession`, not from client-submitted request fields. Client-submitted `RunRequest.surface` should either be validated against the session surface or treated as informational/deprecated.

### Session/request surface mismatch

Add validation in `Runtime::submit()`:

- if `request.surface == RuntimeSurface::Unknown`, replace it with the session surface before storing/executing, or reject it;
- if `request.surface != session.surface`, prefer rejecting with a clear `RuntimeError::InvalidSurface` or equivalent;
- do not allow a client to submit `RunRequest.surface = CliManual` into an `McpServer` session and have that influence enforcement.

Preferred model: session surface is authoritative. Store a normalized request with `request.surface = session.surface` before creating the task record.

### Tests

Add runtime tests proving:

- executor receives the session surface, not a spoofed request surface;
- `Unknown` session surface cannot execute unless resolved at session creation;
- a request surface mismatch is normalized or rejected consistently;
- session scope is visible to the executor context.

## Work Item 2: Fix `EggsecRuntimeExecutor` to Use Actual Surface and Scope

### Desired behavior

`EggsecRuntimeExecutor` must approve using `ctx.surface` and the session’s bound scope. It must not hardcode `CliManual` or `LoadedScope::default_empty()`.

### Implementation guidance

Change executor logic from:

```rust
let surface = RuntimeSurface::CliManual;
let loaded_scope = LoadedScope::default_empty();
approve_run_request(surface, policy, loaded_scope, &request, None)
```

to roughly:

```rust
let surface = ctx.surface;
let loaded_scope = loaded_scope_from_session_scope(ctx.scope.as_ref())?;
approve_run_request(surface, policy, loaded_scope, &ctx.request, manual_override)
```

The adapter needs a reliable conversion from `eggsec_runtime::SessionScope` to `crate::config::LoadedScope`. If full scope rules cannot be reconstructed from lightweight session metadata alone, do not pretend they can. Use one of these safer options:

1. Store a serialized/enough `LoadedScope` representation in the runtime session at creation time.
2. Store a scope path/source and have the executor reload the scope by path when present.
3. For explicit-but-unresolved scope metadata, fail closed for strict automated surfaces and require a real loaded scope before execution.

Do not treat `SessionScope { is_explicit: true }` as equivalent to actual scope rules unless the actual allow/exclude rules are available.

### Manual behavior

Manual CLI/TUI can continue to operate with default-empty scope if that is the documented manual-permissive behavior. Automated surfaces must not.

For strict surfaces:

- if scope is missing or cannot be loaded, fail with an enforcement error;
- if target is out of scope or scope provenance is insufficient, fail closed;
- manual overrides must be rejected.

### Tests

Add executor-level tests with a mock dispatch path or bridge-only approval test proving:

- `McpServer` session without explicit scope fails before dispatch;
- `SecurityAgent` session without explicit scope fails before dispatch;
- `RestApi`/`GrpcApi` sessions reject manual override intent;
- `CliManual` session remains manual permissive;
- `TuiManual` session remains manual permissive;
- request-surface spoofing cannot downgrade an `McpServer` session to `CliManual`.

## Work Item 3: Introduce an Approved Execution Bundle

### Desired behavior

The code should make it difficult to approve one operation and dispatch another.

The current executor obtains `ApprovedOperation` and then calls `dispatch_inner(request, progress_tx)`. That is better than no approval, but `dispatch_inner()` does not require the approval token. Add a stronger coupling.

### Implementation options

Preferred first step:

Create a type in `runtime_bridge` such as:

```rust
pub struct ApprovedRunRequest {
    pub approved: ApprovedOperation,
    pub request: RunRequest,
}
```

Add a helper:

```rust
pub fn approve_run_request_bundle(...) -> Result<ApprovedRunRequest, RuntimeBridgeError>
```

Then add a dispatch wrapper:

```rust
pub async fn dispatch_approved_runtime_request(
    approved: ApprovedRunRequest,
    progress_tx: mpsc::Sender<(u64, u64)>,
) -> anyhow::Result<TaskResult>
```

This wrapper should verify, immediately before dispatch, that:

- approved descriptor operation matches the request task kind mapping;
- approved descriptor target matches the request target;
- request surface matches approved surface/profile expectations where applicable.

Longer term, the lower-level dispatch path can be made private or approval-token-gated, but the wrapper is enough for this corrective pass.

### Tests

Add tests that fail if:

- an approved port scan bundle is mutated to endpoint scan before dispatch;
- approved target differs from request target;
- strict/manual surface mismatch occurs.

If direct mutation is prevented by ownership, test the validation helper with constructed mismatches.

## Work Item 4: Move Terminal State Update Before Terminal Event Emission

### Desired behavior

When a client or daemon subscriber receives `TaskCompleted`, `TaskFailed`, `TaskCancelled`, or a timeout cancellation event, `Runtime::snapshot()` should already show the task in the corresponding terminal state.

### Implementation guidance

In `Runtime::submit()` task spawn:

1. Run executor/timeout.
2. Determine final status/outcome/error.
3. Lock runtime state.
4. If task is still non-terminal, write final status/outcome/error, clear abort/handle, increment generation.
5. Drop lock.
6. Emit the terminal event.

For cancellation from `cancel()` and `cancel_active()`, the state is already updated before emitting. Keep that ordering.

For timeout, ensure `cancel_for_spawn.cancel()` happens before or during terminalization, but the terminal event should still be emitted after the record is marked `TimedOut`.

### Tests

Add a test subscriber that receives `TaskCompleted`, immediately calls `snapshot()`, and asserts the completed task appears in `completed_tasks` with outcome present.

Add equivalent tests for failure and timeout if practical.

## Work Item 5: Fix Terminal Persistence Worker Race

### Desired behavior

The daemon terminal-event persistence task should persist final terminal snapshots reliably.

After Work Item 4, the existing event-driven persistence should work better. Still add defensive validation:

- on terminal event, snapshot session;
- verify the referenced task appears terminal in the snapshot;
- if not, retry once or twice with short backoff and warn if still inconsistent.

This is defensive only; the runtime ordering fix is the primary solution.

### Tests

Add daemon-host or integration tests proving:

- completion persists completed task status/outcome;
- failed task persists failed status/error;
- cancellation persists cancelled status;
- no extra command is required after terminal event.

## Work Item 6: Fix Close-session Semantics and Persistence

### Desired behavior

Closing a session should produce a final closed session state, preserve useful history, and reject future task submission.

### Runtime changes

Update `Runtime::close_session()` to:

- cancel and terminalize active tasks with reason `session closed`;
- mark session closed;
- increment generation;
- emit cancellation events for any active tasks after state update;
- emit `SessionClosed` after state update.

Do not return `SessionNotFound` for an already-closed live session unless this is already a deliberate invariant. Prefer a dedicated `SessionClosed` runtime error variant for submit attempts and idempotent close behavior if reasonable.

### Daemon changes

Change daemon `CloseSession` handling to persist the final closed snapshot instead of deleting the snapshot. Deleting should be a separate explicit command later, not close semantics.

Normal `ListSessions` can continue to omit closed sessions if desired, but persisted session listing should be able to show closed sessions for history.

### Tests

- close with active task terminalizes it as cancelled;
- close persists final closed snapshot;
- submit after close fails;
- snapshot after close works for live runtime if closed sessions are retained;
- persisted snapshot after close is available and marked closed.

## Work Item 7: Honor Cancellation in Real Executor

### Desired behavior

Daemon cancellation should affect real executor tasks where possible.

### Implementation guidance

At minimum:

- check `cancel.is_cancelled()` before approval;
- wrap approval+dispatch in `tokio::select!` against `cancel.cancelled()`;
- when cancelled, return a cancellation runtime error or a structured cancellation result that runtime maps to `Cancelled` without overwriting terminal status.

If individual dispatch functions do not support cancellation, the adapter-level `select!` should stop waiting and let runtime cancellation terminalization win. Longer-term tool-level cancellation can be added separately.

### Tests

Use a controlled long-running task/executor if real dispatch is hard to cancel deterministically. Assert that:

- cancellation does not become completed later;
- runtime stale-completion guard preserves cancelled state;
- executor observes cancellation before dispatch when already cancelled.

## Work Item 8: Forward Dispatch Progress to Runtime Events

### Desired behavior

Real daemon tasks should emit progress events visible to subscribed frontends.

### Implementation guidance

`dispatch_inner()` currently receives an `mpsc::Sender<(u64, u64)>`. The real executor creates `(progress_tx, _progress_rx)` and drops the receiver. Instead:

- keep `progress_rx`;
- spawn or select a progress-forwarding loop;
- call `sink.progress(completed, Some(total), None)` for each update;
- ensure progress forwarding stops at terminal completion/cancellation.

A simple structure:

```rust
let (progress_tx, mut progress_rx) = mpsc::channel(16);
let progress_sink = sink.clone_or_recreate(); // if sink is not cloneable, add Clone or use a helper
let progress_task = tokio::spawn(async move {
    while let Some((done, total)) = progress_rx.recv().await {
        progress_sink.progress(done, Some(total), None);
    }
});
```

If `RuntimeEventSink` is not cloneable, either implement `Clone` or forward progress in the same `select!` loop around dispatch.

### Tests

Add a dispatch/executor test using a fake progress sender or a small task that emits progress. Subscribe to runtime events and assert `TaskProgress` appears before terminal event.

## Work Item 9: Tighten Daemon Capabilities

### Desired behavior

Daemon capabilities should accurately report executor mode and supported task kinds. Do not advertise broad hazardous or feature-gated task kinds unless they are actually enabled and intentionally supported by daemon execution.

### Implementation guidance

Extend capabilities with executor mode if not already present, or encode clearly in existing fields:

- noop/protocol-only: no executable task kinds;
- full executor: only supported initial subset;
- future lab-full executor: hazardous/lab features gated explicitly.

Recommended initial full-executor supported set:

- `port-scan`
- `endpoint-scan`
- `fingerprint`
- `waf`
- `pipeline`
- `recon`
- optionally `load-test` and `fuzz` if enforcement and rate controls are clear

Do not advertise by default:

- `packet-send`
- `wireless-active`
- `db-pentest`
- `intercept`
- `c2`
- dynamic mobile/runtime instrumentation
- stress tests unless explicit lab/full mode is selected

Also ensure dispatcher/descriptor returns `UnsupportedTaskKind` when a task is not in the daemon-supported subset for the current mode, even if it has operation metadata.

### Tests

- noop capabilities list no executable tasks;
- normal full executor capabilities list conservative supported subset;
- unsupported/hazardous task submission returns typed unsupported error;
- explicit lab mode, if implemented, expands capabilities only when configured.

## Work Item 10: Persisted-session Access Control Follow-up

### Desired behavior

Global persisted-session listing should not become a multi-client data leak.

### Implementation guidance

For this pass, choose one of:

1. Require an admin/internal client role for `ListPersistedSessions`.
2. Return only persisted sessions owned by the requesting client when ownership metadata is available.
3. Add a config flag `allow_global_persisted_session_reads` defaulting to true only for single-user/local mode, but document it as transitional.

Preferred: implement owner-filtered listing where possible and admin-only global listing later.

Persist enough owner metadata in session snapshots or a sidecar access table so recovered sessions do not become globally readable by default.

### Tests

- owner can read/list own persisted session;
- unrelated declared client cannot read/list another live/persisted session;
- recovered sessions retain access metadata if persisted;
- internal/admin client, if present, can list all.

## Work Item 11: Documentation and Invariant Tests

Update docs to reflect the corrected daemon semantics.

Docs to update:

- `docs/ARCHITECTURE.md`
- `docs/ARCHITECTURE_INVARIANTS.md`
- `docs/FEATURE_MATRIX.md`
- `architecture/daemon.md`
- `architecture/domain_contract.md` if needed

Key wording:

- The daemon is a runtime host, not an enforcement profile.
- Session surface is authoritative for enforcement.
- Human CLI/TUI surfaces preserve manual ergonomics, embedded or daemon-backed.
- Automated surfaces are strict, embedded or daemon-backed.
- `Unknown` may be resolved at session creation but cannot execute.
- Real daemon executor must use session surface/scope.

Add or extend architecture guard scripts so future changes cannot reintroduce hardcoded `RuntimeSurface::CliManual` in `EggsecRuntimeExecutor` except in tests or explicit manual-only helpers.

## Validation Commands

Run targeted checks first:

```bash
cargo test -p eggsec-runtime
cargo test -p eggsec-daemon
cargo test -p eggsec --lib runtime_bridge
cargo check -p eggsec-daemon --all-targets
cargo check -p eggsec-daemon --features full-executor --all-targets
```

Then run broader validation if feasible:

```bash
cargo test -p eggsec --lib
cargo check --workspace --all-targets
./scripts/check-architecture-guards.sh
```

If full workspace validation is too heavy locally, record the exact targeted commands that passed and any known unrelated blockers.

## Acceptance Checklist

- [ ] Real daemon executor uses session surface, not hardcoded `CliManual`.
- [ ] Real daemon executor uses session scope or fails closed when strict scope is unavailable.
- [ ] `RuntimeSurface::Unknown` cannot execute.
- [ ] Request surface spoofing cannot downgrade strict sessions.
- [ ] Strict daemon sessions fail closed without explicit valid scope.
- [ ] Manual daemon CLI/TUI sessions remain manual permissive/guarded as configured.
- [ ] Approved request and dispatched request are coupled by an approved bundle or equivalent validation.
- [ ] Terminal events are emitted after runtime state is terminal.
- [ ] Daemon terminal-event persistence captures terminal snapshots reliably.
- [ ] Close-session cancels active tasks, marks closed, rejects new tasks, and preserves final persisted history.
- [ ] Real executor honors cancellation at adapter level.
- [ ] Real executor forwards progress into runtime events.
- [ ] Capabilities accurately reflect noop vs real executor and conservative supported task set.
- [ ] Persisted-session listing/access is tightened or explicitly config-gated.
- [ ] Tests cover strict daemon sessions, manual daemon sessions, spoofing, persistence, close semantics, cancellation, progress, and capabilities.

## Handoff Notes

Keep this pass corrective and security-focused. Do not expand transports, frontend SDKs, or new feature families until the daemon executor trust boundary is fixed. The most valuable outcome is making daemon-backed execution safe enough that future CLI/TUI dual-backend work can build on it without inheriting policy-bypass risk.
