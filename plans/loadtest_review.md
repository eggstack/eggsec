# Loadtest Module Architecture Review

Based on review of `architecture/loadtest.md` against implementation in `crates/slapper/src/loadtest/`.

---

## Verified Claims

### Core Components
- **Runner (`runner.rs`)**: Implementation matches documentation. `LoadTestRunner` struct has all documented fields (lines 19-33).
- **Metrics (`metrics.rs`)**: `LoadTestResults` struct matches documentation exactly (lines 8-24 of metrics.rs).
- **High Concurrency Model**: Uses `tokio::task::JoinSet` as documented (runner.rs:284).
- **Worker Model**: `min(concurrency, total_requests)` spawn logic verified at runner.rs:283.
- **Rate Limiting Algorithm**: Lock-protected token bucket with `next_allowed_at` timestamp matching doc (runner.rs:275-281, 306-318).
- **Histograms**: Uses `hdrhistogram::Histogram<u64>` with 3 significant figures (metrics.rs:76).
- **FxHashMap**: Uses `rustc_hash::FxHashMap` for status codes (metrics.rs:68).
- **Response Body Handling**: Non-success bodies consumed before returning to pool (runner.rs:341-345).
- **Auth Headers**: `apply_auth_headers()` method handles Basic, Bearer, Cookie, API Key (runner.rs:176-208).
- **Constructors**: All 4 constructors documented exist and work as described.

### Latency Tracking
- Percentiles (p50, p90, p95, p99) tracked and calculated correctly (metrics.rs:132-135).
- Uses `hdristogram::Histogram<u64>` with 3 significant figures.

---

## Discrepancies

| Section | Documentation | Implementation | Severity |
|---------|---------------|----------------|----------|
| CLI Entry Point | `run_cli(args: LoadArgs)` returns `Result<(), TabError>` | `run_cli(args: LoadArgs, config: &SlapperConfig)` returns `Result<()>` and takes config | Low - Function signature changed but behavior compatible |
| Mod.rs Example | `run_cli()` shown with single arg | Actual takes two args with config | Documentation outdated |

---

## Bugs Found

### 1. **Race Condition in Metrics Recording** (Medium Priority)
**Location**: `runner.rs:336`

The metrics lock is held during the entire HTTP response handling:
```rust
let mut metrics = metrics.lock().await;

match result {
    Ok(response) => { ... } // Lock held while awaiting response body read
    Err(e) => { ... }
}
```

**Problem**: The lock is held while reading the response body (`response.bytes().await`), which can take significant time under slow responses or network issues. This creates a bottleneck where workers block waiting for the lock.

**Fix**: Move body consumption outside the lock, then acquire lock only for recording:
```rust
let response_body = if !status.is_success() {
    response.bytes().await.ok()
} else {
    None
};

let mut metrics = metrics.lock().await;
metrics.record_http_response(latency, status.as_u16());
```

### 2. **Silent Histogram Record Failures** (Low Priority)
**Location**: `metrics.rs:87`

```rust
let _ = self.histogram.record(latency_ms);
```

**Problem**: If histogram recording fails (e.g., value out of range), the failure is silently ignored with `let _`. This could mask data collection issues.

**Fix**: Log a warning if recording fails, or use a Result-returning approach in metrics collection.

---

## Improvement Opportunities

### 1. **Response Body Memory Pressure** (Medium Priority)
**Location**: `runner.rs:342-344`

```rust
if !status.is_success() {
    if let Ok(bytes) = response.bytes().await {
        let _ = bytes;
    }
}
```

**Issue**: `response.bytes().await` loads the entire body into memory. For large error responses (e.g., verbose error pages, HTML error pages), this can cause memory pressure with high concurrency.

**Suggestion**: Use `response.bytes_stream()` and read into a small fixed buffer, draining the stream without allocating for full content:
```rust
let mut body = [0u8; 4096];
while response.body().read(&mut body).await.is_ok() {}
```

### 2. **Error Message Truncation** (Low Priority)
**Location**: `metrics.rs:101-103`

```rust
if self.errors.len() < 100 {
    self.errors.push(format!("HTTP {}", status_code));
}
```

**Issue**: HTTP error messages don't include the URL or any context. If multiple targets are tested, debugging which target returned which error code is difficult.

**Suggestion**: Include target URL in error message or use a structured error type.

### 3. **JoinSet Error Handling** (Medium Priority)
**Location**: `runner.rs:360-363`

```rust
while let Some(join_result) = workers.join_next().await {
    join_result.map_err(|e| SlapperError::Runtime(...))?;
}
```

**Issue**: If any worker panics (e.g., from a bug in request handling), `join_result` will be `Err(JoinError::panicked())`. The current code converts this to a generic runtime error, but panics indicate bugs that should be visible in logs.

**Suggestion**: Distinguish between `Aborted` (normal completion) and `Panicked` (crash):
```rust
match join_result {
    Ok(Ok(())) => {}, // Normal worker completion
    Ok(Err(e)) => tracing::error!("Worker failed: {}", e),
    Err(e) if e.is_panic() => tracing::error!("Worker panicked: {:?}", e),
    Err(e) => tracing::error!("Worker aborted: {}", e),
}
```

### 4. **Missing Test Coverage** (Medium Priority)
**Location**: `loadtest_tests.rs`

**Issue**: Tests exist but don't cover:
- Rate limiting behavior
- Authentication header combinations
- Proxy usage
- Non-2xx success responses (3xx redirects)
- Timeout handling

### 5. **Histogram Initialization Could Fail** (Low Priority)
**Location**: `metrics.rs:76`

```rust
histogram: Histogram::new(3).expect("Failed to create hdrhistogram"),
```

**Issue**: Using `expect()` in struct initialization is correct per AGENTS.override.md guidance, but there's no way for callers to handle this gracefully since `Metrics::new()` doesn't return a `Result`.

**Suggestion**: Change `Metrics::new()` to return `Result<Self>` to allow graceful handling.

---

## Priority Summary

| Finding | Type | Priority |
|---------|------|----------|
| Metrics lock held during async body read | Bug | Medium |
| Response body memory pressure | Performance | Medium |
| JoinSet panic handling | Bug | Medium |
| Silent histogram record failures | Bug | Low |
| Error messages lack context | Improvement | Low |
| Missing test coverage | Improvement | Medium |
| Metrics::new() can't fail gracefully | Improvement | Low |

---

## Recommendations

1. **Immediate**: Fix the metrics lock contention by moving body consumption outside the lock (bug #1).

2. **Short-term**: Add panic-aware error handling to JoinSet loop and test coverage for rate limiting.

3. **Long-term**: Consider streaming response body consumption and structured error types.