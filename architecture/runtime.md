# eggsec-runtime Architecture

> **Corrections (2026-10-06)** — verified against source: `TaskKind` is **30**
> variants, not 29; the previously undocumented 30th variant is **`Resume`**
> (`ResumeParams`, capability `resume`, shares the `pipeline` operation family).
> `RuntimeEvent` decl is `event.rs:114` (was `:115`); `RunRequest` is
> `request.rs:99` (was `:98`); `request.rs` has 5 tests (was 2).
> Removed a duplicated `TaskCancelled` in the critical-event list. All
> citations re-checked.
>
> **Corrections (2026-10-24, milestone 010)** — `full_lab()` and
> `daemon_conservative()` are **gone**. The workspace now has exactly one
> capability list, `RuntimeCapabilities::canonical()`, **derived** from the
> canonical task-kind set and covering all 30 canonical names, `resume`
> included. The task-kind set is declared once, in the `define_task_kinds!`
> table in `request.rs`, which generates the enum, the capability-name
> match, the variant enumeration, and the published description table. The
> trait default of `RuntimeTaskExecutor::supported_task_kinds()` is the
> canonical list, so an executor that rejects every task must declare the
> empty set explicitly.

Frontend-neutral async runtime for task lifecycle management. Provides the bridge between user-facing frontends (TUI, CLI, REST, MCP) and the eggsec engine. Intentionally dependency-light to remain a shared contract without pulling in TUI, transport, or engine dependencies.

## Role & Responsibilities

`eggsec-runtime` owns the `Runtime` — a session-scoped task orchestrator managing submission, execution, cancellation, timeout, and event broadcasting. It also defines the core DTOs (`TaskKind`, `RuntimeSurface`, `RunRequest`, `RuntimeEvent`, `SessionSnapshot`) shared across all frontends.

## Location & Feature Gating

| Crate | Path | Dependencies |
|-------|------|-------------|
| `eggsec-runtime` | `crates/eggsec-runtime/` | `serde`, `serde_json`, `thiserror`, `tokio`, `tokio-util`, `tracing`, `uuid` — **zero workspace deps**; optional `schemars` behind the off-by-default `schema` feature |

The `schema` feature (off by default) exposes `src/schema.rs`: `task_kind_schemas()` returns one entry per `TaskKind` variant (30) with its wire `kind` discriminator, its canonical `operation_id`, its `capability_name`, and a JSON Schema for that variant's params struct; `task_kind_schema()` returns the whole `TaskKind` schema in its adjacently-tagged wire shape. The schema is **derived** from the canonical request DTOs, never hand-written, and it is **descriptive only** — no authorization, dispatch, scope, or policy path consults it.

Architecture guard: zero TUI, transport, persistence, or engine dependencies. Enforced by `scripts/check-architecture-guards.sh`.

## Architecture (14 source files, Phase D)

| Module | File | Purpose |
|--------|------|---------|
| `lib` | `src/lib.rs` | Public API re-exports |
| `runtime` | `src/runtime.rs` | `Runtime` orchestrator, `RuntimeTaskExecutor` trait (facade; config/sink re-exported) |
| `runtime_config` | `src/runtime_config.rs` | `RuntimeConfig`, `SessionOptions` (submission/lifecycle tuning) |
| `runtime_sink` | `src/runtime_sink.rs` | `RuntimeEventReceiver`, `RuntimeEventSink`, broadcast helpers (events/backpressure) |
| `report` | `src/report.rs` | `RuntimeReportStore`, `MAX_RETAINED_REPORTS` — the bounded store of completed-task report documents |
| `session` | `src/session.rs` | `RuntimeSession`, `RuntimeExecutionContext`, `SessionScope`, `SessionSnapshot`, `TaskSnapshot`, `SessionSummary` |
| `request` | `src/request.rs` | `TaskKind` (30 variants), `RuntimeSurface` (11 variants), `RunRequest`, payload structs |
| `event` | `src/event.rs` | `RuntimeEvent` (12 variants), `TaskOutcome` (5 variants), `TaskStatus` (6 variants), `TaskProgress`, `TaskResultEnvelope`, `ArtifactRef`, `LogLevel`, `RuntimeErrorInfo`, `PolicyPrompt`, `RuntimeAuditEvent` |
| `capabilities` | `src/capabilities.rs` | `RuntimeCapabilities`, `TaskCapability` |
| `approval` | `src/approval.rs` | `ApprovalGate`, `ApprovalRegistry`, `ApprovalAnswer` — the runtime's pending-approval registry (milestone 011, `ADR-0012`) |
| `ids` | `src/ids.rs` | `SessionId`, `TaskId`, `ClientId` — UUID newtypes with serde/display |
| `error` | `src/error.rs` | `RuntimeError` (14 variants) |
| `dispatcher` | `src/dispatcher.rs` | `TaskDispatcher` trait (unused by runtime; legacy interface) |
| `cancel` | `src/cancel.rs` | `race_with_cancel` helper + `CancellationToken` re-export (cancellation boundary) |
| `schema` | `src/schema.rs` | **Feature-gated (`schema`)**: `TaskKindSchema`, `task_kind_schemas()`, `task_kind_schema()` — the derived JSON Schema over the canonical request DTOs |

### Core Types

| Type | Location | Purpose |
|------|----------|---------|
| `Runtime` | `runtime.rs` | Main orchestrator: `create_session()`, `submit()`, `cancel()`, `cancel_active()`, `snapshot()`, `subscribe()`, `close_session()`, `hydrate_session()`, plus the synchronous accessors `enabled_features()` and `report_for()` |
| `RuntimeConfig` | `runtime_config.rs` | `default_task_timeout`, `max_active_tasks_per_session`, `event_channel_capacity`, `capabilities` |
| `RuntimeTaskExecutor` | `runtime.rs` | Trait for frontend-supplied execution logic (defined at `runtime.rs:41`) |
| `RuntimeReportStore` | `report.rs:42` | Bounded, in-memory store of completed-task report documents (task id → report JSON) |
| `MAX_RETAINED_REPORTS` | `report.rs:29` | The retention bound: the number of report documents a runtime keeps at once |
| `RuntimeEventSink` | `runtime_sink.rs` | Progress/log/completion/failure reporting for executors |
| `RuntimeEventReceiver` | `runtime_sink.rs` | Broadcast receiver with lag recovery |
| `RuntimeExecutionContext` | `session.rs:35` | Per-execution context: session_id, surface, scope, approval gate |
| `RuntimeSession` | `session.rs:91` | Mutable session state (tasks, surface, scope, generation, capabilities) |
| `SessionSnapshot` | `session.rs:350` | Immutable snapshot for persistence/transport |
| `TaskSnapshot` | `session.rs:332` | Per-task snapshot within a session |
| `SessionSummary` | `session.rs:374` | Lightweight summary for listing |
| `SessionScope` | `session.rs:58` | Scope metadata: `is_explicit`, `source`, `path` |
| `RunRequest` | `request.rs:99` | Task submission: `task_kind`, `requested_by`, `surface`, `labels` |

### RuntimeSurface — 11 variants (`request.rs:21`)

| # | Variant | Label | Automated |
|---|---------|-------|:---------:|
| 1 | `CliManual` | `cli-manual` | No |
| 2 | `CliManualStrict` | `cli-manual-strict` | No |
| 3 | `TuiManual` | `tui-manual` | No |
| 4 | `TuiManualStrict` | `tui-manual-strict` | No |
| 5 | `DesktopGuiManual` | `desktop-gui-manual` | No |
| 6 | `Ci` | `ci` | Yes |
| 7 | `McpServer` | `mcp-server` | Yes |
| 8 | `RestApi` | `rest-api` | Yes |
| 9 | `GrpcApi` | `grpc-api` | Yes |
| 10 | `SecurityAgent` | `security-agent` | Yes |
| 11 | `Unknown` | `unknown` (default) | — |

### TaskKind — 30 variants (`request.rs:64`)

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
| 10 | `Resume` | `resume` | `ResumeParams` |
| 11 | `Recon` | `recon` | `ReconParams` |
| 12 | `PacketCapture` | `packet-capture` | `PacketCaptureParams` |
| 13 | `PacketTraceroute` | `traceroute` | `PacketTracerouteParams` |
| 14 | `PacketSend` | `packet-send` | `PacketSendParams` |
| 15 | `GraphQl` | `graphql` | `GraphQlParams` |
| 16 | `OAuth` | `oauth` | `OAuthParams` |
| 17 | `AuthTest` | `auth-test` | `AuthTestParams` |
| 18 | `Nse` | `nse` | `NseParams` |
| 19 | `Hunt` | `hunt` | `HuntParams` |
| 20 | `Browser` | `browser` | `BrowserParams` |
| 21 | `Compliance` | `compliance` | `ComplianceParams` |
| 22 | `Storage` | `storage` | `StorageParams` |
| 23 | `Integrations` | `integration` | `IntegrationsParams` |
| 24 | `Workflow` | `workflow` | `WorkflowParams` |
| 25 | `Vuln` | `vuln` | `VulnParams` |
| 26 | `Wireless` | `wireless` | `WirelessParams` |
| 27 | `WirelessActive` | `wireless-active` | `WirelessActiveParams` |
| 28 | `DbPentest` | `db-pentest` | `DbPentestParams` |
| 29 | `Intercept` | `intercept` | `InterceptParams` |
| 30 | `C2` | `c2` | `C2Params` |

`Resume` carries its own capability name (`resume`) so a runtime can advertise
pipeline support without advertising resume support; its canonical operation ID
is `pipeline` (it executes the pipeline stage set from a saved checkpoint) and
its canonical target is `None` — the target lives inside the checkpoint
(`request.rs:538`, `:598`, `:644`).

### RuntimeEvent — 12 variants (`event.rs:114`)

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

### TaskOutcome — 5 variants (`event.rs:70`)

| # | Variant | Description |
|---|---------|-------------|
| 1 | `Json(serde_json::Value)` | JSON payload |
| 2 | `Text(String)` | Plain text |
| 3 | `Artifact { artifact_id, summary }` | Reference to external artifact |
| 4 | `Result(TaskResultEnvelope)` | Structured envelope with kind + artifacts (preferred) |
| 5 | `Empty` | No result |

### TaskStatus — 6 variants (`event.rs:8`)

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
- `emit_event_critical()`: logs at `warn` level when no subscribers or on send failure (for policy-relevant events: TaskFailed, TaskCancelled, SessionClosed)
- `RuntimeEventReceiver::recv()`: handles `RecvError::Lagged(n)` by logging warning and continuing (recoverable lag, not closure)
- `RuntimeEventReceiver::try_recv()`: non-blocking variant with same lag handling

### RuntimeCapabilities

| Mode | Constructor | Task Kinds |
|------|-------------|-----------|
| Canonical | `canonical()` (default) | Every capability name the task-kind set defines, `resume` included — derived from `TaskKind::all_variants()`, never transcribed |
| Narrow | `with_task_kinds(list)` | Exactly the given list — how a host makes its **published** set and the runtime's **enforced** set the same object |
| No-op | `noop()` | Empty (no task kinds) |

The capability set is an **advertisement and admission** concern, not an
authorisation one: whether an operation may run is decided by the
enforcement layer from the operation's risk, the operator's execution policy,
the loaded scope, and the enabled features. Publishing a hazard family here
therefore does not permit it — it only stops the runtime from refusing the
submission as an unknown kind.

### The pending-approval registry (`approval.rs`)

A manual operation whose policy demands a confirmation is **held** at that
point. The runtime owns the hold, because the runtime owns task lifecycle; the
daemon owns the process and `eggsec-policy` owns authorisation semantics.

| Piece | Purpose |
|-------|---------|
| `ApprovalGate` | A cheap clonable handle, carried in `RuntimeExecutionContext`, so an executor can hold an operation and await its answer without knowing anything about the daemon, the protocol, or the policy crate |
| `ApprovalRegistry` | The in-memory map of pending decisions, keyed by `(session_id, task_id)` |
| `ApprovalAnswer` | `Approved { reason }` or `Denied { reason }` — the operator's yes or no |

The ordering matters: a decision is registered **before** the prompt is
announced, so a client that answers the instant it sees
`RuntimeEvent::PolicyDecisionRequired` never races the registration. A pending
entry is removed when it is answered, when the holding task is cancelled, or
when the task's timeout fires — the registry entry cannot outlive the task.

`Runtime` exposes the two ends of the conversation: `resolve_approval()`
(answer a pending decision) and `pending_approval()` (what is being asked).

**The runtime stays policy-neutral.** The registry stores a `PolicyPrompt` — a
message, a confirmation-class *label*, and a flag — not a `ConfirmationClass`.
The gate returns an answer; it does not interpret what an approval permits.
Only `eggsec-policy` decides that, and it does so on the caller's side of the
gate, by re-running the authorisation path with an override bounded by the
classes the kernel itself demanded. Architecture guard 174 enforces the
neutrality.

### RuntimeTaskExecutor Trait (`runtime.rs:41`)

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

Beside `execute`, the trait has three **defaulted** methods. A default is what keeps every existing executor compiling and keeps its current behaviour: an executor that says nothing publishes the default rather than having a host guess on its behalf.

| Method | Default | Exact semantics |
|--------|---------|-----------------|
| `supported_task_kinds()` (`runtime.rs:68`) | `RuntimeCapabilities::canonical().task_kinds` | The task kinds this executor can run — the canonical list, so "says nothing" means "declares everything". An executor that rejects every task MUST declare the empty set instead — otherwise a host would advertise capabilities the executor cannot execute. |
| `enabled_features()` (`runtime.rs:89`) | `None` | The engine features compiled into this executor's build — a **build** fact, not a runtime capability. `None` means **unknown**: the executor links no engine, or does not publish the fact. `Some(vec![])` means an engine is linked with **no** features enabled, which is a different, knowable fact. The two must never be collapsed: reading an empty vector as "unknown" would let a client report a wrong reason for an unavailable operation. |
| `report_for(task_id)` (`runtime.rs:100`) | `None` | The report document for a completed task. `None` means **no report for this task** — either the executor produces no reports at all, or this particular result has no report form. The executor's own retention policy decides how long a report stays available; the runtime only reads it. |

The engine crate provides `EggsecRuntimeExecutor` which:
1. Converts `RuntimeSurface` → `ExecutionSurface`
2. Resolves scope from `RuntimeExecutionContext`
3. Calls `approve_run_request_bundle()` for policy enforcement
4. Calls `dispatch_approved_runtime_request()` for tool dispatch

It also overrides both new methods: `enabled_features()` publishes the engine's own compile-time feature registry filtered by the policy bridge's enabled-feature set, and `report_for()` reads the engine's bounded report store (`crates/eggsec/src/runtime_bridge/executor.rs:147`, `:159`).

Without the daemon's `service` profile (deprecated alias `full-executor`): `NoopExecutorStub` rejects all tasks, and the daemon publishes an **empty** task-kind list — the published list is derived from the executor's declared support. Its `enabled_features()` is the default `None`: with no engine linked, the answer is unknown rather than empty.

### `RuntimeReportStore` — bounded report retention

`RuntimeReportStore` (`report.rs:42`) is the bounded store between the engine that *produces* a report document and the daemon that *serves* it. It maps a task id to the report document the engine derived from that task's result, and nothing else: the runtime owns task lifecycle, not report semantics.

Retention is **bounded on purpose**. A long-lived daemon would otherwise accumulate one report document per task for the life of the process, and report documents are among the largest payloads the runtime holds. The bound is `MAX_RETAINED_REPORTS = 64` (`report.rs:29`), and eviction is **oldest-first**: once the store is full, inserting a new document evicts the oldest retained one. The bound is therefore a hard ceiling on retained report documents, not a soft target, and a burst of tasks cannot grow the store without limit.

Every method takes `&self`, and its critical section is a map operation under a `std::sync::Mutex` that is never held across an await point. That keeps the store usable from the synchronous parts of an executor without forcing an async signature on the read path; the engine's executor shares one store across its tasks through an `Arc` (`crates/eggsec/src/runtime_bridge/executor.rs:45`).

## Public API

All session/task methods on `Runtime` are `async` (`runtime.rs:197–772`). Two runtime-level accessors are deliberately **synchronous** — a host reads them while building its published state, and the daemon answers a report read synchronously.

| Method | Signature | Purpose |
|--------|-----------|---------|
| `Runtime::new()` | `(config, executor) -> Self` | Create runtime |
| `enabled_features()` | `() -> Option<&[String]>` | The enabled-feature set the executor published. `None` = unknown; `Some(vec![])` = an engine with nothing enabled |
| `report_for()` | `(&TaskId) -> Option<serde_json::Value>` | The report document the executor recorded for a completed task. `None` = no report available |
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

- `runtime.rs`: 39 tests covering session creation, submit, cancel, timeout, stale completion guard, event emission, multiple sessions, cancel_active, session timeout override
- `session.rs`: 13 tests for snapshot roundtrip, hydration, scope, capabilities, close
- `request.rs`: 5 roundtrip, surface-label, and wireless-active dry-run/legacy-payload tests
- `event.rs`: 4 roundtrip and terminal-state tests
- `capabilities.rs`: 10 conservative/full/noop mode tests, `supports_task_kind` positive/negative
- `report.rs`: 6 tests — the store starts empty, an inserted document reads back unchanged, an unknown task has no report, retention is bounded and evicts the oldest first, re-inserting a task replaces its document, and clearing drops everything

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
12. **Phase 3 closure (2026-09-11)**: runtime types answer transport/session questions; engine contracts answer operation questions. `RuntimeSurface` is a wire DTO (canonical owner: `eggsec::config::ExecutionSurface`; exhaustive bidirectional bridge in `runtime_bridge::surface`). `TaskKind::operation_id()`/`canonical_target()` is the single wire-side match (engine adapters delegate). `race_with_cancel` (`cancel.rs`) is the shared cancellation primitive for both executors. Backpressure: broadcast lag is recoverable (receiver logs + continues); `ExecutionSink` coalesces non-critical progress with an explicit counter while findings/terminal outcomes always deliver.
13. **Report retention is bounded**: `RuntimeReportStore` keeps at most `MAX_RETAINED_REPORTS` (64) report documents and evicts the oldest first. The bound is what stops a long-lived daemon accumulating one report document per task for the life of the process
14. **`None` is a value, not a gap**: `RuntimeTaskExecutor::enabled_features()` returning `None` means "unknown" and `Some(vec![])` means "an engine with nothing enabled" — two different facts. `report_for()` returning `None` means "no report for this task", covering both "this executor produces no reports" and "this result has no report form"

## See Also

- [daemon.md](daemon.md) — Daemon host that owns the Runtime
- [ui_model.md](ui_model.md) — View DTOs derived from runtime types
- [overview.md](overview.md) — System-wide architecture
- [runtime_bridge.md](runtime_bridge.md) — Engine-side surface conversion and approval
- [tui.md](tui.md) — TUI that consumes runtime events
- [cli_commands.md](cli_commands.md) — CLI commands that dispatch through runtime

*Last verified against source: 2026-10-24 (`RuntimeTaskExecutor::enabled_features()` and `report_for()` added as defaulted methods; `Runtime::enabled_features()` and `Runtime::report_for()`; the bounded `RuntimeReportStore` and `MAX_RETAINED_REPORTS`); 2026-09-11 (Phase 3 closure); type lines and test counts re-verified 2026-09-25; full re-verification 2026-10-06 (TaskKind 30 variants incl. `Resume`); 2026-10-23 (`RuntimeSurface` 10 → 11 variants with `DesktopGuiManual`; optional `schema` feature added)*
