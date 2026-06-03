# Load Testing Module

The Load Testing module provides high-performance HTTP benchmarking and stress testing capabilities.

## Core Components (`src/loadtest/`)

### Runner (`runner.rs`)

The `runner.rs` file contains the core logic for generating high volumes of HTTP requests.

- **High Concurrency**: Leverages Rust's async/await and `tokio` to manage thousands of concurrent connections efficiently using `JoinSet`.
- **Worker Model**: Spawns `min(concurrency, total_requests)` workers, each issuing requests via atomic counter.
- **Configurable Workload**: Supports different request methods, headers, and body content.
- **Fixed-Request Workload**: Executes a configured total request count with bounded concurrency.
- **Rate Limiting**: Optionally caps global request issuance with `--rate-limit`. Uses proper interval calculation to avoid timing drift.
- **Auth Headers**: Helper `apply_auth_headers()` method handles Basic, Bearer, Cookie, API Key authentication.
- **Connection Pool**: Non-success response bodies are consumed before returning connections to the pool.
- **Graceful Shutdown**: Uses `CancellationToken` (`tokio_util::sync::CancellationToken`) to signal worker tasks to stop. The token is cloned per-worker and checked at the top of each worker's loop (`runner.rs:305`). On completion, the token is cancelled and all workers are aborted via `JoinSet::abort_all()`.
- **Progress Bar**: Uses `indicatif::ProgressBar` with `ProgressStyle` template `[{spinner:.green}] [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})`. Progress bar is disabled in TUI mode to avoid terminal conflicts.
- **Report Trait**: `LoadTestResults` implements `Report` trait (`runner.rs:380-388`) with `title() -> "Load Test Report"` and `to_json()` using `serde_json::to_string_pretty()`.

#### LoadTestRunner Structure

```rust
pub struct LoadTestRunner {
    url: String,
    total_requests: u64,
    concurrency: usize,
    timeout: Duration,
    method: Method,
    body: Option<String>,
    headers: Vec<(String, String)>,
    insecure: bool,
    proxy: Option<String>,
    proxy_auth: Option<String>,
    user_agent: String,
    rate_limit: Option<u32>,    // requests per second
    tui_mode: bool,
}
```

#### Constructors

| Method | Purpose |
|--------|---------|
| `new(url, total, concurrency, timeout)` | Basic constructor with validation |
| `new_with_tui_mode(...)` | Constructor with explicit TUI mode flag |
| `from_args(args)` | From CLI `LoadArgs` |
| `from_args_with_tui_mode(args, tui_mode)` | CLI args with TUI mode |
| `from_args_with_config(args, config)` | CLI args merged with `SlapperConfig` (used by pipeline) |

**Important**: Use `from_args_with_config()` for pipeline integration to ensure config file settings (proxy, TLS verification, rate limits) are properly merged.

### Metrics (`metrics.rs`)

Collects and processes performance data in real-time.

### CLI Entry Point (`mod.rs`)

The `mod.rs` file provides the `run_cli()` function which serves as the CLI entry point for the load test module.

- **Argument Parsing**: Uses `clap` to parse `LoadArgs` from command-line arguments.
- **Runner Instantiation**: Creates `LoadTestRunner` via `from_args()` or `from_args_with_config()`.
- **Execution**: Runs the load test and handles results output.
- **Error Handling**: Returns a `TabError` for UI integration or exits with appropriate error code on failure.

#### `run_cli()` Function

```rust
pub async fn run_cli(args: LoadArgs, config: &SlapperConfig) -> Result<()>
```

Parses CLI arguments, instantiates a runner, executes the load test, and outputs results. Used by the main CLI dispatcher.

- **Latency Tracking**: Records response times and calculates percentiles (p50, p90, p95, p99) using `hdrhistogram`.
- **Throughput**: Measures requests per second (RPS).
- **Error Rates**: Tracks non-2xx/3xx status codes and transport failures.
- **Histograms**: Uses `hdristogram::Histogram<u64>` with 3 significant figures for efficient and accurate latency tracking.
- **FxHashMap**: Uses `rustc_hash::FxHashMap` for status code distribution (performance optimization).

#### LoadTestResults Structure

```rust
pub struct LoadTestResults {
    pub target_url: String,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_duration_ms: u64,
    pub requests_per_second: f64,
    pub latency_min_ms: f64,
    pub latency_max_ms: f64,
    pub latency_mean_ms: f64,
    pub latency_p50_ms: f64,
    pub latency_p90_ms: f64,
    pub latency_p95_ms: f64,
    pub latency_p99_ms: f64,
    pub status_codes: FxHashMap<u16, u64>,
    pub errors: Vec<String>,
}
```

## Usage

Load testing is typically invoked via the `load` subcommand:

```bash
slapper load https://target.com --requests 10000 --concurrency 100
```

With authentication:
```bash
slapper load https://target.com/api -n 5000 -c 50 -H "Authorization:Bearer token123"
```

With body and custom headers:
```bash
slapper load https://target.com/api -n 1000 -c 20 -m POST -d '{"query":"test"}' -H 'Content-Type:application/json'
```

## Integration

Load testing can be combined with **Fuzzing** to see how a target behaves under stress, or used independently to benchmark web server performance.

### Rate Limiting Algorithm

Rate limiting uses a **semaphore token bucket** approach:

1. A semaphore starts with 0 permits
2. A background task adds 1 permit to the semaphore every `min_interval` (1/rate seconds)
3. Worker acquires a permit via `acquire().await` and calls `forget()` to permanently consume it
4. If no permits available, worker blocks until one is added (backpressure)

Using `forget()` is critical — returning the permit would allow immediate reacquisition, defeating rate limiting. This ensures RPS stays close to the configured limit even under high concurrency without lock contention.

### Response Body Handling

When non-success responses (4xx/5xx) are received, the response body is consumed before recording metrics. This prevents the HTTP client's connection pool from being left in an inconsistent state where a connection has an unread body waiting.

## Override File

For specialized guidance on the loadtest module, see:
- `crates/slapper/src/loadtest/AGENTS.override.md` - Module-specific patterns and bug fixes
