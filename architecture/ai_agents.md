# AI & Agents Module

Slapper features deep integration with AI models for analysis, payload generation, and autonomous security testing via the Model Context Protocol (MCP).

## AI Integration (`src/ai/`)

### AI Client (`client.rs`)

An abstraction layer for interacting with different LLM providers:
- **Providers**: OpenAI, Azure, Anthropic, OpenAICompatible
- **Features**: Bearer/Azure auth, circuit breaker, response normalization
- **Methods**: `chat_completion()`, `analyze_findings()`, `analyze_findings_typed()`, `suggest_payloads()`, `suggest_waf_bypass()`

### Adaptive Fuzzing (`adaptive.rs`)

Using AI to analyze target responses and adjust fuzzing strategies in real-time:
- `AdaptiveScanEngine::adjust_strategy()` analyzes findings and returns strategy
- Strategies: deep, thorough, quick, stealth, standard
- Falls back to severity-based heuristics when AI unavailable

### Payload Generation (`payloads.rs`, `script_gen.rs`)

Generating complex, context-aware payloads:
- `AiPayloadGenerator` - Generates payloads with LRU caching (100 entries, 1hr TTL)
- `ScriptGenerator` - Generates Python security testing scripts (feature-gated)
- Uses `CacheKeyBuilder` for collision-free cache keys

### WAF Bypass Suggestions (`waf_bypass.rs`)

The AI can analyze detected WAF signatures and suggest novel bypass techniques:
- `SmartWafBypass` maintains knowledge base of known bypasses
- Knowledge base persists to `waf_bypasses.json` (max 1000 entries)
- Tracks success/failure per (WAF, payload) pair
- `iterative_bypass()` for multi-iteration refinement

### Caching (`cache.rs`)

TTL-based caching with optional disk persistence:
- `AiCache` - Thread-safe async cache with RwLock
- `CacheEntry` - Value, timestamp, TTL, hit count
- `CacheKeyBuilder` - Builder for consistent key formation
- Persists to configurable path via `with_persistence()`

### AI Planner (`planner.rs`) - Feature-gated `ai-integration`

AI-driven execution planning:
- `AiPlanner::create_plan()` - Creates execution plans with AI
- `AiPlanner::suggest_adjustments()` - Suggests plan modifications
- `AiPlanner::record_outcome()` - Learns from plan outcomes
- Learning cache with success rate tracking

### Script Generation (`script_gen.rs`) - Feature-gated `ai-integration`

Generates Python security testing scripts:
- `generate_waf_bypass_script()`, `generate_payload_script()`, `generate_adaptive_script()`
- Scripts saved to `generated_scripts/` directory
- Includes proper headers and metadata

## Key Types

```rust
pub enum Provider { OpenAI, Azure, Anthropic, OpenAICompatible }

pub struct AiClient {
    client: Client,
    config: AiConfig,
    circuit_breaker: Arc<CircuitBreaker>,
    provider: Provider,
}

pub struct AiPayloadGenerator { client: AiClient, cache: Arc<AiCache> }
pub struct SmartWafBypass { client: AiClient, cache, knowledge_base, persist_path, max_bypasses, max_knowledge_base_size }
pub struct AdaptiveScanEngine { client: Option<AiClient>, strategy, ai_suggested_strategy }

pub enum AiError {
    RequestFailed(String), MissingApiKey, InvalidConfig(String), ApiError(String),
    ParseError(String), Timeout, RateLimited, InvalidResponse, CircuitBreakerOpen
}
```

## Autonomous Agents (`src/agent/`)

Slapper can run as an autonomous scanning agent that executes configured schedules, enforces operational constraints, and handles alert routing.

- **Agent Runner (`mod.rs`)**: Core polling loop, scheduled scan dispatch, and event handling.
- **Memory (`memory.rs`)**: Maintains longitudinal context and baseline-aware finding comparisons.
- **Portfolio (`portfolio.rs`)**: Stores targets, schedules, and scan history metadata.
- **Constraints (`constraints/`)**: Enforces do-not-do rules, target restrictions, and scan/rate limits.
- **Skills (`skills.rs`)**: Represents discrete capabilities the agent can employ (e.g., "scan", "fuzz", "recon").

## MCP Integration

Slapper implements the **Model Context Protocol (MCP)**, allowing it to be used as a "tool" by other AI agents or integrated into larger AI-driven security platforms.

## Recent Bug Fixes (2026-05-22)

### AI Module
1. **waf_bypass.rs:107** - Added `continue` after `failed_attempts >= 3` check to prevent incorrect fallthrough to AI query
2. **planner.rs:456** - Fixed `ExecutionStage` field reference from `s.target` to `s.name.to_lowercase().contains()`
3. **cache lock handling** - Race condition prevention during persist (2026-05-22 earlier fix)
4. **planner cache thresholds** - Lowered from `use_count > 3` to `>= 2` for better hit rate
5. **Knowledge base eviction** - Added `evict_knowledge_base_if_needed()` to prevent unbounded growth
6. **SmartWafBypass Clone** - Fixed Clone implementation
7. **cache.rs** - Changed `HashMap` to `FxHashMap` for performance (AiCache.entries)
8. **planner.rs** - Changed `HashMap` to `FxHashMap` for performance (learning_cache, PlanOutcome.severity_distribution)

### Agent Module
1. **alerts/routing.rs:81** - Removed `expect()` panic on fallback HTTP client creation
2. **alerts/routing.rs:107-112** - Fixed race condition in `cleanup_stale_entries` by inlining cleanup under single lock scope
3. **alerts/routing.rs:117** - Fixed `dedup_key` used before assignment by moving computation before channels_to_send
4. **alerts/routing.rs** - Changed `HashMap`/`HashSet` to `FxHashMap`/`FxHashSet` for performance (ChannelRegistry.channels, recent_alerts, severity_counts, targets, vuln_types)
5. **channels.rs** - Changed `HashMap` to `FxHashMap` for performance (WebhookConfig.headers, AggregatedAlert.severity_counts, SlackTemplate.color_by_severity, PagerDutyTemplate.severity_mapping)
6. **events.rs** - Changed `ScanCompleteEvent.severity_counts` to `FxHashMap`
7. **memory.rs** - Changed `HashMap`/`HashSet` to `FxHashMap`/`FxHashSet` for performance (ScanSummary, LongitudinalMemory.target_locks, PortfolioSnapshot, TemporalAnalysis)
8. **mod.rs** - Changed test event `severity_counts` to `FxHashMap::default()`
9. **memory.rs:137** - Added fallback hash-based name when `file_stem()` returns None
10. **mod.rs:657** - Changed `unwrap_or_default()` to `unwrap_or_else()` with warning log

See `crates/slapper/src/ai/AGENTS.override.md` for detailed AI patterns and `crates/slapper/src/agent/AGENTS.override.md` for agent patterns.
