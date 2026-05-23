# AI Agents Architecture Review

Review of `architecture/ai_agents.md` against implementation in `crates/slapper/src/ai/` and `crates/slapper/src/agent/`.

---

## Verified Claims

### AI Client (`client.rs`)
- **Providers**: OpenAI, Azure, Anthropic, OpenAICompatible - VERIFIED (line 9-14)
- **Bearer/Azure auth**: VERIFIED (lines 35-49, 132-145)
- **Circuit breaker**: VERIFIED (line 70, 172-174, 417-419)
- **Methods**: `chat_completion()`, `analyze_findings()`, `analyze_findings_typed()`, `suggest_payloads()`, `suggest_waf_bypass()` - VERIFIED

### Adaptive Fuzzing (`adaptive.rs`)
- `AdaptiveScanEngine::adjust_strategy()` - VERIFIED (lines 20-55)
- Strategies: The doc says "deep, thorough, quick, stealth, standard" but implementation has: deep, thorough, quick, stealth, standard (line 57-70) - **NOTE: "aggressive" maps to "thorough"**
- Fallback to severity-based heuristics when AI unavailable - VERIFIED (lines 43-44, 72-83)

### Payload Generation (`payloads.rs`)
- `AiPayloadGenerator` with LRU caching (100 entries, 1hr TTL) - VERIFIED (line 17)
- `CacheKeyBuilder` for collision-free cache keys - VERIFIED (line 26)
- Script generation methods - VERIFIED (script_gen.rs)

### WAF Bypass (`waf_bypass.rs`)
- `SmartWafBypass` with knowledge base - VERIFIED (lines 21-28)
- Knowledge base persists to `waf_bypasses.json` (max 1000 entries) - VERIFIED (lines 36-38, 64)
- `iterative_bypass()` for multi-iteration refinement - VERIFIED (lines 136-165)
- Tracks success/failure per (WAF, payload) pair - VERIFIED (lines 102-117)

### Caching (`cache.rs`)
- `AiCache` - Thread-safe async cache with RwLock - VERIFIED (line 73)
- `CacheEntry` - Value, timestamp, TTL, hit count - VERIFIED (lines 10-16)
- `CacheKeyBuilder` - VERIFIED (lines 290-306)
- `with_persistence()` - VERIFIED (lines 142-155)

### AI Planner (`planner.rs`)
- `AiPlanner::create_plan()` - VERIFIED (lines 82-100)
- `AiPlanner::suggest_adjustments()` - VERIFIED (lines 214-232)
- `AiPlanner::record_outcome()` - VERIFIED (lines 446-487)
- Learning cache with success rate tracking - VERIFIED (lines 50-60)

### Autonomous Agents (`src/agent/`)
- Agent Runner (`mod.rs`) - Core polling loop, scheduled scan dispatch - VERIFIED
- Memory (`memory.rs`) - Longitudinal context and baseline-aware finding comparisons - VERIFIED
- Portfolio (`portfolio.rs`) - Stores targets, schedules, scan history metadata - VERIFIED
- Constraints (`constraints/`) - Do-not-do rules, target restrictions, scan/rate limits - VERIFIED
- Skills (`skills.rs`) - Discrete capabilities - VERIFIED

---

## Discrepancies

### 1. Feature-Gated Documentation Inaccurate

**Document says:**
- "AI Planner (`planner.rs`) - Feature-gated `ai-integration`"
- "Script Generation (`script_gen.rs`) - Feature-gated `ai-integration`"

**Actual implementation:**
- `planner.rs` has NO feature gates - always compiled
- `script_gen.rs` has NO feature gates - always compiled

**File: `crates/slapper/src/ai/planner.rs`** - No `#[cfg(feature = "ai-integration")]` guards
**File: `crates/slapper/src/ai/script_gen.rs`** - No `#[cfg(feature = "ai-integration")]` guards

**Impact**: Medium - Documentation incorrectly describes feature gating behavior.

---

### 2. Skills Module Uses Standard HashMap

**Document says:** Agent module uses FxHashMap/FxHashSet per "Recent Bug Fixes" section.

**Actual implementation:**
- `agent/skills.rs:202-203` uses `HashMap<String, Skill>` and `HashMap<String, Vec<String>>`
- `agent/portfolio.rs:112` uses `HashMap<String, usize>` for `severity_counts`

**Impact**: Medium - Performance inconsistency across agent modules.

---

## Bugs Found

### 1. Silent Error Handling in script_gen.rs

**Location:** `script_gen.rs:97, 141, 185, 272`

```rust
let code = Self::extract_code_block(content, "python")
    .or_else(|| Some(content.trim().to_string()))
    .unwrap_or_default();  // Silent failure
```

```rust
let findings_str = serde_json::to_string_pretty(findings).unwrap_or_default();  // Silent failure
```

**Issue:** These silently swallow errors. If serialization fails, empty/unexpected content may propagate.

**Priority:** Medium

---

### 2. Anthropic Message Transformation Silent Fallback

**Location:** `client.rs:241`

```rust
let messages = body
    .get("messages")
    .and_then(|v| v.as_array())
    .cloned()
    .unwrap_or_default();  // Silent fallback to empty array
```

**Issue:** If `messages` extraction fails, an empty array is used silently. This could cause the AI to respond without proper context.

**Priority:** Medium

---

### 3. AiCache Deserialization Silently Swallows Errors

**Location:** `cache.rs:147-150`

```rust
if let Ok(serialized) = serde_json::from_str::<AiCacheSerialized>(&contents) {
    let cache: AiCache = serialized.into();
    self.entries = cache.entries;  // Only updates if parsing succeeds
}
```

**Issue:** Deserialization errors are silently ignored. If `AiCacheSerialized` structure changed, cache silently resets.

**Priority:** Low

---

## Improvement Opportunities

### 1. Convert HashMap to FxHashMap in Agent Modules

**Files affected:**
- `agent/skills.rs:202-203` - `HashMap<String, Skill>`, `HashMap<String, Vec<String>>`
- `agent/portfolio.rs:112` - `HashMap<String, usize>` in `ScanRecord.severity_counts`

**Recommendation:** Replace with `FxHashMap` for performance consistency.

**Priority:** Medium

---

### 2. Add Feature Gates to planner.rs and script_gen.rs

**Current:** Both files compile unconditionally.

**Recommendation:** Add `#[cfg(feature = "ai-integration")]` guards to match documentation, or update documentation to reflect actual behavior.

**Priority:** Low (if intentional, doc needs update)

---

### 3. AiCache persist() Could Fail Silently

**Location:** `cache.rs:276-278`

```rust
if let Ok(json) = serde_json::to_string(&serialized) {
    let _ = std::fs::write(path, json);  // Silent write failure
}
```

**Recommendation:** Log persistence failures explicitly rather than silently ignoring.

**Priority:** Low

---

### 4. AiCache Loading Race Condition

**Location:** `cache.rs:142-155`

The `with_persistence()` method reads from disk during construction, which may fail silently if the file is corrupt or permissions are insufficient.

**Recommendation:** Add explicit error logging and consider graceful degradation rather than silent fallback.

**Priority:** Low

---

## Priority Summary

| Finding | Type | Priority |
|---------|------|----------|
| Feature-gated documentation inaccurate | Discrepancy | Medium |
| Skills module uses HashMap | Discrepancy | Medium |
| Silent error handling in script_gen.rs | Bug | Medium |
| Anthropic message transformation silent fallback | Bug | Medium |
| AiCache deserialization silently swallows errors | Bug | Low |
| FxHashMap migration for skills/portfolio | Improvement | Medium |
| AiCache persist() could fail silently | Improvement | Low |

---

## Recommendations

1. **Update `architecture/ai_agents.md`** to remove incorrect feature-gating claims for `planner.rs` and `script_gen.rs`

2. **Replace HashMap with FxHashMap** in `agent/skills.rs` and `agent/portfolio.rs` for performance consistency

3. **Replace `unwrap_or_default()` with explicit error handling** in `script_gen.rs` and `client.rs` to avoid silent failures

4. **Add tracing for cache persistence failures** to aid debugging

5. **Consider lazy-loading for WAF bypass knowledge base** if it grows large (currently loaded entirely into memory)