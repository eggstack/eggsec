# Web Proxy Security Audit (Code-Level)

**Date**: 2026-06-13  
**Auditor**: Automated code review  
**Scope**: `crates/eggsec/src/proxy/intercept/` (all files)  
**Severity Scale**: Critical / High / Medium / Low / Informational

---

## Executive Summary

The web proxy implementation follows defense-in-depth principles appropriate for a defense-lab security testing tool. The primary threat model is: an authorized operator runs the proxy against systems they own or are authorized to test. The operator controls both the proxy configuration and the target systems.

**Overall Assessment**: The implementation is sound for its intended threat model. Two medium-severity findings and three low/informational findings were identified.

---

## Findings

### M-01: Bundle Signature Verification Uses Non-Constant-Time Comparison

**Severity**: Medium  
**Location**: `proxy/intercept/bundle.rs:230`  
**Description**: The `verify()` method uses `mac.verify_slice()` from the `hmac` crate, which does perform constant-time comparison internally. However, the hex encoding/decoding step introduces a potential timing oracle if an attacker can observe the decode step. In practice, the `hmac` crate's `verify_slice` is constant-time, so this is mitigated.  
**Status**: Mitigated by `hmac` crate internals. No action required for defense-lab use.

### M-02: No Path Traversal Protection on File I/O

**Severity**: Medium  
**Location**: `bundle.rs`, `rules.rs`, `types.rs` (various `save_to_file`/`load_from_file`)  
**Description**: File paths for rule persistence, bundle export/import, and session save/load are taken from CLI arguments or configuration without explicit path traversal validation (e.g., rejecting `../../etc/passwd`). Since this is a CLI tool where the operator controls the file paths, this is acceptable within the threat model. However, if the proxy were ever exposed via MCP or network APIs, this would become a high-severity issue.  
**Mitigation**: The `--allow-web-proxy` policy gate and the standalone defense-lab pattern ensure only authorized operators can invoke file operations. MCP exposure (`web-proxy-mcp` feature) does not expose file I/O paths.  
**Recommendation**: If file paths are ever exposed via network APIs in the future, add explicit path canonicalization and allowlist validation.

### L-01: Unbounded Rule Condition Nesting

**Severity**: Low  
**Location**: `rules.rs` (RuleCondition::And/Or/Not)  
**Description**: `RuleCondition::And` and `RuleCondition::Or` accept `Vec<RuleCondition>` without depth limits. Deeply nested conditions (e.g., 1000 levels of `And([And([And([...])])])`) could cause stack overflow during evaluation. In practice, rules come from trusted configuration files and will not have pathological nesting.  
**Recommendation**: Consider adding a `max_depth` check during rule deserialization if untrusted rule input is ever accepted.

### L-02: Certificate Cache Has No Size Limit

**Severity**: Low  
**Location**: `cert.rs:35` (cache: `HashMap<String, CachedCert>`)  
**Description**: The certificate cache is a `HashMap` with no maximum size. A long-running proxy session that connects to many unique hosts will grow the cache unboundedly. In practice, the cache entries are small (DER-encoded cert + key ~2KB each) and certs expire after `validity_duration` (default 24h), so this is unlikely to cause issues.  
**Recommendation**: Consider adding an LRU eviction policy or a max cache size (e.g., 10,000 entries) for very long-running sessions.

### L-03: `try_write` on Certificate Cache May Silently Drop

**Severity**: Low  
**Location**: `cert.rs:112` (`cache_cert`)  
**Description**: `self.cache.try_write()` returns `None` if the lock is held, silently skipping the cache insert. This means a cert generated for a host may not be cached if there's lock contention, causing regeneration on the next connection to the same host. This is a performance issue, not a security issue, since the cert will still be generated correctly.  
**Recommendation**: Use `.write()` (blocking) instead of `.try_write()` for cache insertion, or log a warning when caching is skipped.

### I-01: No Request Body Size Limit in MITM Proxy

**Severity**: Informational  
**Location**: `mod.rs` (handle_http_request, handle_connect_request)  
**Description**: The proxy reads HTTP headers (limited to 4096 bytes via `read_http_headers`) but does not enforce a maximum body size when forwarding traffic. Budget enforcement (`max_bytes_per_flow`) applies to the *captured* body in reports, not to the forwarded traffic. An attacker could send a multi-gigabyte request body that the proxy forwards to the upstream server.  
**Mitigation**: Budget limits apply to capture, not forwarding. The proxy is designed for interactive use by an authorized operator, not as a production reverse proxy.

### I-02: WebSocket Frame Forwarding Is Unfiltered

**Severity**: Informational  
**Location**: `mod.rs:262-310` (handle_websocket_interception)  
**Description**: WebSocket frames are forwarded as-is between client and server without content inspection or size limits. Malicious WebSocket frames (e.g., extremely large payloads) will be forwarded without restriction. This is by design for the interception model.

---

## Security Controls Verified

| Control | Status | Notes |
|---------|--------|-------|
| CRLF injection prevention | ✅ | `validate_header_value` blocks `\r`, `\n`, `\0` in header names and values |
| Private IP blocking | ✅ | `is_private_ip` blocks RFC 1918, loopback, multicast, broadcast (IPv4 and IPv6) |
| TLS certificate generation | ✅ | `rcgen`-based, per-host caching, configurable validity, ALPN support |
| Policy enforcement | ✅ | `EnforcementContext::evaluate()` gates all proxy operations |
| Dry-run safety | ✅ | Zero network activity, zero CA generation, synthetic flows only |
| Budget enforcement | ✅ | Flows, bytes, duration, concurrency limits enforced |
| Scope validation | ✅ | Private IPs blocked; scope rules evaluated before connection |
| Audit trail | ✅ | All manipulations recorded with before/after values, timestamps, reasons |
| HMAC integrity | ✅ | Bundle signing with HMAC-SHA256 (constant-time via `hmac` crate) |
| Thread safety | ✅ | `parking_lot::RwLock` for cert cache and rule sets |

---

## Recommendations Summary

| # | Severity | Recommendation | Priority |
|---|----------|----------------|----------|
| 1 | Low | Add max depth check for rule condition nesting | Low |
| 2 | Low | Add LRU eviction or max size to cert cache | Low |
| 3 | Low | Use blocking `.write()` instead of `.try_write()` for cert cache | Low |
| 4 | Info | Document body forwarding limits vs capture limits | Low |

---

**Conclusion**: No critical or high-severity issues found. The implementation is appropriate for a defense-lab security testing tool. The medium findings (M-01, M-02) are mitigated by the threat model and existing controls. Low findings are performance/robustness improvements, not security vulnerabilities.
