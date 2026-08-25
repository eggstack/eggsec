# Utils Module

## Overview

Common utility functions and helpers used throughout the Eggsec engine codebase for HTTP handling, URL parsing, input validation, output formatting, rate limiting, fault tolerance, and security-sensitive operations like redaction and timing-safe comparison.

## Location & Feature Gating

**Path**: `crates/eggsec/src/utils/`
**Module root**: `mod.rs` — 20 declared sub-modules (19 unconditional + 1 feature-gated)
**Feature gate**: `privilege` requires `stress-testing` or `packet-inspection` (`cfg(any(...))`)

Every number in this document was verified against source on 2026-08-25.

## Sub-Module Inventory (20 declared)

| # | Module | File | Purpose |
|---|--------|------|---------|
| 1 | `auth` | `auth.rs` | `constant_time_eq()` — timing-safe string comparison via `subtle::ConstantTimeEq` |
| 2 | `cache` | `cache.rs` | `ApiCache` — async TTL cache (`FxHashMap` + `RwLock`), default TTL 3600s, max 10k entries |
| 3 | `circuit_breaker` | `circuit_breaker.rs` | `CircuitBreaker`, `CircuitState` — fault tolerance (Closed/Open/HalfOpen) |
| 4 | `client_pool` | `client_pool.rs` | `ClientPool`, `OptimizedClientPool` — round-robin HTTP client reuse |
| 5 | `error` | `error.rs` | Error message sanitization — strips stack traces, paths, panics; truncates to 200 chars |
| 6 | `formatting` | `formatting.rs` | `strip_controls()`, `preserve_all()`, `truncate_only()` — string truncation helpers |
| 7 | `http` | `http.rs` | HTTP client creation family with proxy, TLS, and redirect policy options |
| 8 | `logging` | `logging.rs` | `sanitize_for_logging()` — ANSI escape/control char stripping, 500-char truncation |
| 9 | `network` | `network.rs` | TCP connect with `TCP_NODELAY` (Nagle disabled) |
| 10 | `output` | `output.rs` | Terminal output helpers — `print_error`, `print_info`, `print_json`, `print_success`, `print_warning` |
| 11 | `parsing` | `parsing.rs` | URL/header/port parsing, host resolution, `contains_ignore_case()` |
| 12 | `progress` | `progress.rs` | `LazyLock<ProgressStyle>` constants for `indicatif` progress bars |
| 13 | `rate_limiter` | `rate_limiter.rs` | `RateLimiter`, `AdaptiveRateLimiter`, `PerTargetRateLimiter`, `JitterConfig`, `SharedRateLimiter` |
| 14 | `redaction` | `redaction.rs` | `redact_sensitive()`, `redact_json()` — evidence redaction for findings |
| 15 | `service_detection` | `service_detection.rs` | Port→service mapping (53 entries), banner-based service guessing |
| 16 | `stealth` | `stealth.rs` | `StealthConfig`, `BrowserFingerprint`, `TlsFingerprint` — scanning evasion |
| 17 | `target` | `target.rs` | Target extraction, normalization, socket address parsing |
| 18 | `urlencoding` | `urlencoding.rs` | URL percent-encoding/decoding with UTF-8 support |
| 19 | `validation` | `validation.rs` | Input validation — concurrency, timeout, rate limit, path traversal, URL |
| 20 | `privilege` | `privilege.rs` | **Feature-gated** (`stress-testing` or `packet-inspection`): `check_privileged()`, `is_root()`, `require_root()` |

**Verified count**: 20 `pub mod` declarations in `mod.rs:29-50` (19 unconditional + 1 behind `cfg(any(...))`).

## Key Re-exports

All re-exports are in `mod.rs:52-77`:

| Symbol | Source | Notes |
|--------|--------|-------|
| `constant_time_eq` | `auth` | Timing-safe comparison |
| `CircuitBreaker`, `CircuitState` | `circuit_breaker` | Fault tolerance |
| `ClientPool`, `OptimizedClientPool` | `client_pool` | HTTP client pooling |
| `strip_controls`, `preserve_all` | `formatting` | String truncation |
| `create_http_client`, `create_http_client_with_options`, `create_http_client_with_proxy`, `create_insecure_client_with_options`, `create_insecure_http_client`, `get_shared_http_client`, `get_shared_insecure_http_client`, `same_host_redirect_policy` | `http` | HTTP client creation |
| `sanitize_for_logging` | `logging` | ANSI/control stripping |
| `connect_with_nodelay`, `connect_with_nodelay_timeout` | `network` | TCP with Nagle disabled |
| `print_error`, `print_info`, `print_json`, `print_json_compact`, `print_success`, `print_warning` | `output` | Terminal output |
| `contains_ignore_case`, `parse_headers`, `parse_url_validated` | `parsing` | Parsing utilities |
| `extract_domain`, `extract_host_port`, `extract_target_from_url`, `is_ip_address`, `normalize_url`, `parse_host_port`, `parse_socket_addr`, `strip_url_protocol` | `target` | Target handling |
| `validate_concurrency`, `validate_git_repo_path`, `validate_path`, `validate_path_string`, `validate_rate_limit`, `validate_timeout`, `validate_url` | `validation` | Input validation |
| `check_privileged`, `is_root`, `require_root` | `privilege` | **Feature-gated** |

## Behavior/API Highlights

### Formatting (`formatting.rs`)

| Function | Behavior |
|----------|----------|
| `strip_controls(s, max_len)` | Strips control chars (keeps space), left-pads to `max_len`, truncates with `...` suffix if exceeded |
| `preserve_all(s, max_len)` | Left-pads to `max_len`, truncates with `...` suffix — no control char stripping |
| `truncate_only(s, max_len)` | Simple `chars().take(max_len)` — no padding, no suffix |

Both `strip_controls` and `preserve_all` are verified by proptest to never exceed `max_len`.

### Regex Conventions

All regexes use `std::sync::LazyLock` for one-time initialization (no runtime allocation). Found in:
- `redaction.rs:10-53` — 9 `LazyLock<Regex>` patterns
- `error.rs:9-28` — 7 `LazyLock<Regex>` patterns
- `service_detection.rs:54` — `PORT_SERVICE_MAP` uses `LazyLock<FxHashMap>`

### Circuit Breaker (`circuit_breaker.rs`)

| State | Behavior |
|-------|----------|
| `Closed` | Normal operation; failures increment counter |
| `Open` | Blocks calls; transitions to `HalfOpen` after `timeout` expires (default: 30s) |
| `HalfOpen` | Allows probe calls; successes reset to `Closed` after `success_threshold` consecutive successes |

**Default thresholds** (`Default::default()` at line 118): `failure_threshold=5`, `success_threshold=3`, `timeout=30s`.
State is protected by `parking_lot::Mutex`; counters use `AtomicU64`/`AtomicUsize`.

### Rate Limiting (`rate_limiter.rs`)

| Type | Algorithm | Notes |
|------|-----------|-------|
| `RateLimiter` | Token bucket, 100ms refill interval | `acquire()` sleeps until token available |
| `AdaptiveRateLimiter` | Response-time-aware adaptive | Scales up to 10x base rate if avg response < 500ms; halves on error rate >10% or response >5s; 5s cooldown |
| `PerTargetRateLimiter` | Per-target `AdaptiveRateLimiter` map | Uses `FxHashMap` behind `Arc<Mutex>` |
| `SharedRateLimiter` | `Arc<Mutex<RateLimiter>>` wrapper | Thread-safe single limiter |
| `JitterConfig` | Random delay in `[min_ms, max_ms]` range | `from_spec("100-500")` parses range strings |

### Redaction (`redaction.rs`)

`redact_sensitive()` applies 10 regex patterns in sequence:
1. Bearer tokens → `[REDACTED]`
2. Basic auth → `[REDACTED]`
3. API keys (16+ char values) → `[REDACTED]`
4. AWS keys (`AKIA*`) → `[REDACTED AWS KEY]`
5. JWT tokens (3 dot-separated base64 segments) → `[REDACTED]`
6. Cookie header values → `[REDACTED]`
7. Private key PEM blocks → `[REDACTED PRIVATE KEY]`
8. Secret/password/token key-value pairs → `[REDACTED]`
9. Connection strings (mysql/postgres/mongodb/redis) → `[REDACTED CONNECTION STRING]`

`redact_json()` recursively walks JSON trees, redacting string values and renaming sensitive object keys (e.g., `"password"` → `"[REDACTED PASSWORD]"`).

### Error Sanitization (`error.rs`)

`sanitize_error_message()` strips: stack traces, internal details, file paths, Rust panics, Python tracebacks, Go panics, Windows paths. Truncates to 200 chars with `...` suffix.

### Logging Sanitization (`logging.rs`)

`sanitize_for_logging()` strips ANSI CSI escape sequences and control chars (preserving tabs), truncates to 500 chars. Uses byte-level parsing for correctness.

### HTTP Client Creation (`http.rs`)

| Function | Key Options |
|----------|-------------|
| `create_http_client(timeout_secs)` | Default client with `tcp_nodelay(true)`, pool settings from constants |
| `create_http_client_with_options(timeout_secs, builder_fn)` | Custom builder closure |
| `create_http_client_with_proxy(timeout_secs, proxy)` | HTTP proxy support |
| `create_insecure_http_client(timeout_secs)` | `danger_accept_invalid_certs(true)` + cookie store |
| `create_insecure_client_with_options(timeout_secs, builder_fn)` | Custom builder + insecure TLS |
| `get_shared_http_client()` | Singleton pool (10 clients), fallback to minimal client |
| `get_shared_insecure_http_client()` | Singleton insecure pool, warns on use |

**TLS provider**: All clients call `crate::install_tls_provider()` first. Uses ring-only rustls (no aws-lc-rs). Request features use `rustls-no-provider` to avoid pulling aws-lc-rs (`Cargo.toml:37`).

**Redirect policy**: `same_host_redirect_policy(max_redirects)` at `http.rs:227` blocks cross-host redirects to prevent scope-bypass via 3xx responses (e.g., to `169.254.169.254`).

**Pool defaults** (from `constants`): `DEFAULT_POOL_MAX_IDLE_PER_HOST`, `DEFAULT_POOL_IDLE_TIMEOUT_SECS`, `DEFAULT_MAX_REDIRECTS`.

### Client Pool (`client_pool.rs`)

`ClientPool` pre-creates N `reqwest::Client` instances and round-robins via `AtomicUsize`. All clients use `same_host_redirect_policy`, `tcp_nodelay(true)`, and configurable proxy/user-agent. Default pool size: 10.

### Network (`network.rs`)

Both `connect_with_nodelay()` and `connect_with_nodelay_timeout()` set `TCP_NODELAY` on the connected stream. The timeout variant wraps `TcpStream::connect` in `tokio::time::timeout`.

### Stealth (`stealth.rs`)

`StealthConfig` provides:
- **User-agent rotation**: 10 default agents (Chrome, Firefox, Safari, Edge, mobile)
- **Jitter**: Random delay in configurable range via `random_delay()`
- **Browser fingerprinting**: Random screen resolution, timezone, language, hardware concurrency
- **TLS fingerprinting**: Chrome/Firefox JA3 hashes, cipher suites, extensions, curves
- **Header rotation**: Randomized `Accept`, `Accept-Language`, plus Sec-Ch-Ua/Sec-Fetch headers
- **WebGL fingerprinting**: Random renderer string

`tool_user_agent()` returns `"Eggsec/{version}"` for non-stealth identification.

### Service Detection (`service_detection.rs`)

- `COMMON_PORTS`: 53-entry static array mapping ports to service names
- `PORT_SERVICE_MAP`: `LazyLock<FxHashMap>` built from `COMMON_PORTS`
- `guess_service_from_banner()`: Keyword matching against SSH, MySQL, Redis, PostgreSQL, MongoDB, FTP, SMTP, HTTP, LDAP, SMB, RDP, VNC
- `is_web_service()`: Ports 80, 443, 8080, 8443, 8888, 8000, 3000, 5000, 9000
- `is_database()`: Ports 1433, 1521, 3306, 5432, 6379, 27017, 9200

### Validation (`validation.rs`)

| Function | Bounds |
|----------|--------|
| `validate_concurrency(v)` | `1..=scan::DEFAULT_PORT_CONCURRENCY` |
| `validate_timeout(v)` | `1..=http::DEFAULT_TIMEOUT_SECS * 10` |
| `validate_rate_limit(v)` | `1..=MAX_REQUESTS_PER_SECOND_LIMIT` |
| `validate_path(base, user_path)` | Path traversal check — canonical path must start with base |
| `validate_url(url)` | Non-empty, valid URL with http/https scheme |

### Progress (`progress.rs`)

7 `LazyLock<ProgressStyle>` constants for `indicatif` progress bars:
`DEFAULT_PROGRESS_STYLE`, `SCAN_PROGRESS_STYLE`, `PORT_SCAN_PROGRESS_STYLE`, `ENDPOINT_DISCOVERY_STYLE`, `FINGERPRINT_STYLE`, `FUZZ_PROGRESS_STYLE`, `LOADTEST_STYLE`.

## Integration Points

| Consuming Module | Utils Used |
|------------------|------------|
| `scanner/` | `service_detection`, `network`, `progress`, `target`, `validation`, `http` |
| `fuzzer/` | `formatting`, `redaction`, `progress`, `http` |
| `recon/` | `parsing`, `target`, `urlencoding`, `http` |
| `waf/` | `redaction`, `http` |
| `stress/` | `network`, `stealth`, `rate_limiter`, `privilege` |
| `packet/` | `network`, `privilege` |
| `loadtest/` | `rate_limiter`, `progress`, `http` |
| `proxy/` | `http`, `redaction`, `stealth` |
| `config/` | `validation` |
| `output/` | `formatting`, `output` |
| `pipeline/` | `rate_limiter`, `circuit_breaker`, `progress` |
| `tool/` | `http`, `client_pool` |

**HTTP client conventions**: All HTTP client creation goes through `http.rs`. Clients use `tcp_nodelay(true)`, pool settings from constants, and the ring-only rustls TLS provider. The `same_host_redirect_policy` is applied to all pooled clients to prevent scope-bypass redirects.

## Testing

Every sub-module has `#[cfg(test)] mod tests` with unit tests. Several modules include property-based tests via `proptest`:
- `formatting.rs` — `strip_controls`/`preserve_all` never exceed `max_len`
- `parsing.rs` — port parsing, header parsing, URL validation
- `urlencoding.rs` — encode/decode roundtrip
- `validation.rs` — concurrency/timeout/rate_limit in-range passes

`circuit_breaker.rs` includes `#[tokio::test] async fn test_concurrent_record()` for concurrent access verification.

## Invariants & Gotchas

1. **`strip_controls` pads, doesn't truncate short strings**: If input is shorter than `max_len`, output is padded with spaces to exactly `max_len`. This is intentional for column alignment in terminal output.
2. **`redact_sensitive` is sequential**: All 10 regex patterns are applied in order. Each `.replace_all()` allocates a new `String`. For high-throughput paths, consider pre-compiled regex sets.
3. **`RateLimiter::acquire()` blocks**: The async `acquire()` sleeps in a loop until a token is available. Callers must use `.await` — this is not a non-blocking check.
4. **`parse_host_port` default_port**: The function signature is `parse_host_port(target, default_port)` — it silently returns the default when no port is present. The two-argument form differs from `extract_host_port` which returns `Option<(String, u16)>`.
5. **Feature-gated `privilege`**: `check_privileged`, `is_root`, `require_root` are only available with `stress-testing` or `packet-inspection` features. Code that uses them behind `cfg` must not reference them unconditionally.
6. **`sanitize_for_logging` max 500 chars**: The `sanitize_bytes` helper at `logging.rs:5` hardcodes a 500-char limit. This is shorter than `strip_controls`'s configurable limit.

## Related

- [logging.md](logging.md) — `utils/logging.rs` provides `sanitize_for_logging()` for stripping ANSI escapes and control characters from log output (used across scanner, fuzzer, pipeline, recon, stress, and waf modules).

*Last verified against source: 2026-08-25*
