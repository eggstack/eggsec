# eggsec-runtime Architecture

Frontend-neutral async runtime for task lifecycle management. Provides the bridge between user-facing frontends (TUI, CLI, REST, MCP) and the eggsec engine. Intentionally dependency-light to remain a shared contract without pulling in TUI, transport, or engine dependencies.

## Role & Responsibilities

`eggsec-runtime` owns the `Runtime` — a session-scoped task orchestrator managing submission, execution, cancellation, timeout, and event broadcasting. It also defines the core DTOs (`TaskKind`, `RuntimeSurface`, `RunRequest`, `RuntimeEvent`, `SessionSnapshot`) shared across all frontends.

## Location & Feature Gating

| Crate | Path | Dependencies |
|-------|------|-------------|
| `eggsec-runtime` | `crates/eggsec-runtime/` | `serde`, `serde_json`, `thiserror`, `tokio`, `tokio-util`, `tracing`, `uuid` — **zero workspace deps** |

Architecture guard: zero TUI, transport, persistence, or engine dependencies. Enforced by `scripts/check-architecture-guards.sh`.

## Architecture (9 source files)

| Module | File | Purpose |
|--------|------|---------|
| `lib` | `src/lib.rs` | Public API re-exports |
| `runtime` | `src/runtime.rs` | `Runtime` orchestrator, `RuntimeConfig`, `RuntimeEventReceiver`, `RuntimeEventSink`, `RuntimeTaskExecutor` trait |
| `session` | `src/session.rs` | `RuntimeSession`, `RuntimeExecutionContext`, `SessionScope`, `SessionSnapshot`, `TaskSnapshot`, `SessionSummary` |
| `request` | `src/request.rs` | `TaskKind` (29 variants), `RuntimeSurface` (10 variants), `RunRequest`, payload structs |
| `event` | `src/event.rs` | `RuntimeEvent` (12 variants), `TaskOutcome` (5 variants), `TaskStatus` (6 variants), `TaskProgress`, `TaskResultEnvelope`, `ArtifactRef`, `LogLevel`, `RuntimeErrorInfo`, `PolicyPrompt`, `RuntimeAuditEvent` |
| `capabilities` | `src/capabilities.rs` | `RuntimeCapabilities`, `TaskCapability` |
| `ids` | `src/ids.rs` | `SessionId`, `TaskId`, `ClientId` — UUID newtypes with serde/display |
| `error` | `src/error.rs` | `RuntimeError` (13 variants) |
| `dispatcher` | `src/dispatcher.rs` | `TaskDispatcher` trait (unused by runtime; legacy interface) |

### Core Types

| Type | Location | Purpose |
|------|----------|---------|
| `Runtime` | `runtime.rs` | Main orchestrator: `create_session()`, `submit()`, `cancel()`, `cancel_active()`, `snapshot()`, `subscribe()`, `close_session()`, `hydrate_session()` |
| `RuntimeConfig` | `runtime.rs:37-49` | `default_task_timeout`, `max_active_tasks_per_session`, `event_channel_capacity`, `capabilities` |
| `RuntimeTaskExecutor` | `runtime.rs:212-228` | Trait for frontend-supplied execution logic |
| `RuntimeEventSink` | `runtime.rs:122-203` | Progress/log/completion/failure reporting for executors |
| `RuntimeEventReceiver` | `runtime.rs:70-119` | Broadcast receiver with lag recovery |
| `RuntimeExecutionContext` | `session.rs:34-42` | Per-execution context: session_id, surface, scope |
| `RuntimeSession` | `session.rs:91-120` | Mutable session state (tasks, surface, scope, generation, capabilities) |
| `SessionSnapshot` | `session.rs:349-370` | Immutable snapshot for persistence/transport |
| `TaskSnapshot` | `session.rs:331-341` | Per-task snapshot within a session |
| `SessionSummary` | `session.rs:374-384` | Lightweight summary for listing |
| `SessionScope` | `session.rs:57-65` | Scope metadata: `is_explicit`, `source`, `path` |
| `RunRequest` | `request.rs:86-92` | Task submission: `task_kind`, `requested_by`, `surface`, `labels` |

### RuntimeSurface — 10 variants (`request.rs:10-22`)

| # | Variant | Label | Automated |
|---|---------|-------|:---------:|
| 1 | `CliManual` | `cli-manual` | No |
| 2 | `CliManualStrict` | `cli-manual-strict` | No |
| 3 | `TuiManual` | `tui-manual` | No |
| 4 | `TuiManualStrict` | `tui-manual-strict` | No |
| 5 | `Ci` | `ci` | Yes |
| 6 | `McpServer` | `mcp-server` | Yes |
| 7 | `RestApi` | `rest-api` | Yes |
| 8 | `GrpcApi` | `grpc-api` | Yes |
| 9 | `SecurityAgent` | `security-agent` | Yes |
| 10 | `Unknown` | `unknown` (default) | — |

### TaskKind — 29 variants (`request.rs:53-83`)

| # | Variant | Capability Name | Params Struct |
|---|---------|----------------|---------------|
| 1 | `LoadTest` | `load-test` | `LoadTestParams` |
| 2 | `StressTest` | `stress-test` | `StressTestParams` |
| 3 | `PortScan` | `port-scan` | `PortScanParams` |
| 4 | `EndpointScan` | `endpoint-scan` | `EndpointScanParams` |
| 5 | `Fingerprint` | `fingerprint` | `FingerprintParams` |
| 6 | `Fuzz` | `fuzz` | `FuzzParams` |
| 7 | `Waf` | `waf` | `WafParams` |
| 8 | `WafStress` | `waf-stress` | `WafStressParams` |
| 9 | `Pipeline` | `pipeline` | `PipelineParams` |
| 10 | `Recon` | `recon` | `ReconParams` |
| 11 | `PacketCapture` | `packet-capture` | `PacketCaptureParams` |
| 12 | `PacketTraceroute` | `traceroute` | `PacketTracerouteParams` |
| 13 | `PacketSend` | `packet-send` | `PacketSendParams` |
| 14 | `GraphQl` | `graphql` | `GraphQlParams` |
| 15 | `OAuth` | `oauth` | `OAuthParams` |
| 16 | `AuthTest` | `auth-test` | `AuthTestParams` |
| 17 | `Nse` | `nse` | `NseParams` |
| 18 | `Hunt` | `hunt` | `HuntParams` |
| 19 | `Browser` | `browser` | `BrowserParams` |
| 20 | `Compliance` | `compliance` | `ComplianceParams` |
| 21 | `Storage` | `storage` | `StorageParams` |
| 22 | `Integrations` | `integration` | `IntegrationsParams` |
| 23 | `Workflow` | `workflow` | `WorkflowParams` |
| 24 | `Vuln` | `vuln` | `VulnParams` |
| 25 | `Wireless` | `wireless` | `WirelessParams` |
| 26 | `WirelessActive` | `wireless-active` | `WirelessActiveParams` |
| 27 | `DbPentest` | `db-pentest` | `DbPentestParams` |
| 28 | `Intercept` | `intercept` | `InterceptParams` |
| 29 | `C2` | `c2` | `C2Params` |

### RuntimeEvent — 12 variants (`event.rs:114-169`)

| # | Variant | Fields |
|---|---------|--------|
| 1 | `SessionCreated` | `session_id` |
| 2 | `Snapshot` | `session_id`, `snapshot: SessionSnapshot` |
| 3 | `TaskQueued` | `session_id`, `task_id`, `request: RunRequest` |
| 4 | `TaskStarted` | `session_id`, `task_id` |
| 5 | `TaskProgress` | `session_id`, `task_id`, `progress: TaskProgress` |
| 6 | `TaskLog` | `session_id`, `task_id: Option<TaskId>`, `level: LogLevel`, `message` |
| 7 | `PolicyDecisionRequired` | `session_id`, `task_id: Option<TaskId>`, `prompt: PolicyPrompt` |
| 8 | `TaskCompleted` | `session_id`, `task_id`, `outcome: TaskOutcome` |
| 9 | `TaskFailed` | `session_id`, `task_id`, `error: RuntimeErrorInfo` |
| 10 | `TaskCancelled` | `session_id`, `task_id`, `reason: Option<String>` |
| 11 | `SessionClosed` | `session_id` |
| 12 | `Audit` | `session_id`, `event: RuntimeAuditEvent` |

### TaskOutcome — 5 variants (`event.rs:70-84`)

| # | Variant | Description |
|---|---------|-------------|
| 1 | `Json(serde_json::Value)` | JSON payload |
| 2 | `Text(String)` | Plain text |
| 3 | `Artifact { artifact_id, summary }` | Reference to external artifact |
| 4 | `Result(TaskResultEnvelope)` | Structured envelope with kind + artifacts (preferred) |
| 5 | `Empty` | No result |

### TaskStatus — 6 variants (`event.rs:8-15`)

`Queued`, `Running`, `Completed`, `Failed`, `Cancelled`, `TimedOut`

Terminal states: `Completed`, `Failed`, `Cancelled`, `TimedOut`

## Behavior/Flow

### Task Lifecycle State Machine

```
submit()
  ├─► Session validation (exists? closed? capability supported?)
  ├─► Single-active-task policy: cancel existing active tasks
  ├─► Increment generation
  ├─► Insert TaskRecord (status: Queued)
  ├─► Emit TaskQueued
  └─► tokio::spawn(execution task)
       ├─► Guard: if already terminal → return (stale submission)
       ├─► Set status: Running
       ├─► Emit TaskStarted
       ├─► Execute with timeout (tokio::time::timeout)
       │     ├─► Ok(Ok(outcome)) → status: Completed, emit TaskCompleted
       │     ├─► Ok(Err(e)) → status: Failed, emit TaskFailed (critical)
       │     └─► Err(timeout) → cancel token, status: TimedOut, emit TaskCancelled (critical)
       ├─► Update task record (status, error, outcome)
       ├─► Stale guard: if already terminal → discard late result
       ├─► Increment generation
       └─► Emit terminal event (AFTER state update for snapshot consistency)
```

### Session Lifecycle

```
create_session(surface, scope?)
  ├─► Insert RuntimeSession into HashMap
  ├─► Emit SessionCreated
  └─► Return SessionId

close_session(session_id)
  ├─► Cancel all active tasks (emit TaskCancelled for each)
  ├─► Mark session closed (closed=true, closed_at=now)
  ├─► Increment generation
  └─► Emit SessionClosed (critical)

hydrate_session(snapshot) [daemon recovery]
  ├─► Construct RuntimeSession from snapshot metadata
  ├─► Restore completed_tasks as hydrated_completed (read-only)
  ├─► Insert into session map
  └─► Emit SessionCreated
```

### Event Broadcasting

Events broadcast via `tokio::sync::broadcast` channel (default capacity: 256).

- `emit_event()`: logs at `trace` level when no subscribers or on send failure
- `emit_event_critical()`: logs at `warn` level when no subscribers or on send failure (for policy-relevant events: TaskFailed, TaskCancelled, TaskCancelled, SessionClosed)
- `RuntimeEventReceiver::recv()`: handles `RecvError::Lagged(n)` by logging warning and continuing (recoverable lag, not closure)
- `RuntimeEventReceiver::try_recv()`: non-blocking variant with same lag handling

### RuntimeCapabilities

| Mode | Constructor | Task Kinds |
|------|-------------|-----------|
| Conservative | `daemon_conservative()` (default) | 20 kinds — excludes stress-test, packet-send, packet-capture, traceroute, wireless, wireless-active, db-pentest, intercept, c2 |
| Full lab | `full_lab()` | All 29 kinds |
| No-op | `noop()` | Empty (no task kinds) |

### RuntimeTaskExecutor Trait (`runtime.rs:212-228`)

```rust
fn execute(
    &self,
    task_id: TaskId,
    request: RunRequest,
    context: RuntimeExecutionContext,
    sink: RuntimeEventSink,
    cancel: CancellationToken,
) -> Pin<Box<dyn Future<Output = Result<TaskOutcome, RuntimeError>> + Send + 'static>>;
```

The engine crate provides `EggsecRuntimeExecutor` which:
1. Converts `RuntimeSurface` → `ExecutionSurface`
2. Resolves scope from `RuntimeExecutionContext`
3. Calls `approve_run_request_bundle()` for policy enforcement
4. Calls `dispatch_approved_runtime_request()` for tool dispatch

Without `full-executor`: `NoopExecutorStub` rejects all tasks.

## Public API

| Method | Signature | Purpose |
|--------|-----------|---------|
| `Runtime::new()` | `(config, executor) -> Self` | Create runtime |
| `create_session()` | `(options, surface) -> Result<SessionId>` | New session |
| `create_session_with_scope()` | `(options, surface, scope) -> Result<SessionId>` | New session with scope |
| `submit()` | `(session_id, request) -> Result<TaskId>` | Submit task |
| `cancel()` | `(session_id, task_id) -> Result<()>` | Cancel task |
| `cancel_active()` | `(session_id) -> Result<()>` | Cancel active task |
| `snapshot()` | `(session_id) -> Result<SessionSnapshot>` | Get snapshot |
| `list_sessions()` | `() -> Vec<SessionSummary>` | List active sessions |
| `close_session()` | `(session_id) -> Result<()>` | Close session |
| `hydrate_session()` | `(snapshot) -> Result<SessionId>` | Restore from snapshot |
| `subscribe()` | `() -> RuntimeEventReceiver` | Subscribe to events |
| `session_surface()` | `(session_id) -> Result<RuntimeSurface>` | Query surface |
| `session_scope()` | `(session_id) -> Result<Option<SessionScope>>` | Query scope |
| `set_session_owner()` | `(session_id, owner) -> Result<()>` | Set owner |

## Integration Points

- **Daemon host**: `DaemonHost` wraps `Arc<Runtime>`, dispatches `ClientCommand` variants to runtime methods
- **Engine executor**: `EggsecRuntimeExecutor` implements `RuntimeTaskExecutor` behind `full-executor`
- **TUI**: subscribes to events, creates sessions, submits tasks via `Runtime` directly or through daemon
- **runtime_bridge**: converts `RuntimeSurface` → `ExecutionSurface`, `RunRequest`/`TaskKind` → `OperationDescriptor`

## Testing

- `runtime.rs`: 17 tests covering session creation, submit, cancel, timeout, stale completion guard, event emission, multiple sessions, cancel_active, session timeout override
- `session.rs`: 12 tests for snapshot roundtrip, hydration, scope, capabilities, close
- `request.rs`: roundtrip and label tests
- `event.rs`: roundtrip and terminal-state tests
- `capabilities.rs`: conservative/full/noop mode tests, `supports_task_kind` positive/negative

## Invariants & Gotchas

1. **No TUI dependencies** — zero `ratatui`/`crossterm` imports
2. **No transport dependencies** — zero `axum`/`tonic`/`tokio-tungstenite`
3. **No reverse dependency on `eggsec`** — engine depends on runtime, not vice versa
4. **Session-derived surface** — executor reads surface from `RuntimeSession`, not from `RunRequest`
5. **Single-active-task** — submitting new task cancels existing active tasks (replaces)
6. **Stale completion guard** — terminal state cannot be overwritten by late executor results
7. **Generation counter** — incremented on every task state change; used for optimistic concurrency
8. **Event ordering** — terminal events emitted AFTER state update (prevents persistence worker from capturing stale snapshots)
9. **CancellationToken re-exported** — `pub use tokio_util::sync::CancellationToken` at crate root
10. **Default timeout**: 300s (5 minutes); session override takes precedence over runtime default
11. **Broadcast channel capacity**: 256 (configurable via `RuntimeConfig::event_channel_capacity`)

## See Also

- [daemon.md](daemon.md) — Daemon host that owns the Runtime
- [ui_model.md](ui_model.md) — View DTOs derived from runtime types
- [overview.md](overview.md) — System-wide architecture
- [runtime_bridge.md](runtime_bridge.md) — Engine-side surface conversion and approval
- [tui.md](tui.md) — TUI that consumes runtime events
- [cli_commands.md](cli_commands.md) — CLI commands that dispatch through runtime

*Last verified against source: 2026-08-25*
