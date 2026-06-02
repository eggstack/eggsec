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

## Rate Limiting (Updated 2026-05-28)

Rate limiting was refactored from mutex-based to semaphore token bucket approach:

```rust
let rate_limit_sem = self.rate_limit.map(|rate| {
    let sem = Arc::new(Semaphore::new(rate as usize));
    let min_interval = Duration::from_secs_f64(1.0 / f64::from(rate));
    let sem_clone = sem.clone();
    tokio::spawn(async move {
        loop {
            sleep(min_interval).await;
            sem_clone.add_permits(1);
        }
    });
    sem
});
```

Workers acquire permits before processing. Initial burst is prevented because semaphore starts with exactly `rate` permits.

## HIGH Priority Issue (Pending Fix)

**Semaphore Unwrap Could Panic at `runner.rs:315`:**

```rust
let _permit = sem.acquire().await.unwrap();
```

The semaphore acquire uses `.unwrap()` which could panic if the semaphore is closed. Handle the error explicitly instead of unwrapping - use match or map_err with tracing:

```rust
let permit = sem.acquire().await.map_err(|e| {
    tracing::error!("Failed to acquire rate limit semaphore: {}", e);
    e
})?;
```

This is a potential panic point under error conditions.

## Testing

```bash
cargo test --test loadtest_tests -p slapper
cargo clippy --lib -p slapper
```