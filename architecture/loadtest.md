# Load Testing Module

The Load Testing module provides high-performance HTTP benchmarking and stress testing capabilities.

## Core Components (`src/loadtest/`)

### Runner (`runner.rs`)

The `runner.rs` file contains the core logic for generating high volumes of HTTP requests.

- **High Concurrency**: Leverages Rust's async/await and `tokio` to manage thousands of concurrent connections efficiently.
- **Configurable Workload**: Supports different request methods, headers, and body content.
- **Fixed-Request Workload**: Executes a configured total request count with bounded concurrency.
- **Rate Limiting**: Optionally caps global request issuance with `--rate-limit`.

### Metrics (`metrics.rs`)

Collects and processes performance data in real-time.

- **Latency Tracking**: Records response times and calculates percentiles (p50, p90, p99).
- **Throughput**: Measures requests per second (RPS).
- **Error Rates**: Tracks non-2xx/3xx status codes and transport failures.
- **Histograms**: Uses `hdrhistogram` for efficient and accurate latency tracking.

## Usage

Load testing is typically invoked via the `load` subcommand:

```bash
slapper load https://target.com --requests 10000 --concurrency 100
```

## Integration

Load testing can be combined with **Fuzzing** to see how a target behaves under stress, or used independently to benchmark web server performance.
