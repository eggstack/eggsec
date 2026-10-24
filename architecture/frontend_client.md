# eggsec-frontend-client Architecture

Frontend-neutral runtime-client contract shared by Eggsec's interactive frontends. It owns the small trait a frontend uses to drive the Eggsec runtime, plus the two reference implementations — one that wraps an in-process `Runtime`, one that speaks the daemon's Unix-socket JSON-lines protocol.

The crate exists so a second interactive frontend (the desktop GUI) can reuse the exact contract the terminal UI already binds to without depending on `eggsec-tui` — and therefore without inheriting `ratatui`, `crossterm`, or any other terminal-frontend dependency.

The crate carries **no frontend's identity**. The kind recorded in the daemon's client registry, the label the daemon shows in its listings, and the namespace for request ids are all the caller's to choose, through `ClientIdentity`.

## Role & Responsibilities

| Concern | Owner |
|---------|-------|
| The `RuntimeClient` contract between a frontend and a runtime backend | `eggsec-frontend-client` |
| Embedded (in-process) backend implementation | `eggsec-frontend-client` (`EmbeddedRuntimeClient`) |
| Daemon-backed (Unix socket) backend implementation | `eggsec-frontend-client` (`DaemonRuntimeClient`) |
| The runtime itself, its task lifecycle, and its DTOs | `eggsec-runtime` |
| The wire protocol the daemon backend speaks | `eggsec-daemon-protocol` |
| Enforcement, policy, and dispatch | The engine — **not** this crate |

## Location & Feature Gating

| Crate | Path | Dependencies | Features |
|-------|------|-------------|----------|
| `eggsec-frontend-client` | `crates/eggsec-frontend-client/` | `eggsec-runtime`, `eggsec-daemon-protocol`, `tokio` (`rt`, `sync`, `time`, `io-util`, `net`), `serde_json`, `thiserror`, `tracing`, `rustc-hash` | None |

There are no features: the crate is always what it is. The Tokio feature set is deliberately narrow — `rt` (spawn), `sync` (oneshot/mpsc/Mutex/RwLock), `time` (request timeout), `io-util` (lines/write/flush), `net` (`UnixStream`). No `rt-multi-thread`, no `macros`.

### Boundary rule

The crate depends on **no frontend framework** (no `ratatui`, no `crossterm`) and **not on the engine** (no `eggsec` crate). This is enforced structurally by architecture guard 153 (`scripts/check-architecture-guards.sh`): the manifest may not name `eggsec`, `eggsec-tui`, `ratatui`, or `crossterm`, and no source file in `src/` may reference `ratatui`, `crossterm`, `eggsec_tui`, or `eggsec::`.

The rule is the point of the crate. A frontend that depends on it inherits a runtime contract and nothing else; if it pulled in the engine, every additional frontend would drag the whole composition root (and its dependency closure) behind it.

## Architecture (3 source files)

| Module | File | Purpose |
|--------|------|---------|
| `lib` | `src/lib.rs` | The `RuntimeClient` trait, `RuntimeClientFuture`, `RuntimeEventReceiverHandle`, module declarations, and public re-exports |
| `embedded` | `src/embedded.rs` | `EmbeddedRuntimeClient` — wraps an in-process `Arc<Runtime>` |
| `daemon` | `src/daemon.rs` | `DaemonRuntimeClient`, `DaemonClientError` — Unix-socket JSON-lines client |

## Public API

| Type | Location | Purpose |
|------|----------|---------|
| `RuntimeClient` | `src/lib.rs:66` | The trait a frontend binds to. `Send + Sync + 'static`. |
| `RuntimeClientFuture<T>` | `src/lib.rs:37` | `Pin<Box<dyn Future<Output = Result<T, String>> + Send + 'static>>` — the trait's error type is deliberately coarse (`String`), because that is the contract the existing frontends already bind to. |
| `RuntimeEventReceiverHandle` | `src/lib.rs:92` | Frontend-neutral handle over a `tokio::sync::mpsc::UnboundedReceiver<RuntimeEvent>`; `recv()` / `try_recv()`. |
| `ClientIdentity` | `src/lib.rs:46` | The caller's own client identity: `kind`, `label`, `request_prefix`. `request_id(counter)` mints an id in that namespace (`src/lib.rs:60`). |
| `EmbeddedRuntimeClient` | `src/embedded.rs:11` | In-process backend. `new(Arc<Runtime>)`, `runtime()`. |
| `DaemonRuntimeClient` | `src/daemon.rs:71` | Daemon backend. `connect(socket_path, identity)`, `declare_client()`, `subscribe_events()`, plus the daemon-only `describe_operations()` / `get_task_report()` / `health()` / `daemon_capabilities()`. `Clone`. |
| `DaemonClientError` | `src/daemon.rs:41` | `Io`, `Serialization`, `Protocol(String)`, `ConnectionClosed`. Publicly exported. |

### The `RuntimeClient` trait (8 methods)

| Method | Returns | Notes |
|--------|---------|-------|
| `capabilities()` | `RuntimeCapabilities` | Daemon backend reads `ServerMessage::Capabilities`; embedded backend returns the runtime default. |
| `create_session(surface, scope, labels)` | `SessionId` | `surface` is a `RuntimeSurface` — this is how a frontend declares which surface it is (see [runtime_bridge.md](runtime_bridge.md)). |
| `list_sessions()` | `Vec<SessionSummary>` | |
| `snapshot(session_id)` | `SessionSnapshot` | |
| `submit(session_id, request)` | `TaskId` | `request` is a `RunRequest`. |
| `cancel(session_id, task_id)` | `()` | |
| `cancel_active(session_id)` | `()` | |
| `subscribe(session_id)` | `RuntimeEventReceiverHandle` | |

## The Two Implementations

### `EmbeddedRuntimeClient`

Wraps an `Arc<Runtime>` and calls it directly. It is a thin adapter — it forwards to the runtime's own methods and maps errors to `String`. No sockets, no serialization, no daemon.

### `DaemonRuntimeClient`

Speaks the daemon's Unix-socket JSON-lines protocol using `eggsec-daemon-protocol`'s `ClientCommand`/`ServerMessage` types.

- `connect(socket_path, identity)` splits the socket, spawns **one** reader task, and registers the pending-response and event-channel registries. The `identity` argument is the caller's `ClientIdentity`: the crate cannot pick a kind, a label, or a request-id namespace on the caller's behalf (`src/daemon.rs:82-85`).
- `send_command()` writes one JSON line and awaits the matching response through a `oneshot` channel, bounded by a 30s timeout (`src/daemon.rs:375`). Responses are routed by request id: the reader task hands each answer to the waiter registered under that id, so a refusal that omits the request id resolves nobody and the caller waits out the timeout instead of learning that its command was refused. The daemon therefore echoes the request id on every refusal it sends about a command it parsed — see [daemon.md](daemon.md).
- `subscribe_events()` registers a channel in the fan-out list; the reader task clones each `RuntimeEvent` out to every registered sender and prunes senders whose receiver has been dropped.
- Request ids are minted by an `AtomicU64` counter in the caller's namespace — `ClientIdentity::request_id()` (`src/lib.rs:60`) formats `{request_prefix}{counter}` verbatim, so a caller that wants a separator includes it in the prefix itself (`"gui-"` yields `gui-0`).

The client is declared to the daemon with the caller's own identity. The TUI passes `ClientKind::Tui` / `eggsec-tui` / `tui-`; the desktop GUI passes `ClientKind::Gui` / `eggsec-gui` / `gui-`. Nothing about either frontend is hardcoded here.

### The four daemon-only inherent methods

`DaemonRuntimeClient` carries four methods that have no embedded counterpart:

| Method | Returns | Purpose |
|--------|---------|---------|
| `describe_operations()` (`src/daemon.rs:233`) | `DescribeOperationsResponse` | The daemon's published operation catalog and per-task-kind request schemas. |
| `get_task_report()` (`src/daemon.rs:257`) | `Result<Option<serde_json::Value>>` | The report document for one completed task. `Ok(None)` means no report is available. |
| `health()` (`src/daemon.rs:290`) | `DaemonHealth` | The daemon's liveness and its advertised `protocol_version`. |
| `daemon_capabilities()` (`src/daemon.rs:319`) | `DaemonCapabilities` | The daemon's advertised capabilities, including the `operation_schema` flag and the `enabled_features` set. |

They are **inherent methods, not `RuntimeClient` trait methods**. The trait's eight methods are unchanged, and deliberately so: the embedded backend has no daemon to ask. Adding `describe_operations()` to the trait would force every embedded implementation to synthesize a catalog it has no source for. Keeping them inherent lets a daemon-backed frontend ask the daemon questions that only a daemon can answer, without widening the contract that the embedded backend must satisfy.

Note that `RuntimeClient::capabilities()` already exists for the embedded path; `daemon_capabilities()` is the daemon's own answer to the same question, and the GUI uses the latter because it needs the raw `operation_schema` flag rather than the trait's flattened view.

### `get_task_report()` — the report read path and its degradation contract

`get_task_report(session_id, task_id)` asks the daemon for one completed task's report document. The report is carried as JSON, so the client returns `serde_json::Value` and the caller deserializes it into the canonical report model.

The method distinguishes two outcomes, and only one of them is a failure:

| Outcome | Meaning |
|---------|---------|
| `Ok(Some(report))` | The daemon has a report document for that task. |
| `Ok(None)` | **No report is available** — either the daemon has none for that task, or the daemon is too old to know the command. |
| `Err(e)` | A genuine failure: a transport error, or a protocol answer that is neither a report nor a recognised "unavailable" refusal. |

`Ok(None)` covers two different daemon states, and the client maps both to it: `ErrorCode::TaskNotFound` (the daemon knows the command and has no report for that task) and `ErrorCode::UnsupportedCommand` (the daemon is too old to know `GetTaskReport` at all). A client MUST NOT treat either as a failure — a client with no report available still works, it simply cannot export that task. Anything else that arrives as `ErrorCode` is a real error and is returned as one.

The `UnsupportedCommand` degradation is what makes a **newer client against an older daemon** a survivable condition rather than a hang: the daemon recovers the request id from the raw line, refuses the unknown command with `ErrorCode::UnsupportedCommand` and that request id, and the client's pending request is resolved immediately. A refusal that omitted the request id would leave the client waiting out its 30s response timeout instead.

### Where the facts behind these methods live

The two facts a daemon-backed frontend needs beyond the trait — the daemon's **enabled-feature set** and a task's **report document** — are declared on the **runtime** crate's `RuntimeTaskExecutor` trait, as **defaulted** methods:

| Method | Default | Meaning |
|--------|---------|---------|
| `RuntimeTaskExecutor::enabled_features()` (`crates/eggsec-runtime/src/runtime.rs:89`) | `None` | The engine features compiled into the executor's build. `None` = unknown; `Some(vec![])` = an engine is linked with no features enabled. |
| `RuntimeTaskExecutor::report_for(task_id)` (`crates/eggsec-runtime/src/runtime.rs:100`) | `None` | The report document for a completed task. `None` = no report for this task. |

Both are **defaulted**, so every existing executor keeps compiling and keeps its current behaviour, and a host that links no engine publishes `None` rather than guessing. The `eggsec-frontend-client` crate's own trait surface is **unchanged** by this: `RuntimeClient` still has its eight methods, and these two remain facts the *daemon* publishes and this crate reads over the wire. See [runtime.md](runtime.md) for the runtime side and [daemon.md](daemon.md) for the daemon side.

### The `Arc<DaemonConnection>` sharing requirement

`DaemonRuntimeClient` is `Clone`, and every clone shares one `Arc<DaemonConnection>` (`src/daemon.rs:72`). The connection owns everything that must be shared:

- the socket's **write half** (behind a `tokio::sync::Mutex`, so concurrent sends cannot interleave a line),
- the **pending-response registry** — a map from `request_id` to the `oneshot::Sender` awaiting that response,
- the **event fan-out list** — the `UnboundedSender`s for every active `subscribe_events()` channel,
- the **request-id counter**.

The reader task is spawned once, at `connect()`, and lives for the life of the connection. This is why the sharing matters: the reader task must be the *only* reader of the socket, and it must be able to reach the pending-response map and the subscriber list. A second `connect()` would mean a second socket, a second reader task, and a second daemon-side client registration — so a frontend that needs a second handle must `clone()` the client rather than reconnect.

The same shape appears on the embedded side: `EmbeddedRuntimeClient` holds an `Arc<Runtime>`, so cloning it shares one runtime.

## Not an Enforcement Surface

This crate makes **no authorization decision**. The contract carries no policy, no scope evaluation, and no override handling: it is a transport-shaped view of a runtime.

- The daemon applies its own access controls (RBAC roles, session ownership, declared-client identity) — see [daemon.md](daemon.md).
- The embedded runtime applies the engine's enforcement, exactly as before the move.
- A `RuntimeSurface` passed through `create_session()` is an **identity claim**, not an authorization. The engine's runtime bridge is what converts it into an `ExecutionSurface` and evaluates it — see [runtime_bridge.md](runtime_bridge.md) and [config.md](config.md).

Because the crate depends on neither the engine nor `eggsec-policy`, it cannot reach the enforcement path even by accident.

## Integration Points

- **TUI**: `eggsec-tui` depends on `eggsec-frontend-client` for `RuntimeClient`, `RuntimeEventReceiverHandle`, `EmbeddedRuntimeClient`, and `DaemonRuntimeClient`. It no longer depends on `eggsec-daemon-protocol` directly, and its former `crates/eggsec-tui/src/runtime_client/` module is gone. Its daemon connect now passes its own `ClientIdentity` — `ClientKind::Tui` / `eggsec-tui` / `tui-` (`crates/eggsec-tui/src/app/mod.rs:390-397`). The observable behaviour is unchanged. See [tui.md](tui.md).
- **Desktop GUI**: the second consumer, and a daemon-only one — `eggsec-gui` binds the same contract and passes `ClientKind::Gui` / `eggsec-gui` / `gui-`. It has no embedded backend and never links the engine. See [desktop_gui.md](desktop_gui.md).
- **`eggsec-daemon-protocol`**: supplies the wire types the daemon backend speaks.
- **`eggsec-runtime`**: supplies `RunRequest`, `RuntimeSurface`, `SessionId`, `SessionSnapshot`, `SessionSummary`, `TaskId`, `RuntimeCapabilities`, `RuntimeEvent`.

## Key Invariants

1. **No frontend framework** — zero `ratatui`/`crossterm` imports (guard 153)
2. **No engine dependency** — zero `eggsec` crate imports (guard 153)
3. **No enforcement** — the crate owns no policy, scope, or override logic
4. **One connection per client** — clones share one socket, one reader task, one pending-response registry
5. **Frontend-neutral** — no terminal, GUI-toolkit, or windowing types appear in the public API
6. **No hardcoded frontend identity** — the kind, the label, and the request-id namespace are the caller's, through `ClientIdentity`
7. **The trait is the portable contract** — the daemon-only questions (`describe_operations`, `get_task_report`, `health`, `daemon_capabilities`) are inherent methods, because the embedded backend has no daemon to ask
8. **An unavailable report is not a failure** — `get_task_report()` returns `Ok(None)` when the daemon has no report for a task *and* when the daemon is too old to know the command. Both are degradable states; only a transport or protocol failure is an error
9. **A refusal must carry its request id** — responses are routed by request id, so a refusal that omits it resolves no pending request and turns a refusal into a timeout. The daemon echoes it; the client's 30s bound is the backstop, not the mechanism

## See Also

- [tui.md](tui.md) — the terminal UI, the contract's first consumer
- [desktop_gui.md](desktop_gui.md) — the desktop GUI, the contract's second consumer
- [runtime.md](runtime.md) — the runtime the embedded backend wraps
- [daemon.md](daemon.md) — the daemon the daemon backend speaks to
- [runtime_bridge.md](runtime_bridge.md) — where a `RuntimeSurface` becomes an `ExecutionSurface`
- [overview.md](overview.md) — System-wide architecture and crate counts

*Last verified against source: 2026-10-24 (`get_task_report()` added as a fourth daemon-only inherent method, with the `Ok(None)` degradation contract; the `UnsupportedCommand` degradation and the request-id routing it depends on); earlier passes 2026-10-23 (crate extraction from `crates/eggsec-tui/src/runtime_client/`; trait renamed `TuiRuntimeClient` → `RuntimeClient`; `ClientIdentity` seam added and `connect` widened to take it, with the three daemon-only methods — `describe_operations`, `health`, `daemon_capabilities` — added as inherent methods)*
