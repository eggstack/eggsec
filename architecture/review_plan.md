# Architecture Review Plan

**Status:** READY FOR EXECUTION
**Created:** 2026-05-31
**Purpose:** Systematic review of all architecture documents, verification against codebase, and stale item pruning.

## Overview

This plan orchestrates a deep review of all 17 architecture documents in `architecture/` (excluding `review_plan.md`). Each document is assigned to a dedicated subagent that will:

1. Read and analyze the architecture document thoroughly
2. Verify every claim against actual implementation in `crates/slapper/src/` and `slapper-nse/`
3. Identify bugs, inconsistencies, and improvement opportunities
4. Write findings to `plans/<module>_review.md`

After all reviews complete, a final pass detects and prunes stale items from `architecture/`.

## Modules to Review

| # | Document | Module Directory | Review Output |
|---|----------|------------------|---------------|
| 1 | `ai_agents.md` | `src/ai/` | `plans/ai_agents_review.md` |
| 2 | `cli_commands.md` | `src/commands/` | `plans/cli_commands_review.md` |
| 3 | `config.md` | `src/config/` | `plans/config_review.md` |
| 4 | `defense_lab.md` | N/A (cross-cutting) | `plans/defense_lab_review.md` |
| 5 | `distributed.md` | `src/distributed/` | `plans/distributed_review.md` |
| 6 | `feature_matrix.md` | N/A (cross-cutting) | `plans/feature_matrix_review.md` |
| 7 | `fuzzer.md` | `src/fuzzer/` | `plans/fuzzer_review.md` |
| 8 | `loadtest.md` | `src/loadtest/` | `plans/loadtest_review.md` |
| 9 | `networking.md` | `src/packet/` | `plans/networking_review.md` |
| 10 | `nse_integration.md` | `slapper-nse/` | `plans/nse_integration_review.md` |
| 11 | `output.md` | `src/output/` | `plans/output_review.md` |
| 12 | `overview.md` | N/A (cross-cutting) | `plans/overview_review.md` |
| 13 | `pipeline.md` | `src/pipeline/` | `plans/pipeline_review.md` |
| 14 | `recon.md` | `src/recon/` | `plans/recon_review.md` |
| 15 | `scanner.md` | `src/scanner/` | `plans/scanner_review.md` |
| 16 | `tui.md` | `src/tui/` | `plans/tui_review.md` |
| 17 | `waf.md` | `src/waf/` | `plans/waf_review.md` |

## Execution Plan

### Phase 1: Document Reviews (Parallel Subagents)

Launch 5 subagents concurrently. Each agent receives a batch of documents to review. Every agent must follow the Review Checklist and output format specified below.

#### Agent 1 - AI & CLI
- `architecture/ai_agents.md` → `plans/ai_agents_review.md`
- `architecture/cli_commands.md` → `plans/cli_commands_review.md`

#### Agent 2 - Configuration & Core
- `architecture/config.md` → `plans/config_review.md`
- `architecture/output.md` → `plans/output_review.md`
- `architecture/pipeline.md` → `plans/pipeline_review.md`

#### Agent 3 - Security Modules
- `architecture/fuzzer.md` → `plans/fuzzer_review.md`
- `architecture/scanner.md` → `plans/scanner_review.md`
- `architecture/waf.md` → `plans/waf_review.md`
- `architecture/recon.md` → `plans/recon_review.md`

#### Agent 4 - Network & Distributed
- `architecture/networking.md` → `plans/networking_review.md`
- `architecture/loadtest.md` → `plans/loadtest_review.md`
- `architecture/distributed.md` → `plans/distributed_review.md`

#### Agent 5 - Special Topics & Cross-Cutting
- `architecture/nse_integration.md` → `plans/nse_integration_review.md`
- `architecture/tui.md` → `plans/tui_review.md`
- `architecture/overview.md` → `plans/overview_review.md`
- `architecture/defense_lab.md` → `plans/defense_lab_review.md`
- `architecture/feature_matrix.md` → `plans/feature_matrix_review.md`

### Phase 2: Stale Item Detection

After all 17 review files exist in `plans/`:

1. **Outdated references**: Check if any `architecture/*.md` documents reference modules, files, or types that no longer exist in the codebase
2. **Orphaned documents**: Any architecture doc without a corresponding implementation in `crates/slapper/src/` or `slapper-nse/`
3. **Statistical drift**: Compare documented counts (files, modules, tabs, payloads, etc.) against actual codebase metrics
4. **Missing coverage**: Ensure all source modules under `crates/slapper/src/` have corresponding architecture docs
5. **Duplicate content**: Flag overlapping information across documents
6. **Write findings** to `plans/stale_items.md`

### Phase 3: Verification & Commit

1. Run `cargo check --lib -p slapper` to verify no structural breakage
2. Run `cargo test --lib -p slapper` to verify tests pass
3. Commit all `plans/*_review.md` files and `plans/stale_items.md`
4. Commit updated `architecture/review_plan.md` with final status

## Review Checklist

Each subagent MUST verify the following for every document:

- [ ] **File paths**: All referenced file paths exist in the codebase
- [ ] **Line numbers**: Cited line numbers match actual code locations
- [ ] **Type definitions**: All `struct`, `enum`, `trait` names exist and match signatures
- [ ] **Method signatures**: Documented methods exist with correct parameters and return types
- [ ] **Error handling**: Described error patterns are actually implemented
- [ ] **Configuration**: Schema details, defaults, and environment variables are current
- [ ] **Dependencies**: Referenced crates and feature flags are accurate
- [ ] **Known issues**: Any "TODO", "known limitation", or "planned" items still apply
- [ ] **Undocumented changes**: New patterns, optimizations, or APIs not yet in docs
- [ ] **Deprecated content**: Patterns marked deprecated that should be removed from doc
- [ ] **Statistics**: Counts of modules, files, tabs, payloads, etc. match reality
- [ ] **Cross-references**: Links between architecture docs are valid

## Output Format

Each review file MUST use this structure:

```markdown
# <Module> Architecture Review

**Document:** architecture/<module>.md
**Reviewed:** <date>
**Accuracy:** <High/Medium/Low>

## Verified Claims
- [Claim 1]: Verified at <file:line>
- [Claim 2]: Verified at <file:line>

## Discrepancies
- [Claim X]: Documented as <X>, but actual implementation is <Y> (<file:line>)

## Bugs Found
- [Bug 1]: <Description> (<file:line>)

## Improvement Opportunities
- [Improvement 1]: <Description> (priority: high/medium/low)

## Stale Items
- [Item 1]: <Why it's stale and recommended action>
```

## Constraints

- **No code changes**: Reviews identify and document only. Do NOT edit source files.
- **No assumptions**: If a claim cannot be verified, mark it as "UNVERIFIED" with reason.
- **Line references**: All claims must cite `<file:line>` for traceability.
- **Scope**: Only review what the document claims. Don't expand scope beyond the doc's topic.
- **Working directory**: All work stays in `/home/sugarwookie/projects/slapper/`.

## Notes

- Cross-cutting docs (`overview.md`, `feature_matrix.md`, `defense_lab.md`) require checking against ALL modules, not just one
- The `tui.md` document is the largest; agent should focus on structural claims, not every pixel detail
- `nse_integration.md` spans a separate crate (`slapper-nse/`); agent must check both crates
- Feature flags in `Cargo.toml` at root and `crates/slapper/Cargo.toml` must be cross-referenced
