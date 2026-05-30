# Architecture Review Plan

## Overview

This plan orchestrates a systematic review of all architecture documents in `architecture/` (excluding `review_plan.md` itself). Each document is assigned to a dedicated subagent that will:
1. Read and analyze the architecture document
2. Verify claims against actual implementation in `crates/slapper/src/`
3. Identify bugs, inconsistencies, and improvement opportunities
4. Write findings to `plans/<module>_review.md`

After all reviews complete, stale items in `architecture/` will be pruned.

## Modules to Review

| # | Document | Module Directory | Review Output |
|---|----------|------------------|---------------|
| 1 | `ai_agents.md` | `src/ai/` | `plans/ai_agents_review.md` |
| 2 | `cli_commands.md` | `src/commands/` | `plans/cli_commands_review.md` |
| 3 | `config.md` | `src/config/` | `plans/config_review.md` |
| 4 | `defense_lab.md` | N/A | `plans/defense_lab_review.md` |
| 5 | `distributed.md` | `src/distributed/` | `plans/distributed_review.md` |
| 6 | `feature_matrix.md` | N/A | `plans/feature_matrix_review.md` |
| 7 | `fuzzer.md` | `src/fuzzer/` | `plans/fuzzer_review.md` |
| 8 | `loadtest.md` | `src/loadtest/` | `plans/loadtest_review.md` |
| 9 | `networking.md` | `src/packet/` | `plans/networking_review.md` |
| 10 | `nse_integration.md` | `slapper-nse/` | `plans/nse_integration_review.md` |
| 11 | `output.md` | `src/output/` | `plans/output_review.md` |
| 12 | `overview.md` | N/A | `plans/overview_review.md` |
| 13 | `pipeline.md` | `src/pipeline/` | `plans/pipeline_review.md` |
| 14 | `recon.md` | `src/recon/` | `plans/recon_review.md` |
| 15 | `scanner.md` | `src/scanner/` | `plans/scanner_review.md` |
| 16 | `tui.md` | `src/tui/` | `plans/tui_review.md` |
| 17 | `waf.md` | `src/waf/` | `plans/waf_review.md` |

## Review Process

### Phase 1: Document Review (Parallel Execution)

Launch 5 subagents concurrently to cover all 17 documents:

**Agent 1 - AI & Commands:**
- `ai_agents.md` → `plans/ai_agents_review.md`
- `cli_commands.md` → `plans/cli_commands_review.md`

**Agent 2 - Configuration & Infrastructure:**
- `config.md` → `plans/config_review.md`
- `output.md` → `plans/output_review.md`
- `pipeline.md` → `plans/pipeline_review.md`

**Agent 3 - Security Modules:**
- `fuzzer.md` → `plans/fuzzer_review.md`
- `scanner.md` → `plans/scanner_review.md`
- `waf.md` → `plans/waf_review.md`
- `recon.md` → `plans/recon_review.md`

**Agent 4 - Network & Protocol:**
- `networking.md` → `plans/networking_review.md`
- `loadtest.md` → `plans/loadtest_review.md`
- `distributed.md` → `plans/distributed_review.md`

**Agent 5 - Special Topics:**
- `nse_integration.md` → `plans/nse_integration_review.md`
- `tui.md` → `plans/tui_review.md`
- `overview.md` → `plans/overview_review.md`
- `defense_lab.md` → `plans/defense_lab_review.md`
- `feature_matrix.md` → `plans/feature_matrix_review.md`

### Phase 2: Stale Item Detection

After all reviews complete, detect stale architecture items:

1. **Outdated documents**: Check if any `architecture/*.md` documents reference modules/code that no longer exist
2. **Orphaned files**: Any architecture doc without a corresponding implementation in `crates/slapper/src/` or `slapper-nse/`
3. **Inconsistencies**: Compare document timestamps with implementation file timestamps
4. **Missing coverage**: Ensure all source modules have corresponding architecture docs

### Review Checklist for Each Document

Each subagent must verify:

- [ ] File paths and line numbers in doc match actual code
- [ ] Type definitions (`struct`, `enum`, `trait`) exist and match
- [ ] Method signatures and return types are accurate
- [ ] Error handling patterns described are implemented
- [ ] Configuration/schema details are current
- [ ] Dependencies and feature flags are accurate
- [ ] Any "known issues" or limitations still apply
- [ ] New patterns/optimizations not documented
- [ ] Deprecated patterns that should be removed from doc

### Output Format for Each Review File

```markdown
# <Module> Architecture Review

## Document Summary
- Last reviewed: <date>
- Accuracy rating: <High/Medium/Low>

## Verified Claims
- [List claims that pass verification]

## Discrepancies Found
- [List claims that don't match implementation]

## Bugs Identified
- [Bugs discovered during review]

## Improvement Opportunities
- [Code improvements suggested]

## Stale Items to Prune
- [Items to remove from architecture directory]
```

## Stale Item Pruning Process

1. Collect list of all `architecture/*.md` files
2. Cross-reference with actual module directories
3. Flag documents for orphaned modules
4. Review for duplicate content across documents
5. Check `plans/` for any stale references
6. Create `plans/stale_items.md` listing items to prune
7. Present to user for confirmation before removal

## Notes

- Reviews should NOT propose direct code changes
- Focus on verification and identification, not implementation
- Use `cargo check --lib -p slapper` to verify compilation after any structural changes
- Reference actual source files with line numbers for all claims