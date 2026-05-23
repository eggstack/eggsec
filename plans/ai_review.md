# AI Agents Architecture Review

**Reviewed**: `architecture/ai_agents.md` against `crates/slapper/src/ai/`
**Date**: 2026-05-23
**Branch**: `architecture/ai-cli-review`

## Summary

Implementation aligns well with architecture documentation. Most features are correctly implemented with proper feature gating and error handling.

## Verified Implementations

| Architecture Claim | Implementation | Status |
|---|---|---|
| `AiClient` with 4 providers (OpenAI, Azure, Anthropic, OpenAICompatible) | `client.rs:9-14` - `Provider` enum with all 4 variants | ✅ |
| Circuit breaker integration | `client.rs:70` - `CircuitBreaker::new(5, 3, 60s)` | ✅ |
| Bearer/Azure auth methods | `client.rs:132-146` - `apply_auth()` | ✅ |
| `analyze_findings()`, `analyze_findings_typed()`, `suggest_payloads()`, `suggest_waf_bypass()` | `client.rs:322-415` - all present | ✅ |
| `AdaptiveScanEngine::adjust_strategy()` | `adaptive.rs:20-55` - returns strategy | ✅ |
| Strategy types (deep, thorough, quick, stealth, standard) | `adaptive.rs:57-70` - `extract_strategy_from_ai_response()` | ✅ |
| Fallback severity-based heuristics | `adaptive.rs:72-83` - `fallback_strategy()` | ✅ |
| `AiPayloadGenerator` with LRU cache (100 entries, 1hr TTL) | `payloads.rs:17` - `AiCache::new(100, 3600s)` | ✅ |
| `CacheKeyBuilder` for collision-free keys | `cache.rs:290-306` | ✅ |
| `SmartWafBypass` with knowledge base | `waf_bypass.rs:21-28` - all fields present | ✅ |
| Knowledge base persists to `waf_bypasses.json` | `waf_bypass.rs:36-47` - path via `ProjectDirs` | ✅ |
| Max 1000 knowledge base entries | `waf_bypass.rs:55` - `max_knowledge_base_size: 1000` | ✅ |
| `iterative_bypass()` method | `waf_bypass.rs:127-156` - present | ✅ |
| `AiCache` with RwLock for thread-safety | `cache.rs:73` - `Arc<RwLock<FxHashMap<...>>>` | ✅ |
| `CacheEntry` with value, timestamp, TTL, hit_count | `cache.rs:9-16` - all fields | ✅ |
| `AiPlanner` with learning cache | `planner.rs:47-52` - present | ✅ |
| Feature-gated `ai-integration` | `mod.rs:6-7,18-21` - `#[cfg(feature = "ai-integration")]` | ✅ |
| `ScriptGenerator` with 3 methods | `script_gen.rs:72-199` - all 3 methods present | ✅ |
| Script directory `generated_scripts/` | `script_gen.rs:62-64` - correct path | ✅ |
| `AiError` enum with 8 variants | `errors.rs:5-33` - all variants present | ✅ |

## Issues Found

### 1. `unwrap_or_default()` Used in Non-Test Production Code

**File**: `ai/waf_bypass.rs:44`

```rust
.unwrap_or_default()
```

**Context**: In `with_config()` when loading knowledge base from file:
```rust
let knowledge_base = if persist_path.exists() {
    std::fs::read_to_string(&persist_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()  // <-- silently suppresses deserialization errors
} else {
    Vec::new()
};
```

**Impact**: If `waf_bypasses.json` contains corrupted JSON, the error is silently suppressed and an empty knowledge base is used, potentially losing learned bypasses.

**Recommendation**: Use `map_err()` with logging:
```rust
.unwrap_or_else(|e| {
    tracing::warn!("Failed to load WAF bypass knowledge base: {}", e);
    Vec::new()
})
```

### 2. `.expect()` in Test Code Only

All `.expect()` calls found are in `#[cfg(test)]` blocks. This is acceptable since test failures should panic.

- `client.rs:451,455,470,490,498,512,653,661,670` - all in test module
- `waf_bypass.rs:248` - in test module
- `script_gen.rs:338` - in test module
- `types.rs:48-49,64-65,78-79,93-94,115-116` - all in test module

### 3. FxHashMap Usage Correct

All HashMap/HashSet usages correctly use `FxHashMap` from `rustc_hash`:
- `cache.rs:73,81,96,125,135,236,255` - all `FxHashMap`
- `planner.rs:42,50,77` - all `FxHashMap`

### 4. Clock Skew Handling in planner.rs

`planner.rs:207-209,467-470,480-483` use `unwrap_or_else(|_| Duration::from_secs(0))` which correctly handles clock skew. ✅

## Architecture Discrepancies

None found. All documented features are correctly implemented.

## Performance Observations

1. **Good**: `Arc<RwLock<FxHashMap>>` pattern used correctly in `AiCache`
2. **Good**: No `unwrap_or_default()` in hot paths
3. **Good**: Regex operations avoided in favor of simpler string operations

## Recommendations

1. Fix `waf_bypass.rs:44` to use `unwrap_or_else` with logging instead of `unwrap_or_default`
2. Consider adding persistence failure warning when knowledge base fails to load

## Test Coverage

Tests are comprehensive and cover:
- Provider parsing (OpenAI, Azure, Anthropic, OpenAICompatible)
- Auth application (bearer, azure)
- Circuit breaker state transitions
- Cache operations (get, set, expiry, eviction)
- Strategy extraction from AI responses
- Fallback strategy selection
- WAF bypass record success/failure
- Script generation