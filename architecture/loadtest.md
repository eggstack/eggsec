# Load Testing Module

## Overview

The load testing module provides HTTP performance benchmarking — measuring server throughput, latency percentiles, and error rates under controlled concurrency. Unlike the [stress module](stress.md) (which generates raw network floods for defense-lab DoS simulation), loadtest issues real HTTP requests and collects precise latency histograms via `hdrhistogram`.

**Phase D (2026-09-16):** the module was decoupled into a transport-neutral core (`plan` + `executor` + `metrics` + `progress`) with engine adaptation above it (`adapter`) and a scope-aware Reqwest backend (`backend`) behind the `eggsec-transport` seam. The core constructs no Reqwest client, owns no Clap/TUI/indicatif/config-file behavior, and never prints. `LoadTestRunner`/`LoadTestRunConfig` remain as compatibility facades. No `eggsec-loadtest` crate was created (Gate D1: rejected — single consumer, thin dependency payoff; see [capability_segregation.md](capability_segregation.md)).

**Corrective pass (2026-09-17):** authorization and physical-route gaps closed without a new crate. Execution scope is mandatory (`LoadTestRunner` stores `Option<Scope>`; ordinary `run()` fails closed without the `EnforcementContext` snapshot; no `default_facade_scope()` wildcard). Strict approval carries scope via engine-owned `ApprovedExecution` (`EnforcementContext::approve_execution()` / `approve_manual_execution()`); canonical execution uses `execute_approved_execution()` / `execute_canonical_with_scope()`; tool dispatch uses `ToolExecutionContext` + `execute_with_context()` via `EnforcedDispatcher::dispatch_execution()` (raw `LoadTestTool::execute()` fails closed). Direct and supported proxied load testing dispatches through the pinned Eggfetch backend (`eggsec-transport-eggfetch` over `eggfetch-core 0.1.5`, H1/H2 via ALPN, approved-IP pinning, pinned proxy peers/targets where enforceable). The Reqwest backend remains as a fail-closed transition backend (no semantic fallbacks, no direct-for-proxy fallback, credential-partitioned cache) but is no longer the production load-test path.

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

### `ReqwestTransport` (`backend.rs`, transition/fail-closed)

Scope-aware Reqwest backend implementing `HttpTransport` (verified + insecure base clients, proxied clients cached by endpoint + routing mode + TLS + credential fingerprint). All construction is fail-closed (`new()` / `with_system_resolver()` / `client_for()` return `Result`; no `Client::new()` default, no verified/insecure cross-fallback, no placeholder proxy, no direct-for-proxy fallback). Per-hop checkpoint order mirrors the recording fake, now including proxy-peer `authorize_proxy_resolved` / `authorize_proxy_socket` (authorization only — Reqwest cannot pin the proxy dial). Responses are drained for connection reuse. `reqwest::Error` maps to `TransportError::Backend` with stable classifiable prefixes (no secret material).

TOCTOU note: Reqwest re-resolves hostnames (and proxies) internally, so this backend cannot pin the connector to the exact approved address the way the Eggfetch adapter does. Production load-test traffic uses Eggfetch for physical pinning; this backend remains for transition/adversarial parity only.

### Backend route matrix (corrective pass + 2026-09-17 singular-binding follow-up)

| Route | Proxy peer pin | Ultimate pin | Disposition |
|---|---|---|---|
| direct HTTP/HTTPS | n/a | required (pinned wire URL, single selected IP) | supported via Eggfetch |
| HTTP/HTTPS proxy → HTTPS origin (CONNECT) | required, singular (`Proxy::resolved_addresses([proxy_peer])`) | required, singular (`proxy_target_addresses([ultimate_peer])`) | supported via Eggfetch 0.1.5 |
| SOCKS5 local-resolution → HTTP/HTTPS | required, singular | required, singular | supported via Eggfetch 0.1.5 |
| SOCKS5H remote-resolution | required | cannot be locally enforced | fail closed (explicit `Proxy` denial) |
| HTTP forward proxy → plaintext HTTP | required | standard proxy cannot enforce requested IP | fail closed (explicit `Proxy` denial) |

Single-address rule: one authorization cycle selects one physical address
per connection leg (`proxy_peer` = the address passed to
`authorize_proxy_socket`; `ultimate_peer` = the address passed to
`authorize_socket`). The backend receives single-element pin sets and has
no authorized alternate to fail over to — if the selected address fails,
the request fails. A retry may select another candidate only after a fresh
authorization cycle and selected-socket checkpoint (future failover must
live above the opaque backend retry layer, never as backend-internal
fallback). `ConnectionInfo.remote_addr` is the socket-authorized peer by
construction (direct: ultimate; proxied: proxy peer).

No automatic Reqwest fallback after an Eggfetch route error. Backend choice is composition, not error recovery. Redirects re-run logical authorization + physical binding per hop and build a fresh single-address route; cross-origin pinned-route reuse fails closed unless a new authorized snapshot is constructed.

### `ProgressSink` (`progress.rs`)

- `NoopSink` — library/Python/daemon default.
- `FnSink` — closure-backed (CLI indicatif renderer is driven from `mod.rs`, never from the core).
- `ChannelSink` — `mpsc`-backed structured consumer (TUI/daemon; `try_send` so slow consumers never block workers).
- `tui_mode` is **not** part of the core contract. The facade retains the flag for source compat but ignores it.

## Behavior & Flow

### Request execution flow

1. **Adaptation** (`adapter.rs`): CLI/`EggsecConfig`/auth input → `(LoadTestPlan, RequestTemplate)`; timeout falls back to `config.http.timeout_secs`; TLS/proxy/rate/user-agent merge with config defaults; auth applied canonically.
2. **Composition**: `LoadTestExecutor::new(plan, template, transport, authority, cancellation)` — CLI passes `ctx.scope` via `run_cli_with_scope`; `LoadTestRunner` requires an explicit scope (`with_scope()`/`set_scope()`, else `run()` fails before I/O); `run_with(transport, authority, ...)` stays explicit for tests/structured consumers. Strict paths carry the approval scope via `ApprovedExecution` (canonical) / `ToolExecutionContext` (tool dispatch), never a reloaded config or wildcard.
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

Loadtest dispatches through the scoped transport contract: every request passes the destination/scope checkpoints under the caller-supplied authority, redirects are transport-controlled and re-authorized per hop, and proxy/TLS intent travels as transport DTOs. CLI pre-dispatch policy (`evaluate_and_enforce_operation` in `handle_load`) is unchanged; per-request authorization uses the same scope snapshot (`ctx.scope`) carried via `ApprovedExecution` / explicit runner scope. `ApprovedOperation` target/tool binding remains mandatory and cannot be paired with a caller-selected wider scope through the normal strict API. Missing execution scope fails before DNS/network I/O. Client/transport initialization failure never broadens policy; a requested proxy never falls back to direct.

Execution-scope ownership:

- CLI/TUI manual: `EnforcementContext` scope snapshot → `approve_manual_execution()` → `execute_canonical_with_scope()` / `run_cli_with_scope()` / `TuiDispatcherContext.scope`.
- Strict REST/MCP/gRPC/agent/tool: `approve_execution()` → `EnforcedDispatcher::dispatch_execution()` / `execute_approved_execution()` with `ToolExecutionContext{scope}`.
- Daemon/runtime: `ApprovedRunRequest` bundle (token + scope) → `dispatch_approved_runtime_request()` → `execute_approved_execution()`.
- Python/library: Python `Scope` snapshot → `runner.set_scope()` → per-hop authority; `error_kinds` parity included.
- Tests/explicit: `run_with(transport, authority, ...)` with caller-owned authority (facade scope ignored).

Built-in safety measures:
1. **Constructor validation:** Rejects `concurrency == 0`, `total_requests == 0`, `timeout == 0`, userinfo URLs, non-http(s) schemes.
2. **Per-request timeout:** carried as `TimeoutPolicy` on every scoped request.
3. **Rate limit validation:** 0 is unlimited with a warning; > 100,000 logs an ineffectiveness warning.
4. **Error caps:** Error message list capped at 1000 entries; counters saturate (never wrap).
5. **No terminal output in core:** progress is structured events; only `mod.rs` (CLI) owns a progress widget.

## Probe Risk & Pipeline Integration

Load testing is tagged with `ProbeIntent::LoadBearing` and `ProbeRisk::Stress` (risk level 4) in the shared probe classification system (`crates/eggsec/src/probe.rs`, `architecture/probe.md`). Defense-lab profiles must explicitly include load-bearing probes and budget for `Stress`-level risk.

## Performance (Phase D WS3 guard + corrective-pass Eggfetch qualification)

Loopback fixture (`wiremock`, no-default-features, release-equal debug build):

```text
cargo test -p eggsec --no-default-features --test loadtest_tests
```

Phase D Reqwest baseline (temporary probe, removed before commit):

- IP literal, 200 req / 10 workers: ~24k RPS, p50 0ms, wall 0.02s
- IP literal, 1000 req / 50 workers: ~17k RPS, p50 2ms, p95 2ms, p99 17ms, wall 0.07s
- Hostname (`localhost`, DNS-checkpoint path), 500 req / 25 workers: ~19k RPS, p50 1ms, wall 0.03s

Corrective-pass Eggfetch direct (`eggfetch-core 0.1.5`, `Auto { allow_http3: false }`, approved-IP pinning, same fixtures):

```text
backend=eggfetch-0.1.5 reqs=200 conc=1 rps=11452 p50=0 p95=0 p99=0 wall=0.02s
backend=eggfetch-0.1.5 reqs=500 conc=10 rps=25894 p50=0 p95=0 p99=0 wall=0.02s
backend=eggfetch-0.1.5 reqs=1000 conc=50 rps=20943 p50=1 p95=2 p99=13 wall=0.05s
backend=eggfetch-0.1.5 reqs=2000 conc=100 rps=20659 p50=3 p95=5 p99=26 wall=0.10s
```

No performance win bypasses authorization or pinning. Eggfetch meets or exceeds the Reqwest baseline at representative concurrency (1/10/50/100) with identical Host/SNI, redirect, and reuse semantics; H1 retained, H2 negotiated via ALPN where offered (intentional protocol difference, explicitly accepted; HTTP/3 out of scope). Connection reuse never crosses incompatible physical-route identity (route/cache keys include pin state; proven by redirect/pin fixtures + upstream route-cache qualification).

The scoped seam adds no measurable overhead versus raw dispatch at these levels: authority checks are in-memory scope matching on the IP-literal fast path, clients are shared per run (no per-request construction), and drained bodies preserve keep-alive reuse. CPU/allocation profiling was not run (criterion: acceptable throughput + reuse evidence, both met).

## Public API

```rust
// Facade (scope mandatory; no wildcard default).
let runner = LoadTestRunner::new(url, 1000, 50, Duration::from_secs(30))?
    .with_scope(scope);
let results = runner.run().await?; // fails before I/O without scope

// New composition (explicit authority + structured progress):
let executor = LoadTestExecutor::new(plan, template, transport, authority, token);
let results = executor.run(&sink).await?;

// CLI with scope:
run_cli_with_scope(args, &config, scope).await?;

// Strict approved execution (token + scope snapshot):
let execution = ctx.approve_execution(surface, descriptor)?;
let outcome = execute_approved_execution(&execution, canonical, &sink).await?;

// Strict tool dispatch:
let execution = ctx.approve_execution(surface, descriptor)?;
let resp = dispatcher.dispatch_execution(&execution, request).await?;
```

## Integration Points

- **CLI:** `run_cli()` / `run_cli_with_scope()` (`mod.rs`) — parses `LoadArgs`, merges `EggsecConfig`, attaches scope, drives indicatif from channel events, writes output. `handle_load` passes `ctx.scope`.
- **TUI:** consumes `LoadTestResults` (`error_kinds` defaults empty for stored payloads); `TuiDispatcherContext.scope` carries the manual scope into `execute_canonical_with_scope()`; dispatch progress forwards structured events to the legacy channel.
- **Pipeline:** `Pipeline::with_scope()` / `set_scope()` carries the enforcement snapshot into the load-test stage; missing scope fails closed (no wildcard).
- **Tool/strict surfaces:** `LoadTestTool::execute_with_context()` via `dispatch_execution()` with `ApprovedExecution`; raw `execute()` fails closed. Other tools use the default `execute_with_context()` delegation (no mechanical rewrite).
- **Distributed:** standalone `process_load_test()` fails closed (no scope in `Task`); strict distributed dispatch goes through `dispatch_execution()` with a bundle once `Task` carries scope provenance.
- **Python:** Python `Scope` snapshot → `runner.set_scope()`; `LoadTestResultPy.error_kinds` parity (additive, `#[serde(default)]`).
- **Dispatch:** `run_load_test_with_scope()` forwards executor events to `progress_tx` via a non-blocking sink; unscoped `run_load_test()` / `execute_canonical()` load-test arm fail closed with explicit errors.
- **Report trait:** `LoadTestResults` implements `Report` for JSON export.

## Testing

```bash
cargo test --test loadtest_tests -p eggsec
cargo test --lib -p eggsec loadtest
cargo clippy --lib -p eggsec
```

Unit coverage: plan validation/rate/method, adapter auth/proxy/timeout shapes, metrics accounting/distribution/categorization/percentiles/cancellation/merge/zero-one/saturation/cap/legacy-serde, executor success/denial/cancellation/progress through the fake, progress sinks, Reqwest fail-closed construction/proxy-cache isolation, scope-propagation regressions (`loadtest_authorization_regression`: redirect/DNS denial, canonical/tool scope use, missing-scope fail-closed, explicit-authority path, proxy no-direct-fallback, invalid-proxy/userinfo denial).

## Invariants & Gotchas

1. **No warm-up phase:** requests start at full concurrency immediately.
2. **Latency = time-to-first-byte plus body drain:** measured from dispatch start to response completion (headers + drained body), recorded for successful and failed requests alike.
3. **Error classification:** HTTP 2xx/3xx = successful; 4xx/5xx = failed (`http_status` kind); transport denials/failures map to neutral kinds without backend types.
4. **FxHashMap for status codes/kinds:** performance over `std::collections::HashMap`.
5. **Histogram precision:** 3 significant figures; merged across workers with `add` (exact).
6. **No global mutex on the hot path:** per-worker accumulators + CAS pacer; the only mutex in the transition Reqwest backend is a short non-async lock for proxied-client cache clone-or-insert (never across I/O), keyed by full client identity (endpoint + mode + TLS + credential fingerprint).
7. **`tui_mode` is facade-only:** setting it changes nothing about execution; use a sink for progress.
8. **No wildcard default:** `LoadTestRunner` without `with_scope()`/`set_scope()` fails `run()` before I/O. There is no `default_facade_scope()`; production paths attach the `EnforcementContext` snapshot.
9. **No route fallback:** requested proxy never becomes direct; client/proxy build failures error; SOCKS5H/plaintext-forward ultimate pinning fails closed; backend choice is composition, not error recovery. The Eggfetch backend never falls back across DNS-approved addresses: one cycle pins one address per leg (singular `proxy_peer` / `ultimate_peer`); failure fails the request.

## See Also

- [overview.md](overview.md) — system-wide module index
- [probe.md](probe.md) — shared probe intent/risk vocabulary
- [defense_lab.md](defense_lab.md) — defense-lab profiles and risk budgets
- [stress.md](stress.md) — raw network flood testing
- [transport.md](transport.md) — scoped transport contract (incl. proxy-peer checkpoints)
- [transport_eggfetch.md](transport_eggfetch.md) — Eggfetch backend (pinned direct + qualified proxy routes; `eggfetch-core 0.1.5`)
- [capability_segregation.md](capability_segregation.md) — Gate D1/D2 rejection records
- [utils.md](utils.md) — engine utility ownership (Phase D `cache` removal)

*Last verified against source: 2026-09-17 (corrective pass: scope propagation, Reqwest fail-closed, Eggfetch direct + qualified proxy, MSRV 1.89)*
