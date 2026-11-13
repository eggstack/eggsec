---
name: eggsec-frontend-client
description: "Frontend-neutral runtime client contract for Eggsec frontends - use when working with the RuntimeClient trait, embedded vs daemon runtime backends, remote attach mode, or building a second frontend (desktop GUI)."
---

# Eggsec Frontend Client Skill

The shared runtime-client contract that every interactive frontend uses to reach
the engine runtime.

## Why this crate exists

The runtime-client contract used to live inside `eggsec-tui` and was named
`TuiRuntimeClient`. That made it impossible for a second frontend to reuse the
contract without depending on the whole TUI (ratatui, crossterm, themes,
clipboard). It now lives in its own leaf crate so the TUI, the desktop GUI,
and any other interactive frontend can all depend on the same contract while
the engine stays frontend-neutral. The desktop GUI is the second consumer:
`crates/eggsec-gui` reaches the daemon through this crate and through nothing
else.

## Crate facts

| Fact | Value |
|------|-------|
| Crate path | `crates/eggsec-frontend-client/` |
| Public API | `RuntimeClient`, `RuntimeClientFuture`, `RuntimeEventReceiverHandle`, `EmbeddedRuntimeClient`, `DaemonRuntimeClient`, `DaemonClientError`, `ClientIdentity` |
| Workspace deps | `eggsec-runtime`, `eggsec-daemon-protocol` only |
| Forbidden deps | any frontend framework (`ratatui`, `crossterm`), the engine (`eggsec`), and `eggsec-tui` |
| Tokio features | `rt`, `sync`, `time`, `io-util`, `net` — no `rt-multi-thread`, no `macros` |
| Enforced by | architecture guard 153 (no frontend framework, no engine dep) |

Headline crate and surface counts live in `architecture/overview.md` — point at
it rather than restating the numbers.

## Module structure

```
crates/eggsec-frontend-client/src/
├── lib.rs      # Crate docs, public re-exports, the RuntimeClient trait, ClientIdentity
├── embedded.rs # EmbeddedRuntimeClient — in-process, wraps Arc<eggsec_runtime::Runtime>
└── daemon.rs   # DaemonRuntimeClient — Unix socket + JSON-lines to eggsec-daemon
```

## The trait

`RuntimeClient` is the contract every frontend implements against:

- `capabilities`, `create_session`, `list_sessions`, `snapshot`, `submit`,
  `cancel`, `cancel_active`, `approve_task`, `subscribe`.
- Supertraits: `Send + Sync + 'static`, so `Arc<dyn RuntimeClient + Send + Sync>`
  is the holder type.
- Every method returns `RuntimeClientFuture<T>` — a boxed `Send + 'static`
  future of `Result<T, String>`. The boxed-future alias is what makes the
  trait object-safe.

`RuntimeEventReceiverHandle` wraps a `tokio::sync::mpsc::UnboundedReceiver<RuntimeEvent>`
and hands out `recv` / `try_recv`.

## The two implementations

- **Embedded** — `EmbeddedRuntimeClient` wraps `Arc<eggsec_runtime::Runtime>`
  directly. Full typed `TaskResult` is available.
- **Daemon** — `DaemonRuntimeClient` speaks the daemon's JSON-lines protocol
  over a Unix socket. Typed `TaskResult` is unavailable, so frontends fall back
  to envelope rendering.

## Client identity: the caller names itself

The crate is frontend-neutral, so it hardcodes **no** frontend's identity. The
caller supplies a `ClientIdentity`
(`crates/eggsec-frontend-client/src/lib.rs:46`):

```rust
pub struct ClientIdentity {
    pub kind: ClientKind,      // the daemon's client registry kind
    pub label: String,         // the label the daemon shows in its listings
    pub request_prefix: String, // this client's request-id namespace
}
```

`DaemonRuntimeClient::connect` takes it as its second argument
(`crates/eggsec-frontend-client/src/daemon.rs:80`):

```rust
DaemonRuntimeClient::connect(socket_path, identity).await?
```

The prefix is used verbatim, so a caller that wants a separator includes it in
the prefix itself (`"gui-"` yields `gui-0`,
`crates/eggsec-frontend-client/src/lib.rs:60`).

The two callers:

| Frontend | `kind` | `label` | `request_prefix` |
|---|---|---|---|
| TUI | `ClientKind::Tui` | `"eggsec-tui"` | `"tui-"` |
| Desktop GUI | `ClientKind::Gui` | `"eggsec-gui"` | `"gui-"` |

**The crate carries no TUI-specific identity.** A test asserts that the
production half of `daemon.rs` contains no `"tui-"`, no `ClientKind::Tui`, and
no `"eggsec-tui"`
(`crates/eggsec-frontend-client/src/lib.rs:279`). A TUI-shaped literal in the
non-test source is a regression toward the frontend-specific behavior this
crate was extracted to remove.

## The daemon-only methods

Three methods are **inherent** on `DaemonRuntimeClient`, not `RuntimeClient`
trait methods, because the embedded client has no daemon to ask:

| Method | Returns | Source |
|---|---|---|
| `describe_operations()` | `DescribeOperationsResponse` — the operation catalog plus the published JSON Schema | `crates/eggsec-frontend-client/src/daemon.rs:230` |
| `health()` | `DaemonHealth`, carrying the daemon's `protocol_version` | `crates/eggsec-frontend-client/src/daemon.rs:251` |
| `daemon_capabilities()` | `DaemonCapabilities`, carrying the `operation_schema` flag | `crates/eggsec-frontend-client/src/daemon.rs:280` |

`daemon_capabilities()` is not a duplicate of the trait's `capabilities()`:
the trait method returns the *runtime* half of the advertisement, while this
one returns the daemon's own flags — notably `operation_schema`, which says
whether the connected daemon can publish the operation catalog at all.

The `RuntimeClient` trait's method set gained `approve_task` in milestone 011
(`ADR-0012`): it answers a policy decision the runtime is holding for a task.
The answer names *yes* or *no*; it never names the authorisation.

## Critical implementation detail: shared connection state

`DaemonRuntimeClient` is `Clone`, and every clone must share **one**
`Arc<DaemonConnection>`. The connection owns:

- the request counter and the write half of the socket,
- the pending-response map (`FxHashMap<String, oneshot::Sender<ServerMessage>>`),
- the event-channel list (`RwLock<Vec<UnboundedSender<RuntimeEvent>>>`).

A spawned reader task owns the read half and fans every runtime event out to
**all** registered event channels. If a call site clones the connection state
instead of the `Arc`, the fan-out silently breaks: one client stops receiving
events that another client still receives. Clone the client, never the
connection internals.

## Boundaries

- This crate is **not** an enforcement surface. It never evaluates policy, never
  constructs an `ApprovedExecution`, and never calls `Scope::is_target_allowed`.
  Every dispatch goes through the daemon, which enforces.
- The crate depends on no frontend framework. A frontend that needs rendering
  brings its own; this crate only speaks to the runtime.
- The crate names no frontend. Every frontend's identity — kind, label, and
  request-id namespace — is the caller's to supply, so adding a frontend never
  requires editing this crate.
- `DaemonClientError` is public so a frontend can distinguish an I/O failure
  from a protocol error from a closed connection.

## Where to look

- `architecture/frontend_client.md` — the deep-dive architecture doc.
- `architecture/tui.md` — how the TUI wires the client into its app layer.
- `architecture/daemon.md` — the daemon side of the protocol.
- `.opencode/skills/eggsec-gui/SKILL.md` — the second consumer of this contract.
- `plans/implementation/desktop-gui-frontend/002-frontend-client-contract-extraction.md`
  — the milestone plan this crate came from.
