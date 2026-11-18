# Security Assurance Hardening Milestone 001 — Enforcement Gate Integrity

Status: closed

Repository baseline: `77188755` (`feat/auto-20261117-7451c367`, 2026-11-17)

Source roadmap:

- `plans/subsystems/security-assurance-hardening-roadmap.md` §7 Milestone 1
- Finding A1 in §4 of the same roadmap

Long-term requirements:

- `plans/000-long-term-specification.md` (enforcement boundary)
- `plans/001-terminology-and-domain-model.md` (`ApprovedOperation`, manual surface)
- `AGENTS.md` — "Enforcement (critical)" and "Dispatch / scope / runtime ownership"

Applicable ADRs: none. This milestone makes an existing documented invariant true;
it does not choose a new architecture.

Primary class: invariant

## 1. Objective

Make the manual-surface approval token load-bearing: a task cannot be submitted
to the TUI runtime without an `ApprovedOperation` that matches the request's
operation and target, and the unchecked engine dispatch entry points stop being
general-purpose public API.

## 2. Why this milestone is ready

- No hard dependencies. All relevant code is in-tree and exercised by existing
  tests.
- The engine already owns canonical identity for both sides of the comparison:
  `TaskKind::operation_id()` / `TaskKind::canonical_target()`
  (`crates/eggsec-runtime/src/request.rs:582,628`) and
  `ApprovedOperation::descriptor()` (`crates/eggsec-policy/src/approval.rs:65`).
- The `ToolRequest` equivalent already exists and is proven:
  `validate_request_binding` (`crates/eggsec/src/tool/dispatcher.rs:81`), with
  tests for alias mismatch, target mismatch, and quarantined operations
  (`crates/eggsec/src/dispatch/canonical_execution.rs:2197-2340`).

## 3. Current implementation evidence

The gate is nominal:

```rust
// crates/eggsec-tui/src/app/task_runtime.rs:229-231
pub(crate) fn spawn_task(
    &mut self,
    request: Option<RunRequest>,
    _approved: Option<ApprovedOperation>,
) {
```

`_approved` is never read. Both call sites pass a real approval
(`crates/eggsec-tui/src/app/mod.rs:654`, `:1002`), so **there is no live
bypass today**. The defect is assurance: two doc comments claim the token
structurally gates dispatch (`app/mod.rs:637`,
`app/enforcement_facade.rs:169`), and a future call site can pass `None`
without any compile-time or runtime objection.

Engine-side, two unchecked entry points remain reachable:

- `dispatch::execute_canonical` — `crates/eggsec/src/dispatch/canonical_execution.rs:1131`
- `dispatch::dispatch_inner` — `crates/eggsec/src/dispatch/mod.rs:132`

`dispatch_inner`'s only non-test in-tree caller is `dispatch_task`
(`dispatch/mod.rs:79`); the `eggsec-daemon` reference at
`crates/eggsec-daemon/src/host.rs:2570` is a test comment, not a call.
`execute_canonical` is additionally called from an integration test
(`crates/eggsec/tests/canonical_dispatch_ownership.rs:423`), so it cannot simply
become `pub(crate)` without breaking `make test-ci`.

The architecture guard at `scripts/check-architecture-guards.sh:1964` *requires*
`crates/eggsec-tui/src/app/task_dispatcher.rs` to reference `execute_canonical`.
That requirement is intentional (the TUI is a manual surface) and must be
preserved.

## 4. Invariants that must not regress

- `EnforcementContext::evaluate()` remains the mandatory pre-dispatch gate for
  every surface.
- Strict surfaces (REST/MCP/agent/gRPC) keep dispatching only via
  `EnforcedDispatcher::dispatch_execution()` with an `ApprovedExecution` bundle.
- `OperationMetadata` remains the single source of truth for operation policy;
  no inline policy check is added.
- No adapter calls `Scope::is_target_allowed` or `tool.execute()` directly.
- The TUI continues to route through `execute_canonical` (guard 1964).
- No new `unwrap()`/`expect()` on user-controlled data; binding failures are
  typed errors surfaced as a tab error.
- `let _ =` and silent `filter_map(|e| e.ok())` remain forbidden.

## 5. Scope

### In scope

- `spawn_task` signature and behavior in `eggsec-tui`.
- A `RunRequest` ↔ `ApprovedOperation` binding validator in the engine.
- Visibility/annotations for `dispatch_inner` and `execute_canonical`.
- Regression tests and one static guard.
- Doc-comment corrections in `eggsec-tui` and the relevant `AGENTS.md` line if
  it states the gate is structural.

### Explicitly out of scope

- Any change to `ExecutionProfile`, `DenialClass`, confirmation UX, or override
  semantics.
- New manual override paths.
- Reworking `TuiDispatcherContext` beyond what the binding check needs.
- The daemon/agent/MCP dispatch paths (already strict).
- Milestone 004's secret-handling changes, even though both touch
  `eggsec-tui` task-management files (serialize the two milestones).

## 6. Required production changes

### Core/domain

Add `crates/eggsec/src/dispatch/binding.rs` (or an equivalent module inside
`dispatch/`) exposing:

```rust
pub fn validate_run_request_binding(
    approval: &ApprovedOperation,
    request: &RunRequest,
    effective_target: Option<&str>,
) -> Result<(), RunRequestBindingError>;
```

`effective_target` is the same target string the caller used to build the
descriptor. For most task kinds it is `request.task_kind.canonical_target()`.
`TaskKind::Resume` is the documented exception: `canonical_target()` returns
`None` because the target lives in the checkpoint
(`crates/eggsec-runtime/src/request.rs:639-644`), while the manual surface
builds a target-bearing descriptor from the selected session. The caller passes
that session target explicitly; the validator must not special-case `Resume` by
name.

Rules, mirroring `validate_request_binding` semantics:

1. `approval.descriptor().operation` must equal
   `request.task_kind.operation_id()`.
2. If `descriptor.normalized_target` is `OperationTarget::None`, then
   `effective_target` must be `None`.
3. Otherwise `effective_target` must be `Some`, and
   `normalize_target(target, None)` must equal `descriptor.normalized_target`.
4. Mismatch is a typed error carrying both sides; no silent coercion, no
   case-folding beyond what `normalize_target` already does.

The module must not import any frontend type (Clap/Ratatui/daemon protocol).

### Storage and migrations

None.

### Protocol and DTOs

None. `RunRequest`/`TaskKind` are unchanged. Do not add a serialized approval to
the wire.

### Runtime and concurrency

`spawn_task` keeps its existing submission flow and timeouts. The binding check
runs synchronously before `runtime.submit`, on the TUI thread, and returns
before any task is created.

### Frontend or operator surface

`spawn_task` becomes:

```rust
pub(crate) fn spawn_task(
    &mut self,
    request: Option<RunRequest>,
    approved: ApprovedOperation,
) -> Result<(), TaskSubmitError>;
```

or an equivalent shape where the token cannot be omitted. Preferred: take
`RunRequest` and `ApprovedOperation` directly (both non-optional) and keep the
`Option` only at the existing `if let Some(request)` call sites if that avoids a
larger refactor. A binding failure sets the same tab error used today and
returns without submitting.

Correct the two doc comments to describe what the code actually enforces.

### Security and authorization

This is the security change: fail closed on binding mismatch, and surface the
reason with `tracing::warn!` (operation id + target label only; never params).

### Documentation and static guards

- `crates/eggsec-tui/src/app/task_runtime.rs` doc comment.
- `crates/eggsec-tui/src/app/mod.rs:637` and
  `crates/eggsec-tui/src/app/enforcement_facade.rs:169` doc comments.
- `architecture/tui.md` if it repeats the structural-gate claim.
- New guard: fail if `_approved` reappears in `eggsec-tui`, and fail if
  `spawn_task` gains an `Option<ApprovedOperation>` parameter.

## 7. Ordered work packages

### Work package A — Engine binding validator

Intent: give both sides a single canonical comparison.

Required changes:

- New `dispatch::binding` module with the validator and a typed error.
- Reuse `normalize_target` and `operation_id`/`canonical_target`; do not
  duplicate normalization.
- Unit tests covering: match, operation mismatch, missing target, unexpected
  target, URL normalization equivalence, host case equivalence, and the `Resume`
  path (session target matches / mismatched session target is rejected).

Acceptance evidence: tests in the new module; `cargo test -p eggsec --lib`.

### Work package B — TUI gate

Intent: make the token unmissable and enforce it.

Required changes:

- Change `spawn_task` signature; update both call sites.
- Call the validator before submission, passing the same target string used to
  build the descriptor (`canonical_target()` normally; the selected session's
  target on the `Resume` path); on error, set the tab error and return.
- Remove the dead-parameter comment and the `_approved` naming.

Acceptance evidence: a TUI unit test proving a mismatched approval is rejected
and no runtime submit occurs (use the existing test harness in
`crates/eggsec-tui/src/app/` — inspect how other dispatch tests fake the
runtime before writing one).

### Work package C — Entry-point tightening

Intent: reduce the unchecked surface without breaking the test contract.

Required changes:

- `dispatch_inner` → `pub(crate)`. Verify no external caller first.
- `execute_canonical`: choice (a) is implemented, keeping it `pub` but adding `#[doc(hidden)]` and a caller-scope guard.
  The guard restricts references to `dispatch/mod.rs`, `eggsec-tui`, and tests,
  preserving the integration-test contract without changing `make test-ci`.
- Update the module doc comment to state that these are manual-surface
  primitives, not general API.

Acceptance evidence: `cargo check --workspace --no-default-features`;
`make test-ci` unchanged.

### Work package D — Guard and docs

Intent: prevent recurrence.

Required changes:

- Add the guard checks described in §6.
- Update the doc comments and `architecture/tui.md` if applicable.

Acceptance evidence: guard passes on the new tree and fails when the parameter
is locally reintroduced (verify by temporary local edit, then revert).

## 8. Failure, cancellation, restart, and contention semantics

- Binding failure: no task is created, no channels are opened, no audit event
  claims execution. The operator sees a tab error; the previous task state is
  untouched.
- Cancellation: unchanged; this milestone adds no spawn and no timeout.
- Restart: no persisted state is involved.
- Contention: the check runs before any lock is taken on runtime state; it must
  not hold a lock across `await`.

## 9. Compatibility and migration

- Internal API only. `eggsec-tui` is the only `spawn_task` caller in-tree.
- The Python/daemon/protocol surfaces are unaffected.
- No configuration or on-disk format change.

## 10. Required tests

### Focused unit tests

- Binding validator matrix (§7 WP A).
- TUI rejection path and acceptance path.

### Integration tests

- `crates/eggsec/tests/canonical_dispatch_ownership.rs` still passes after the
  visibility decision.

### Restart and recovery tests

Not applicable.

### Contention and cancellation tests

Not applicable; assert the validator is synchronous and side-effect free.

### Security and negative tests

- Operation mismatch rejected.
- Target mismatch rejected (including `example.com` vs `example.com:443`).
- Targetless approval + target-bearing request rejected.
- Target-bearing approval + targetless request rejected.

### Migration and compatibility tests

Not applicable.

## 11. Required verification commands

```bash
make check
cargo test -p eggsec-tui
make test-ci
```

`make test-tui-pty` is expected for this milestone because the TUI dispatch path
changes; run it if the environment supports PTY tests and record the outcome.

Do not claim commands that were not actually run in the closure record.

## 12. Documentation updates

- `crates/eggsec-tui/src/app/task_runtime.rs` (signature + doc).
- `crates/eggsec-tui/src/app/mod.rs:637`,
  `crates/eggsec-tui/src/app/enforcement_facade.rs:169`.
- `architecture/tui.md` if it restates the claim.
- `AGENTS.md` only if it asserts something now false (check before editing).

## 13. Acceptance criteria

1. `spawn_task` cannot be called without an `ApprovedOperation`.
2. A request whose operation id or normalized target differs from the approval
   is rejected before submission, with a typed error and a `tracing` warning.
3. `dispatch_inner` is not reachable from outside the engine crate.
4. `execute_canonical` is either non-public or guard-restricted, with the choice
   recorded in the plan deviation log.
5. Existing TUI dispatch tests and `make test-ci` pass unchanged.
6. The new guard fails when the dead parameter is reintroduced.

## 14. Stop conditions

Stop and report rather than improvise when:

- the binding check cannot be expressed without changing `eggsec-policy`;
- tightening visibility requires editing more than one integration test or
  changing CI feature flags;
- the TUI test harness has no way to fake the runtime without new production
  seams (report the seam instead of inventing one);
- repository evidence shows `spawn_task` already consumed by a surface this plan
  did not account for.

## 15. Closure evidence required

- The exact commit(s) and PR.
- The two-sided binding test matrix with pass output.
- `make check` and `make test-ci` output.
- Proof that the guard fails on a deliberate local regression.
- The visibility decision and its rationale.
- Any deviation from this plan, recorded rather than hidden.

## 16. Handoff notes

- Read `crates/eggsec-tui/src/app/enforcement_facade.rs` and
  `evaluate_policy_and_dispatch` before editing; the approval already flows
  through that path.
- Do not add a serialized approval field to `RunRequest`; the daemon protocol
  must stay unchanged.
- If `normalize_target` produces a different variant for the same logical target
  than the descriptor builder did (e.g. host vs URL), report it as a
  normalization inconsistency instead of loosening the comparison.
- Preserve unrelated user changes in the worktree.
