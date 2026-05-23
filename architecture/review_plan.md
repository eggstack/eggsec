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

## Execution

Launch all 15 module reviews in parallel using subagents.