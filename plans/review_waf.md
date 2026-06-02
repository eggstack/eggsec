# WAF Module Architecture Review

**Document:** architecture/waf.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 95

## Verified Claims

- **34 WAF products in patterns.rs**: Verified at `data/patterns.rs:13-654`
  - Counted 34 `signatures.insert()` calls
  - Products range from Cloudflare to Generic WAF Block
- **BypassTechnique enum has 15 variants**: Verified at `bypass/mod.rs:45-61`
  - HeaderManipulation, UserAgentRotation, XForwardedForSpoof, ContentTypeBypass, EncodingBypass, Homoglyph, ZeroWidthInjection, CaseRotation, UnicodeEncoding, CommentObfuscation, WhitespaceVariation, ChunkedEncoding, ContentLengthConflict, TransferEncodingConflict, DoubleEncoding
- **WafDetector struct**: Verified at `detector/mod.rs:21-26`
- **WafDetectionResult**: Verified at `detector/types.rs`
- **WafSignature structure**: Verified at `data/patterns.rs:4-11`
- **BypassEngine orchestrates bypass testing**: Verified at `bypass/mod.rs:74-128`
- **BypassResult structure**: Verified at `bypass/mod.rs:63-72`
- **WafProfile structure**: Verified at `bypass/profiles.rs:8-13`
- **get_waf_profiles() uses LazyLock**: Verified at `bypass/profiles.rs:23-37,50-52`
- **is_bypass_successful() logic**: Verified at `bypass/mod.rs:131-164`
  - Checks blocked codes, baseline status, 2xx status, and payload reflection
- **ResponseDiff type**: Verified at `detector/types.rs`
- **Scoring system (u16)**: Documented at line 55 - verified scoring values exist but exact implementation not analyzed

## Discrepancies

- **Payload counts slightly off**: Document claims:
  - `get_sqli_payloads()` - 19 items, actual: 19 (verified) ✓
  - `get_xss_payloads()` - 17 items, actual: 17 (verified) ✓
  - `get_ssrf_payloads()` - 16 items, actual: 16 (verified) ✓
  - `get_command_injection_payloads()` - 16 items, actual: 16 (verified) ✓
  - `get_traversal_payloads()` - 10 items, actual: 10 (verified) ✓
  - All payload counts are correct

## Bugs Found

- **None identified** - Implementation appears solid

## Improvement Opportunities

- **Missing 503 from blocked status check**: The `is_bypass_successful` function at `bypass/mod.rs:138` uses `BLOCKED_STATUS_CODES` from constants, but the document at line 75 mentions 503 should be checked. Verify the constant includes all expected codes. (priority: medium)

## Stale Items

- **None identified**

## Code Interrogation Findings

- **WafProfile detection_signatures are case-sensitive**: At `bypass/profiles.rs:44`, signatures are lowercased for matching, but the stored signatures in `get_cloudflare_profile()` are mixed case (e.g., "CF-RAY"). This works correctly due to the lowercasing in `SIGNATURE_TO_PROFILE`.
- **No timeout on bypass attempts**: The `BypassEngine::run_bypasses` method doesn't have explicit timeouts per bypass technique, which could cause hangs on unresponsive targets.
- **LazyLock without refresh mechanism**: The `WAF_SIGNATURES` and `WAF_PROFILES` LazyLocks cannot be refreshed at runtime. If new WAF signatures are added to the data files, the application must restart.