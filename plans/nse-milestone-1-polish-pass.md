# NSE Milestone 1 Polish Pass

## Purpose

This plan is a final polish and handoff pass for the NSE Milestone 1 loader/profile work.

Milestone 1 is functionally closed: script and module loading is resolver-owned, manual versus automated profile semantics are explicit, manual script-file behavior works under `ManualPermissive`, restricted profiles enforce canonical containment, and regression tests exist for the key policy cases.

This polish pass should not reopen the loader architecture. It should improve handoff durability, documentation consistency, verification visibility, and readiness for Milestone 2.

## Current State

The current implementation has the intended technical shape:

- `ScriptResolver` owns script/module file policy enforcement.
- `resolve_script_file()` handles manual empty-root behavior correctly.
- `validate_existing_path_under_roots()` is read-only and cannot authorize non-existent files through parent fallback.
- `validate_parent_under_roots()` is separated and documented as write/create-only.
- `NseScriptPolicy` and `NseModulePolicy` document empty-root semantics.
- `script_file_policy_tests.rs` covers manual permissive, strict, agent/CI, module-root, and CLI resolver-path cases.
- `architecture/nse_integration.md` has a Milestone 1 closure note.

Remaining polish issues:

1. The executed final corrective plan file was deleted from `plans/`, which weakens handoff/audit history.
2. Verification is documented in commit messages/docs, but no GitHub status checks were visible through the connector.
3. Loader/profile semantics are now repeated in several places; wording should be checked for drift and contradictions.
4. The next line of work, Milestone 2, needs a clean boundary so future work does not reopen Milestone 1 accidentally.
5. Architecture guard expectations should be easy for future agents to follow.

## Non-Goals

Do not redesign `NseScriptPolicy` or `NseModulePolicy`.

Do not add new NSE libraries.

Do not start Milestone 2 implementation.

Do not change the manual/automated policy model.

Do not remove manual-only constructors.

Do not add broad new enforcement beyond the current Milestone 1 contract.

## Workstream 1: Restore Handoff/Audit Plan History

### Problem

The executed plan `plans/nse-milestone-1-final-corrective-pass.md` was removed after implementation. This creates a gap in the planning trail. The repo has generally used `plans/` as a handoff and audit directory, not only an active queue.

### Required Outcome

The repo should retain enough plan history for future maintainers and agents to understand what was requested, what was implemented, and why.

### Implementation Options

Choose one repo convention and apply it consistently:

1. **Preferred**: Restore the deleted plan file under `plans/` unchanged, and optionally add an `Executed` / `Status` section at the top.
2. **Alternative**: Restore it under `plans/archive/` or `plans/completed/` if the project wants active plans separated from completed plans.
3. **Alternative**: Keep it deleted only if the repo introduces a documented convention that executed plans are removed and summarized elsewhere.

The preferred option is simplest and preserves existing handoff style.

### Implementation Steps

1. Restore `plans/nse-milestone-1-final-corrective-pass.md` from the commit history.
2. Add a small status header:

```markdown
> Status: Executed. See the Milestone 1 closure note in `architecture/nse_integration.md` and regression tests in `crates/eggsec-nse/tests/script_file_policy_tests.rs`.
```

3. Add a short note that the plan is retained for audit/handoff purposes.
4. Avoid rewriting the plan heavily; it should remain an accurate historical record.

### Acceptance Criteria

- The final corrective plan is present in the repo again.
- The file clearly states it has been executed.
- Future agents can trace from roadmap → corrective plan → final corrective plan → closure note.

## Workstream 2: Consolidate Milestone 1 Closure Index

### Problem

Milestone 1 closure information is spread across:

- `architecture/nse_integration.md`
- `.opencode/skills/eggsec-nse/SKILL.md`
- `crates/eggsec-nse/AGENTS.override.md`
- test names and comments
- commit messages

This is acceptable, but there should be a single short index pointing future maintainers to the canonical places.

### Required Outcome

Add a compact closure index that identifies:

- the canonical implementation files,
- the canonical tests,
- the docs that define the policy contract,
- the deferred Milestone 3 cancellation caveat,
- the next Milestone 2 boundary.

### Implementation Steps

1. In `architecture/nse_integration.md`, add a short `Milestone 1 Closure Index` section after the existing closure note.
2. Include links or paths to:
   - `crates/eggsec-nse/src/resolver.rs`
   - `crates/eggsec-nse/src/profile.rs`
   - `crates/eggsec-nse/src/executor_core.rs`
   - `crates/eggsec-nse/tests/script_file_policy_tests.rs`
   - `crates/eggsec-nse/tests/profile_guard_tests.rs`
   - `crates/eggsec-nse/tests/execution_limits_tests.rs`
3. State the canonical policy assertions:
   - `require()` filesystem module loading is resolver-owned.
   - Manual script files with empty roots are allowed only under `ManualPermissive` semantics.
   - Filesystem modules require explicit roots.
   - Agent/CI deny arbitrary script files and filesystem modules before path checks.
   - Blocking Rust-side helper cancellation is deferred.
4. Add one sentence that Milestone 2 should begin at library registry/rule/report truthfulness, not loader-policy redesign.

### Acceptance Criteria

- A maintainer can find the relevant code/tests/docs from one section.
- The closure index does not introduce new policy semantics.

## Workstream 3: Documentation Drift Pass

### Problem

The same policy semantics are now described in several files. This can drift quickly, especially around empty roots and manual/automated boundaries.

### Required Outcome

All user-facing and agent-facing docs should use identical semantics and avoid ambiguous phrases.

### Files to Inspect

- `architecture/nse_integration.md`
- `.opencode/skills/eggsec-nse/SKILL.md`
- `crates/eggsec-nse/AGENTS.override.md`
- root `AGENTS.md`
- `README.md`
- any NSE-specific README or module docs

### Required Wording Rules

Use these exact concepts consistently:

- `ManualPermissive` is manual-only and not agent-safe.
- Empty `allowed_script_roots` under `ManualPermissive` means unrestricted manual script-file selection; extension and size checks still apply.
- Empty `allowed_module_roots` means no filesystem module loading.
- `AgentSafe` and `CiSafe` reject arbitrary script files and filesystem modules before path authorization.
- Restricted profiles require configured roots for filesystem script/module loading.
- Read-path authorization requires the file itself to exist and canonicalize.
- Rust-side blocking helper cancellation is deferred to Milestone 3 capability wrappers.

### Implementation Steps

1. Search docs for `ManualPermissive`, `empty roots`, `script file`, `filesystem modules`, `require`, `cancellation`, and `Milestone 1`.
2. Correct wording that implies all script/module loads always require roots; manual script files are the deliberate exception.
3. Correct wording that implies manual-permissive can load filesystem modules without roots; it cannot.
4. Correct wording that implies blocking Rust-side helper calls are fully cancellable; they are not.
5. Keep docs concise; remove duplicated deep explanations where a link/path to the closure index is enough.

### Acceptance Criteria

- No doc contradicts the policy table in `NseScriptPolicy` / `NseModulePolicy`.
- Agent-facing docs clearly direct automated surfaces away from manual constructors.

## Workstream 4: Verification Visibility

### Problem

The closure commit and docs report tests, but GitHub status checks were not visible through the connector. For future release-readiness work, there should be an obvious verification record.

### Required Outcome

Add a lightweight, durable verification record for NSE Milestone 1.

### Implementation Options

Choose one:

1. Add a small `Verification` section in `architecture/nse_integration.md` listing commands and latest observed pass status.
2. Add `plans/nse-milestone-1-verification.md` as a dedicated verification note.
3. Add a `make test-nse-milestone-1` target if the repo convention supports make targets.

The lowest-friction option is a documentation note plus ensuring existing make targets are referenced accurately.

### Required Commands

Record the intended gate:

```bash
cargo check -p eggsec-nse
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
cargo check -p eggsec --features nse
make test-nse
```

If the implementer reruns commands, record exact command output summaries. If some commands cannot run in the local environment, record the reason and closest equivalent.

### Acceptance Criteria

- Future agents know the exact gate for Milestone 1.
- Verification is not only buried in commit messages.
- Any unsupported command is documented explicitly.

## Workstream 5: Architecture Guard Polish

### Problem

The code now has stronger boundaries, but future agents may accidentally add direct file reads or manual profile defaults in the wrong place.

### Required Outcome

Architecture guard expectations should be clear and cheap to enforce.

### Implementation Steps

1. Review `profile_guard_tests.rs` and `script_file_policy_tests.rs` for readability and failure messages.
2. Add comments in tests where they encode architectural policy rather than ordinary behavior.
3. If not already present, add or update source-scanning guard tests for:
   - direct `std::fs::read_to_string` in execution paths outside `resolver.rs` / approved test files;
   - `ManualPermissive` use outside manual CLI/TUI and tests;
   - automated surfaces using `NseExecutor::new()`, `with_sandbox()`, or `with_target()`.
4. Keep guard tests tolerant enough to avoid brittle false positives, but strict enough to prevent obvious regressions.
5. Ensure any allowlist is documented inline.

### Acceptance Criteria

- The guard tests explain what boundary they protect.
- Future regressions fail with actionable messages.

## Workstream 6: Milestone 2 Boundary Prep

### Problem

With Milestone 1 functionally closed, the next work should not restart loader-policy debate. Milestone 2 should focus on library registry, rule semantics, compatibility truthfulness, and report structure.

### Required Outcome

Create a clean handoff paragraph for Milestone 2.

### Implementation Steps

1. Add a short `Next Work: Milestone 2` section in the closure index or a new plan file.
2. State explicitly:
   - loader and profile enforcement are closed unless tests reveal a regression;
   - Milestone 2 should build on `ScriptResolver`, not bypass it;
   - library registration should move toward a declarative registry/truthfulness matrix;
   - NSE rule matching should document approximate semantics and gaps;
   - structured run reports should expose profile, resolver diagnostics, limits, and compatibility status.
3. Avoid writing the full Milestone 2 plan in this pass unless specifically requested.

### Acceptance Criteria

- The repo communicates the next boundary clearly.
- Future work starts from the closed Milestone 1 contract.

## Recommended Commit Structure

1. Restore executed final corrective plan or add completed-plan archive convention.
2. Add Milestone 1 closure index and verification record.
3. Normalize docs and agent guidance wording.
4. Polish guard tests and comments.
5. Add Milestone 2 boundary note.

## Final Acceptance Criteria

This polish pass is complete when:

- The deleted final corrective plan is restored or an explicit completed-plan convention exists.
- Milestone 1 closure has a single index pointing to implementation, tests, and docs.
- Policy wording is consistent across architecture, skill, AGENTS, and README docs.
- Verification commands are recorded outside commit messages.
- Guard tests are understandable and protect the intended boundaries.
- Milestone 2 has a clear boundary without reopening Milestone 1 loader/profile semantics.

## Handoff Notes

Keep this pass documentation- and guard-focused. Do not make broad code changes unless a drift check finds an actual contradiction or regression. The main value is preserving the hard-won Milestone 1 boundary so future work can proceed without destabilizing manual/automated policy semantics.
