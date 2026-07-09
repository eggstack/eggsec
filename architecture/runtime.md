# eggsec-runtime Architecture

Frontend-neutral async runtime for task lifecycle management. Provides the bridge between user-facing frontends (TUI, CLI, REST, MCP) and the eggsec engine.

## Purpose

`eggsec-runtime` owns the `Runtime` — a session-scoped task orchestrator that manages submission, execution, cancellation, and event broadcasting for security assessment tasks. It is intentionally dependency-light (serde, tokio, tracing only) to remain a shared contract between all frontends without pulling in TUI, transport, or engine dependencies.

## Crate Dependencies

```
eggsec-runtime (leaf — no workspace deps)
    ↑
    ├── eggsec-ui-model   (view DTOs for rendering)
    ├── eggsec-daemon     (persistent session host)
    └── eggsec            (engine — for RuntimeTaskExecutor impl)
```

## Core Types

| Type | Location | Purpose |
|------|----------|---------|
| `Runtime` | `runtime.rs` | Main orchestrator: submit/cancel/snapshot/subscribe |
| `RuntimeConfig` | `runtime.rs` | Timeouts, capacity, single-active-task policy config |
| `RuntimeTaskExecutor` | `runtime.rs` | Trait for frontend-supplied execution logic |
| `RuntimeEventSink` | `runtime.rs` | Event emission interface |
| `RuntimeEventReceiver` | `runtime.rs` | Broadcast receiver for session events |
| `RuntimeSession` | `session.rs` | Mutable session state (tasks, surface, scope) |
| `RuntimeExecutionContext` | `session.rs` | Per-execution context passed to executor |
| `SessionSnapshot` | `session.rs` | Immutable snapshot for persistence/transport |
| `TaskSnapshot` | `session.rs` | Per-task snapshot within a session |
| `RunRequest` | `request.rs` | Task submission request |
| `RuntimeSurface` | `request.rs` | Caller identity (10 variants) |
| `TaskKind` | `request.rs` | Operation category (29 variants) |
| `TaskOutcome` | `event.rs` | Terminal result (5 variants) |
| `RuntimeEvent` | `event.rs` | Streaming events (11 variants) |
| `RuntimeCapabilities` | `capabilities.rs` | Declares allowed task kinds per mode |
| `SessionId` / `TaskId` / `ClientId` | `ids.rs` | UUID newtypes with serde/display |

## RuntimeSession

Owns the trust boundary for a session:

- **`RuntimeSurface`** is bound at session creation and is authoritative. Request surfaces are normalized to match — `RunRequest.surface` is informational only and must not influence enforcement.
- **Single-active-task policy**: new submissions cancel existing active tasks.
- **`SessionScope`** carries the loaded scope for enforcement resolution.
- **`generation`** field enables optimistic concurrency tracking for snapshot hydration.

## RuntimeTaskExecutor Trait

```rust
#[async_trait]
pub trait RuntimeTaskExecutor: Send + Sync {
    async fn execute(
        &self,
        context: RuntimeExecutionContext,
        request: RunRequest,
        sink: RuntimeEventSink,
        cancel: tokio::sync::watch::Receiver<bool>,
    ) -> TaskOutcome;
}
```

Frontends supply this trait to bridge runtime DTOs to actual tool execution. The engine crate (`eggsec`) provides `EggsecRuntimeExecutor` which performs:
1. `runtime_surface_to_execution_surface()` — converts runtime DTO → engine type
2. Scope resolution from `RuntimeExecutionContext`
3. `approve_run_request_bundle()` — policy enforcement
4. `dispatch_approved_runtime_request()` — actual tool dispatch

Without the `full-executor` feature, `NoopExecutorStub` rejects all tasks.

## RuntimeCapabilities

Gates which task kinds are allowed based on daemon mode:

| Mode | Allowed Task Kinds |
|------|-------------------|
| `daemon_conservative` | Excludes: stress-test, packet-send, wireless, db-pentest, intercept, c2 |
| `full_lab` | All 29 task kinds |

Capabilities are advertised to clients via `DaemonCapabilities` so frontends can show/hide UI elements.

## Event System

Events are broadcast via tokio broadcast channel (best-effort):

| Event | Description |
|-------|-------------|
| `TaskSubmitted` | New task queued |
| `TaskStarted` | Execution began |
| `TaskProgress` | Progress update (percentage + message) |
| `TaskCompleted` | Terminal success |
| `TaskFailed` | Terminal failure |
| `TaskCancelled` | Operator cancelled |
| `SessionCreated` | New session |
| `SessionClosed` | Session ended |
| `AuditEvent` | Security-relevant action |
| `PolicyPrompt` | Confirmation required |
| `Heartbeat` | Liveness signal |

Critical event loss is logged at warn level.

## Persistence

- **Stale completion guards**: terminal state cannot be overwritten by late executor results.
- **`hydrate_from_snapshot`**: restores `RuntimeSession` from `SessionSnapshot` for daemon recovery.
- **Session snapshots** include `generation` for optimistic concurrency.

## Key Invariants

1. **No TUI dependencies** — no `ratatui`/`crossterm` imports.
2. **No transport dependencies** — no `axum`/`tonic`/`tokio-tungstenite`.
3. **No reverse dependency on `eggsec`** — the engine depends on runtime, not vice versa.
4. **Session-derived surface** — executor derives surface from `RuntimeSession`, not request defaults.
5. **Single-active-task** — submitting a new task cancels the current active task.
6. **Dependency-light** — only serde, tokio, tokio-util, tracing, uuid.

## See Also

- [daemon.md](daemon.md) — Daemon host that owns the Runtime
- [overview.md](overview.md) — System-wide architecture
- `docs/ARCHITECTURE.md` §4.8 — Daemon/Runtime execution flow
