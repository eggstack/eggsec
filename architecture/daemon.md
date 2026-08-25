# Daemon & Daemon Protocol

Two crates provide the persistent session host and its IPC wire format: `eggsec-daemon-protocol` (shared types, RBAC) and `eggsec-daemon` (server, persistence, optional HTTP transport). Together they enable multi-client session management, background task execution, and durable state across restarts.

## Role & Responsibilities

| Concern | Crate |
|---------|-------|
| Wire-format types (`ClientCommand`, `ServerMessage`, `ErrorCode`) | `eggsec-daemon-protocol` |
| RBAC client registry (`ClientRegistry`, `ClientKind`, `ClientRole`, `CommandPermission`) | `eggsec-daemon-protocol` |
| Unix socket server, command dispatch, event fan-out | `eggsec-daemon` |
| SQLite persistence, schema migration, startup recovery | `eggsec-daemon` |
| Optional HTTP/SSE transport | `eggsec-daemon` (`http-api` feature) |
| Optional full engine executor | `eggsec-daemon` (`full-executor` feature) |

## Location & Feature Gating

| Crate | Path | Dependencies | Features |
|-------|------|-------------|----------|
| `eggsec-daemon-protocol` | `crates/eggsec-daemon-protocol/` | `eggsec-runtime` only (no persistence, transport, or TUI deps) | None |
| `eggsec-daemon` | `crates/eggsec-daemon/` | `eggsec-runtime`, `eggsec-daemon-protocol`, `rusqlite 0.31 (bundled)`, `tokio`, `tracing`, `serde_json`, `anyhow`, `clap` | `http-api` (axum + async-stream + futures), `full-executor` (engine crate) |

Architecture guards enforce:
- `eggsec-daemon` has no TUI dependencies (`ratatui`/`crossterm`)
- Transport deps (`axum`, `async-stream`, `futures`) are optional behind `http-api`
- Engine dep (`eggsec`) is optional behind `full-executor`
- Default dependencies: `eggsec-runtime` + `eggsec-daemon-protocol` only

## Architecture

### eggsec-daemon-protocol (3 source files)

| Module | File | Contents |
|--------|------|----------|
| `lib` | `src/lib.rs` | Re-exports `client_registry` and `protocol` modules |
| `protocol` | `src/protocol.rs` | `ClientCommand` (14 variants), `ServerMessage` (13 variants), `ErrorCode` (11 variants), `TransportKind` (4 variants), `DaemonCapabilities`, `TransportCapability`, `DaemonRequestContext`, `DAEMON_PROTOCOL_VERSION` (= 1) |
| `client_registry` | `src/client_registry.rs` | `ClientKind` (7 variants), `ClientRole` (4 variants), `CommandPermission` (6 variants), `ClientInfo`, `ClientAccessRule`, `SessionAccess`, `ClientRegistry`, `check_permission()`, `command_permission()` |

#### ClientCommand — 14 variants (`protocol.rs:71-134`)

| # | Variant | Fields | Permission |
|---|---------|--------|-----------|
| 1 | `Health` | `request_id` | Public |
| 2 | `Capabilities` | `request_id` | Public |
| 3 | `DeclareClient` | `request_id`, `kind: ClientKind`, `label: Option<String>` | DeclaredClient |
| 4 | `CreateSession` | `request_id`, `surface: RuntimeSurface`, `scope: Option<SessionScope>`, `labels: Vec<String>` | DeclaredClient |
| 5 | `ListSessions` | `request_id` | DeclaredClient |
| 6 | `GetSnapshot` | `request_id`, `session_id: SessionId` | Observer |
| 7 | `SubmitTask` | `request_id`, `session_id: SessionId`, `request: RunRequest` | Controller |
| 8 | `CancelTask` | `request_id`, `session_id: SessionId`, `task_id: TaskId` | Controller |
| 9 | `CancelActive` | `request_id`, `session_id: SessionId` | Controller |
| 10 | `Subscribe` | `request_id`, `session_id: SessionId` | Observer |
| 11 | `CloseSession` | `request_id`, `session_id: SessionId` | Owner |
| 12 | `ApprovePolicy` | `request_id`, `session_id: SessionId`, `task_id: TaskId`, `approved: bool`, `reason: Option<String>` | Approver |
| 13 | `ListPersistedSessions` | `request_id` | DeclaredClient |
| 14 | `GetPersistedSnapshot` | `request_id`, `session_id: SessionId` | DeclaredClient |

#### ServerMessage — 13 variants (`protocol.rs:196-252`)

| # | Variant | Fields |
|---|---------|--------|
| 1 | `Ok` | `request_id` |
| 2 | `Error` | `request_id`, `code: ErrorCode`, `message: String` |
| 3 | `ClientDeclared` | `request_id`, `client_id: ClientId` |
| 4 | `SessionCreated` | `request_id`, `session_id: SessionId` |
| 5 | `Sessions` | `request_id`, `sessions: Vec<SessionSummary>` |
| 6 | `Snapshot` | `request_id`, `snapshot: SessionSnapshot` |
| 7 | `TaskSubmitted` | `request_id`, `task_id: TaskId` |
| 8 | `Capabilities` | `request_id`, `capabilities: DaemonCapabilities` |
| 9 | `Health` | `request_id`, `status: String`, `version: String`, `protocol_version: u32` |
| 10 | `RuntimeEvent` | `session_id: SessionId`, `event: RuntimeEvent` |
| 11 | `SessionClosed` | `request_id` |
| 12 | `PersistedSessions` | `request_id`, `sessions: Vec<SessionSummary>` |
| 13 | `PersistedSnapshot` | `request_id`, `snapshot: Option<SessionSnapshot>` |

#### ErrorCode — 11 variants (`protocol.rs:51-66`)

| # | Variant | Meaning |
|---|---------|---------|
| 1 | `InvalidRequest` | Malformed command or invalid field |
| 2 | `SessionNotFound` | Session ID does not exist |
| 3 | `TaskNotFound` | Task ID does not exist |
| 4 | `TaskAlreadyCompleted` | Cannot cancel a terminal task |
| 5 | `UnsupportedCommand` | Command not recognized |
| 6 | `Internal` | Unrecoverable server error |
| 7 | `PermissionDenied` | RBAC check failed |
| 8 | `InvalidSurface` | Surface mismatch |
| 9 | `ClientNotDeclared` | Client must call DeclareClient first |
| 10 | `Unsupported` | Operation not wired yet (e.g. ApprovePolicy) |
| 11 | `InvalidState` | Operation cannot proceed in current state |

#### ClientKind — 7 variants (`client_registry.rs:11-19`)

`Cli`, `Tui`, `DaemonInternal`, `Mcp`, `Rest`, `Agent`, `Unknown` (default)

#### ClientRole — 4 variants (`client_registry.rs:25-30`)

`Owner`, `Controller`, `Observer`, `Approver`

#### CommandPermission — 6 variants (`client_registry.rs:37-50`)

`Public`, `DeclaredClient`, `Observer`, `Controller`, `Owner`, `Approver`

### eggsec-daemon (11 source files)

| Module | File | Purpose |
|--------|------|---------|
| `main` | `src/main.rs` | Binary entry point: CLI args (clap), store setup, host creation, shutdown signal, event persistence loop |
| `lib` | `src/lib.rs` | Library root: re-exports `protocol` and `client_registry` from daemon-protocol; declares `host`, `server`, `config`, `error`, `store`, `client`; `http` behind `http-api` |
| `host` | `src/host.rs` | `DaemonHost`: command dispatch, RBAC enforcement, persistence fan-out, recovery |
| `server` | `src/server.rs` | Unix socket server: JSON-line protocol, client handler loop, subscribe streaming, bounded read |
| `client` | `src/client.rs` | `DaemonClient`: typed client library for Unix socket communication |
| `config` | `src/config.rs` | `DaemonConfig`: socket path, max clients, default surface, data dir, persistence toggle |
| `error` | `src/error.rs` | `DaemonError`: Io, Serialization, Protocol, Runtime |
| `store/mod` | `src/store/mod.rs` | `DaemonStore` trait, `PersistedAuditEvent`, `noop_store()` |
| `store/sqlite` | `src/store/sqlite.rs` | `SqliteStore` (WAL, foreign keys, schema version 2), `NoopStore` |
| `protocol` | `src/protocol.rs` | Re-exports from daemon-protocol (backward compat) |
| `client_registry` | `src/client_registry.rs` | Re-exports from daemon-protocol (backward compat) |
| `http` | `src/http.rs` | HTTP/SSE transport (behind `http-api`): 14 axum routes, SSE streaming, auth header, bind validation |

## Behavior & Flows

### Client Connect → Register → Auth → Command Loop → SSE Fan-out

```
Client connects to Unix socket
  └─► handle_client() accepts connection, acquires semaphore permit
       └─► JSON-line read loop
            ├─► DeclareClient → returns ClientId, captured for connection
            ├─► Subscribe → ack Ok, enter streaming loop:
            │     ├─► receiver.recv() → filter by session_id → write RuntimeEvent
            │     └─► read_bounded_line() → dispatch further commands inline
            └─► Other commands → handle_command() → write response
```

Key behaviors:
- **Idle timeout**: 300s read timeout per connection (`server.rs:193`)
- **Max line**: 1 MiB per JSON frame (`server.rs:17`)
- **Max clients**: semaphore-limited (default 10)
- **Socket permissions**: 0o600 after bind (`server.rs:95`)
- **Subscribe**: long-lived; receives broadcast events filtered by session ID; further commands handled inline during streaming

### Startup Recovery

```
main.rs → host.recover_persisted_state()
  ├─► store.load_all_sessions()
  ├─► For each snapshot:
  │     ├─► Mark non-terminal tasks as Cancelled ("interrupted by daemon restart")
  │     ├─► Reconstruct SessionAccess from owner_client_id
  │     └─► runtime.hydrate_session(snapshot)
  └─► Record "daemon-recovery" audit event
```

### Persistence Fan-out

Lifecycle commands (CreateSession, SubmitTask, CancelTask, CancelActive, CloseSession) fire-and-forget persistence via `tokio::spawn(persistence_with_timeout(...))` with a 30s upper bound. A background event-listener task in `main.rs` persists snapshots on terminal events (TaskCompleted, TaskFailed, TaskCancelled) plus a periodic 5s sweep for broadcast overflow recovery.

### HTTP/SSE Transport (feature: `http-api`)

| Route | Method | Maps to |
|-------|--------|---------|
| `/health` | GET | `Health` |
| `/capabilities` | GET | `Capabilities` |
| `/clients/declare` | POST | `DeclareClient` |
| `/sessions` | GET | `ListSessions` |
| `/sessions` | POST | `CreateSession` |
| `/sessions/{id}/snapshot` | GET | `GetSnapshot` |
| `/sessions/{id}/tasks` | POST | `SubmitTask` |
| `/sessions/{id}/tasks/{tid}/cancel` | POST | `CancelTask` |
| `/sessions/{id}/cancel-active` | POST | `CancelActive` |
| `/sessions/{id}/events` | GET | Subscribe (SSE) |
| `/sessions/{id}/policy/approve` | POST | `ApprovePolicy` |
| `/sessions/{id}` | DELETE | `CloseSession` |
| `/sessions/persisted` | GET | `ListPersistedSessions` |
| `/sessions/persisted/{id}` | GET | `GetPersistedSnapshot` |

- Auth via `X-Eggsec-Client-Id` header (`http.rs:20`)
- Default bind: `127.0.0.1:9876` (loopback enforced unless `allow_public_bind`)
- SSE: `async-stream` + `futures::Stream`, `KeepAlive` default
- `CancellationToken` leaked via `Box::leak` for axum graceful shutdown (one per process)

## RBAC Model

### Permission Matrix

| Command | Public | DeclaredClient | Observer | Controller | Owner | Approver |
|---------|:------:|:--------------:|:--------:|:----------:|:-----:|:--------:|
| `Health`, `Capabilities` | ✓ | — | — | — | — | — |
| `DeclareClient`, `CreateSession`, `ListSessions`, `ListPersistedSessions`, `GetPersistedSnapshot` | — | ✓ | — | — | — | — |
| `GetSnapshot`, `Subscribe` | — | — | ✓ | ✓ | ✓ | ✓ |
| `SubmitTask`, `CancelTask`, `CancelActive` | — | — | ✗ | ✓ | ✓ | ✗ |
| `CloseSession` | — | — | ✗ | ✓ | ✓ | ✗ |
| `ApprovePolicy` (manual surface) | — | — | ✗ | ✓ | ✓ | ✓ |
| `ApprovePolicy` (strict surface) | — | — | ✗ | ✗ | ✓ | ✗ |

### Session Access Control

Three-tier resolution for `GetPersistedSnapshot` (`host.rs:814-882`):
1. In `session_access` + authorized (owner or allow-listed) → allow
2. In `session_access` + NOT authorized → deny immediately
3. NOT in `session_access` (recovered session) → check `snapshot.owner_client_id`:
   - Owner matches → allow
   - Owner present, doesn't match → deny
   - No owner (legacy) → allow

### Persisted Session Listing Policy

`ListPersistedSessions` (`host.rs:762-811`):
- `DaemonInternal` clients: see all sessions
- CLI/TUI clients: see only own sessions (owner match) + legacy sessions without owner

## Public API

### RuntimeCoreTypes

| Type | Location | Purpose |
|------|----------|---------|
| `DaemonHost` | `host.rs` | Command dispatch, RBAC, persistence |
| `DaemonClient` | `client.rs` | Unix socket client library |
| `DaemonConfig` | `config.rs` | Socket path, max clients, data dir, persistence |
| `DaemonStore` | `store/mod.rs` | Persistence trait (7 methods) |
| `SqliteStore` | `store/sqlite.rs` | SQLite implementation (WAL, schema version 2) |
| `NoopStore` | `store/sqlite.rs` | Test/disabled stub |
| `DaemonError` | `error.rs` | Io, Serialization, Protocol, Runtime |
| `run_server` | `server.rs` | Unix socket accept loop |
| `run_http_server` | `http.rs` | HTTP/SSE transport (feature-gated) |

### DaemonStore Trait Methods (`store/mod.rs:25-52`)

| Method | Async | Blocking | Purpose |
|--------|:-----:|:--------:|---------|
| `save_session_snapshot()` | ✓ | | Upsert snapshot |
| `load_session_snapshot()` | ✓ | | Load by ID |
| `load_all_sessions()` | ✓ | | Load all (recovery) |
| `record_audit_event()` | ✓ | | Append audit |
| `delete_session()` | ✓ | | Remove snapshot |
| `blocking_list_sessions()` | | ✓ | Summary listing |
| `blocking_get_snapshot()` | | ✓ | Snapshot retrieval |

## Integration Points

- **TUI attach mode**: `eggsec-tui` connects as a daemon client via `DaemonClient`, creates sessions, submits tasks, and streams events
- **CLI daemon commands**: `eggsec daemon history` and `eggsec daemon show <id>` use `ListPersistedSessions`/`GetPersistedSnapshot` via `DaemonClient`
- **Engine runtime_bridge**: `EggsecRuntimeExecutor` (behind `full-executor`) bridges runtime DTOs to engine dispatch
- **Python daemon-client**: provisional `eggsec.daemon` module uses `eggsec-daemon-protocol` types

## SQLite Schema (`store/sqlite.rs:10-31`)

```sql
session_snapshots (
    session_id TEXT PRIMARY KEY,
    snapshot_json TEXT NOT NULL,
    created_at_secs INTEGER NOT NULL
);

audit_events (
    audit_id INTEGER PRIMARY KEY AUTOINCREMENT,
    action TEXT NOT NULL,
    surface TEXT NOT NULL,
    outcome TEXT NOT NULL,
    client_id TEXT,
    session_id TEXT,
    created_at_secs INTEGER NOT NULL
);

schema_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
```

Schema version: `2` (stored in `schema_meta`). Migration refuses to load when stored version > current.

## Configuration (`config.rs:5-17`)

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `socket_path` | `String` | `/tmp/eggsec-daemon.sock` | Unix socket path |
| `max_clients` | `usize` | `10` | Concurrent connection limit |
| `default_surface` | `RuntimeSurface` | `Unknown` | Fallback for sessions without explicit surface |
| `data_dir` | `Option<String>` | `None` (→ `~/.local/share/eggsec/daemon/`) | SQLite database directory |
| `enable_persistence` | `bool` | `true` | Enable/disable snapshot persistence |

## Testing

- `daemon-protocol`: serialization round-trips for all `ClientCommand`, `ServerMessage`, `ErrorCode` variants; `command_permission_mapping_covers_all_variants` exhaustive check
- `daemon/host.rs`: command dispatch tests (health, capabilities, create/list/submit/snapshot/close); permission denial tests
- `daemon/server.rs`: Unix socket round-trips, subscribe event delivery, multi-subscriber fan-out, shutdown signal, invalid JSON handling
- `daemon/client.rs`: client library round-trips (health, create session, declare, close)
- `daemon/http.rs`: HTTP route tests, SSE delivery, auth enforcement, bind validation
- Local smoke test: `scripts/smoke-daemon-local.sh`

## Invariants & Gotchas

1. **No TUI deps in daemon**: architecture guard enforces zero `ratatity`/`crossterm` imports
2. **Transport/engine deps stay feature-gated**: `http-api` and `full-executor` are opt-in
3. **Persistence timeout**: all fire-and-forget persistence tasks bounded by `PERSISTENCE_TASK_TIMEOUT` (30s)
4. **ApprovePolicy is unsupported**: returns `ErrorCode::Unsupported` with audit trail
5. **Socket file cleanup**: `run_server` removes socket on exit; `main.rs` removes on startup
6. **CancellationToken leak in HTTP**: `Box::leak` for axum `&'static` requirement — one per process
7. **Subscribe handled at transport level**: `handle_command` returns error for Subscribe; actual streaming is in `server.rs` and `http.rs`
8. **Schema version mismatch**: refuses to load when stored > current; logs warning on downgrade migration
9. **Client ID tracking per connection**: Unix socket handler captures `client_id` from `DeclareClient` response; subsequent commands on same connection carry that ID

## See Also

- [runtime.md](runtime.md) — Runtime orchestrator that `DaemonHost` wraps
- [ui_model.md](ui_model.md) — View DTOs for daemon session/task state
- [overview.md](overview.md) — System-wide architecture, dependency map
- [runtime_bridge.md](runtime_bridge.md) — Engine-side surface conversion and approval
- [tui.md](tui.md) — TUI daemon attach mode
- [cli_commands.md](cli_commands.md) — CLI daemon/session/task commands

*Last verified against source: 2026-08-25*
