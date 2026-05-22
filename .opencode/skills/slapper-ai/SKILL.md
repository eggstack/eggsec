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

## Resources
- `crates/slapper/src/ai/AGENTS.override.md` - Detailed AI patterns and bug fixes
- `AGENTS.md` - General project guidelines
- `ARCHITECTURE.md` - Overall design
- `architecture/ai_agents.md` - AI module architecture
