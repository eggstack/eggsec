# Eggsec Loadtest Skill

HTTP load testing module workflows and patterns.

## Key Types and Patterns

### LoadTestRunner (`loadtest/runner.rs`)
Main load test executor that manages concurrent HTTP request workers.

### Metrics (`loadtest/metrics.rs`)
Real-time metrics collection using `hdrhistogram` for latency percentiles.

### LoadTestResults (`loadtest/metrics.rs`)
Aggregated results with percentiles (p50, p90, p95, p99).

### Worker Model
- Uses `tokio::task::JoinSet` for concurrent workers
- `worker_count = min(concurrency, total_requests)`
- Each worker loops, fetching `request_index = issued_requests.fetch_add(1, Ordering::Relaxed)`
- Rate limiting via optional `rate_limit` (requests/sec)

## Testing

### Running Loadtest Tests
```bash
cargo test --test loadtest_tests -p eggsec
cargo test --lib -p eggsec loadtest
```

### Writing Tests
Follow existing patterns in `tests/loadtest_tests.rs`:
- Use `create_test_server()` + `mock_ok()` from test helpers
- Test basic, concurrency, error handling, validation

## Common Tasks

### Adding a New Load Test Configuration Option
1. Add field to `LoadArgs` in `cli/http.rs`
2. Handle in `LoadTestRunner::from_args*` methods
3. Apply in `run()` method
4. Add tests for new option

### Adding a New Metric
1. Add field to `LoadTestResults` in `metrics.rs`
2. Track in `Metrics` struct
3. Populate in `to_results()`
4. Display in `Display` impl and serialize in `Serialize` impl

## CLI Usage

```bash
# Basic load test
eggsec load https://example.com -n 1000 -c 50

# With body and headers
eggsec load https://example.com/api -n 500 -c 20 -m POST -d '{"key":"value"}' -H 'Content-Type:application/json'

# With rate limiting
eggsec load https://example.com -n 10000 -c 100 --rate-limit 50

# JSON output
eggsec load https://example.com -n 1000 -c 50 --json

# Output to file
eggsec load https://example.com -n 1000 -c 50 -o results.json
```

## Metrics Collected

- `total_requests` - Total requests sent
- `successful_requests` - 2xx-3xx responses
- `failed_requests` - 4xx-5xx + network errors
- `requests_per_second` - Throughput
- `latency_min_ms`, `latency_mean_ms`, `latency_max_ms` - Latency stats
- `latency_p50_ms`, `latency_p90_ms`, `latency_p95_ms`, `latency_p99_ms` - Percentiles
- `status_codes` - Map of HTTP status code to count
- `errors` - Error messages (first 5 displayed)

## Code Conventions

- Use `rustc_hash::FxHashMap` for `status_codes` maps (not `std::collections::HashMap`)
- Histogram uses `hdrhistogram::Histogram<u64>` with 3 significant figures
- Suppress histogram errors with `let _ = self.histogram.record(...)` not `.ok()`
- Auth headers handled via `apply_auth_headers()` helper
- Non-success response bodies are consumed to avoid memory leaks in the connection pool
- Rate limiting uses a global lock with proper interval calculation to avoid drift

## Bugs Fixed

### 2026-05-28 (Wave 1 & 2)

| File | Issue | Fix |
|------|-------|-----|
| `runner.rs:275-281` | Rate limiting initial burst | Changed `now() - min_interval` to `now() + min_interval` |
| `runner.rs:306-317` | Rate limit lock contention | Replaced `Arc<Mutex<TokioInstant>>` with `tokio::sync::Semaphore` token bucket |
| `runner.rs:322-333` | Missing request cancellation on timeout | Added `CancellationToken` checked each loop iteration |

## Implementation Notes

### Response Body Handling
When a non-success response is received (4xx, 5xx), the response body is consumed before recording the metrics. This prevents the underlying HTTP client connection from being closed prematurely and returned to the pool in an inconsistent state.

### Rate Limiting Algorithm
Rate limiting uses a lock-protected token bucket approach:
1. Worker acquires lock on `next_allowed_at`
2. If `now < next`, sleep until `next`
3. Update `next = now_after_sleep + interval` (not `next + interval`) to maintain correct rate

### Dead Code
`Metrics::record_success()` exists for external callers but is not used internally by `LoadTestRunner`. It does not check HTTP status codes - it blindly records as successful. Prefer `record_http_response()` which correctly distinguishes 2xx-3xx as success.

## Resources
- `crates/eggsec/src/loadtest/` - Module source
- `architecture/loadtest.md` - Architecture documentation
- `AGENTS.md` - General project guidelines
