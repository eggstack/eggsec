# Phase 6 Plan: Pre-Daemon Runtime Readiness

## Goal

Convert the post-closure runtime/TUI architecture into a daemon-ready baseline without adding daemon transport yet. This phase verifies that the embedded runtime contract is stable, non-TUI clients can rely on runtime events and snapshots, and the remaining TUI-only bridges are explicitly isolated.

This phase is a gate. Do not build a daemon until this plan passes.

## Current Baseline

The repo now has the correct broad shape:

- `eggsec-runtime` owns IDs, runtime events, task lifecycle, sessions, snapshots, scope/surface binding, and protocol-neutral result envelopes.
- `eggsec::dispatch` owns canonical task execution and returns `TaskResult` directly.
- `eggsec-tui` submits `RunRequest` values, consumes runtime events, and uses `TuiRuntimeAdapter` for lifecycle/view updates.
- The TUI still uses typed channels for rich tab rendering, while runtime emits `TaskOutcome::Result(TaskResultEnvelope)` for non-TUI consumers.
- Architecture guards protect against reintroducing TUI workers and against TUI/transport dependency creep in `eggsec-runtime`.

## Primary Questions to Answer

This phase should produce clear answers to these questions:

1. Can a non-TUI in-process client create a runtime, create a session, submit a task, receive events, cancel a task, and inspect a snapshot?
2. Do completed task events always include useful protocol-neutral outcome data?
3. Are capabilities accurate under the current feature set?
4. Are surface/scope bindings preserved across session creation and snapshots?
5. Are TUI-only rendering paths clearly optional for non-TUI clients?

## Workstream 1: In-Process Client Contract Tests

Add integration-style tests that exercise runtime behavior without constructing `eggsec-tui::App`.

Suggested test module:

```text
crates/eggsec-runtime/tests/in_process_client.rs
```

If integration tests need the engine dispatcher, place the tests in `crates/eggsec/tests/` or `crates/eggsec/tests/runtime_dispatch.rs` to avoid circular dependencies.

Test coverage:

- Create runtime with a test executor.
- Create session with `RuntimeSurface::CliManual`.
- Create session with `RuntimeSurface::McpServer`.
- Bind `SessionScope` and verify `snapshot.scope`.
- Submit a representative task request.
- Receive `TaskQueued`, `TaskStarted`, and terminal event.
- Verify terminal event contains non-empty `TaskOutcome::Result` or clear structured failure.
- Cancel a long-running task and verify `TaskCancelled` plus snapshot status.
- Query `Runtime::snapshot()` and verify active/completed task lists.

Acceptance criteria:

- Tests prove runtime can be used independently of TUI.
- Tests do not import `eggsec-tui`.
- Snapshots and events are sufficient for a minimal daemon client.

## Workstream 2: Result Envelope Completeness Audit

Audit `task_result_to_envelope()` and ensure each `eggsec::dispatch::TaskResult` variant maps to a meaningful envelope.

Requirements per variant:

- Stable `kind` string.
- Non-empty summary for successful and failed results.
- Payload is either meaningful JSON or intentionally `{}` with a comment explaining why.
- Artifact references are populated for result types that write files or produce output paths.

Suggested outcome quality tiers:

- Tier 1: `kind` + summary only. Acceptable for early daemon MVP.
- Tier 2: `kind` + summary + key scalar payload fields. Preferred for common tasks.
- Tier 3: artifact refs or full structured payload. Needed for rich frontend parity.

Minimum closure target:

- Common tasks such as port scan, endpoint scan, recon, fingerprint, load test, WAF, GraphQL, OAuth, auth, DB pentest, packet tasks, intercept, and C2 have useful summaries.
- File/output-producing tasks attach `ArtifactRef` when an output path is known.

Implementation steps:

1. Inventory every `TaskResult` variant.
2. Add a table in comments or docs mapping variant -> envelope kind -> summary/payload/artifact behavior.
3. Add unit tests for representative common, error, and feature-gated variants.
4. Add a guard or test that fails when a new `TaskResult` variant lacks an envelope mapping.

Acceptance criteria:

- No `TaskResult` variant silently maps to an unhelpful generic envelope.
- Non-TUI consumers can show useful completion information for common tasks.
- Artifact-producing results expose artifact refs.

## Workstream 3: Capability Reporting Audit

Runtime capability reporting must reflect implemented behavior exactly.

Checks:

- `transports` should only include `in-process` in this phase.
- `supports_multiple_active_tasks` must be false unless runtime actually supports it under config.
- `supports_multiple_sessions` must match tested behavior.
- Feature-gated task capabilities should not mislead clients.

Implementation options for feature-gated task capabilities:

1. Keep `RuntimeCapabilities::default()` as baseline generic capability metadata, but document that it is build-level only.
2. Add `RuntimeCapabilities::for_build_features()` using `cfg` gates.
3. Add engine-provided capability discovery later, but stub accurately now.

Preferred for this phase: add `for_build_features()` or equivalent and use it for runtime/session capability snapshots.

Acceptance criteria:

- Capability tests verify no unimplemented transports are advertised.
- Capability tests verify single-active behavior.
- Feature-gated capability behavior is documented and covered by tests where feasible.

## Workstream 4: Embedded TUI Regression Tests

The runtime/TUI boundary should remain transparent to current users.

Add or update tests around:

- Starting a task from a tab registers the originating tab.
- Runtime completion routes to the initiating tab after tab switch.
- Typed result path still updates rich tab state.
- Envelope path still exists and is non-empty.
- Cancel/clear behavior does not leave dangling task mappings.
- Duplicate terminal events are ignored.

Acceptance criteria:

- `cargo test -p eggsec-tui -- runtime_adapter` covers edge cases.
- Existing tab result rendering tests still pass.
- There is no regression in manual TUI launch/task behavior.

## Workstream 5: Documentation and Phase Index Update

Update the completed phase index and architecture docs with the post-closure state.

Required docs:

- `plans/tui-runtime-daemon-completed-phase-index.md`
- `architecture/tui.md`
- `architecture/overview.md`
- `docs/CI_ARCHITECTURE_GUARDS.md`
- `.opencode/skills/eggsec-tui/SKILL.md` if repo convention expects it

Document:

- Embedded runtime is the only supported runtime mode at this phase.
- Daemon transport is intentionally not implemented yet.
- Result envelopes are the non-TUI result contract.
- Typed channels are TUI rendering compatibility, not daemon API.
- Runtime dependency boundary remains intentional: `eggsec -> eggsec-runtime`, never reverse.

## Non-Goals

Do not add socket, WebSocket, gRPC, REST, or SSE transport.

Do not add persistence.

Do not implement remote attach.

Do not add multi-client semantics.

Do not remove TUI typed rendering unless envelope coverage is rich enough to replace it without UI regression.

## Validation

Run:

```bash
cargo fmt --all --check
cargo check -p eggsec-runtime
cargo test -p eggsec-runtime
cargo check -p eggsec
cargo test --lib -p eggsec
cargo check -p eggsec-tui
cargo test -p eggsec-tui
cargo check -p eggsec-cli
./scripts/check-architecture-guards.sh
```

Feature checks where practical:

```bash
cargo check -p eggsec-tui --features stress-testing,packet-inspection
cargo check -p eggsec-tui --features nse
cargo check -p eggsec-tui --features db-pentest
cargo check -p eggsec-tui --features web-proxy
cargo check -p eggsec-tui --features wireless,wireless-advanced
cargo check -p eggsec-cli --features rest-api
```

## Final Acceptance Criteria

- A non-TUI in-process client path is tested.
- Runtime events and snapshots are sufficient for a minimal daemon client.
- Result envelopes are useful for common task completions.
- Runtime capabilities are truthful.
- TUI behavior remains stable.
- Architecture guards still pass.
- Documentation clearly marks the system as daemon-ready but not yet daemon-enabled.
