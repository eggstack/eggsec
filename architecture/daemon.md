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
| `CloseSession` | `close-session` | Yes (final snapshot with closed=true + cancelled tasks; preserves history — does NOT delete the session) |
| `DeclareClient` | `declare-client` | No (audit only) |
| `ApprovePolicy` | `approve-policy` | No (audit only, unsupported) |
| Permission denied | `command-denied:{discriminant}` | No (audit only) |

All persistence operations are guarded by `DaemonConfig::enable_persistence`. When disabled, writes are skipped silently.

## Startup Recovery

`DaemonHost::recover_persisted_state()` runs at daemon startup:

1. Loads all snapshots from `DaemonStore::load_all_sessions()`
2. Marks any non-terminal tasks (`Running`, `Queued`) as `Cancelled` with reason `"interrupted by daemon restart"`
3. Hydrates each snapshot into the runtime via `Runtime::hydrate_session()`
4. Populates `session_access` from `snapshot.owner_client_id` for recovered sessions
5. Records a `daemon-recovery` audit event with recovery counts

Failed session recoveries are logged at warn level and skipped. Active tasks are **never auto-resumed** — they are interrupted and must be resubmitted by clients. The `Cancelled` rewrite is informational only: the runtime's `hydrate_session` preserves only completed task records, so the rewrite is not strictly required for correctness but documents the recovery semantics for downstream consumers.

## Session Ownership & Access Control

### Owner Persistence

Each `SessionSnapshot` and `SessionSummary` includes an optional `owner_client_id: Option<ClientId>` field, set at session creation time via `Runtime::set_session_owner()`. This field is persisted to disk as part of the snapshot JSON and survives daemon restarts.

On `CreateSession`, the daemon stores `SessionAccess` in memory AND calls `set_session_owner()` so the owner is included in the persisted snapshot. On recovery, `session_access` is reconstructed from `snapshot.owner_client_id`.

### Access Control Model

Three-tier access control for `GetPersistedSnapshot`:

1. **In `session_access` + authorized** (owner or allow-listed) → allow
2. **In `session_access` + NOT authorized** → deny immediately (no fallback)
3. **NOT in `session_access`** (recovered session) → check `snapshot.owner_client_id`:
   - Owner matches → allow
   - Owner present but doesn't match → deny
   - No owner (legacy) → allow

For `ListPersistedSessions`, elevated client kinds (`Cli`, `Tui`, `DaemonInternal`) see all sessions. Non-elevated clients see only sessions where `owner_client_id` matches their own. Sessions without owner info are included (legacy compatibility).

### Backward Compatibility

The `owner_client_id` field uses `#[serde(default)]` for deserialization, ensuring existing persisted snapshots that lack the field are loaded without error (owner defaults to `None`).

### Persisted-Session Listing Policy

`ListPersistedSessions` applies different visibility rules based on client kind:

- **CLI/TUI clients** (`Cli`, `Tui`): See only sessions where `owner_client_id` matches their own client ID. Sessions without owner info are included for backward compatibility.
- **DaemonInternal clients**: See all sessions regardless of owner. This enables daemon-level history inspection and administrative tooling.

## Schema Migration

`SqliteStore::migrate()` (`store/sqlite.rs`) is version-aware:

- Reads the stored `schema_version` from `schema_meta` if present.
- Compares against the compile-time `SCHEMA_VERSION` (current: `2`).
- **Refuses** to load when the stored version is newer than the current version, returning an error from `SqliteStore::new`. This prevents silent data corruption on downgrade.
- Logs a warning when migrating an older stored version.
- Always rewrites the stored `schema_version` to the current value after the migration step.

When `create_dir_all` for the data directory fails, or `SqliteStore::new` fails for any reason (locked file, invalid path, schema version mismatch), `main.rs` falls back to `NoopStore` and logs a `tracing::warn!`. The daemon continues running with persistence disabled; this degradation is observable in logs but not in `DaemonCapabilities` (operators must consult logs to confirm persistence mode).

## Configuration

`DaemonConfig` (`config.rs`):

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `data_dir` | `Option<String>` | `None` (implies `~/.local/share/eggsec/daemon/`) | Directory for SQLite database |
| `enable_persistence` | `bool` | `true` | Enable/disable snapshot persistence |

When `enable_persistence` is `false`, the daemon uses `NoopStore` behavior and recovery is a no-op.

## Capabilities

`RuntimeCapabilities` reflects the daemon's actual execution capacity:

| Executor Mode | Capabilities | Task Kinds |
|---------------|-------------|------------|
| Real (`--full-executor`) | `RuntimeCapabilities::full()` | All 29 task kinds |
| Conservative (default) | `RuntimeCapabilities::conservative()` | Safe subset only — excludes hazardous task families (stress, packet, wireless-deauth, postex, c2, evasion) unless lab mode is configured |
| No-op (no `full-executor`) | `RuntimeCapabilities::noop()` | Empty (no task kinds advertised) |

Capabilities are set per-session at creation time via `RuntimeConfig`. Clients can discover capabilities via the `Capabilities` command or `GET /capabilities` HTTP endpoint. The daemon does not advertise task kinds it cannot execute.

Strict daemon execution (automated surfaces such as REST, MCP, gRPC, agent) currently requires resolvable explicit scope metadata — `LoadedScope::is_explicit_manifest()` must be true. Permissive manual surfaces allow `LoadedScope::default_empty()`.

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

## Local Smoke Test

`scripts/smoke-daemon-local.sh` is the canonical local-only lifecycle test for the daemon. It:

- Uses an ephemeral socket path and a temporary workspace (`mktemp -d`); no public network exposure.
- Pre-builds the daemon and CLI binaries into the temp workspace to avoid `cargo run` recompile noise leaking into assertions.
- Verifies daemon start, health, client declaration, session create/list/snapshot, observer-deny + owner-allow posture, persisted history/show, event stream subscription, and graceful SIGTERM shutdown.

Run with:

```bash
bash scripts/smoke-daemon-local.sh                 # default ephemeral socket
bash scripts/smoke-daemon-local.sh /path/to/socket # custom socket path
```

## Dependencies

- `rusqlite = "0.31"` (bundled SQLite) in `eggsec-daemon/Cargo.toml`
- `serde_json` for snapshot serialization
- `async_trait` for the `DaemonStore` trait

## Transport Abstraction

The noop daemon mode operates at the protocol/session level only — it handles session creation, task queuing, and cancellation without executing any tasks. The real executor mode (`--full-executor` / `full-executor` feature) adds actual task dispatch via `EggsecRuntimeExecutor`.

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
| `peer` | `Option<String>` | Peer address (for TCP/HTTP transports) |
| `transport` | `TransportKind` | Which transport the request arrived on |

`DaemonHost::handle_command()` accepts `DaemonRequestContext` instead of a bare client ID, ensuring transport provenance is available for audit and policy decisions.

### ErrorCode Enum

```rust
pub enum ErrorCode {
    InvalidRequest,
    SessionNotFound,
    TaskNotFound,
    TaskAlreadyCompleted,
    UnsupportedCommand,
    Internal,
    PermissionDenied,
    InvalidSurface,
    ClientNotDeclared,
    Unsupported,
    InvalidState,
}
```

### DAEMON_PROTOCOL_VERSION

```rust
pub const DAEMON_PROTOCOL_VERSION: u32 = 1;
```

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
| `bind_addr` | `String` | `127.0.0.1:9876` | Bind address (loopback enforced unless overridden) |
| `require_auth` | `bool` | `false` | Require `X-Eggsec-Client-Id` header on every request |
| `allow_public_bind` | `bool` | `false` | Allow non-loopback bind addresses (emits warning) |

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

## Signal Handling

The daemon handles `SIGINT` and `SIGTERM` for graceful shutdown (`main.rs` installs both via `tokio::signal::unix::signal(SignalKind::terminate())` plus `tokio::signal::ctrl_c()`):

- `SIGINT` (Ctrl+C) and `SIGTERM` both trigger `CancellationToken::cancel()`.
- The server loop exits cleanly and `run_server` removes the socket file before returning.
- Both signals are tested by the local smoke script (`scripts/smoke-daemon-local.sh`).
