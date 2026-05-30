# Architecture Stale Items Report

**Generated:** 2026-05-30
**Phase:** Phase 2: Stale Item Detection

## Summary

Based on the comprehensive review of all 17 architecture documents in `architecture/`, the following stale items have been identified that should be pruned or updated.

## Stale Items to Prune/Update

### 1. `architecture/overview.md` - Stale Statistics

**Issue:** Quick Facts section contains outdated statistics:

| Statistic | Documented | Actual | Action |
|-----------|------------|--------|--------|
| Modules | 41 | 39 | UPDATE |
| Source files | 743 | 526 | UPDATE |
| Payload types | 31 | 30 | UPDATE |
| NSE libraries | 164+ | 169 | UPDATE |

**Recommended Action:** UPDATE the Quick Facts section (lines 5-12) with current counts.

### 2. `architecture/defense_lab.md` - Implementation Status Outdated

**Issue:** Document claims (line 100-102) that defense-lab profiles are "planned but not yet implemented" and references TODOs in `cli/mod.rs` and `pipeline/stage.rs`.

**Reality:** All 5 defense-lab profiles are fully implemented:
- `DefenseLab` at `cli/mod.rs:262`, `stage.rs:92-98`
- `SynvoidLocal` at `cli/mod.rs:263`, `stage.rs:99-104`
- `WafRegression` at `cli/mod.rs:264`, `stage.rs:105`
- `ProtocolEdge` at `cli/mod.rs:265`, `stage.rs:106`
- `NseSafe` at `cli/mod.rs:266`, `stage.rs:107`

**Recommended Action:** UPDATE "Planned Defense-Lab Profiles" section to reflect implementation status.

### 3. `architecture/pipeline.md` - Same Defense-Lab Issue

**Issue:** Lines 88-100 describe profiles as "planned but not yet implemented" with TODOs in `cli/mod.rs` and `pipeline/stage.rs`.

**Recommended Action:** UPDATE section to reflect profiles are implemented.

### 4. `architecture/feature_matrix.md` - Incorrect Feature Counts

**Issue:** Summary section claims:

| Statistic | Documented | Actual |
|-----------|------------|--------|
| Total features | 33 | 28 |
| In `full` | 18 | 16 |

**Recommended Action:** UPDATE Summary table with correct counts.

### 5. `architecture/tui.md` - Tab Count and Reference Issues

**Issue 1:** Document says "29 different tabs" (lines 3, 23) but `Tab` enum has 28 entries.

**Issue 2:** Line 1111 in session fixes table references "plugin" tab which doesn't exist.

**Recommended Action:**
- UPDATE tab count documentation
- REMOVE stale "plugin" reference

### 6. `architecture/cli_commands.md` - Line Number References

**Issue:** Line references in bug fix section are outdated (e.g., handlers/mod.rs:155-169 is actually 197-206).

**Recommended Action:** UPDATE or remove specific line number references from bug fix section.

## Documents Requiring Updates Only (Not Pruning)

| Document | Issue | Action |
|----------|-------|--------|
| `ai_agents.md` | Line numbers stale in bug fix section | UPDATE references |
| `cli_commands.md` | Line numbers stale, cluster.rs fix not applied | UPDATE references, APPLY fix |
| `config.md` | Some field locations in different files than documented | UPDATE file references |
| `fuzzer.md` | Missing `calibration.rs` and `chain.rs` modules | ADD to documentation |
| `loadtest.md` | `run_cli()` signature is async, requires config | UPDATE signature |
| `networking.md` | UDP IPv6 spoofing not supported | UPDATE to clarify |
| `nse_integration.md` | Library count 164+ vs actual 169 | UPDATE count |
| `output.md` | Some type locations incorrect in table | UPDATE table |
| `pipeline.md` | Defense-lab profiles implemented | UPDATE status |
| `recon.md` | 14 tasks vs 13 unconditional | UPDATE task count |
| `scanner.md` | 224 endpoints vs 223 actual | UPDATE count |
| `waf.md` | WAF list shows 29 names but claims 34 | UPDATE list |

## Orphaned Architecture Documents

**None identified.** All architecture documents have corresponding implementation directories:
- `ai_agents.md` → `src/ai/`, `src/agent/`
- `cli_commands.md` → `src/cli/`, `src/commands/`
- `config.md` → `src/config/`
- `defense_lab.md` → N/A (design doc, not implemented module - content is stale)
- `distributed.md` → `src/distributed/`
- `feature_matrix.md` → N/A (reference doc, no module)
- `fuzzer.md` → `src/fuzzer/`
- `loadtest.md` → `src/loadtest/`
- `networking.md` → `src/packet/`, `src/stress/`
- `nse_integration.md` → `slapper-nse/`
- `output.md` → `src/output/`
- `overview.md` → N/A (index doc)
- `pipeline.md` → `src/pipeline/`
- `recon.md` → `src/recon/`
- `scanner.md` → `src/scanner/`
- `tui.md` → `src/tui/`
- `waf.md` → `src/waf/`

## Stale References in Plans Directory

**None identified.** All review files in `plans/` are newly created as part of this review cycle.

## Recommended Actions

1. **UPDATE** `architecture/overview.md` - Fix Quick Facts statistics
2. **UPDATE** `architecture/defense_lab.md` - Mark profiles as implemented
3. **UPDATE** `architecture/pipeline.md` - Mark profiles as implemented
4. **UPDATE** `architecture/feature_matrix.md` - Fix feature counts
5. **UPDATE** `architecture/tui.md` - Fix tab count, remove "plugin" reference
6. **UPDATE** Various documents - Fix stale line number references

## Items NOT to Prune

The following items should NOT be pruned despite being flagged in initial analysis:
- `defense_lab.md` - Still useful as design documentation, just needs updating
- `feature_matrix.md` - Reference document, useful, just needs updating
- `overview.md` - Index document, useful, just needs updating
- `nse_tool/` reference in `overview.md` - The `nse_tool.rs` file exists (not a directory as previously suspected)

## Verification

All stale items were identified through systematic review by subagents verifying claims against actual implementation. No items should be pruned without first verifying the implementation status has not changed.