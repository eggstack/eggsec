# Load Testing Module

The Load Testing module provides high-performance HTTP benchmarking and stress testing capabilities.

## Core Components (`src/loadtest/`)

### Runner (`runner.rs`)

The `runner.rs` file contains the core logic for generating high volumes of HTTP requests.

- **High Concurrency**: Leverages Rust's async/await and `tokio` to manage thousands of concurrent connections efficiently using `JoinSet`.
- **Worker Model**: Spawns `min(concurrency, total_requests)` workers, each issuing requests via atomic counter.
- **Configurable Workload**: Supports different request methods, headers, and body content.
- **Fixed-Request Workload**: Executes a configured total request count with bounded concurrency.
- **Rate Limiting**: Optionally caps global request issuance with `--rate-limit`.
- **Auth Headers**: Helper `apply_auth_headers()` method handles Basic, Bearer, Cookie, API Key authentication.

### Metrics (`metrics.rs`)

Collects and processes performance data in real-time.

- **Latency Tracking**: Records response times and calculates percentiles (p50, p90, p95, p99) using `hdrhistogram`.
- **Throughput**: Measures requests per second (RPS).
- **Error Rates**: Tracks non-2xx/3xx status codes and transport failures.
- **Histograms**: Uses `hdrhistogram::Histogram<u64>` for efficient and accurate latency tracking.
- **FxHashMap**: Uses `rustc_hash::FxHashMap` for status code distribution (performance optimization).

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
