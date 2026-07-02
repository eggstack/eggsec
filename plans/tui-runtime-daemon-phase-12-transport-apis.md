# Phase 12 Plan: Transport APIs Beyond Local Socket

## Goal

Add carefully scoped transport APIs on top of the hardened daemon protocol without moving transport dependencies into `eggsec-runtime` or weakening daemon authorization. This phase should make the daemon protocol available over additional local or explicitly configured transports for future frontends and harnesses.

Default remains local-only. Any network-facing transport must be explicit opt-in.

## Baseline Requirements Before Starting

Do not start this phase until:

- daemon RBAC uses `CommandPermission` and actual session surfaces;
- `ApprovePolicy` is either properly wired or explicitly unsupported;
- local socket daemon tests pass;
- persistence/audit behavior is either implemented or intentionally deferred;
- `eggsec-runtime` remains transport-free.

## Transport Strategy

Keep the daemon protocol transport-neutral.

Recommended split:

- `eggsec-daemon`: protocol, host, local socket server, daemon client, shared auth/session/access logic.
- `eggsec-daemon-http` or a feature-gated module: HTTP/WebSocket/SSE transport if needed.
- `eggsec-daemon-grpc` or feature-gated module: gRPC transport if needed.

Avoid making `eggsec-runtime` depend on `axum`, `tonic`, `tower`, `hyper`, or WebSocket crates.

## Initial Transport Set

Implement in this order:

1. Local Unix socket remains canonical.
2. Loopback HTTP + SSE/WebSocket for local web UI/dev clients.
3. Optional gRPC only if there is a concrete consumer.
4. Public network binding remains disabled by default and gated behind explicit config.

## Protocol Mapping

Map existing daemon protocol commands, do not invent separate behavior per transport.

HTTP endpoints can be thin wrappers:

```text
GET  /health
GET  /capabilities
POST /clients/declare
GET  /sessions
POST /sessions
GET  /sessions/{session_id}/snapshot
POST /sessions/{session_id}/tasks
POST /sessions/{session_id}/tasks/{task_id}/cancel
POST /sessions/{session_id}/cancel-active
GET  /sessions/{session_id}/events
POST /sessions/{session_id}/policy/approve
DELETE /sessions/{session_id}
```

Event streaming options:

- SSE for simple browser/client event stream.
- WebSocket for bidirectional command/event transport.
- Keep JSON message shapes aligned with `ClientCommand`/`ServerMessage`.

## Security Defaults

Required defaults:

- Bind only to Unix socket or `127.0.0.1` by default.
- Reject public binds unless config explicitly enables them.
- Emit warning on public bind.
- Require client declaration on every connection/session.
- Preserve RBAC for every transport.
- Do not treat browser/web transport as manual TUI surface by default.
- Do not allow policy approval on strict/programmatic surfaces.

## Authentication

For this phase, avoid internet-facing authentication complexity.

Allowed options:

- local-only no-auth with OS socket permissions;
- loopback-only token generated at daemon start;
- explicit static token in local config for dev web UI.

Do not claim production remote authentication until a full auth plan exists.

## Workstream 1: Transport Abstraction

Add a small abstraction that lets transports call into `DaemonHost` consistently.

Suggested shape:

```rust
pub struct DaemonRequestContext {
    pub client_id: Option<ClientId>,
    pub peer: Option<String>,
    pub transport: TransportKind,
}

pub enum TransportKind {
    UnixSocket,
    LoopbackHttp,
    WebSocket,
    Grpc,
}
```

`DaemonHost::handle_command` should remain the central authorization gate.

## Workstream 2: HTTP/SSE Local API

Add local HTTP API behind feature flag if needed:

```toml
http-api = ["dep:axum", "dep:tower", ...]
```

Implementation requirements:

- Route handlers convert HTTP requests to `ClientCommand`.
- Use shared daemon client/session context.
- Event streaming uses runtime event subscriptions filtered by session.
- JSON error responses include protocol `ErrorCode`.
- Tests cover unauthorized mutating requests.

## Workstream 3: WebSocket Command/Event API

If WebSocket is implemented:

- use same `ClientCommand`/`ServerMessage` framing;
- require `DeclareClient` first;
- maintain connection client ID;
- stream `RuntimeEvent` messages after `Subscribe`;
- handle backpressure/dropped clients safely.

## Workstream 4: gRPC Evaluation

Do not implement gRPC unless there is a concrete reason. If implemented, generate/provide proto definitions that mirror daemon protocol semantics and preserve error codes.

## Workstream 5: Docs and Capability Truth

Update capabilities to advertise transports actually enabled in build/config.

`RuntimeCapabilities` should not advertise transports. Daemon capabilities should include transport information.

Suggested split:

```rust
RuntimeCapabilities
DaemonCapabilities { runtime: RuntimeCapabilities, transports: Vec<TransportCapability> }
```

## Files Likely to Change

- `crates/eggsec-daemon/Cargo.toml`
- `crates/eggsec-daemon/src/server.rs`
- `crates/eggsec-daemon/src/protocol.rs`
- `crates/eggsec-daemon/src/host.rs`
- `crates/eggsec-daemon/src/http.rs` if added
- `crates/eggsec-daemon/src/ws.rs` if added
- `crates/eggsec-cli/src/daemon_cli.rs`
- `crates/eggsec-tui/src/runtime_client/daemon.rs`
- `architecture/overview.md`
- `docs/CI_ARCHITECTURE_GUARDS.md`
- `scripts/check-architecture-guards.sh`

## Tests

- HTTP health/capabilities.
- HTTP declare client then create session.
- Unauthorized submit without declaration denied.
- Observer submit denied.
- Strict-surface approval denied/unsupported.
- SSE/WebSocket event stream receives task events.
- Public bind disabled by default.
- Runtime crate remains free of transport deps.

## Non-Goals

Do not expose public remote API by default.

Do not implement full user accounts.

Do not replace local socket transport.

Do not fork daemon semantics per transport.

Do not add transport dependencies to `eggsec-runtime`.

## Validation

Run:

```bash
cargo fmt --all --check
cargo check -p eggsec-daemon
cargo test -p eggsec-daemon
cargo check -p eggsec-daemon --features http-api
cargo test -p eggsec-daemon --features http-api
cargo check -p eggsec-runtime
cargo test -p eggsec-runtime
./scripts/check-architecture-guards.sh
```

## Acceptance Criteria

- New transports call the same daemon authorization path.
- Default remains local-only.
- Public bind requires explicit config and warning.
- Client declaration and RBAC work over every transport.
- Event streaming works without blocking runtime.
- Capabilities accurately report configured daemon transports.
- `eggsec-runtime` remains transport-free.
