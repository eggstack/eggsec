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

## Bug Fix (2026-05-28)

| File | Issue | Fix |
|------|-------|-----|
| `metrics.rs:76` | Panic message "3 significant figures is invalid" is incorrect | Use `expect("Failed to create hdrhistogram")` instead |

## Rate Limiting (Updated 2026-06-03)

Rate limiting uses a **semaphore token bucket** with `forget()` to permanently consume permits:

1. A semaphore starts with 0 permits
2. A background task adds 1 permit every `min_interval` (1/rate seconds)
3. Workers acquire a permit and call `forget()` to permanently consume it (preventing reacquisition)
4. Workers that can't acquire a permit block until one is available

This ensures RPS stays close to the configured limit. Using `forget()` is critical — returning the permit to the semaphore would allow immediate reacquisition, defeating rate limiting.

## Bug Fix (2026-06-03)

| File | Issue | Fix |
|------|-------|-----|
| `metrics.rs:87,94` | Silent error suppression with `let _ =` on `histogram.record()` | Now logs with `tracing::warn!` on failure |
| `runner.rs:314-317` | Rate limiting acquire discarded the permit, allowing immediate reacquisition | Now uses `permit.forget()` to permanently consume permits, enforcing actual rate limits |
| `runner.rs:271-282` | Semaphore started with `rate` permits allowing unbounded initial burst | Changed to start with 0 permits; background task adds permits at configured rate |
| `tool/implementations/loadtest.rs:80-108` | `LoadTestTool::execute()` discarded actual `LoadTestResults` | Now runs via `LoadTestRunner::from_args_with_config()` directly and returns serialized results |

## Testing

```bash
cargo test --test loadtest_tests -p slapper
cargo clippy --lib -p slapper
```