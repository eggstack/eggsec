# Phase 13 Plan: Frontend Plugin and Component Model

## Goal

Define a stable frontend extension model for Eggsec now that the runtime/daemon boundary exists. The objective is to let future frontends, TUI panels, dashboards, and harness integrations consume daemon/runtime state without reaching into TUI internals or duplicating task execution logic.

This phase is about extension seams and component contracts, not arbitrary code execution.

## Current Baseline

The architecture now has:

- runtime sessions and snapshots;
- daemon protocol and local daemon transport;
- TUI embedded/daemon runtime clients;
- CLI daemon client operations;
- role-based daemon authorization;
- protocol-neutral result envelopes;
- planned persistence/artifact handling.

The next risk is frontend drift: each frontend could start inventing its own task/result/session model. This phase prevents that by defining common frontend-facing components and view DTOs.

## Scope

Add a frontend component model for:

- session list views;
- task list/history views;
- task progress/status rendering;
- result envelope rendering;
- artifact lists;
- permission/status indicators;
- policy prompt state, if wired later;
- dashboard summaries.

Do not implement untrusted plugin execution in this phase. If plugins are needed, keep them declarative or compile-time registered.

## Component Contract

Define frontend-neutral view DTOs separate from runtime internals.

Suggested module:

```text
crates/eggsec-runtime/src/view.rs
```

or a new lightweight crate:

```text
crates/eggsec-ui-model
```

Preferred if DTOs start to grow: `eggsec-ui-model`, because runtime should remain lifecycle/protocol focused.

Suggested DTOs:

```rust
SessionView
TaskView
TaskProgressView
ResultEnvelopeView
ArtifactView
PermissionView
PolicyPromptView
DashboardSummaryView
```

These should be serializable and usable by TUI, CLI JSON, web UI, and future desktop/mobile clients.

## Result Rendering Registry

Add a registry that maps `TaskResultEnvelope.kind` to a view renderer descriptor.

Descriptor example:

```rust
pub struct ResultRendererDescriptor {
    pub kind: &'static str,
    pub title: &'static str,
    pub summary_fields: &'static [&'static str],
    pub artifact_kinds: &'static [&'static str],
    pub supports_rich_tui: bool,
    pub supports_json_detail: bool,
}
```

This is not a code plugin system. It is metadata for consistent frontend rendering.

## TUI Component Adapter

Refactor TUI rendering to consume view DTOs where practical.

Targets:

- dashboard session/task summaries;
- history/completed task list;
- daemon attach envelope-only task completion;
- artifact list display;
- permission/role display.

Do not rewrite every tab. Start with daemon-facing components where remote mode currently has summary-only rendering.

## CLI JSON Consistency

CLI daemon commands should reuse the same view DTOs for JSON output.

Examples:

```text
eggsec session list --json
eggsec session snapshot --json
eggsec task watch --json
```

Output should be stable enough for agents/harnesses to parse.

## Frontend Capability Metadata

Expose frontend-relevant capabilities:

- available task kinds;
- result kinds and renderer descriptors;
- artifact kinds;
- supported transports;
- supported permission roles;
- policy approval support status.

This should be daemon capability metadata, not runtime-only capability metadata.

## Plugin Boundary Decision

If true plugins are desired later, document the path but do not implement dynamic code loading here.

Potential later options:

- WASM UI plugins with restricted host API;
- static Rust plugin registry;
- JSON schema-driven panels;
- external process frontends consuming daemon protocol.

For this phase, prefer static descriptors and view DTOs.

## Files Likely to Change

- `crates/eggsec-runtime/src/event.rs`
- `crates/eggsec-runtime/src/session.rs`
- `crates/eggsec-ui-model/*` if added
- `crates/eggsec-daemon/src/protocol.rs`
- `crates/eggsec-daemon/src/host.rs`
- `crates/eggsec-cli/src/daemon_cli.rs`
- `crates/eggsec-tui/src/app/runtime_adapter/mod.rs`
- `crates/eggsec-tui/src/tabs/*`
- `architecture/tui.md`
- `architecture/overview.md`

## Tests

- View DTO serialization round-trip.
- Result renderer descriptor exists for every known envelope kind.
- CLI JSON uses view DTOs for sessions/tasks.
- TUI daemon attach can render envelope-only completion through shared view adapter.
- Unknown result kind degrades gracefully.
- Permission view accurately distinguishes owner/controller/observer/approver.

## Non-Goals

Do not add arbitrary dynamic plugin execution.

Do not add WASM execution yet.

Do not replace all tab-specific rich rendering.

Do not expose unsafe artifact contents through UI DTOs.

Do not make frontend descriptors bypass daemon authorization.

## Validation

Run:

```bash
cargo fmt --all --check
cargo check -p eggsec-runtime
cargo test -p eggsec-runtime
cargo check -p eggsec-daemon
cargo test -p eggsec-daemon
cargo check -p eggsec-cli
cargo test -p eggsec-cli
cargo check -p eggsec-tui
cargo test -p eggsec-tui
./scripts/check-architecture-guards.sh
```

## Acceptance Criteria

- Frontends can consume stable session/task/result/artifact view DTOs.
- Result envelope rendering is described by a registry/descriptor model.
- TUI and CLI share daemon-facing view semantics.
- Unknown result kinds render safely.
- Plugin execution remains out of scope and documented as future work.
- Runtime/daemon authorization remains the only authority for actions.
