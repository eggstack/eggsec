# Slapper Codebase Improvement Plan

Consolidated from code review findings, plan2.md analysis, and live codebase verification.

**Status: Committed as `36ede7d`.** See bottom for completion summary.

---

## Phase 1: Hygiene & Cleanup ✅

### 1.1 Remove untracked files from version control
- [x] Remove `.DS_Store` files from `crates/slapper/src/`, `fuzzer/`, `waf/`, `tui/`
- [x] Remove `tui/mod.rs.bak` backup file
- [x] Add to `.gitignore`:
  ```
  .DS_Store
  *.bak
  ```

**Additional fix:** Changed `.gitignore` pattern `slapper` to `/slapper` — the unanchored
pattern was blocking the entire `crates/slapper/` directory from version control.

**Verification:** passed

---

## Phase 2: Unify Severity Enum (6 → 1) ✅

### Tasks

- [x] 2.1 Create canonical `Severity` in `types.rs` (new file)
- [x] 2.2 Add `types` module and re-export in `lib.rs`
- [x] 2.3 Remove duplicate definitions and re-export:
  - `fuzzer/payloads/mod.rs` — re-export `crate::types::Severity`
  - `config/settings.rs` — re-export `crate::types::Severity`
  - `recon/secrets.rs` — re-export `crate::types::Severity` (fixes missing `Info`)
  - `output/agent.rs` — re-export `crate::types::Severity`
  - `waf/types.rs` — re-export `crate::types::Severity`
  - `output/trend.rs` — re-export `crate::types::Severity`
- [x] 2.4 Handle `tool/response.rs` `None` variant
- [x] 2.5 Merge unique methods into canonical impl (`from_str`, `from_cvss`, `as_str`, `as_int`, `cvss_color`)

### Deviation from plan

The plan proposed using `Option<Severity>` for the `tool/response.rs` None variant.
Instead, we renamed it to `ResponseSeverity` — a separate local enum with `None`.
This was simpler than changing `Finding.severity` field type and updating all
callers. The conversion layer (`tool/convert.rs:62`) maps `ResponseSeverity::None`
to `AgentSeverity::Info`.

**Also added:** `Default` derive with `Info` as default, `Serialize`/`Deserialize`.

**Verification:** `grep -rn "enum Severity"` shows only 1 result (types.rs). 328 tests pass.

---

## Phase 3: Centralize WAF Magic Numbers ✅

### Tasks

- [x] 3.1 Add 8 constants to `constants::waf` module
- [x] 3.2 Replace all magic numbers in `waf/detector.rs`

**Verification:** clippy clean

---

## Phase 4: Optimize WAF Detector Performance ⚠️ Partially Complete

### Completed
- [x] 4.3 Early exit on high confidence (`score >= 90`)

### Deferred
- [ ] 4.1 Pre-compute lowercase signatures — Added complexity without measurable
  benefit at current usage patterns. Worth revisiting if WAF detection becomes
  a bottleneck.
- [ ] 4.2 Cache headers as HashMap once per detection — Same reasoning. The current
  per-signature loop is clear and correct; optimization premature.

**Verification:** clippy clean

---

## Phase 5: Improve Error Handling ⚠️ Partially Complete

### 5.1 Replace `.expect()` with `Result` in FuzzEngine ✅
- [x] Update `FuzzEngine::new()` return type to `Result<Self>`
- [x] Update `FuzzEngine::new_with_tui_mode()` return type to `Result<Self>`
- [x] Update callers: `fuzzer/mod.rs`, `tui/workers/runner.rs`, `pipeline/executor.rs`, `distributed/worker.rs`
- [x] Update `new_from_waf_args()` to propagate error

### 5.2 Consistent error handling in WafDetector — DEFERRED
- [ ] `waf/detector.rs:44`: `detect()` returns `Ok(default)` on request failure
- [ ] `waf/detector.rs:176`: `check_waf_block()` uses `?` operator
- [ ] Decide convention and document

**Reason for deferral:** The current behavior (graceful degradation in `detect()`,
error propagation in `check_waf_block()`) is intentional — `detect()` is a
best-effort lookup, `check_waf_block()` needs the actual response. Changing this
risks breaking callers without clear benefit.

**Verification:** 328 tests pass

---

## Phase 6: Reduce Code Duplication ⚠️ Partially Complete

### Completed
- [x] 6.1 `payload_vec!` macro created in `fuzzer/payloads/macros.rs`
- [x] 6.1 Applied to `sqli.rs` (8 `for` loops → 1 macro call + 1 small loop)
- [x] 6.2 Extracted `fn build_client(args: &FuzzArgs) -> Result<Client>`

### Deferred
- [ ] 6.1 Apply macro to `xss.rs`, `traversal.rs`, `ssrf.rs`, and other payload modules
  — Mechanical but tedious. Good first-contribution task. The macro works
  (verified via sqli.rs tests), applying elsewhere is copy-paste.
- [ ] 6.2 Implement `From<WafStressArgs> for FuzzArgs` — `new_from_waf_args()` works
  correctly; the `From` impl would be cleaner but low priority.
- [ ] 6.3 Break down `FuzzEngine::run_return_session()` — 86 lines is acceptable.
  Extraction would add indirection without improving readability.

**Verification:** 328 tests pass, sqli tests pass (11 tests), clippy clean

---

## Phase 7: Memory Security (Credentials) ✅

### Tasks

- [x] 7.1 Add `zeroize` dependency to Cargo.toml
- [x] 7.2 Create `SensitiveString` wrapper in `types.rs`
- [x] 7.3 Apply to 9 config fields (all from plan, plus `WaybackConfig.api_key`)
- [x] 7.4 Update callers (14 call sites fixed)

### Enhancements beyond plan
- Made field **private** (plan had `pub String`)
- Added `into_secret()` method using `std::mem::take` for `ZeroizeOnDrop` safety
- Implemented **constant-time `PartialEq`** via `subtle::ConstantTimeEq`
- Added custom `Debug` and `Display` that show `[REDACTED]`
- Added transparent `Serialize`/`Deserialize` for config compatibility
- Added 7 unit tests for `SensitiveString`

**Verification:** 328 tests pass

---

## Phase 8: Split Large Files — SKIPPED

### 8.1 `config/settings.rs` (495 lines)
Not needed — file dropped below 500-line threshold after Phase 2 (Severity unification
removed ~50 lines of enum definitions).

### 8.2 Extract WAF detector tests
Not done — tests use `super::*` for private types; extraction would require making
internals public or moving types, adding complexity for no benefit.

---

## Already Implemented (Do Not Repeat)

These items from plan2.md are **already implemented** in the codebase and should not be duplicated:

| Item | Location |
|------|----------|
| `SecurityTool` trait | `tool/traits.rs:117` |
| `ToolRegistry` | `tool/registry.rs:9` |
| `ToolDispatcher` | `tool/registry.rs:135` |
| WAF Severity re-export | `waf/types.rs:5` (`pub use crate::fuzzer::payloads::Severity`) |

---

## Completion Summary

| Phase | Status | Notes |
|-------|--------|-------|
| 1. Hygiene | ✅ Complete | Also fixed critical `.gitignore` bug |
| 2. Severity unification | ✅ Complete | Deviation: `ResponseSeverity` instead of `Option<Severity>` |
| 3. WAF constants | ✅ Complete | |
| 4. WAF optimization | ⚠️ Partial | Early exit done; pre-computation deferred |
| 5. Error handling | ⚠️ Partial | FuzzEngine done; WafDetector deferred |
| 6. Deduplication | ⚠️ Partial | Macro + sqli done; other payloads deferred |
| 7. Memory security | ✅ Complete | Enhanced beyond plan (private field, constant-time eq) |
| 8. File splitting | ⏭ Skipped | Not needed — file sizes already acceptable |

### Deferred items (for future work)

See **Deferred Items — Detailed Plan** below for full specifications.

1. Apply `payload_vec!` to remaining payload modules (D1)
2. Pre-compute lowercase WAF signatures (D2)
3. Normalize WafDetector error handling convention (D3)
4. Implement `From<WafStressArgs> for FuzzArgs` (D4)
5. Extract helper methods from `FuzzEngine::run_return_session()` (D5)

### Post-commit review fixes

The review identified and fixed these issues before committing:
- `.gitignore` pattern `slapper` → `/slapper` (was blocking entire crate from git)
- `SensitiveString` field made private (was `pub String`)
- `SensitiveString::PartialEq` changed to constant-time comparison
- `into_secret()` added using `std::mem::take` for `ZeroizeOnDrop` safety
- 12 unit tests added (5 Severity + 7 SensitiveString)
- Severity `Ord` test corrected (derives declaration order, not semantic order)

---

## Deferred Items — Detailed Plan

### D1: Apply `payload_vec!` to remaining payload modules

**Effort:** Medium | **Impact:** Medium | **Risk:** Low

The macro is proven (11 sqli tests pass). Applying it is mechanical: replace
`for` loops with macro invocations. Modules split into two tiers by complexity.

#### Tier 1 — Standard 3-tuple pattern (straightforward)

These modules use `for (payload, desc, severity) in &array { payloads.push(...) }`
with single tags. Direct 1:1 macro replacement.

| Module | Loops | Lines saved (est.) | Notes |
|--------|-------|--------------------|-------|
| `redos.rs` | 5 | ~50 | All single-tag, pure standard |
| `redirect.rs` | 6 | ~60 | All single-tag, pure standard |
| `ssrf.rs` | 5 | ~60 | 1 multi-tag loop (dns-rebinding) |
| `xss.rs` | 6 | ~60 | 1 multi-tag loop (special-context + ssti) |
| `traversal.rs` | 8 | ~80 | 3 multi-tag loops |
| `sqli.rs` | ✅ done | — | 8 loops reduced to 1 macro call |

**Multi-tag handling:** Use the same pattern as sqli.rs — run the macro for
single-tag groups, then post-process to add extra tags:

```rust
// After macro call:
for p in &mut payloads {
    if p.tags.contains(&"unix".to_string()) && !p.tags.contains(&"file-read".to_string()) {
        p.tags.push("file-read".to_string());
    }
}
```

**Steps for each file:**
1. Remove `let mut payloads = Vec::new();`
2. Remove all `for` loop blocks
3. Add single `payload_vec!(...)` invocation with all groups
4. Add post-processing loops for multi-tag cases (if any)
5. Run module-specific tests: `cargo test --lib -p slapper -- payloads::<name>`

#### Tier 2 — Non-standard patterns (needs macro variant or manual handling)

| Module | Pattern | Notes |
|--------|---------|-------|
| `headers.rs` | 4-tuple `(name, value, desc, severity)` | Uses `format!("{}: {}", name, value)` for payload. Needs macro variant or keep manual. |
| `ssti.rs` | 4-tuple `(payload, desc, engine, severity)` | Dynamic tag from `engine` field. Keep manual — the dynamic tag generation doesn't fit the macro. |

**Recommendation:** Skip `headers.rs` and `ssti.rs`. Their patterns are different
enough that forcing the macro adds complexity. Focus on Tier 1 only.

#### Estimated total

6 modules × ~10 min each = ~1 hour. Test each module individually.

---

### D2: Pre-compute lowercase WAF signatures

**Effort:** Low | **Impact:** Low | **Risk:** Medium

**Current state:** `waf/detector.rs` calls `.to_lowercase()` on every signature
pattern and every response header for every detection request. With 30+ signatures
having 3-5 patterns each, this means 100+ allocations per detection.

#### Approach

Add a `signatures_lower` field to `WafDetector` that stores pre-computed lowercase
copies of all signature patterns at construction time.

```rust
pub struct WafDetector {
    client: Client,
    signatures: HashMap<String, WafSignature>,
    signatures_lower: HashMap<String, WafSignatureLower>,  // new
    common_patterns: Vec<String>,
}

struct WafSignatureLower {
    headers: Vec<String>,       // pre-lowered
    cookies: Vec<String>,       // pre-lowered
    body_patterns: Vec<String>, // pre-lowered
}
```

In `detect()`, lowercase the response data once, then compare against pre-computed
signatures without further allocations.

#### Steps

1. Add `WafSignatureLower` struct and `signatures_lower` field
2. Compute `signatures_lower` in `new()` from existing signatures
3. Update `detect()` to use pre-computed patterns
4. Verify no regressions: `cargo test --lib -p slapper -- waf`
5. Benchmark if possible (theoretical benefit only — WAF detection is not a bottleneck)

#### Risk

Medium — the `WafDetector` test suite is extensive (inline tests + integration tests)
but the logic change touches the hot loop. Must verify all 30+ WAF signatures still
match correctly.

---

### D3: Normalize WafDetector error handling

**Effort:** Low | **Impact:** Low | **Risk:** Medium

**Current state:**
- `detect()` returns `Ok(WafDetectionResult { waf_detected: false, .. })` on
  request failure — graceful degradation
- `check_waf_block()` returns `Err(...)` on request failure — strict

#### Proposed convention

Both methods should follow the same pattern. Two options:

**Option A: Both graceful (recommended)**
- `check_waf_block()` returns `Ok(false)` on error (can't confirm blocking if
  request failed)
- Pros: consistent, callers don't need error handling for WAF checks
- Cons: hides network errors

**Option B: Both strict**
- `detect()` returns `Err(...)` on error
- Pros: callers know something went wrong
- Cons: breaks existing callers that rely on graceful degradation

**Recommendation:** Option A. The WAF detector is a best-effort tool. If the
request fails, we can't detect a WAF, so `Ok(false)` / `Ok(WafDetectionResult::default())`
is the right semantic.

#### Steps

1. Change `check_waf_block()` to catch errors and return `Ok(false)`
2. Add a `log::warn!` on error so failures aren't silently swallowed
3. Update doc comments to document the convention
4. Verify: `cargo test --lib -p slapper -- waf`

---

### D4: Implement `From<WafStressArgs> for FuzzArgs`

**Effort:** Low | **Impact:** Low | **Risk:** Low

**Current state:** `FuzzEngine::new_from_waf_args()` manually maps 25+ fields from
`WafStressArgs` to `FuzzArgs`. This is a 45-line function that's essentially a
`From` impl.

#### Steps

1. Add `impl From<WafStressArgs> for FuzzArgs` in `fuzzer/engine/types.rs`
2. Replace `new_from_waf_args()` body with `Self::new(FuzzArgs::from(args))`
3. Verify: `cargo test --lib -p slapper -- waf`

#### Why not do now

Low risk, but also low impact. The function works correctly. The refactor is
purely cosmetic. Good cleanup if someone is already touching the FuzzEngine code.

---

### D5: Extract helpers from `FuzzEngine::run_return_session()`

**Effort:** Medium | **Impact:** Low | **Risk:** Medium

**Current state:** `run_return_session()` is 86 lines with three logical sections:
1. Advanced type dispatch (lines 212-214)
2. Standard payload execution (lines 216-247)
3. Target-specific payloads (lines 251-271)

#### Proposed extraction

```rust
async fn run_payload_batch(&mut self, payloads: Vec<Payload>) -> Result<Vec<FuzzResult>> {
    let results = match self.args.mode {
        FuzzMode::Sequential => self.run_sequential_with_session(payloads).await?,
        FuzzMode::Burst => self.run_burst_with_session(payloads).await?,
        FuzzMode::Adaptive => self.run_adaptive_with_session(payloads).await?,
    };
    if self.args.diffing && self.differ.is_some() {
        self.apply_diffing(results).await
    } else {
        Ok(results)
    }
}
```

Then `run_return_session()` becomes:
```rust
for pt in payload_types {
    if advanced_types.contains(&pt_str.as_str()) {
        all_results.extend(self.run_advanced_fuzzer(&pt_str).await?);
    } else {
        let payloads = self.prepare_payloads(pt)?;
        all_results.extend(self.run_payload_batch(payloads).await?);
    }
}
```

#### Why defer

The current function is readable at 86 lines. Extraction adds indirection.
Worth doing only if the function grows further or if the mode-diffing logic
is needed elsewhere.

#### Steps

1. Extract `run_payload_batch()` method
2. Extract `prepare_payloads()` method (mutation + grammar + standard)
3. Verify: `cargo test --lib -p slapper -- fuzzer`

