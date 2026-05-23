# WAF Module Architecture Review

**Review Date:** 2026-05-23  
**Reviewer:** Architecture Review Session  
**Document Under Review:** `architecture/waf.md`

## Executive Summary

The WAF module architecture document is largely accurate and well-structured. However, there are several discrepancies between documented claims and actual implementation, including payload count mismatches, a misleading architectural description, and one performance bug (local constant defined inside a loop).

---

## Verified Claims

### Core Components Structure ✓
The directory structure in `architecture/waf.md` matches the actual implementation exactly:

| Documented | Actual |
|------------|--------|
| `waf/mod.rs` | `waf/mod.rs` - WafEngine, public exports, run_cli() |
| `waf/types.rs` | `waf/types.rs` - OwaspCategory, Finding, ScanResults, ScanSummary |
| `waf/output.rs` | `waf/output.rs` - Text/JSON output formatting |
| `waf/waf_patterns.rs` | `waf/waf_patterns.rs` - Pattern utilities |
| `bypass/mod.rs` | `bypass/mod.rs` - BypassEngine, BypassResult, BypassTechnique, TestType |
| `bypass/profiles.rs` | `bypass/profiles.rs` - WAF profiles |
| `data/patterns.rs` | `data/patterns.rs` - 34 WAF signatures |
| `detector/mod.rs` | `detector/mod.rs` - WafDetector struct |
| `detector/detect.rs` | `detector/detect.rs` - WAF detection logic |
| `payloads/encoding.rs` | `payloads/encoding.rs` - Payload sets |

### Data Structure Locations ✓
All key data structures are located where documented:

- `WafDetector` - `detector/mod.rs` ✓
- `WafDetectionResult` - `detector/types.rs` ✓
- `WafSignature` - `data/patterns.rs` ✓
- `WafEngine` - `mod.rs` ✓
- `BypassEngine` - `bypass/mod.rs` ✓
- `BypassResult` - `bypass/mod.rs` ✓
- `BypassTechnique` - `bypass/mod.rs` ✓ (15 variants)
- `WafProfile` - `bypass/profiles.rs` ✓
- `ResponseDiff` - `detector/types.rs` ✓

### WAF Detection - 34 Products ✓
**Confirmed:** The system supports exactly 34 WAF products:
- 26 explicitly defined in `data/patterns.rs` static initialization
- 8 auto-generated via `get_generated_profiles()` for WAFs without manual profiles
- Total: 34 WAF signatures

### Scoring System ✓
**Confirmed:** The scoring system uses `u16` to prevent overflow, with correct constant values:

| Component | Documented | Actual | Location |
|-----------|-----------|--------|----------|
| Header match | +25 | 25 | `constants.rs:70` |
| Cookie match | +20 | 20 | `constants.rs:71` |
| Body pattern | +15 | 15 | `constants.rs:72` |
| IP match | +20 | 20 | `constants.rs:73` |
| High confidence exit | 90 | 90 | `constants.rs:76` |

### Bypass Profile Caching ✓
**Confirmed:** `get_waf_profiles()` and `get_profile_by_name()` in `profiles.rs` use `static LazyLock<Vec<WafProfile>>` to cache profiles.

### Bypass Success Detection Logic ✓
**Confirmed:** `is_bypass_successful()` in `bypass/mod.rs:131-157` correctly implements all four checks:
1. Response status NOT in blocked codes (403, 406, 429, 503)
2. Response status differs from baseline
3. Response status is 2xx (200-299)
4. Payload (or URL-encoded version) reflected in response body

### HTTP Smuggling ✓
**Confirmed:** `smuggling.rs` implements HTTP desync techniques including:
- CL.TE (Content-Length vs Transfer-Encoding)
- TE.CL (Transfer-Encoding vs Content-Length)
- Chunked malformed
- Request tunneling
- HTTP/2 upgrade probes

---

## Discrepancies

### 1. Supported WAFs List Incomplete in Code Docstring
**Location:** `crates/slapper/src/waf/mod.rs:16-21`

**Issue:** The docstring in `mod.rs` lists only 23 WAFs, while `architecture/waf.md` correctly lists 34 WAFs.

**Documented:** Cloudflare, Akamai, AWS WAF, Azure WAF, Google Cloud Armor, Fastly, Imperva, Sucuri, CloudFront, F5 BIG-IP, Barracuda, Fortinet, Citrix NetScaler, ModSecurity, Wordfence, DataDome, PerimeterX, Nginx, Traefik, Kong, Varnish, Radware, Signal Sciences, Wallarm, Reblaze (24 listed)

**Actual:** 34 WAFs in system (26 explicit + 8 auto-generated)

**Impact:** Low - documentation elsewhere is accurate.

### 2. Payload Count Mismatches
**Location:** `crates/slapper/src/waf/payloads/encoding.rs`

| Payload Type | Documented | Actual | Discrepancy |
|--------------|------------|--------|-------------|
| SQLi | 19 | **19** | None |
| XSS | 18 | **17** | -1 |
| SSRF | 15 | **16** | +1 |
| Cmd Injection | 16 | **16** | None |
| Traversal | 11 | **10** | -1 |

**Code Reference:**
- SQLi: lines 96-117 (19 items)
- XSS: lines 74-93 (17 items - missing one)
- SSRF: lines 120-138 (16 items - one extra)
- Cmd: lines 141-159 (16 items)
- Traversal: lines 162-174 (10 items - one missing)

**Impact:** Low - functional but documentation is incorrect.

### 3. BypassEngine Description Misleading
**Location:** `crates/slapper/src/waf/mod.rs:43`

**Documented:** "Orchestrates bypass testing across three sub-engines"

**Issue:** This implies parallel/concurrent execution across sub-engines (Header, Evasion, Smuggling). In reality, `BypassEngine::run_bypasses()` in `bypass/mod.rs:95-128` executes each bypass type **sequentially**, not in parallel.

```rust
// Actual sequential execution:
if self.args.header_bypass || self.args.bypass {
    let header_bypass = HeaderBypass::new(profile.cloned());
    results.extend(header_bypass.run(...).await?);
}

if self.args.evasion || self.args.bypass {
    let evasion_bypass = EvasionBypass::new(profile.cloned());
    results.extend(evasion_bypass.run(...).await?);
}

if self.args.smuggling || self.args.bypass {
    let smuggling_bypass = SmugglingBypass::new(profile.cloned());
    results.extend(smuggling_bypass.run(...).await?);
}
```

**Impact:** Low - functional but misleading architecture description.

---

## Bugs Found

### BUG 1: Local Constant Defined Inside Loop (Performance)
**Location:** `crates/slapper/src/waf/detector/detect.rs:76`

**Severity:** Medium

**Issue:** The constant `HEADER_VALUE_MAX_LEN` is defined **inside** the outer `for` loop at line 69, causing it to be re-created on every iteration of the outer loop.

```rust
for (sig_key, signature) in self.signatures.iter() {
    let sig_lower = &self.signatures_lower[sig_key];
    let mut score = 0u16;
    // ... other code ...

    const HEADER_VALUE_MAX_LEN: usize = 256;  // <-- BUG: Defined inside loop!

    for header_pattern_lower in &sig_lower.headers {
        for (name_lower, value_lower) in &headers_lower {
            // ... uses HEADER_VALUE_MAX_LEN ...
        }
    }
    // ... rest of loop ...
}
```

**Impact:** Unnecessary memory allocation and initialization on each loop iteration. While the compiler may optimize this, it's poor code style and could cause confusion.

**Fix:** Move `HEADER_VALUE_MAX_LEN` to module level or as a static constant.

---

### BUG 2: HTTP/2 Smuggling Techniques Always Skipped
**Location:** `crates/slapper/src/waf/bypass/smuggling.rs:298-300`

**Severity:** Medium

**Issue:** `supports_http2_probes()` is hardcoded to return `false`, causing all HTTP/2 smuggling techniques (H2CUpgrade, Http2Frame) to be silently skipped:

```rust
fn supports_http2_probes() -> bool {
    false  // <-- Hardcoded to false
}
```

This means the HTTP/2 cleartext (h2c) upgrade smuggling technique will never work, even when HTTP/2 support is available.

**Impact:** Reduces the effectiveness of the smuggling bypass module for targets that support HTTP/2.

---

### BUG 3: Smuggling `execute_raw_http1` Memory Not Zeroized
**Location:** `crates/slapper/src/waf/bypass/smuggling.rs:365`

**Severity:** Low (Security)

**Issue:** While `request_bytes` is zeroized after use, the variable remains in scope for the duration of the function. This is a minor issue but good practice would be to zeroize immediately after the request is sent.

```rust
let mut request_bytes = self.build_raw_request(host, req);
// ... send request ...
request_bytes.fill(0);  // Zeroized at end of function, not immediately after send
```

**Impact:** Low - timing window is minimal but should be addressed for security-sensitive code.

---

## Improvement Opportunities

### HIGH Priority

#### 1. Move `HEADER_VALUE_MAX_LEN` to Module Level
**Location:** `crates/slapper/src/waf/detector/detect.rs:76`

**Current:**
```rust
const HEADER_VALUE_MAX_LEN: usize = 256;  // Inside loop
```

**Suggested Fix:**
```rust
const HEADER_VALUE_MAX_LEN: usize = 256;

impl WafDetector {
    pub async fn detect(&self, url: &str) -> Result<WafDetectionResult> {
        // ... existing code ...
        for (sig_key, signature) in self.signatures.iter() {
            // Remove the local constant definition
            // Use module-level constant
        }
    }
}
```

**Estimated Impact:** Minor performance improvement, better code clarity.

---

### MEDIUM Priority

#### 2. Implement Actual HTTP/2 Detection
**Location:** `crates/slapper/src/waf/bypass/smuggling.rs:298-300`

**Current:**
```rust
fn supports_http2_probes() -> bool {
    false
}
```

**Suggested Fix:** Implement actual HTTP/2 capability detection, possibly using a feature flag or runtime detection:

```rust
fn supports_http2_probes() -> bool {
    #[cfg(feature = "http2-support")]
    {
        // Check if HTTP/2 can actually be used
        true
    }
    #[cfg(not(feature = "http2-support"))]
    {
        false
    }
}
```

**Estimated Impact:** Enables HTTP/2 smuggling bypass techniques when appropriate.

---

#### 3. Update Payload Counts in Documentation
**Location:** `architecture/waf.md:83-87`

**Current (Incorrect):**
```
- `get_sqli_payloads()` - 19 SQL injection payloads
- `get_xss_payloads()` - 18 XSS payloads
- `get_ssrf_payloads()` - 15 SSRF payloads
- `get_command_injection_payloads()` - 16 cmd injection payloads
- `get_traversal_payloads()` - 11 path traversal payloads
```

**Suggested Fix:**
```
- `get_sqli_payloads()` - 19 SQL injection payloads
- `get_xss_payloads()` - 17 XSS payloads
- `get_ssrf_payloads()` - 16 SSRF payloads
- `get_command_injection_payloads()` - 16 cmd injection payloads
- `get_traversal_payloads()` - 10 path traversal payloads
```

**Estimated Impact:** Documentation accuracy.

---

#### 4. Update mod.rs Supported WAFs List
**Location:** `crates/slapper/src/waf/mod.rs:16-21`

**Suggested Fix:** Add the remaining WAFs to match the 34-product count:

```rust
//! ## Supported WAFs
//!
//! Cloudflare, Akamai, AWS WAF, Azure WAF, Google Cloud Armor, Fastly, Imperva,
//! Sucuri, CloudFront, F5 BIG-IP, Barracuda, Fortinet, Citrix NetScaler,
//! ModSecurity, Wordfence, DataDome, PerimeterX, Nginx, Traefik, Kong,
//! Varnish, Radware, Signal Sciences, Wallarm, Reblaze, F5 BIG-IP Advanced WAF,
//! Palo Alto, Qrator, Imunify360, SiteGuard, StackPath WAF, Humanity, Datadog,
//! Generic WAF Block
```

**Estimated Impact:** Documentation accuracy.

---

#### 5. Clarify BypassEngine Architecture Description
**Location:** `crates/slapper/src/waf/mod.rs:43`

**Current:** "BypassEngine - Orchestrates bypass testing across three sub-engines"

**Suggested Fix:** "BypassEngine - Sequentially executes header, evasion, and smuggling bypass techniques"

**Estimated Impact:** Prevents misleading expectations about parallel execution.

---

### LOW Priority

#### 6. Add Constant for Max WAF Count (Maintainability)
**Location:** `crates/slapper/src/waf/data/patterns.rs`

**Suggested Addition:**
```rust
/// Total number of supported WAF products (26 explicit + auto-generated)
pub const SUPPORTED_WAF_COUNT: usize = 34;
```

**Estimated Impact:** Self-documenting code, easier to verify count matches documentation.

---

#### 7. Consider Adding Integration Tests for Payload Counts
**Location:** `crates/slapper/src/waf/payloads/encoding.rs`

**Suggested Test:**
```rust
#[test]
fn test_payload_counts_match_documentation() {
    assert_eq!(get_sqli_payloads().len(), 19);
    assert_eq!(get_xss_payloads().len(), 17);  // Update to match doc
    assert_eq!(get_ssrf_payloads().len(), 16);
    assert_eq!(get_command_injection_payloads().len(), 16);
    assert_eq!(get_traversal_payloads().len(), 10);
}
```

**Estimated Impact:** Prevents future documentation drift.

---

## Summary Table

| Category | Finding | Severity | Priority |
|----------|---------|----------|----------|
| Constant inside loop | `detect.rs:76` | Medium | HIGH |
| HTTP/2 always disabled | `smuggling.rs:298` | Medium | MEDIUM |
| Payload count mismatch | `encoding.rs` (XSS -1, SSRF +1, Traversal -1) | Low | MEDIUM |
| Docstring WAF list incomplete | `mod.rs:16-21` | Low | MEDIUM |
| BypassEngine description misleading | `mod.rs:43` | Low | LOW |
| Memory zeroization delay | `smuggling.rs:365` | Low | LOW |

---

## Conclusion

The WAF module is well-implemented overall. The core detection engine, bypass system, and data structures are solid. The main issues are:

1. **One performance bug** (constant inside loop)
2. **One functional limitation** (HTTP/2 disabled)
3. **Several documentation inaccuracies** (payload counts, WAF list)

The module would benefit from addressing the performance bug and HTTP/2 issue, plus updating documentation to match actual counts. The overall architecture is sound and aligns well with the documented design intent.
