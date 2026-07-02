# Phase 10 Plan: Multi-Session and Multi-Frontend Semantics

## Goal

Define and implement the first coherent multi-session and multi-frontend semantics for Eggsec daemon mode. After this phase, multiple clients should be able to observe or attach to sessions without corrupting runtime state, confusing policy decisions, or depending on TUI-local assumptions.

This phase should still remain local-daemon-oriented. It is about semantics and correctness, not broad network exposure.

## Current Baseline

By this point the repo should have:

- Runtime sessions and snapshots.
- Local daemon MVP.
- TUI remote attach mode.
- CLI daemon client commands.
- Runtime event subscription.
- Protocol-neutral result envelopes.

The next risk is not basic functionality. The risk is ambiguous ownership: who may create sessions, who may submit tasks, who may cancel tasks, who may approve policy prompts, and how many clients may observe the same session.

## Core Model

Introduce clear concepts:

- **Session**: runtime-owned unit of scope, surface, task history, capabilities, and audit trail.
- **Client**: a connected daemon user/frontend with an ID, kind, permissions, and connection metadata.
- **Frontend**: TUI, CLI, future web/desktop/mobile, MCP/agent, or REST/gRPC client.
- **Observer**: client subscribed to events but not allowed to mutate session state.
- **Controller**: client allowed to submit/cancel tasks for a session.
- **Approver**: manual client allowed to answer policy prompts for manual surfaces.

Do not infer permissions only from transport. Local socket does not mean every client has TUI-manual authority.

## Session Ownership and Access

Add session metadata:

```rust
pub struct SessionAccess {
    pub owner_client_id: Option<ClientId>,
    pub allowed_clients: Vec<ClientAccessRule>,
    pub default_observer_allowed: bool,
    pub default_controller_allowed: bool,
}
```

For the first implementation, keep policy simple:

- The client that creates a session is the owner/controller.
- Additional clients may attach as observers by default.
- Mutating operations from non-owner clients require explicit permission or local manual configuration.
- Strict/agent sessions should not accept manual approvals from unrelated TUI clients.

## Client Identity

Add a daemon-side client registry.

Minimum client metadata:

```rust
pub struct ClientInfo {
    pub client_id: ClientId,
    pub kind: ClientKind,
    pub surface: RuntimeSurface,
    pub connected_at_secs: u64,
    pub label: Option<String>,
}
```

Suggested client kinds:

- `Cli`
- `Tui`
- `DaemonInternal`
- `Mcp`
- `Rest`
- `Agent`
- `Unknown`

A client should declare kind on connect or first command. The daemon resolves effective permissions.

## Event Fanout

Support multiple subscribers per session.

Requirements:

- Runtime events are broadcast to all subscribed clients for that session.
- Slow clients cannot block runtime or other clients.
- Dropped clients are removed from subscription registry.
- Event ordering is preserved per client as much as the transport allows.
- Event messages include session ID and task ID where relevant.

If runtime already uses a broadcast channel, daemon should filter per session and fan out. If runtime events are global, keep that internal; do not expose cross-session leakage to clients.

## Task Mutation Rules

Define command permissions.

Suggested rules:

- `Health`, `Capabilities`: any local client.
- `ListSessions`: any local client, but may redact details for strict/agent sessions later.
- `GetSnapshot`: observers and controllers.
- `Subscribe`: observers and controllers.
- `SubmitTask`: controllers only.
- `CancelTask`/`CancelActive`: controllers only.
- `ApprovePolicy`: approvers only.
- `CloseSession`: owner/controller only.

Add structured error codes:

```text
permission-denied
session-not-found
task-not-found
invalid-surface
policy-approval-not-allowed
client-not-attached
```

## Policy Prompt Semantics

This is the most important safety portion of the phase.

Rules:

- Programmatic strict surfaces should not delegate policy approvals to arbitrary manual clients.
- TUI manual sessions may prompt the attached TUI controller for confirmation.
- CLI manual sessions may prompt the CLI controller if interactive, or fail if non-interactive.
- MCP/agent/CI sessions should treat confirmation-required operations according to strict policy rules, not TUI manual override.
- Every approval/denial must be auditable with client ID, surface, session ID, task ID, operation metadata, and reason.

If policy prompt transport is too large for this phase, explicitly reject `PolicyDecisionRequired` over daemon for strict surfaces and support it only for TUI manual sessions.

## Snapshot Consistency

Multiple clients can call snapshot while tasks are running. Snapshots must be internally consistent enough for UI clients.

Minimum:

- Session metadata included.
- Active tasks included with status/progress.
- Completed tasks included with outcome summary/envelope/artifact refs where available.
- Snapshot has monotonic or generation-like field if practical.

Potential addition:

```rust
pub struct SessionSnapshot {
    pub generation: u64,
    ...
}
```

Increment generation on task state changes. This helps clients ignore stale snapshots/events later.

## Result and Artifact Access

Result envelopes are currently lightweight. For multi-frontend support:

- Completed task records should retain `TaskOutcome` or result envelope.
- Snapshot completed task summaries should include outcome summaries.
- Artifact refs should be fetchable through daemon if local file paths are not directly usable by remote frontend.

For this phase, artifact fetch can be stubbed or local-only, but the model should be clear.

## Files Likely to Change

- `crates/eggsec-runtime/src/ids.rs`
- `crates/eggsec-runtime/src/session.rs`
- `crates/eggsec-runtime/src/runtime.rs`
- `crates/eggsec-runtime/src/event.rs`
- `crates/eggsec-daemon/src/client.rs`
- `crates/eggsec-daemon/src/protocol.rs`
- `crates/eggsec-daemon/src/server.rs`
- `crates/eggsec-daemon/src/session_registry.rs` if added
- `crates/eggsec-tui/src/runtime_client/*`
- `crates/eggsec-cli/src/daemon.rs`
- `architecture/overview.md`
- `architecture/tui.md`
- `docs/CI_ARCHITECTURE_GUARDS.md`

## Implementation Steps

1. Add `ClientInfo`, `ClientKind`, and daemon client registry.
2. Add client declaration/handshake to daemon protocol.
3. Add session owner/controller/observer metadata.
4. Add permission checks for daemon commands.
5. Add structured permission error responses.
6. Add multi-subscriber event fanout tests.
7. Add snapshot generation or equivalent consistency marker if feasible.
8. Preserve completed task outcome/envelope in runtime session records if not already stored.
9. Add policy prompt routing rules for TUI manual vs strict surfaces.
10. Update TUI daemon attach to attach as controller or observer explicitly.
11. Update CLI daemon commands to declare client kind and surface.
12. Update docs with session/client permission model.

## Tests

Runtime tests:

- Multiple sessions remain independent.
- Completed task records preserve outcome envelopes.
- Snapshot includes enough result state for observers.

Daemon tests:

- Two clients subscribe to one session and both receive events.
- Slow/dropped subscriber does not block another subscriber.
- Observer can snapshot/subscribe but cannot submit/cancel.
- Controller can submit/cancel.
- Non-owner mutation returns permission-denied.
- Strict session rejects manual approval from unrelated TUI client.
- TUI manual session allows approval only from attached approver/controller.

TUI/CLI tests:

- TUI attach declares client kind `Tui`.
- CLI daemon commands declare client kind `Cli`.
- TUI can attach observer-only and render state without mutation controls.
- CLI JSON output includes permission errors clearly.

## Non-Goals

Do not implement network authentication for public remote use.

Do not implement user accounts.

Do not implement durable persistence unless needed for snapshot tests.

Do not implement full artifact download service unless already easy.

Do not make multi-active-task execution the default unless the runtime is explicitly ready.

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
cargo test -p eggsec-cli
./scripts/check-architecture-guards.sh
```

Manual smoke checks:

```text
1. Start daemon.
2. Connect TUI client and create session.
3. Connect CLI client and list sessions.
4. Attach CLI as observer and watch events.
5. Submit task from TUI.
6. Confirm both clients receive events.
7. Attempt submit from observer-only CLI and confirm permission-denied.
8. Cancel from controller and confirm event fanout.
```

## Acceptance Criteria

- Daemon tracks clients explicitly.
- Sessions have clear owner/controller/observer semantics.
- Multiple clients can observe one session.
- Mutating operations enforce permissions.
- Policy approval routing cannot be abused by strict/programmatic surfaces.
- Runtime snapshots retain enough state for observers.
- Event fanout works without blocking runtime.
- TUI and CLI daemon clients declare client kind/surface.
- Documentation explains the session/client model before broader transport expansion.
