# Daemon/Frontend Separation and Core Infrastructure Roadmap

## Purpose

This roadmap defines the remaining work required to turn Eggsec's current runtime and daemon scaffolding into a true pluggable-frontend architecture while preserving the intended product split:

- Human-facing CLI and TUI are first-class manual operator surfaces.
- Manual CLI/TUI should remain practical and comparable to legitimate security tooling: warnings, targeted confirmations, and explicit operator overrides where appropriate.
- MCP, autonomous agent, REST, gRPC, and other programmatic surfaces must remain strict, fail-closed, and unable to honor manual overrides.
- The daemon is a runtime host and session coordinator, not synonymous with agent mode. Enforcement behavior is determined by the bound execution surface/profile, not by whether the runtime is embedded or daemon-backed.

The current repository already has many of the right pieces: `eggsec-runtime` contains frontend-neutral request/session/event DTOs; `eggsec-daemon` contains a protocol, session host, client registry, persistence scaffolding, and Unix socket transport; `eggsec` contains the central enforcement model and `EnforcedDispatcher`; CLI/TUI/manual surfaces are already separated conceptually from strict automated surfaces. The remaining work is closure: runtime correctness, daemon hardening, canonical runtime-to-enforcement bridging, and real daemon execution wiring.

## Target Operating Model

Eggsec should support three execution topologies.

### 1. Embedded manual mode

This is the default CLI/TUI path. The process links the Eggsec engine directly and executes in-process. It does not require a daemon. This mode is important for real-world use because a human operator should be able to run `eggsec scan`, `eggsec plan`, `eggsec waf-stress`, or TUI workflows without daemon lifecycle ceremony.

Embedded manual mode should use the same enforcement primitives as every other path, but with manual profiles:

- `CliManual` / `TuiManual` map to `ManualPermissive`.
- `CliManualStrict` / `TuiManualStrict` map to `ManualGuarded`.
- Manual override flags and interactive confirmations are honored only for manual permissive surfaces.
- Manual strict mode is available when the operator wants hard enforcement.

### 2. Daemon-backed manual mode

This is the persistent/multi-frontend path. CLI, TUI, desktop, mobile, or web clients attach to `eggsec-daemon`, create or resume sessions, submit tasks, subscribe to runtime events, cancel tasks, and inspect history.

This mode must preserve human-facing semantics. A daemon-backed CLI or TUI session bound as `CliManual` or `TuiManual` must not inherit MCP/agent strictness. The daemon is an execution host; it does not itself imply automation.

### 3. Strict automated mode

MCP, agent, CI, REST, and gRPC are programmatic surfaces. They must map to strict profiles and fail closed on warnings, confirmation requirements, and denials. Manual override fields must not exist in these schemas, and any equivalent model-supplied override intent must be ignored or rejected before dispatch.

Strict mode requires central approval:

`RuntimeSurface -> ExecutionSurface -> EnforcementContext::evaluate() -> ApprovedOperation -> EnforcedDispatcher::dispatch_checked()`

No strict programmatic path should reach raw dispatch.

## Architectural Goal

The final architecture should allow a new frontend to be written against a stable client protocol and DTO package without linking the full `eggsec` engine. A frontend should be able to:

1. declare client identity and kind;
2. negotiate daemon protocol version;
3. read daemon capabilities;
4. create or attach to a session;
5. submit a frontend-neutral `RunRequest`;
6. subscribe to runtime events;
7. render `SessionSnapshot` and task outcomes;
8. cancel tasks;
9. approve manual policy prompts only when the session surface is manual;
10. operate without knowing internal scanner, dispatcher, or TUI types.

The CLI remains first class in both embedded and daemon-backed forms.

## Milestone Sequence

### Milestone A: Runtime correctness closure

Fix runtime semantics before more infrastructure depends on them.

Scope:

- Honor `SessionOptions.task_timeout` instead of ignoring it.
- Preserve cancelled/replaced task history instead of dropping task records.
- Add stale-completion guards so a task cancelled or replaced by the runtime cannot later overwrite its terminal state.
- Clarify snapshot timestamp semantics.
- Improve runtime event-send handling so expected no-subscriber cases are harmless but unexpected channel failures are observable.

Acceptance criteria:

- Per-session timeouts work and are tested independently from global defaults.
- Replacing an active task leaves a terminal cancelled task in session history.
- A stale executor result cannot overwrite cancellation or timeout state.
- Snapshot timestamp fields have unambiguous names and serialization semantics.
- Runtime tests cover timeout, cancellation, stale completion, snapshot, and event behavior.

### Milestone B: Daemon host hardening

Make the daemon a robust session host before real execution is wired through it.

Scope:

- Enforce `DaemonConfig.max_clients`.
- Add a real daemon CLI parser and config controls.
- Persist session snapshots on terminal runtime events, not only on create/submit/cancel commands.
- Implement real close-session lifecycle semantics.
- Tighten persisted-session access controls.
- Improve capability reporting so clients know whether execution is available.

Acceptance criteria:

- Over-limit clients are rejected or closed deterministically.
- Daemon startup supports explicit socket, data-dir, persistence, max-clients, default-surface, and logging options.
- Completed/failed/cancelled/timed-out tasks are persisted without requiring another client command.
- Closed sessions cannot accept new tasks.
- Persisted snapshots are not globally readable by arbitrary declared clients unless explicitly configured.

### Milestone C: Runtime/enforcement bridge

Build the security-critical bridge from daemon/runtime DTOs to the main Eggsec enforcement model.

Scope:

- Implement exhaustive `RuntimeSurface -> ExecutionSurface` conversion.
- Implement `RunRequest -> OperationDescriptor` conversion for the initial supported task set.
- Implement manual and strict approval flows over runtime requests.
- Add tests proving automated surfaces are strict and manual surfaces preserve intended operator behavior.

Acceptance criteria:

- Every runtime surface maps explicitly to the correct enforcement surface.
- Automated surfaces never map to manual permissive behavior.
- Manual overrides are honored only for `CliManual` and `TuiManual` surfaces.
- Strict surfaces fail closed on `Warn`, `RequireConfirmation`, or `Deny`.
- Initial task kinds can produce operation descriptors without frontend/TUI dependencies.

### Milestone D: Real daemon execution wiring

Replace the daemon's `NoopExecutor` path with a real Eggsec executor adapter while preserving a lightweight/noop mode for protocol testing.

Scope:

- Add an `EggsecRuntimeExecutor` adapter outside the dependency-light runtime crate.
- Convert `RunRequest` into enforced tool/orchestrator execution.
- Ensure daemon execution uses the same `ApprovedOperation` path as strict embedded surfaces.
- Report honest daemon capabilities depending on the executor mode and enabled features.
- Add end-to-end daemon execution tests against localhost fixtures or deterministic low-risk operations.

Acceptance criteria:

- The daemon can execute at least the initial low-risk task set through real Eggsec dispatch.
- The daemon can still be built or run in protocol-only/noop mode for lightweight tests.
- CLI/TUI future daemon-backed clients can rely on the same protocol.
- End-to-end tests create a session, submit a real task, observe events, and retrieve a terminal snapshot.

## Follow-on Milestones

### Milestone E: CLI/TUI dual backend mode

Add embedded and daemon-backed backend selection for CLI/TUI. Embedded remains the default for normal manual use. Daemon-backed mode enables persistence, attach/reconnect, multi-client observation, and shared session state.

### Milestone F: Transport expansion and frontend SDK

Keep Unix socket as the default local transport. Add loopback HTTP/SSE and WebSocket behind explicit config. Consider gRPC only when typed external integrations justify it. Extract or formalize a daemon client library so frontends do not hand-roll protocol IO.

### Milestone G: Audit, docs, and release polish

Normalize audit events across embedded and daemon-backed execution. Document manual vs automated behavior clearly. Revisit the `full` feature name or documentation because it aggregates hazardous/lab capabilities. Add release validation for embedded manual, daemon-backed manual, MCP strict, agent strict, and REST/gRPC strict when enabled.

## Non-goals

- Do not force CLI/TUI to require the daemon.
- Do not make manual CLI/TUI inherit MCP/agent restrictions.
- Do not put policy authorization into domain crates.
- Do not let daemon transport/authentication decisions replace enforcement profile decisions.
- Do not expose manual override semantics to MCP, agent, REST, or gRPC request schemas.

## Recommended Execution Order

Execute milestones A through D in order. A and B remove lifecycle bugs that would otherwise become harder to reason about once daemon-backed execution is real. C establishes the trust-boundary conversion. D wires actual execution through the daemon only after the runtime and enforcement bridge are reliable.
