---
name: eggsec-daemon
description: "Persistent session daemon and frontend-neutral runtime - use when working with eggsec-daemon, session persistence, Unix-socket IPC, daemon clients, task lifecycle, or the runtime bridge."
---

# Eggsec Daemon & Runtime Skill

Persistent session host (`eggsec-daemon`), its IPC protocol (`eggsec-daemon-protocol`), and the frontend-neutral runtime (`eggsec-runtime`) that TUI/CLI/daemon share.

## Crate Layout

| Crate | Purpose |
|-------|---------|
| `crates/eggsec-daemon/` | Session host: `server.rs`, `host.rs` (facade), `host_auth.rs` (RBAC), `host_persistence.rs` (fan-out), `client.rs`, `config.rs`, `error.rs`, `http.rs`, SQLite store (`store/`). |
| `crates/eggsec-daemon-protocol/` | Wire types (`protocol.rs`) + `client_registry.rs` — the shared `ClientKind` / `CommandPermission` RBAC types. **There is no daemon-local `client_registry.rs`**: `command_permission()` lives here, so daemon and clients cannot drift. |
| `crates/eggsec-runtime/` | `Runtime`, `RuntimeTaskExecutor`, task lifecycle DTOs (`runtime.rs` facade; `runtime_config.rs`, `runtime_sink.rs`) |

## Dependency Rules (guard-enforced)

- `eggsec-runtime`: only serde/serde_json, thiserror, tokio, tokio-util, tracing, uuid. No TUI/transport/persistence.
- `eggsec-daemon`: default deps are `eggsec-runtime` + `eggsec-daemon-protocol` only. Engine dep optional behind `full-executor`; HTTP/SSE transport behind `http-api`. Never TUI deps.

## Transport

- Primary: Unix domain socket, line-based JSON protocol
- Optional: loopback HTTP/SSE behind the `http-api` feature (daemon crate)

The desktop GUI (`crates/eggsec-gui`) connects to this daemon over the Unix
socket and nothing else — it never links the daemon crate. It requires an
`eggsec-daemon --full-executor`: `eggsec daemon start` and a default-built
`eggsec-daemon` host a **no-op executor** that answers every command correctly
and rejects every task, which the GUI reports as "the daemon advertises no
task kinds" rather than presenting a submit button that cannot work.

## Client Contract

1. Connect to socket
2. Send `DeclareClient { kind: ClientKind::Tui|Cli|Gui|..., label }` - must succeed before session-scoped commands
3. Authorization uses the `CommandPermission` enum (per-command RBAC)
4. Observers cannot submit/cancel tasks (`ErrorCode::PermissionDenied`)
5. On strict-surface sessions only the Owner can approve policies; `ApprovePolicy` returns `ErrorCode::Unsupported` until wired
6. Durable retrieval (protocol v2, Phase E): `GetTaskResult { session_id, task_id }` → `TaskResult { status, outcome }` reads live state first, persisted snapshot second — reconnecting clients fetch completed results without event replay. Reconnect = re-declare + re-subscribe; missed events recover as state. See `docs/DAEMON_PARITY.md`.
7. `DescribeOperations` publishes the canonical operation catalog plus a JSON Schema derived from the request DTOs. Read-only and additive: no session, no session role, but a **declared client** is required (same rule as the other daemon-wide reads). `DaemonCapabilities.operation_schema` advertises availability and is `#[serde(default)]`, so a client built against an older protocol still decodes a newer daemon's capabilities. A daemon too old to know the command answers `ErrorCode::UnsupportedCommand` — degrade to a static form set, never fail.
8. Session-scoped audit events record the **session's runtime surface** label (`desktop-gui-manual`, `tui-manual`, `cli-manual`, ...), so each frontend is attributable in the persisted trail. Events with no session (`declare-client`, startup recovery, `approve-policy`) keep the literal `"daemon"`.

## Client kinds

`ClientKind` lives in `crates/eggsec-daemon-protocol/src/client_registry.rs`
and is the shared `ClientKind` / `CommandPermission` RBAC type — there is no
daemon-local copy, so daemon and clients cannot drift.

`ClientKind::Gui` was added additively for the desktop GUI
(`crates/eggsec-daemon-protocol/src/client_registry.rs:19`). The protocol
version stays at 2: a daemon that predates the variant fails to deserialize
the `DeclareClient` command and rejects the client, which is a fail-closed
outcome rather than a silent misclassification. `ClientCommand` and
`ServerMessage` variant sets are unchanged.

The RBAC matrix's manual-approval arm now includes
`RuntimeSurface::DesktopGuiManual` beside `TuiManual` and `CliManual`
(`crates/eggsec-daemon-protocol/src/client_registry.rs:199`), so an unrelated
controller may approve a policy on a GUI session exactly as on the CLI and TUI.
That mirrors the classification `eggsec-policy` makes for the surface; the two
must not drift.

## CLI Integration

Daemon client commands live in the `eggsec-cli` shell (`crates/eggsec-cli/src/daemon_cli.rs`), feature-gated `daemon-client`:

```bash
eggsec daemon start|status|stop
eggsec daemon history [--json]
eggsec daemon show <session-id> [--json]
eggsec session list|create|snapshot
eggsec task submit|cancel|watch
eggsec task result <session-id> <task-id> [--json]   # durable result retrieval (protocol v2)
```

Local lifecycle smoke test: `bash scripts/smoke-daemon-local.sh [socket-path]` (ephemeral socket, observer-deny + owner-allow posture checks).

## Runtime Bridge

`crates/eggsec/src/runtime_bridge/` converts runtime DTOs (`RuntimeSurface`, `RunRequest`, `TaskKind`) into engine enforcement types (`ExecutionSurface`, `OperationDescriptor`, `EnforcementContext`). `preflight_run_request()` previews policy; `approve_run_request()` produces the pre-dispatch authorization bundle. Phase 1: the bundle dispatches via the single canonical boundary (`dispatch::execute_approved` over `CanonicalOperationRequest::from_task_kind`), sharing executor selection with embedded TUI execution (`execute_canonical`). Phase 3 closure: `RuntimeSurface` is a wire DTO with exhaustive bidirectional conversion (`Unknown` rejected); engine operation-ID/target adapters delegate to the wire-side `TaskKind` match (no parallel tables); `TaskResult`→envelope conversion is engine-owned (`dispatch::task_result_envelope`); both adapters route cancellation through the shared `eggsec-runtime::race_with_cancel` primitive (no ad-hoc select). Closure tests: `crates/eggsec/tests/runtime_contract_closure.rs`.

## Testing

```bash
cargo test -p eggsec-daemon
cargo test -p eggsec-runtime
cargo check --workspace --no-default-features   # dependency-rule baseline
```

## Resources

- `docs/DAEMON.md` - Transport config, schema, CLI reference
- `architecture/daemon.md` - Persistence, session lifecycle, transport deep dive
- `architecture/runtime.md` / `architecture/runtime_bridge.md` - Runtime types and bridge contract
- `.opencode/skills/eggsec-gui/SKILL.md` - the desktop GUI client of this daemon
- `.opencode/skills/eggsec-frontend-client/SKILL.md` - the shared runtime-client contract

## Phase D Hotspots (2026-09-09)

`host.rs` is a stable facade; RBAC lives in `host_auth.rs` (`role_for_session`, `may_observe_session`), persistence fan-out in `host_persistence.rs`. `runtime.rs` is a facade over `runtime_config.rs`/`runtime_sink.rs`. Import from the cohesive module, not the facade internals.
