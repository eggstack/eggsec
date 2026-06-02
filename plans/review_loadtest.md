# Loadtest Module Architecture Review

**Document:** architecture/loadtest.md
**Reviewed:** 2026-06-02
**Accuracy:** High
**Lines Reviewed:** 143

## Verified Claims

- **LoadTestRunner struct**: Verified at `crates/slapper/src/loadtest/runner.rs:20-34` - all fields match: url, total_requests, concurrency, timeout, method, body, headers, insecure, proxy, proxy_auth, user_agent, rate_limit, tui_mode
- **Worker count calculation**: Verified at `runner.rs:286` - `min(concurrency, total_requests as usize)`
- **Rate limiting semaphore approach**: Verified at `runner.rs:271-282` - uses `Semaphore` with `add_permits(1)` every `min_interval`
- **CancellationToken for graceful shutdown**: Verified at `runner.rs:284` and token check at line 305
- **Progress bar style**: Verified at `runner.rs:259-264` - template matches `{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})`
- **Report trait implementation**: Verified at `runner.rs:380-388` - `title()` returns "Load Test Report", `to_json()` uses `serde_json::to_string_pretty()`
- **LoadTestResults struct**: Verified at `crates/slapper/src/loadtest/metrics.rs:8-24` - all fields match including latency percentiles (p50, p90, p95, p99) and FxHashMap for status_codes
- **run_cli() signature**: Verified at `crates/slapper/src/loadtest/mod.rs:66` - `pub async fn run_cli(args: LoadArgs, config: &SlapperConfig) -> Result<()>`
- **from_args_with_config() for pipeline**: Verified at `runner.rs:110-134` - merges config settings including proxy, TLS verification, rate limits
- **hdrhistogram with 3 significant figures**: Verified at `metrics.rs:76` - `Histogram::new(3)`
- **FxHashMap for status codes**: Verified at `metrics.rs:22,68`
- **apply_auth_headers() method**: Verified at `runner.rs:172-204` - handles Basic, Bearer, Cookie, API Key authentication
- **Response body consumption for non-success**: Verified at `runner.rs:338-340` - `if status_code >= 400 { let _ = response.bytes().await; }`

## Discrepancies

- **None identified**: All constructor methods, fields, and behavior match between documentation and implementation.

## Bugs Found

- **Bug**: In `runner.rs:315`, the semaphore acquire uses `.unwrap()`:
  ```rust
  if let Some(sem) = &rate_limit_sem {
      let _permit = sem.acquire().await.unwrap();
  }
  ```
  This could panic if the semaphore is closed. Should handle the error explicitly rather than unwrapping. (runner.rs:315)

## Improvement Opportunities

- **Priority: Medium**: The `rate_limit` field is `Option<u32>` (requests per second) in `LoadTestRunner` but the documentation describes it as "requests per second" while the semaphore is initialized with `rate as usize` permits. The comment about "Semaphore starts with `rate` permits, preventing initial burst" at line 272 is slightly misleading - it actually starts with `rate` permits which allows `rate` requests through immediately before the first interval tick adds more. This is correct behavior but could be clearer.
  
- **Priority: Low**: The `LoadTestResults` struct in `metrics.rs` does not include a field for "requests per second" as a separate calculation - it's computed on-demand in `to_results()`. This is fine but the document could note this.

## Stale Items

- **None identified**: Rate limiting algorithm description matches the current semaphore-based implementation (replaced mutex approach on 2026-05-28 as documented).

## Code Interrogation Findings

- **Finding**: In `runner.rs:286`, the worker count calculation `min(concurrency, total_requests as usize)` means if you request 5 total requests with concurrency 10, only 5 workers spawn. This is correct behavior.
- **Finding**: The `set_common_with_config()` method at `runner.rs:149-170` properly merges config settings with CLI args, but does not call `apply_auth_headers()` with the merged config's auth settings if `common.auth` is `None`. This could be a gap if config has default auth.
- **Finding**: The `to_results()` method at `metrics.rs:114-139` calculates `requests_per_second` as `total / duration_secs`. If `duration_secs` is 0 (which shouldn't happen in practice since the test runs at least one request), this would panic with division by zero. However, `start.elapsed()` should always be > 0 after the first request completes.

## Summary

The loadtest module architecture documentation is highly accurate. All key components, data structures, and algorithms are correctly documented. The semaphore-based rate limiting implementation is correctly described. One minor unwrap() issue found that could be improved for robustness.