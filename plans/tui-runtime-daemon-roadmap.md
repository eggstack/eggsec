# Eggsec TUI Runtime and Daemon Architecture Roadmap

## Purpose

This roadmap defines the migration path for turning the current Eggsec TUI from a local terminal application that owns task orchestration into a frontend over a reusable runtime. The desired end state is a codegg-style architecture where Eggsec can run as a daemon with multiple pluggable frontends while preserving the existing manual CLI/TUI operator-discretion model and the stricter MCP/agent automation model.

The current repository already has several good foundations: `eggsec`, `eggsec-core`, `eggsec-cli`, and `eggsec-tui` are separate workspace members; execution surfaces already distinguish CLI manual, TUI manual, CI, MCP server, REST API, and security agent behavior; and the TUI has internal tab/view abstractions. The main gap is that `eggsec-tui` still owns runtime responsibilities such as task request enums, task result enums, worker dispatch, task handles, timeout policy, progress/result channels, and direct result-to-tab mutation.

This roadmap should be implemented as an incremental architecture migration. Avoid a TUI rewrite. Preserve current TUI behavior while extracting reusable boundaries.

## Target Architecture

The intended long-term crate and responsibility split is:

- `eggsec-core`: dependency-light shared primitives, stable DTOs where appropriate, constants, and low-level domain types.
- `eggsec`: assessment engine, command handlers, operation metadata, enforcement evaluation, scope/config loading, and feature-gated capability implementation.
- `eggsec-runtime`: frontend-neutral sessions, task lifecycle, task registry, runtime commands/events, task cancellation, policy prompt mediation, task history, event fanout, and embedded-runtime APIs.
- `eggsec-daemon`: optional long-running host for `eggsec-runtime` with local socket/WebSocket/SSE/gRPC transports.
- `eggsec-cli`: direct command entrypoint and daemon client. It may optionally include TUI support behind a feature, but should support headless builds without TUI dependencies.
- `eggsec-tui`: terminal frontend only. It owns Ratatui rendering, crossterm input, local view state, overlays, themes, tab focus/input buffers, and event-to-view reducers. It should not own engine execution.

The core invariant is that a frontend should be replaceable. Any behavior needed by a web frontend, daemon client, mobile frontend, codegg integration, or remote TUI must not live only in `eggsec-tui`.

## Safety and Enforcement Invariants

The migration must preserve the existing security model:

- Manual CLI/TUI operation remains operator-discretion-oriented.
- Agent, MCP, CI, and daemon automation remain strict by execution surface.
- Frontends do not decide enforcement. They declare execution surface/client identity; runtime and engine enforcement decide whether an operation is allowed, denied, warned, or requires manual confirmation.
- Manual confirmation must remain auditable and scoped to manual/permissive surfaces only.
- Remote/manual frontend support must not accidentally grant MCP/agent style clients the TUI's permissive manual override behavior.

## Phase 0: Architecture Inventory and Boundary Notes

Document current coupling before code movement. Classify the current TUI code into frontend view state, frontend input state, runtime/session state, and engine execution. The expected output is an architecture note describing what must remain TUI-local and what must move to runtime.

Key findings to encode:

- `App` currently mixes view state, session state, task runtime, enforcement facade, history, theme state, and terminal redraw flags.
- `eggsec-tui::workers` currently behaves as a local runtime and dispatch layer.
- `TaskConfig` and `TaskResult` currently define an interactive job protocol inside the TUI crate.
- `App::spawn_task` owns task handles, timeout, progress/result channels, and cancellation.
- `App::handle_result` mutates specific tabs directly from task results.

Success criteria:

- A committed architecture note exists.
- No behavior changes are required.
- The note defines the target boundaries and execution-surface policy matrix.

## Phase 1: Runtime DTO and Protocol Skeleton

Introduce a frontend-neutral runtime surface without changing execution behavior. Add `eggsec-runtime` as a workspace crate or an internal `eggsec::runtime` module if crate churn must be avoided. Prefer a crate because future TUI, daemon, CLI, and transport layers should depend on it without importing terminal dependencies.

Define serializable DTOs:

- `SessionId`
- `TaskId`
- `RunRequest`
- `TaskKind`
- `TaskStatus`
- `TaskProgress`
- `TaskOutcome`
- `RuntimeEvent`
- `RuntimeError`
- `SessionSnapshot`
- `RuntimeCapabilities`
- `FrontendSurface` or reuse `ExecutionSurface` where appropriate

Do not depend on Ratatui, crossterm, tab structs, `InputGroup`, `ScrollableText`, or any TUI module. Avoid result types that live under `eggsec-tui::tabs`.

Success criteria:

- `eggsec-runtime` builds without `eggsec-tui`.
- DTOs derive `Serialize`, `Deserialize`, `Debug`, and appropriate clone/eq traits.
- DTO JSON round-trip tests exist.
- Existing TUI behavior remains unchanged.

## Phase 2: Task Lifecycle Extraction

Move task spawning, task handles, cancellation, timeout policy, task IDs, progress/result channels, and active-task bookkeeping out of `App` into the runtime layer.

The TUI should submit runtime requests and receive runtime events. It may keep a small map from `TaskId` to initiating tab for rendering. The runtime should own cancellation and timeouts.

Expected runtime API shape:

```rust
Runtime::create_session(config) -> SessionId
Runtime::submit(session_id, request) -> TaskId
Runtime::cancel(session_id, task_id) -> Result<()>
Runtime::snapshot(session_id) -> SessionSnapshot
Runtime::subscribe(session_id) -> Receiver<RuntimeEvent>
```

Preserve current behavior where only one active task is supported, unless the implementation deliberately introduces multi-task support. Do not conflate current UI pause behavior with runtime pause; current pause freezes event consumption while the task continues.

Success criteria:

- `App` no longer owns raw task `JoinHandle`s.
- Runtime owns task timeout and cancellation.
- TUI task launch still works as before.
- Unit tests cover submit, progress delivery, completion, timeout, and cancellation.

## Phase 3: Worker Dispatch Migration

Move execution dispatch out of `eggsec-tui/src/workers` and into `eggsec-runtime` or the engine crate. The runtime should translate neutral `TaskKind` values into engine calls.

This phase can preserve the current match-based dispatch design initially. The important boundary is that the frontend no longer owns how a WAF test, recon run, port scan, packet task, OAuth test, GraphQL test, DB pentest, proxy intercept task, or C2 task is executed.

Feature gates must remain compatible with current behavior. If a feature-gated task is unavailable, runtime capability discovery and errors should report that cleanly.

Success criteria:

- `eggsec-tui/src/workers` is deleted or reduced to thin compatibility glue.
- Runtime dispatch covers the same task set as the existing TUI workers.
- No runtime module imports TUI tab modules.
- Tests cover representative base and feature-gated task dispatch.

## Phase 4: Runtime Event Reducer and TUI Adapter

Convert result handling from direct TUI mutation into an event reducer. Runtime emits events; the TUI maps events to view updates.

The TUI should introduce a reducer/adapter layer such as `TuiRuntimeAdapter` or `RuntimeEventReducer`. It should consume `RuntimeEvent` and mutate tabs, history display, notifications, and progress views.

Runtime events should include at least:

- `TaskQueued`
- `TaskStarted`
- `TaskProgress`
- `TaskLog`
- `TaskFinding`
- `PolicyDecisionRequired`
- `TaskCompleted`
- `TaskFailed`
- `TaskCancelled`
- `AuditEvent`

Success criteria:

- `App::handle_result` is no longer the canonical task-result path.
- Results are associated with `TaskId` and session, not only with current tab.
- Existing visual rendering remains stable.
- Tests verify that runtime events update the correct tab even if the current tab changed during execution.

## Phase 5: Session/View State Split

Split canonical runtime session state from local TUI view state. The runtime owns config snapshot, loaded scope, execution surface, task registry, active/completed task records, audit stream, and result records. The TUI owns current tab, input mode, overlays, focus state, command palette, theme state, bookmarks, scroll offsets, and rendered buffers.

Tabs may keep input buffers and formatted display content, but task results should exist in runtime state as well. This enables reconnecting frontends and non-TUI clients.

Success criteria:

- Runtime sessions can be created and tested without constructing `App`, Ratatui, or crossterm.
- TUI can be constructed around an existing runtime session handle.
- Session/task state is queryable independently of terminal rendering.
- Current session restore behavior is preserved or explicitly migrated.

## Phase 6: Embedded Runtime Compatibility Closure

Close the first milestone by ensuring the TUI still behaves exactly like the pre-refactor local application when running in embedded mode. This is a stabilization phase.

Success criteria:

- `eggsec` with no command still launches the TUI as before.
- Embedded TUI can run all previously supported tasks.
- Scope/enforcement behavior is unchanged for TUI manual mode.
- Headless command execution remains unchanged.
- `cargo check` and relevant tests pass under default and selected feature combinations.

## Phase 7: Local Daemon MVP

Add an optional daemon host for the runtime. Start with a local-only transport such as Unix domain socket on Unix/macOS/Linux. Add loopback TCP or named-pipe support later as needed.

The daemon should support session creation, attach/list sessions, task submission, cancellation, event subscription, and snapshot retrieval. It should not reimplement task logic; it should host `eggsec-runtime`.

Success criteria:

- Daemon runs headless.
- A minimal CLI client can submit a task and stream events.
- Cancellation works through daemon transport.
- Audit records include client identity and execution surface.

## Phase 8: TUI Remote Attach Mode

Allow `eggsec-tui` to run against either embedded runtime or a remote/local daemon runtime. Embedded mode should remain the default until daemon mode is mature.

Success criteria:

- TUI can connect to an existing daemon.
- TUI can attach to an existing session.
- Reconnect restores task status and recent results.
- Embedded mode and daemon mode use the same protocol commands/events.

## Phase 9: CLI Headless and Daemon Cleanup

Refactor CLI packaging so headless builds do not need terminal dependencies. Add daemon-client CLI operations for listing sessions, attaching, submitting tasks, streaming events, and cancelling tasks.

Success criteria:

- `eggsec-cli` can build without `eggsec-tui` behind an appropriate feature split.
- Existing direct commands remain compatible.
- Daemon client commands work with the local daemon.

## Phase 10: Multi-session and Multi-client Semantics

Add true multi-session behavior and event fanout. Multiple frontends should be able to observe one session without corrupting runtime state.

Success criteria:

- Multiple clients can subscribe to one session.
- Runtime state is canonical and frontend rendering state is local.
- Policy approvals have explicit authorization rules.
- Stale clients do not block task completion.

## Phase 11: Persistence, Artifacts, and Resumability

Persist session metadata, task records, results, audit events, and artifacts. Artifacts should be first-class references, not raw local file assumptions.

Success criteria:

- Daemon restart can list previous sessions.
- Completed task records survive restart.
- Artifact references can be fetched through daemon APIs.
- TUI reconnect can show recent session state.

## Phase 12: WebSocket/SSE/gRPC Transport Expansion

Expose the same protocol over additional transports after local daemon mode is stable. Prefer WebSocket or REST/SSE first for browser/desktop/mobile friendliness. Add gRPC if stronger typed clients are valuable.

Success criteria:

- A WebSocket or SSE client can create a session, submit a task, receive events, cancel a task, and fetch artifacts.
- Transport code remains thin.
- Strict execution surfaces remain strict.

## Phase 13: Frontend Capability and Plugin Model

Define frontend capabilities in terms of runtime protocol capabilities rather than compile-time TUI tabs. TUI tabs should become views over runtime capabilities.

Success criteria:

- A mock non-TUI frontend can use the same runtime protocol.
- Tab availability can be derived from runtime capabilities.
- Adding a new task type does not require adding execution dispatch to `eggsec-tui`.

## Phase 14: Final Boundary Hardening

Remove transitional duplication, tighten feature flags, and add dependency-boundary tests. Document where new task types, result types, transports, and frontend views should be added.

Success criteria:

- `eggsec-runtime` does not depend on Ratatui/crossterm.
- `eggsec-tui` does not own engine execution.
- Daemon builds without TUI.
- Headless CLI builds without TUI.
- Tests enforce crate-boundary expectations.

## Validation Strategy

Each phase should include at least:

- `cargo check --workspace` under default features where feasible.
- Targeted checks for `eggsec-tui` and any newly introduced runtime crate.
- Feature-gated checks for at least `rest-api`, `stress-testing`, `packet-inspection`, and one higher-risk optional surface if practical.
- Regression tests for TUI task launch/cancel/result routing.
- Policy tests that verify TUI manual, CLI manual, MCP, CI, and agent surfaces retain distinct behavior.

## Implementation Principle

Prefer reversible extraction over broad rewrites. The migration should keep existing TUI functionality intact while moving one ownership boundary at a time: DTOs first, task lifecycle second, worker dispatch third, event reduction fourth, and session/view split fifth. Do not build the daemon before the runtime boundary exists, or the daemon will fossilize the current TUI-owned execution model behind a network API.
