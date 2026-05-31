# WAF Architecture Review

**Document:** architecture/waf.md
**Reviewed:** 2026-05-31
**Accuracy:** High

## Verified Claims
- Module structure matches actual directory layout at `crates/slapper/src/waf/`
- WafDetector, WafDetectionResult, WafSignature, WafEngine, BypassEngine, BypassResult, BypassTechnique, WafProfile, ResponseDiff all exist
- 34 WAF products verified - `signatures.insert()` count is 34 at `crates/slapper/src/waf/data/patterns.rs`
- BypassTechnique enum has 15 variants verified at `crates/slapper/src/waf/bypass/mod.rs:45-61`
- Scoring system verified: Header +25, Cookie +20, Body +15, IP +20, High confidence exit 90 at `crates/slapper/src/constants.rs:70-76`
- Scoring uses `u16` internally verified at `crates/slapper/src/waf/detector/detect.rs:103`
- `get_waf_profiles()` and `get_profile_by_name()` in profiles.rs use static LazyLock verified at `crates/slapper/src/waf/bypass/profiles.rs`
- is_bypass_successful() function verified at `crates/slapper/src/waf/bypass/mod.rs:131-150`
- Blocked codes include 403, 406, 429, 503 verified at `crates/slapper/src/constants.rs:77`
- get_sqli_payloads (19 payloads), get_xss_payloads (17 payloads), get_ssrf_payloads (16 payloads), get_command_injection_payloads (16 payloads), get_traversal_payloads (10 payloads) verified at `crates/slapper/src/waf/payloads/encoding.rs:74-175`
- Five bypass categories (Encodings, Header Manipulation, Payload Splitting, Protocol Obfuscation, HTTP Smuggling) verified via bypass/ directory structure
- HTTP Smuggling via smuggling.rs verified at `crates/slapper/src/waf/bypass/smuggling.rs`
- AI integration with SmartWafBypass verified (feature-gated) at `crates/slapper/src/waf/mod.rs:116,132-140`

## Discrepancies
- [XSS payload count]: Documented as "18 XSS payloads", but actual count is 17 in `get_xss_payloads()` (`crates/slapper/src/waf/payloads/encoding.rs:74-94`)
- [SSRF payload count]: Documented as "15 SSRF payloads", but actual count is 16 in `get_ssrf_payloads()` (`crates/slapper/src/waf/payloads/encoding.rs:120-139`)
- [Traversal payload count]: Documented as "11 path traversal payloads", but actual count is 10 in `get_traversal_payloads()` (`crates/slapper/src/waf/payloads/encoding.rs:162-175`)
- [Supported WAFs list]: Documented list includes "F5 BIG-IP Advanced WAF, Palo Alto, Qrator, Imunify360, SiteGuard, StackPath WAF, Humanity, Datadog, Generic WAF Block" but the module doc lists only 25 names (Cloudflare through Reblaze). The full 34 product list in data/patterns.rs should be cross-referenced. (priority: low)

## Bugs Found
- None identified

## Improvement Opportunities
- [Payload count accuracy]: Update documentation to reflect actual payload counts: XSS=17, SSRF=16, Traversal=10. (priority: low)
- [WAF product list]: Ensure the supported WAFs list in the doc header matches all 34 products defined in data/patterns.rs. (priority: low)

## Stale Items
- None identified
