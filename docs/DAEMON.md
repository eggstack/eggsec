# Daemon Persistence

The `eggsec-daemon` crate provides durable session state backed by SQLite. The daemon is **optional** — CLI/TUI manual mode operates without a daemon by default, with `ManualPermissive` as the first-class enforcement profile.

## Session Lifecycle

Session snapshots are persisted at lifecycle points (create, submit, cancel, close) and recovered automatically on daemon restart. Closing a session persists a final snapshot (with `closed=true` and cancelled tasks) but does **not** delete the session. History is preserved and accessible via `daemon history` / `daemon show`.

On startup, `recover_persisted_state()` hydrates all persisted sessions. Running/queued tasks are dropped (not auto-resumed) and recorded as `Cancelled` with `last_error: "interrupted by daemon restart"`. Only completed task records are preserved across restarts.

## Result Retrieval After Reconnect

Completed task results do not depend on transient event delivery. `GetTaskResult { session_id, task_id }` reads live runtime state first and falls back to the persisted snapshot, so reconnecting clients (and clients connecting after a daemon restart) get the same `TaskResult { status, outcome }` without replaying events:

```bash
eggsec task result <session-id> <task-id>
eggsec task result <session-id> <task-id> --json
```

See [docs/DAEMON_PARITY.md](DAEMON_PARITY.md) for the full local-vs-daemon parity matrix, reconnect/replay semantics, and the Python client mapping.

## Capabilities

The daemon publishes a **conservative** task-kind list — a safe subset that
excludes the hazardous families (stress, packet, wireless-deauth, postex, c2,
evasion). The published list is **derived**, never declared beside the
executor:

> published task kinds = the executor's declared support ∩ the daemon's conservative policy

A daemon that cannot execute anything therefore publishes **nothing**: the
"advertises twenty, executes none" state is unreachable by construction. The
declared support comes from the executor itself
(`RuntimeTaskExecutor::supported_task_kinds()`), so a rejecting executor
publishes an empty list and a real one publishes the conservative set.

Published is not the same as executable. A published kind whose *engine
feature* is off is denied at dispatch with `FeatureMissing`, not
`UnsupportedTaskKind` — the feature-level gap is real, and
[Obtaining the daemon](#obtaining-the-daemon) says which build compiles which
features.

## Obtaining the daemon

The daemon is a separate cargo package with its own binary, so it installs on
its own. Two invocations cover the whole story: the daemon binary, and the CLI
that talks to it.

```bash
# The daemon binary, with the engine executor compiled in.
cargo install --path crates/eggsec-daemon --features service --locked

# The deprecated alias for the same profile. `service` is the canonical
# spelling; `full-executor` still works and still produces the same binary.
cargo install --path crates/eggsec-daemon --features full-executor --locked

# The CLI, with the `daemon`, `session`, and `task` subcommands.
cargo install --path crates/eggsec-cli --features daemon-client --locked
```

`--locked` builds against the workspace lockfile, so the recipe is
reproducible.

### The cargo feature is not the runtime flag

`--full-executor` is a **runtime flag** on the daemon binary. `service` (and
its deprecated alias `full-executor`) is a **cargo feature** the binary is
**built** with. They are different mechanisms:

- A binary **built** with `--features service` has the engine executor
  compiled in. The runtime `--full-executor` flag then turns that executor
  on.
- A binary **built without** the cargo feature still accepts the runtime
  `--full-executor` flag, warns, and falls back to a no-op executor: it
  publishes an **empty** task-kind list and rejects every task. The flag
  alone cannot add an engine that was not compiled in.

The same split applies to the CLI: `daemon-client` is **not** in the CLI's
default feature set, so `cargo install --path crates/eggsec-cli --locked`
produces a CLI **without** the `daemon`, `session`, and `task` subcommands.

### What the resulting daemon uses by default

| Setting | Default |
|---------|---------|
| Socket path | `/tmp/eggsec-daemon.sock` |
| Data directory | `$HOME/.local/share/eggsec/daemon` |
| Max clients | 10 |
| Log level | `info` |

### The honest limitation

An advertised capability is not a promise that this build can execute it.

- The **daemon binary** built with `--features service` links the engine's
  daemon service profile, so the capabilities that profile names are compiled
  in.
- The **CLI-hosted** daemon (`eggsec daemon start`) links the engine with the
  `cli` feature only. In that build 8 of the 20 advertised capabilities are
  feature-gated off and are denied at dispatch with `FeatureMissing` — not
  `UnsupportedTaskKind`. Milestone 008 owns the daemon's engine profile and
  milestone 006 owns the client-side resolution of that gap.
- A daemon **built without** the cargo feature is a third thing again: it
  publishes an **empty** task-kind list and rejects every task.

### What this recipe does not cover

| Not covered | Why | Owner |
|---|---|---|
| A **packaged** artifact (`.deb`, `.rpm`, `.pkg`, Homebrew formula, installer) | Packaging is the distribution workstream's | M007 → distribution workstream |
| **Signing** or **notarization** | Same | M007 → distribution workstream |
| **Update channels** or a version-pinned release feed | Same | M007 → distribution workstream |
| **Windows** | `ADR-0010`: the daemon's transport is a Unix domain socket, so the daemon and the GUI do not compile for Windows | A daemon-subsystem transport milestone, with its own ADR |
| A **`PATH` guarantee** | `cargo install` writes to `$CARGO_HOME/bin` (default `~/.cargo/bin`), which the project does **not** promise. **No component looks for the daemon there** — not the GUI, not the CLI | Standing rule |
| A **feature-complete** daemon | The CLI-hosted daemon links the engine with the `cli` feature only, so 8 of the 20 advertised capabilities are feature-gated off in that build | **M008** (profile) and **M006** (publication) |
| **Session persistence** for the CLI-hosted daemon | The CLI's start path uses `noop_store()` | The daemon binary |

`cargo install` is a build recipe, not a distribution channel: there is no
signed artifact, no installer, and no update feed behind it.

### Starting a daemon honestly

Both start paths refuse to displace a daemon that is already serving the
socket path, and both report success only after the daemon answers `Health`:

- The **pre-flight** connects to the socket path and asks for `Health`. A
  daemon that answers owns the path: the start is refused, the socket is left
  untouched, and the command exits non-zero. An unreachable socket (a stale
  file left by a crashed daemon) is *not* live — the start proceeds and the
  server's own unlink clears it.
- The **readiness wait** polls `Health` (and `Capabilities`) until the daemon
  answers, bounded by a named timeout. The success message names the socket
  path, the protocol version, and the number of advertised task kinds; it
  never claims that every advertised capability is executable, and it states
  that the CLI-hosted daemon does not persist sessions.

## Transport

The daemon supports pluggable transport layers for client connectivity:

| Transport | Feature Flag | Default | Description |
|-----------|-------------|---------|-------------|
| Unix socket | Built-in | Yes (`/tmp/eggsec-daemon.sock`) | JSON-line protocol over Unix domain socket; primary IPC transport |
| HTTP/SSE | `http-api` | No (`127.0.0.1:9876`) | HTTP REST + Server-Sent Events via `axum`; 14 routes mapping to `ClientCommand`; loopback-only bind by default; requires explicit `http-api` feature |

WebSocket and gRPC transports were evaluated but deferred — they are not implemented in Phase 12.

The daemon advertises its available transports to clients via `DaemonCapabilities` (returned in `ServerMessage::Capabilities`). Clients send requests through `DaemonRequestContext` which carries the client ID, peer address, and transport kind. The daemon includes a `DAEMON_PROTOCOL_VERSION` (currently `2`; v2 adds durable `GetTaskResult` / `TaskResult`) in its health response for client-side compatibility checks.

### HTTP Transport Details

- Binds to loopback only (`127.0.0.1`) by default; public bind (`0.0.0.0`) requires explicit config and emits a warning
- Uses `McpStrict` enforcement profile by default — noninteractive, no manual overrides
- 14 HTTP routes map to `ClientCommand` variants (create session, submit task, get task result, cancel, list sessions, persisted history, etc.)
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
# Start a daemon in the foreground. This hosts the engine executor, but it
# does NOT persist sessions — the daemon binary is the persistent one.
eggsec daemon start

# List all persisted sessions
eggsec daemon history
eggsec daemon history --json

# Inspect a specific session's persisted snapshot
eggsec daemon show <session-id>
eggsec daemon show <session-id> --json

# Check daemon health
eggsec daemon status

# Fetch one task's status + completed outcome (durable across reconnect)
eggsec task result <session-id> <task-id>
eggsec task result <session-id> <task-id> --json

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

## Hotspot Modules (Phase D, 2026-09-09)

`host.rs` is a stable facade. RBAC ownership lives in `host_auth.rs` (`role_for_session`, `may_observe_session`); persistence fan-out lives in `host_persistence.rs` (`PERSISTENCE_TASK_TIMEOUT`, `persistence_with_timeout`, `record_audit_event_logged`). Session ownership behavior is unchanged (legacy ownerless sessions stay visible; owned sessions restrict to owner + allowed clients).
