# Architecture Review Plan

**Status:** COMPLETED - All review phases complete, implementation in progress

This document outlines the plan for reviewing all architecture documents and verifying their claims against the codebase.

## Review Status

| Batch | Modules | Status |
|-------|---------|--------|
| Batch 1 | AI Agents, CLI Commands, Config, Distributed, Fuzzer | ✅ Reviews Complete |
| Batch 2 | Loadtest, Networking, Output, Overview, Pipeline | ✅ Reviews Complete |
| Batch 3 | Plugins/NSE, Recon, Scanner, TUI, WAF | ✅ Reviews Complete |

## Implementation Status

### Completed Fixes

| Branch | Fixes | Status |
|--------|-------|--------|
| `fix/ai-agents-hashmap-and-bugs` | AlertRoutingRules, ConstraintChecker, PortfolioData HashMap→FxHashMap; scope private IP bypass; WAF detector constant | ✅ Merged |
| `fix/distributed-pipeline-bugs` | Queue race condition fix; Pipeline session save error logging | ✅ Merged |
| `fix/scanner-fuzzer-improvements` | CMS error handling; Fuzzer adaptive rate limiter | ✅ Merged |
| `fix/loadtest-recon-improvements` | Loadtest error cap 100→1000; Cloud parallelization with tokio::join | ✅ Merged |

### Implementation Summary

| Issue | Module | Fix | Branch |
|-------|--------|-----|--------|
| AlertRoutingRules HashMap | agent/alerts | → FxHashMap | fix/ai-agents-hashmap-and-bugs |
| ConstraintChecker HashMap | agent/constraints | → FxHashMap | fix/ai-agents-hashmap-and-bugs |
| PortfolioData HashMap | agent/portfolio | → FxHashMap | fix/ai-agents-hashmap-and-bugs |
| Private IP bypass in parse() | config/scope | is_private_ip check | fix/ai-agents-hashmap-and-bugs |
| HEADER_VALUE_MAX_LEN in loop | waf/detector/detect | → module level | fix/ai-agents-hashmap-and-bugs |
| Queue race condition | distributed/queue | Atomic lock acquisition | fix/distributed-pipeline-bugs |
| Session save silent failure | pipeline/executor | warn→error | fix/distributed-pipeline-bugs |
| CMS unwrap_or_default | scanner/cms | explicit error handling | fix/scanner-fuzzer-improvements |
| rate ≤ 1 premature stop | fuzzer/engine/execution | rate < 1 | fix/scanner-fuzzer-improvements |
| Error list cap 100 | loadtest/metrics | 100→1000 | fix/loadtest-recon-improvements |
| Sequential cloud enum | recon/cloud | tokio::join! | fix/loadtest-recon-improvements |

### Remaining Issues (Not Fixed)

These issues were identified but not fixed due to scope constraints or needing further design:

| Issue | Module | Priority | Notes |
|-------|--------|----------|-------|
| CVE duplicate entry | plugins_nse | Medium | Needs data structure change |
| load_plugin_with_timeout unused | plugins_nse | Medium | API change needed |
| Async CVE blocking HTTP | recon | Medium | Would require API redesign |
| TUI unwrap_or_default (14 instances) | tui | Medium | Pre-existing, many files |
| WAF HTTP/2 always disabled | waf | Medium | Feature flag needed |
| Networking IPv4 options bounds | networking | Low | Edge case |
| ReDoS patterns clone | fuzzer | Low | Arc::clone instead |

## Modules Reviewed

| # | Module | Document | Issues Found |
|---|--------|----------|--------------|
| 1 | AI Agents | `architecture/ai_agents.md` | HashMap usage, cache key collision |
| 2 | CLI Commands | `architecture/cli_commands.md` | Missing CLI files in docs |
| 3 | Config | `architecture/config.md` | Private IP bypass (HIGH) |
| 4 | Distributed | `architecture/distributed.md` | Race condition (HIGH) |
| 5 | Fuzzer | `architecture/fuzzer.md` | Adaptive rate limiter (MEDIUM) |
| 6 | Loadtest | `architecture/loadtest.md` | Error list cap 100 |
| 7 | Networking | `architecture/networking.md` | UDP checksum allocation |
| 8 | Output | `architecture/output.md` | None critical |
| 9 | Overview | `architecture/overview.md` | unwrap_or_default (pre-existing) |
| 10 | Pipeline | `architecture/pipeline.md` | Session save error (MEDIUM) |
| 11 | Plugins/NSE | `architecture/plugins_nse.md` | CVE duplicate, timeout unused |
| 12 | Recon | `architecture/recon.md` | Cloud sequential (HIGH→fixed) |
| 13 | Scanner | `architecture/scanner.md` | CMS unwrap_or_default |
| 14 | TUI | `architecture/tui.md` | 14 unwrap_or_default instances |
| 15 | WAF | `architecture/waf.md` | Constant in loop, HTTP/2 disabled |

## Key Findings Summary

- **High Priority Fixed:** 2 (Config private IP bypass, Distributed race condition)
- **Medium Priority Fixed:** 4 (Fuzzer rate limiter, Scanner CMS, Loadtest errors, Recon cloud)
- **Low Priority Fixed:** 2 (WAF constant, Cloud parallelization)

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