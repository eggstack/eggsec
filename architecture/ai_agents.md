# AI & Agents Module

Eggsec features deep integration with AI models for analysis, payload generation, and agent-readable security orchestration via the Model Context Protocol (MCP).

## AI Integration (`src/ai/`)

### AI Client (`client.rs`)

An abstraction layer for interacting with different LLM providers:
- **Providers**: OpenAI, Azure, Anthropic, OpenAICompatible
- **Features**: Bearer/Azure auth, circuit breaker, response normalization
- **Methods**: `chat_completion_from_messages()`, `analyze_findings()`, `analyze_findings_typed()`, `suggest_payloads()`, `suggest_waf_bypass()`, `into_payload_generator()`, `circuit_breaker_state()`
- **Note**: `chat_completion()` is private — use `chat_completion_from_messages()` instead
- **AiConfig fields**: `provider`, `model`, `api_key`, `base_url`, `max_tokens`, `temperature`, `max_payloads`, `max_bypasses`
- **Anthropic normalization**: Anthropic responses are normalized to OpenAI format, with `usage` data preserved at the top level and original response under `provider_response`

### Adaptive Fuzzing (`adaptive.rs`)

Using AI to analyze target responses and adjust fuzzing strategies in real-time:
- `AdaptiveScanEngine::adjust_strategy()` analyzes findings and returns strategy
- `AdaptiveScanEngine::get_strategy()` returns current strategy string
- `AdaptiveScanEngine::get_ai_suggestion()` returns AI-suggested strategy if available
- `AdaptiveScanEngine::fallback_to_standard()` resets to standard strategy
- Strategies: deep, thorough, quick, stealth, standard
- Falls back to severity-based heuristics when AI unavailable

### Payload Generation (`payloads.rs`, `script_gen.rs`)

Generating complex, context-aware payloads:
- `AiPayloadGenerator` - Generates payloads with LRU caching (100 entries, 1hr TTL)
- `ScriptGenerator` - Generates Python security testing scripts (feature-gated)
- `ScriptGenerator::save_script()` saves generated script to disk with metadata header
- `PluginLanguage` enum: `Python`, `Ruby`, `Rust` (only Python is currently implemented)
- Uses `CacheKeyBuilder` for collision-free cache keys

### WAF Bypass Suggestions (`waf_bypass.rs`)

The AI can analyze detected WAF signatures and suggest novel bypass techniques:
- `SmartWafBypass` maintains knowledge base of known bypasses
- `SmartWafBypass::with_config(client, max_bypasses)` configurable constructor
- Knowledge base persists to `waf_bypasses.json` (max 1000 entries)
- Tracks success/failure per (WAF, payload) pair
- `SmartWafBypass::record_success()` records successful bypass in knowledge base
- `SmartWafBypass::record_failure()` records failed bypass attempt
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
- **Note**: `record_outcome()` uses a heuristic to match plans — plans with the same `total_tools` count and target substring in any stage name are considered equivalent

### Script Generation (`script_gen.rs`) - Feature-gated `ai-integration`

Generates Python security testing scripts:
- `generate_waf_bypass_script()`, `generate_payload_script()`, `generate_adaptive_script()`
- Scripts saved to `{config_dir}/generated_scripts/` with naming convention `script_{vuln_type}_{timestamp}.py`
- Includes proper headers and metadata
- **PluginLanguage** enum: `Python`, `Ruby`, `Rust` (only Python is currently implemented)

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

pub struct AiAnalysisResult { reassessed_severity, exploitability, impact, remediation, confidence }
pub struct AiPayloadSuggestion { payload, description, expected_result }
pub struct AiWafBypassSuggestion { technique, payload, explanation }
pub struct ScanFinding { id, title, severity, description }

pub struct CacheStats { total_entries, expired_entries, total_hits }

pub enum AiError {
    RequestFailed(String), MissingApiKey, InvalidConfig(String), ApiError(String),
    ParseError(String), Timeout, RateLimited, InvalidResponse, CircuitBreakerOpen
}

pub enum ScriptTarget {
    WafBypass { waf_name, blocked_payload },
    PayloadGeneration { vuln_type, context },
    AdaptiveScript { findings },
}
```

## Agent Orchestration (`src/agent/`)

Eggsec can run as an agent-readable scanning orchestrator that executes configured schedules, enforces operational constraints, and handles alert routing.

### Agent Runtime Types

```rust
pub struct AgentRuntimeStatus {
    pub running: bool,
    pub started_at: Option<DateTime<Utc>>,
    pub last_tick_at: Option<DateTime<Utc>>,
    pub next_tick_at: Option<DateTime<Utc>>,
    pub portfolio_targets_total: usize,
    pub portfolio_targets_enabled: usize,
    pub last_scan_started_at: Option<DateTime<Utc>>,
    pub last_scan_completed_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub scans_completed: u64,
    pub scans_failed: u64,
    pub alerts_sent: u64,
    pub last_preflight_denial: Option<AgentPreflightDenial>,
    pub recent_denial_count: usize,
}

pub struct AgentRuntimePersisted {
    pub started_at, last_tick_at, last_scan_started_at, last_scan_completed_at,
    pub scans_completed, scans_failed, alerts_sent, last_error,
    pub last_shutdown_at, last_preflight_denial, recent_denial_count,
}

pub struct AgentPreflightDenial {
    pub operation: String,
    pub target: String,
    pub timestamp: DateTime<Utc>,
    pub denied_reasons: Vec<String>,
}
```

- **Agent Runner (`mod.rs`)**: Core polling loop, scheduled scan dispatch, and event handling. `Agent::run_once()` executes a single pass; `Agent::record_policy_denial()` / `Agent::recent_policy_denials()` track enforcement denials.
- **Enforcement (`enforcement.rs`)**: Factored helper functions for per-scan enforcement — maps scan depth and scan type to `OperationRisk` and `Capability` lists (`risk_for_agent_scan_depth`, `capabilities_for_agent_scan`, `operation_descriptor_for_agent_scan`). Called immediately before dispatch in `execute_scan_with_depth` to re-evaluate enforcement per-scan.
- **Memory (`memory.rs`)**: Maintains longitudinal context and baseline-aware finding comparisons.
- **Portfolio (`portfolio.rs`)**: Stores targets, schedules, and scan history metadata.
- **Constraints (`constraints/`)**: Enforces do-not-do rules, target restrictions, and scan/rate limits. `ConstraintChecker` methods: `evaluate_action()`, `evaluate_target()`, `evaluate_scan_depth()`, `evaluate_rate_limit()`, `evaluate_payload()`, `evaluate_off_peak()`, `evaluate_approval()`, `evaluate_all()`.
- **Skills (`skills.rs`)**: Represents discrete capabilities the agent can employ (e.g., "scan", "fuzz", "recon").
- **Config Watcher (`config_watcher.rs`)**: Hot-reloading of agent configuration via `ConfigWatcher`.
- **Logging**: Centralized in `logging/init.rs`. Agent mode composes a rolling JSON file layer alongside console output.
- **Alerts (`alerts/`)**: Alert routing, aggregation, and channel delivery (Slack, PagerDuty, email, webhook).
- **Events (`events.rs`)**: Event handler trait and security event types.

## MCP Integration

Eggsec implements the **Model Context Protocol (MCP)**, allowing it to be used as a "tool" by other AI agents or integrated into larger AI-driven security platforms.

### Profile-Based Policy Enforcement

The MCP server uses profiles to control tool availability, safety policies, and output schemas.

> For MCP and autonomous-agent execution, `EnforcementContext::evaluate()` is the mandatory pre-dispatch gate. Scope provenance must come from `LoadedScope`; raw `Scope` is not sufficient for automated execution. Agent execution defensively rebuilds `AgentStrict` in the handler and validates it at runtime. Baseline strict-automated capabilities are `PassiveFingerprint`, `ActiveProbe`, `Crawl`, `WafDetect`; non-baseline require explicit `allowed_capabilities`. Manual permissive can downgrade only safe scope-selection misses; explicit exclusions, feature gates, risk gates, and capability denials remain hard denials.

```rust
pub enum McpProfile {
    OpsAgent,    // Full access, no restrictions
    CodingAgent, // Bounded tools, enforced safety
}

pub struct McpProfilePolicy {
    pub profile: McpProfile,
    pub default_target_policy: TargetPolicy,
    pub allowed_tool_ids: ToolSelector,
    pub denied_tool_ids: ToolSelector,
    pub allowed_categories: ToolSelector,
    pub denied_categories: ToolSelector,
    pub max_concurrency: usize,
    pub max_timeout_ms: u64,
    pub max_batch_size: usize,
    pub allow_streaming: bool,
    pub allow_sessions: bool,
    pub allow_plan_endpoint: bool,
    pub require_explicit_scope: bool,
    pub allow_external_network: bool,
    pub allow_stress_testing: bool,
    pub allow_packet_features: bool,
    pub allow_broad_recon: bool,
    pub denied_argument_keys: Vec<String>,
}
```

**Policy enforcement points:**

| Enforcement | Location | Description |
|-------------|----------|-------------|
| Tool filtering | `tools/list` | Only tools allowed by profile are returned |
| Argument validation | `tool/execute` | Denied arguments are rejected before execution |
| Target validation | `tool/execute` | Target must match policy's `TargetPolicy` |
| Concurrency clamping | `tool/execute` | Requested concurrency is clamped to policy max |
| Timeout clamping | `tool/execute` | Requested timeout is clamped to policy max |

**Profile policy definitions:**

```rust
impl McpProfilePolicy {
    pub fn ops_agent() -> Self {
        // No restrictions: all tools, broad concurrency/timeout caps
        Self {
            profile: McpProfile::OpsAgent,
            default_target_policy: TargetPolicy::AnyWithScopeEngine,
            allowed_tool_ids: ToolSelector::All,
            denied_tool_ids: ToolSelector::None,
            allowed_categories: ToolSelector::All,
            denied_categories: ToolSelector::None,
            max_concurrency: 50,
            max_timeout_ms: 600_000,
            max_batch_size: 100,
            allow_streaming: true,
            allow_sessions: true,
            allow_plan_endpoint: true,
            require_explicit_scope: true,
            allow_external_network: true,
            allow_stress_testing: true,
            allow_packet_features: true,
            allow_broad_recon: true,
            denied_argument_keys: Vec::new(),
        }
    }

    pub fn coding_agent() -> Self {
        // Restricted: localhost/private only, narrow tools, tight caps
        Self {
            profile: McpProfile::CodingAgent,
            default_target_policy: TargetPolicy::ScopeOrLocalDevOnly,
            allowed_tool_ids: ToolSelector::Exact(vec![
                "scan", "scan-ports", "fingerprint", "endpoints", "waf-detect", "search",
            ]),
            denied_tool_ids: ToolSelector::None,
            allowed_categories: ToolSelector::None,
            denied_categories: ToolSelector::Exact(vec!["stresstesting", "loadtesting"]),
            max_concurrency: 5,
            max_timeout_ms: 60_000,
            max_batch_size: 10,
            allow_streaming: true,
            allow_sessions: false,
            allow_plan_endpoint: false,
            require_explicit_scope: true,
            allow_external_network: false,
            allow_stress_testing: false,
            allow_packet_features: false,
            allow_broad_recon: false,
            denied_argument_keys: vec![
                "stealth", "proxy_rotation", "spoof_source", "raw_packet",
            ],
        }
    }
}
```

### Coding Agent Output Schema (`coding_agent_output.rs`)

Typed output schema for the coding-agent profile:

- `CodingAgentFindingReport` - Top-level report with schema version, target, findings, and summary
- `CodingAgentFinding` - Individual finding with severity, confidence, evidence, and patch relevance
- `CodingAgentEvidence` - Evidence snippet (raw exploit payloads stripped by default)
- `CodingAgentSummary` - Aggregated counts by severity

**Patch relevance mapping**: Critical/High → `blocks_merge`, Medium → `should_fix`, Low → `review_manually`

## Recent Bug Fixes (2026-05-22)

### AI Module
1. **waf_bypass.rs:124-133** - Added `continue` after `failed_attempts >= 3` check to prevent incorrect fallthrough to AI query
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

### MCP Module
1. **policy.rs** - Fixed CGNAT check dead code: replaced `&& false` with proper 100.64.0.0/10 range check via `is_cgnat()`
2. **cache.rs** - Replaced `blocking_read()` in `From<&AiCache>` with `try_read()` to prevent tokio runtime panics

### WAF Bypass
1. **waf_bypass.rs** - Fixed eviction order to evict failed/stale entries first instead of useful entries

See `crates/eggsec/src/ai/AGENTS.override.md` for detailed AI patterns and `crates/eggsec/src/agent/AGENTS.override.md` for agent patterns.
