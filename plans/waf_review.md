# WAF Module Architecture Review

## Overview

Reviewed `architecture/waf.md` against implementation in `crates/slapper/src/waf/` and supporting code.

---

## Verified Claims

### 1. Core Components Structure
**Claim**: Directory structure matches documentation.
**Verdict**: ✅ Verified - all files present as documented.

### 2. WAF Signature Count (34 products)
**Claim**: `data/patterns.rs` contains 34 WAF signatures.
**Verdict**: ✅ Verified - `constants::SUPPORTED_WAF_COUNT = 34` with test validation.

### 3. FxHashMap Usage
**Claim**: WAF patterns stored in `FxHashMap<String, WafSignature>`.
**Verdict**: ✅ Verified - `patterns.rs:13` uses `LazyLock<FxHashMap<String, WafSignature>>`.

### 4. Scoring System (u16 to prevent overflow)
**Claim**: Scoring uses `u16` internally with specific point values.
**Verdict**: ✅ Verified - `constants.rs:69-76` defines:
- Header match: +25 (`HEADER_MATCH_SCORE: u16 = 25`)
- Cookie match: +20 (`COOKIE_MATCH_SCORE: u16 = 20`)
- Body pattern: +15 (`BODY_MATCH_SCORE: u16 = 15`)
- IP match: +20 (`IP_MATCH_SCORE: u16 = 20`)
- High confidence exit: 90 (`HIGH_CONFIDENCE_EXIT: u16 = 90`)

### 5. LazyLock for WAF Profiles
**Claim**: `profiles.rs` uses `LazyLock<Vec<WafProfile>>` to cache profiles.
**Verdict**: ✅ Verified - `profiles.rs:22` uses `static WAF_PROFILES: LazyLock<Vec<WafProfile>>`.

### 6. BypassTechnique Count (15 techniques)
**Claim**: Enum of 15 bypass techniques.
**Verdict**: ✅ Verified - `mod.rs:43-60` defines 15 variants:
HeaderManipulation, UserAgentRotation, XForwardedForSpoof, ContentTypeBypass, EncodingBypass, Homoglyph, ZeroWidthInjection, CaseRotation, UnicodeEncoding, CommentObfuscation, WhitespaceVariation, ChunkedEncoding, ContentLengthConflict, TransferEncodingConflict, DoubleEncoding.

### 7. is_bypass_successful Logic
**Claim**: Function checks blocked codes, status diff, 2xx status, payload reflection.
**Verdict**: ✅ Verified - `mod.rs:129-155` implements all 4 checks.

### 8. Payload Functions
**Claim**: Returns specific payload counts.
**Verdict**: ✅ Verified:
- `get_sqli_payloads()`: 19 items (`encoding.rs:96-118`)
- `get_xss_payloads()`: 18 items (`encoding.rs:74-94`)
- `get_ssrf_payloads()`: 15 items (`encoding.rs:120-139`)
- `get_command_injection_payloads()`: 16 items (`encoding.rs:141-160`)
- `get_traversal_payloads()`: 11 items (`encoding.rs:162-175`)

---

## Discrepancies

### 1. BypassEngine Name Mismatch
**Doc Claim**: `bypass/mod.rs` exports `BypassEngine`.
**Verdict**: ❌ **Discrepancy** - `bypass/mod.rs:72` defines `pub struct BypassEngine`, but `mod.rs:88` re-exports it via `pub use bypass::{..., BypassEngine}`. The structure is correct but the doc implies direct definition location.

### 2. WafEngine Location
**Doc Claim**: `mod.rs` has `WafEngine` as "High-level orchestrator".
**Verdict**: ⚠️ **Partially accurate** - `WafEngine` is defined in `waf/mod.rs:108`, but the doc doesn't mention it's only accessible via `WafEngine` (not re-exported at module root).

---

## Bugs Found

### Bug 1: Cookie Name Extraction Bug in detect.rs
**File**: `detector/detect.rs:92-99`
```rust
let cookie_name = cookie_header
    .split(';')
    .next()
    .unwrap_or("")
    .split('=')
    .next()
    .unwrap_or("")
    .trim();
```
**Issue**: Splits on `;` then `=`, but `split('=').next()` on `"foo=bar; baz=qux"` returns `"foo"` (correct), BUT if the cookie has no `=`, it returns the whole string which is then trimmed. More critically, if the cookie value contains `;` (valid base64 or JSON), we get wrong name.

**Example**: `session=eyJhbGciOiOiJ7O3=2}` → name extracted as `"session"` - actually works.

But for `__cfduid=abc123; HttpOnly; Secure`, `split('=').next()` gives `"__cfduid"` - correct.

However, cookie like `data=foo;bar;baz` would extract `"data"` - correct.

The real bug: It doesn't handle trailing `;` properly. Cookie `foo=bar;` (trailing semicolon) gives name `"foo"` - still correct. But `;foo=bar` (invalid) gives `""` - OK.

Actually looking closer: `split('=').next().unwrap_or("").trim()` - if cookie is `foo` (no value), this returns `"foo"`. That's fine since pattern match checks for containment.

**Severity**: Low - edge case handling works correctly for well-formed cookies.

### Bug 2: Signatures Lower Clone Inefficiency
**File**: `detector/mod.rs:34-46`
```rust
let signatures_lower = signatures
    .iter()
    .map(|(key, sig)| {
        (
            key.clone(),
            WafSignatureLower {
                headers: sig.headers.iter().map(|h| h.to_lowercase()).collect(),
                ...
            },
        )
    })
    .collect();
```
**Issue**: `key.clone()` creates String copies for every key. Should use `signatures.iter().map(|(key, sig)| ...)` with `key` reference, or restructure to avoid clone.

**Severity**: Medium - startup cost for 34 signatures is negligible but anti-pattern.

### Bug 3: Header Value Length Check Uses Magic Number
**File**: `detector/detect.rs:81`
```rust
value_lower.contains(header_pattern_lower.as_str())
    && value_lower.len() <= 256;
```
**Issue**: Magic number 256 not defined as constant. Hard to discover and maintain.

**Severity**: Low - not a bug per se, but inconsistent with other constants.

---

## Improvement Opportunities

### 1. HIGH: Profile Auto-Detection Fuzzy Match is Slow
**File**: `waf/mod.rs:151-162`
```rust
for profile in bypass::get_waf_profiles() {
    for sig in &profile.detection_signatures {
        let sig_lower = sig.to_lowercase();
        if waf_lower == sig_lower
            || waf_lower.starts_with(&sig_lower)
            || waf_lower.ends_with(&sig_lower)
            || waf_lower.contains(&format!(" {}", &sig_lower))
        }
    }
}
```
**Issue**: Linear scan with 4 string comparisons per signature. For 34 WAFs with avg 4 signatures each = 136 comparisons.

**Suggestion**: Build HashMap of signature → profile name during profile initialization. Use exact match first, then prefix/suffix only if needed.

### 2. MEDIUM: SmugglingBypass Ignores Client Parameter
**File**: `bypass/smuggling.rs:48`
```rust
pub async fn run(
    &self,
    _client: &Client,  // <-- Unused!
    url: &str,
    detection: &WafDetectionResult,
) -> Result<Vec<BypassResult>>
```
**Issue**: `_client` parameter accepted but not used. Smuggling uses raw TCP/TLS instead. Not a bug but confusing API.

**Suggestion**: Remove unused parameter or document why client is passed but not used.

### 3. MEDIUM: EvasionBypass Generates Redundant Payloads
**File**: `bypass/evasion.rs:101-157`
```rust
for sqli in get_sqli_payloads() {
    payloads.push((BypassTechnique::CaseRotation, apply_case_rotation(sqli), ...));
}
for sqli in get_sqli_payloads() {  // <-- Iterates again
    payloads.push((BypassTechnique::Homoglyph, apply_homoglyphs(sqli), ...));
}
```
**Issue**: Calls `get_sqli_payloads()` 7 times for SQLi alone, each returning 19 items. Creates 133 SQLi variants (19×7) but many are duplicates across techniques.

**Suggestion**: Generate payloads once per technique type, reuse where applicable.

### 4. LOW: get_waf_signatures Returns Clone
**File**: `data/patterns.rs:656-657`
```rust
pub fn get_waf_signatures() -> FxHashMap<String, WafSignature> {
    WAF_SIGNATURES.clone()
}
```
**Issue**: Every call clones the entire map (34 entries). Called at minimum in `detector/mod.rs:33` for every WafDetector creation.

**Suggestion**: Return `&'static FxHashMap<String, WafSignature>` instead to avoid clone.

### 5. LOW: WafDetector Stores Both Original and Lowercase Signatures
**File**: `detector/mod.rs:20-21`
```rust
signatures: FxHashMap<String, WafSignature>,
signatures_lower: FxHashMap<String, WafSignatureLower>,
```
**Issue**: Doubles memory for 34 signatures. `WafSignatureLower` duplicates headers/cookies/body_patterns as lowercase strings.

**Suggestion**: Store only lowercase versions, convert original to lowercase on demand or during signature load.

### 6. LOW: Missing Error Context in BypassResult
**File**: `bypass/mod.rs:63-70`
```rust
pub struct BypassResult {
    pub technique: BypassTechnique,
    pub success: bool,
    pub description: String,
    pub payload: Option<String>,
    pub status_code: u16,
    pub response_diff: Option<i64>,
}
```
**Issue**: Network errors produce `status_code: 0` with generic description. No way to distinguish timeout from connection refused from DNS failure.

**Suggestion**: Add optional `error: Option<String>` field to capture network error details.

---

## Priority Summary

| Finding | Type | Priority |
|---------|------|----------|
| Profile auto-detection linear scan | Performance | HIGH |
| Signatures lower clone inefficiency | Performance | MEDIUM |
| SmugglingBypass unused client param | API Design | MEDIUM |
| EvasionBypass redundant payload gen | Performance | MEDIUM |
| get_waf_signatures returns clone | Performance | LOW |
| WafDetector stores dual signatures | Memory | LOW |
| Missing error context in BypassResult | Debugging | LOW |
| Magic number 256 in header check | Maintainability | LOW |

---

## Testing Coverage

- 54 tests pass for WAF module
- Signature count validated against constant
- All signatures have at least one indicator
- IP ranges validated format (contain `/`)
- All signature names unique
- Blocked status codes include 403, 406, 429, 503

---

## Recommendations

1. **HIGH**: Refactor `select_profile()` in `waf/mod.rs` to use HashMap-based lookup instead of linear scan
2. **MEDIUM**: Change `get_waf_signatures()` to return static reference
3. **MEDIUM**: Change `get_waf_profiles()` to return static reference (already LazyLock but function clones)
4. **LOW**: Add `error` field to `BypassResult` for better error reporting
5. **LOW**: Remove unused `_client` parameter from `SmugglingBypass::run()` or document why it exists