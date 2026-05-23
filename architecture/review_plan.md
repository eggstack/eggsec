# Architecture Review Plan

This document outlines the plan for reviewing all architecture documents and verifying their claims against the codebase.

## Modules to Review

| # | Module | Document | Review Agent Output |
|---|--------|----------|---------------------|
| 1 | AI Agents | `architecture/ai_agents.md` | `plans/ai_agents_review.md` |
| 2 | CLI Commands | `architecture/cli_commands.md` | `plans/cli_commands_review.md` |
| 3 | Config | `architecture/config.md` | `plans/config_review.md` |
| 4 | Distributed | `architecture/distributed.md` | `plans/distributed_review.md` |
| 5 | Fuzzer | `architecture/fuzzer.md` | `plans/fuzzer_review.md` |
| 6 | Loadtest | `architecture/loadtest.md` | `plans/loadtest_review.md` |
| 7 | Networking | `architecture/networking.md` | `plans/networking_review.md` |
| 8 | Output | `architecture/output.md` | `plans/output_review.md` |
| 9 | Overview | `architecture/overview.md` | `plans/overview_review.md` |
| 10 | Pipeline | `architecture/pipeline.md` | `plans/pipeline_review.md` |
| 11 | Plugins/NSE | `architecture/plugins_nse.md` | `plans/plugins_nse_review.md` |
| 12 | Scanner | `architecture/scanner.md` | `plans/scanner_review.md` |
| 13 | TUI | `architecture/tui.md` | `plans/tui_review.md` |
| 14 | WAF | `architecture/waf.md` | `plans/waf_review.md` |
| 15 | Recon | `architecture/recon.md` | `plans/recon_review.md` |

## Review Workflow

For each module, a subagent will:
1. Read the architecture document
2. Verify claims against the actual codebase implementation
3. Identify discrepancies, bugs, and improvement opportunities
4. Write a structured improvement plan to the designated output file in `plans/`

## Subagent Prompts

Each subagent will be given this task:

> Review the architecture document at `architecture/{module}.md`. For each section:
> - Identify the key claims and design decisions
> - Search the codebase to verify each claim
> - Note any discrepancies between documentation and implementation
> - Identify bugs, performance issues, or anti-patterns
> - Suggest concrete improvements
> 
> Write your findings to `plans/{module}_review.md` with sections:
> - **Verified Claims** - What matches implementation
> - **Discrepancies** - Documentation vs implementation mismatches
> - **Bugs Found** - Actual bugs discovered
> - **Improvement Opportunities** - Refactoring and optimization suggestions
> - **Priority** - High/Medium/Low for each finding

## Review Status

✅ **PHASE 1: Reviews Complete** - All 15 module reviews finished (2026-05-28)
✅ **PHASE 2: Implementation Complete** - All 4 waves implemented (2026-05-28)

## Implementation Summary

| Wave | Items | Status | Commit |
|------|-------|--------|--------|
| Wave 1 | 8 production-critical bugs | ✅ Completed | `baddde3` |
| Wave 2 | 10 high-priority issues | ✅ Completed | `baddde3` |
| Wave 3 | 10 medium-priority improvements | ✅ Completed | `997df0e` |
| Wave 4 | 12 low-priority/documentation fixes | ✅ Completed | `89e20e8` |

## Key Fixes by Module

| Module | Key Fixes |
|--------|-----------|
| Distributed | QueueError traits, unified CAPABILITIES constant |
| Networking | UDP checksum includes payload, TCP checksum computed |
| Tool | FxHashMap in ToolRegistry |
| Pipeline | spoof_config persisted in PipelineSession |
| Scanner | Static slice references for fingerprint probes |
| AI | FxHashMap in agent modules, explicit error handling |
| WAF | Static reference for signatures, HEADER_VALUE_MAX_LEN constant |
| Output | has_regressions checks Severity::High |
| Loadtest | Metrics lock optimization, JoinSet panic handling |

## Not Implemented (Known Limitations)

| # | Module | Issue | Reason |
|---|--------|-------|--------|
| 20 | CLI | EndpointScanArgs uses spoof_ip not source_ip | Breaking API change |
| 22 | Distributed | Heartbeat connection churn | Requires connection pooling refactor |
| 23 | Distributed | Rate limit race condition | Lock restructuring needed |
| 24 | Networking | ICMP IPv6 parsing missing | Requires ICMPv6 protocol impl |
| 26 | Scanner | UDP socket per-port binding | Requires session-level socket pooling |
| 37 | Recon | CveMapper cache doesn't persist | Would need module-level cache or file persistence |
| 40 | WAF | Magic number 256 | Already fixed |

## Execution

All reviews and implementations completed. See `plans/*_review.md` for detailed findings.