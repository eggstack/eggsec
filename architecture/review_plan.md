# Architecture Review Plan (COMPLETED 2026-05-23)

This document outlines the plan to review architecture documents and verify implementation claims.

## Modules Reviewed

| Module | Architecture File | Implementation Path | Status | Notes |
|--------|------------------|---------------------|--------|-------|
| AI Agents | `ai_agents.md` | `crates/slapper/src/ai/` | ✅ | 1 issue: waf_bypass.rs:44 unwrap_or_default() |
| CLI Commands | `cli_commands.md` | `crates/slapper/src/cli/` | ✅ | No issues |
| Config | `config.md` | `crates/slapper/src/config/` | ✅ | FxHashMap correctly used, private IP blocking works |
| Distributed | `distributed.md` | `crates/slapper/src/distributed/` | ✅ | Worker capabilities string mismatch, env field issue, rate limit lock contention |
| Fuzzer | `fuzzer.md` | `crates/slapper/src/fuzzer/` | ✅ | Division-by-zero potential in analyzer.rs:190 |
| Loadtest | `loadtest.md` | `crates/slapper/src/loadtest/` | ✅ | Imprecise panic message in metrics.rs:76 |
| Networking | `networking.md` | `crates/slapper/src/networking/` | ✅ | Bounds check needed in DNS parsing |
| Output | `output.md` | `crates/slapper/src/output/` | ✅ | No issues |
| Pipeline | `pipeline.md` | `crates/slapper/src/pipeline/` | ✅ | All stages/profiles match docs |
| Plugins/NSE | `plugins_nse.md` | `crates/slapper-nse/src/` | ⚠️ | 4 files still using std HashMap: api.rs, http.rs, datafiles.rs, creds.rs |
| Recon | `recon.md` | `crates/slapper/src/recon/` | ⚠️ | 18 unwrap_or_default() in production, secrets not in pipeline |
| Scanner | `scanner.md` | `crates/slapper/src/scanner/` | ✅ | All bug fixes applied |
| TUI | `tui.md` | `crates/slapper/src/tui/` | ✅ | No issues |
| WAF | `waf.md` | `crates/slapper/src/waf/` | ✅ | 34 WAF products correct |

## Review Methodology

For each module, subagents will:

1. Read the architecture document to understand the intended design
2. Locate the implementation in the corresponding source path
3. Verify implementation matches documented claims
4. Check for bugs: `unwrap()`/`expect()` calls, HashMap/HashSet usage, error handling
5. Check for performance issues: FxHashMap/FxHashSet, lock contention, allocations
6. Verify patterns: traits, error handling, feature gating
7. Document findings in `plans/<module>_review.md`

## Subagent Tasks

### Subagent 1: AI Agents + CLI Commands
- Review `architecture/ai_agents.md` against `crates/slapper/src/ai/`
- Review `architecture/cli_commands.md` against `crates/slapper/src/cli/`
- Write findings to `plans/ai_review.md` and `plans/cli_review.md`

### Subagent 2: Config + Distributed
- Review `architecture/config.md` against `crates/slapper/src/config/`
- Review `architecture/distributed.md` against `crates/slapper/src/distributed/`
- Write findings to `plans/config_review.md` and `plans/distributed_review.md`

### Subagent 3: Fuzzer + Loadtest
- Review `architecture/fuzzer.md` against `crates/slapper/src/fuzzer/`
- Review `architecture/loadtest.md` against `crates/slapper/src/loadtest/`
- Write findings to `plans/fuzzer_review.md` and `plans/loadtest_review.md`

### Subagent 4: Networking + Output
- Review `architecture/networking.md` against `crates/slapper/src/networking/`
- Review `architecture/output.md` against `crates/slapper/src/output/`
- Write findings to `plans/networking_review.md` and `plans/output_review.md`

### Subagent 5: Pipeline + Plugins/NSE
- Review `architecture/pipeline.md` against `crates/slapper/src/pipeline/`
- Review `architecture/plugins_nse.md` against `crates/slapper-nse/src/`
- Write findings to `plans/pipeline_review.md` and `plans/nse_review.md`

### Subagent 6: Recon + Scanner
- Review `architecture/recon.md` against `crates/slapper/src/recon/`
- Review `architecture/scanner.md` against `crates/slapper/src/scanner/`
- Write findings to `plans/recon_review.md` and `plans/scanner_review.md`

### Subagent 7: TUI + WAF
- Review `architecture/tui.md` against `crates/slapper/src/tui/`
- Review `architecture/waf.md` against `crates/slapper/src/waf/`
- Write findings to `plans/tui_review.md` and `plans/waf_review.md`

## Review Checklist

- [x] AI Agents (`ai_agents.md`) - Issue: waf_bypass.rs:44
- [x] CLI Commands (`cli_commands.md`) - No issues
- [x] Config (`config.md`) - Correct implementation
- [x] Distributed (`distributed.md`) - 3 issues found
- [x] Fuzzer (`fuzzer.md`) - Division-by-zero potential
- [x] Loadtest (`loadtest.md`) - Imprecise panic message
- [x] Networking (`networking.md`) - DNS bounds check needed
- [x] Output (`output.md`) - No issues
- [x] Pipeline (`pipeline.md`) - All stages match docs
- [x] Plugins/NSE (`plugins_nse.md`) - 4 files with std HashMap
- [x] Recon (`recon.md`) - 18 unwrap_or_default(), secrets discrepancy
- [x] Scanner (`scanner.md`) - All bug fixes applied
- [x] TUI (`tui.md`) - No issues
- [x] WAF (`waf.md`) - 34 WAF products correct

## Findings Summary

### High Priority Issues

| Module | File | Issue |
|--------|------|-------|
| NSE | `public_api/api.rs` | Uses std HashMap at 4 locations (lines 107-108, 381, 413, 463, 486, 532) |
| Distributed | `worker.rs:115-123` | Worker capabilities strings don't match TaskType enum |
| Recon | Multiple | 18 unwrap_or_default() calls silently suppress errors |
| Networking | `parse_impl.rs:531` | DNS parsing needs bounds check |

### Medium Priority Issues

| Module | File | Issue |
|--------|------|-------|
| NSE | `libraries/http.rs:143` | Uses std HashMap |
| NSE | `libraries/datafiles.rs:31-33` | Uses std HashMap |
| NSE | `libraries/creds.rs:102,123` | Uses std HashSet |
| Distributed | `command.rs:146-149` | env field accepted but rejected at execution |
| Distributed | `remote.rs:127-146` | Rate limit holds write lock too long |
| Fuzzer | `analyzer.rs:190` | Division-by-zero potential |
| Loadtest | `metrics.rs:76` | Imprecise panic message |

### Documentation Discrepancies

| Module | Issue |
|--------|-------|
| Recon | secrets module not in FULL_RECON_PIPELINE_MODULES but documented |
| Recon | FxHashMap count (13 documented vs 55 actual) |
| Distributed | Worker capabilities string mismatch |

## Expected Output

Each subagent will write an improvement plan to `plans/<module>_review.md` containing:
- Summary of what's implemented correctly
- List of bugs/issues with file:line references
- Recommended fixes
- Discrepancies between arch and impl

## Execution

Reviews will be executed in parallel using subagents. Results will be compiled once all subagents complete.