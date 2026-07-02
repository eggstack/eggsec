# Phase 7 Plan: Local Daemon MVP

## Goal

Add the first headless daemon host for `eggsec-runtime`. The daemon should expose the existing runtime/session/task/event model over a local-only transport without changing task execution semantics or weakening execution-surface enforcement.

This phase should produce a minimal local daemon suitable for CLI and future TUI attach work. Keep it narrow: session creation, task submission, event streaming, cancellation, snapshots, capabilities, and health.

## Scope

Build a local daemon MVP around the runtime that already exists. The daemon should host `eggsec-runtime`; it must not reimplement runtime task lifecycle, dispatch, policy, or session state.

Preferred initial crate:

```text
crates/eggsec-daemon/
```

Alternative if crate churn is too high:

```text
crates/eggsec/src/daemon/
```

Preferred: new crate, because daemon transport dependencies should not enter `eggsec-runtime`.

## Transport Choice

Start with local-only transport.

Recommended MVP:

- Unix domain socket on macOS/Linux.
- Loopback TCP fallback only if Unix socket support complicates Windows or test coverage.
- JSON line protocol for MVP, unless an existing project-standard transport layer already exists.

Do not add WebSocket/gRPC/SSE yet unless the repo already has a thin, reusable transport stack that can be wired without scope creep.

## Execution Surface Rules

The daemon must not make all clients equivalent to `TuiManual`.

Define explicit daemon client surfaces:

- `DaemonLocalManual` if adding a new surface is warranted; or
- use `CliManual` for local CLI-initiated manual sessions; and
- use strict surfaces for programmatic clients, MCP, REST, CI, and agents.

Minimum for this phase:

- A session creation request must include requested surface or client kind.
- Runtime session stores the resolved surface.
- Agent/MCP-like clients cannot request TUI manual override behavior.
- Audit/log metadata includes client identity and surface.

## Protocol Commands

Define request/response DTOs outside transport-specific code. Suggested module:

```text
crates/eggsec-runtime/src/protocol.rs
```

Or, if avoiding runtime expansion, use:

```text
crates/eggsec-daemon/src/protocol.rs
```

Required commands:

```rust
ClientCommand::CreateSession { surface, scope, labels }
ClientCommand::ListSessions
ClientCommand::GetSnapshot { session_id }
ClientCommand::SubmitTask { session_id, request }
ClientCommand::CancelTask { session_id, task_id }
ClientCommand::CancelActive { session_id }
ClientCommand::Subscribe { session_id }
ClientCommand::Capabilities
ClientCommand::Health
```

Required server responses/events:

```rust
ServerMessage::Ok { request_id }
ServerMessage::Error { request_id, code, message }
ServerMessage::SessionCreated { request_id, session_id }
ServerMessage::Sessions { request_id, sessions }
ServerMessage::Snapshot { request_id, snapshot }
ServerMessage::TaskSubmitted { request_id, task_id }
ServerMessage::Capabilities { request_id, capabilities }
ServerMessage::Health { request_id, status }
ServerMessage::RuntimeEvent { session_id, event }
```

Every command should include a client-generated `request_id` or equivalent correlation ID.

## Daemon Runtime Host

Add a host struct that owns:

- `Arc<Runtime>`
- dispatcher/executor wiring
- session registry if runtime does not already expose list-session support
- client subscription registry
- daemon config
- shutdown token

Suggested shape:

```rust
pub struct DaemonHost {
    runtime: Arc<Runtime>,
    config: DaemonConfig,
}
```

Avoid putting socket accept loops inside `eggsec-runtime`.

## Session Listing

If runtime does not expose session listing, add a minimal `Runtime::list_sessions()` returning serializable session summaries.

Suggested DTO:

```rust
pub struct SessionSummary {
    pub session_id: SessionId,
    pub surface: RuntimeSurface,
    pub scope: Option<SessionScope>,
    pub active_count: usize,
    pub completed_count: usize,
    pub created_at_secs: u64,
}
```

Acceptance criteria:

- Daemon can list sessions without exposing internal locks or task records.
- TUI remote attach phase can use this summary.

## Event Subscription Semantics

For MVP, support one subscription stream per client connection. A subscribed client receives `RuntimeEvent` values for the chosen session.

Minimum behavior:

- Client sends `Subscribe { session_id }`.
- Server sends subsequent `RuntimeEvent` messages for that session.
- If runtime currently uses one global broadcast channel, filter by session ID in daemon host.
- Dropped clients do not block runtime.

Do not implement replay in this phase beyond `GetSnapshot`.

## CLI Entrypoint

Add daemon start command only if it fits the existing CLI model. Otherwise, add a standalone binary crate.

Possible commands:

```text
eggsec daemon start --socket <path>
eggsec daemon status --socket <path>
```

If CLI integration is deferred to Phase 9, this phase can provide a daemon binary and test client only.

## Files Likely to Change

- `Cargo.toml`
- `crates/eggsec-daemon/Cargo.toml`
- `crates/eggsec-daemon/src/lib.rs`
- `crates/eggsec-daemon/src/main.rs`
- `crates/eggsec-daemon/src/protocol.rs`
- `crates/eggsec-daemon/src/server.rs`
- `crates/eggsec-daemon/src/socket.rs`
- `crates/eggsec-runtime/src/runtime.rs`
- `crates/eggsec-runtime/src/session.rs`
- `crates/eggsec-runtime/src/capabilities.rs`
- `crates/eggsec/src/dispatch/*`
- `crates/eggsec-cli/src/main.rs` only if adding daemon start command now
- `scripts/check-architecture-guards.sh`
- `architecture/overview.md`

## Implementation Steps

1. Add `eggsec-daemon` workspace crate.
2. Add minimal daemon config type: socket path, max clients, shutdown behavior, default surface policy.
3. Add daemon protocol DTOs and JSON round-trip tests.
4. Add `Runtime::list_sessions()` or equivalent session summary API.
5. Implement daemon host wrapping existing runtime and dispatcher.
6. Implement local socket accept loop.
7. Implement request parsing and response serialization.
8. Implement commands: health, capabilities, create session, list sessions, snapshot, submit, cancel, subscribe.
9. Add integration test using a local temporary socket or in-memory transport abstraction.
10. Add architecture guard ensuring `eggsec-runtime` does not gain daemon transport dependencies.
11. Document daemon is local-only and experimental.

## Tests

Protocol tests:

- JSON round-trip for every command and response.
- Unknown command returns structured error.
- Invalid session ID returns structured error.

Host tests:

- Create session through daemon host.
- Submit task through daemon host using test executor.
- Subscribe and receive lifecycle events.
- Cancel task through daemon host.
- Snapshot reflects completed/cancelled task.

Socket tests if practical:

- Start server on temporary socket.
- Connect one client, send health, read response.
- Connect client, create session, submit task, read task event.

## Non-Goals

Do not implement remote network authentication beyond local-only restrictions.

Do not add WebSocket, REST, SSE, or gRPC.

Do not implement persistent sessions.

Do not implement multi-client authorization beyond basic client identity/surface tracking.

Do not make TUI remote attach depend on this until daemon MVP is tested.

## Validation

Run:

```bash
cargo fmt --all --check
cargo check -p eggsec-runtime
cargo test -p eggsec-runtime
cargo check -p eggsec-daemon
cargo test -p eggsec-daemon
cargo check -p eggsec
cargo check -p eggsec-tui
cargo check -p eggsec-cli
./scripts/check-architecture-guards.sh
```

## Acceptance Criteria

- `eggsec-daemon` or equivalent daemon host builds without TUI dependencies.
- Daemon hosts `eggsec-runtime` rather than duplicating lifecycle logic.
- Local-only transport supports health, capabilities, session create/list/snapshot, submit, cancel, and subscribe.
- Runtime events are streamable to a client.
- Execution surface is explicit at session creation.
- Strict surfaces cannot inherit TUI manual override behavior.
- Architecture guards still prevent transport dependencies in `eggsec-runtime`.
