# WAF Architecture Review

**Document:** architecture/waf.md
**Reviewed:** 2026-05-31
**Accuracy:** High
**Lines Reviewed:** 95

## Verified Claims

| Claim | Status | Evidence |
|-------|--------|----------|
| 34 WAF products supported | ✅ Verified | `crates/slapper/src/waf/data/patterns.rs` - 34 `signatures.insert()` calls |
| `WafSignature` has headers, cookies, body_patterns, ip_ranges | ✅ Verified | `crates/slapper/src/waf/data/patterns.rs:4-11` - struct definition |
| Scoring: Header +25, Cookie +20, Body +15, IP +20 | ✅ Verified | `crates/slapper/src/constants.rs:70-73` |
| High confidence exit at 90 points | ✅ Verified | `crates/slapper/src/constants.rs:76` - `HIGH_CONFIDENCE_EXIT: u16 = 90` |
| 15 BypassTechnique variants | ✅ Verified | `crates/slapper/src/waf/bypass/mod.rs:44-61` - 15 enum variants |
| Five bypass categories: Encodings, Header Manipulation, Payload Splitting, Protocol Obfuscation, HTTP Smuggling | ✅ Partially verified | Header, Evasion, Smuggling modules exist. "Payload Splitting" and "Protocol Obfuscation" are subsumed by `EvasionBypass` in `bypass/evasion.rs` |
| `WafProfile` with `LazyLock<Vec<WafProfile>>` | ✅ Verified | `crates/slapper/src/waf/bypass/profiles.rs:23` - `static WAF_PROFILES: LazyLock<Vec<WafProfile>>` |
| `get_waf_profiles()` and `get_profile_by_name()` | ✅ Verified | `crates/slapper/src/waf/bypass/profiles.rs:50-64` |
| `is_bypass_successful()` checks | ✅ Verified (partial) | `crates/slapper/src/waf/bypass/mod.rs:131-164` - see Discrepancies |
| Blocked codes: 403, 406, 429, 503 | ✅ Verified | `crates/slapper/src/constants.rs:77` - `[403, 406, 429, 503]` |
| `get_sqli_payloads()` - 19 payloads | ✅ Verified | `crates/slapper/src/waf/payloads/encoding.rs:96-117` - 19 items |
| `get_xss_payloads()` - 17 payloads | ✅ Verified | `crates/slapper/src/waf/payloads/encoding.rs:74-93` - 17 items |
| `get_ssrf_payloads()` - 16 payloads | ✅ Verified | `crates/slapper/src/waf/payloads/encoding.rs:120-138` - 16 items |
| `get_command_injection_payloads()` - 16 payloads | ✅ Verified | `crates/slapper/src/waf/payloads/encoding.rs:141-159` - 16 items |
| `get_traversal_payloads()` - 10 payloads | ✅ Verified | `crates/slapper/src/waf/payloads/encoding.rs:162-174` - 10 items |
| `WafDetectionResult` with confidence 0-100 | ✅ Verified | `crates/slapper/src/waf/detector/detect.rs:205` - `conf = score.min(100) as u8` |
| `ResponseDiff` and `is_waf_blocked()` | ✅ Verified | `crates/slapper/src/waf/detector/types.rs:25,37` |
| `WafSignatureLower` type | ✅ Verified | `crates/slapper/src/waf/detector/types.rs:18` |
| File tree matches source | ✅ Verified | Directory listing of `waf/` matches document structure |
| Supported WAFs list (34 products) | ✅ Verified | All 34 names match `signatures.insert()` keys in `data/patterns.rs` |

## Discrepancies

### 1. `is_bypass_successful()` Omits `body_looks_blocked()` Check

**Severity:** Low

The document states `is_bypass_successful()` verifies 4 conditions:
1. Response status NOT in blocked codes (403, 406, 429, 503)
2. Response status differs from baseline
3. Response status is 2xx (200-299)
4. Payload (or URL-encoded version) is reflected in response body

The actual implementation (`crates/slapper/src/waf/bypass/mod.rs:131-164`) also checks:
- `body_looks_blocked(response_body)` - response body contains blocked patterns (line 141)
- `response_diff.is_waf_blocked()` - diff indicates WAF blocking (lines 146-149)

These are additional bypass failure conditions not documented.

**Evidence:**
- `crates/slapper/src/waf/bypass/mod.rs:141` - `body_looks_blocked(response_body)`
- `crates/slapper/src/waf/bypass/mod.rs:146-149` - `response_diff.is_waf_blocked()`
- `crates/slapper/src/waf/bypass/mod.rs:177-181` - `body_looks_blocked()` checks for "access denied", "blocked", "firewall", etc.

### 2. Five Bypass Categories: Documentation vs Implementation Mismatch

**Severity:** Low

The document lists 5 bypass categories:
1. Encodings
2. Header Manipulation
3. Payload Splitting
4. Protocol Obfuscation
5. HTTP Smuggling

The implementation has 3 bypass modules:
- `HeaderBypass` (headers.rs) - handles Header Manipulation
- `EvasionBypass` (evasion.rs) - handles Encodings, and likely Payload Splitting and Protocol Obfuscation
- `SmugglingBypass` (smuggling.rs) - handles HTTP Smuggling

The document's "Payload Splitting" and "Protocol Obfuscation" categories are not separate modules but are likely sub-techniques within `EvasionBypass`. The `BypassTechnique` enum (`bypass/mod.rs:44-61`) has 15 variants that span these categories, but the mapping is not 1:1 with the documented categories.

**Evidence:**
- `crates/slapper/src/waf/bypass/mod.rs:1-4` - only 3 bypass modules: `evasion`, `headers`, `smuggling`
- `crates/slapper/src/waf/bypass/mod.rs:44-61` - `BypassTechnique` enum with 15 variants

### 3. Blocked Codes Include 503: Document Says "403, 406, 429, 503" but Fuzzer Uses Different Set

**Severity:** Informational

The document correctly states the WAF module's blocked codes include 503 (`constants.rs:77`). However, the fuzzer module uses a separate constant with only 3 codes (`engine/utils.rs:18: [403, 406, 429]`). This divergence is not documented.

**Evidence:**
- `crates/slapper/src/constants.rs:77` - `[403, 406, 429, 503]`
- `crates/slapper/src/fuzzer/engine/utils.rs:18` - `[403, 406, 429]`

## Bugs

No bugs found in the document. All code references are accurate.

## Improvements

### 1. Document the `body_looks_blocked()` Function

The `body_looks_blocked()` function at `bypass/mod.rs:177-181` checks response body for blocked patterns. This is a significant bypass failure condition that should be documented alongside the status code check.

### 2. Clarify Bypass Category Mapping

The document's 5 bypass categories don't map cleanly to the 3 implementation modules. Consider documenting how `EvasionBypass` handles multiple categories (Encodings, Payload Splitting, Protocol Obfuscation) or listing the 15 `BypassTechnique` variants as the canonical bypass taxonomy.

## Stale Items

No stale items found. All claims match current codebase state.
