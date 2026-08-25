---
name: eggsec-daemon
description: "Persistent session daemon and frontend-neutral runtime - use when working with eggsec-daemon, session persistence, Unix-socket IPC, daemon clients, task lifecycle, or the runtime bridge."
---

# Eggsec Daemon & Runtime Skill

Persistent session host (`eggsec-daemon`), its IPC protocol (`eggsec-daemon-protocol`), and the frontend-neutral runtime (`eggsec-runtime`) that TUI/CLI/daemon share.

## Crate Layout

| Crate | Purpose |
|-------|---------|
| `crates/eggsec-daemon/` | Session host: `server.rs`, `host.rs`, `client.rs`, `client_registry.rs`, `protocol.rs`, `http.rs`, SQLite store (`store/`) |
| `crates/eggsec-daemon-protocol/` | Wire types + client registry shared by daemon and clients |
| `crates/eggsec-runtime/` | `Runtime`, `RuntimeTaskExecutor`, task lifecycle DTOs |

## Dependency Rules (guard-enforced)

- `eggsec-runtime`: only serde/serde_json, thiserror, tokio, tokio-util, tracing, uuid. No TUI/transport/persistence.
- `eggsec-daemon`: default deps are `eggsec-runtime` + `eggsec-daemon-protocol` only. Engine dep optional behind `full-executor`; HTTP/SSE transport behind `http-api`. Never TUI deps.

## Transport

- Primary: Unix domain socket, line-based JSON protocol
- Optional: loopback HTTP/SSE behind the `http-api` feature (daemon crate)

## Client Contract

1. Connect to socket
2. Send `DeclareClient { kind: ClientKind::Tui|Cli|..., label }` - must succeed before session-scoped commands
3. Authorization uses the `CommandPermission` enum (per-command RBAC)
4. Observers cannot submit/cancel tasks (`ErrorCode::PermissionDenied`)
5. On strict-surface sessions only the Owner can approve policies; `ApprovePolicy` returns `ErrorCode::Unsupported` until wired

## CLI Integration

Daemon client commands live in the `eggsec-cli` shell (`crates/eggsec-cli/src/daemon_cli.rs`), feature-gated `daemon-client`:

```bash
eggsec daemon start|status|stop
eggsec daemon history [--json]
eggsec daemon show <session-id> [--json]
eggsec session list|create|snapshot
eggsec task submit|cancel|watch
```

Local lifecycle smoke test: `bash scripts/smoke-daemon-local.sh [socket-path]` (ephemeral socket, observer-deny + owner-allow posture checks).

## Runtime Bridge

`crates/eggsec/src/runtime_bridge/` converts runtime DTOs (`RuntimeSurface`, `RunRequest`, `TaskKind`) into engine enforcement types (`ExecutionSurface`, `OperationDescriptor`, `EnforcementContext`). `preflight_run_request()` previews policy; `approve_run_request()` produces the pre-dispatch authorization bundle.

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
