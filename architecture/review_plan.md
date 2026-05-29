# Architecture Review Plan

**Status:** INCOMPLETE — Iterative improvement in progress

## Completion Log

- **Phase 1** (2026-06-09): All 15 module reviews completed
  - Group A: ai_agents, config, output, overview, waf
  - Group B: cli_commands, recon, scanner, loadtest, networking
  - Group C: distributed, fuzzer, pipeline, nse_integration, tui
- **Phase 2** (2026-06-09): Stale items detection completed → `plans/stale_items_review.md`
- **Phase 3** (2026-06-09): Synthesis completed → `plans/review_summary.md`

## Next Steps

- Address HIGH severity bugs identified in review summary
- Fix stale statistics in overview.md
- Fix discrepancies documented in per-module reviews

This document defines the systematic review of all architecture documents, verifying claims against the codebase, identifying bugs, and producing per-module improvement plans.

## Scope

Review all 15 architecture documents (excluding this file) against their corresponding source code. Each review produces a standalone improvement plan in `plans/`.

### Modules

| # | Module | Document | Implementation Path | Review File |
|---|--------|----------|---------------------|-------------|
| 1 | AI Agents | `ai_agents.md` | `crates/slapper/src/ai/`, `crates/slapper/src/agent/`, `crates/slapper/src/tool/` | `plans/ai_agents_review.md` |
| 2 | CLI Commands | `cli_commands.md` | `crates/slapper/src/cli/`, `crates/slapper/src/commands/` | `plans/cli_commands_review.md` |
| 3 | Config | `config.md` | `crates/slapper/src/config/` | `plans/config_review.md` |
| 4 | Distributed | `distributed.md` | `crates/slapper/src/distributed/` | `plans/distributed_review.md` |
| 5 | Fuzzer | `fuzzer.md` | `crates/slapper/src/fuzzer/` | `plans/fuzzer_review.md` |
| 6 | Loadtest | `loadtest.md` | `crates/slapper/src/loadtest/` | `plans/loadtest_review.md` |
| 7 | Networking | `networking.md` | `crates/slapper/src/packet/`, `crates/slapper/src/stress/` | `plans/networking_review.md` |
| 8 | Output | `output.md` | `crates/slapper/src/output/` | `plans/output_review.md` |
| 9 | Overview | `overview.md` | N/A (cross-cutting) | `plans/overview_review.md` |
| 10 | Pipeline | `pipeline.md` | `crates/slapper/src/pipeline/` | `plans/pipeline_review.md` |
| 11 | NSE | `nse_integration.md` | `slapper-nse/` | `plans/nse_integration_review.md` |
| 12 | Recon | `recon.md` | `crates/slapper/src/recon/` | `plans/recon_review.md` |
| 13 | Scanner | `scanner.md` | `crates/slapper/src/scanner/` | `plans/scanner_review.md` |
| 14 | TUI | `tui.md` | `crates/slapper/src/tui/` | `plans/tui_review.md` |
| 15 | WAF | `waf.md` | `crates/slapper/src/waf/` | `plans/waf_review.md` |

## Review Process

### Phase 1: Subagent Dispatch (parallel)

Launch one subagent per module. Each subagent:

1. Reads the architecture document completely.
2. Locates all source files referenced by the document.
3. For each section in the document:
   - Verify type definitions, function signatures, constants, and patterns against code.
   - Note any discrepancies (documented behavior vs actual implementation).
   - Search for anti-patterns: silent error suppression (`let _ =`), missing bounds checks, `unwrap()` in non-test code, `to_lowercase()` in loops, direct array indexing without bounds, missing `is_running()` guards, missing `is_empty()` guards on collections.
   - Check for division-by-zero risks, integer overflow/underflow, race conditions, resource leaks.
   - Identify stale references to moved/renamed/deleted modules.
4. Runs `cargo check --lib -p slapper` (and workspace crates if relevant) to confirm compilation.
5. Writes findings to `plans/{module}_review.md` in the prescribed format.

### Phase 2: Stale Item Detection

After all module reviews complete, a dedicated subagent:

1. Reads each architecture document and cross-references every module path, file reference, and type name against the current filesystem.
2. Flags:
   - References to files/modules that no longer exist.
   - Module paths that have moved.
   - Type names or function signatures that have changed.
   - Feature flags that are documented but no longer present in `Cargo.toml`.
   - Feature flags present in `Cargo.toml` but undocumented.
   - Statistics (file counts, test counts, library counts) that are stale.
3. Writes findings to `plans/stale_items_review.md` with a table of all stale references and recommended corrections.

### Phase 3: Synthesis

A final subagent:

1. Reads all `plans/*_review.md` files produced in Phases 1-2.
2. Produces `plans/review_summary.md` with:
   - Aggregate bug counts by severity.
   - Cross-module issues (patterns that appear in multiple modules).
   - Top 10 highest-impact improvements across the codebase.
   - Recommended priority order for addressing findings.

## Subagent Task Prompt

Each Phase 1 subagent receives this prompt (module-specific values substituted):

```
Review the architecture document at `architecture/{MODULE}.md`.

Your task:
1. Read the entire architecture document.
2. For each claim in the document, search the codebase to verify it exists as described.
3. Search the implementation directory for anti-patterns and bugs:
   - Silent error suppression: `let _ =`, `.ok()`, `.unwrap_or_default()` on Results
   - Missing bounds checks: direct array indexing without `.get()`
   - Missing guards: `is_running()`, `is_empty()` on collections before operations
   - Division by zero: division without `.max(1)` or equivalent guard
   - Resource leaks: missing `.abort()` on JoinHandle timeouts, missing cleanup
   - Unwrap in non-test code
   - `to_lowercase()` called inside loops instead of cached
   - HashMap used where FxHashMap would be appropriate
4. Check compilation: run `cargo check --lib -p slapper` (or relevant crate).
5. Write your findings to `plans/{MODULE}_review.md` using the format below.

Output format:

# {Module} Architecture Review

**Document:** architecture/{MODULE}.md
**Review Date:** {DATE}
**Implementation Path:** {PATH}

## Summary Statistics

| Metric | Count |
|--------|-------|
| Verified Claims | N |
| Discrepancies | N |
| Bugs Found | N |
| Improvements | N |

## Verified Claims
- [claim] — Verified in file:line

## Discrepancies
- [issue] — Documented as X, implementation is Y

## Bugs Found
1. **[HIGH/MEDIUM/LOW]** [title]
   - File: [path:line]
   - Description: [what's wrong]
   - Fix: [suggested approach]

## Improvement Opportunities
1. **[HIGH/MEDIUM/LOW]** [title]
   - Current: [description]
   - Suggested: [description]
   - Impact: [performance/correctness/maintainability]
```

## Dispatch Order

Modules are grouped by dependency to allow parallel execution:

**Group A** (independent, no cross-dependencies):
- AI Agents, Config, Output, Overview, WAF

**Group B** (light cross-references):
- CLI Commands, Recon, Scanner, Loadtest, Networking

**Group C** (heavy cross-references):
- Distributed, Fuzzer, Pipeline, Plugins/NSE, TUI

All groups launch in parallel. Each subagent works only in the working directory.

## Verification Criteria

For each documented claim, verify:
- Type definitions match: struct/enum names, fields, variants
- Function signatures match: name, parameters, return type
- Constants and magic numbers are documented and accurate
- Error handling matches documented behavior
- Feature flags match `Cargo.toml` definitions
- Statistics in overview.md match actual counts
- Inter-module dependency arrows in overview.md match actual `use` statements

## Stale Item Detection Criteria

For each architecture document, check:
- Every file path referenced exists on disk
- Every type name referenced exists in the source
- Every feature flag referenced exists in `Cargo.toml`
- Every module listed in tables has corresponding source directory
- Statistics (file counts, test counts, library counts) are current
- No orphaned sections describing removed functionality

## Output Files

| File | Produced By | Purpose |
|------|-------------|---------|
| `plans/{module}_review.md` | Phase 1 subagent | Per-module review findings |
| `plans/stale_items_review.md` | Phase 2 subagent | Cross-document stale reference audit |
| `plans/review_summary.md` | Phase 3 subagent | Aggregate analysis and priorities |

## Execution Notes

- All subagents work exclusively within the working directory.
- Subagents do NOT make code changes — they only write review/plan files.
- Each subagent runs `cargo check` to validate compilation.
- Review files use the prescribed markdown format for consistency.
- This plan is a review specification, not an implementation plan.
