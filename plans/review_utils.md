# Utils Module Architecture Review

**Document:** architecture/utils.md
**Reviewed:** 2026-06-02
**Accuracy:** Medium
**Lines Reviewed:** 38

## Verified Claims
- [23 submodules exist in utils directory]: Verified - directory has 23 entries: auth.rs, cache.rs, circuit_breaker.rs, client_pool.rs, error.rs, formatting.rs, http.rs, logging.rs, mod.rs, network.rs, output.rs, parsing.rs, privilege.rs, progress.rs, rate_limiter.rs, redaction.rs, scope.rs, serialization.rs, service_detection.rs, stealth.rs, target.rs, urlencoding.rs, validation.rs

## Discrepancies
- [Table lists 23 modules but only shows 21 in the listing]: Document shows modules: auth, cache, circuit_breaker, client_pool, error, formatting, http, logging, network, output, parsing, progress, rate_limiter, redaction, scope, service_detection, stealth, target, urlencoding, validation, privilege - that's 21 modules listed, not 23. Missing: `serialization` and `error` is listed but doesn't appear in the table's column
- [serialization module not listed in table]: The utils directory has `serialization.rs` but the architecture document's table does not include it

## Bugs Found
- None

## Improvement Opportunities
- [High]: Document incorrectly states "23 files" in subtitle but only lists 21 modules in the table. Should be corrected for accuracy
- [Medium]: Missing `serialization` module from the table - this module should be added
- [Low]: The `privilege` module is listed as "(feature-gated: `stress-testing` or `packet-inspection`)" but this feature gate condition should match exactly how it's declared in mod.rs line 51-52

## Stale Items
- None

## Code Interrogation Findings
- [Info]: The privilege module is feature-gated with `#[cfg(any(feature = "stress-testing", feature = "packet-inspection"))]` (mod.rs:51-52)
- [Info]: Key re-exports verified at utils/mod.rs:54-79:
  - CircuitBreaker, CircuitState from circuit_breaker (line 55)
  - ClientPool, OptimizedClientPool from client_pool (line 56)
  - strip_controls, preserve_all from formatting (line 57)
  - create_http_client family from http (lines 58-62)
- [Info]: Error module exists in utils (error.rs) but is NOT re-exported in mod.rs public API - it appears to be internal