---
name: http_load_testing
description: "HTTP load and stress testing for performance and availability assessment"
triggers:
  - load test
  - loadtesting
  - stress
  - performance
  - benchmark
  - concurrency
  - rate limit
  - dos
  - http flood
metadata:
  category: loadtesting
  tools: [loadtest]
  scope: targets
---

## Overview

Load testing assesses how systems behave under stress. This includes performance benchmarks, concurrency testing, rate limiting detection, and DoS vulnerability assessment.

## Capabilities

- Request rate simulation
- Concurrency testing (concurrent users)
- Duration-based load tests
- Request profiling and timing
- Connection reuse optimization
- Keep-alive testing
- Rate limit detection
- Response time distribution
- Throughput measurement

## Usage

### Basic Load Test

```bash
slapper loadtest --target https://example.com --requests 10000 --concurrency 100
```

### Rate-Based Test

```bash
slapper loadtest --target https://example.com --rate 1000 --duration 60
```

### Timing Profile

```bash
slapper loadtest --target https://example.com --profile --requests 1000
```

### Rate Limit Detection

```bash
slapper loadtest --target https://api.example.com --rate-limit-test
```

## Metrics

| Metric | Description |
|--------|-------------|
| RPS | Requests per second |
| Latency | Time to first byte (TTFB) |
| Throughput | Bytes per second |
| Error Rate | Failed requests percentage |
| Percentiles | p50, p90, p99 response times |

## Triggers

Keywords: load, stress, performance, benchmark, concurrency, rate limit, dos, flood, requests, throughput, latency, ttfb, concurrent, throughput

## Best Practices

1. Always get baseline metrics before stress testing
2. Use rate limiting to avoid overwhelming target systems
3. Monitor for rate limit responses (429, 503)
4. Test with realistic payload sizes
5. Check for graceful degradation under load

## Configuration

```toml
[loadtest]
default_concurrency = 100
default_duration = 60
timeout = 30
keep_alive = true
```