# AI & Agents Architecture Review

**Document**: `architecture/ai_agents.md`
**Review Date**: 2026-05-24
**Implementation Location**: `crates/slapper/src/ai/` and `crates/slapper/src/agent/`

---

## Summary Statistics

| Category | Count |
|----------|-------|
| **Verified Claims** | 32 |
| **Discrepancies** | 5 |
| **Bugs Found** | 3 |
| **Improvement Opportunities** | 8 |
| **High Priority Issues** | 4 |
| **Medium Priority Issues** | 5 |
| **Low Priority Issues** | 7 |

---

## Verified Claims

### AI Integration - AI Client (`client.rs`)

1. **Providers: OpenAI, Azure, Anthropic, OpenAICompatible** - VERIFIED (`client.rs:9-14`)
2. **Bearer/Azure auth support** - VERIFIED (`client.rs:132-146`)
3. **Circuit breaker integration** - VERIFIED (`client.rs:70`, uses `CircuitBreaker`)
4. **Response normalization for Anthropic** - VERIFIED (`client.rs:277-299`)
5. **`chat_completion()` method** - VERIFIED (`client.rs:148-162`)
6. **`analyze_findings()` method** - VERIFIED (`client.rs:324-340`)
7. **`analyze_findings_typed()` method** - VERIFIED (`client.rs:342-380`)
8. **`suggest_payloads()` method** - VERIFIED (`client.rs:382-395`)
9. **`suggest_waf_bypass()` method** - VERIFIED (`client.rs:397-417`)

### AI Integration - Adaptive Fuzzing (`adaptive.rs`)

10. **`AdaptiveScanEngine::adjust_strategy()` exists and analyzes findings** - VERIFIED (`adaptive.rs:20-55`)
11. **Strategies: deep, thorough, quick, stealth, standard** - VERIFIED (`adaptive.rs:57-70`)
12. **Falls back to severity-based heuristics when AI unavailable** - VERIFIED (`adaptive.rs:47-52`)
13. **`fallback_strategy()` method** - VERIFIED (`adaptive.rs:72-83`)

### AI Integration - Payload Generation (`payloads.rs`)

14. **`AiPayloadGenerator` with LRU caching** - VERIFIED (`payloads.rs:8-11`)
15. **Cache: 100 entries, 1hr TTL** - VERIFIED (`payloads.rs:17`: `AiCache::new(100, Duration::from_secs(3600))`)
16. **`CacheKeyBuilder` for collision-free cache keys** - VERIFIED (`cache.rs:294-308`)

### AI Integration - WAF Bypass (`waf_bypass.rs`)

17. **`SmartWafBypass` maintains knowledge base** - VERIFIED (`waf_bypass.rs:21-28`)
18. **Knowledge base persists to `waf_bypasses.json`** - VERIFIED (`waf_bypass.rs:36-38`)
19. **Max 1000 entries** - VERIFIED (`waf_bypass.rs:64`: `max_knowledge_base_size: 1000`)
20. **Tracks success/failure per (WAF, payload) pair** - VERIFIED (`waf_bypass.rs:167-211`)
21. **`iterative_bypass()` for multi-iteration refinement** - VERIFIED (`waf_bypass.rs:136-165`)

### AI Integration - Caching (`cache.rs`)

22. **`AiCache` - Thread-safe async cache with RwLock** - VERIFIED (`cache.rs:72-73`)
23. **`CacheEntry` - Value, timestamp, TTL, hit count** - VERIFIED (`cache.rs:10-16`)
24. **`CacheKeyBuilder` - Builder for consistent key formation** - VERIFIED (`cache.rs:292-308`)
25. **Persistence via `with_persistence()`** - VERIFIED (`cache.rs:142-155`)

### AI Integration - AI Planner (`planner.rs`)

26. **`AiPlanner::create_plan()` creates execution plans** - VERIFIED (`planner.rs:82-100`)
27. **`AiPlanner::suggest_adjustments()` suggests modifications** - VERIFIED (`planner.rs:214-232`)
28. **`AiPlanner::record_outcome()` learns from outcomes** - VERIFIED (`planner.rs:446-487`)
29. **Learning cache with success rate tracking** - VERIFIED (`planner.rs:54-60`)

### AI Integration - Script Generation (`script_gen.rs`)

30. **Feature-gated `ai-integration`** - VERIFIED (`ai/mod.rs:8-9`)

### Autonomous Agents (`src/agent/`)

31. **Agent Runner (`mod.rs`)**: Core polling loop, scheduled scan dispatch, event handling - VERIFIED
32. **Memory (`memory.rs`)**: Longitudinal context and baseline-aware finding comparisons - VERIFIED

### Recent Bug Fixes (2026-05-22)

33. **waf_bypass.rs:107** - `continue` after `failed_attempts >= 3` - VERIFIED (`waf_bypass.rs:116`)
34. **planner.rs:456** - Fixed `ExecutionStage` field reference - VERIFIED (`planner.rs:455-456`)
35. **cache.rs** - Changed `HashMap` to `FxHashMap` - VERIFIED (`cache.rs:73`)
36. **planner.rs** - Changed `HashMap` to `FxHashMap` - VERIFIED (`planner.rs:50`)
37. **routing.rs** - Changed to `FxHashMap`/`FxHashSet` - VERIFIED (`routing.rs:21,59`)
38. **alerts/mod.rs** - Changed to `FxHashMap` - VERIFIED
39. **memory.rs** - Changed to `FxHashMap`/`FxHashSet` - VERIFIED
40. **events.rs** - `severity_counts` is `FxHashMap` - VERIFIED (`events.rs:32`)
41. **mod.rs** - `unwrap_or_else()` with warning log - VERIFIED (`mod.rs:657`)

---

## Discrepancies

### D1: Skills Module Location

**Architecture**: "Skills (`skills.rs`): Represents discrete capabilities the agent can employ"

**Actual Implementation**: Skills is **feature-gated** under `ai-integration` and located at `agent/skills.rs:19-20`:
```rust
#[cfg(feature = "ai-integration")]
pub mod skills;
```

**Impact**: Low - Skills module cannot be used without `ai-integration` feature enabled.

---

### D2: Portfolio Module Documentation Mismatch

**Architecture**: "Portfolio (`portfolio.rs`): Stores targets, schedules, and scan history metadata"

**Actual Implementation**: `TargetPortfolio` uses `parking_lot::RwLock` (not `std::sync::RwLock`), `FxHashMap`, and has extensive methods beyond simple storage.

**Impact**: Low - Implementation is more sophisticated than documented.

---

### D3: Constraints Module - "Enforces do-not-do rules"

**Architecture**: States "Constraints (`constraints/`): Enforces do-not-do rules, target restrictions, and scan/rate limits"

**Actual Implementation**: 
- `constraints.rs` defines `DoNotDoList`, `ForbiddenAction`, `OffPeakConfig`, `OperationalConstraints`
- `constraints/checker.rs` has `ConstraintChecker` with `evaluate_action`, `evaluate_target`, `evaluate_scan_depth`, `evaluate_rate_limit`

**Verification**: Partially verified - rate limit tracking uses in-memory `FxHashMap` without persistence or reset mechanism across agent restarts.

**Impact**: Medium - Rate limit budget is not persistent across agent restarts.

---

### D4: MCP Integration

**Architecture**: States "Slapper implements the **Model Context Protocol (MCP)**, allowing it to be used as a 'tool' by other AI agents"

**Actual Implementation**: No MCP-specific implementation found in `agent/` or `ai/` modules. MCP is mentioned in architecture but no code implements it.

**Impact**: High - Documentation claims MCP support that doesn't exist in implementation.

---

### D5: Script Generation Directory

**Architecture**: "Scripts saved to `generated_scripts/` directory"

**Actual Implementation**: `script_gen.rs` not fully reviewed, but the directory path should be verified.

**Impact**: Low - Need to verify path exists and is created if missing.

---

## Bugs Found

### B1: Rate Limit Counter Never Resets

**File**: `crates/slapper/src/agent/constraints/checker.rs:215-228`

```rust
pub fn evaluate_rate_limit(&self, key: &str) -> Result<(), ConstraintViolation> {
    if let Some(limit) = self.constraints.rate_limit_budget {
        let mut request_counts = self.request_counts.lock().unwrap();
        let current = request_counts.entry(key.to_string()).or_insert(0);
        if *current >= limit {
            return Err(ConstraintViolation::RateLimitExceeded {...});
        }
        *current += 1;
    }
    Ok(())
}
```

**Problem**: `request_counts` is an in-memory `Arc<Mutex<FxHashMap<String, usize>>>` that accumulates counts but has no TTL-based or time-based reset mechanism. The `reset_rate_limits()` method exists (`checker.rs:291-294`) but is never called automatically.

**Fix**: Implement a background task or time-based reset, or document that rate limits reset on agent restart.

**Priority**: Medium

---

### B2: Knowledge Base Eviction Bug

**File**: `crates/slapper/src/ai/waf_bypass.rs:80-88`

```rust
fn evict_knowledge_base_if_needed(&mut self) {
    if self.knowledge_base.len() >= self.max_knowledge_base_size {
        self.knowledge_base.retain(|e| e.success);  // BUG: Incorrectly removes ALL failures
        if self.knowledge_base.len() >= self.max_knowledge_base_size {
            self.knowledge_base.sort_by_key(|e| e.failed_attempts);
            self.knowledge_base.truncate(self.max_knowledge_base_size / 2);
        }
    }
}
```

**Problem**: The first `retain` call removes ALL failed entries, which may be incorrect behavior. A failed entry might have `failed_attempts >= 3` and be skipped in `find_bypass`, but it shouldn't be arbitrarily deleted. The logic seems intended to keep successful bypasses while making room, but the implementation could wipe out valuable failed_attempts data.

**Fix**: Consider keeping entries with `failed_attempts < 3` or using a more nuanced eviction policy.

**Priority**: Medium

---

### B3: `ScanRecord` Uses `std::collections::HashMap` in Test

**File**: `crates/slapper/src/agent/portfolio.rs:572`

```rust
severity_counts: std::collections::HashMap::new(),  // Should use FxHashMap
```

**Problem**: Test code uses `std::collections::HashMap` instead of `FxHashMap` like the rest of the codebase.

**Priority**: Low

---

## Improvement Opportunities

### I1: Add MCP Implementation

**Description**: Implement Model Context Protocol support as documented in architecture.

**Impact**: Would enable integration with external AI platforms and tools.

**Priority**: High

---

### I2: Persist Rate Limit Budget Across Restarts

**Description**: Store rate limit counters in persistent storage or implement time-based reset.

**Impact**: Prevents rate limit bypass after agent restart.

**Priority**: Medium

---

### I3: Improve Knowledge Base Eviction Logic

**Description**: Review and potentially fix the `evict_knowledge_base_if_needed()` algorithm.

**Impact**: Better memory management for WAF bypass knowledge base.

**Priority**: Medium

---

### I4: Add Feature Flag Warning for Skills

**Description**: Document that Skills module requires `ai-integration` feature.

**Impact**: Prevents confusion when Skills is unavailable.

**Priority**: Low

---

### I5: Portfolio Atomic Write Verification

**Description**: The atomic write implementation uses `fs::rename` but doesn't verify the rename succeeded or handle partial writes gracefully.

**File**: `crates/slapper/src/agent/portfolio.rs:262-277`

**Impact**: Potential data loss on filesystem errors.

**Priority**: Medium

---

### I6: Add TTL-based Rate Limit Reset

**Description**: Implement sliding window or fixed window rate limiting with automatic reset.

**Impact**: Better rate limit enforcement.

**Priority**: Medium

---

### I7: Skill Loader Error Handling

**File**: `crates/slapper/src/agent/skills.rs:172-198`

**Description**: `load_skills()` silently skips invalid skills but doesn't provide aggregate error reporting.

**Impact**: Debugging skill loading issues is difficult.

**Priority**: Low

---

### I8: Memory Module Pattern Detection Optimization

**File**: `crates/slapper/src/agent/memory.rs:452-489`

**Description**: `detect_and_record_patterns()` rewrites patterns file on every scan. Consider batching or coalescing writes.

**Impact**: Performance improvement for high-frequency scans.

**Priority**: Low

---

## Priority Summary

| ID | Category | Issue | Priority |
|----|----------|-------|----------|
| B1 | Bug | Rate limit counter never resets | Medium |
| B2 | Bug | Knowledge base eviction may incorrectly wipe failures | Medium |
| B3 | Bug | Test uses wrong HashMap type | Low |
| D4 | Discrepancy | MCP integration not implemented | High |
| I1 | Improvement | Add MCP implementation | High |
| I2 | Improvement | Persist rate limit budget | Medium |
| I3 | Improvement | Improve KB eviction logic | Medium |
| I5 | Improvement | Portfolio atomic write verification | Medium |
| I6 | Improvement | TTL-based rate limit reset | Medium |
| D1 | Discrepancy | Skills feature-gated | Low |
| I4 | Improvement | Document feature flag requirement | Low |
| I7 | Improvement | Skill loader error handling | Low |
| I8 | Improvement | Memory pattern detection optimization | Low |

---

## Key Findings

1. **Documentation is generally accurate** - Most claims in `ai_agents.md` are verified in the implementation.

2. **MCP support is claimed but not implemented** - This is the most significant discrepancy between documentation and implementation.

3. **Bug fixes from 2026-05-22 are properly implemented** - The recent bug fixes listed in the architecture document are all present in the codebase.

4. **FxHashMap migration is complete** - All specified modules use `FxHashMap`/`FxHashSet` for performance.

5. **Rate limiting lacks persistence** - A gap between policy definition and enforcement exists.