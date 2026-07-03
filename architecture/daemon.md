# Daemon Persistence & Transport

The `eggsec-daemon` crate provides durable session persistence via a SQLite-backed store, enabling recovery across daemon restarts and historical session inspection. It also provides a transport abstraction layer with pluggable client connectivity.

## Persistence Layer

### `DaemonStore` Trait (`store/mod.rs`)

Async trait defining the persistence contract:

| Method | Purpose |
|--------|---------|
| `save_session_snapshot()` | Upsert a session snapshot (replaces existing) |
| `load_session_snapshot()` | Load a single session by ID |
| `load_all_sessions()` | Load all persisted snapshots (for recovery) |
| `record_audit_event()` | Append an audit event |
| `delete_session()` | Remove a session snapshot |
| `blocking_list_sessions()` | Synchronous summary listing (for `spawn_blocking`) |
| `blocking_get_snapshot()` | Synchronous snapshot retrieval |

### Implementations

| Store | Description |
|-------|-------------|
| `SqliteStore` | Production implementation. WAL mode, foreign keys enabled. Schema version tracked in `schema_meta` table. |
| `NoopStore` | Test stub. All writes are no-ops, all reads return empty. |

### SQLite Schema (`store/sqlite.rs`)

Three tables:

```sql
session_snapshots (
    session_id TEXT PRIMARY KEY,
    snapshot_json TEXT NOT NULL,       -- serialized SessionSnapshot
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
    value TEXT NOT NULL                -- tracks schema_version
);
```

Snapshots are stored as JSON via `serde_json`. The `SessionSnapshot` type (from `eggsec-runtime`) includes a `generation` field for optimistic concurrency tracking.

## Lifecycle Persistence

Snapshots are written at these `DaemonHost` command handler points (fire-and-forget via `tokio::spawn`):

| Command | Audit Action | Snapshot Saved |
|---------|-------------|----------------|
| `CreateSession` | `create-session` | Yes |
| `SubmitTask` | `submit-task` | Yes |
| `CancelTask` | `cancel-task` | Yes |
| `CancelActive` | `cancel-active` | Yes |
| `CloseSession` | `close-session` | No (audit only) |
| `DeclareClient` | `declare-client` | No (audit only) |
| `ApprovePolicy` | `approve-policy` | No (audit only, unsupported) |
| Permission denied | `command-denied:{discriminant}` | No (audit only) |

All persistence operations are guarded by `DaemonConfig::enable_persistence`. When disabled, writes are skipped silently.

## Startup Recovery

`DaemonHost::recover_persisted_state()` runs at daemon startup:

1. Loads all snapshots from `DaemonStore::load_all_sessions()`
2. Marks any non-terminal tasks (`Running`, `Queued`) as `Cancelled` with reason `"interrupted by daemon restart"`
3. Hydrates each snapshot into the runtime via `Runtime::hydrate_session()`
4. Records a `daemon-recovery` audit event with recovery counts

Failed session recoveries are logged at warn level and skipped. Active tasks are never auto-resumed — they are interrupted and must be resubmitted by clients.

## Configuration

`DaemonConfig` (`config.rs`):

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `data_dir` | `Option<String>` | `None` (implies `~/.local/share/eggsec/daemon/`) | Directory for SQLite database |
| `enable_persistence` | `bool` | `true` | Enable/disable snapshot persistence |

When `enable_persistence` is `false`, the daemon uses `NoopStore` behavior and recovery is a no-op.

## Protocol Extensions

Two `ClientCommand` variants and corresponding `ServerMessage` responses support persisted state queries:

| Command | Permission | Response | Purpose |
|---------|-----------|----------|---------|
| `ListPersistedSessions` | `DeclaredClient` | `PersistedSessions { sessions: Vec<SessionSummary> }` | List all stored session summaries |
| `GetPersistedSnapshot { session_id }` | `DeclaredClient` | `PersistedSnapshot { snapshot: Option<SessionSnapshot> }` | Retrieve full snapshot by ID |

Both use `spawn_blocking` to avoid blocking the async runtime on SQLite I/O.

## CLI Commands

Two `daemon` subcommands expose persisted state inspection:

| Command | Description |
|---------|-------------|
| `eggsec daemon history [--json]` | Lists all persisted sessions with surface, active task count, and completed task count |
| `eggsec daemon show <session-id> [--json]` | Shows full snapshot details: surface, scope, generation, task list with statuses |

Both connect to the daemon via Unix socket and use `ListPersistedSessions` / `GetPersistedSnapshot` protocol commands.

## Dependencies

- `rusqlite = "0.31"` (bundled SQLite) in `eggsec-daemon/Cargo.toml`
- `serde_json` for snapshot serialization
- `async_trait` for the `DaemonStore` trait

## Transport Abstraction

The daemon supports multiple transport layers for client connectivity, declared via `TransportKind` and advertised through `DaemonCapabilities`.

### Transport Types (`protocol.rs`)

| Type | Description | Status |
|------|-------------|--------|
| `TransportKind::UnixSocket` | Unix domain socket (JSON-line protocol) | Default, built-in |
| `TransportKind::LoopbackHttp` | HTTP REST + SSE via `axum` | Feature-gated (`http-api`) |
| `TransportKind::WebSocket` | WebSocket transport | Deferred (not implemented) |
| `TransportKind::Grpc` | gRPC transport | Deferred (not implemented) |

### Request Context

`DaemonRequestContext` carries per-request metadata through the handler pipeline:

| Field | Type | Purpose |
|-------|------|---------|
| `client_id` | `ClientId` | Identifying the calling client |
| `peer` | `Option<SocketAddr>` | Peer address (for TCP/HTTP transports) |
| `transport` | `TransportKind` | Which transport the request arrived on |

`DaemonHost::handle_command()` accepts `DaemonRequestContext` instead of a bare client ID, ensuring transport provenance is available for audit and policy decisions.

### Capabilities Advertisement

`DaemonCapabilities` is returned in `ServerMessage::Capabilities`:

```rust
pub struct DaemonCapabilities {
    pub runtime: RuntimeCapabilities,
    pub transports: Vec<TransportCapability>,
}
```

`TransportCapability` describes a single available transport (kind, address, supported features). Clients use this to discover which transports the daemon supports.

### HTTP/SSE Transport (`http.rs`, feature-gated `http-api`)

| Property | Value |
|----------|-------|
| Feature flag | `http-api` (on `eggsec-daemon`) |
| Optional deps | `axum`, `async-stream`, `futures` |
| Bind default | Loopback only (`127.0.0.1`) |
| Public bind | Requires explicit config; emits warning |
| Enforcement profile | `McpStrict` (noninteractive, no manual overrides) |
| Routes | 12 HTTP routes mapping 1:1 to `ClientCommand` variants |
| SSE endpoint | Real-time session event streaming |

The HTTP server validates that bind addresses are loopback by default. Explicit non-loopback binds (e.g., `0.0.0.0`) require configuration and produce a startup warning. This prevents accidental exposure of the daemon on public interfaces.

```bash
# Build daemon with HTTP transport
cargo build --release -p eggsec-daemon --features http-api
```

### `HttpConfig`

Configuration for the HTTP/SSE server:

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `bind_address` | `SocketAddr` | `127.0.0.1:0` | Bind address (loopback enforced unless overridden) |
| `require_loopback` | `bool` | `true` | Enforce loopback-only binds |

## Audit Events

`PersistedAuditEvent` records security-relevant daemon actions with:

| Field | Description |
|-------|-------------|
| `action` | Event type (e.g., `create-session`, `submit-task`, `command-denied:submit-task`) |
| `surface` | Execution surface (`daemon`) |
| `outcome` | Result (`allow`, `denied`, `unsupported`, `recovered`) |
| `client_id` | Initiating client (if applicable) |
| `session_id` | Target session (if applicable) |
| `timestamp_secs` | Unix timestamp |

Audit events are appended to the `audit_events` table and are not pruned.
