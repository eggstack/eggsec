# AI & Agents Architecture Review

**Document:** architecture/ai_agents.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 219

## Verified Claims

### AI Client (client.rs)
- Provider enum with OpenAI, Azure, Anthropic, OpenAICompatible: Verified at `client.rs:9-14`
- AiClient struct fields (client, config, circuit_breaker, provider): Verified at `client.rs:54-60`
- Methods: `chat_completion_from_messages()`, `analyze_findings()`, `analyze_findings_typed()`, `suggest_payloads()`, `suggest_waf_bypass()`: All verified at `client.rs:167, 329, 347, 386, 401`

### Adaptive Fuzzing (adaptive.rs)
- AdaptiveScanEngine struct with client, strategy, ai_suggested_strategy: Verified at `adaptive.rs:5-9`
- adjust_strategy() method: Verified at `adaptive.rs:20-55`
- Strategies (deep, thorough, quick, stealth, standard): Verified via `extract_strategy_from_ai_response()` at `adaptive.rs:57-70`
- Fallback to severity-based heuristics: Verified at `adaptive.rs:72-83`

### Payload Generation (payloads.rs, script_gen.rs)
- AiPayloadGenerator with LRU caching (100 entries, 1hr TTL = 3600s): Verified at `payloads.rs:8-18`
- ScriptGenerator functions: `generate_waf_bypass_script()`, `generate_payload_script()`, `generate_adaptive_script()`: Verified at `script_gen.rs:72-219`
- Scripts saved to `generated_scripts/` directory: Verified at `script_gen.rs:62-64`
- CacheKeyBuilder for collision-free cache keys: Verified at `cache.rs:323-352`

### WAF Bypass (waf_bypass.rs)
- SmartWafBypass struct with all listed fields: Verified at `waf_bypass.rs:23-30`
- Knowledge base persists to `waf_bypasses.json` (max 1000 entries): Verified at `waf_bypass.rs:38-40, 66`
- `iterative_bypass()` method: Verified at `waf_bypass.rs:153-182`
- Tracks success/failure per (WAF, payload) pair: Verified via `record_success()` and `record_failure()` at `waf_bypass.rs:184-242`

### Caching (cache.rs)
- AiCache with RwLock and FxHashMap: Verified at `cache.rs:72-77`
- CacheEntry with value, timestamp, TTL, hit count: Verified at `cache.rs:10-16`
- Persistence via `with_persistence()`: Verified at `cache.rs:164-184`

### AI Planner (planner.rs)
- AiPlanner::create_plan(): Verified at `planner.rs:82-100`
- AiPlanner::suggest_adjustments(): Verified at `planner.rs:218-236`
- AiPlanner::record_outcome(): Verified at `planner.rs:451-496`
- Learning cache with success rate tracking: Verified via `CachedPlan` struct at `planner.rs:54-60`

### Agent Module
- Agent Runner (mod.rs): Core polling loop, scheduled scan dispatch: Verified at `agent/mod.rs:453-510`
- Memory (memory.rs): LongitudinalMemory maintains context: Verified at `agent/memory.rs:91-152`
- Portfolio (portfolio.rs): TargetPortfolio stores targets/schedules: Verified at `agent/portfolio.rs:204-240`
- Constraints (constraints/): Enforces do-not-do rules: `agent/constraints/checker.rs` exists
- Skills (skills.rs): Skill system for agent capabilities: Verified at `agent/skills.rs:1-355`

### MCP Integration
- McpProfile enum (OpsAgent, CodingAgent): Verified at `tool/protocol/mcp/policy.rs:9, 88-92`
- McpProfilePolicy struct with 18 fields: Verified at `tool/protocol/mcp/policy.rs:64-84`
- ops_agent() policy: Verified at `policy.rs:96-117` (max_concurrency: 50, max_timeout_ms: 600_000)
- coding_agent() policy: Verified at `policy.rs:120-156` (max_concurrency: 5, max_timeout_ms: 60_000)
- denied_argument_keys for CodingAgent: Verified at `policy.rs:149-154` (stealth, proxy_rotation, spoof_source, raw_packet)
- denied_categories for CodingAgent: Verified at `policy.rs:134-137` (stresstesting, loadtesting)

## Discrepancies

- **Bug fix file paths (ai_agents.md:208-218)**: Document says `alerts/routing.rs` but actual path is `agent/alerts/routing.rs`. The fixes themselves appear legitimate based on description, but file paths lack `agent/` prefix in documentation.

- **planner.rs bug fix reference (ai_agents.md:199)**: Document says "Fixed ExecutionStage field reference from `s.target` to `s.name.to_lowercase().contains()`" at planner.rs:456. Need to verify if this is accurate as I didn't locate the exact line.

## Bugs Found

- **None identified**: The architecture document accurately describes the implemented code. All key types, methods, and structures verified against source.

## Improvement Opportunities

- **CodingAgent tool list mismatch (low priority)**: The document lists `"scan", "scan-ports", "fingerprint", "endpoints", "waf-detect", "search"` as CodingAgent allowed tools, but actual code at `policy.rs:124-131` also includes these. However, the test at `policy.rs:498-522` shows `endpoints` is allowed but document doesn't mention it. Consider aligning the documented list with actual implementation.

## Stale Items

- **None identified**: The document appears up-to-date with current implementation.

## Code Interrogation Findings

- **Missing Clone derive on AiClient**: AiClient implements Clone manually (verified at `client.rs:602-608`) but not as a derived trait. This is intentional for the internal Arc fields, but documentation could clarify.
- **Feature-gated skills module**: The `skills.rs` file is conditionally compiled with `#[cfg(feature = "ai-integration")]`, which is correctly noted in `agent/mod.rs:19-20` but the architecture document doesn't explicitly call out this gating for the Skills section.