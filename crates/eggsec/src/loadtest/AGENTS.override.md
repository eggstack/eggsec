# Loadtest Module Override

## Key Files

| File | Purpose |
|------|---------|
| `crates/eggsec/src/loadtest/mod.rs` | Module entry, `run_cli()` entry point |
| `crates/eggsec/src/loadtest/runner.rs` | `LoadTestRunner` - worker/concurrency model, rate limiting |
| `crates/eggsec/src/loadtest/metrics.rs` | `Metrics` + `LoadTestResults` - latency histogram, percentiles |

## Performance: Use FxHashMap

For performance-critical code, use `rustc_hash::FxHashMap` instead of `std::collections::HashMap`:

```rust
use rustc_hash::FxHashMap;

let mut status_codes: FxHashMap<u16, u64> = FxHashMap::default();
```

## Code Conventions

1. **Worker Model**: Uses `tokio::task::JoinSet` with `worker_count = min(concurrency, total_requests)`
2. **Rate Limiting**: Semaphore token bucket with `forget()` for permanent permit consumption
3. **Metrics**: Uses `hdrhistogram::Histogram<u64>` for latency percentiles (p50, p90, p95, p99)
4. **Response Body Handling**: All response bodies (success and error) consumed to enable connection reuse
5. **Histogram Errors**: Log with `tracing::warn!` on failure, never suppress with `let _ =`

## Rate Limiting

Rate limiting uses a **semaphore token bucket** with `forget()` to permanently consume permits:

1. A semaphore starts with 0 permits
2. A background task adds 1 permit every `min_interval` (1/rate seconds), using `CancellationToken` for clean shutdown
3. Workers acquire a permit and call `forget()` to permanently consume it (preventing reacquisition)
4. Workers that can't acquire a permit block until one is available
5. Rate limit of 0 is rejected with a warning and ignored

## Validation

The constructor validates:
- `concurrency > 0`
- `total_requests > 0`
- `timeout > 0`

The `set_common*` methods validate:
- `rate_limit > 0` (0 is ignored with a warning)

## Latency Measurement

- Latency is measured as time-to-first-byte (from request send to headers received)
- Latency is recorded for **both** successful and failed requests
- When all requests fail, the Display impl shows "no successful requests" instead of misleading 0ms values

## Bug Fixes

| Date | File | Issue | Fix |
|------|------|-------|-----|
| 2026-05-22 | `runner.rs` | `from_args_with_config()` used `new()` bypassing validation | Changed to `new_with_tui_mode()` |
| 2026-05-22 | `runner.rs` | Non-success response bodies not consumed | Now consumes all response bodies |
| 2026-05-28 | `metrics.rs` | `expect()` panic message was incorrect | Use `expect("Failed to create hdrhistogram")` |
| 2026-06-03 | `metrics.rs` | Silent error suppression with `let _ =` on `histogram.record()` | Now logs with `tracing::warn!` |
| 2026-06-03 | `runner.rs` | Rate limiting acquire discarded permit, allowing reacquisition | Now uses `permit.forget()` |
| 2026-06-03 | `runner.rs` | Semaphore started with `rate` permits, allowing unbounded burst | Changed to start with 0 permits |
| 2026-06-05 | `runner.rs` | `--rate-limit 0` caused panic (division by zero in Duration) | Added validation, 0 is ignored with warning |
| 2026-06-05 | `runner.rs` | Rate limit background task leaked (infinite loop, no cancellation) | Added `CancellationToken` to break loop on shutdown |
| 2026-06-05 | `runner.rs` | Successful response bodies not consumed — connection pool starvation | Now consumes all response bodies |
| 2026-06-05 | `runner.rs` | Latency not recorded for failed requests — misleading percentiles | Now records latency for all requests |
| 2026-06-05 | `runner.rs` | Unknown HTTP methods silently defaulted to GET | Now logs warning for unknown methods |
| 2026-06-05 | `runner.rs` | Malformed auth credentials silently ignored | Now logs warning for invalid auth format |
| 2026-06-05 | `runner.rs` | CancellationToken/abort_all() were dead code (called after workers drained) | Removed no-op calls, token used only for rate limit task |
| 2026-06-05 | `metrics.rs` | `record_success()` was dead code — never called | Removed method |
| 2026-06-05 | `metrics.rs` | Empty histogram produced misleading 0ms latency values | Display impl shows "no successful requests" when histogram empty |
| 2026-06-05 | `metrics.rs` | Status codes displayed in non-deterministic HashMap order | Now sorts by status code before display |
| 2026-06-05 | `metrics.rs` | `let _ = writeln!` inconsistent with rest of Display impl | Now uses `?` consistently |
| 2026-06-05 | `tool/loadtest.rs` | Timeout error reported `timeout_ms: 0` instead of `60000` | Fixed to report actual timeout |
| 2026-06-05 | `tool/loadtest.rs` | `estimated_duration_ms` (120s) inconsistent with actual timeout (60s) | Fixed to 60_000 |
| 2026-06-05 | `distributed/worker.rs` | Load test results discarded — coordinator received no metrics | Now returns full results in JSON response |

## Testing

```bash
cargo test --test loadtest_tests -p eggsec
cargo clippy --lib -p eggsec
```
