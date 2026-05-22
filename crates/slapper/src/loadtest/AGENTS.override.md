# Loadtest Module Override

## Key Files

| File | Purpose |
|------|---------|
| `crates/slapper/src/loadtest/mod.rs` | Module entry, `run_cli()` entry point |
| `crates/slapper/src/loadtest/runner.rs` | `LoadTestRunner` - worker/concurrency model, rate limiting |
| `crates/slapper/src/loadtest/metrics.rs` | `Metrics` + `LoadTestResults` - latency histogram, percentiles |

## Performance: Use FxHashMap

For performance-critical code, use `rustc_hash::FxHashMap` instead of `std::collections::HashMap`:

```rust
use rustc_hash::FxHashMap;

let mut status_codes: FxHashMap<u16, u64> = FxHashMap::default();
```

## Recent Bug Fixes (2026-05-22)

| File | Issue | Fix |
|------|-------|-----|
| `runner.rs:116-122` | `from_args_with_config()` used `new()` bypassing validation | Changed to `new_with_tui_mode()` to ensure validation |
| `runner.rs:275-317` | Rate limiting interval calculation could drift | Changed to `now_after_sleep + interval` instead of `next + interval` |
| `runner.rs:341-345` | Non-success response bodies not consumed | Now consumes response body before recording metrics |

## Code Conventions

1. **Worker Model**: Uses `tokio::task::JoinSet` with `worker_count = min(concurrency, total_requests)`
2. **Rate Limiting**: Lock-protected token bucket with proper drift correction
3. **Metrics**: Uses `hdrhistogram::Histogram<u64>` for latency percentiles (p50, p90, p95, p99)
4. **Response Body Handling**: Non-success bodies consumed to prevent connection pool issues
5. **Histogram Errors**: Suppress with `let _ = self.histogram.record(...)` not `.ok()`

## Testing

```bash
cargo test --test loadtest_tests -p slapper
cargo clippy --lib -p slapper
```