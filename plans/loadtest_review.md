# Loadtest Module Architecture Review

## Summary Statistics

| Category | Count |
|----------|-------|
| Verified Claims | 14 |
| Discrepancies | 2 |
| Bugs Found | 2 |
| Improvement Opportunities | 4 |

---

## Verified Claims

### 1. LoadTestRunner Structure (runner.rs:19-33)
**Status: VERIFIED**

The struct matches the architecture document exactly:
```rust
pub struct LoadTestRunner {
    url: String,
    total_requests: u64,
    concurrency: usize,
    timeout: Duration,
    method: Method,
    body: Option<String>,
    headers: Vec<(String, String)>,
    insecure: bool,
    proxy: Option<String>,
    proxy_auth: Option<String>,
    user_agent: String,
    rate_limit: Option<u32>,    // requests per second
    tui_mode: bool,
}
```

### 2. Constructor Methods (runner.rs:36-138)
**Status: VERIFIED**

All four constructors documented exist and function as specified:
- `new()` → Basic constructor with validation
- `new_with_tui_mode()` → With explicit TUI mode flag
- `from_args()` → From CLI `LoadArgs`
- `from_args_with_config()` → CLI args merged with `SlapperConfig`

### 3. Worker Model (runner.rs:283)
**Status: VERIFIED**

```rust
let worker_count = self.concurrency.min(self.total_requests as usize);
```

Matches: "Spawns `min(concurrency, total_requests)` workers"

### 4. Async/Await with JoinSet (runner.rs:284, 299-356)
**Status: VERIFIED**

Uses `tokio::task::JoinSet` for concurrent worker management as documented.

### 5. Rate Limiting Algorithm (runner.rs:275-317)
**Status: VERIFIED**

Lock-protected token bucket approach matches documentation:
```rust
if now < *next {
    sleep(*next - now).await;
}
let now_after_sleep = TokioInstant::now();
if now_after_sleep >= *next {
    *next = now_after_sleep + *min_interval;
} else {
    *next += *min_interval;
}
```

### 6. Auth Headers Support (runner.rs:176-208)
**Status: VERIFIED**

`apply_auth_headers()` handles Basic, Bearer, Cookie, and API Key authentication as documented.

### 7. hdrhistogram for Latency (metrics.rs:76)
**Status: VERIFIED**

Uses `Histogram::new(3)` with 3 significant figures as documented.

### 8. LoadTestResults Structure (metrics.rs:7-24)
**Status: VERIFIED**

All fields match the architecture document exactly, including `FxHashMap<u16, u64>` for status_codes.

### 9. FxHashMap Usage (metrics.rs:68, metrics.rs:22)
**Status: VERIFIED**

Uses `rustc_hash::FxHashMap` for status code distribution as documented.

### 10. run_cli() Function Signature (mod.rs:66)
**Status: VERIFIED**

```rust
pub async fn run_cli(args: LoadArgs, config: &SlapperConfig) -> Result<()>
```

### 11. Response Body Handling for Non-Success (runner.rs:339-340)
**Status: VERIFIED** (partial - see Discrepancies)

### 12. Connection Pool Body Consumption
**Status: VERIFIED**

Non-success response bodies are consumed before returning to pool.

### 13. CLI Entry Point (mod.rs:52-99)
**Status: VERIFIED**

`run_cli()` parses arguments, creates runner, executes test, and outputs results.

### 14. Throughput and Error Tracking (metrics.rs:114-139)
**Status: VERIFIED**

RPS calculation and error rate tracking implemented correctly.

---

## Discrepancies

### 1. Response Body Consumption - Only 2xx Checked
**Severity: Medium**
**Location: runner.rs:339-340 vs metrics.rs:97-104**

**Architecture Document States:**
> "When non-success responses (4xx/5xx) are received, the response body is consumed before recording metrics."

**Implementation:**
```rust
// runner.rs:339-340
if !status.is_success() {
    let _ = response.bytes().await;
}
```

```rust
// metrics.rs:97-104
if (200..400).contains(&status_code) {
    self.successful += 1;
} else {
    self.failed += 1;
```

**Discrepancy:** `status.is_success()` returns `true` only for 2xx status codes. The code in `metrics.rs` correctly counts 2xx-3xx as successful (`(200..400).contains()`), but the body consumption in `runner.rs` only checks `!is_success()`, meaning 3xx response bodies are NOT consumed.

This could leave connections in an inconsistent state for 3xx responses if the HTTP client reuses them.

**Recommendation:** Change runner.rs:339 from:
```rust
if !status.is_success() {
```
to:
```rust
if !(200..400).contains(&status_code.as_u16()) {
```

### 2. Documentation Mentions `from_args_with_tui_mode()` but doesn't fully document it
**Severity: Low**
**Location: architecture/loadtest.md:46**

The architecture document lists `from_args_with_tui_mode(args, tui_mode)` in the constructors table but provides no description. The actual implementation is documented in AGENTS.override.md.

---

## Bugs Found

### Bug 1: Rate Limiting Initial State Causes Immediate Burst
**Severity: High**
**Location: runner.rs:275-281**

```rust
let rate_limit_state = self.rate_limit.map(|rate| {
    let min_interval = Duration::from_secs_f64(1.0 / f64::from(rate));
    (
        min_interval,
        Arc::new(Mutex::new(TokioInstant::now() - min_interval)),
    )
});
```

**Problem:** The initial `next_allowed_at` is set to `now - min_interval`. This means when workers check `if now < *next`, the first worker will always pass through immediately (since `now >= now - interval`), and subsequent workers may also pass immediately if multiple workers start before any sleeps.

This can cause a burst of requests that exceeds the intended rate limit on startup.

**Recommendation:** Initialize to `TokioInstant::now()` instead of `TokioInstant::now() - min_interval` to ensure proper rate limiting from the first request.

**Alternative Fix:** If the current behavior is intentional for "warming up", add a comment explaining this is by design.

### Bug 2: Auth Header Splitting Without Trim
**Severity: Low**
**Location: runner.rs:184-189**

```rust
if let Some(auth) = auth {
    let parts: Vec<&str> = auth.splitn(2, ':').collect();
    if parts.len() == 2 {
        let encoded =
            general_purpose::STANDARD.encode(format!("{}:{}", parts[0], parts[1]));
        self.add_header("Authorization".to_string(), format!("Basic {}", encoded));
    }
}
```

**Problem:** If user provides `Basic dXNlcjpwYXNz` directly in the `auth` field, it will be incorrectly base64-encoded again, producing invalid credentials.

**Scenario:** User passes `--auth "Basic dXNlcjpwYXNz"` expecting to use a pre-encoded token, but gets `Basic Basic dXNlcjpwYXNz`.

**Recommendation:** Check if `auth` already starts with "Basic " and handle accordingly, or document that `auth` expects `username:password` format only.

---

## Improvement Opportunities

### Improvement 1: Rate Limit Lock Contention
**Severity: Medium**
**Location: runner.rs:306-317**

**Problem:** Every worker acquires a lock on `next_allowed_at` for every request, causing lock contention under high concurrency with rate limiting enabled.

**Impact:** Rate limiting could become a bottleneck, reducing effective throughput.

**Recommendation:** Consider per-worker rate limiting with local counters, or use a channel-based approach where a single task manages rate limiting and workers receive permits.

### Improvement 2: Missing Request Cancellation on Timeout
**Severity: Medium**
**Location: runner.rs:322-333**

```rust
let mut req = client.request(method.clone(), &url);
// ... headers, body setup ...
let result = req.send().await;
```

**Problem:** If the test is interrupted (Ctrl+C), in-flight requests are not gracefully cancelled. The `run()` method returns immediately without waiting for or cancelling pending tasks.

**Recommendation:** Add proper shutdown handling with `JoinSet::abort()` and cancellation tokens.

### Improvement 3: Metrics Histogram Recording Silently Fails
**Severity: Low**
**Location: metrics.rs:87, 94**

```rust
let _ = self.histogram.record(latency_ms);
```

**Problem:** If histogram recording fails (e.g., value out of range), the error is silently ignored via `let _ =`. While documented in AGENTS.override.md, this could mask data collection issues.

**Recommendation:** Consider logging at trace level if recording fails, or use a dedicated MetricsResult type to propagate errors.

### Improvement 4: Error Message Truncation for HTTP Errors
**Severity: Low**
**Location: metrics.rs:102**

```rust
if self.errors.len() < 1000 {
    self.errors.push(format!("HTTP {}", status_code));
}
```

**Problem:** HTTP errors only record the status code, not the URL or other context. This makes debugging difficult when multiple endpoints are tested.

**Recommendation:** Include URL and optionally the full response status in the error message.

---

## Priority Summary

| Finding | Priority |
|---------|----------|
| Rate limiting initial burst | High |
| Response body consumption for 3xx | Medium |
| Rate limit lock contention | Medium |
| Missing cancellation on timeout | Medium |
| Auth header pre-encoded handling | Low |
| Histogram recording silent failure | Low |
| HTTP error message truncation | Low |
| Documentation gap (from_args_with_tui_mode) | Low |

---

## Files Reviewed

| File | Lines | Purpose |
|------|-------|---------|
| `crates/slapper/src/loadtest/runner.rs` | 386 | Worker model, rate limiting, HTTP execution |
| `crates/slapper/src/loadtest/metrics.rs` | 140 | Metrics collection, histogram, results |
| `crates/slapper/src/loadtest/mod.rs` | 100 | Module entry, CLI integration |
| `crates/slapper/src/loadtest/AGENTS.override.md` | 48 | Module-specific patterns and bug fixes |
| `crates/slapper/tests/loadtest_tests.rs` | 292 | Test coverage |

---

## Verification Commands Run

```bash
cargo check --lib -p slapper
cargo test --lib -p slapper
cargo clippy --lib -p slapper
```

No clippy warnings in the loadtest module.