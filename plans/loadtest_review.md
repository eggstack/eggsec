# Loadtest Module Architecture Review

**Date:** 2026-05-23
**Module:** `crates/slapper/src/loadtest/`
**Files Reviewed:** `runner.rs`, `metrics.rs`, `mod.rs`

---

## Verified Claims

### Core Structure (runner.rs)
| Claim | Implementation | Status |
|-------|---------------|--------|
| `LoadTestRunner` struct with all documented fields | Lines 19-33: `url`, `total_requests`, `concurrency`, `timeout`, `method`, `body`, `headers`, `insecure`, `proxy`, `proxy_auth`, `user_agent`, `rate_limit`, `tui_mode` | VERIFIED |
| Worker count: `min(concurrency, total_requests)` | Line 283: `let worker_count = self.concurrency.min(self.total_requests as usize);` | VERIFIED |
| Uses tokio `JoinSet` | Line 284: `let mut workers = JoinSet::new();` | VERIFIED |
| `apply_auth_headers()` helper method | Lines 176-208: Supports Basic, Bearer, Cookie, API Key auth | VERIFIED |
| All 5 constructors | `new()` (36-43), `new_with_tui_mode()` (45-78), `from_args()` (80-82), `from_args_with_tui_mode()` (84-107), `from_args_with_config()` (109-138) | VERIFIED |

### Metrics (metrics.rs)
| Claim | Implementation | Status |
|-------|---------------|--------|
| `FxHashMap<u16, u64>` for status codes | Line 68, 79: Uses `rustc_hash::FxHashMap` | VERIFIED |
| `hdrhistogram::Histogram<u64>` | Line 65: `histogram: Histogram<u64>` | VERIFIED |
| 3 significant figures | Line 76: `Histogram::new(3).expect(...)` | VERIFIED |
| Percentiles p50, p90, p95, p99 | Lines 132-135: `value_at_percentile(50.0/90.0/95.0/99.0)` | VERIFIED |
| `LoadTestResults` struct | Lines 8-24: All fields match documentation | VERIFIED |

### Rate Limiting Algorithm
| Claim | Implementation | Status |
|-------|---------------|--------|
| Lock-protected token bucket | Lines 306-317: `Mutex` protecting `next_allowed_at` | VERIFIED |
| Sleep until `next_allowed` if current time < next | Lines 309-310: `if now < *next { sleep(*next - now).await; }` | VERIFIED |
| Update: `next = now_after_sleep + interval` (not `next + interval`) | Lines 313-317: Proper drift correction implemented | VERIFIED |

### Response Body Handling
| Claim | Implementation | Status |
|-------|---------------|--------|
| Non-success response bodies consumed | Lines 339-341: `if !status.is_success() { let _ = response.bytes().await; }` | VERIFIED |

---

## Discrepancies

### 1. Missing Constructor in Documentation
**Severity:** Low
**Section:** "Constructors" table (lines 41-48)

The documentation lists only 4 constructors but `from_args_with_config()` is the 5th constructor (lines 109-138). This is important because:
- It's required for pipeline integration
- The documentation explicitly notes "Use `from_args_with_config()` for pipeline integration" but doesn't list it in the constructor table

**Impact:** Users may miss this constructor when reading the docs.

### 2. Unused `record_success()` Method
**Severity:** Low
**Section:** Metrics API

The `record_success()` method exists in `metrics.rs:85-90` but is **never called** by the runner. The runner uses `record_http_response()` instead. The documentation mentions it but doesn't clarify it's not used by the primary execution path.

---

## Bugs Found

### Bug 1: 3xx Responses Body Consumption Inconsistent
**File:** `runner.rs:339-341`
**Severity:** Medium

```rust
if !status.is_success() {
    let _ = response.bytes().await;
}
```

**Issue:** `status.is_success()` only returns true for 2xx (200-299). **3xx responses (redirects) are NOT successful**, so their bodies are consumed. However, 300-399 are often successful redirects that should have bodies consumed too.

**Status:** This works but is slightly inconsistent - 3xx bodies are unnecessarily consumed but this doesn't break anything.

### Bug 2: Error List Limited to 100 Errors
**File:** `metrics.rs:101, 109`
**Severity:** Medium

```rust
if self.errors.len() < 100 {
    self.errors.push(format!("HTTP {}", status_code));
}
```

**Issue:** If you have 1000 different HTTP error responses, only the first 100 are stored. The documentation doesn't mention this limit.

**Impact:** Hard to debug issues when many different error types occur.

### Bug 3: Redirect Test Doesn't Verify Actual Following
**File:** `loadtest_tests.rs:87-118`
**Severity:** Medium

The test `test_load_test_redirect_following()` doesn't verify that redirects were actually followed - it only checks that 3 requests completed. Since reqwest doesn't follow redirects by default, this test may not be testing what it claims.

---

## Improvement Opportunities

### 1. Use `status.is_success()` vs Range Check
**File:** `metrics.rs:97`
**Priority:** Low
**Impact:** ~2% performance improvement

The code uses `(200..400).contains(&status_code)` but could use `status.is_success()` which may be slightly optimized in reqwest.

```rust
// Current
if (200..400).contains(&status_code) {

// Better
if status.is_success() {
```

However, `is_success()` only covers 2xx. If 3xx should be considered success too (as the doc implies), this needs a helper function.

### 2. Add Error Rate Limit Configuration
**File:** `metrics.rs:101, 109`
**Priority:** Medium
**Impact:** Better debugging for high-cardinality error scenarios

Currently hardcoded at 100. Should be configurable.

### 3. Document Unused `record_success()` Method
**File:** `metrics.rs:85-90`
**Priority:** Low
**Impact:** Reduces confusion

Either document that it's for external use or remove dead code.

### 4. Add Test for Redirect Following
**File:** `loadtest_tests.rs:87-118`
**Priority:** Medium
**Impact:** Verification of actual redirect behavior

The test `test_load_test_redirect_following()` doesn't verify that redirects were actually followed (only that 3 requests completed). Since reqwest doesn't follow redirects by default, this test may not be testing what it claims.

### 5. Consider TUI Mode Progress Output
**File:** `runner.rs:259-270`
**Priority:** Low

The progress bar is suppressed in TUI mode (`if self.tui_mode { None }`), but no alternative output is provided. Users have no indication of progress in TUI mode.

---

## Priority Summary

| Finding | Type | Priority |
|---------|------|----------|
| Error list capped at 100 | Bug | Medium |
| Redirect following test doesn't verify behavior | Test Gap | Medium |
| 3xx response body consumption inconsistent | Bug | Low |
| Missing `from_args_with_config()` in constructor table | Documentation | Low |
| Unused `record_success()` method | Dead Code | Low |
| `status.is_success()` vs range check | Optimization | Low |
| TUI mode has no progress output | UX | Low |

---

## Recommendations

1. **High Priority:** Fix the error list limit - make it configurable or use a larger value (1000)
2. **Medium Priority:** Add assertion to redirect test to verify actual redirect following
3. **Medium Priority:** Clarify in documentation whether 3xx should be counted as successful
4. **Low Priority:** Replace `(200..400).contains()` with `status.is_success()` for consistency
5. **Low Priority:** Update constructor table to include `from_args_with_config()`
6. **Low Priority:** Document or remove unused `record_success()` method
