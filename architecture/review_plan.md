# Architecture Review Plan

**Status:** IN PROGRESS - Review batches dispatched

This document outlines the plan for reviewing all architecture documents and verifying their claims against the codebase.

## Overview

For each architecture module, a subagent will:
1. Read the architecture document
2. Locate the corresponding implementation in the codebase
3. Verify claims against actual implementation
4. Identify bugs, discrepancies, and improvement opportunities
5. Write findings to `plans/{module}_review.md`

## Architecture Modules to Review

| # | Module | Document | Implementation Path |
|---|--------|----------|---------------------|
| 1 | AI Agents | `ai_agents.md` | `crates/slapper/src/agent/` |
| 2 | CLI Commands | `cli_commands.md` | `crates/slapper/src/cli/` |
| 3 | Config | `config.md` | `crates/slapper/src/config/` |
| 4 | Distributed | `distributed.md` | `crates/slapper/src/distributed/` |
| 5 | Fuzzer | `fuzzer.md` | `crates/slapper/src/fuzzer/` |
| 6 | Loadtest | `loadtest.md` | `crates/slapper/src/loadtest/` |
| 7 | Networking | `networking.md` | `crates/slapper/src/networking/` |
| 8 | Output | `output.md` | `crates/slapper/src/output/` |
| 9 | Overview | `overview.md` | N/A (high-level summary) |
| 10 | Pipeline | `pipeline.md` | `crates/slapper/src/pipeline/` |
| 11 | Plugins/NSE | `plugins_nse.md` | `slapper-nse/` |
| 12 | Recon | `recon.md` | `crates/slapper/src/recon/` |
| 13 | Scanner | `scanner.md` | `crates/slapper/src/scanner/` |
| 14 | TUI | `tui.md` | `crates/slapper/src/tui/` |
| 15 | WAF | `waf.md` | `crates/slapper/src/waf/` |

## Review Batches

### Batch 1
- AI Agents → `plans/ai_agents_review.md`
- CLI Commands → `plans/cli_commands_review.md`
- Config → `plans/config_review.md`
- Distributed → `plans/distributed_review.md`
- Fuzzer → `plans/fuzzer_review.md`

### Batch 2
- Loadtest → `plans/loadtest_review.md`
- Networking → `plans/networking_review.md`
- Output → `plans/output_review.md`
- Overview → `plans/overview_review.md`
- Pipeline → `plans/pipeline_review.md`

### Batch 3
- Plugins/NSE → `plans/plugins_nse_review.md`
- Recon → `plans/recon_review.md`
- Scanner → `plans/scanner_review.md`
- TUI → `plans/tui_review.md`
- WAF → `plans/waf_review.md`

## Subagent Task Configuration

Each subagent should perform the following task for their designated module:

> Review the architecture document at `architecture/{module}.md`.
>
> Locate the corresponding implementation in the codebase (likely in `crates/slapper/src/{module}/` or `slapper-nse/` for NSE).
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
> - **Summary Statistics** - Count of verified claims, discrepancies, bugs, and improvements

## Verification Criteria

For each claim in the architecture document, verify:
- Type definitions match documented structures
- Function signatures match documented APIs
- Constants and magic numbers are documented
- Error handling matches documented behavior
- Performance characteristics match documented expectations
- Security considerations are properly implemented

## Output Format

Each review file in `plans/` should follow this structure:

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

## Review Status

| Batch | Modules | Status |
|-------|---------|--------|
| Batch 1 | AI Agents, CLI Commands, Config, Distributed, Fuzzer | ✅ Complete |
| Batch 2 | Loadtest, Networking, Output, Overview, Pipeline | ✅ Complete |
| Batch 3 | Plugins/NSE, Recon, Scanner, TUI, WAF | ✅ Complete |

## Key Findings Summary

### Critical Bugs Found

| Module | Bug | Priority | File:Line |
|--------|-----|----------|-----------|
| Distributed | Task results processed but never sent to coordinator | CRITICAL | `worker.rs:169-183` |
| Distributed | WorkerStats/heartbeat hardcoded to zero | HIGH | `worker.rs:78-82`, `worker.rs:151-157` |
| AI Agents | MCP integration claimed but not implemented | HIGH | Documentation gap |
| CLI Commands | Resume scope bypass | HIGH | `scan.rs:60` |
| CLI Commands | Stress handler scope missing | HIGH | `stress.rs:9` |
| Loadtest | Rate limiting initial burst issue | HIGH | `runner.rs:279` |
| WAF | Cookie matching index fallback incorrect | MEDIUM | `unwrap_or(0)` |

### Review Files Created

| Module | Review File | Verified Claims | Bugs Found | Improvements |
|--------|-------------|-----------------|------------|--------------|
| AI Agents | `plans/ai_agents_review.md` | 32 | 3 | 8 |
| CLI Commands | `plans/cli_commands_review.md` | 18 | 7 | 12 |
| Config | `plans/config_review.md` | 24 | 0 | 1 |
| Distributed | `plans/distributed_review.md` | 9 | 3 | 7 |
| Fuzzer | `plans/fuzzer_review.md` | - | - | - |
| Loadtest | `plans/loadtest_review.md` | 14 | 2 | 4 |
| Networking | `plans/networking_review.md` | 15 | 4 | 8 |
| Output | `plans/output_review.md` | 23 | 3 | 6 |
| Overview | `plans/overview_review.md` | - | - | - |
| Pipeline | `plans/pipeline_review.md` | - | - | - |
| Plugins/NSE | `plans/plugins_nse_review.md` | - | - | - |
| Recon | `plans/recon_review.md` | 28 | 4 | 8 |
| Scanner | `plans/scanner_review.md` | 14 | 2 | 4 |
| TUI | `plans/tui_review.md` | 18 | 2 | 7 |
| WAF | `plans/waf_review.md` | 18 | 3 | 7 |