# AI Module Override

Specialized guidance for the AI/LLM integration module.

## Circuit Breaker

`utils/circuit_breaker.rs` provides `CircuitBreaker`:
- Individual breaker with state (Closed/Open/HalfOpen)
- Tracks failure/success counts, total calls, failure rate
- Exposes `total_calls()`, `total_failures()`, `failure_rate()` methods

Each AI client creates its own breaker directly via `CircuitBreaker::new()`.

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

## Important Patterns

### Cache Key Building
Use `CacheKeyBuilder` for consistent cache key formation:
- `CacheKeyBuilder::for_payload_suggestion(vuln_type, context)` -> `"payload\x00{vuln_type}\x00{context}"`
- `CacheKeyBuilder::for_waf_bypass(waf, blocked_payload)` -> `"waf_bypass\x00{waf}\x00{blocked_payload}"`

Uses null byte (`\x00`) separator to prevent collisions when input contains colons.

### Lock Handling in AiCache
When modifying cache and persisting, keep lock scope minimal. Always check `persist_path.is_some()` before persisting:
```rust
let should_persist;
{
    let mut entries = self.entries.write().await;
    // ... modify entries ...
    should_persist = self.persist_path.is_some();
}
if should_persist {
    self.persist().await;
}
```

### Knowledge Base Eviction in SmartWafBypass
The knowledge base has a maximum size (default 1000). Always call `evict_knowledge_base_if_needed()` before adding new entries:
```rust
self.evict_knowledge_base_if_needed();
self.knowledge_base.push(new_entry);
```

### Planner Cache Conditions
For `AiPlanner::query_ai_for_plan`, the cache lookup requires:
- `use_count >= 2` (not `> 3`)
- `success_rate >= 0.5` (not `> 0.8`)

This ensures new plans with moderate success are also cached.

## Bug Fixes (2026-05-22)

1. **waf_bypass.rs:80-97** - Fixed logic bug where failed entries with < 3 attempts would incorrectly fall through to AI query instead of returning None
2. **cache.rs:172-197** - Fixed race condition by restructuring lock scope before async persist
3. **planner.rs:112** - Lowered cache lookup threshold from `use_count > 3` to `use_count >= 2`
4. **waf_bypass.rs** - Added `max_knowledge_base_size` field and `evict_knowledge_base_if_needed()` method

## Bug Fixes (2026-05-29)

1. ~~**CacheKeyBuilder colon separator**~~ - FIXED: Changed to `\x00` separator
2. ~~**Three AI Agents files HashMap**~~ - VERIFIED: All use FxHashMap (false positive)

## Known Issues

1. **SmartWafBypass knowledge base eviction** (`waf_bypass.rs:80-88`) - Eviction logic may incorrectly wipe all failures when size limit is reached. Verify `evict_knowledge_base_if_needed()` handles partial eviction correctly.

## Testing

```bash
cargo test --lib -p slapper ai::
cargo clippy --lib -p slapper
```

## MCP Profile Policy

- `McpProfilePolicy` struct in `tool/protocol/mcp/policy.rs` has **18 fields** — not 7 as some docs claim
- `TargetPolicy` has 4 variants: `ExplicitScopeOnly`, `LocalhostAndPrivateCidrsOnly`, `ScopeOrLocalDevOnly`, `AnyWithScopeEngine` (no `None` variant)
- `ops_agent()` defaults: `max_concurrency: 50`, `max_timeout_ms: 600_000`
- `coding_agent()` is deny-by-default for stress/load/packet/broad-recon tools
- `chat_completion()` on `AiClient` is **private** — use `chat_completion_from_messages()` instead
