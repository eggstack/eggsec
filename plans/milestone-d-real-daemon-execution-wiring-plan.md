# Milestone D: Real Daemon Execution Wiring Plan

## Objective

Replace the daemon's protocol-only/noop execution path with a real Eggsec runtime executor adapter while preserving a lightweight/noop mode for protocol tests and minimal daemon startup.

After this milestone, `eggsec-daemon` should be able to execute an initial set of real Eggsec tasks through the same enforcement path used elsewhere:

```text
RunRequest -> RuntimeSurface -> ExecutionSurface -> OperationDescriptor -> EnforcementContext -> ApprovedOperation -> EnforcedDispatcher::dispatch_checked()
```

This milestone depends on Milestone A runtime correctness, Milestone B daemon hardening, and Milestone C runtime/enforcement bridge.

## Current Findings

`crates/eggsec-daemon/src/main.rs` currently wires `NoopExecutor`, which rejects all tasks with `UnsupportedTaskKind`. The comments explicitly identify real dispatch wiring as later work. This means the daemon can host protocol/session flows but cannot yet function as a real backend for CLI/TUI/desktop/mobile frontends.

The daemon should become an optional canonical backend for persistent/multi-frontend workflows, not a mandatory dependency for normal CLI use.

## Design Requirements

### Preserve dependency-light runtime

Do not make `eggsec-runtime` depend on the main `eggsec` crate. Real execution must be supplied by an adapter that implements `RuntimeTaskExecutor`.

### Preserve protocol-only daemon mode

The daemon should still be able to start in a lightweight/noop or protocol-only mode for tests, development, and future minimal packaging. This mode must advertise that task execution is unavailable or limited.

### Preserve manual vs automated semantics

A daemon-backed manual CLI/TUI session remains manual. It may warn or require targeted operator confirmation. A daemon-backed MCP/agent/REST/gRPC session remains strict and cannot honor manual overrides.

### Dispatch through enforcement

The real executor must not call raw tool dispatch directly. It must use the bridge from Milestone C to obtain an `ApprovedOperation`, then dispatch through `EnforcedDispatcher::dispatch_checked()` or an equivalent enforced orchestrator path.

## Work Item D1: Decide Adapter Location and Feature Shape

### Recommended structure

Add the adapter in the main `eggsec` crate, because it can depend on both the runtime DTOs and the internal dispatcher/enforcement machinery.

Suggested module:

```text
crates/eggsec/src/runtime_executor.rs
```

or:

```text
crates/eggsec/src/runtime_bridge/executor.rs
```

Expose a type like:

```rust
pub struct EggsecRuntimeExecutor { ... }
```

that implements:

```rust
eggsec_runtime::RuntimeTaskExecutor
```

The daemon can then depend on the main `eggsec` crate behind a feature, or a small composition crate can be added later if dependency boundaries become too heavy.

### Feature options

Option A: Add feature to `eggsec-daemon`:

```toml
full-executor = ["dep:eggsec"]
```

Default remains noop/protocol-only.

Option B: Create a new binary or feature-gated main path:

```text
eggsec-daemon --executor noop
eggsec-daemon --executor eggsec
```

Option A is simpler for the first pass. Option B is better UX once configuration is stable.

## Work Item D2: Build `EggsecRuntimeExecutor`

### Responsibilities

The executor should:

1. receive `task_id`, `RunRequest`, `RuntimeEventSink`, and cancellation token;
2. convert runtime surface to enforcement surface through the Milestone C bridge;
3. resolve policy and loaded scope for the session/request;
4. convert the runtime request to an operation descriptor;
5. approve the operation using the correct manual or strict path;
6. convert the runtime request to the internal tool/orchestrator request;
7. dispatch through enforced dispatch;
8. emit progress/log/completion/failure events through the runtime sink;
9. respect cancellation before and during execution where tool paths support cancellation.

### Policy and scope inputs

The executor needs access to policy and loaded scope. There are several viable designs.

Preferred first pass:

- session scope metadata remains in `RuntimeSession`;
- daemon host creates sessions with explicit scope metadata;
- executor has a policy provider and scope loader callback;
- when executing, the daemon/host passes enough context to resolve `LoadedScope` before approval.

If the current `RuntimeTaskExecutor::execute(...)` signature lacks session ID or session scope, update it carefully. It currently receives `task_id`, `RunRequest`, sink, and cancel. Because the sink contains session ID privately but does not expose it, the executor may need either:

- an expanded `execute(...)` signature including `session_id` and optional `SessionScope`; or
- a context object passed to executor; or
- a daemon-specific executor wrapper that can look up session metadata before submitting.

Prefer a clean context object if signature churn is acceptable:

```rust
pub struct RuntimeExecutionContext {
    pub session_id: SessionId,
    pub task_id: TaskId,
    pub surface: RuntimeSurface,
    pub scope: Option<SessionScope>,
    pub request: RunRequest,
}
```

Do not smuggle security-sensitive context through labels or request params.

### Manual approval handling

This first real executor pass does not need to implement full interactive approval if the daemon policy prompt path is still incomplete. It may return a structured `PolicyDecisionRequired` event and fail/pause the task until Milestone E/UI work if approval is required.

However, strict surfaces must fail closed immediately. They must never wait for manual approval.

For manual surfaces, choose one of:

1. return a `PolicyDecisionRequired` event and mark the task blocked/pending if runtime supports pending state;
2. fail with `ConfirmationRequired` until daemon approval plumbing is completed;
3. support manual override only if it is supplied by an explicit manual frontend request path.

Preferred: implement the policy prompt state if it already exists in `RuntimeEvent`; otherwise fail clearly and leave full manual prompt flow to the CLI/TUI backend milestone.

## Work Item D3: Convert Runtime Requests to Internal Tool Requests

### Desired behavior

For the initial supported task set, `RunRequest` should become the internal request type expected by `EnforcedDispatcher` or the orchestrator.

Start with low-risk and commonly useful task kinds:

- `PortScan`
- `EndpointScan`
- `Fingerprint`
- `Waf`
- `Pipeline`
- optionally `Recon`, `LoadTest`, `Fuzz`, and `WafStress` if existing mappings are straightforward

Do not wire hazardous task kinds first. Packet send, active wireless, dynamic mobile, DB pentest real runs, web proxy interception, postex, and C2 should wait until the bridge and approval model are mature.

### Implementation guidance

Use the same operation IDs and metadata as the command registry. Avoid parallel metadata definitions.

When converting, include target in both internal request target and params if existing code expects one or both. `EnforcedDispatcher::dispatch_checked()` validates the request target and `params["target"]` against the approved descriptor where applicable, so conversion must be consistent.

### Tests

For each supported task kind:

- create runtime request;
- bridge to descriptor;
- approve under a safe test policy/scope;
- convert to internal request;
- assert tool ID/target match the approved descriptor.

## Work Item D4: Wire Daemon to Real Executor Behind Feature

### Desired behavior

The daemon binary should be able to start with either noop executor or real Eggsec executor depending on feature/config.

Suggested behavior:

- default build: noop/protocol-only if keeping daemon lightweight is preferred;
- `--features full-executor` or equivalent: daemon can use `EggsecRuntimeExecutor`;
- CLI flag or config can select executor mode if both are compiled.

### Main changes

Update `crates/eggsec-daemon/src/main.rs`:

- parse executor mode from CLI/config if implemented in Milestone B;
- instantiate `NoopExecutor` for protocol-only mode;
- instantiate `EggsecRuntimeExecutor` for full mode;
- advertise correct capabilities.

If Rust type constraints make runtime selection between different executor concrete types awkward, use boxed trait objects or an enum executor wrapper.

## Work Item D5: Honest Capability Reporting

### Desired behavior

Frontends must be able to know whether the daemon can execute real tasks and which task kinds are available.

Extend capability output to include:

- executor mode: `noop`, `limited`, or `full`;
- supported runtime task kinds;
- unsupported/hazardous task kinds omitted unless wired;
- persistence enabled;
- enabled transports;
- policy approval support status.

If the protocol was already extended in Milestone B, populate those fields accurately here.

### Tests

- noop daemon advertises no real task execution;
- real executor daemon advertises the initial supported task kinds;
- unsupported task submission returns a typed unsupported error, not generic internal failure.

## Work Item D6: End-to-end Daemon Execution Tests

### Desired behavior

Add at least one e2e daemon test proving real execution works through the daemon protocol.

### Test shape

Use a deterministic low-risk operation and local target/fixture. Good candidates:

- fingerprint against a local mock HTTP server;
- endpoint scan against a local wiremock server;
- port scan against a local listener;
- plan/dry-run style operation if it still exercises the enforced execution path.

Test flow:

1. start daemon host with real executor and temporary persistence;
2. declare a CLI or TUI client;
3. create a `CliManual` or `TuiManual` session with explicit localhost scope;
4. submit supported runtime task;
5. subscribe or poll until terminal event;
6. assert task completed successfully or produced expected controlled findings;
7. retrieve live snapshot;
8. retrieve persisted snapshot;
9. assert terminal state and request summary are present.

Add strict-surface test:

1. create `McpServer` or `SecurityAgent` session without explicit manifest;
2. submit a networked operation requiring explicit scope;
3. assert denial/failure before dispatch.

## Work Item D7: Cancellation Integration

### Desired behavior

Daemon cancellation should propagate to real tool execution where possible.

### Implementation guidance

`RuntimeTaskExecutor::execute()` receives a cancellation token. The executor should:

- check cancellation before policy evaluation;
- pass cancellation to tool requests if `ToolRequest` supports it;
- use `tokio::select!` around long-running operations where the underlying tool does not natively support cancellation;
- return a cancellation error or empty outcome consistently when cancelled.

Do not attempt perfect cancellation for every existing tool in this milestone. Focus on ensuring the adapter does not ignore cancellation entirely.

### Tests

Use a long-running controlled test executor or local operation if available. Assert daemon `CancelTask` results in terminal cancelled state and does not later become completed.

## Validation Commands

Run at minimum:

```bash
cargo test -p eggsec-runtime
cargo test -p eggsec-daemon
cargo test -p eggsec --lib runtime_bridge
cargo check -p eggsec-daemon --all-targets
cargo check -p eggsec --all-targets
```

For full executor feature:

```bash
cargo test -p eggsec-daemon --features full-executor
cargo check -p eggsec-daemon --features full-executor --all-targets
```

If feature names differ, update the commands in this plan during implementation.

## Acceptance Checklist

- [ ] A real `EggsecRuntimeExecutor` or equivalent adapter exists.
- [ ] The adapter uses the Milestone C bridge for surface, descriptor, and approval.
- [ ] Real daemon execution never bypasses `ApprovedOperation` enforcement.
- [ ] Noop/protocol-only daemon mode remains available.
- [ ] Daemon capabilities honestly report executor mode and supported task kinds.
- [ ] At least one real low-risk daemon task executes end-to-end through the protocol.
- [ ] Strict daemon sessions fail closed where expected.
- [ ] Cancellation propagates through the real executor path at least at adapter level.
- [ ] Tests cover supported and unsupported task kinds.

## Handoff Notes

This milestone should stop short of making CLI/TUI default to daemon mode. That belongs in the later CLI/TUI dual-backend milestone. The goal here is to make the daemon capable of real enforced execution so frontends can safely build on it.
