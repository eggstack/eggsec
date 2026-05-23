# Architecture Review Plan

This document outlines the plan to review architecture documents and verify implementation claims.

## Modules to Review

| Module | Architecture File | Implementation Path |
|--------|------------------|---------------------|
| AI Agents | `ai_agents.md` | `crates/slapper/src/ai/` |
| CLI Commands | `cli_commands.md` | `crates/slapper/src/cli/` |
| Config | `config.md` | `crates/slapper/src/config/` |
| Distributed | `distributed.md` | `crates/slapper/src/distributed/` |
| Fuzzer | `fuzzer.md` | `crates/slapper/src/fuzzer/` |
| Loadtest | `loadtest.md` | `crates/slapper/src/loadtest/` |
| Networking | `networking.md` | `crates/slapper/src/networking/` |
| Output | `output.md` | `crates/slapper/src/output/` |
| Pipeline | `pipeline.md` | `crates/slapper/src/pipeline/` |
| Plugins/NSE | `plugins_nse.md` | `crates/slapper-nse/src/` |
| Recon | `recon.md` | `crates/slapper/src/recon/` |
| Scanner | `scanner.md` | `crates/slapper/src/scanner/` |
| TUI | `tui.md` | `crates/slapper/src/tui/` |
| WAF | `waf.md` | `crates/slapper/src/waf/` |

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

- [ ] AI Agents (`ai_agents.md`)
- [ ] CLI Commands (`cli_commands.md`)
- [ ] Config (`config.md`)
- [ ] Distributed (`distributed.md`)
- [ ] Fuzzer (`fuzzer.md`)
- [ ] Loadtest (`loadtest.md`)
- [ ] Networking (`networking.md`)
- [ ] Output (`output.md`)
- [ ] Pipeline (`pipeline.md`)
- [ ] Plugins/NSE (`plugins_nse.md`)
- [ ] Recon (`recon.md`)
- [ ] Scanner (`scanner.md`)
- [ ] TUI (`tui.md`)
- [ ] WAF (`waf.md`)

## Expected Output

Each subagent will write an improvement plan to `plans/<module>_review.md` containing:
- Summary of what's implemented correctly
- List of bugs/issues with file:line references
- Recommended fixes
- Discrepancies between arch and impl

## Execution

Reviews will be executed in parallel using subagents. Results will be compiled once all subagents complete.