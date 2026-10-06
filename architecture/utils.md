# Utils Module

> **Corrections (verified against source 2026-10-06).** Module count corrected
> 13 → **12** declared sub-modules (the header still counted `redaction`, removed in
> Phase G). `validate_concurrency` bound corrected: it is a local
> `MAX_CONCURRENCY = 1000` (`validation.rs:137`), not `scan::DEFAULT_PORT_CONCURRENCY`.
> The `reqwest` `rustls-no-provider` cite moved to `crates/eggsec/Cargo.toml:40`
> (was `Cargo.toml:37`, which no longer exists there). The Integration Points table
> was re-derived from actual `crate::utils::` call sites: `packet/`, `proxy/`,
> `config/` and `output/` have **zero** utils consumers, and the listed
> per-module symbol sets were wrong in several rows. A stale Invariant #2 about
> `redact_sensitive` was removed with the module it described.

## Overview

Engine-internal runtime helpers shared by several domains but not yet stable enough for a crate. Phase A (2026-09-16) removed false ownership: service tables moved to `scanner::service_data`, privilege gates to `platform`, cron to `eggsec-agent::cron`; dead presentation/pool/evasion helpers (`output`, `progress`, `client_pool`, `stealth`) were removed rather than relocated. Phase G later removed `redaction` as the last dead module.

## Location & Feature Gating

**Path**: `crates/eggsec/src/utils/`
**Module root**: `mod.rs` — 12 declared sub-modules, all unconditional (no feature gates)

Every number in this document was verified against source on 2026-10-06.

## Sub-Module Inventory (12 declared, no orphans)

| # | Module | File | Purpose |
|---|--------|------|---------|
| 1 | `auth` | `auth.rs` | `constant_time_eq()` — timing-safe string comparison via `subtle::ConstantTimeEq` |
| 2 | `circuit_breaker` | `circuit_breaker.rs` | `CircuitBreaker`, `CircuitState` — consecutive-failure breaker, single half-open probe |
| 3 | `error` | `error.rs` | Error message sanitization — strips stack traces, paths, panics; truncates to 200 chars |
| 4 | `formatting` | `formatting.rs` | `strip_controls()`, `preserve_all()`, `truncate_only()` — string truncation helpers |
| 5 | `http` | `http.rs` | HTTP client creation family + `tool_user_agent()`; single shared clients, same-host redirects |
| 6 | `logging` | `logging.rs` | `sanitize_for_logging()` — ANSI escape/control char stripping, 500-char truncation |
| 7 | `network` | `network.rs` | TCP connect with `TCP_NODELAY` (Nagle disabled) |
| 8 | `parsing` | `parsing.rs` | URL/header/port parsing, host resolution, `contains_ignore_case()` |
| 9 | `rate_limiter` | `rate_limiter.rs` | `RateLimiter` (burst=1s token bucket), `AdaptiveRateLimiter`, `PerTargetRateLimiter`, `JitterConfig`, `SharedRateLimiter` |
| 10 | `target` | `target.rs` | Target extraction, normalization, socket address parsing |
| 11 | `urlencoding` | `urlencoding.rs` | URL percent-encoding/decoding with UTF-8 support |
| 12 | `validation` | `validation.rs` | Input validation — concurrency, timeout, rate limit, path traversal, URL |

Removed in Phase A (see completion record in `plans/crate-boundary-consolidation-phase-a-ownership-and-primitive-cleanup.md`): `client_pool` (single shared client is canonical), `output` (dead terminal printers; engine returns values), `progress` (dead `indicatif` styles; frontends own presentation), `service_detection` (moved to `scanner::service_data`), `stealth` (dead evasion semantics; only live `tool_user_agent()` moved to `http`), `privilege` (moved to `platform::check_privileged`/`require_root`/`is_root`).

Removed in Phase D (see completion record in `plans/crate-boundary-consolidation-phase-d-loadtest-resilience-reuse-closure.md`): `cache` (`ApiCache`, zero production consumers; the `ai::cache::AiCache` used by AI code is a separate module and is unaffected).

Removed in Phase G (see `plans/implementation/security-knowledge-corpus/001-redaction-dead-code-disposition.md` and `plans/closure/security-knowledge-corpus/001-closure.md`): `redaction` (`redact_sensitive()`/`redact_json()`, 366 lines, 26 tests, zero production consumers). The repo's redaction contract is the declarative `RedactionState` carried per evidence item in `eggsec-report-model` (`envelope.rs`) and consumed by `eggsec-output`, `eggsec-db-lab`, and `eggsec-mobile-lab`; debug-only masking lives in `eggsec-transport` as `redacted_headers_debug`/`redact_url_for_debug`, bound to that crate's four-dependency closure by check 108. Check 147 prevents silent recurrence.

**Verified count**: 12 `pub mod` declarations in `mod.rs` (`mod.rs:36-47`, all unconditional), matching the 13 `.rs` files on disk (`mod.rs` + 12 modules). Note: the former orphan `serialization.rs` referenced in earlier revisions no longer exists on disk — it has been deleted, so there is no orphan file.

## Key Re-exports

All re-exports are in `mod.rs`:

| Symbol | Source | Notes |
|--------|--------|-------|
| `constant_time_eq` | `auth` | Timing-safe comparison |
| `CircuitBreaker`, `CircuitState` | `circuit_breaker` | Fault tolerance |
| `strip_controls`, `preserve_all` | `formatting` | String truncation |
| `create_http_client`, `create_http_client_with_options`, `create_http_client_with_proxy`, `create_insecure_client_with_options`, `create_insecure_http_client`, `get_shared_http_client`, `get_shared_insecure_http_client`, `same_host_redirect_policy`, `tool_user_agent` | `http` | HTTP client creation + tool identity |
| `sanitize_for_logging` | `logging` | ANSI/control stripping |
| `connect_with_nodelay`, `connect_with_nodelay_timeout` | `network` | TCP with Nagle disabled |
| `contains_ignore_case`, `parse_headers`, `parse_url_validated` | `parsing` | Parsing utilities |
| `extract_domain`, `extract_host_port`, `extract_target_from_url`, `is_ip_address`, `normalize_url`, `parse_host_port`, `parse_socket_addr`, `strip_url_protocol` | `target` | Target handling |
| `validate_concurrency`, `validate_git_repo_path`, `validate_path`, `validate_path_string`, `validate_rate_limit`, `validate_timeout`, `validate_url` | `validation` | Input validation |

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
- `error.rs:9-29` — 8 `LazyLock<Regex>` patterns (`PATH_PATTERN`, `STACK_TRACE_PATTERN`, `INTERNAL_PATTERN`, `RATE_LIMIT_DETAIL`, `RUST_PANIC`, `PYTHON_TRACEBACK`, `GO_PANIC`, `WINDOWS_PATH`)

`redaction.rs`'s 10 `LazyLock<Regex>` statics were removed with the module in Phase G.

### Circuit Breaker (`circuit_breaker.rs`)

| State | Behavior |
|-------|----------|
| `Closed` | Normal operation; **consecutive** failures increment counter; any success resets failure history to 0 |
| `Open` | Blocks calls; transitions to `HalfOpen` after `timeout` expires (default: 30s), admitting a single probe |
| `HalfOpen` | **Single concurrent probe only**; success increments probe count (close at `success_threshold`); any failure re-opens immediately |

Rejected calls (via `is_available() == false`) are not counted in `total_calls`/`total_failures`. Time uses `std::time::Instant` only (no Tokio `test-util`).

**Default thresholds** (`Default::default()`): `failure_threshold=5`, `success_threshold=3`, `timeout=30s`.
State is protected by `parking_lot::Mutex`; counters use `AtomicU64`/`AtomicUsize`; half-open permit uses `AtomicBool`.

### Rate Limiting (`rate_limiter.rs`)

| Type | Algorithm | Notes |
|------|-----------|-------|
| `RateLimiter` | Token bucket, burst = 1s (`max = rps`), 100ms refill interval | `new(0)` clamps to 1; `acquire()` sleeps in bounded waits (drop-cancellable); `try_acquire()` non-blocking |
| `AdaptiveRateLimiter` | Response-time-aware delay pacing (`1/rate` + 5s failure cooldown) | Success does not immediately clear failure history (10 fast successes raise rate); failure halves rate |
| `PerTargetRateLimiter` | Per-target `AdaptiveRateLimiter` map (`Arc<Mutex<…>>` per target) | Global map lock held only to clone `Arc`, never across target `await`; target A cannot serialize target B |
| `SharedRateLimiter` | `Arc<Mutex<RateLimiter>>` wrapper | Releases lock while sleeping; `try_acquire()` available |
| `JitterConfig` | Random delay in `[min_ms, max_ms]` range | `from_spec("100-500")` parses range strings |

DTO separation: `RateLimitStatus` conversion lives in a dedicated adapter block; core acquisition never takes DTO types. Operation-specific limiters (e.g. `fuzzer::rate_limit` lock-free consecutive-error limiter) stay with their operation; reconsidered only in Phase D.

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
| `get_shared_http_client()` | Single long-lived client (cloned `Arc`-internally by reqwest); fallback to minimal client |
| `get_shared_insecure_http_client()` | Single long-lived insecure client, warns on use |
| `tool_user_agent()` | Honest `"Eggsec/{version}"` identifier (moved from removed `stealth`; not evasion) |

**TLS provider**: All clients call `crate::install_tls_provider()` first. Uses ring-only rustls (no aws-lc-rs). Request features use `rustls-no-provider` to avoid pulling aws-lc-rs (`crates/eggsec/Cargo.toml:40`).

**Redirect policy**: `same_host_redirect_policy(max_redirects)` blocks cross-host redirects to prevent scope-bypass via 3xx responses (e.g., to `169.254.169.254`).

**Pool defaults** (from `constants`): `DEFAULT_POOL_MAX_IDLE_PER_HOST`, `DEFAULT_POOL_IDLE_TIMEOUT_SECS`, `DEFAULT_MAX_REDIRECTS`.

**No multi-client pool (Phase A WS6)**: each `reqwest::Client` already manages its own internal connection pool; the removed N-client round-robin pool sharded connections/TLS state with identical per-client config and no isolation property. Canonical path is one cloned shared client (or the scoped `eggsec-transport` seam for eligible consumers).

### Network (`network.rs`)

Both `connect_with_nodelay()` and `connect_with_nodelay_timeout()` set `TCP_NODELAY` on the connected stream. The timeout variant wraps `TcpStream::connect` in `tokio::time::timeout`.

Service fingerprint knowledge lives in `scanner::service_data` (Phase A), not here. Privilege gates live in `platform` (`check_privileged`/`require_root`/`is_root`), not here.

### Validation (`validation.rs`)

| Function | Bounds |
|----------|--------|
| `validate_concurrency(v)` | `1..=1000` — local `MAX_CONCURRENCY` const (`validation.rs:137`), deliberately above the port-scan default so valid fuzzer/endpoint values are not rejected |
| `validate_timeout(v)` | `1..=http::DEFAULT_TIMEOUT_SECS * 10` |
| `validate_rate_limit(v)` | `1..=constants::MAX_REQUESTS_PER_SECOND_LIMIT` |
| `validate_path(base, user_path)` | Path traversal check — canonical path must start with base |
| `validate_url(url)` | Non-empty, valid URL with http/https scheme |

## Integration Points

Re-derived from actual `crate::utils::` call sites on 2026-10-06.

| Consuming Module | Utils Used |
|------------------|------------|
| `recon/` | `http` (client family), `network` (`connect_with_nodelay`, WHOIS), `target` (`extract_target_from_url`), `validation` (`validate_git_repo_path`) — the heaviest consumer |
| `scanner/` | `network` (`connect_with_nodelay_timeout`), `parsing` (`resolve_host`, `parse_ports`), `formatting`, `logging`, `validation`; owns `service_data` (the `eggsec-service-db` facade) |
| `tool/` | `http` (shared clients), `circuit_breaker`, `rate_limiter`, `auth` (`constant_time_eq`), `parsing`, `logging`, `error` |
| `commands/` | `target` (`extract_target_from_url`, `parse_host_port`), `validation` |
| `auth/` | `http` (insecure client), `urlencoding`, `network` |
| `fuzzer/` | `urlencoding`, `formatting` (`strip_controls`), `http`, `validation`, `logging` |
| `waf/` | `http` (insecure client + `same_host_redirect_policy`), `circuit_breaker`, `parsing` (`contains_ignore_case`), `logging` |
| `dispatch/` | `http` (shared clients), `parsing`, `validation` |
| `pipeline/` | `parsing` (`parse_ports`), `logging` (`sanitize_for_logging`) |
| `loadtest/` | `parsing` (`parse_headers`), `http` (`tool_user_agent`), `formatting` (`preserve_all` in results `Display`); pacing is operation-local (`GlobalPacer`, intentionally distinct from the shared token buckets — see Phase D record) |
| `distributed/` | `network`, `parsing` |
| `stress/` | `http`, `logging` (privilege checks come from `platform`, not `utils`) |
| `agent/` | `validation` |
| `ai/` | `circuit_breaker` |
| `container/`, `notify/`, `hunt/` | `http` |
| `platform/` | owns `is_root`/`check_privileged`/`require_root` (moved from `utils::privilege`) |
| `packet/`, `proxy/`, `config/`, `output/` | **no utils consumers** (0 call sites) |

**HTTP client conventions**: All HTTP client creation goes through `http.rs`. Clients use `tcp_nodelay(true)`, pool settings from constants, and the ring-only rustls TLS provider. The `same_host_redirect_policy` is applied to shared clients to prevent scope-bypass redirects. Terminal presentation lives in CLI/TUI/process-host layers; engine code returns values/errors.

## Testing

Every sub-module has `#[cfg(test)] mod tests` with unit tests. Several modules include property-based tests via `proptest`:
- `formatting.rs` — `strip_controls`/`preserve_all` never exceed `max_len`
- `parsing.rs` — port parsing, header parsing, URL validation
- `urlencoding.rs` — encode/decode roundtrip
- `validation.rs` — concurrency/timeout/rate_limit in-range passes

`circuit_breaker.rs` includes `#[tokio::test] async fn test_concurrent_record()` for concurrent access verification.

## Invariants & Gotchas

1. **`strip_controls` pads, doesn't truncate short strings**: If input is shorter than `max_len`, output is padded with spaces to exactly `max_len`. This is intentional for column alignment in terminal output.
2. **`RateLimiter::acquire()` blocks**: The async `acquire()` sleeps in bounded waits until a permit is available. Callers must `.await` and should race with cancellation (`eggsec-runtime::race_with_cancel`); the future is drop-cancellable.
3. **`parse_host_port` default_port**: The function signature is `parse_host_port(target, default_port)` — it silently returns the default when no port is present. The two-argument form differs from `extract_host_port` which returns `Option<(String, u16)>`.
4. **Privilege lives in `platform`**: `check_privileged`, `is_root`, `require_root` are unconditional in `platform` (moved from feature-gated `utils::privilege` in Phase A). Callers use `crate::platform::…`.
5. **`sanitize_for_logging` max 500 chars**: The `sanitize_bytes` helper at `logging.rs:5` hardcodes a 500-char limit. This is shorter than `strip_controls`'s configurable limit.

## Related

- [logging.md](logging.md) — `utils/logging.rs` provides `sanitize_for_logging()` for stripping ANSI escapes and control characters from log output (used across scanner, fuzzer, pipeline, recon, stress, and waf modules).

*Last verified against source: 2026-09-16 (Phase A ownership cleanup); counts re-verified 2026-09-22 (systematic review); regex counts, redact_sensitive pattern ownership, and orphan-file note fixed 2026-09-25 (systematic review)*
