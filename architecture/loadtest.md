# Load Testing Module

## Overview

The load testing module provides HTTP performance benchmarking — measuring server throughput, latency percentiles, and error rates under controlled concurrency. Unlike the [stress module](stress.md) (which generates raw network floods for defense-lab DoS simulation), loadtest issues real HTTP requests and collects precise latency histograms via `hdrhistogram`.

**Phase D (2026-09-16):** the module was decoupled into a transport-neutral core (`plan` + `executor` + `metrics` + `progress`) with engine adaptation above it (`adapter`) and a scope-aware Reqwest backend (`backend`) behind the `eggsec-transport` seam. The core constructs no Reqwest client, owns no Clap/TUI/indicatif/config-file behavior, and never prints. `LoadTestRunner`/`LoadTestRunConfig` remain as compatibility facades. No `eggsec-loadtest` crate was created (Gate D1: rejected — single consumer, thin dependency payoff; see [capability_segregation.md](capability_segregation.md)).

**Feature gate:** None on the module itself (always compiled). The CLI entry points `run_cli()`/`run_cli_with_scope()` are gated behind the `cli` feature (`mod.rs`).

**Role:** HTTP performance testing — RPS, latency percentiles (p50/p90/p95/p99), status code distribution, transport-error categorization.

## Module Structure

| File | Purpose |
|------|---------|
| `mod.rs` | Module entry, `run_cli()` / `run_cli_with_scope()` CLI entry points (own the only `indicatif` progress widget) |
| `plan.rs` | `LoadTestPlan`, `RatePolicy` — transport-neutral plan (no Clap/config/Reqwest/indicatif) |
| `executor.rs` | `LoadTestExecutor<T: HttpTransport>` — generic executor, per-worker sharded metrics, CAS global pacer |
| `metrics.rs` | `Metrics` (pure single-threaded accumulator + `merge`) + `LoadTestResults` + `LoadTestErrorKind` |
| `progress.rs` | `LoadTestEvent`, `LoadTestProgress`, `ProgressSink` (`NoopSink`, `FnSink`, `ChannelSink`), `SharedProgress` |
| `adapter.rs` | `RequestTemplate` + `plan_from_adapter()` — engine translation above the core (no `CommonHttpArgs` in the plan) |
| `backend.rs` | `ReqwestTransport` (scope-aware Reqwest `HttpTransport`) + `OwnedScopeAuthority` (`'static` scope handle) |
| `runner.rs` | `LoadTestRunner` / `LoadTestRunConfig` compatibility facades (retain `tui_mode` for source compat; ignored) |

## Key Types

### `LoadTestPlan` (`plan.rs`)

Transport-neutral reusable primitive:

```rust
pub struct LoadTestPlan {
    pub url: String,              // validated: http/https, host present, no userinfo
    pub total_requests: u64,
    pub concurrency: usize,
    pub timeout: Duration,        // per-request, never zero
    pub method: String,           // one of the 8 parity verbs (normalized)
    pub body: Option<Vec<u8>>,    // replayable bytes
    pub headers: Vec<(String, String)>, // auth already applied by the adapter
    pub rate: RatePolicy,         // Unlimited | PerSecond(n)
}
```

Validation rejects zero concurrency/requests/timeout, userinfo URLs, and non-http(s) schemes. Unknown methods normalize to `GET` with a warning (at the adapter layer).

### `LoadTestExecutor<T: HttpTransport>` (`executor.rs`)

Generic executor composed of plan + template + transport + authority + cancellation token:

- Workers pull indices via an atomic counter; count is `min(concurrency, total_requests)`.
- Each worker owns a private `Metrics`; the run merges them at the end — hot-path recording never touches a shared async mutex.
- Global pacing uses a CAS slot allocator (`GlobalPacer`, no mutex across sleeps); aggregate throughput matches the configured rate at any worker count. Cancellation preempts waits.
- Dispatch races the transport future against the cancellation token (dropping the future performs no further hops per the transport contract).
- Transport errors map to `LoadTestErrorKind` without exposing backend types.

### `LoadTestResults` (`metrics.rs`)

Serializable output; `error_kinds` (added in Phase D, `#[serde(default)]` so pre-Phase-D payloads still deserialize) counts per-category transport failures:

| Field | Type | Description |
|-------|------|-------------|
| `target_url` | `String` | Target URL |
| `total_requests` | `u64` | Total issued (saturating) |
| `successful_requests` | `u64` | HTTP 2xx/3xx |
| `failed_requests` | `u64` | HTTP 4xx/5xx, transport errors, cancellations |
| `total_duration_ms` | `u64` | Wall-clock duration |
| `requests_per_second` | `f64` | Total / duration_secs |
| `latency_min_ms` | `f64` | Histogram minimum |
| `latency_max_ms` | `f64` | Histogram maximum |
| `latency_mean_ms` | `f64` | Histogram mean |
| `latency_p50_ms` | `f64` | 50th percentile |
| `latency_p90_ms` | `f64` | 90th percentile |
| `latency_p95_ms` | `f64` | 95th percentile |
| `latency_p99_ms` | `f64` | 99th percentile |
| `status_codes` | `FxHashMap<u16, u64>` | Status code distribution |
| `errors` | `Vec<String>` | Error messages (capped at 1000) |
| `error_kinds` | `FxHashMap<String, u64>` | `http_status` / `policy_denied` / `dns` / `timeout` / `connect` / `invalid_request` / `backend` / `cancelled` |

Implements `Display` with sorted status codes, sorted error kinds, and first-5 error display. Implements `Report` trait (`runner.rs`) with `title() -> "Load Test Report"` and `to_json()`.

### `RequestTemplate` (`adapter.rs`)

Transport-neutral request template derived from CLI/config/auth input. `scoped_request(&plan)` builds the per-request `ScopedHttpRequest` DTO (method/URL/headers/body + timeout/redirect/proxy/TLS/hints). Auth-flag shapes (`user:pass`, bearer, cookie merge, `Name:value` API keys) are parsed at this boundary and applied through the canonical `eggsec_transport::merge_cookie_header` semantics — never reimplemented in the executor.

### `ReqwestTransport` (`backend.rs`)

Scope-aware Reqwest backend implementing `HttpTransport`. Owns the only `reqwest::Client` contact point for load testing (verified + insecure base clients, cached proxied clients per endpoint). Per-hop checkpoint order mirrors the recording fake (initial-URL → host → DNS/re-resolution → socket → TLS-consistency → proxy → dispatch; redirects re-authorized per hop with the redirect-policy gate). Responses are drained for connection reuse. `reqwest::Error` maps to `TransportError::Backend` with stable classifiable prefixes (no secret material).

TOCTOU note: Reqwest re-resolves hostnames internally, so this backend cannot pin the connector to the exact approved address the way the Eggfetch adapter does. It closes the gap as far as the Reqwest API allows (fresh resolve + full candidate authorization + binding validation + socket re-verification on every hop). Full IP pinning arrives with the Eggfetch migration (which currently defers proxied execution); proxied load tests stay on this backend meanwhile.

### `ProgressSink` (`progress.rs`)

- `NoopSink` — library/Python/daemon default.
- `FnSink` — closure-backed (CLI indicatif renderer is driven from `mod.rs`, never from the core).
- `ChannelSink` — `mpsc`-backed structured consumer (TUI/daemon; `try_send` so slow consumers never block workers).
- `tui_mode` is **not** part of the core contract. The facade retains the flag for source compat but ignores it.

## Behavior & Flow

### Request execution flow

1. **Adaptation** (`adapter.rs`): CLI/`EggsecConfig`/auth input → `(LoadTestPlan, RequestTemplate)`; timeout falls back to `config.http.timeout_secs`; TLS/proxy/rate/user-agent merge with config defaults; auth applied canonically.
2. **Composition**: `LoadTestExecutor::new(plan, template, transport, authority, cancellation)` — CLI passes `ctx.scope` via `run_cli_with_scope`; the facade default scope is permissive `["*"]` (pre-Phase-D behavior preserved) while every request still traverses the authority seam.
3. **Worker loop** (`executor.rs`): atomic index → global pace slot → build scoped request → race transport dispatch vs cancellation → record locally → emit structured event.
4. **Merge**: per-worker `Metrics` merged (histogram `add` + saturating counters); error list capped at 1000 across workers.
5. **Presentation** (process-host only): CLI drives its indicatif bar from channel events in `mod.rs`; `Display`/`Report` render results.

### Rate pacing algorithm

CAS slot allocator (`GlobalPacer` in `executor.rs`): workers claim issue slots via `compare_exchange` on an atomic nanos counter. Aggregate throughput equals the configured rate at any worker count; no mutex is held across sleeps; waits are cancellation-preemptible. Zero rate means unlimited (logged at the adapter layer).

### Response body handling

All response bodies are drained (`response.bytes().await`) before recording, preserving connection reuse. Body-read failures map to transport errors (classified, never panicking).

### Warm-up

No warm-up phase is implemented. Requests are issued at full concurrency from the first iteration (paced runs ramp via the slot allocator from t=0).

## Safety & Authorization

Loadtest dispatches through the scoped transport contract: every request passes the destination/scope checkpoints under the caller-supplied authority, redirects are transport-controlled and re-authorized per hop, and proxy/TLS intent travels as transport DTOs. CLI pre-dispatch policy (`evaluate_and_enforce_operation` in `handle_load`) is unchanged; per-request authorization uses the same scope (`ctx.scope`).

Built-in safety measures:
1. **Constructor validation:** Rejects `concurrency == 0`, `total_requests == 0`, `timeout == 0`, userinfo URLs, non-http(s) schemes.
2. **Per-request timeout:** carried as `TimeoutPolicy` on every scoped request.
3. **Rate limit validation:** 0 is unlimited with a warning; > 100,000 logs an ineffectiveness warning.
4. **Error caps:** Error message list capped at 1000 entries; counters saturate (never wrap).
5. **No terminal output in core:** progress is structured events; only `mod.rs` (CLI) owns a progress widget.

## Probe Risk & Pipeline Integration

Load testing is tagged with `ProbeIntent::LoadBearing` and `ProbeRisk::Stress` (risk level 4) in the shared probe classification system (`crates/eggsec/src/probe.rs`, `architecture/probe.md`). Defense-lab profiles must explicitly include load-bearing probes and budget for `Stress`-level risk.

## Performance (Phase D WS3 guard)

Loopback fixture (`wiremock`, no-default-features, release-equal debug build):

```text
cargo test -p eggsec --no-default-features --test loadtest_tests
```

Measured with a temporary probe (removed before commit; exact commands in the Phase D completion record):

- IP literal, 200 req / 10 workers: ~24k RPS, p50 0ms, wall 0.02s
- IP literal, 1000 req / 50 workers: ~17k RPS, p50 2ms, p95 2ms, p99 17ms, wall 0.07s
- Hostname (`localhost`, DNS-checkpoint path), 500 req / 25 workers: ~19k RPS, p50 1ms, wall 0.03s

The scoped seam adds no measurable overhead versus raw dispatch at these levels: authority checks are in-memory scope matching on the IP-literal fast path, clients are shared per run (no per-request construction), and drained bodies preserve keep-alive reuse. CPU/allocation profiling was not run (criterion: acceptable throughput + reuse evidence, both met).

## Public API

```rust
// Facade (compat): unchanged construction sites keep compiling.
let runner = LoadTestRunner::new(url, 1000, 50, Duration::from_secs(30))?;
let results = runner.run().await?;

// New composition (explicit authority + structured progress):
let executor = LoadTestExecutor::new(plan, template, transport, authority, token);
let results = executor.run(&sink).await?;

// CLI with scope:
run_cli_with_scope(args, &config, scope).await?;
```

## Integration Points

- **CLI:** `run_cli()` / `run_cli_with_scope()` (`mod.rs`) — parses `LoadArgs`, merges `EggsecConfig`, attaches scope, drives indicatif from channel events, writes output.
- **Handler:** `handle_load` passes `ctx.scope` so per-request authorization matches the pre-dispatch verdict.
- **TUI:** consumes `LoadTestResults` (new `error_kinds` defaults empty for stored payloads); dispatch progress forwards structured events to the legacy channel.
- **Pipeline/tool/distributed/Python:** unchanged facade paths (`from_config_with_engine` + `run`); Python pre-checks scope via `scope.enforce_target` as before.
- **Dispatch:** `run_load_test` forwards executor events to `progress_tx` via a non-blocking sink.
- **Report trait:** `LoadTestResults` implements `Report` for JSON export.

## Testing

```bash
cargo test --test loadtest_tests -p eggsec
cargo test --lib -p eggsec loadtest
cargo clippy --lib -p eggsec
```

Unit coverage: plan validation/rate/method, adapter auth/proxy/timeout shapes, metrics accounting/distribution/categorization/percentiles/cancellation/merge/zero-one/saturation/cap/legacy-serde, executor success/denial/cancellation/progress through the fake, progress sinks.

## Invariants & Gotchas

1. **No warm-up phase:** requests start at full concurrency immediately.
2. **Latency = time-to-first-byte plus body drain:** measured from dispatch start to response completion (headers + drained body), recorded for successful and failed requests alike.
3. **Error classification:** HTTP 2xx/3xx = successful; 4xx/5xx = failed (`http_status` kind); transport denials/failures map to neutral kinds without backend types.
4. **FxHashMap for status codes/kinds:** performance over `std::collections::HashMap`.
5. **Histogram precision:** 3 significant figures; merged across workers with `add` (exact).
6. **No global mutex on the hot path:** per-worker accumulators + CAS pacer; the only mutex in the backend is a short non-async lock for proxied-client cache clone-or-insert (never across I/O).
7. **`tui_mode` is facade-only:** setting it changes nothing about execution; use a sink for progress.
8. **Facade default scope is permissive:** `LoadTestRunner` without `with_scope`/`set_scope` authorizes against `["*"]` (pre-Phase-D behavior). Production CLI supplies the real scope.

## See Also

- [overview.md](overview.md) — system-wide module index
- [probe.md](probe.md) — shared probe intent/risk vocabulary
- [defense_lab.md](defense_lab.md) — defense-lab profiles and risk budgets
- [stress.md](stress.md) — raw network flood testing
- [transport.md](transport.md) — scoped transport contract
- [transport_eggfetch.md](transport_eggfetch.md) — Eggfetch backend (full IP pinning; proxy deferred)
- [capability_segregation.md](capability_segregation.md) — Gate D1/D2 rejection records
- [utils.md](utils.md) — engine utility ownership (Phase D `cache` removal)

*Last verified against source: 2026-09-16 (Phase D decoupling)*
