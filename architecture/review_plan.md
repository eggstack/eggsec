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

### Wave 1 - Completed (2026-05-28)
- [x] ai_agents.md review - `plans/ai_agents_review.md` (commit: a24b0d2)
  - **Key findings**: unwrap() on SystemTime (planner.rs:208,469,482), silent persist errors (waf_bypass.rs:204-211), tool/planner.rs uses HashSet instead of FxHashSet
- [x] cli_commands.md review - `plans/cli_commands_review.md` (commit: 9599a46)
  - **Key findings**: HashMap in report.rs:44-57, ai_analyze.rs:38 unwrap(), webhook.rs:58 HashMap
- [x] config.md review - `plans/config_review.md` (commit: c4d0e66)
  - **Key findings**: Minor doc issue - ScanConfig.profiles reference in wrong location
- [x] distributed.md review - `plans/distributed_review.md` (commit: 97c0c2f)
  - **Key findings**: No bugs found - all documented fixes verified correct
- [x] fuzzer.md review - `plans/fuzzer_review.md` (commit: 44ff663)
  - **Key findings**: targets/api.rs HashMap usage (P1), unwrap_or_default() in calibration.rs, ssti.rs, oauth.rs, chain.rs (P2-P4), LazyLock regex unwrap() (chain.rs:381)

### Wave 2 - Completed (2026-05-28)
- [x] loadtest.md review - `plans/loadtest_review.md` (commit: on branch architecture/loadtest-review)
  - **Key findings**: Lock contention on shared metrics (MEDIUM), histogram expect() (LOW), progress bar silent fallback (LOW)
- [x] networking.md review - `plans/networking_review.md` (commit: f3d6b1b)
  - **Key findings**: capture.rs:47-53 silent system time error, capture.rs:208-210 silent pcap write errors, traceroute/udp join errors silently ignored
- [x] output.md review - `plans/output_review.md` (commit: 6fb87a1)
  - **Key findings**: convert.rs:88-89 silently swallows quick_xml::Error, markdown.rs:135 silent error suppression
- [x] overview.md review - `plans/overview_review.md` (commit: 41bc686)
  - **Key findings**: Minor - PayloadType has 31 variants not 30, 13 modules lack architecture docs (not critical)
- [x] pipeline.md review - `plans/pipeline_review.md` (commit: 4909b42)
  - **Key findings**: Arc::try_unwrap().expect() can panic (tool/implementations/pipeline.rs:111-112), blocking file I/O in async context (session.rs:15-24)

### Wave 3 - Completed (2026-05-28)
- [x] plugins_nse.md review - `plans/plugins_nse_review.md` (commit: fde6fa5)
  - **Key findings**: Duplicate function definitions in smbauth.rs, HashMap still in vulns.rs, rpc.rs, smbauth.rs, public_api/api.rs, creds.rs
- [x] recon.md review - `plans/recon_review.md` (commit: c0b9712)
  - **Key findings**: Regex compilation with unwrap() in LazyLock (email.rs, js.rs), silent error suppression in callback metadata (mod.rs:256-264), stubbed functions returning empty results
- [x] scanner.md review - `plans/scanner_review.md` (commit: 4439828)
  - **Key findings**: templates/marketplace.rs:266 unwrap() pattern should use unwrap_or_else
- [x] tui.md review - `plans/tui_review.md` (commit: 66a09c0)
  - **Key findings**: Key binding table outdated (b for toggle_bookmark should be Ctrl+b), unwrap_or_default() calls in state_update.rs, VecDeque in app/mod.rs:84
- [x] waf.md review - `plans/waf_review.md` (commit: fc48c85)
  - **Key findings**: Module documentation only lists 25 WAF products instead of 34 (mod.rs:16-21)

## Summary

All 15 architecture documents reviewed. Key recurring issues:
1. **HashMap/HashSet vs FxHashMap/FxHashSet** - Still present in some files (cli/report.rs, fuzzer/targets/api.rs, slapper-nse multiple files, plugins_nse/smbauth.rs)
2. **unwrap()/expect() calls** - Some in production code (planner.rs SystemTime, chain.rs LazyLock regex)
3. **Silent error suppression** - unwrap_or_default() in various places
4. **Documentation discrepancies** - Some module docs don't match actual implementation (config.md ScanConfig.profiles, waf.md 25 vs 34 WAFs, tui.md key bindings)

## Incomplete Items

None - all 15 reviews completed. See individual review files in `plans/` for detailed findings and recommended fixes.