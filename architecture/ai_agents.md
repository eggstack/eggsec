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

1. Fixed WAF bypass knowledge base lookup logic for failed entries
2. Fixed cache lock handling to prevent race conditions during persist
3. Lowered planner cache thresholds for better hit rate
4. Added knowledge base eviction to prevent unbounded growth
5. Fixed Clone implementation for SmartWafBypass

See `crates/slapper/src/ai/AGENTS.override.md` for detailed patterns.
