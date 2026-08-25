# Constants

## Overview

Centralized constants for the Eggsec workspace. Canonical definitions live in `crates/eggsec-core/src/constants.rs` (zero internal dependencies). The engine crate re-exports them via `pub use eggsec_core::constants::*` at `crates/eggsec/src/constants.rs:12`, preserving existing `crate::constants::*` paths. The engine crate also contains a compile-time validation test for `SUPPORTED_WAF_COUNT`.

Related: [types.md](types.md), [error.md](error.md), [overview.md](overview.md).

---

## Location & Feature Gating

| Scope | File | Notes |
|-------|------|-------|
| Canonical definitions | `crates/eggsec-core/src/constants.rs` | Zero workspace deps |
| Engine re-export | `crates/eggsec/src/constants.rs:12` | `pub use eggsec_core::constants::*` |
| Compile-time WAF count test | `crates/eggsec/src/constants.rs:14-25` | `#[cfg(test)]` only |

No feature gating on any constants — they are always compiled.

---

## Top-Level Constants

All defined in `crates/eggsec-core/src/constants.rs`.

### Identity & Config

| Constant | Type | Value | Line | Description |
|----------|------|-------|------|-------------|
| `PROJECT_QUALIFIER` | `&str` | `"tools"` | `:7` | Project qualifier |
| `PROJECT_NAME` | `&str` | `"eggsec"` | `:8` | Project name |
| `DEFAULT_EXPORT_DIR` | `&str` | `"./exports"` | `:10` | Default export directory |
| `DEFAULT_CONFIG_FILE` | `&str` | `"eggsec.toml"` | `:14` | Default config filename |

### Network & Connection

| Constant | Type | Value | Line | Description |
|----------|------|-------|------|-------------|
| `DEFAULT_REMOTE_PORT` | `u16` | `7890` | `:12` | Default remote port |
| `DEFAULT_MAX_RETRIES` | `u32` | `3` | `:16` | Default max retries |
| `DEFAULT_RETRY_DELAY_MS` | `u64` | `1000` | `:17` | Default retry delay (ms) |
| `DEFAULT_POOL_IDLE_TIMEOUT_SECS` | `u64` | `30` | `:19` | Connection pool idle timeout (secs) |
| `DEFAULT_POOL_MAX_IDLE_PER_HOST` | `usize` | `20` | `:20` | Max idle connections per host |
| `MAX_REQUESTS_PER_SECOND_LIMIT` | `u32` | `10000` | `:32` | Max requests/sec limit |

### Timeouts

| Constant | Type | Value | Line | Description |
|----------|------|-------|------|-------------|
| `DEFAULT_TOOL_TIMEOUT_MS` | `u64` | `30000` | `:22` | Default tool timeout (30s) |
| `DEFAULT_BROWSER_TIMEOUT_MS` | `u64` | `60000` | `:23` | Browser timeout (60s) |
| `BROWSER_TIMEOUT_BUFFER_MS` | `u64` | `10000` | `:24` | Browser timeout buffer (10s) |
| `DEFAULT_PROXY_TIMEOUT_MS` | `u64` | `10000` | `:25` | Default proxy timeout (10s) |

### Task Queue & Agent

| Constant | Type | Value | Line | Description |
|----------|------|-------|------|-------------|
| `DEFAULT_TASK_QUEUE_CAPACITY` | `usize` | `10000` | `:27` | Task queue capacity |
| `WORKER_STALE_TIMEOUT_SECS` | `i64` | `90` | `:28` | Worker stale timeout (secs) |
| `DEFAULT_LEASE_DURATION_MS` | `u64` | `300000` | `:29` | Agent lease duration (5 min) |
| `DEFAULT_SCHEDULER_RETRY_DELAY_MS` | `u64` | `30000` | `:30` | Scheduler retry delay (30s) |

### HTTP Status Codes

| Constant | Type | Value | Line | Description |
|----------|------|-------|------|-------------|
| `STATUS_RATE_LIMITED` | `u16` | `429` | `:34` | HTTP 429 Too Many Requests |
| `STATUS_FORBIDDEN` | `u16` | `403` | `:35` | HTTP 403 Forbidden |
| `STATUS_LOCKED` | `u16` | `423` | `:36` | HTTP 423 Locked |
| `STATUS_SERVER_ERROR` | `u16` | `503` | `:37` | HTTP 503 Service Unavailable |

### WAF

| Constant | Type | Value | Line | Description |
|----------|------|-------|------|-------------|
| `SUPPORTED_WAF_COUNT` | `usize` | `34` | `:39` | Number of WAF detector signatures |

---

## Nested Modules

### http (`constants.rs:41-45`)

| Constant | Type | Value | Description |
|----------|------|-------|-------------|
| `DEFAULT_TIMEOUT_SECS` | `u64` | `30` | HTTP request timeout |
| `DEFAULT_MAX_REDIRECTS` | `u32` | `10` | Max redirect hops |
| `DEFAULT_CONCURRENCY` | `usize` | `10` | Default concurrent requests |

### scan (`constants.rs:47-49`)

| Constant | Type | Value | Description |
|----------|------|-------|-------------|
| `DEFAULT_PORT_CONCURRENCY` | `usize` | `100` | Concurrent port scan workers |

### cache (`constants.rs:51-53`)

| Constant | Type | Value | Description |
|----------|------|-------|-------------|
| `DEFAULT_TTL_SECS` | `u64` | `3600` | Cache TTL (1 hour) |

### waf (`constants.rs:55-79`)

**Scoring constants**:

| Constant | Type | Value | Line | Description |
|----------|------|-------|------|-------------|
| `MAX_REDIRECTS` | `usize` | `5` | `:56` | Max redirects during WAF probing |
| `HEADER_MATCH_SCORE` | `u16` | `25` | `:57` | Score for WAF header match |
| `COOKIE_MATCH_SCORE` | `u16` | `20` | `:58` | Score for WAF cookie match |
| `BODY_MATCH_SCORE` | `u16` | `15` | `:59` | Score for WAF body match |
| `IP_MATCH_SCORE` | `u16` | `20` | `:60` | Score for WAF IP match |
| `UNKNOWN_WAF_CONFIDENCE` | `u16` | `30` | `:61` | Confidence for unknown WAF |
| `LENGTH_DIFF_THRESHOLD` | `usize` | `100` | `:62` | Block-page length diff threshold |
| `HIGH_CONFIDENCE_EXIT` | `u16` | `90` | `:63` | Early exit threshold for high confidence |

**Detection patterns**:

| Constant | Type | Value | Line | Description |
|----------|------|-------|------|-------------|
| `BLOCKED_STATUS_CODES` | `[u16; 4]` | `[403, 406, 429, 503]` | `:64` | Status codes indicating block |
| `BLOCKED_PATTERNS` | `[&str; 8]` | `["access denied", "request blocked", "your request has been blocked", "malicious request", "security policy violation", "forbidden", "waf", "firewall"]` | `:65-74` | Body patterns indicating block |
| `WEAK_BLOCK_INDICATOR_PATTERNS` | `[&str; 4]` | `["security", "unauthorized", "suspicious", "rate limit"]` | `:75-76` | Weak indicators (ambiguous) |
| `UNKNOWN_WAF_WEAK_PATTERN_THRESHOLD` | `usize` | `2` | `:77` | Min weak patterns for unknown WAF match |

**Smuggling timeouts**:

| Constant | Type | Value | Line | Description |
|----------|------|-------|------|-------------|
| `SMUGGLING_TIMEOUT_SECS` | `u64` | `15` | `:78` | HTTP smuggling test timeout (secs) |
| `SMUGGLING_TIMEOUT_MS` | `u64` | `15_000` | `:79` | HTTP smuggling test timeout (ms) |

---

## Compile-Time Validation

The engine crate validates `SUPPORTED_WAF_COUNT` at test time:

```rust
// crates/eggsec/src/constants.rs:19-25
#[test]
fn supported_waf_count_matches_actual() {
    let count = crate::waf::waf_patterns::get_waf_signatures().len();
    assert_eq!(
        count, SUPPORTED_WAF_COUNT,
        "SUPPORTED_WAF_COUNT must match actual detector count"
    );
}
```

This ensures the constant stays in sync with the actual WAF signature registry (`waf/data/patterns.rs:656`). The `get_waf_signatures()` function returns a `&'static FxHashMap<String, WafSignature>` built via `LazyLock`.

---

## Integration Points

| Constant family | Primary consumers |
|-----------------|-------------------|
| Timeouts (`*_TIMEOUT_*`) | `config/`, `scanner/`, `fuzzer/`, `waf/`, `recon/`, `browser/`, `loadtest/`, `proxy/` |
| Connection pool (`POOL_*`) | HTTP client construction (`utils/http_client.rs`) |
| Task queue (`TASK_QUEUE_*`) | `distributed/`, `agent/` |
| Agent (`LEASE_*`, `SCHEDULER_*`, `WORKER_*`) | `agent/`, `distributed/` |
| HTTP status (`STATUS_*`) | `error/mod.rs`, `utils/error.rs`, WAF detection |
| WAF (`waf::*`) | `waf/detection.rs`, `waf/bypass.rs`, `waf/data/patterns.rs` |
| Scan (`SCAN::*`) | `scanner/` |
| HTTP (`http::*`) | HTTP client defaults |
| Cache (`cache::*`) | `utils/caching.rs` |
| WAF count (`SUPPORTED_WAF_COUNT`) | Compile-time test only |

---

## Invariants & Gotchas

1. **Compile-time WAF count sync**: `SUPPORTED_WAF_COUNT` must exactly match `get_waf_signatures().len()`. The test in `crates/eggsec/src/constants.rs:19` catches drift. When adding a WAF signature, update the constant or the test fails.

2. **`WORKER_STALE_TIMEOUT_SECS` is `i64`**: Unlike other timeout constants which are `u64`, this is `i64` (`:28`). This is intentional — it's used with SQLite timestamp arithmetic.

3. **`SMUGGLING_TIMEOUT_SECS` and `SMUGGLING_TIMEOUT_MS` are redundant**: Both represent 15 seconds in different units (`:78-79`). The `_MS` variant avoids runtime multiplication in hot paths.

4. **`BLOCKED_STATUS_CODES` includes 406**: Not just 403/429/503 — 406 Not Acceptable is also a WAF block indicator (`:64`).

5. **`BLOCKED_PATTERNS` is case-sensitive at the constant level**: The matching code applies case-insensitive comparison; the constants themselves are lowercase.

6. **No `Default` or `new()` on nested modules**: The nested modules (`http`, `scan`, `cache`, `waf`) are pure namespace groupings with no associated types or constructors.

7. **Architecture guard**: `eggsec-core` has zero workspace crate dependencies. Constants must not reference any engine or domain types.

---

*Last verified against source: 2026-08-25*
