# Daemon & Daemon Protocol

Two crates provide the persistent session host and its IPC wire format: `eggsec-daemon-protocol` (shared types, RBAC) and `eggsec-daemon` (server, persistence, optional HTTP transport). Together they enable multi-client session management, background task execution, and durable state across restarts.

**Corrections (2026-10-24)**: `ClientCommand` 16 → **17** variants (new `GetTaskReport`), `ServerMessage` 15 → **16** variants (new `TaskReport`); `DaemonCapabilities` gained the `enabled_features` field (`None` = unknown, `Some(vec![])` = an engine is linked with no features enabled); a command the daemon does not know is refused with `ErrorCode::UnsupportedCommand` and the request id it was asked about, so the client's pending request is resolved instead of waiting out its timeout. `DAEMON_PROTOCOL_VERSION` stays at **2**, `ErrorCode` stays at 11 variants, and the HTTP route table stays at 14 routes.

**Corrections (2026-10-23)**: `ClientCommand` 15 → **16** variants (new `DescribeOperations`), `ServerMessage` 14 → **15** variants (new `Operations`); `eggsec-daemon-protocol` is now **4** source files (new `operations` module) and depends on `eggsec-runtime` + `eggsec-policy`; `DaemonCapabilities` gained the `operation_schema` flag; session-scoped audit events now record the session's runtime surface label instead of the blanket `"daemon"`. `ErrorCode` stays at 11 variants and the HTTP route table stays at 14 routes.

**Corrections (2026-10-06 re-verification)**: `rusqlite` dependency corrected 0.31 → **0.32** (cleared RUSTSEC-2024-0363); the `eggsec-daemon` dependency list was missing seven crates (`serde`, `tokio-util`, `tracing-subscriber`, `rustc-hash`, `thiserror`, `uuid`, `async-trait`); `full-executor` now also enables the engine's `cli` feature. All variant counts, permission mappings, route tables, socket/timeout constants, schema version, and `file:line` cites were re-checked against source and are current — no other changes.

## Role & Responsibilities

| Concern | Crate |
|---------|-------|
| Wire-format types (`ClientCommand`, `ServerMessage`, `ErrorCode`) | `eggsec-daemon-protocol` |
| RBAC client registry (`ClientRegistry`, `ClientKind`, `ClientRole`, `CommandPermission`) | `eggsec-daemon-protocol` |
| Unix socket server, command dispatch, event fan-out | `eggsec-daemon` |
| SQLite persistence, schema migration, startup recovery | `eggsec-daemon` |
| Optional HTTP/SSE transport | `eggsec-daemon` (`http-api` feature) |
| Optional engine executor (the GUI-serving daemon profile) | `eggsec-daemon` (`service` feature; `full-executor` is its deprecated alias) |

## Location & Feature Gating

| Crate | Path | Dependencies | Features |
|-------|------|-------------|----------|
| `eggsec-daemon-protocol` | `crates/eggsec-daemon-protocol/` | `eggsec-runtime` + `eggsec-policy` (no persistence, transport, or TUI deps) | None |
| `eggsec-daemon` | `crates/eggsec-daemon/` | `eggsec-runtime`, `eggsec-daemon-protocol`, `rusqlite 0.32 (bundled)`, `tokio`, `tokio-util`, `serde`, `serde_json`, `tracing`, `tracing-subscriber`, `rustc-hash`, `thiserror`, `uuid`, `async-trait`, `anyhow`, `clap`; optional `eggsec` | `http-api` (axum + async-stream + futures), `service` (the engine profile: `dep:eggsec` + `eggsec/cli` + the daemon's engine feature set), `full-executor` (deprecated alias for `service`) |

The `service` feature is the **profile**: the engine feature set a
GUI-serving daemon compiles. The engine owns the list
(`eggsec::config::daemon_profile::DAEMON_SERVICE_PROFILE`) and the daemon's
manifest must name exactly it. `full-executor` is kept as a deprecated alias
(`full-executor = ["service"]`) so existing invocations keep working. The
daemon's **runtime** flag `--full-executor` is a different mechanism: it
turns the compiled-in executor on, and a binary built without the cargo
feature accepts the flag and falls back to a no-op executor with a warning.

Architecture guards enforce:
- `eggsec-daemon` has no TUI dependencies (`ratatui`/`crossterm`)
- Transport deps (`axum`, `async-stream`, `futures`) are optional behind `http-api`
- Engine dep (`eggsec`) is optional behind the `service` profile
- Default dependencies: `eggsec-runtime` + `eggsec-daemon-protocol` only

## Architecture

### eggsec-daemon-protocol (4 source files)

| Module | File | Contents |
|--------|------|----------|
| `lib` | `src/lib.rs` | Re-exports `client_registry`, `operations`, and `protocol` modules |
| `protocol` | `src/protocol.rs` | `ClientCommand` (17 variants), `ServerMessage` (16 variants), `ErrorCode` (11 variants), `TransportKind` (4 variants), `DaemonCapabilities`, `TransportCapability`, `DaemonRequestContext`, `DAEMON_PROTOCOL_VERSION` (= 2; v2 adds `GetTaskResult`/`TaskResult`) |
| `client_registry` | `src/client_registry.rs` | `ClientKind` (8 variants), `ClientRole` (4 variants), `CommandPermission` (6 variants), `ClientInfo`, `ClientAccessRule`, `SessionAccess`, `ClientRegistry`, `check_permission()`, `command_permission()` |
| `operations` | `src/operations.rs` | `OperationDescription`, `TaskKindDescription`, `DescribeOperationsResponse`, `describe_operations()` — the read-only publication of the canonical operation catalog plus the derived request schema |

`eggsec-daemon-protocol` gained an `eggsec-policy` dependency (a pure semantic kernel: no Tokio, no I/O) so that the published operation descriptions are **read from** `eggsec_policy::ALL_OPERATION_METADATA` rather than transcribed. It still has no workspace dependency beyond `eggsec-runtime` and `eggsec-policy`.

#### ClientCommand — 17 variants (`protocol.rs:113`)

| # | Variant | Fields | Permission |
|---|---------|--------|-----------|
| 1 | `Health` | `request_id` | Public |
| 2 | `Capabilities` | `request_id` | Public |
| 3 | `DeclareClient` | `request_id`, `kind: ClientKind`, `label: Option<String>` | DeclaredClient |
| 4 | `CreateSession` | `request_id`, `surface: RuntimeSurface`, `scope: Option<SessionScope>`, `labels: Vec<String>` | DeclaredClient |
| 5 | `ListSessions` | `request_id` | DeclaredClient |
| 6 | `GetSnapshot` | `request_id`, `session_id: SessionId` | Observer |
| 7 | `GetTaskResult` | `request_id`, `session_id: SessionId`, `task_id: TaskId` | Observer |
| 8 | `SubmitTask` | `request_id`, `session_id: SessionId`, `request: RunRequest` | Controller |
| 9 | `CancelTask` | `request_id`, `session_id: SessionId`, `task_id: TaskId` | Controller |
| 10 | `CancelActive` | `request_id`, `session_id: SessionId` | Controller |
| 11 | `Subscribe` | `request_id`, `session_id: SessionId` | Observer |
| 12 | `CloseSession` | `request_id`, `session_id: SessionId` | Owner |
| 13 | `ApprovePolicy` | `request_id`, `session_id: SessionId`, `task_id: TaskId`, `approved: bool`, `reason: Option<String>` | Approver |
| 14 | `ListPersistedSessions` | `request_id` | DeclaredClient |
| 15 | `GetPersistedSnapshot` | `request_id`, `session_id: SessionId` | DeclaredClient |
| 16 | `DescribeOperations` | `request_id` | DeclaredClient |
| 17 | `GetTaskReport` | `request_id`, `session_id: SessionId`, `task_id: TaskId` | Observer |

#### ServerMessage — 16 variants (`protocol.rs:317`)

| # | Variant | Fields |
|---|---------|--------|
| 1 | `Ok` | `request_id` |
| 2 | `Error` | `request_id`, `code: ErrorCode`, `message: String` |
| 3 | `ClientDeclared` | `request_id`, `client_id: ClientId` |
| 4 | `SessionCreated` | `request_id`, `session_id: SessionId` |
| 5 | `Sessions` | `request_id`, `sessions: Vec<SessionSummary>` |
| 6 | `Snapshot` | `request_id`, `snapshot: SessionSnapshot` |
| 7 | `TaskResult` | `request_id`, `session_id: SessionId`, `task_id: TaskId`, `status: TaskStatus`, `outcome: Option<TaskOutcome>` |
| 8 | `TaskSubmitted` | `request_id`, `task_id: TaskId` |
| 9 | `Capabilities` | `request_id`, `capabilities: DaemonCapabilities` |
| 10 | `Health` | `request_id`, `status: String`, `version: String`, `protocol_version: u32` |
| 11 | `RuntimeEvent` | `session_id: SessionId`, `event: RuntimeEvent` |
| 12 | `SessionClosed` | `request_id` |
| 13 | `PersistedSessions` | `request_id`, `sessions: Vec<SessionSummary>` |
| 14 | `PersistedSnapshot` | `request_id`, `snapshot: Option<SessionSnapshot>` |
| 15 | `Operations` | `request_id`, `operations: DescribeOperationsResponse` |
| 16 | `TaskReport` | `request_id`, `session_id: SessionId`, `task_id: TaskId`, `report: serde_json::Value` |

`TaskResult` (Phase E WS5) is durable single-task retrieval: live snapshot
first, persisted snapshot fallback, `None` outcome while active. See
[docs/DAEMON_PARITY.md](../docs/DAEMON_PARITY.md) for reconnect semantics.

#### DaemonCapabilities

| Field | Type | Purpose |
|-------|------|---------|
| `runtime` | `RuntimeCapabilities` | Runtime-level capabilities (task kinds, cancellation, transports) |
| `transports` | `Vec<TransportCapability>` | Available daemon transports |
| `operation_schema` | `bool` | Whether this daemon can publish the operation catalog and the derived request schema via `ClientCommand::DescribeOperations` |
| `enabled_features` | `Option<Vec<String>>` | The engine features compiled into the connected daemon's build — `None` when the fact is unknown, `Some(vec![])` when an engine is linked with no features enabled |

`operation_schema` is `#[serde(default)]`, so an older client still decodes a newer daemon's capabilities and vice versa — the field defaults to `false` rather than failing the whole handshake. A client that sees `false` — or that receives `ErrorCode::UnsupportedCommand` from a daemon too old to know the command — must **degrade to a static form set** rather than fail.

`enabled_features` is `#[serde(default)]` as well, which keeps the field wire-additive in both directions: a daemon that predates it decodes as `None`, and a client that predates it ignores the field. It is a **build** fact rather than a runtime capability — it says which feature-gated operations the linked engine could run at all, so a client can tell "unavailable because the daemon was built without the feature" apart from "unavailable because the daemon does not advertise it".

The three states are distinct and a client MUST NOT collapse them:

| Value | Meaning |
|-------|---------|
| `None` | **Unknown** — the daemon links no engine, or predates the field |
| `Some(vec![])` | An engine is linked with **no** features enabled — a different, knowable fact |
| `Some(vec![…])` | The enabled feature names, in the engine's own registry order |

`None` is never "nothing is enabled", and `Some(vec![])` is never "unknown". The value is produced by the executor and captured once at host construction (`host.rs:95`, set from `host.rs:120`/`147`/`170`). The executor publishes the engine's own compile-time registry filtered by the policy bridge's enabled-feature snapshot (`crates/eggsec/src/runtime_bridge/executor.rs:147`) — the same snapshot the enforcement path evaluates against (`crates/eggsec/src/config/policy_decision.rs:379`), not a second table. A daemon built without the engine executor publishes `None`, because the fact is genuinely unknown rather than empty.

#### DescribeOperations — the operation/schema publication command

`ClientCommand::DescribeOperations { request_id }` → `ServerMessage::Operations { request_id, operations }`.

- **Read-only and additive**: it creates no session, mutates no task state, and takes part in no authorization decision. The payload is assembled from immutable registry data, so it is safe to serve concurrently and reproduces exactly.
- **Requires a declared client identity** — the same rule as the other daemon-wide reads such as `ListPersistedSessions` — but **no session and no session role**: the command carries no session id, so it is answerable before any session exists.
- **Carries** `DescribeOperationsResponse`: every canonical operation from the catalog (`OperationDescription`), every task-kind wire shape with its JSON Schema (`TaskKindDescription`), and the whole `TaskKind` schema in its adjacently-tagged wire shape (`{"kind": ..., "params": ...}`).
- **Descriptive only**: the published identity comes from the canonical `OperationMetadata` catalog and the parameter structure is mechanically derived from the canonical request DTOs (`eggsec-runtime`'s optional `schema` feature). Nothing in the published payload grants, narrows, or widens what the engine will run — it is a view of the operation set for frontends that render a form for an operation they can already invoke. The engine's own enforcement and validation remain authoritative.
- **Graceful degradation is mandatory**: a daemon that does not know the command answers `ErrorCode::UnsupportedCommand`, and the client MUST treat that as "schema unavailable" and fall back to a static form set.

The HTTP/SSE transport is unchanged: no new axum route was added for this command, so the route table below still has 14 routes.

#### GetTaskReport — the report read path

`ClientCommand::GetTaskReport { request_id, session_id, task_id }` → `ServerMessage::TaskReport { request_id, session_id, task_id, report }`.

- **Read-only and additive**: it creates no session, mutates no task state, and takes part in no authorization decision beyond the ordinary read-only permission check. It adds no required field and removes no variant, so `DAEMON_PROTOCOL_VERSION` stays at `2` — the version is bumped only on breaking wire-format changes (`protocol.rs:6-14`).
- **Permission**: `Observer` — the same read-only level as `GetTaskResult` (`client_registry.rs:76-78`). The report document is that task's result in report form, so it carries no wider access than the result read does.
- **What the daemon serves**: whatever the engine already produced. `DaemonHost` answers from `Runtime::report_for(&task_id)` (`host.rs:660`), which delegates to the executor behind the runtime (`crates/eggsec-runtime/src/runtime.rs:179`). The daemon does not derive, re-derive, or re-render anything.

Where the report comes from, end to end:

1. The engine derives the document from the completed task result through the **pure** bridge `eggsec::runtime_bridge::report::report_for_result` (`crates/eggsec/src/runtime_bridge/report.rs:54`). The bridge performs no I/O and reads no state, so the same result always yields the same document.
2. The engine keeps the derived document in the runtime's bounded report store (`crates/eggsec-runtime/src/report.rs:42`), reached through the executor's `report_for` (`crates/eggsec/src/runtime_bridge/executor.rs:159`). Retention is capped at `MAX_RETAINED_REPORTS` = 64, oldest-first evicted (`crates/eggsec-runtime/src/report.rs:29`).
3. The daemon serves it over `GetTaskReport` and the client reads it as JSON.

The **envelope contract is untouched**: `TaskResultEnvelope.payload` stays `json!({})` and `.artifacts` stays `vec![]`. The report body travels on its own path and is never folded into the envelope.

The report document is carried as `serde_json::Value` rather than as a typed report-model struct, so the protocol crate keeps **no report-model dependency**; the consumer deserializes the JSON into the canonical report model.

**No report available is not an error.** A task whose result has no report form simply has no report; the daemon answers `ErrorCode::TaskNotFound` with the request id echoed (`host.rs:667-671`), and the client turns that into "no report available" rather than a failure. A daemon too old to know the command answers `ErrorCode::UnsupportedCommand`, which the client degrades the same way. See [frontend_client.md](frontend_client.md) for the client's side of that contract.

The HTTP/SSE transport is unchanged here as well: no axum route was added for this command, so the route table below still has 14 routes.

#### ErrorCode — 11 variants (`protocol.rs:86`)

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

#### Unknown commands — `UnsupportedCommand` vs `InvalidRequest`

A client routes every response by request id: the pending request is registered under its id and the reader task hands the answer to whichever waiter owns that id. A refusal that carries an **empty** request id therefore resolves nobody — the pending request stays registered and the client waits out its response timeout (30s, `crates/eggsec-frontend-client/src/daemon.rs:375`) instead of learning that its command was refused. Every refusal the daemon sends about a command it parsed must echo the request id it was asked about.

The daemon classifies a line it could not deserialize as a command (`server.rs:95`) by a best-effort parse of the raw line, recovering the request id and the command type. `ClientCommand::command_names()` (`protocol.rs:291`) enumerates the discriminants this daemon knows, so the two refusals are distinguishable:

| The raw line | Refusal |
|--------------|---------|
| Names a command this daemon does not know | `ErrorCode::UnsupportedCommand`, with the request id the line carried |
| Names a command this daemon knows, but the payload does not fit | `ErrorCode::InvalidRequest`, with the request id the line carried |
| Is not JSON at all | `ErrorCode::InvalidRequest`, empty request id — there is nothing to recover |

A request id the daemon cannot read as a string is not recovered either: the refusal carries an empty request id rather than a wrong one.

This is what makes a **newer client talking to an older daemon** a degradable condition rather than a hang: the client sends a command the older daemon has never heard of, the older daemon answers `UnsupportedCommand` with the request id, and the client learns immediately that the command is unavailable. A client that adds a command must therefore be prepared for the peer to answer `UnsupportedCommand`, and must degrade rather than fail — see [frontend_client.md](frontend_client.md).

#### ClientKind — 8 variants (`client_registry.rs:11`)

`Cli`, `Tui`, `Gui`, `DaemonInternal`, `Mcp`, `Rest`, `Agent`, `Unknown` (default)

`Gui` is the desktop GUI frontend's kind (`client_registry.rs:14-23`). It is
**additive on the wire**: a daemon that predates the variant fails to
deserialize the `DeclareClient` command and rejects the client. That is a
fail-closed outcome, not a silent misclassification — an old daemon never
accepts a GUI it cannot identify. `DAEMON_PROTOCOL_VERSION` stays at `2`: the
`Gui` kind is carried inside the existing `DeclareClient` payload, so it adds
no protocol variant and no required field. The version is bumped only on
breaking wire-format changes (`protocol.rs:6-14`).

#### ClientRole — 4 variants (`client_registry.rs:31`)

`Owner`, `Controller`, `Observer`, `Approver`

#### CommandPermission — 6 variants (`client_registry.rs:43`)

`Public`, `DeclaredClient`, `Observer`, `Controller`, `Owner`, `Approver`

### eggsec-daemon (12 source files)

| Module | File | Purpose |
|--------|------|---------|
| `main` | `src/main.rs` | Binary entry point: CLI args (clap), store setup, host creation, shutdown signal, event persistence loop |
| `lib` | `src/lib.rs` | Library root: re-exports `protocol`, `client_registry`, and `operations` from daemon-protocol; declares `host`, `host_auth`, `host_persistence`, `server`, `config`, `error`, `store`, `client`; `http` behind `http-api` |
| `host` | `src/host.rs` | `DaemonHost`: command dispatch, recovery (facade; RBAC/persistence delegated) |
| `host_auth` | `src/host_auth.rs` | RBAC ownership/role helpers (`role_for_session`, `may_observe_session`) |
| `host_persistence` | `src/host_persistence.rs` | Persistence fan-out (`PERSISTENCE_TASK_TIMEOUT` = 30s at `:13`, `persistence_with_timeout` at `:29`, `record_audit_event_logged`) |
| `server` | `src/server.rs` | Unix socket server: JSON-line protocol, client handler loop, subscribe streaming, bounded read |
| `client` | `src/client.rs` | `DaemonClient`: typed client library for Unix socket communication |
| `config` | `src/config.rs` | `DaemonConfig`: socket path, max clients, default surface, data dir, persistence toggle |
| `error` | `src/error.rs` | `DaemonError`: Io, Serialization, Protocol, Runtime |
| `store/mod` | `src/store/mod.rs` | `DaemonStore` trait, `PersistedAuditEvent`, `noop_store()` |
| `store/sqlite` | `src/store/sqlite.rs` | `SqliteStore` (WAL, foreign keys, schema version 2), `NoopStore` |
| `client_registry` | (single source: `eggsec-daemon-protocol/src/client_registry.rs`, re-exported via `eggsec-daemon/src/lib.rs:1`) | **No orphan**: `host.rs`/`server.rs`/`client.rs` all resolve RBAC types from the protocol crate; the former duplicate `eggsec-daemon/src/client_registry.rs` has been deleted |
| `http` | `src/http.rs` | HTTP/SSE transport (behind `http-api`): 14 axum `.route()` calls (15 method rows; `GET`+`POST /sessions` share one route), SSE streaming, auth header, bind validation |

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
- **Idle timeout**: 300s read timeout per connection (`server.rs:254`)
- **Max line**: 1 MiB per JSON frame (`server.rs:17`, `MAX_LINE_LEN`)
- **Max clients**: semaphore-limited (default 10)
- **Socket permissions**: 0o600 after bind (`server.rs:152`)
- **Subscribe**: long-lived; receives broadcast events filtered by session ID; further commands handled inline during streaming

### Desktop GUI Submission and Event Flow

The desktop GUI is a **daemon-only** frontend: it holds no engine code and
performs no outbound HTTP. Every engine request travels over this Unix socket.

```
GUI window
  └─► connect(socket_path, ClientIdentity { kind: Gui, label: "eggsec-gui", request_prefix: "gui-" })
       ├─► DeclareClient { kind: Gui }        → ClientId   (a pre-Gui daemon rejects it: fail closed)
       ├─► Health                             → protocol_version must equal DAEMON_PROTOCOL_VERSION
       ├─► Capabilities                       → operation_schema must be true
       ├─► CreateSession { surface: DesktopGuiManual, scope: … }
       ├─► DescribeOperations                 → the published task kinds + request schemas
       └─► Subscribe                          → one subscription for the session's lifetime

GUI form submit
  └─► SubmitTask { request: RunRequest { task_kind: { kind, params }, surface: DesktopGuiManual, … } }
       └─► runtime bridge → approve_run_request_bundle() → dispatch → RuntimeEvent

RuntimeEvent ──► the GUI's single `eggsec://task-event` channel
                (task_upsert / logs / counters); the GUI's own state is bounded
```

The GUI never sets the scope or the surface from the webview: the surface is
`DesktopGuiManual`, and the scope is resolved by the daemon's runtime bridge
from the session. The GUI's session is therefore auditable as
`desktop-gui-manual` — see the Audit Trail section below.

See [desktop_gui.md](desktop_gui.md) for the GUI's own side of this flow.

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
| `/sessions/{id}/tasks/{tid}` | GET | `GetTaskResult` |
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
| `DeclareClient`, `CreateSession`, `ListSessions`, `ListPersistedSessions`, `GetPersistedSnapshot`, `DescribeOperations` | — | ✓ | — | — | — | — |
| `GetSnapshot`, `GetTaskResult`, `GetTaskReport`, `Subscribe` | — | — | ✓ | ✓ | ✓ | ✓ |
| `SubmitTask`, `CancelTask`, `CancelActive` | — | — | ✗ | ✓ | ✓ | ✗ |
| `CloseSession` | — | — | ✗ | ✓ | ✓ | ✗ |
| `ApprovePolicy` (manual surface) | — | — | ✗ | ✓ | ✓ | ✓ |
| `ApprovePolicy` (strict surface) | — | — | ✗ | ✗ | ✓ | ✗ |

The manual-approval arm of `check_permission` covers `TuiManual`, `CliManual`
**and `DesktopGuiManual`** (`client_registry.rs:194-207`). The desktop GUI's
surface is manual by classification in `eggsec-policy`, so the RBAC matrix and
the policy crate must agree about which surfaces are manual; leaving the GUI
out of the arm would make them disagree.

`ApprovePolicy` remains **unwired** in practice: the daemon answers it with
`ErrorCode::Unsupported` (`host.rs:901-933`), so no approval is granted either
way. The manual arm is the permission the command *would* need, not a grant.
The desktop GUI handles `PolicyDecisionRequired` defensively and offers no
approval affordance — see [desktop_gui.md](desktop_gui.md).

### Session Access Control

Three-tier resolution for `GetPersistedSnapshot` (`host.rs:987-1090`):
1. In `session_access` + authorized (owner or allow-listed) → allow
2. In `session_access` + NOT authorized → deny immediately
3. NOT in `session_access` (recovered session) → check `snapshot.owner_client_id`:
   - Owner matches → allow
   - Owner present, doesn't match → deny
   - No owner (legacy) → allow

### Persisted Session Listing Policy

`ListPersistedSessions` (`host.rs:935-985`):
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

- **TUI attach mode**: `eggsec-tui` connects as a daemon client via `eggsec-frontend-client`'s `DaemonRuntimeClient` (the shared frontend runtime-client contract), creates sessions, submits tasks, and streams events
- **CLI daemon commands**: `eggsec daemon history` and `eggsec daemon show <id>` use `ListPersistedSessions`/`GetPersistedSnapshot` via `DaemonClient`
- **Engine runtime_bridge**: `EggsecRuntimeExecutor` (behind the `service` cargo feature, whose deprecated `full-executor` alias selects the same profile) bridges runtime DTOs to engine dispatch
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

## Audit Trail

`PersistedAuditEvent.surface` records the **session's runtime surface label**, not a blanket `"daemon"` literal. The label set is the one the engine's `ExecutionSurface` produces: `cli-manual`, `cli-manual-strict`, `tui-manual`, `tui-manual-strict`, `desktop-gui-manual`, `mcp-server`, `rest-api`, `grpc-api`, `security-agent`, `ci`.

| Audit action | `surface` |
|--------------|-----------|
| `create-session` | the surface the session was created with |
| `submit-task`, `cancel-task`, `cancel-active`, `close-session`, `command-denied:<cmd>` | the session's current surface label |
| `declare-client`, `daemon-recovery`, `approve-policy` | `daemon` — these events have no session |

A desktop GUI session is therefore auditable as `desktop-gui-manual` and a TUI session as `tui-manual`, which is what makes the persisted trail usable for telling frontends apart after the fact. The label is read from the session's registered `RuntimeSurface` (`eggsec-runtime/src/request.rs:54`), so it always matches the surface the engine's runtime bridge evaluates.

## Configuration (`config.rs:6-26`)

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
- `daemon/server.rs`: Unix socket round-trips, subscribe event delivery, multi-subscriber fan-out, shutdown signal, invalid JSON handling; the unknown-command refusal classification (`server.rs:794`)
- `daemon/host.rs`: the report read path — a task with a retained report is answered with `TaskReport`, a task without one with `TaskNotFound` (`host.rs:1486`, `host.rs:1529`); the published enabled-feature set and its `None` = unknown state (`host.rs:1434`, `host.rs:1462`)
- `daemon/client.rs`: client library round-trips (health, create session, declare, close)
- `daemon/http.rs`: HTTP route tests, SSE delivery, auth enforcement, bind validation
- `daemon/host.rs`: the capability-honesty tests — the published list is the declared support ∩ the canonical list; a rejecting executor publishes nothing; the default declaration publishes the canonical set
- `crates/eggsec-daemon/tests/manual_capability_policy.rs`: the published list and the runtime's enforced list are the same object; the canonical capabilities are published; the hazard families are admitted by the daemon and refused by the policy, not by admission
- `crates/eggsec-daemon/tests/daemon_start_honesty.rs`: the pre-flight liveness check (live, stale, hung), the socket's ownership and removal, the concurrent-start race, the protocol version, and the `Capabilities` message shape
- `daemon/src/main.rs`: the daemon binary's start path refuses to displace a live daemon
- `eggsec-cli/src/daemon_cli.rs`: the CLI's start path — the readiness wait's bound, the "no success when the socket did not bind" property, the stale-socket start, and the shutdown-token cancellation
- Local smoke test: `scripts/smoke-daemon-local.sh`

## Invariants & Gotchas

1. **No TUI deps in daemon**: architecture guard enforces zero `ratatui`/`crossterm` imports
2. **Transport/engine deps stay feature-gated**: `http-api` and the `service` profile (deprecated alias `full-executor`) are opt-in
3. **Persistence timeout**: all fire-and-forget persistence tasks bounded by `PERSISTENCE_TASK_TIMEOUT` (30s)
4. **ApprovePolicy is unsupported**: returns `ErrorCode::Unsupported` with audit trail
5. **Socket file cleanup**: `run_server` removes socket on exit; `main.rs` removes on startup
6. **CancellationToken leak in HTTP**: `Box::leak` for axum `&'static` requirement — one per process
7. **Subscribe handled at transport level**: `handle_command` returns error for Subscribe; actual streaming is in `server.rs` and `http.rs`
8. **Schema version mismatch**: refuses to load when stored > current; logs warning on downgrade migration
9. **Client ID tracking per connection**: Unix socket handler captures `client_id` from `DeclareClient` response; subsequent commands on same connection carry that ID
10. **The published capability list is derived, not declared, and it is the admitted list**: it is the canonical capability list — derived from the task-kind set — intersected with the executor's declared support (`RuntimeTaskExecutor::supported_task_kinds()`), computed once at construction. The host then hands that list to the runtime as its enforced set, so publication and admission are the same object. A daemon that cannot execute anything therefore publishes **nothing** — the "advertises thirty, executes none" state is unrepresentable — and an executor that rejects everything must declare the empty set explicitly, because the trait's default is the canonical list. A *published* kind whose engine feature is off is a different matter: it is denied at dispatch with `FeatureMissing`, not `UnsupportedTaskKind`
11. **Readiness is a protocol check, never a log scrape**: a start path proves the daemon is up by connecting and exchanging `Health`, bounded by a named timeout. Nothing depends on the daemon's `"Daemon listening on …"` log line — a log format is not a contract
12. **The pre-flight liveness check belongs to the start paths**: before binding, a start path connects to the socket path and asks for `Health`. A daemon that answers owns the path: the start is refused, the socket is left untouched, and the command exits non-zero. The check is a **connection**, never a `stat` — a socket file left behind by a crashed daemon is indistinguishable from a live one by file existence. `run_server` still unlinks the socket path before binding, so the server-side hardening (refusing to bind over a *live* socket) is deferred to a daemon-subsystem milestone
13. **A refusal must carry the request id it was asked about**: the client routes responses by request id, so a refusal with an empty request id resolves no pending request and the client waits out its 30s response timeout instead of learning that its command was refused. The daemon recovers the request id and the command type from the raw line (`server.rs:95`)
14. **`enabled_features` is a build fact, and its absence is a fact too**: the daemon publishes the engine's compile-time feature registry filtered by the policy bridge's enabled-feature set. `None` means the daemon links no engine — the answer is unknown; `Some(vec![])` means an engine is linked with nothing enabled. The two are different facts and a client must not collapse them
15. **The report read path is separate from the task envelope**: `TaskResultEnvelope.payload` stays `json!({})` and `.artifacts` stays `vec![]`. The report document travels on its own command, is produced by the engine from the completed result, and is retained in a bounded store — see [runtime.md](runtime.md)

## See Also

- [runtime.md](runtime.md) — Runtime orchestrator that `DaemonHost` wraps
- [frontend_client.md](frontend_client.md) — Frontend-neutral runtime-client contract the TUI and the GUI bind to
- [desktop_gui.md](desktop_gui.md) — The desktop GUI, the daemon-only frontend
- [ui_model.md](ui_model.md) — View DTOs for daemon session/task state
- [overview.md](overview.md) — System-wide architecture, dependency map
- [runtime_bridge.md](runtime_bridge.md) — Engine-side surface conversion and approval
- [tui.md](tui.md) — TUI daemon attach mode
- [cli_commands.md](cli_commands.md) — CLI daemon/session/task commands

*Last verified against source: 2026-10-24 (`GetTaskReport`/`TaskReport` command and response, `DaemonCapabilities.enabled_features` and its `None` = unknown semantics, the unknown-command refusal with the request id echoed); earlier passes 2026-10-23 (`ClientKind::Gui` added — 7 → 8 variants; the manual-approval arm of `check_permission` now covers `DesktopGuiManual`; the desktop GUI's submission and event flow; DescribeOperations command + Operations response, `DaemonCapabilities.operation_schema`, session-scoped audit surface labels, daemon-protocol dependency set and source-file count); earlier passes 2026-08-25 / 2026-09-22 / 2026-09-25 / 2026-10-06 (full re-verification: variant counts, permission mapping, HTTP route table, socket/timeout constants, SQLite schema v2, dependency set, all `file:line` cites)*
