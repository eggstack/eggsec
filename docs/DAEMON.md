# Daemon Persistence

The `eggsec-daemon` crate provides durable session state backed by SQLite. The daemon is **optional** — CLI/TUI manual mode operates without a daemon by default, with `ManualPermissive` as the first-class enforcement profile.

## Session Lifecycle

Session snapshots are persisted at lifecycle points (create, submit, cancel, close) and recovered automatically on daemon restart. Closing a session persists a final snapshot (with `closed=true` and cancelled tasks) but does **not** delete the session. History is preserved and accessible via `daemon history` / `daemon show`.

On startup, `recover_persisted_state()` hydrates all persisted sessions. Running/queued tasks are dropped (not auto-resumed) and recorded as `Cancelled` with `last_error: "interrupted by daemon restart"`. Only completed task records are preserved across restarts.

## Capabilities

The daemon's default capabilities are **conservative** — when `full-executor` is enabled but lab mode is not configured, only a safe subset of task kinds is advertised (excluding hazardous families such as stress, packet, wireless-deauth, postex, c2, and evasion). Full capabilities require explicit `--full-executor` configuration.

## Transport

The daemon supports pluggable transport layers for client connectivity:

| Transport | Feature Flag | Default | Description |
|-----------|-------------|---------|-------------|
| Unix socket | Built-in | Yes (`/tmp/eggsec-daemon.sock`) | JSON-line protocol over Unix domain socket; primary IPC transport |
| HTTP/SSE | `http-api` | No (`127.0.0.1:9876`) | HTTP REST + Server-Sent Events via `axum`; 12 routes mapping to `ClientCommand`; loopback-only bind by default; requires explicit `http-api` feature |

WebSocket and gRPC transports were evaluated but deferred — they are not implemented in Phase 12.

The daemon advertises its available transports to clients via `DaemonCapabilities` (returned in `ServerMessage::Capabilities`). Clients send requests through `DaemonRequestContext` which carries the client ID, peer address, and transport kind. The daemon includes a `DAEMON_PROTOCOL_VERSION` (currently `1`) in its welcome message for client-side compatibility checks.

### HTTP Transport Details

- Binds to loopback only (`127.0.0.1`) by default; public bind (`0.0.0.0`) requires explicit config and emits a warning
- Uses `McpStrict` enforcement profile by default — noninteractive, no manual overrides
- 12 HTTP routes map to `ClientCommand` variants (create session, submit task, cancel, list sessions, etc.)
- SSE endpoint provides real-time session event streaming

## Configuration

| Field | Default | Description |
|-------|---------|-------------|
| `enable_persistence` | `true` | Persist session snapshots and audit events to SQLite |
| `data_dir` | `~/.local/share/eggsec/daemon/` | Directory for the `eggsec-daemon.sqlite` database file |

## Features

- **Session snapshots** — `SessionSnapshot` stored as JSON with timestamps in `session_snapshots` table
- **Session recovery** — On startup, `recover_persisted_state()` hydrates all persisted sessions; running/queued tasks are dropped (not auto-resumed) and recorded as `Cancelled` with `last_error: "interrupted by daemon restart"`. Only completed task records are preserved across restarts.
- **Audit event logging** — Security actions (create-session, submit-task, cancel, etc.) recorded with action, surface, outcome, client/session IDs, and timestamp
- **Artifact indexing** — Task artifacts (`ArtifactRef`) persisted within session snapshots, tracked by session association with kind, path, and MIME type
- **Schema migration** — SQLite schema versioned via `schema_meta` table (current: `2`); WAL mode enabled for concurrent reads; newer-than-current stored versions are explicitly refused to avoid silent corruption on downgrade.

## CLI Commands

```bash
# Start daemon with persistence (default)
eggsec daemon start

# List all persisted sessions
eggsec daemon history
eggsec daemon history --json

# Inspect a specific session's persisted snapshot
eggsec daemon show <session-id>
eggsec daemon show <session-id> --json

# Check daemon health
eggsec daemon status

# Stop daemon
eggsec daemon stop
```

## Local Smoke Test

`scripts/smoke-daemon-local.sh` is the canonical local-only lifecycle test for the
daemon. It runs against an ephemeral socket and a temporary data directory, with
no public network exposure. It validates daemon start, health, client
declaration, session create/list/snapshot, observer-deny + owner-allow
permission posture, persisted history/show, event stream subscription, and
graceful SIGTERM shutdown. Run with:

```bash
bash scripts/smoke-daemon-local.sh                 # defaults
bash scripts/smoke-daemon-local.sh /custom/path    # custom socket path
```

## Database Schema

| Table | Columns | Purpose |
|-------|---------|---------|
| `session_snapshots` | `session_id` (PK), `snapshot_json`, `created_at_secs` | Session state snapshots |
| `audit_events` | `audit_id` (PK), `action`, `surface`, `outcome`, `client_id`, `session_id`, `created_at_secs` | Security audit log |
| `schema_meta` | `key` (PK), `value` | Schema version tracking |
