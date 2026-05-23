# Fuzzer Module Architecture Review

Review of `architecture/fuzzer.md` against implementation in `crates/slapper/src/fuzzer/`

---

## Verified Claims

### Core Architecture

| Claim | Status | Implementation |
|-------|--------|----------------|
| State Management (`state.rs`) | ✅ Verified | `fuzzer/state.rs` - `HttpSession`, `SessionManager`, `AuthHandler` |
| Mutator (`mutator.rs`) | ✅ Verified | `fuzzer/mutator.rs` - 10 mutation types implemented |
| Rate Limiting (`rate_limit.rs`) | ✅ Verified | `fuzzer/rate_limit.rs` - `AdaptiveRateLimiter`, `RateLimiterTokenBucket` |
| Sequential/Burst/Adaptive modes | ✅ Verified | `engine/execution.rs` - `run_sequential`, `run_burst`, `run_adaptive` |
| Grammar-based Fuzzing (`grammar.rs`) | ✅ Verified | `fuzzer/grammar.rs` - `Grammar`, `GrammarFuzzer` for JSON/GraphQL/XML/JWT/SSTI |

### Payloads

| Claim | Status | Implementation |
|-------|--------|----------------|
| SQLi, XSS, Command Injection, Template Injection | ✅ Verified | `payloads/sqli.rs`, `xss.rs`, `cmd.rs`, `ssti.rs` |
| Path Traversal, LFI/RFI | ✅ Verified | `payloads/traversal.rs` |
| Authentication bypass, Parameter Pollution | ✅ Verified | `payloads/idor.rs`, `payloads/mass_assign.rs` |
| 31 payload types | ✅ Verified | `payloads/mod.rs:38-70` - `PayloadType` enum has 31 variants |

### Detection

| Claim | Status | Implementation |
|-------|--------|----------------|
| Error-based detection | ✅ Verified | `detection/patterns.rs` - `get_database_error_patterns()` |
| Boolean-based detection | ✅ Verified | `detection/aho_corasick.rs` - `PatternMatcher` |
| Time-based detection | ✅ Verified | `detection/analyzer.rs` - `TimingAnalyzer` with IQR |
| Diffing (`diff.rs`) | ✅ Verified | `fuzzer/diff.rs` - `ResponseDiffer` |

### WAF Fingerprinting

| Claim | Status | Implementation |
|-------|--------|----------------|
| WAF detection & bypass | ✅ Verified | `fuzzer/waf_fingerprint.rs` exists |

### Specialized Fuzzing

| Claim | Status | Implementation |
|-------|--------|----------------|
| API Schema Fuzzing (`api_schema/`) | ✅ Verified | `fuzzer/api_schema/mod.rs` - OpenAPI 3.0 parsing, type-aware fuzzing |
| Advanced Threat Hunting (`advanced.rs`) | ✅ Verified | `fuzzer/advanced.rs` - GraphQL, JWT, OAuth, IDOR, SSTI, WebSocket, gRPC |
| ReDoS Detection (`redos_detect.rs`) | ✅ Verified | `fuzzer/redos_detect.rs` - `RegexExecutor`, `ReDosDetector` |

### Code Conventions

| Claim | Status | Implementation |
|-------|--------|----------------|
| FxHashMap/FxHashSet | ✅ Verified | Used in `state.rs`, `api_schema/mod.rs`, `redos_detect.rs`, `diff.rs` |
| Magic numbers as constants | ✅ Verified | `detection/analyzer.rs:27-29` - `DEFAULT_SPIKE_THRESHOLD`, `DEFAULT_REDOS_THRESHOLD_MS`, `DEFAULT_MIN_SAMPLES_FOR_BASELINE` |
| WAF blocked status codes constant | ✅ Verified | `engine/utils.rs:18` - `const WAF_BLOCKED_STATUS_CODES: &[u16] = &[403, 406, 429];` |
| IQR NaN handling | ✅ Verified | `detection/analyzer.rs:168-176` - explicit `partial_cmp` with NaN handling |
| OVERSIZED_PAYLOAD_SIZES constant | ✅ Verified | `api_schema/mod.rs:7` - `[1_000, 10_000, 100_000, 1_000_000]` |
| Auth bypass headers | ✅ Verified | `api_schema/mod.rs:229-232` - `X-Original-URL`, `X-Override-URL`, `X-Rewrite-URL` |

---

## Discrepancies

### 1. Payload Type Count Mismatch
- **Documentation**: Line 1 mentions "30 payload types"
- **Implementation**: `payloads/mod.rs:38-70` defines **31** `PayloadType` variants (Sqli through Oast)
- **Impact**: Low - documentation undercounts by 1
- **Priority**: Low

### 2. Execution Modes Documentation Incomplete
- **Documentation**: Line 14 says "Burst (concurrent up to 500)"
- **Implementation**: `engine/core.rs:134` clamps concurrency to maximum 500
- **Actual limit enforced**: Both min and max clamped (line: `args.concurrency.clamp(1, 500)`)
- **Documentation gap**: Doesn't mention minimum of 1 is also enforced
- **Priority**: Low

### 3. WebSocket Payload Type Reference
- **Documentation**: Line 114 says "use `PayloadType::Websocket`, not `PayloadType::Grpc`"
- **Implementation**: `advanced.rs:432` correctly uses `PayloadType::Websocket`
- **Status**: Accurate ✅
- **Priority**: N/A (claim verified correct)

---

## Bugs Found

### Bug 1: Adaptive Rate Limiter Can Reach Zero (Denial of Service)
- **Location**: `rate_limit.rs:106-113`
- **Issue**: `backoff()` can reduce rate to `min_rate` (configurable to 1), but `execution.rs:266-270` checks `if rate == 0` and **breaks the fuzzing loop**, stopping all further tests
- **Code**: `execution.rs:267` - `if rate == 0 { tracing::warn!("Adaptive rate limiter backed off to 0, stopping"); break; }`
- **Problem**: If `min_rate` is set to 0 (or overflow occurs), adaptive mode terminates prematurely
- **Severity**: Medium - can cause fuzzing to stop early under error conditions
- **Priority**: Medium

### Bug 2: TimingAnalyzer IQR Division by Zero (Already Fixed)
- **Location**: `detection/analyzer.rs:188-190`
- **Issue**: Before fix, could divide by zero if `iqr_samples` was empty
- **Status**: ✅ **Fixed** - lines 188-191 now check `if iqr_samples.is_empty()` before division
- **Priority**: Resolved

### Bug 3: Concurrent Execution Result Ordering via DashMap
- **Location**: `engine/execution.rs:95, 207-218`
- **Issue**: Uses `DashMap<usize, FuzzResult>` for concurrent insertion, then sorts by index
- **Problem**: `Arc::try_unwrap(results)` at line 207 can fail if workers still hold references, returning an error instead of results
- **Code**: Lines 207-215 show error handling when `try_unwrap` fails
- **Severity**: Low - error path exists but is unlikely to trigger in practice
- **Priority**: Low

### Bug 4: Session State Update Only on Success Status Codes
- **Location**: `engine/utils.rs:194`
- **Issue**: Only updates session state for `status_code == 200 || status_code == 302`
- **Problem**: Other successful status codes (201, 204, etc.) won't trigger auth detection even if response contains relevant leaks
- **Severity**: Low - limited impact on fuzzing effectiveness
- **Priority**: Low

---

## Improvement Opportunities

### 1. Remove `unwrap_or_default()` in Fuzzer Module (High Impact)

**Locations**:
- `advanced.rs:93` - `let target_url = self.target_url.clone().unwrap_or_default();`
- `targets/api.rs:162` - `.unwrap_or_default()`

**Recommendation**: Use explicit match with tracing as per architecture guidelines.

**Priority**: Medium

### 2. ResponseDiffer Could Use Constant for Body Length Threshold

**Location**: `diff.rs:228`
- Currently: `const BODY_LENGTH_ANOMALY_THRESHOLD: isize = 1000;` (local to `compute_diff`)
- **Recommendation**: Move to module level or constants file for consistency with `TIMING_ANOMALY_THRESHOLD_MS` (line 293)

**Priority**: Low

### 3. GrammarFuzzer Could Share RNG Seed Option

**Location**: `grammar.rs:212-220`
- `with_seed()` exists but `GrammarFuzzer::new()` always uses `from_entropy()`
- **Recommendation**: Add constructor that accepts seed parameter for reproducible fuzzing

**Priority**: Low

### 4. ReDosDetector Known Patterns Could Be LazyLock

**Location**: `redos_detect.rs:229-247`
- `default_vulnerable_patterns()` is called on every `ReDosDetector::new()` 
- **Recommendation**: Make `KNOWN_VULNERABLE_PATTERNS` a `LazyLock` static

**Priority**: Low

### 5. TimingAnalyzer Clone Implementation

**Location**: `detection/analyzer.rs:31-46`
- **Issue**: `Clone` impl manually clones atomic values using `load(Ordering::Relaxed)`
- **Minor concern**: Not a bug but could be simplified; current approach is safe
- **Priority**: None (functionally correct)

### 6. Missing `PartialEq` for TimingResult

**Location**: `detection/analyzer.rs:4-10`
- `TimingResult` has `pub` fields but no `PartialEq` derive
- **Recommendation**: Add `#[derive(PartialEq)]` if comparison is needed

**Priority**: None (not currently needed)

---

## Priority Summary

| Finding | Type | Priority |
|---------|------|----------|
| Adaptive rate limiter can reach zero and stop fuzzing | Bug | Medium |
| `unwrap_or_default()` in fuzzer module | Anti-pattern | Medium |
| Payload type count off by one in docs | Discrepancy | Low |
| Concurrent result ordering complexity | Design | Low |
| ResponseDiffer threshold constant placement | Consistency | Low |
| ReDosDetector pattern static caching | Performance | Low |
| GrammarFuzzer seed reproducibility | Feature | Low |

---

## Recommendations

1. **Fix adaptive rate limiter zero-check**: Change `if rate == 0` to `if rate <= 1` or use a sentinel minimum
2. **Update documentation**: Fix payload count from 30 to 31
3. **Add seed parameter to GrammarFuzzer**: For reproducible fuzzing runs
4. **Cache ReDosDetector patterns**: Use `LazyLock` for known vulnerable patterns