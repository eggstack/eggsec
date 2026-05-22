# Fuzzer Module Review - Improvement Plan

## Summary of Architecture Document

The `architecture/fuzzer.md` document describes the fuzzer module as the most advanced part of Slapper, designed to find vulnerabilities by sending semi-random or crafted data to targets and analyzing responses.

### Key Components Described:

1. **Fuzzing Engine (`engine/`)**: State management, mutator, rate limiting, and execution modes (Sequential, Burst, Adaptive)

2. **Payloads (`payloads/`)**: Injection (SQLi, XSS, Command), File System (Path Traversal, LFI/RFI), Logic (Auth bypass, Parameter Pollution), Grammar-based fuzzing

3. **Detection (`detection/`)**: Error-based, Boolean-based, Time-based detection, and response diffing

4. **WAF Fingerprinting & Bypass (`waf_fingerprint.rs`)**: WAF detection and bypass techniques

5. **Specialized Fuzzing**: API Schema (OpenAPI/gRPC), Advanced Threat Hunting, ReDoS Detection

### Code Conventions Specified in Architecture:

- **Hash Collections**: Use `rustc_hash::FxHashMap` and `FxHashSet` instead of `std::collections::HashMap/HashSet`
- **Magic Numbers**: Extract to named constants at module level
- **Error Handling**: Prefer explicit match over `unwrap_or_default()`
- **WAF Detection**: Status codes 403, 406, 429 defined as `WAF_BLOCKED_STATUS_CODES` constant
- **Timing Analysis**: `TimingAnalyzer` handles NaN values explicitly in `partial_cmp`
- **API Schema Fuzzing**: Uses `OVERSIZED_PAYLOAD_SIZES` constant
- **Advanced Fuzzers**: `WebSocketFuzzer.into_fuzz_result()` uses `PayloadType::Websocket` (NOT `PayloadType::Grpc`)

---

## Verification of Key Claims

| Claim | Status | Notes |
|-------|--------|-------|
| `WAF_BLOCKED_STATUS_CODES` constant in `engine/utils.rs` | VERIFIED | Line 18: `const WAF_BLOCKED_STATUS_CODES: &[u16] = &[403, 406, 429];` |
| `OVERSIZED_PAYLOAD_SIZES` constant in `api_schema/mod.rs` | VERIFIED | Line 7: `const OVERSIZED_PAYLOAD_SIZES: [usize; 4] = [1_000, 10_000, 100_000, 1_000_000];` |
| TimingAnalyzer handles NaN in `partial_cmp` | VERIFIED | `detection/analyzer.rs` lines 168-176 and 214-222 have explicit NaN handling |
| `PayloadType::Websocket` used in WebSocketFuzzer | VERIFIED | `payloads/websocket.rs` line 126 uses `PayloadType::Websocket` |
| FxHashMap/FxHashSet used for performance | PARTIAL | Most files use correctly, but `fuzzer/targets/api.rs` uses `std::collections::HashMap` |

---

## Bugs Found

### 1. Unwrap/Expect Calls in Test Code (Lower Priority - Tests Only)

| File | Line | Issue |
|------|------|-------|
| `engine/utils.rs` | 410 | `FuzzEngine::new(args).expect("engine should construct")` in test |
| `engine/utils.rs` | 527, 535, 543 | `result.unwrap()` in test code |
| `engine/types.rs` | 407, 415, 416, 440 | `serde_json::...unwrap()` in test code |
| `api_schema/mod.rs` | 633, 650, 656, 676, 696 | `ApiSchemaFuzzer::parse_openapi(...).unwrap()` in tests |
| `engine/execution.rs` | 382, 392, 402, 411, 420 | `FuzzEngine::new(args).unwrap()` in tests |
| `engine/advanced.rs` | 231, 237, 250, etc. | Multiple `FuzzEngine::new(args).unwrap()` in tests |
| `engine/core.rs` | 518, 532, 543, etc. | Multiple `engine.unwrap()` in tests |
| `detection/aho_corasick.rs` | 53 | `.expect("Failed to create Aho-Corasick matcher")` in test |
| `jwt.rs` | 709, 724 | `.expect()` and `.unwrap()` in test code |

**Assessment**: These are all in test code (`#[cfg(test)]` modules), so they don't affect production but should still be addressed for code quality.

### 2. Unwrap_or_default() Calls Silently Suppressing Errors (Production Impact)

| File | Line | Issue |
|------|------|-------|
| `chain.rs` | 288, 293, 298, 303 | `.unwrap_or_default()` on `get()` for variable extraction |
| `engine/chained.rs` | 125, 130 | `.unwrap_or_default()` on variable lookups |
| `calibration.rs` | 104 | `response.text().await.unwrap_or_default()` - silently fails |
| `targets/api.rs` | 162 | `.unwrap_or_default()` on header/cookie value |
| `payloads/ssti.rs` | 280 | `resp.text().await.unwrap_or_default()` - silently fails |
| `payloads/oauth.rs` | 559 | `resp.text().await.unwrap_or_default()` - silently fails |

**Assessment**: These silently suppress errors. The architecture document explicitly says to use explicit match with tracing instead.

### 3. LazyLock Regex Unwrap in Production Path

| File | Line | Issue |
|------|------|-------|
| `chain.rs` | 381 | `static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\$\{(\w+)\}").unwrap());` |

**Assessment**: If the regex is invalid, this will panic at first use. The pattern `\$\{(\w+)\}` is hardcoded and should be valid, but using `expect()` instead of `unwrap()` would be clearer.

### 4. GrpcFuzzer Uses Wrong PayloadType

| File | Line | Issue |
|------|------|-------|
| `payloads/grpc.rs` | 91 | Uses `PayloadType::Ssrf` instead of `PayloadType::Grpc` |

**Note**: This appears to be intentional as the `fuzz()` method returns `FuzzResult` with `PayloadType::Ssrf`. While the architecture doc says to use `PayloadType::Grpc`, the actual behavior may be deliberate for test categorization.

---

## Performance Issues

### 1. HashMap/HashSet Instead of FxHashMap/FxHashSet

| File | Line | Type | Impact |
|------|------|------|--------|
| `fuzzer/targets/api.rs` | 4 | `use std::collections::HashMap` | Module-level import |
| `fuzzer/targets/api.rs` | 12 | `pub paths: HashMap<String, PathItem>` | OpenAPISpec |
| `fuzzer/targets/api.rs` | 47 | `pub responses: HashMap<String, Response>` | Operation |
| `fuzzer/targets/api.rs` | 64 | `pub content: HashMap<String, MediaType>` | RequestBody |
| `fuzzer/targets/api.rs` | 77 | `pub properties: Option<HashMap<String, Schema>>` | Schema |
| `fuzzer/targets/api.rs` | 95 | `pub content: Option<HashMap<String, MediaType>>` | Response |
| `fuzzer/targets/api.rs` | 96 | `pub headers: Option<HashMap<String, serde_json::Value>>` | Response |
| `fuzzer/targets/api.rs` | 101 | `pub schemas: Option<HashMap<String, Schema>>` | Components |
| `fuzzer/targets/api.rs` | 102 | `pub security_schemes: Option<HashMap<String, SecurityScheme>>` | Components |
| `fuzzer/targets/api.rs` | 117 | `pub schemes: HashMap<String, Vec<String>>` | SecurityRequirement |

**Recommendation**: Change all `HashMap` to `FxHashMap` in `targets/api.rs`.

---

## Pattern Violations

### 1. Error Handling Pattern Violation

The architecture document (lines 64-76) specifies:
```rust
// Instead of:
let body = response.text().await.unwrap_or_default();

// Use:
let body = match response.text().await {
    Ok(text) => text,
    Err(e) => {
        tracing::debug!("Failed to read response body: {}", e);
        String::new()
    }
};
```

**Violations Found**:
- `calibration.rs:104` - `response.text().await.unwrap_or_default()`
- `targets/api.rs:162` - `.unwrap_or_default()` on header value
- `payloads/ssti.rs:280` - `resp.text().await.unwrap_or_default()`
- `payloads/oauth.rs:559` - `resp.text().await.unwrap_or_default()`

### 2. WebSocketFuzzer Correct Usage Verified

The architecture document (line 114) says: "use `PayloadType::Websocket`, not `PayloadType::Grpc`"

`payloads/websocket.rs:126` correctly uses `PayloadType::Websocket`. This is verified correct.

---

## Recommended Fixes

### Priority 1 - Performance (FxHashMap/FxHashSet)

**File**: `fuzzer/targets/api.rs`

Change all `std::collections::HashMap` to `rustc_hash::FxHashMap`:

```rust
// Line 4: Change import
use rustc_hash::FxHashMap;

// Lines 12, 47, 64, 77, 95, 96, 101, 102, 117: Change type usages
pub paths: FxHashMap<String, PathItem>,
pub responses: FxHashMap<String, Response>,
// etc.
```

### Priority 2 - Error Handling (unwrap_or_default)

**File**: `fuzzer/calibration.rs:104`
```rust
// Before:
let body = response.text().await.unwrap_or_default();

// After:
let body = match response.text().await {
    Ok(text) => text,
    Err(e) => {
        tracing::debug!("Failed to read calibration response body: {}", e);
        String::new()
    }
};
```

**File**: `fuzzer/payloads/ssti.rs:280`
```rust
// Before:
let body = resp.text().await.unwrap_or_default();

// After:
let body = match resp.text().await {
    Ok(text) => text,
    Err(e) => {
        tracing::debug!("Failed to read SSTI response body: {}", e);
        String::new()
    }
};
```

**File**: `fuzzer/payloads/oauth.rs:559`
```rust
// Before:
let body = resp.text().await.unwrap_or_default();

// After:
let body = match resp.text().await {
    Ok(text) => text,
    Err(e) => {
        tracing::debug!("Failed to read OAuth response body: {}", e);
        String::new()
    }
};
```

### Priority 3 - Chain Variable Extraction

**File**: `fuzzer/chain.rs:282-304`

The `execute_extract` function uses `unwrap_or_default()` on variable lookups. While this may be intentional (fallback to empty string when variable doesn't exist), explicit handling with tracing would be better:

```rust
// Line 288: Change
.unwrap_or_default()
// To:
.map(|s| s.clone())
.unwrap_or_else(|| {
    tracing::debug!("Variable {} not found in chain execution", "_last_body");
    String::new()
});
```

### Priority 4 - LazyLock Expect

**File**: `fuzzer/chain.rs:381`
```rust
// Before:
static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\$\{(\w+)\}").unwrap());

// After:
static RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\$\{(\w+)\}").expect("Interpolation regex must be valid")
});
```

---

## Summary of Changes Needed

| Priority | File | Issue | Lines |
|----------|------|-------|-------|
| P1 | `fuzzer/targets/api.rs` | HashMap -> FxHashMap | 4, 12, 47, 64, 77, 95, 96, 101, 102, 117 |
| P2 | `fuzzer/calibration.rs` | unwrap_or_default -> explicit match | 104 |
| P2 | `fuzzer/payloads/ssti.rs` | unwrap_or_default -> explicit match | 280 |
| P2 | `fuzzer/payloads/oauth.rs` | unwrap_or_default -> explicit match | 559 |
| P3 | `fuzzer/chain.rs` | unwrap_or_default -> explicit with tracing | 288, 293, 298, 303 |
| P4 | `fuzzer/chain.rs` | LazyLock unwrap -> expect | 381 |

---

## Verification Notes

The architecture document is largely accurate in describing the implementation:

1. **Verified Correct**:
   - `WAF_BLOCKED_STATUS_CODES` constant exists at line 18 of `engine/utils.rs`
   - `OVERSIZED_PAYLOAD_SIZES` constant exists at line 7 of `api_schema/mod.rs`
   - TimingAnalyzer has explicit NaN handling in `partial_cmp` (lines 168-176, 214-222)
   - `WebSocketFuzzer` correctly uses `PayloadType::Websocket`

2. **Needs Improvement**:
   - `fuzzer/targets/api.rs` should use `FxHashMap` instead of `HashMap`
   - Error handling should use explicit match with tracing instead of `unwrap_or_default()`

3. **Minor Issues**:
   - `grpc.rs:91` uses `PayloadType::Ssrf` instead of `PayloadType::Grpc` (may be intentional)
   - Many test files use `.unwrap()` which is acceptable in test contexts but could use `.unwrap_or_else()` for clearer error messages
