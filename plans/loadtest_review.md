# Loadtest Architecture Review

## Summary

The loadtest module architecture is well-implemented and matches the documented design. The `LoadTestRunner`, `Metrics`, and `LoadTestResults` components align with the architecture specification. High concurrency, worker model, rate limiting, authentication headers, and response body handling are all correctly implemented.

## Verified Implementation

### LoadTestRunner (`runner.rs`)
- **High Concurrency**: Uses `tokio` async/await with `JoinSet` for managing concurrent connections ✓
- **Worker Model**: Spawns `min(concurrency, total_requests)` workers, each via atomic counter (`runner.rs:283,301`) ✓
- **Configurable Workload**: Supports methods, headers, body content ✓
- **Fixed-Request Workload**: Executes configured request count with bounded concurrency ✓
- **Rate Limiting**: Proper token bucket with interval calculation (`runner.rs:275-281,306-318`) ✓
- **Auth Headers**: `apply_auth_headers()` method handles Basic, Bearer, Cookie, API Key (`runner.rs:176-208`) ✓
- **Connection Pool**: Non-success response bodies consumed before returning connections (`runner.rs:342-345`) ✓

### Struct Fields Verified
`LoadTestRunner` matches architecture doc:
```rust
pub struct LoadTestRunner {
    url: String,                    // ✓
    total_requests: u64,            // ✓
    concurrency: usize,             // ✓
    timeout: Duration,              // ✓
    method: Method,                 // ✓
    body: Option<String>,           // ✓
    headers: Vec<(String, String)>, // ✓
    insecure: bool,                 // ✓
    proxy: Option<String>,          // ✓
    proxy_auth: Option<String>,     // ✓
    user_agent: String,             // ✓
    rate_limit: Option<u32>,         // ✓
    tui_mode: bool,                 // ✓
}
```

### Constructors Verified
| Constructor | Implementation | Status |
|-------------|----------------|--------|
| `new(url, total, concurrency, timeout)` | Lines 36-43 | ✓ |
| `new_with_tui_mode(...)` | Lines 45-78 | ✓ |
| `from_args(args)` | Lines 80-82 | ✓ |
| `from_args_with_tui_mode(args, tui_mode)` | Lines 84-107 | ✓ |
| `from_args_with_config(args, config)` | Lines 109-138 | ✓ |

All constructors exist as documented. `from_args_with_config()` properly merges config settings (proxy, TLS verification, rate limits).

### Metrics (`metrics.rs`)
- **Latency Tracking**: Uses `hdrhistogram::Histogram<u64>` with 3 significant figures ✓
- **Percentiles**: p50, p90, p95, p99 properly calculated ✓
- **Throughput**: RPS calculated correctly ✓
- **Error Rates**: Tracks non-2xx/3xx status codes and transport failures ✓
- **FxHashMap**: Uses `FxHashMap<u16, u64>` for status code distribution (`metrics.rs:22,68,79`) ✓

### LoadTestResults (`metrics.rs:7-24`)
```rust
pub struct LoadTestResults {
    pub target_url: String,          // ✓
    pub total_requests: u64,        // ✓
    pub successful_requests: u64,   // ✓
    pub failed_requests: u64,       // ✓
    pub total_duration_ms: u64,     // ✓
    pub requests_per_second: f64,  // ✓
    pub latency_min_ms: f64,       // ✓
    pub latency_max_ms: f64,       // ✓
    pub latency_mean_ms: f64,      // ✓
    pub latency_p50_ms: f64,        // ✓
    pub latency_p90_ms: f64,        // ✓
    pub latency_p95_ms: f64,        // ✓
    pub latency_p99_ms: f64,        // ✓
    pub status_codes: FxHashMap<u16, u64>, // ✓
    pub errors: Vec<String>,        // ✓
}
```

### Rate Limiting Algorithm (`runner.rs:306-318`)
```rust
if let Some((min_interval, next_allowed_at)) = &rate_limit_state {
    let mut next = next_allowed_at.lock().await;
    let now = TokioInstant::now();
    if now < *next {
        sleep(*next - now).await;
    }
    let now_after_sleep = TokioInstant::now();
    if now_after_sleep >= *next {
        *next = now_after_sleep + *min_interval;
    } else {
        *next += *min_interval;
    }
}
```

**Verification**: Algorithm matches architecture doc exactly:
1. Worker acquires lock on `next_allowed_at` timestamp ✓
2. If current time < next_allowed, sleep until next_allowed ✓
3. Update `next_allowed = now_after_sleep + interval` (not `next + interval`) ✓

### Response Body Handling (`runner.rs:341-345`)
```rust
if !status.is_success() {
    if let Ok(bytes) = response.bytes().await {
        let _ = bytes;
    }
}
```
**Verification**: Non-success response bodies are consumed before returning connections to pool ✓

### CLI Entry Point (`mod.rs`)
- `run_cli(args, config)` function properly parses CLI args and executes load test ✓
- Returns `Result<()>` with `TabError` for UI integration (`mod.rs:66`) ✓

## Issues Found

### 1. Histogram Creation Panic Message
**File**: `metrics.rs:76`
```rust
histogram: Histogram::new(3).unwrap_or_else(|_| panic!("Failed to create histogram: 3 significant figures is invalid for hdrhistogram")),
```

**Issue**: The panic message is incorrect - `Histogram::new(3)` failing doesn't mean "3 significant figures is invalid" (that's a valid value). The actual error would be something else like memory allocation failure.

**Recommended Fix**: Use a more accurate error message:
```rust
histogram: Histogram::new(3).expect("Failed to create hdrhistogram"),
```

### 2. Division by Zero Guard for RPS
**File**: `metrics.rs:124-128`
```rust
requests_per_second: if duration_secs > 0.0 {
    total as f64 / duration_secs
} else {
    0.0
},
```

**Issue**: None - this is correctly guarded against division by zero ✓

### 3. Division by Zero in Histogram Stats
**File**: `metrics.rs:129-135`

**Issue**: None - hdrhistogram operations handle empty histograms internally and return appropriate defaults.

## Architecture Discrepancies

None. The implementation matches the architecture document closely.

## Performance Assessment

- **Hash Collections**: Uses `FxHashMap` for status codes ✓
- **Lock Contention**: Rate limiting uses fine-grained locking with `TokioInstant` ✓
- **Allocations**: Response body bytes explicitly consumed to prevent connection pool issues ✓
- **Concurrency**: Worker count is `min(concurrency, total_requests)` preventing resource exhaustion ✓

## Recommendations

1. **Low Priority**: Fix the histogram creation panic message to be more accurate about the actual failure cause