# Fuzzer Module Architecture Review

**Review Date:** Sat May 23 2026  
**Document Reviewed:** architecture/fuzzer.md  
**Implementation Path:** crates/slapper/src/fuzzer/

---

## Verified Claims

### Core Architecture

| Claim | Implementation | Status |
|-------|----------------|--------|
| State Management (state.rs) | state.rs - HttpSession, SessionManager, AuthHandler | VERIFIED |
| Mutator (mutator.rs) | mutator.rs - 11 mutation types with SmallRng | VERIFIED |
| Rate Limiting (rate_limit.rs) | rate_limit.rs - AdaptiveRateLimiter + RateLimiterTokenBucket | VERIFIED |
| Execution Modes: Sequential, Burst, Adaptive | execution.rs - run_sequential, run_burst, run_adaptive | VERIFIED |
| Grammar-based Fuzzing (grammar.rs) | grammar.rs - Grammar, GrammarFuzzer with 5 grammar kinds | VERIFIED |

### Payloads

| Claim | Implementation | Status |
|-------|----------------|--------|
| 30 Payload Types | mod.rs - PayloadType enum has 31 variants (lines 39-70) | DISCREPANCY |
| Injection payloads (SQLi, XSS, etc.) | payloads/sqli.rs, xss.rs, cmd.rs, etc. | VERIFIED |
| File System (Path Traversal, LFI/RFI) | payloads/traversal.rs | VERIFIED |
| Grammar-based payload generation | grammar.rs - GrammarFuzzer with Json, GraphQL, Xml, Jwt, Ssti | VERIFIED |

### Detection

| Claim | Implementation | Status |
|-------|----------------|--------|
| Error-based detection | detection/patterns.rs - get_detection_patterns(), get_database_error_patterns() | VERIFIED |
| Boolean-based detection | Implemented via response comparison in core.rs | VERIFIED |
| Time-based detection | detection/analyzer.rs - TimingAnalyzer with IQR-based baseline | VERIFIED |
| Diffing (diff.rs) | diff.rs - ResponseDiffer with SHA256 body hashing | VERIFIED |

### WAF Fingerprinting & Bypass

| Claim | Implementation | Status |
|-------|----------------|--------|
| WAF detection logic | waf_fingerprint.rs - WafFingerprinter with 18 fingerprints | VERIFIED |
| Bypass techniques | Each fingerprint includes bypass_techniques vector | VERIFIED |

### Specialized Fuzzing

| Claim | Implementation | Status |
|-------|----------------|--------|
| API Schema Fuzzing (api_schema/) | api_schema/mod.rs - OpenAPI 3.0 parsing | VERIFIED |
| Advanced Threat Hunting (advanced.rs) | advanced.rs - GraphQL, JWT, OAuth, IDOR, SSTI, WebSocket, gRPC fuzzers | VERIFIED |
| ReDoS Detection (redos_detect.rs) | redos_detect.rs - RegexExecutor with timeout and iteration limits | VERIFIED |

### Code Conventions

| Claim | Implementation | Status |
|-------|----------------|--------|
| FxHashMap/FxHashSet usage | state.rs:75-79, diff.rs:2-3, chain.rs:4, etc. | VERIFIED |
| Magic numbers extracted to constants | detection/analyzer.rs:27-29, diff.rs:228,293, api_schema/mod.rs:7 | VERIFIED |
| IQR-based TimingAnalyzer with NaN handling | detection/analyzer.rs:166-176 | VERIFIED |
| WAF blocked status codes constant | engine/utils.rs:18 | VERIFIED |

---

## Discrepancies

### 1. Payload Type Count Discrepancy (Low Priority - Documentation)

**Arch Doc:** "30 payload types"
**Implementation:** PayloadType enum has 31 variants (lines 39-70):
Sqli, Xss, Traversal, Ssrf, Redirect, Redos, Headers, Compression, GraphQL, OAuth, Jwt, Idor, Ssti, Grpc, Xxe, Ldap, Cmd, Deser, Host, Cache, Csv, Soap, Websocket, Nosql, Xpath, Expression, Prototype, Race, MassAssign, Oast

**Note:** The module docstring at mod.rs:1 says "30 payload types" but the enum has 31. This is a documentation bug.

### 2. Grammar Fuzzer RNG Not Serializable (Medium Priority - Design Issue)

**Arch Doc:** Grammar Fuzzer can use with_seed() for deterministic fuzzing
**Implementation:** grammar.rs:221-246 - GrammarFuzzer struct has rng: rand::rngs::SmallRng which does NOT implement Serialize/Deserialize

**Impact:** If an AI planner tries to serialize and restore GrammarFuzzer state (e.g., for mid-session recovery), it will fail because SmallRng is not serializable.

---

## Bugs Found

### BUG 1: Adaptive Rate Limiter Premature Termination (Medium Priority)

**File:** crates/slapper/src/fuzzer/engine/execution.rs:265-270

```
let rate = limiter.get_rate();
if rate <= 1 {
    tracing::warn!("Adaptive rate limiter backed off to 0, stopping");
    break;
}
```

**Issue:** The rate limiter's min_rate defaults to 1 in rate_limit.rs:28. The condition rate <= 1 will ALWAYS be true when the rate limiter has backed off to its minimum, causing premature termination.

**Recommended Fix:** Change condition to rate < 1 instead of rate <= 1.

### BUG 2: JWT Payload unwrap_or_default Silent Error Loss (Medium Priority)

**File:** crates/slapper/src/fuzzer/payloads/jwt.rs - multiple lines (176, 313, 334, 355, 380, 401)

```
serde_json::from_str(&parts.header).unwrap_or_default();
```

**Issue:** These are inside analysis loops where unwrap_or_default() silently falls back to empty errors, losing potentially valuable debugging information.

**Recommended Fix:** Use explicit error handling with tracing.

### BUG 3: Known Vulnerable Patterns Clone Per Instance (Low Priority)

**File:** crates/slapper/src/fuzzer/redos_detect.rs:241

```
known_vulnerable_patterns: KNOWN_VULNERABLE_PATTERNS.clone(),
```

**Issue:** The .clone() creates a new heap allocation for every ReDosDetector::new() call.

**Recommended Fix:** Use Arc::clone() to share the static.

---

## Improvement Opportunities

### IMPROVEMENT 1: Payload Cache Initializes All Types on First Access (High Priority)

**Location:** crates/slapper/src/fuzzer/payloads/mod.rs:140-150

**Issue:** PAYLOAD_CACHE is initialized on first access with ALL 31 payload types, loading ~3000+ payloads into memory immediately, even if only one payload type is requested.

**Recommended Fix:** Change to per-type lazy initialization using std::sync::OnceLock.

### IMPROVEMENT 2: TimingAnalyzer Clone Resets Statistics (Medium Priority)

**Location:** crates/slapper/src/fuzzer/detection/analyzer.rs:31-47

**Issue:** When a TimingAnalyzer is cloned mid-session, the samples vector is also cloned, creating independent histories.

**Recommended Fix:** Document whether clone behavior is intentional or use Arc<Mutex<TimingAnalyzer>> for shared state.

### IMPROVEMENT 3: Missing Progress Bar in run_sequential_with_session (Low Priority)

**Location:** crates/slapper/src/fuzzer/engine/execution.rs:236-252

**Issue:** Unlike run_sequential() which updates a progress bar, run_sequential_with_session() has no progress tracking.

**Recommended Fix:** Add optional progress bar support.

---

## Priority Summary

| Finding | Type | Priority | File:Line |
|---------|------|----------|-----------|
| Payload count documentation (31 vs 30) | Discrepancy | Low | mod.rs:1 vs line 39-70 |
| GrammarFuzzer RNG not serializable | Improvement | Medium | grammar.rs:221 |
| Adaptive rate limiter rate <= 1 bug | Bug | Medium | execution.rs:267 |
| JWT unwrap_or_default silent errors | Bug | Medium | jwt.rs:176,313,334,355,380,401 |
| Known vulnerable patterns clone per instance | Bug | Low | redos_detect.rs:241 |
| Payloads cached upfront (memory) | Improvement | High | payloads/mod.rs:140-150 |
| TimingAnalyzer clone behavior | Improvement | Medium | analyzer.rs:31-47 |
| Missing progress bar in sequential_with_session | Improvement | Low | execution.rs:236-252 |

---

## Summary

The fuzzer module is well-architected and largely matches its documentation. Key strengths:

- Proper use of FxHashMap/FxHashSet throughout the codebase
- Good separation of concerns (engine, payloads, detection, diff)
- Comprehensive payload library with 31 types (discrepancy: doc says 30)
- Well-implemented timing analysis with IQR-based baseline
- WAF fingerprinting with 18 vendor signatures
- Good test coverage with execution tests

**Issues found:**
- 2 medium-priority bugs (adaptive rate limiter termination, JWT silent errors)
- 1 low-priority bug (pattern clone inefficiency)
- 2 medium-priority improvements (serialization, clone behavior)
- 1 high-priority improvement (payload cache memory)
- 1 documentation discrepancy (payload count 31 vs 30)

The module demonstrates good Rust patterns and follows most architecture guidelines correctly. No critical issues were found.
