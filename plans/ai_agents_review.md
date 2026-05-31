# AI & Agents Module Architecture Review

**Document:** architecture/ai_agents.md
**Reviewed:** 2026-05-31
**Accuracy:** Medium

## Verified Claims

- **Provider enum variants (OpenAI, Azure, Anthropic, OpenAICompatible)**: Verified at `crates/slapper/src/ai/client.rs:8-14`
- **AiClient struct fields (client, config, circuit_breaker, provider)**: Verified at `crates/slapper/src/ai/client.rs:54-60`
- **AiClient methods: `chat_completion_from_messages()`, `analyze_findings()`, `analyze_findings_typed()`, `suggest_payloads()`, `suggest_waf_bypass()`**: Verified at `crates/slapper/src/ai/client.rs:167,329,347,386,401`
- **AdaptiveScanEngine struct (client, strategy, ai_suggested_strategy)**: Verified at `crates/slapper/src/ai/adaptive.rs:5-9`
- **AdaptiveScanEngine strategies: deep, thorough, quick, stealth, standard**: Verified at `crates/slapper/src/ai/adaptive.rs:57-69`
- **Fallback to severity-based heuristics**: Verified at `crates/slapper/src/ai/adaptive.rs:72-83`
- **AiPayloadGenerator struct (client, cache)**: Verified at `crates/slapper/src/ai/payloads.rs:8-11`
- **AiPayloadGenerator LRU cache: 100 entries, 1hr TTL**: Verified at `crates/slapper/src/ai/payloads.rs:17`
- **CacheKeyBuilder for collision-free cache keys**: Verified at `crates/slapper/src/ai/cache.rs:323-353`
- **SmartWafBypass struct fields**: Verified at `crates/slapper/src/ai/waf_bypass.rs:23-30`
- **Knowledge base persists to `waf_bypasses.json`, max 1000 entries**: Verified at `crates/slapper/src/ai/waf_bypass.rs:38-40,66`
- **Tracks success/failure per (WAF, payload) pair**: Verified at `crates/slapper/src/ai/waf_bypass.rs:184-242`
- **`iterative_bypass()` for multi-iteration refinement**: Verified at `crates/slapper/src/ai/waf_bypass.rs:153`
- **AiCache with RwLock, CacheEntry, with_persistence()**: Verified at `crates/slapper/src/ai/cache.rs:72-184`
- **AiPlanner::create_plan()**: Verified at `crates/slapper/src/ai/planner.rs:82`
- **AiPlanner::suggest_adjustments()**: Verified at `crates/slapper/src/ai/planner.rs:218`
- **AiPlanner::record_outcome()**: Verified at `crates/slapper/src/ai/planner.rs:451`
- **Learning cache with success rate tracking**: Verified at `crates/slapper/src/ai/planner.rs:50-60,112`
- **ScriptGenerator methods**: Verified at `crates/slapper/src/ai/script_gen.rs:72,123,174`
- **Scripts saved to `generated_scripts/` directory**: Verified at `crates/slapper/src/ai/script_gen.rs:62-64`
- **planner.rs and script_gen.rs feature-gated `ai-integration`**: Verified at `crates/slapper/src/ai/mod.rs:6-9`
- **FxHashMap usage in cache.rs**: Verified at `crates/slapper/src/ai/cache.rs:73`
- **FxHashMap in planner.rs (learning_cache, PlanOutcome.severity_distribution)**: Verified at `crates/slapper/src/ai/planner.rs:42,50`
- **`evict_knowledge_base_if_needed()`**: Verified at `crates/slapper/src/ai/waf_bypass.rs:82-99`
- **SmartWafBypass Clone implementation**: Verified at `crates/slapper/src/ai/waf_bypass.rs:258-268`
- **McpProfile enum (OpsAgent, CodingAgent)**: Verified at `crates/slapper/src/tool/protocol/mcp/profile.rs:5-8`
- **AiError enum variants**: Verified at `crates/slapper/src/ai/errors.rs:6-33`
- **Agent module files: mod.rs, memory.rs, portfolio.rs, constraints.rs, skills.rs**: Verified at `crates/slapper/src/agent/`

## Discrepancies

- **`chat_completion()` listed as public method**: Documented as a method of AiClient but it is private (`async fn chat_completion` at `crates/slapper/src/ai/client.rs:151`). The public method is `chat_completion_from_messages()`.
- **McpProfilePolicy struct underspecified**: The document shows only 7 fields (`profile`, `target_policy`, `max_concurrency`, `max_timeout_ms`, `max_batch_size`, `allow_external_network`, `allow_stress_testing`, `allow_broad_recon`). The actual struct has 18 fields including `default_target_policy`, `allowed_tool_ids`, `denied_tool_ids`, `allowed_categories`, `denied_categories`, `allow_streaming`, `allow_sessions`, `allow_plan_endpoint`, `require_explicit_scope`, `allow_packet_features`, `denied_argument_keys` (`crates/slapper/src/tool/protocol/mcp/policy.rs:64-84`).
- **`target_policy` field name wrong**: Documented as `target_policy` but actual field is `default_target_policy` (`crates/slapper/src/tool/protocol/mcp/policy.rs:66`).
- **`TargetPolicy::None` doesn't exist**: Documented as `TargetPolicy::None` for ops_agent, but actual value is `TargetPolicy::AnyWithScopeEngine` (`crates/slapper/src/tool/protocol/mcp/policy.rs:99`). There is no `None` variant in the `TargetPolicy` enum.
- **ops_agent() max_concurrency wrong**: Documented as 20, actual is 50 (`crates/slapper/src/tool/protocol/mcp/policy.rs:104`).
- **ops_agent() max_timeout_ms wrong**: Documented as 300,000ms, actual is 600,000ms (`crates/slapper/src/tool/protocol/mcp/policy.rs:105`).
- **coding_agent() missing fields in doc**: The actual coding_agent() also sets `allowed_tool_ids: ToolSelector::Exact(vec!["scan", "scan-ports", "fingerprint", "endpoints", "waf-detect", "search"])`, `denied_categories: ToolSelector::Exact(vec!["stresstesting", "loadtesting"])`, `allow_streaming: true`, `allow_sessions: false`, `allow_plan_endpoint: false`, `require_explicit_scope: false`, `allow_packet_features: false`, and `denied_argument_keys: vec!["stealth", "proxy_rotation", "spoof_source", "raw_packet"]` (`crates/slapper/src/tool/protocol/mcp/policy.rs:120-156`). None of these appear in the document.
- **Bug fix line reference waf_bypass.rs:107 wrong**: The doc says "waf_bypass.rs:107 - Added `continue` after `failed_attempts >= 3` check". Line 107 is actually `return Err(AiError::invalid_config("waf name cannot be empty"));`. The `continue` after `failed_attempts >= 3` is at line 133 (`crates/slapper/src/ai/waf_bypass.rs:124-133`).
- **Agent bug fix claim incorrect**: Doc says "alerts/routing.rs:81 - Removed `expect()` panic on fallback HTTP client creation". The `expect()` is still present at line 79: `.expect("Failed to create fallback HTTP client")` (`crates/slapper/src/agent/alerts/routing.rs:79`). This claim is stale/incorrect.
- **Agent module memory.rs:137 line reference**: Doc says "memory.rs:137 - Added fallback hash-based name when `file_stem()` returns None". This is verified at lines 137-146, but the line number should reference the full block (137-146), not just line 137.

## Bugs Found

- **Stale bug fix claim for alerts/routing.rs:81**: The document claims `expect()` was removed from fallback HTTP client creation, but `expect("Failed to create fallback HTTP client")` is still present at `crates/slapper/src/agent/alerts/routing.rs:79`. Either the fix was never applied or was reverted. (priority: high)
- **Incorrect line number for waf_bypass.rs fix**: Line 107 referenced in doc does not correspond to the `failed_attempts >= 3` check. The actual line is 124-133. (priority: medium)

## Improvement Opportunities

- **Document McpProfilePolicy fully**: The actual struct has 18 fields. The document should include all fields, especially `denied_argument_keys` and the tool selector fields, which are critical for understanding policy enforcement. (priority: high)
- **Document TargetPolicy enum variants**: The doc references `TargetPolicy::None` which doesn't exist. All 4 variants (`ExplicitScopeOnly`, `LocalhostAndPrivateCidrsOnly`, `ScopeOrLocalDevOnly`, `AnyWithScopeEngine`) should be documented. (priority: high)
- **Add MCP enforcement point for argument key validation**: The enforcement table mentions argument validation but doesn't mention `denied_argument_keys` filtering. (priority: medium)
- **Document McpProfilePolicy::for_profile() method**: This is a convenience constructor that dispatches to ops_agent()/coding_agent() based on profile (`crates/slapper/src/tool/protocol/mcp/policy.rs:88-93`). (priority: low)
- **Document PolicyViolation enum**: The error type for policy violations is not mentioned in the architecture doc (`crates/slapper/src/tool/protocol/mcp/policy.rs:332-339`). (priority: low)

## Stale Items

- **"waf_bypass.rs:107" bug fix reference**: Line number is wrong (should be ~124-133). Update or remove. (Recommended action: correct line numbers)
- **"alerts/routing.rs:81 - Removed `expect()` panic"**: This claim is incorrect - the expect() is still present. Remove this item or verify the actual state. (Recommended action: remove or correct)
- **"handlers/mod.rs:155-169" in Agent section**: This line reference does not appear in the AI agents doc but the same incorrect reference exists in cli_commands.md. Verify relevance. (Recommended action: N/A - not in this doc)
