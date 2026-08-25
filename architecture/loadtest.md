# Load Testing Module

## Overview

The load testing module provides HTTP performance benchmarking — measuring server throughput, latency percentiles, and error rates under controlled concurrency. Unlike the [stress module](stress.md) (which generates raw network floods for defense-lab DoS simulation), loadtest issues real HTTP requests through `reqwest` and collects precise latency histograms via `hdrhistogram`.

**Feature gate:** None on the module itself (always compiled). The CLI entry point `run_cli()` is gated behind the `cli` feature (`mod.rs:62`).

**Role:** HTTP performance testing — RPS, latency percentiles (p50/p90/p95/p99), status code distribution, error tracking.

## Module Structure

| File | Lines | Feature-gated | Purpose |
|------|-------|---------------|---------|
| `mod.rs` | 107 | `cli` (for `run_cli()`) | Module entry, `run_cli()` CLI entry point |
| `runner.rs` | 507 | no | `LoadTestRunner` — worker/concurrency model, rate limiting, request execution |
| `metrics.rs` | 142 | no | `Metrics` + `LoadTestResults` — hdrhistogram latency tracking, percentile extraction |

**Total:** 3 files (+ `AGENTS.override.md`), 656 lines (code only).

## Key Types

### `LoadTestRunner`

Main executor (`runner.rs:76-90`):

```rust
pub struct LoadTestRunner {
    url: String,
    total_requests: u64,
    concurrency: usize,
    timeout: Duration,
    method: Method,
    body: Option<Bytes>,
    headers: Vec<(String, String)>,
    insecure: bool,
    proxy: Option<String>,
    proxy_auth: Option<String>,
    user_agent: String,
    rate_limit: Option<u32>,
    tui_mode: bool,
}
```

#### Constructors

| Method | Purpose |
|--------|---------|
| `new(url, total, concurrency, timeout)` | Basic constructor with validation (`runner.rs:93-100`) |
| `new_with_tui_mode(...)` | Constructor with explicit TUI mode flag (`runner.rs:102-140`) |
| `from_config(cfg)` | From plain `LoadTestRunConfig` (`runner.rs:156-158`) |
| `from_config_with_mode(cfg, tui_mode)` | `LoadTestRunConfig` with TUI mode (`runner.rs:162-185`) |
| `from_config_with_engine(cfg, config)` | `LoadTestRunConfig` merged with `EggsecConfig` — used by pipeline (`runner.rs:191-204`) |
| `from_args_with_config(args, config)` | CLI `LoadArgs` merged with `EggsecConfig` (`runner.rs:148-153`) |
| `from_args_with_tui_mode(args, tui_mode)` | CLI args with TUI mode (`runner.rs:143-145`) |

**Important:** Use `from_config_with_engine()` for pipeline integration to ensure config file settings (proxy, TLS verification, rate limits) are properly merged (`runner.rs:191-204`).

#### Validation

The constructor validates (`runner.rs:109-123`):
- `concurrency > 0`
- `total_requests > 0`
- `timeout > 0`

The `apply_common()` method validates (`runner.rs:250-263`):
- `rate_limit > 0` (0 is ignored with a warning)
- Rate limit > 100,000 logs a warning about potential ineffectiveness

### `LoadTestRunConfig`

Plain configuration type (no Clap derives) for engine/pipeline/Python consumers (`runner.rs:26-57`):

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | `String` | — | Target URL |
| `requests` | `u64` | — | Total request count |
| `concurrency` | `usize` | — | Max concurrent workers |
| `timeout` | `Duration` | — | Per-request timeout |
| `method` | `String` | `"GET"` | HTTP method |
| `body` | `Option<String>` | `None` | Request body |
| `headers` | `Vec<String>` | `[]` | Raw header strings |
| `common` | `CommonHttpArgs` | default | Proxy, TLS, auth, rate-limit, user-agent |
| `tui_mode` | `bool` | `false` | Suppress progress bar |

### `LoadTestResults`

Serializable output (`metrics.rs:7-24`):

| Field | Type | Description |
|-------|------|-------------|
| `target_url` | `String` | Target URL |
| `total_requests` | `u64` | Total issued |
| `successful_requests` | `u64` | HTTP 2xx/3xx |
| `failed_requests` | `u64` | HTTP 4xx/5xx or transport errors |
| `total_duration_ms` | `u64` | Wall-clock duration |
| `requests_per_second` | `f64` | Total / duration_secs |
| `latency_min_ms` | `f64` | Histogram minimum |
| `latency_max_ms` | `f64` | Histogram maximum |
| `latency_mean_ms` | `f64` | Histogram mean |
| `latency_p50_ms` | `f64` | 50th percentile (`metrics.rs:134`) |
| `latency_p90_ms` | `f64` | 90th percentile (`metrics.rs:135`) |
| `latency_p95_ms` | `f64` | 95th percentile (`metrics.rs:136`) |
| `latency_p99_ms` | `f64` | 99th percentile (`metrics.rs:137`) |
| `status_codes` | `FxHashMap<u16, u64>` | Status code distribution |
| `errors` | `Vec<String>` | Error messages (capped at 1000) |

Implements `Display` (`metrics.rs:26-63`) with sorted status codes and first-5 error display.

Implements `Report` trait (`runner.rs:499-507`) with `title() -> "Load Test Report"` and `to_json()`.

## Behavior & Flow

### Request execution flow (`runner.rs:332-496`)

1. **TLS provider installation:** `crate::install_tls_provider()` (`runner.rs:333`).
2. **Client construction:** Builds `reqwest::Client` with timeout, TLS settings, optional proxy (`runner.rs:340-357`).
3. **Metrics init:** `Metrics::new(url)` creates hdrhistogram with precision 3 (`runner.rs:359`).
4. **Progress bar:** Created unless `tui_mode` is true (`runner.rs:361-372`).
5. **Rate limit semaphore:** If configured, spawns background task that adds 1 permit every `1/rate` seconds (`runner.rs:379-397`).
6. **Worker spawning:** `worker_count = min(concurrency, total_requests)` workers spawned into `JoinSet` (`runner.rs:399-475`).
7. **Worker loop:** Each worker:
   - Checks `CancellationToken` (`runner.rs:418-419`)
   - Acquires atomic request index (`runner.rs:422-424`)
   - Acquires rate limit permit if configured (`runner.rs:427-434`)
   - Records request start time (`runner.rs:436`)
   - Sends request with headers, body, user-agent (`runner.rs:438-449`)
   - On success: drains response body (connection pool reuse), records latency + status (`runner.rs:452-461`)
   - On failure: records latency + error (`runner.rs:463-467`)
   - Increments progress bar (`runner.rs:470-472`)
8. **Join:** Workers joined via `JoinSet::join_next()` with panic/error logging (`runner.rs:477-485`).
9. **Cleanup:** Cancel rate-limit task, finish progress bar, compute results (`runner.rs:487-495`).

### Worker/concurrency model

- Workers are `tokio::task::JoinSet` tasks (`runner.rs:400`).
- Each worker independently pulls work via atomic counter (`AtomicU64`, `Ordering::Relaxed`) — no work stealing.
- Worker count is bounded by `min(concurrency, total_requests)` (`runner.rs:399`).
- Graceful shutdown via `CancellationToken` checked at loop top (`runner.rs:418-419`). Token is cancelled after all workers complete (`runner.rs:487`).

### Histogram recording & percentile extraction

- `hdrhistogram::Histogram<u64>` with 3 significant figures (`metrics.rs:78`).
- Latency recorded in milliseconds: `latency.as_millis() as u64` (`metrics.rs:89,106`).
- `record()` errors logged with `tracing::warn!` (never suppressed) (`metrics.rs:90-92,107-109`).
- Percentile extraction at `metrics.rs:134-137`:
  ```rust
  latency_p50_ms: self.histogram.value_at_percentile(50.0) as f64,
  latency_p90_ms: self.histogram.value_at_percentile(90.0) as f64,
  latency_p95_ms: self.histogram.value_at_percentile(95.0) as f64,
  latency_p99_ms: self.histogram.value_at_percentile(99.0) as f64,
  ```
- Min/max/mean extracted from histogram (`metrics.rs:131-133`).
- Status codes tracked via `FxHashMap<u16, u64>` for performance (`metrics.rs:70`).
- Error messages capped at 1000 entries (`metrics.rs:99,112`).

### Rate limiting algorithm

Semaphore token bucket approach (`runner.rs:379-397`):

1. A semaphore starts with **0 permits** (`Semaphore::new(0)`).
2. A background task adds 1 permit every `min_interval` (`1/rate` seconds) using `tokio::select!` with `CancellationToken` for clean shutdown.
3. Worker acquires a permit via `acquire().await` and calls `forget()` to **permanently consume** it (`runner.rs:428-429`).
4. If no permits available, worker blocks until one is added (backpressure).

Using `forget()` is critical — returning the permit would allow immediate reacquisition, defeating rate limiting. This ensures RPS stays close to the configured limit even under high concurrency without lock contention.

### Response body handling

All response bodies (success and error) are consumed before returning connections to the pool (`runner.rs:457-459`). This prevents HTTP client connection pool starvation where a connection has an unread body waiting.

### Warm-up

**Note:** The overview.md module index mentions "warm-up" but no warm-up phase is implemented in the current source. Requests are issued at full concurrency from the first iteration. There is no ramp-up period.

## Safety & Authorization

Unlike the [stress module](stress.md), loadtest does not require scope authorization, root privileges, or explicit feature flags. It is a standard HTTP client tool.

Built-in safety measures:
1. **Constructor validation:** Rejects `concurrency == 0`, `total_requests == 0`, `timeout == 0` (`runner.rs:109-123`).
2. **Per-request timeout:** `reqwest::Client` configured with `timeout` (`runner.rs:341`).
3. **Rate limit validation:** 0 is ignored with warning; > 100,000 logs ineffectiveness warning (`runner.rs:250-263`).
4. **Error caps:** Error message list capped at 1000 entries (`metrics.rs:99,112`).
5. **TUI mode:** Progress bar suppressed when running inside TUI to avoid terminal conflicts (`runner.rs:361-362`).

## Probe Risk & Pipeline Integration

Load testing is tagged with `ProbeIntent::LoadBearing` and `ProbeRisk::Stress` (risk level 4) in the shared probe classification system (`crates/eggsec/src/probe.rs`, `architecture/probe.md:20-21,33,46`). This means:

- **Defense-lab profiles** must explicitly include load-bearing probes and budget for `Stress`-level risk.
- **Pipeline scheduling** enforces feature gates and scope requirements for load test stages.

## Public API

```rust
// Basic usage
let runner = LoadTestRunner::new(url, 1000, 50, Duration::from_secs(30))?;
let results = runner.run().await?;

// With config merge (pipeline)
let runner = LoadTestRunner::from_config_with_engine(cfg, &config)?;

// With auth
runner.set_method("POST".to_string());
runner.set_body("{}".to_string());
runner.add_header("Content-Type".to_string(), "application/json".to_string());
runner.set_common(common_args);
```

## Integration Points

- **CLI:** `run_cli()` (`mod.rs:63-107`) — parses `LoadArgs`, creates runner via `from_config_with_engine()`, executes, outputs results.
- **TUI:** `LoadTestRunner` with `tui_mode: true` suppresses progress bar.
- **Pipeline:** `from_config_with_engine()` merges `EggsecConfig` defaults (proxy, TLS, headers).
- **Distributed:** Worker processes return load test results in JSON (`architecture/distributed.md`).
- **Report trait:** `LoadTestResults` implements `Report` for JSON export (`runner.rs:499-507`).
- **Dispatch:** `LoadTest` task kind dispatched via `runtime_bridge` from daemon/runtime surfaces (`architecture/runtime_bridge.md`).

## Testing

```bash
cargo test --test loadtest_tests -p eggsec
cargo clippy --lib -p eggsec
```

## Invariants & Gotchas

1. **No warm-up phase:** Requests start at full concurrency immediately. No ramp-up.
2. **Latency = time-to-first-byte:** Measured from request send to response headers received, not full body transfer.
3. **Error classification:** HTTP 2xx/3xx = successful; 4xx/5xx = failed (`metrics.rs:95-102`). Transport errors are also failures.
4. **FxHashMap for status codes:** Uses `rustc_hash::FxHashMap` for performance over `std::collections::HashMap` (`metrics.rs:3,70`).
5. **Histogram precision:** 3 significant figures — sufficient for ms-level latency tracking but not sub-microsecond.
6. **No timeouts on spawned tasks:** Worker tasks in the `JoinSet` and the rate-limit background task (`runner.rs:384,400`) are spawned without explicit `tokio::time::timeout` wrappers. Workers are bounded by the atomic request counter and `CancellationToken`, but a hung `reqwest` request could block a worker until the per-request timeout fires. The rate-limit task is bounded by `CancellationToken`. This violates the AGENTS.md invariant that all spawned tokio tasks need timeout wrappers.

## See Also

- [overview.md](overview.md) — system-wide module index
- [probe.md](probe.md) — shared probe intent/risk vocabulary
- [defense_lab.md](defense_lab.md) — defense-lab profiles and risk budgets
- [stress.md](stress.md) — raw network flood testing (SYN/UDP/ICMP/HTTP DoS simulation)

*Last verified against source: 2026-08-25*
