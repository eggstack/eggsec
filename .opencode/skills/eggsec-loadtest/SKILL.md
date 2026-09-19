---
name: eggsec-loadtest
description: "HTTP load testing and performance benchmarking - use when working with LoadTestRunner, LoadTestPlan, LoadTestExecutor, concurrent workers, latency percentiles, rate limiting, or hdrhistogram metrics."
---

# Eggsec Loadtest Skill

HTTP load testing module workflows and patterns (Phase D core + 2026-09-17 corrective pass + 2026-09-19 0.1.7 adoption: mandatory execution scope, logical-URL + resolved-address Eggfetch backend).

Execution scope is mandatory: `LoadTestRunner` stores `Option<Scope>` and ordinary `run()` fails closed without the `EnforcementContext` snapshot (no wildcard default). New entry points require execution context — never synthesize `allowed_targets = ["*"]`. Strict paths carry scope via `ApprovedExecution` (`approve_execution()` → `execute_approved_execution()` / `execute_canonical_with_scope()`) and `ToolExecutionContext` (`execute_with_context()` via `dispatch_execution()`); raw `LoadTestTool::execute()` fails closed. Direct + supported proxied traffic uses the pinned Eggfetch backend (`eggfetch-core 0.1.7`, logical-URL + singular resolved-address direct, H1/H2 route reuse, total deadline through body EOF, proxy peers/targets); Reqwest is fail-closed transition only (no fallback, credential-partitioned cache).

## Key Types and Patterns

### LoadTestPlan (`loadtest/plan.rs`)
Transport-neutral reusable primitive: target, request template, budgets, explicit `RatePolicy`. No Clap args, `EggsecConfig`, Reqwest, or indicatif.

### LoadTestExecutor<T: HttpTransport> (`loadtest/executor.rs`)
Generic executor over the scoped transport seam. Each worker owns a private `Metrics` (merged at end — no shared mutex); global pacing via CAS slot allocator (`GlobalPacer`); cancellation races transport dispatch.

### ReqwestTransport (`loadtest/backend.rs`, transition/fail-closed)
Scope-aware Reqwest `HttpTransport` (verified + insecure clients, proxied cache keyed by endpoint + mode + TLS + credential fingerprint). Construction fails closed (`Result`; no `Client::new()` default, no verified/insecure fallback, no placeholder proxy, no direct-for-proxy fallback). Proxy peers authorized via `authorize_proxy_resolved`/`authorize_proxy_socket` (no pinning under Reqwest). Production load-test traffic uses Eggfetch, not this backend.

### Eggfetch production backend (`eggsec-transport-eggfetch`, `eggfetch-core 0.1.7`)
Logical-URL + singular `resolved_addresses([selected_target])` direct (no IP-literal shim; no origin DNS; Hyper H1/H2 route reuse) + qualified proxy routes (singular `proxy_peer` / `ultimate_peer` → single-element `Proxy::resolved_addresses` + `proxy_target_addresses`; CONNECT + SOCKS5 local supported, SOCKS5H/plaintext fail closed). H1/H2 via ALPN (`Auto { allow_http3: false }`, H3 off, env proxy never used, downgrade stays Allow). Aggregate `Timeout.total` spans body EOF per hop (remaining-budget mapping). One authorization cycle pins one address per leg — backend has no authorized alternate to fail over to; failure fails the request (fresh cycle required to try another candidate). `ConnectionInfo.remote_addr` is the socket-authorized peer by construction. Evidence split: Eggsec-local H1/H2/SOCKS5-local fixtures (`parity` + `h2_mux` + `socks5_local`) prove correctness through `EggfetchTransport`; upstream 0.1.7 Tier 1/extended/HTTPX exact-SHA gates qualify the published crate; `architecture/loadtest.md` records noisy 1/10/50/100 measurements (never a CI threshold).

### LoadTestRunner / LoadTestRunConfig (`loadtest/runner.rs`)
Compatibility facades. `tui_mode` is retained for source compat but **ignored** — progress is structured events (`progress.rs`: `NoopSink`, `FnSink`, `ChannelSink`). Scope is `Option<Scope>`: attach via `with_scope()`/`set_scope()` or `run()` fails before I/O; `run_with(transport, authority, ...)` stays explicit (facade scope ignored).

### RequestTemplate (`loadtest/adapter.rs`)
Engine adaptation above the core: CLI/config/auth → plan + transport-neutral template. Auth applied through canonical transport helpers, never reimplemented in the executor.

### Metrics (`loadtest/metrics.rs`)
Pure single-threaded accumulator (`merge` for sharded workers) + `LoadTestResults` + `LoadTestErrorKind` categorization (`error_kinds`).

### Worker Model
- `worker_count = min(concurrency, total_requests)`
- Each worker loops, fetching `request_index = issued_requests.fetch_add(1, Ordering::Relaxed)`
- Global pacing via CAS allocator (aggregate rate holds at any worker count; no mutex on hot path)

## Testing

### Running Loadtest Tests
```bash
cargo test --test loadtest_tests -p eggsec
cargo test --lib -p eggsec loadtest
```

### Writing Tests
Follow existing patterns in `tests/loadtest_tests.rs`:
- Use `create_test_server()` + `mock_ok()` from test helpers
- Test basic, concurrency, error handling, validation
- Unit tests use `RecordingFakeTransport` (no network): see `executor.rs` tests

## Common Tasks

### Adding a New Load Test Configuration Option
1. Add field to `LoadArgs` in `cli/http.rs`
2. Thread through `AdapterInput` in `loadtest/adapter.rs` (never store `CommonHttpArgs` in `LoadTestPlan`)
3. Apply in `RequestTemplate` / `plan_from_adapter`
4. Add adapter unit tests + integration tests for new option

### Adding a New Metric
1. Add field to `LoadTestResults` in `metrics.rs` (use `#[serde(default)]` for back-compat)
2. Track in `Metrics` struct (saturating counters)
3. Populate in `to_results()`, merge in `merge()`
4. Display in `Display` impl and serialize in `Serialize` impl

## CLI Usage

```bash
# Basic load test
eggsec load https://example.com -n 1000 -c 50

# With body and headers
eggsec load https://example.com/api -n 500 -c 20 -m POST -d '{"key":"value"}' -H 'Content-Type:application/json'

# With rate limiting
eggsec load https://example.com -n 10000 -c 100 --rate-limit 50

# JSON output
eggsec load https://example.com -n 1000 -c 50 --json

# Output to file
eggsec load https://example.com -n 1000 -c 50 -o results.json
```

## Metrics Collected

- `total_requests` - Total requests sent
- `successful_requests` - 2xx-3xx responses
- `failed_requests` - 4xx-5xx + transport errors + cancellations
- `requests_per_second` - Throughput
- `latency_min_ms`, `latency_mean_ms`, `latency_max_ms` - Latency stats
- `latency_p50_ms`, `latency_p90_ms`, `latency_p95_ms`, `latency_p99_ms` - Percentiles
- `status_codes` - Map of HTTP status code to count
- `error_kinds` - Transport-error category counts (policy_denied/dns/timeout/connect/...)
- `errors` - Error messages (first 5 displayed, capped at 1000 stored)

## Code Conventions

- Use `rustc_hash::FxHashMap` for `status_codes`/`error_kinds` maps (not `std::collections::HashMap`)
- Histogram uses `hdrhistogram::Histogram<u64>` with 3 significant figures
- Histogram record/merge failures log with `tracing::warn!` — never `let _ =` or silent suppression
- Auth headers handled via canonical transport helpers (`merge_cookie_header` semantics)
- All response bodies are drained before recording (connection reuse)
- Rate pacing uses the CAS `GlobalPacer` (no shared mutex on the hot path)
- Core never touches `indicatif`, `reqwest`, Clap, or `EggsecConfig` — presentation lives in `run_cli`, HTTP in `backend.rs`
- No `eggsec-loadtest` crate (Gate D1 rejected: single consumer); no `eggsec-resilience` crate (Gate D2 rejected: no second consumer)

## Resources
- `crates/eggsec/src/loadtest/` - Module source
- `architecture/loadtest.md` - Architecture documentation
- `architecture/capability_segregation.md` - Gate D1/D2 rejection records
- `AGENTS.md` - General project guidelines
