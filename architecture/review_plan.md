# Architecture Review Plan

Generated: 2026-05-22

## Overview

This plan organizes the review of architecture documents in the `architecture/` directory. Each module will be reviewed by a dedicated subagent that will:
1. Read the architecture document
2. Verify claims against the actual implementation in `crates/slapper/src/`
3. Identify potential bugs, performance issues, and improvements
4. Write an improvement plan to `plans/<module>_review.md`

## Modules to Review

| # | Architecture Document | Module Path | Review Output |
|---|----------------------|-------------|---------------|
| 1 | `ai_agents.md` | `crates/slapper/src/ai/` | `plans/ai_agents_review.md` |
| 2 | `cli_commands.md` | `crates/slapper/src/cli/` | `plans/cli_commands_review.md` |
| 3 | `config.md` | `crates/slapper/src/config/` | `plans/config_review.md` |
| 4 | `distributed.md` | `crates/slapper/src/distributed/` | `plans/distributed_review.md` |
| 5 | `fuzzer.md` | `crates/slapper/src/fuzzer/` | `plans/fuzzer_review.md` |
| 6 | `loadtest.md` | `crates/slapper/src/loadtest/` | `plans/loadtest_review.md` |
| 7 | `networking.md` | `crates/slapper/src/networking/` or `packet/` | `plans/networking_review.md` |
| 8 | `output.md` | `crates/slapper/src/output/` | `plans/output_review.md` |
| 9 | `overview.md` | Full codebase | `plans/overview_review.md` |
| 10 | `pipeline.md` | `crates/slapper/src/pipeline/` | `plans/pipeline_review.md` |
| 11 | `plugins_nse.md` | `slapper-nse/src/` | `plans/plugins_nse_review.md` |
| 12 | `recon.md` | `crates/slapper/src/recon/` | `plans/recon_review.md` |
| 13 | `scanner.md` | `crates/slapper/src/scanner/` | `plans/scanner_review.md` |
| 14 | `tui.md` | `crates/slapper/src/tui/` | `plans/tui_review.md` |
| 15 | `waf.md` | `crates/slapper/src/waf/` | `plans/waf_review.md` |

## Review Methodology

For each module, the subagent should:
1. **Read the architecture document** to understand the intended design
2. **Identify key claims** - extract specific functionality, patterns, and behaviors described
3. **Verify against code** - locate the implementation in the codebase and check if it matches
4. **Check for bugs** - look for unwrap/expect calls, race conditions, error handling gaps
5. **Check for performance** - look for HashMap/HashSet usage, missing FxHashMap/FxHashSet
6. **Check for patterns** - verify traits, abstractions, and conventions are followed
7. **Write improvement plan** - document findings, issues, and recommended fixes

## Execution

Subagents will be launched in parallel to maximize efficiency. Each subagent will:
- Be given the full path to their architecture document
- Be given the corresponding module path in the codebase
- Write their output to the designated plans/ file
- Return a summary of findings

## Status

- [ ] ai_agents.md review
- [ ] cli_commands.md review
- [ ] config.md review
- [ ] distributed.md review
- [ ] fuzzer.md review
- [ ] loadtest.md review
- [ ] networking.md review
- [ ] output.md review
- [ ] overview.md review
- [ ] pipeline.md review
- [ ] plugins_nse.md review
- [ ] recon.md review
- [ ] scanner.md review
- [ ] tui.md review
- [ ] waf.md review