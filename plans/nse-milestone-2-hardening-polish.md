# NSE Milestone 2 Hardening and Polish

## Purpose

This plan closes the remaining Milestone 2 polish items after the library truthfulness follow-up.

The current implementation now has the correct semantic direction: `NseRunReport.libraries` represents observed per-run `require()` activity rather than a fabricated list of all registered capabilities, rule reports are produced from real evaluation paths, and the deleted phase 05 plan was restored. The remaining work is hardening: prevent regressions, record final verification, tighten documentation, and make report semantics durable for Milestone 3.

## Current State

Milestone 2 is close to closure:

- `ExecutorCore` tracks observed `require()` attempts in `required_modules`.
- `setup_require()` records built-in/global, filesystem, missing, blocked, invalid, and unknown require outcomes.
- `NseLibraryUseReport::from_required_module()` converts observed require activity into truthful report metadata.
- `run_cli_with_profile()` uses runtime library reports and falls back to static require extraction only when runtime tracking is empty.
- Static require fallback marks entries as `loaded = false` with a warning.
- Tests now assert that no-require scripts do not fabricate library usage and that a script requiring `stdnse` plus a missing module reports exactly those modules.
- `plans/nse-milestone-2-phase-05-docs-release-gate.md` has been restored.

Remaining polish items:

1. The architecture guard still appears to check only that `.with_libraries()` and `.with_rules()` are called, not that production code avoids all-registry-loaded fabrication.
2. Verification status is not visible through connector status checks, so a durable verification note should be recorded.
3. Documentation should explicitly state that `libraries` is per-run observed/attempted `require()` usage, while registry APIs represent capability metadata.
4. Static require fallback should be documented as approximate and non-loaded.
5. Milestone 3 should start from a clean boundary: capability wrappers and helper enforcement, not report-truthfulness cleanup.

## Non-Goals

Do not reopen Milestone 1 loader/profile semantics.

Do not redesign the Milestone 2 report schema unless a hard bug is found.

Do not expand the library registry beyond its current scope.

Do not implement Milestone 3 capability wrappers in this polish pass.

Do not claim full Nmap parity.

## Workstream 1: Strengthen Architecture Guards Against Library Report Fabrication

### Problem

The current guard verifies that JSON report paths call `.with_libraries()` and `.with_rules()`, but a previous implementation satisfied that check while marking every registry entry as `loaded: true`. The guard should reject both empty placeholders and all-registry-loaded placeholders.

### Required Outcome

`bash scripts/check-architecture-guards.sh` must fail if production report paths directly turn `registry::all_libraries()` into `NseRunReport.libraries` with `loaded: true`.

### Implementation Steps

1. Update the report metadata guard to detect patterns in production files such as:

```rust
registry::all_libraries().iter().map(... loaded: true ...)
```

2. Reject `.with_libraries(Vec::new())` or local `let library_reports: Vec<...> = Vec::new()` in production report paths, unless explicitly inside test-only code.
3. Require evidence that production reports use runtime observations, for example:
   - `executor.library_reports()`;
   - `required_modules()`;
   - `library_use_reports_from_required_modules()`;
   - labeled static fallback through `library_use_reports_from_static_requires()`.
4. Permit full registry iteration only in registry docs/matrix generation or explicitly named capability snapshots, not per-run `libraries`.
5. Add guard messages explaining the distinction:

```text
NseRunReport.libraries is per-run require activity. Do not populate it from the full registry capability set.
```

### Acceptance Criteria

- The old all-registry-loaded implementation would fail the guard.
- Current runtime-tracked implementation passes.
- The guard remains readable and has a small allowlist.

## Workstream 2: Finalize Report Semantics Documentation

### Problem

Users and future agents need to understand the difference between per-run library usage, registered capabilities, and static fallback.

### Required Outcome

Docs should make report semantics unambiguous.

### Files to Update

- `architecture/nse_integration.md`
- `.opencode/skills/eggsec-nse/SKILL.md`
- `crates/eggsec-nse/AGENTS.override.md`
- root `AGENTS.md` if needed
- README wording if it mentions NSE compatibility reports

### Required Wording

Use these concepts consistently:

- `NseRunReport.libraries` is per-run observed or attempted `require()` activity.
- `loaded = true` means runtime observed a successful module load.
- `loaded = false` means a require was attempted but failed, was blocked, was missing, invalid, or statically detected without runtime confirmation.
- Registry APIs describe available capability metadata, not per-run usage.
- Static require fallback is approximate and is labeled with a warning.
- `rules` comes from real rule evaluation where available and may report unsupported/non-boolean/error outcomes.

### Acceptance Criteria

- No docs describe report `libraries` as a full registry dump.
- JSON examples, if present, show only libraries actually used or attempted by the script.

## Workstream 3: Verification Record

### Problem

Connector status checks are empty, so verification needs to be durable in repo docs.

### Required Outcome

A `Milestone 2 Final Verification` section should be recorded outside commit messages.

### Implementation Steps

1. Add a final verification subsection to `architecture/nse_integration.md` or a dedicated plan note.
2. Record commands run and results. At minimum:

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

3. If a command is unavailable or fails due to pre-existing warnings, record that precisely.
4. Do not mark the milestone closed unless the meaningful verification gate is green or failures are clearly documented and non-blocking.

### Acceptance Criteria

- Verification state is inspectable without GitHub status checks.
- Future agents know exactly what was run.

## Workstream 4: Final Integration Tests for Report Truthfulness

### Required Tests

Add or confirm tests for:

1. No `require()` script → `report.libraries.is_empty()`.
2. Single `require "stdnse"` → only `stdnse` loaded, no unrelated registry entries.
3. Repeated `require "stdnse"` → stable de-duplicated report.
4. `pcall(require, "missing")` → missing/unregistered module appears with `loaded = false` and warning.
5. Static fallback path → entries are `loaded = false` and warning mentions static detection.
6. CLI JSON or equivalent run helper → `libraries` and `rules` are populated from runtime results, not fabricated placeholders.

### Acceptance Criteria

- Tests fail if all registry entries are marked loaded.
- Tests fail if empty placeholders return for scripts with observed `require()` calls.

## Workstream 5: Milestone 2 Closure Boundary

### Required Outcome

Add a short closure note stating that Milestone 2 is closed once this polish passes, and that Milestone 3 begins at capability wrappers / helper enforcement.

### Required Boundary Statement

Milestone 3 should focus on:

- capability wrappers for side-effecting Rust helpers;
- network/filesystem/process/time/randomness accounting;
- cancellation checks before and after blocking helper calls;
- profile-aware denial/allowance for helper operations;
- report integration for helper-side effects.

Milestone 3 should not:

- redesign loader/profile semantics;
- redo library registry truthfulness;
- claim full Nmap parity.

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

## Final Acceptance Criteria

This hardening/polish pass is complete when:

- Architecture guards reject all-registry-loaded and empty-placeholder report patterns.
- Documentation clearly distinguishes per-run usage from registry capability metadata.
- Tests protect no-require, single-require, missing-require, repeated-require, static fallback, and CLI/report behavior.
- Final verification is recorded.
- Milestone 2 has a closure note and Milestone 3 has a clean starting boundary.
