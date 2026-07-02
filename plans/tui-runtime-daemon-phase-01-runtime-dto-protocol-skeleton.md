# Phase 1 Plan: Runtime DTO and Protocol Skeleton

## Goal

Introduce a frontend-neutral runtime DTO and protocol skeleton for Eggsec without changing current TUI behavior. This phase creates the architectural seam that later phases will use to move task lifecycle, worker dispatch, result events, daemon transports, and pluggable frontends out of `eggsec-tui`.

The immediate deliverable is a new runtime-facing API surface that can compile independently of Ratatui/crossterm and can serialize stable commands, task identifiers, task states, runtime events, and session snapshots.

## Current Problem

The current TUI contains the effective interactive job protocol inside `eggsec-tui/src/workers/runner.rs`. `TaskConfig` and `TaskResult` are TUI-local. They include engine concepts such as load tests, port scans, recon, fuzzing, WAF, GraphQL, OAuth, packet tasks, DB pentest, intercept proxy, C2, browser, compliance, storage, integrations, workflow, vuln, and wireless tasks.

This blocks a daemon architecture because a non-TUI frontend cannot submit the same task requests or consume the same results without depending on `eggsec-tui`. It also makes it hard to add a codegg-style daemon, because the daemon would have to either import terminal UI code or duplicate the request/result model.

## Desired End State for This Phase

A new runtime DTO surface exists and builds without terminal dependencies. The current TUI still runs exactly as before. The new DTOs do not yet need to drive execution. This is an additive phase.

## Proposed Crate Layout

Prefer adding a new workspace crate:

```text
crates/eggsec-runtime/
  Cargo.toml
  src/lib.rs
  src/ids.rs
  src/request.rs
  src/event.rs
  src/session.rs
  src/capabilities.rs
  src/error.rs
```

Update root `Cargo.toml` workspace members to include `crates/eggsec-runtime`.

The crate should depend only on low-risk shared dependencies:

- `serde`
- `serde_json` for tests or helpers if needed
- `thiserror`
- `chrono` if timestamps are needed now
- `uuid` for IDs if aligned with existing workspace usage
- `eggsec-core`
- optionally `eggsec` only if DTOs need existing execution surface types; prefer avoiding full engine dependency in the first pass if practical

Do not depend on:

- `eggsec-tui`
- `ratatui`
- `crossterm`
- terminal/theme/session UI modules

## DTOs to Add

### IDs

Add opaque IDs:

```rust
pub struct SessionId(Uuid);
pub struct TaskId(Uuid);
pub struct ClientId(Uuid);
```

Requirements:

- Derive or implement `Clone`, `Copy` where appropriate, `Debug`, `Display`, `Serialize`, `Deserialize`, `PartialEq`, `Eq`, `Hash`.
- Provide constructors such as `SessionId::new()`, `TaskId::new()`, `ClientId::new()`.
- Avoid exposing inner UUID mutation.

### Runtime Request Types

Add a minimal request model:

```rust
pub struct RunRequest {
    pub task_kind: TaskKind,
    pub requested_by: Option<ClientId>,
    pub surface: RuntimeSurface,
    pub labels: Vec<String>,
}
```

Add `RuntimeSurface` as either:

- a wrapper around existing `eggsec::config::ExecutionSurface`, if depending on `eggsec` is acceptable; or
- a local mirror with conversion later, if keeping `eggsec-runtime` engine-light is preferred.

Preferred for Phase 1: define a local serializable `RuntimeSurface` with variants matching the current conceptual surfaces:

- `CliManual`
- `CliManualStrict`
- `TuiManual`
- `Ci`
- `McpServer`
- `RestApi`
- `SecurityAgent`
- `Unknown`

Later phases can implement conversion to/from `eggsec::config::ExecutionSurface`.

### TaskKind

Define `TaskKind` as a frontend-neutral request enum. In Phase 1, it does not need to cover every detail perfectly, but it should cover the existing TUI task categories enough to guide migration.

Expected initial variants:

- `LoadTest`
- `StressTest`
- `PortScan`
- `EndpointScan`
- `Fingerprint`
- `Fuzz`
- `Waf`
- `WafStress`
- `Pipeline`
- `Recon`
- `PacketCapture`
- `PacketTraceroute`
- `PacketSend`
- `GraphQl`
- `OAuth`
- `AuthTest`
- feature-gated or capability-tagged variants for `Nse`, `Hunt`, `Browser`, `Compliance`, `Storage`, `Integrations`, `Workflow`, `Vuln`, `Wireless`, `WirelessActive`, `DbPentest`, `Intercept`, `C2`

Keep the struct payloads serializable and TUI-free. If a payload currently uses a TUI-local option struct, define a neutral option struct here and add conversion later.

Do not move execution yet. Duplicating some shape from `eggsec-tui::workers::TaskConfig` is acceptable in this phase.

### TaskStatus and Progress

Add:

```rust
pub enum TaskStatus {
    Queued,
    Running,
    Completing,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

pub struct TaskProgress {
    pub completed: u64,
    pub total: Option<u64>,
    pub message: Option<String>,
}
```

Use `Option<u64>` for `total` so streaming or unbounded tasks are supported later.

### Runtime Events

Add:

```rust
pub enum RuntimeEvent {
    SessionCreated { session_id: SessionId },
    Snapshot { session_id: SessionId, snapshot: SessionSnapshot },
    TaskQueued { session_id: SessionId, task_id: TaskId, request: RunRequest },
    TaskStarted { session_id: SessionId, task_id: TaskId },
    TaskProgress { session_id: SessionId, task_id: TaskId, progress: TaskProgress },
    TaskLog { session_id: SessionId, task_id: Option<TaskId>, level: LogLevel, message: String },
    PolicyDecisionRequired { session_id: SessionId, task_id: Option<TaskId>, prompt: PolicyPrompt },
    TaskCompleted { session_id: SessionId, task_id: TaskId, outcome: TaskOutcome },
    TaskFailed { session_id: SessionId, task_id: TaskId, error: RuntimeErrorInfo },
    TaskCancelled { session_id: SessionId, task_id: TaskId, reason: Option<String> },
    Audit { session_id: SessionId, event: RuntimeAuditEvent },
}
```

For Phase 1, keep `TaskOutcome` generic enough to avoid deep result migration:

```rust
pub enum TaskOutcome {
    Json(serde_json::Value),
    Text(String),
    Artifact { artifact_id: String, summary: Option<String> },
    Empty,
}
```

Later phases can add typed outcomes.

### Session Snapshot

Add:

```rust
pub struct SessionSnapshot {
    pub session_id: SessionId,
    pub active_tasks: Vec<TaskSnapshot>,
    pub completed_tasks: Vec<TaskSnapshot>,
    pub capabilities: RuntimeCapabilities,
}
```

Add `TaskSnapshot` with task ID, status, request summary, progress, timestamps if available, and last error/summary.

### Capabilities

Add a capability model:

```rust
pub struct RuntimeCapabilities {
    pub task_kinds: Vec<TaskCapability>,
    pub transports: Vec<String>,
    pub supports_cancellation: bool,
    pub supports_multiple_sessions: bool,
    pub supports_multiple_active_tasks: bool,
}
```

Capabilities should eventually drive tab availability. For Phase 1, this can be simple.

## Implementation Steps

1. Add `crates/eggsec-runtime/Cargo.toml`.
2. Add module skeleton under `crates/eggsec-runtime/src/`.
3. Add ID types and serde tests.
4. Add `RuntimeSurface` and protocol DTOs.
5. Add `TaskKind` with neutral payload structs for the current TUI task set.
6. Add `RuntimeEvent`, `TaskOutcome`, `RuntimeErrorInfo`, `PolicyPrompt`, and audit placeholder types.
7. Add `RuntimeCapabilities` and `SessionSnapshot`.
8. Add JSON round-trip tests for IDs, a representative `RunRequest`, a representative `RuntimeEvent`, and a `SessionSnapshot`.
9. Update workspace `Cargo.toml` to include `eggsec-runtime`.
10. Optionally add `eggsec-runtime` as a dev-only or normal dependency to `eggsec-tui` only if needed for compile smoke tests. Do not migrate TUI code in this phase unless trivial.

## Files Likely to Inspect

- `Cargo.toml`
- `crates/eggsec-tui/Cargo.toml`
- `crates/eggsec-tui/src/workers/runner.rs`
- `crates/eggsec-tui/src/app/task_runtime.rs`
- `crates/eggsec-tui/src/app/state_update.rs`
- `crates/eggsec/src/config/*`
- `crates/eggsec/src/commands/registry.rs`
- `crates/eggsec/src/cli.rs`

## Non-goals

Do not move TUI task execution yet.

Do not change `eggsec-tui::run()` behavior.

Do not introduce daemon transport yet.

Do not remove existing `TaskConfig` or `TaskResult` from the TUI crate in this phase.

Do not make the protocol perfect. It only needs to be stable enough to support the next extraction phases.

## Validation

Run:

```bash
cargo check -p eggsec-runtime
cargo test -p eggsec-runtime
cargo check -p eggsec-tui
cargo check -p eggsec-cli
```

If feature combinations are practical, also run:

```bash
cargo check -p eggsec-tui --features stress-testing,packet-inspection
cargo check -p eggsec-cli --features rest-api
```

## Acceptance Criteria

- `eggsec-runtime` is part of the workspace.
- `eggsec-runtime` has no dependency on `eggsec-tui`, Ratatui, or crossterm.
- Runtime DTOs serialize and deserialize cleanly.
- The current TUI and CLI compile unchanged.
- The roadmap for Phase 2 can depend on the new DTOs without needing to import TUI modules.
