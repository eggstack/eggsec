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
- Review outputs stored in `plans/*_review.md`
- 35+ issues identified across all modules

## Implementation Waves

### Wave 1: Production-Critical Bugs (8 items)
| # | Module | Issue | File | Type |
|---|--------|-------|------|------|
| 1 | Distributed | QueueError missing Display/Error traits | queue.rs:150-154 | Bug |
| 2 | Distributed | Capabilities mismatch (worker vs coordinator) | worker.rs:32-45, remote.rs:105-121 | Bug |
| 3 | Networking | UDP checksum calculation incomplete | stress/udp.rs:82-113 | Bug |
| 4 | Networking | TCP checksum not computed | craft.rs:236-253 | Bug |
| 5 | Overview | ToolRegistry uses HashMap not FxHashMap | tool/registry.rs:2,24 | Bug |
| 6 | Pipeline | Session restoration loses spoof_config | executor.rs:134-140 | Bug |
| 7 | Plugins/NSE | Duplicate CVE-2024-27956 entry | vulns.rs:209-238 | Bug |
| 8 | Scanner | Dynamic Vec allocation in fingerprint hot path | fingerprint.rs:347-391 | Bug |

### Wave 2: High-Priority Issues (10 items)
| # | Module | Issue | File | Priority |
|---|--------|-------|------|----------|
| 9 | AI Agents | FxHashMap migration (skills.rs, portfolio.rs) | agent/*.rs | Medium |
| 10 | AI Agents | Silent error handling in script_gen.rs | script_gen.rs:97,141,185,272 | Medium |
| 11 | AI Agents | Anthropic message transformation silent fallback | client.rs:241 | Medium |
| 12 | CLI Commands | WafStressArgs output silently discarded | fuzz.rs:292 | Medium |
| 13 | Fuzzer | Adaptive rate limiter can reach zero | rate_limit.rs:106-113 | Medium |
| 14 | Loadtest | Metrics lock held during async body read | runner.rs:336 | Medium |
| 15 | Loadtest | JoinSet panic handling missing | runner.rs:360-363 | Medium |
| 16 | Output | has_regressions only checks Critical | diff.rs:136-140 | Medium |
| 17 | WAF | Profile auto-detection linear scan slow | waf/mod.rs:151-162 | High |
| 18 | WAF | get_waf_signatures clones entire map | patterns.rs:656-657 | Medium |

### Wave 3: Medium-Priority Improvements (10 items)
| # | Module | Issue | File | Priority |
|---|--------|-------|------|----------|
| 19 | AI Agents | AiCache persist() could fail silently | cache.rs:276-278 | Low |
| 20 | CLI Commands | EndpointScanArgs uses spoof_ip instead of source_ip | scan.rs:194 | Medium |
| 21 | Config | DNS resolution failure should fail closed for CIDR | scope.rs:58-97 | Medium |
| 22 | Distributed | Heartbeat creates new connection every time | worker.rs:137-172 | Medium |
| 23 | Distributed | Rate limit race condition | remote.rs:127-146 | Medium |
| 24 | Networking | ICMP IPv6 parsing missing | parse_impl.rs:166-212 | Medium |
| 25 | Pipeline | Hardcoded default ports in two locations | executor.rs:276-282 | Medium |
| 26 | Scanner | UDP socket per-port binding | udp_fingerprint.rs:169 | Medium |
| 27 | Scanner | Missing error context in spoofed scan | spoofed.rs:285-307 | Medium |
| 28 | WAF | EvasionBypass generates redundant payloads | bypass/evasion.rs:101-157 | Medium |

### Wave 4: Documentation & Low-Priority Fixes (12 items)
| # | Module | Issue | File | Priority |
|---|--------|-------|------|----------|
| 29 | AI Agents | planner.rs and script_gen.rs missing feature gates | ai/*.rs | Low |
| 30 | CLI Commands | Command count mismatch (35+ vs 41) | cli/mod.rs | Low |
| 31 | Config | validate_url returns Ok(false) not Err | scope.rs:117-126 | Low |
| 32 | Fuzzer | Payload type count off by one in docs | fuzzer/payloads/mod.rs | Low |
| 33 | Overview | SecurityTool trait documentation incomplete | tool/traits.rs | Medium |
| 34 | Pipeline | Profile-to-stages mapping duplication | stage.rs:31-92 | Medium |
| 35 | Plugins/NSE | Ruby security docs point to wrong file | security.rs | Low |
| 36 | Plugins/NSE | FxHashMap in PluginManager | plugin/lib.rs:296-297 | Low |
| 37 | Recon | CveMapper cache doesn't persist | cve.rs:31 | High |
| 38 | Recon | FxHashMap count mismatch (55 vs actual) | recon/*.rs | Medium |
| 39 | Scanner | Endpoint wordlist count mismatch (224 vs 223) | endpoints.rs:35 | Low |
| 40 | WAF | Magic number 256 in header check | detect.rs:81 | Low |

## Execution

Launch subagents in waves, one wave at a time. Each subagent works on its assigned branch.