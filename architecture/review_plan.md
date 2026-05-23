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
| 12 | Recon | `architecture/recon.md` | `plans/recon_review.md` |
| 13 | Scanner | `architecture/scanner.md` | `plans/scanner_review.md` |
| 14 | TUI | `architecture/tui.md` | `plans/tui_review.md` |
| 15 | WAF | `architecture/waf.md` | `plans/waf_review.md` |

## Review Workflow

For each module, a subagent will:
1. Read the architecture document for the designated module
2. Search the codebase to locate the corresponding implementation module
3. Verify claims against the actual codebase implementation
4. Identify discrepancies, bugs, and improvement opportunities
5. Write a structured improvement plan to the designated output file in `plans/`

## Subagent Task Configuration

Each subagent will be given this task:

> Review the architecture document at `architecture/{module}.md`. 
> 
> Locate the corresponding implementation in the codebase (likely in `crates/slapper/src/{module}/`).
> 
> For each section in the architecture document:
> - Identify the key claims and design decisions
> - Search the codebase to verify each claim against actual implementation
> - Note any discrepancies between documentation and implementation
> - Identify bugs, performance issues, or anti-patterns
> - Suggest concrete improvements with estimated impact
> 
> Write your findings to `plans/{module}_review.md` with sections:
> - **Verified Claims** - What matches implementation
> - **Discrepancies** - Documentation vs implementation mismatches  
> - **Bugs Found** - Actual bugs discovered (with file:line references)
> - **Improvement Opportunities** - Refactoring and optimization suggestions
> - **Priority** - High/Medium/Low for each finding

## Execution Plan

Reviews will be executed in batches using subagents in parallel:

**Batch 1 (5 agents):** AI Agents, CLI Commands, Config, Distributed, Fuzzer  
**Batch 2 (5 agents):** Loadtest, Networking, Output, Overview, Pipeline  
**Batch 3 (5 agents):** Plugins/NSE, Recon, Scanner, TUI, WAF

## Verification Criteria

For each claim in the architecture document, subagents should verify:
- Type definitions match documented structures
- Function signatures match documented APIs
- Constants and magic numbers are documented
- Error handling matches documented behavior
- Performance characteristics match documented expectations

## Output Format

Each review file in `plans/` will contain:
```markdown
# {Module} Architecture Review

## Verified Claims
- [claim description] - Verified in [file:line]

## Discrepancies
- [issue] - Documented as X, Implementation is Y

## Bugs Found
1. **[HIGH/MEDIUM/LOW]** [brief title]
   - File: [path]
   - Description: [what's wrong]
   - Fix: [suggested approach]

## Improvement Opportunities
1. **[HIGH/MEDIUM/LOW]** [title]
   - Current: [description]
   - Suggested: [description]
   - Impact: [performance/correctness/maintainability]

## Summary
- Total Verified Claims: N
- Total Discrepancies: N
- Total Bugs: N
- Total Improvements: N
```