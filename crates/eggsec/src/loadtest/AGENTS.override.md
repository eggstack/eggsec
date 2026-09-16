# Loadtest Module Override

Phase D (2026-09-16): transport-neutral core + engine adapters. The core
(`plan`, `executor`, `metrics`, `progress`) never touches Reqwest, indicatif,
Clap, `EggsecConfig`, or the filesystem. `LoadTestRunner` is a compatibility
facade (`tui_mode` retained but ignored).

## Key Files

| File | Purpose |
|------|---------|
| `crates/eggsec/src/loadtest/mod.rs` | Module entry, `run_cli()` / `run_cli_with_scope()` (own the only `indicatif` widget; `cli`-gated) |
| `crates/eggsec/src/loadtest/plan.rs` | `LoadTestPlan`, `RatePolicy` — transport-neutral plan |
| `crates/eggsec/src/loadtest/executor.rs` | `LoadTestExecutor<T: HttpTransport>` — generic executor, sharded metrics, CAS pacer |
| `crates/eggsec/src/loadtest/metrics.rs` | `Metrics` (pure + `merge`) + `LoadTestResults` + `LoadTestErrorKind` |
| `crates/eggsec/src/loadtest/progress.rs` | `ProgressSink` (`NoopSink`/`FnSink`/`ChannelSink`), structured events |
| `crates/eggsec/src/loadtest/adapter.rs` | `RequestTemplate`, `plan_from_adapter()` — engine translation above the core |
| `crates/eggsec/src/loadtest/backend.rs` | `ReqwestTransport`, `OwnedScopeAuthority` — scope-aware Reqwest `HttpTransport` |
| `crates/eggsec/src/loadtest/runner.rs` | `LoadTestRunner`, `LoadTestRunConfig` — compatibility facades |

## Performance: Use FxHashMap

For performance-critical code, use `rustc_hash::FxHashMap` instead of `std::collections::HashMap`:

```rust
use rustc_hash::FxHashMap;

let mut status_codes: FxHashMap<u16, u64> = FxHashMap::default();
```

## Code Conventions

1. **Worker Model**: Uses `tokio::task::JoinSet` with `worker_count = min(concurrency, total_requests)`; each worker owns a private `Metrics`, merged at end
2. **Rate Limiting**: CAS slot allocator (`GlobalPacer`) — aggregate rate holds at any worker count, no mutex on hot path, cancellation-preemptible
3. **Metrics**: Uses `hdrhistogram::Histogram<u64>` for latency percentiles (p50, p90, p95, p99); `merge` via `histogram.add` (exact)
4. **Response Body Handling**: All response bodies drained before recording to enable connection reuse
5. **Histogram Errors**: Log with `tracing::warn!` on failure, never suppress with `let _ =`
6. **Transport errors**: Classify into `LoadTestErrorKind` (neutral kinds); never expose backend error types
7. **Progress**: Structured `ProgressSink` events only; core never prints and never imports `indicatif`
8. **No new crates**: Gate D1/D2 rejected `eggsec-loadtest` / `eggsec-resilience` (see `architecture/capability_segregation.md`); keep the internal boundary

## Validation

The plan constructor validates:
- `concurrency > 0`
- `total_requests > 0`
- `timeout > 0`
- URL has http/https scheme, host present, no userinfo

The adapter validates:
- `rate_limit > 0` (0 is unlimited with a warning)

## Latency Measurement

- Latency is measured per dispatch (request start to response completion incl. body drain)
- Latency is recorded for **both** successful and failed requests
- Transport-error categories travel in `error_kinds` (serde-defaulted for back-compat)

## Testing

```bash
cargo test --test loadtest_tests -p eggsec
cargo test --lib -p eggsec loadtest
cargo clippy --lib -p eggsec
```
