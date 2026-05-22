# Slapper Loadtest Skill

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
cargo test --test loadtest_tests -p slapper
cargo test --lib -p slapper loadtest
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
slapper load https://example.com -n 1000 -c 50

# With body and headers
slapper load https://example.com/api -n 500 -c 20 -m POST -d '{"key":"value"}' -H 'Content-Type:application/json'

# With rate limiting
slapper load https://example.com -n 10000 -c 100 --rate-limit 50

# JSON output
slapper load https://example.com -n 1000 -c 50 --json

# Output to file
slapper load https://example.com -n 1000 -c 50 -o results.json
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

## Resources
- `crates/slapper/src/loadtest/` - Module source
- `architecture/loadtest.md` - Architecture documentation
- `AGENTS.md` - General project guidelines
