# WAF Module Architecture Review

**Date:** 2026-05-22
**Reviewer:** Architecture Review
**Branch:** architecture/waf-review

---

## 1. Summary: What's Implemented Correctly

The WAF module implementation aligns well with the architecture document. Here are the verified correct aspects:

### 1.1 WAF Product Count (34 products) ✓
- `constants.rs:24` defines `SUPPORTED_WAF_COUNT: usize = 34`
- `constants.rs:31-37` has a test ensuring actual signature count matches
- `data/patterns.rs` contains 34 WAF signature entries verified:
  1. Cloudflare
  2. Akamai
  3. AWS WAF
  4. Azure WAF
  5. Google Cloud Armor
  6. Fastly
  7. Imperva
  8. Sucuri
  9. CloudFront
  10. F5 BIG-IP
  11. Barracuda
  12. Fortinet
  13. Citrix NetScaler
  14. ModSecurity
  15. Wordfence
  16. DataDome
  17. PerimeterX
  18. Nginx
  19. Traefik
  20. Kong
  21. Varnish
  22. Radware
  23. Signal Sciences
  24. Wallarm
  25. Reblaze
  26. F5 BIG-IP Advanced WAF
  27. Palo Alto
  28. Qrator
  29. Imunify360
  30. SiteGuard
  31. StackPath WAF
  32. Humanity
  33. Datadog
  34. Generic WAF Block

### 1.2 Scoring System Uses u16 ✓
- `constants.rs:69-90` defines all WAF constants as `u16`:
  - `HEADER_MATCH_SCORE: u16 = 25`
  - `COOKIE_MATCH_SCORE: u16 = 20`
  - `BODY_MATCH_SCORE: u16 = 15`
  - `IP_MATCH_SCORE: u16 = 20` (correctly added per recent bug fix)
  - `HIGH_CONFIDENCE_EXIT: u16 = 90`
- `detector/detect.rs:71` uses `u16` for score accumulator

### 1.3 FxHashMap/FxHashSet Usage ✓
The module correctly uses `rustc_hash` types throughout:
- `detector/mod.rs:13,20-21`: `WafDetector` uses `FxHashMap`
- `detector/types.rs:2`: `ResponseDiff` uses `FxHashMap`
- `detector/compare.rs:3`: `FxHashMap` for header collection
- `bypass/profiles.rs:3`: `FxHashSet` for existing profile names
- `data/patterns.rs:1`: `FxHashMap` for signatures

### 1.4 LazyLock for Profile Caching ✓
- `bypass/profiles.rs:22-36` uses `static WAF_PROFILES: LazyLock<Vec<WafProfile>>`
- `get_waf_profiles()` at line 38 returns reference to cached profiles
- `get_profile_by_name()` at line 42 uses cached profiles

### 1.5 Bypass Success Detection Logic ✓
- `bypass/mod.rs:129-166` implements `is_bypass_successful()` correctly checking:
  1. Response status NOT in blocked codes (403, 406, 429, 503)
  2. Response status differs from baseline
  3. Response status is 2xx (200-299)
  4. Payload (or URL-encoded version) is reflected in response body

### 1.6 Documentation Accuracy
- `waf.md:51` correctly states 34 WAF products
- `waf.md:55-59` correctly describes u16 scoring system
- `waf.md:64` correctly describes LazyLock profile caching
- `waf.md:74-78` correctly describes bypass success detection logic

---

## 2. Bugs/Issues Found

### 2.1 Test File Unwrap Usage (Low Severity)
**File:** `waf/detector/tests.rs:210-211, 229-230`

In test code, `unwrap()` is used on serialization operations. While this is in test code and won't cause production failures, it's inconsistent with the error handling patterns in the codebase.

```rust
let json = serde_json::to_string(&result).unwrap();
let deserialized: WafDetectionResult = serde_json::from_str(&json).unwrap();
```

**Recommendation:** Consider using `unwrap_or_else` with descriptive panic messages, or use test utility functions that return `Result` types. However, since this is test code only, this is low priority.

### 2.2 Minor Documentation Discrepancy
**File:** `waf.md:45` vs `bypass/mod.rs:44-60`

The architecture document states "15 bypass techniques" and `BypassTechnique` enum has 15 variants - documentation is correct.

### 2.3 Supported WAF List Discrepancy (Minor)
**File:** `waf.md:93-94` lists 34 WAF products but `mod.rs:18-21` documentation only lists 25 names.

The module documentation in `mod.rs:18-21`:
```
Cloudflare, Akamai, AWS WAF, Azure WAF, Google Cloud Armor, Fastly, Imperva,
Sucuri, CloudFront, F5 BIG-IP, Barracuda, Fortinet, Citrix NetScaler,
ModSecurity, Wordfence, DataDome, PerimeterX, Nginx, Traefik, Kong,
Varnish, Radware, Signal Sciences, Wallarm, Reblaze
```

That's only 25 names. The full 34 are in `data/patterns.rs`. The module docstring should be updated to reflect all 34 products.

---

## 3. Recommended Fixes

### 3.1 Update Module Documentation
**File:** `crates/slapper/src/waf/mod.rs:16-21`

Update the module docstring to list all 34 WAF products:

```rust
/// ## Supported WAFs
///
/// Cloudflare, Akamai, AWS WAF, Azure WAF, Google Cloud Armor, Fastly, Imperva,
/// Sucuri, CloudFront, F5 BIG-IP, Barracuda, Fortinet, Citrix NetScaler,
/// ModSecurity, Wordfence, DataDome, PerimeterX, Nginx, Traefik, Kong,
/// Varnish, Radware, Signal Sciences, Wallarm, Reblaze, F5 BIG-IP Advanced WAF,
/// Palo Alto, Qrator, Imunify360, SiteGuard, StackPath WAF, Humanity, Datadog,
/// Generic WAF Block
```

### 3.2 Consider Test Error Handling (Optional)
**File:** `waf/detector/tests.rs:210-211, 229-230`

While acceptable for test code, consider this pattern for consistency:
```rust
let json = serde_json::to_string(&result)
    .unwrap_or_else(|e| panic!("Failed to serialize WafDetectionResult: {}", e));
```

---

## 4. Notes on Discrepancies

| Item | Architecture Doc | Actual Implementation | Status |
|------|------------------|---------------------|--------|
| WAF count | 34 | 34 | ✓ Match |
| Scoring u16 | Yes (line 55-59) | Yes (constants.rs:69-90) | ✓ Match |
| LazyLock caching | Yes (line 64) | Yes (profiles.rs:22) | ✓ Match |
| Bypass detection | 4-point check | 4-point check (mod.rs:129-166) | ✓ Match |
| FxHashMap usage | Yes | Yes | ✓ Match |
| Module docstring | Lists 25 | Actually 34 | ⚠ Update needed |

---

## 5. Verification Commands

To verify the WAF module is working correctly:

```bash
# Check WAF product count
cargo test --lib -p slapper supported_waf_count_matches_actual

# Run WAF module tests
cargo test --lib -p slapper -- waf

# Verify constants are u16
grep -n "u16" crates/slapper/src/constants.rs | grep waf

# Verify FxHashMap usage
grep -rn "FxHashMap\|FxHashSet" crates/slapper/src/waf/
```

---

## 6. Conclusion

The WAF module implementation is **well-aligned** with the architecture document. All major claims are verified:

- ✓ 34 WAF products correctly implemented
- ✓ u16 scoring system prevents overflow
- ✓ FxHashMap/FxHashSet used for performance
- ✓ LazyLock profile caching implemented
- ✓ Bypass success detection correctly implemented
- ✓ No unwrap/expect in production code (only in tests)

**One minor issue:** The module docstring in `mod.rs:18-21` only lists 25 WAF products instead of the full 34. This should be updated for consistency.
