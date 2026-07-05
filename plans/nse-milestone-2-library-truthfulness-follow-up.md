# NSE Milestone 2 Library Truthfulness Follow-Up

## Purpose

This follow-up closes the remaining Milestone 2 truthfulness gap after the corrective pass.

The latest implementation improved rule reporting and removed the most obvious empty `Vec::new()` placeholders from CLI JSON reports. However, library reporting now appears to overcorrect: production report paths build library reports by iterating every registry entry and marking each as `loaded: true`. That is not per-run library-use metadata. It is a registry capability snapshot mislabeled as loaded runtime usage.

This pass should make library reporting truthful by separating:

1. **available/registered library capability metadata** — what Eggsec can provide;
2. **per-run loaded/required library usage** — what the script actually required or attempted to require.

It should also restore the deleted phase 05 plan for handoff continuity.

## Current State

Confirmed improvements from the previous corrective pass:

- `run_cli_with_profile()` now runs scripts through `run_script_with_rules()` and passes rule reports into JSON output.
- Rule evaluation now uses `evaluate_rule()` and distinguishes boolean true, boolean false, nil, non-boolean unsupported values, and Lua errors.
- Dedicated rule evaluation tests exist.
- `NseRunReport` carries an `unsupported` field for rule reports.

Remaining issues:

1. `plans/nse-milestone-2-phase-05-docs-release-gate.md` is still missing.
2. `run_cli_with_profile()` calls `build_library_reports()` before execution and that helper maps every registry descriptor to `loaded: true`.
3. `NseExecutor::build_report()` also maps every registry descriptor to `loaded: true`.
4. Tests assert that library reports are non-empty and include `stdnse`, but do not assert that unused libraries are absent or marked as not loaded.
5. There is not yet a clear distinction in reports between registry capability metadata and actual per-run library usage.

## Non-Goals

Do not reopen Milestone 1 loader/profile semantics.

Do not rewrite the whole registry model.

Do not implement Milestone 3 capability wrappers.

Do not require full Nmap parity.

Do not expand the standard registry to all protocol-specific Rust modules unless this is needed for report correctness.

Do not remove useful registry metadata; reclassify it correctly.

## Workstream 1: Restore Deleted Phase 05 Plan

### Problem

The Milestone 2 phase 05 plan was deleted and has not yet been restored.

### Required Outcome

`plans/nse-milestone-2-phase-05-docs-release-gate.md` is present again, preferably with a status header.

### Implementation Steps

1. Restore the deleted file from commit history.
2. Add a header like:

```markdown
> Status: Executed / superseded by follow-up. Retained for handoff and audit continuity. See `plans/nse-milestone-2-library-truthfulness-follow-up.md` for remaining library-report truthfulness work.
```

3. If the repo wants an archive convention, move it to a documented `plans/completed/` or `plans/archive/` path instead of deleting it.
4. Add a short note to agent-facing guidance if appropriate: plan files should be retained or archived after execution.

### Acceptance Criteria

- The phase 05 plan is present again or intentionally archived.
- Future reviewers can trace overview → phase plans → corrective pass → library truthfulness follow-up.

## Workstream 2: Define Report Semantics: Available vs Loaded

### Problem

Current `NseLibraryUseReport.loaded = true` for every registry entry conflates capability metadata with runtime usage. This creates false positives and weakens the compatibility report.

### Required Outcome

The report schema and documentation must distinguish available libraries from actually loaded/required libraries.

### Recommended Model

Choose the least disruptive model:

### Option A: Keep `libraries` as per-run usage only

- `NseRunReport.libraries` contains only libraries required or attempted during this script run.
- Registry capability metadata remains available through separate registry APIs and docs.
- A script with no `require()` calls can legitimately have `libraries: []`.

This is preferred for truthfulness.

### Option B: Add separate fields

```rust
pub struct NseRunReport {
    pub libraries: Vec<NseLibraryUseReport>,          // per-run usage
    pub available_libraries: Vec<NseLibraryCapabilityReport>, // optional capability snapshot
}
```

Use this only if CLI consumers need both in the same JSON payload.

### Required Semantics

- `loaded = true`: the script actually required/loaded the library successfully during this run.
- `loaded = false`: the script attempted to require the library, but loading failed or was denied.
- `registered = true`: the name exists in the registry.
- `registered = false`: the name was attempted but no registry descriptor exists.
- Registry entries not touched by this run must not appear in `libraries` as `loaded = true`.

### Acceptance Criteria

- Docs and tests use the same semantics.
- No production report path marks all registry entries as loaded.

## Workstream 3: Track Runtime `require()` Activity

### Problem

The runtime currently has no clear per-run record of which libraries were required.

### Required Outcome

`ExecutorCore` should record `require()` attempts and outcomes during execution.

### Proposed Internal Type

Add a small internal/public summary type:

```rust
#[derive(Debug, Clone)]
pub struct NseRequiredModuleReport {
    pub name: String,
    pub loaded: bool,
    pub source: NseRequiredModuleSource,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum NseRequiredModuleSource {
    BuiltinGlobal,
    Filesystem,
    Missing,
    BlockedByPolicy,
    InvalidName,
    Unknown,
}
```

Keep it serializable only if it becomes part of public report output. Otherwise convert it into `NseLibraryUseReport`.

### Implementation Steps

1. Add a field to `ExecutorCore`:

```rust
required_modules: Arc<Mutex<Vec<NseRequiredModuleReport>>>
```

or equivalent.

2. In `setup_require()` record each `require(name)` attempt after validation:
   - invalid module name → `loaded = false`, `source = InvalidName`, error populated;
   - built-in/global module found → `loaded = true`, `source = BuiltinGlobal`;
   - filesystem module resolved and evaluated → `loaded = true`, `source = Filesystem`;
   - filesystem module load failed → `loaded = false`, `source = Filesystem`, error populated;
   - not found → `loaded = false`, `source = Missing`.
3. De-duplicate stable by module name and outcome, or preserve first attempt plus final outcome. Choose one and document it.
4. Expose:

```rust
pub fn required_modules(&self) -> Vec<NseRequiredModuleReport>
```

through `ExecutorCore` and `NseExecutor`.
5. Clear or snapshot semantics must be explicit. If each executor instance is single-run for CLI, simple append is acceptable. If reused, add `clear_required_modules()` before each run.

### Acceptance Criteria

- A script requiring only `stdnse` yields exactly one loaded required module for `stdnse` unless the runtime internally requires additional modules.
- Failed `require()` attempts are visible.
- Repeated `require("stdnse")` does not produce unstable duplicate spam.

## Workstream 4: Convert Required Modules to `NseLibraryUseReport`

### Problem

The registry exists but is not tied to actual require activity.

### Required Outcome

Per-run required modules should be converted to library-use reports using registry metadata.

### Implementation Steps

1. Add helper in `report.rs` or registry module:

```rust
pub fn library_use_report_from_required_module(
    required: &NseRequiredModuleReport,
) -> NseLibraryUseReport
```

2. Lookup `find_library(&required.name)`.
3. If registered:
   - copy category, side effects, fallback behavior, notes;
   - `registered = true`;
   - `loaded = required.loaded`;
   - warnings include failure/error/source notes as needed.
4. If unregistered:
   - `registered = false`;
   - category = `unknown`;
   - side effects = empty or `unknown` if the type supports it;
   - fallback behavior = `unknown`;
   - notes = `not present in NSE library registry`;
   - warnings include error/source.
5. Update `NseExecutor::build_report()` to use actual required modules.
6. Update `run_cli_with_profile()` to build reports after execution using the executor’s required-module snapshot, not before execution.

### Acceptance Criteria

- Reports show only actual attempted libraries in `libraries`.
- Unknown libraries are represented truthfully.
- A registry capability list is not mislabeled as runtime-loaded usage.

## Workstream 5: Static Require Extraction Fallback

### Problem

A script can fail before runtime `require()` instrumentation captures all library intent. For pre-execution failures, it may still be useful to show statically detected `require()` names.

### Required Outcome

Add a small, conservative fallback only if needed. Do not replace runtime tracking with static extraction.

### Implementation Steps

1. Add a simple parser for common forms:
   - `require "name"`
   - `require 'name'`
   - `require("name")`
   - `require('name')`
2. Use it only when runtime required-module list is empty and script content is available.
3. Mark extracted entries with a warning:

```text
detected statically; runtime load did not complete
```

4. Do not attempt complex Lua parsing or dynamic require analysis.

### Acceptance Criteria

- Static fallback never marks entries as `loaded = true` unless runtime confirmed load.
- Fallback entries are clearly labeled.

## Workstream 6: Fix Tests to Detect False Positives

### Problem

Current tests allow every registry entry to be marked loaded.

### Required Outcome

Tests must fail if reports claim unused libraries were loaded.

### Required Tests

1. **Single required library**
   - Script: `local stdnse = require "stdnse"; return "ok"`
   - Assert `libraries` contains `stdnse`.
   - Assert obvious unrelated library such as `http`, `smb`, or `mysql` is absent or `loaded = false` if available capabilities are included separately.
2. **No require**
   - Script: `return "ok"`
   - Assert per-run `libraries` is empty.
3. **Repeated require**
   - Script requires `stdnse` twice.
   - Assert stable de-duplicated result or documented repeat behavior.
4. **Unknown require**
   - Script uses `pcall(require, "not_real_lib")`.
   - Assert `registered = false`, `loaded = false`, warning/error present.
5. **Filesystem module require** if configured roots allow it.
   - Assert `source = Filesystem` or equivalent warning/metadata.
6. **CLI JSON path**
   - Use a helper or direct `run_cli_with_profile()` test if feasible.
   - Assert JSON library reports are per-run, not all registry entries.

### Acceptance Criteria

- Tests fail under the current all-registry-loaded implementation.
- Tests pass only when report output reflects actual require activity.

## Workstream 7: Strengthen Architecture Guards

### Problem

The previous guard caught empty placeholders but not all-registry-loaded placeholders.

### Required Outcome

Architecture guards must reject production helpers that map `registry::all_libraries()` directly into `NseRunReport.libraries` with `loaded: true`.

### Implementation Steps

1. Update the report metadata guard to reject patterns like:

```rust
registry::all_libraries().iter().map(... loaded: true ...)
```

inside production report paths.
2. Allow this pattern only if the field is explicitly named `available_libraries` or similar.
3. Require production report paths to call something like:
   - `required_modules()`;
   - `library_use_reports_from_required_modules()`;
   - `extract_static_requires()` with warnings.
4. Add guard output explaining the difference between registry capability metadata and per-run usage.

### Acceptance Criteria

- The current all-registry-loaded implementation would fail the guard.
- Legitimate capability-matrix generation is still allowed under docs/tools paths.

## Workstream 8: Documentation Corrections

### Required Updates

Update:

- `architecture/nse_integration.md`
- `.opencode/skills/eggsec-nse/SKILL.md`
- `AGENTS.md` if it mentions NSE reports
- README wording if it references compatibility reports

### Required Wording

Use these concepts exactly:

- `libraries` in `NseRunReport` means per-run required/attempted libraries.
- Registry capability metadata is not the same as loaded runtime usage.
- `loaded = true` requires observed runtime load success.
- Unused registered libraries must not appear as loaded in per-run reports.
- Static require detection, if used, is approximate and must be labeled.

### Acceptance Criteria

- Docs do not imply the library list is a full registry dump.
- Users can interpret `loaded`, `registered`, and warnings correctly.

## Verification Gate

Run:

```bash
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse,sandbox
cargo check -p eggsec --features nse
cargo test -p eggsec --features nse --test nse_tests
bash scripts/check-architecture-guards.sh
cargo fmt --all --check
cargo clippy --lib -p eggsec-nse --features nse
cargo clippy --lib -p eggsec --features nse
```

If `make test-nse` is available, run it too. If not, document the equivalent commands.

## Final Acceptance Criteria

This follow-up is complete when:

- The deleted phase 05 plan is restored or archived.
- Per-run reports no longer mark every registry entry as loaded.
- Actual `require()` attempts are tracked with load outcome.
- `NseRunReport.libraries` represents per-run required/attempted library usage.
- Available registry/capability metadata is either separate or omitted from per-run reports.
- Tests prove no-require, single-require, repeated-require, unknown-require, and CLI/report behavior.
- Architecture guards reject both empty placeholders and all-registry-loaded placeholders.
- Docs explain the difference between registry capability and runtime usage.

## Handoff Notes

Keep this pass small and semantic. The runtime already has a registry and report model. The missing piece is truthfulness: do not represent availability as usage. If a perfect runtime require tracker is difficult, implement a conservative tracker plus static fallback, but never mark unobserved libraries as loaded.
