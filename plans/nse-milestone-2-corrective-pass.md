# NSE Milestone 2 Corrective Pass

## Purpose

This corrective pass closes the remaining gaps after the initial Milestone 2 implementation.

The implementation landed the right major artifacts: a library registry, structured report types, compatibility corpus tests, report tests, documentation updates, and architecture guard additions. However, the current runtime/report path still appears partly scaffolded. In particular, CLI JSON reports build the `NseRunReport` envelope but pass empty library and rule vectors, and rule evaluation still appears to rely on legacy boolean helpers that collapse errors/non-boolean results into `false`.

This pass should make Milestone 2 truthful end-to-end rather than only type-complete.

## Current State Summary

Confirmed progress:

- `crates/eggsec-nse/src/resolver/registry.rs` exists and defines a machine-readable registry for 43 standard Nmap Lua library entries.
- `crates/eggsec-nse/src/report.rs` exists and defines `NseRunReport` plus serializable summaries for profile, sandbox, limits, resolver diagnostics, libraries, rules, output, compatibility, warnings, and errors.
- `NseExecutor::build_report()` exists.
- `run_script_file()` and `run_script_file_with_output()` continue to route through `ScriptResolver`.
- `report_tests.rs` validates report builders and JSON serialization.
- `compatibility_corpus_tests.rs` validates representative resolver/report scenarios.
- `scripts/check-architecture-guards.sh` gained registry/report/no-full-parity checks.

Remaining corrective issues:

1. `plans/nse-milestone-2-phase-05-docs-release-gate.md` was deleted after execution. Restore or archive it for handoff/audit continuity.
2. `run_cli_with_profile()` currently constructs `library_reports: Vec<NseLibraryUseReport> = Vec::new()` and `rule_reports: Vec<NseRuleEvaluationReport> = Vec::new()`, then passes those empty vectors to the report builder. This satisfies a shallow guard but does not satisfy structured-report truthfulness.
3. There does not appear to be a runtime rule-semantics API that reports `NotPresent`, `PresentEvaluated`, `PresentErrored`, `PresentUnsupported`, or approximation/fidelity. Legacy `check_portrule()` / `check_hostrule()` still return booleans and collapse Lua errors/non-boolean values into `false`.
4. Library-use reporting is not yet connected to actual `require()` activity or script metadata. The registry exists, but actual run reports do not show which libraries were used.
5. The architecture guard for CLI report metadata checks only that `.with_libraries()` and `.with_rules()` are called. It does not catch empty placeholder vectors.
6. Corpus/report tests often manually construct reports rather than proving the real executor/CLI path populates non-empty rule/library metadata.

## Non-Goals

Do not reopen Milestone 1 loader/profile semantics.

Do not expand the registry from 43 standard Nmap Lua libraries to every protocol-specific Rust implementation unless doing so is required by a guard or report contract.

Do not implement Milestone 3 capability wrappers.

Do not pursue full Nmap parity.

Do not rewrite all existing library implementations. This pass is about truthful metadata plumbing and tests.

## Workstream 1: Restore Phase 05 Plan History

### Problem

The final docs/release-gate plan was removed. The repo has repeatedly benefited from retaining plan files for audit, handoff, and future agent context.

### Required Outcome

`plans/nse-milestone-2-phase-05-docs-release-gate.md` should be present again, or moved to an explicit completed-plan archive with a status header.

### Implementation Steps

1. Restore `plans/nse-milestone-2-phase-05-docs-release-gate.md` from commit history.
2. Add a short status header:

```markdown
> Status: Executed / partially executed. Retained for handoff and audit continuity. See `plans/nse-milestone-2-corrective-pass.md` for the remaining closure items.
```

3. Avoid deleting executed plans unless a documented archive convention exists.
4. Consider adding a small note to future agents in `AGENTS.md` or `plans/README.md` if such a file exists: executed plans should be retained or archived, not deleted.

### Acceptance Criteria

- The phase 05 plan is present again.
- It clearly indicates execution status.
- Future reviewers can trace overview → phase plans → corrective pass.

## Workstream 2: Add Real Library-Use Tracking

### Problem

Structured reports can include `NseLibraryUseReport`, but CLI JSON currently passes an empty vector. The registry is not useful to users or agents unless reports show which libraries were referenced/loaded and what their compatibility posture is.

### Required Outcome

When a script requires or otherwise uses known NSE libraries, the resulting `NseRunReport` must include non-empty `libraries` entries with registry-derived metadata.

### Implementation Options

Choose the smallest reliable implementation that works with current architecture:

1. **Preferred**: Track actual `require()` calls inside `ExecutorCore::setup_require()`.
   - Add a runtime collection such as `required_modules: Arc<Mutex<Vec<String>>>` or equivalent.
   - When `require(name)` succeeds or fails after validation, record name, success/failure, and whether it resolved as built-in/global/filesystem/missing.
   - Expose a method such as `ExecutorCore::required_modules()` or `NseExecutor::required_modules()`.
   - Convert names to `NseLibraryUseReport` using `find_library(name)`.
2. **Fallback**: Static extraction from script content for `require "name"` / `require('name')` patterns.
   - This is less exact but better than empty vectors.
   - Mark extracted-only entries with a warning such as `detected from static require scan, not runtime require hook`.
3. **Hybrid**: Runtime tracking first, static scan fallback for scripts that fail before execution.

### Required Report Fields

For each library use report:

- `name`
- `registered`
- `category`
- `side_effects`
- `fallback_behavior`
- `notes`
- `loaded`
- `warnings`

If `find_library(name)` returns `None`, set `registered = false`, category to `unknown`, and include a warning.

### Implementation Steps

1. Add a `NseLibraryUse` internal type if needed.
2. Track runtime `require()` names and load outcomes.
3. Add conversion helpers:

```rust
pub fn library_use_report_from_name(name: &str, loaded: bool) -> NseLibraryUseReport;
pub fn library_use_reports_from_names(names: &[NseRequiredModule]) -> Vec<NseLibraryUseReport>;
```

4. Ensure duplicate requires are de-duplicated or reported with stable ordering.
5. Include library reports in `NseExecutor::build_report()` and `run_cli_with_profile()`.
6. Add tests where a script requiring `stdnse` yields a report with a non-empty `libraries` array and a registered `stdnse` entry.

### Acceptance Criteria

- CLI JSON for a script that does `require "stdnse"` includes a `libraries` entry for `stdnse`.
- Unknown required modules appear as unregistered warnings.
- Empty library arrays are allowed only when the script contains and executes no `require()` calls or no library uses are detectable due to pre-execution failure.

## Workstream 3: Add Real Rule-Semantics APIs

### Problem

`NseRuleEvaluationReport` exists, but rule evaluation still appears to be legacy boolean-only. Errors and non-boolean returns are collapsed into `false`, which hides compatibility truth.

### Required Outcome

The executor must provide structured rule evaluation reports for `portrule`, `hostrule`, `prerule`, and `postrule` when those functions are present or when evaluation is requested.

### Proposed Types

The current `NseRuleEvaluationReport` has string fields:

```rust
pub struct NseRuleEvaluationReport {
    pub kind: String,
    pub evaluated: bool,
    pub matched: bool,
    pub exactness: String,
    pub error: Option<String>,
    pub summary: String,
}
```

This can be used for the corrective pass. If time allows, add enums later, but avoid broad churn.

### Implementation Steps

1. Add methods to `NseExecutor` such as:

```rust
pub fn evaluate_portrule_report(&self) -> NseRuleEvaluationReport;
pub fn evaluate_hostrule_report(&self) -> NseRuleEvaluationReport;
pub fn evaluate_prerule_report(&self) -> NseRuleEvaluationReport;
pub fn evaluate_postrule_report(&self) -> NseRuleEvaluationReport;
pub fn evaluate_rule_reports(&self) -> Vec<NseRuleEvaluationReport>;
```

2. Each method should distinguish:
   - function not present;
   - evaluated and matched true;
   - evaluated and matched false;
   - evaluated but returned non-boolean when boolean expected;
   - Lua error;
   - unsupported rule context.
3. For `portrule`/`hostrule`, mark exactness conservatively:
   - `exact` only if the current context is enough to satisfy the implemented contract;
   - `approximate` or `synthetic-input` if host/port tables are synthetic or incomplete;
   - `unsupported` if the rule cannot be meaningfully invoked.
4. Preserve existing `check_portrule()` / `check_hostrule()` for compatibility, but route them through report APIs where practical:

```rust
Ok(report.evaluated && report.matched && report.error.is_none())
```

5. Ensure non-boolean returns are not silently indistinguishable from `false` in report APIs.

### Tests

Add tests for:

- no rule functions → reports are `not-present` or equivalent;
- `portrule` true → evaluated/matched;
- `portrule` false → evaluated/not matched;
- `portrule` Lua error → error populated;
- `portrule` non-boolean → error or unsupported populated;
- `hostrule` true/false;
- `prerule` present and returns value;
- `postrule` present and returns value.

### Acceptance Criteria

- Report APIs do not collapse error/non-boolean into `false`.
- Legacy boolean APIs remain available.
- `NseRunReport.rules` can be populated from actual executor state.

## Workstream 4: Wire Reports End-to-End

### Problem

The report builder exists, but actual CLI JSON does not yet carry real library/rule metadata.

### Required Outcome

`run_cli_with_profile()` and `NseExecutor::build_report()` must produce a report with real library and rule metadata whenever available.

### Implementation Steps

1. Update `NseExecutor::build_report()` to include:
   - `self.required_modules()` converted into `NseLibraryUseReport`; or static detection fallback;
   - `self.evaluate_rule_reports()`.
2. Update `run_cli_with_profile()` so it does not construct empty placeholder vectors.
3. Avoid manually creating `library_reports: Vec::new()` and `rule_reports: Vec::new()` in production report paths.
4. If a script fails before execution, still emit a report with resolver diagnostics and an error where possible.
5. In JSON mode, emit report even for resolver/execution failures if practical. If current error flow makes this too broad, add a follow-up note and test at least successful runs.

### Acceptance Criteria

- CLI JSON for built-in/default scripts includes library metadata for `stdnse` and any other required modules that execute.
- CLI JSON for scripts with rule functions includes rule metadata.
- Reports for resolver failures include resolver diagnostics and compatibility `Failed` or `Partial` as appropriate.

## Workstream 5: Strengthen Architecture Guards

### Problem

The current guard checks only that `.with_libraries()` and `.with_rules()` appear in `lib.rs`. This does not catch empty placeholder vectors.

### Required Outcome

Architecture guards must fail if production report paths use empty placeholder vectors for library/rule metadata.

### Implementation Steps

1. Update Check 29 in `scripts/check-architecture-guards.sh` to reject patterns like:

```rust
let library_reports: Vec<...> = Vec::new();
let rule_reports: Vec<...> = Vec::new();
.with_libraries(Vec::new())
.with_rules(Vec::new())
```

2. Require evidence of real report population, such as calls to:
   - `build_report()`;
   - `evaluate_rule_reports()`;
   - `library_use_reports_from_*()`;
   - `required_modules()`.
3. Keep a narrow allowlist for tests that intentionally construct empty reports.
4. Add guard output telling implementers how to populate metadata.

### Acceptance Criteria

- Current placeholder pattern would fail the guard.
- Tests and report builders can still construct empty reports for scripts with no metadata.
- Guard remains maintainable and documented inline.

## Workstream 6: Upgrade Integration Tests

### Problem

Current tests validate report builders and manually composed reports. They do not sufficiently prove that the real executor/CLI path populates library/rule arrays.

### Required Outcome

Add integration tests that fail if actual run reports contain empty `libraries`/`rules` for scripts that require libraries or define rules.

### Required Tests

1. **Runtime library use**:
   - script: `local stdnse = require "stdnse"; return "ok"`
   - expected report: `libraries` contains registered `stdnse`, loaded true.
2. **Unknown library**:
   - script tries `pcall(require, "not_real_lib")`
   - expected report: unregistered or failed/missing library metadata visible.
3. **Portrule report**:
   - script defines `portrule = function(host, port) return true end`
   - expected report: `rules` contains `portrule`, evaluated true, matched true, exactness not empty.
4. **Rule error**:
   - script defines `portrule = function() error("boom") end`
   - expected report: `rules` contains error.
5. **CLI JSON smoke** where feasible:
   - run `run_cli_with_profile()` or a test helper that exercises the same code path;
   - assert JSON report includes non-empty `libraries`/`rules` for a fixture that requires both.

### Acceptance Criteria

- Builder-only tests remain, but at least one real execution/report test validates non-empty metadata.
- Tests distinguish `false rule result` from `rule errored`.
- Tests fail on placeholder `Vec::new()` metadata in production paths.

## Workstream 7: Documentation Corrections

### Problem

Docs may now imply Milestone 2 is complete even though report metadata is scaffolded. Correct the docs as part of implementation.

### Steps

1. Update `architecture/nse_integration.md` to describe the actual final state after this corrective pass.
2. If library/rule metadata is still partial, state the limits precisely.
3. Add a `Milestone 2 Corrective Closure` note once the pass is complete.
4. Update `.opencode/skills/eggsec-nse/SKILL.md` with report API usage and the rule/library metadata requirement.
5. Preserve the statement that full Nmap parity is not claimed.

### Acceptance Criteria

- Docs no longer overclaim report completeness.
- JSON/report examples show non-empty library/rule metadata for scripts that use them.
- Deferred work remains explicit.

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

If `make test-nse` is available, run it too. If not, document the closest equivalent.

## Final Acceptance Criteria

This corrective pass is complete when:

- The deleted phase 05 plan is restored or archived.
- Production report paths no longer pass empty placeholder library/rule vectors.
- Actual run reports include library metadata for executed/observed `require()` calls.
- Actual run reports include rule metadata for present/evaluated rule functions.
- Errors and non-boolean rule returns are visible in rule reports.
- Architecture guards reject placeholder metadata in production report paths.
- Integration tests prove non-empty library/rule metadata in real execution/report paths.
- Docs accurately state what Milestone 2 does and does not guarantee.

## Handoff Notes

Keep this pass narrow. Do not redesign the entire report schema unless required. The core issue is end-to-end truthfulness: registry and report types must be populated by actual runtime paths, not only by tests that manually construct metadata.
