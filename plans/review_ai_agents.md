# AI Agents Architecture Review
**Document:** architecture/ai_agents.md
**Reviewed:** 2026-05-31
**Accuracy:** High
**Lines Reviewed:** 219

## Verified Claims
- Provider enum variants (OpenAI, Azure, Anthropic, OpenAICompatible): Verified at `ai/client.rs:8-14`
- AiClient struct fields (client, config, circuit_breaker, provider): Verified at `ai/client.rs:54-60`
- Circuit breaker initialization (5 failures, 3 half-open, 60s timeout): Verified at `ai/client.rs:73`
- AiPayloadGenerator struct: Verified at `ai/payloads.rs:8-11`
- Cache LRU with 100 entries and 1hr TTL: Verified at `ai/payloads.rs:17`
- CacheKeyBuilder for collision-free keys: Verified at `ai/cache.rs:323-352`
- AiError enum variants: Verified at `ai/errors.rs:6-33`
- McpProfile enum (OpsAgent, CodingAgent): Verified at `tool/protocol/mcp/profile.rs:3-8`
- McpProfilePolicy struct fields: Verified at `tool/protocol/mcp/policy.rs:63-84`
- ops_agent() policy values (concurrency 50, timeout 600s, batch 100): Verified at `tool/protocol/mcp/policy.rs:96-117`
- coding_agent() policy values (concurrency 5, timeout 60s, batch 10): Verified at `tool/protocol/mcp/policy.rs:120-156`
- SmartWafBypass struct and knowledge base: Verified at `ai/waf_bypass.rs:23-30`
- Knowledge base max 1000 entries: Verified at `ai/waf_bypass.rs:66`
- AdaptiveScanEngine strategies (deep, thorough, quick, stealth, standard): Verified at `ai/adaptive.rs:57-69`
- AiPlanner with learning cache: Verified at `ai/planner.rs:47-52`
- ScriptGenerator with Python script generation: Verified at `ai/script_gen.rs:55-70`
- Cache uses FxHashMap: Verified at `ai/cache.rs:73`
- WAF bypass entries track success/failure per (WAF, payload) pair: Verified at `ai/waf_bypass.rs:11-21`
- iterative_bypass() for multi-iteration refinement: Verified at `ai/waf_bypass.rs:153-182`
- evict_knowledge_base_if_needed() prevents unbounded growth: Verified at `ai/waf_bypass.rs:82-99`
- Policy enforcement points (tool filtering, argument validation, target validation, concurrency clamping, timeout clamping): Verified at `tool/protocol/mcp/policy.rs:158-263`

## Discrepancies
- Document states "AdaptiveScanEngine::adjust_strategy() analyzes findings and returns strategy" - The actual method returns `Result<&str>` not just a strategy string (`ai/adaptive.rs:20`)
- Document shows AiError with "InvalidConfig(String)" variant - Actual uses `InvalidConfig(String)` but has helper method `invalid_config()` (`ai/errors.rs:56-58`)
- Document states coding_agent() allowed_tool_ids includes "search" - Verified correct at `tool/protocol/mcp/policy.rs:130`
- Document states coding_agent() denied_categories includes "stresstesting" and "loadtesting" - Verified correct at `tool/protocol/mcp/policy.rs:134-137`

## Bugs Found
- No bugs found in the architecture documentation. All claims are accurate.

## Improvement Opportunities
- [Item]: Consider documenting the `into_payload_generator()` method on AiClient (`ai/payloads.rs:63-65`) for completeness (priority: low)
- [Item]: Document the `PluginLanguage` enum variants (Python, Ruby, Rust) in script_gen.rs (`ai/script_gen.rs:7-11`) (priority: low)
- [Item]: Add note about Anthropic format transformation in client.rs (`ai/client.rs:229-280`) (priority: low)

## Stale Items
- [Item]: "Recent Bug Fixes (2026-05-22)" section contains specific line references - These may become stale as code evolves. Consider converting to a general changelog reference or removing specific line numbers (priority: medium)
