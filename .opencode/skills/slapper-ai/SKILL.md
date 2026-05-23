# Slapper AI Skill

AI/LLM integration module workflows and patterns for autonomous security testing.

## Key Components

| File | Purpose |
|------|---------|
| `client.rs` | AI client for LLM providers (OpenAI, Azure, Anthropic) |
| `adaptive.rs` | Adaptive scanning engine |
| `payloads.rs` | AI-powered payload generator with caching |
| `waf_bypass.rs` | Smart WAF bypass suggestion system with knowledge base |
| `planner.rs` | AI-driven execution planning (feature-gated `ai-integration`) |
| `script_gen.rs` | Generates Python security scripts (feature-gated `ai-integration`) |
| `cache.rs` | TTL cache for AI responses with persistence |
| `types.rs` | Core types (`AiAnalysisResult`, `ScanFinding`, etc.) |
| `errors.rs` | `AiError` enum with 10 variants |

## Key Types

### Providers
```rust
pub enum Provider { OpenAI, Azure, Anthropic, OpenAICompatible }
```

### Cache
- `AiCache` - TTL-based cache with optional disk persistence
- `CacheKeyBuilder` - Builder for consistent cache keys
- `CacheStats` - Statistics for cache hits/entries

### AI Client
- `AiClient::new(config)` - Creates client with circuit breaker
- `chat_completion()`, `analyze_findings()`, `suggest_payloads()`, `suggest_waf_bypass()`

### Smart WAF Bypass
- `SmartWafBypass::new(client)` - Creates with default max 1000 knowledge base entries
- `find_bypass()`, `iterative_bypass()`, `record_success()`, `record_failure()`
- Knowledge base persists to `waf_bypasses.json`

### Adaptive Scan Engine
- `AdaptiveScanEngine::new(client)` - Creates with optional AI client
- `adjust_strategy()` - Returns strategy: deep, thorough, quick, stealth, standard

## Circuit Breaker

`utils/circuit_breaker.rs` provides `CircuitBreaker`:
- Individual breaker with state (Closed/Open/HalfOpen)
- Tracks failure/success counts, total calls, failure rate
- Exposes `total_calls()`, `total_failures()`, `failure_rate()` methods

Each AI client creates its own breaker via `CircuitBreaker::new(5, 3, 60s)`:
- 5 failures to open
- 3 successes in half-open to close
- 60s timeout

## Testing

### Running AI Tests
```bash
cargo test --lib -p slapper ai::
```

### Writing Tests
Follow existing test patterns in `ai/` modules, testing circuit breaker integration and LLM client logic.

## Common Tasks

### Adding a New AI Client
1. Implement client logic in `ai/`
2. Create dedicated `CircuitBreaker` via `CircuitBreaker::new()`
3. Track failure/success counts for circuit breaker state
4. Add tests for new AI client

### Using Cache Properly
Use `CacheKeyBuilder` for cache keys to avoid collisions:
```rust
let cache_key = CacheKeyBuilder::for_payload_suggestion(vuln_type, context);
let cache_key = CacheKeyBuilder::for_waf_bypass(waf, blocked_payload);
```

### Adding to Knowledge Base
Before adding to `SmartWafBypass.knowledge_base`, call eviction:
```rust
self.evict_knowledge_base_if_needed();
self.knowledge_base.push(entry);
```

## Bug Fixes (2026-05-22)

### waf_bypass.rs:find_bypass loop
Added `continue` after `failed_attempts >= 3` check to properly skip non-matching entries instead of falling through to AI query for remaining entries.

### planner.rs:record_outcome
Fixed `ExecutionStage` field reference - changed `s.target.contains()` to `s.name.to_lowercase().contains()` since `ExecutionStage` has `name`, not `target`.

### planner.rs:cache.rs Clock Skew Fix (2026-05-23)
Fixed clock skew panic prevention in `AiPlanner` - changed `SystemTime::now().duration_since(UNIX_EPOCH).unwrap()` to `unwrap_or_else(|_| Duration::from_secs(0))` at 3 locations (lines 206-209, 467-470, 480-483). Prevents panic if system clock moves backwards (NTP correction).

### cache.rs Eviction Loop Fix (2026-05-23)
Changed cache eviction from single-pass `if` to `while` loop to remove ALL excess entries when over capacity.

### waf_bypass.rs Persist Error Logging (2026-05-23)
Added error logging to `SmartWafBypass::persist()` - previously file operation failures were silently ignored with `let _ = ...`. Now uses `tracing::warn` for failures.

### cache.rs and planner.rs Performance Fixes
Changed `std::collections::HashMap` to `rustc_hash::FxHashMap` for:
- `AiCache.entries` - Thread-safe async cache storage
- `AiPlanner.learning_cache` - Learning cache for plan outcomes
- `PlanOutcome.severity_distribution` - Severity distribution tracking

## Agent Module FxHashMap Migration (2026-05-22)

The agent module also migrated to FxHashMap for performance:
- `AlertRouter.recent_alerts`, `ChannelRegistry.channels`
- `AlertRouter::aggregate_findings` severity_counts and targets
- `AlertRouter::generate_recommendations` vuln_types
- `WebhookConfig.headers`, `AggregatedAlert.severity_counts`
- `SlackTemplate.color_by_severity`, `PagerDutyTemplate.severity_mapping`
- `ScanCompleteEvent.severity_counts`
- `LongitudinalMemory` internal collections
- `PortfolioSnapshot.findings_by_severity`, `TemporalAnalysis.findings_by_day`

## Resources
- `crates/slapper/src/ai/AGENTS.override.md` - Detailed AI patterns and bug fixes
- `AGENTS.md` - General project guidelines
- `ARCHITECTURE.md` - Overall design
- `architecture/ai_agents.md` - AI module architecture
