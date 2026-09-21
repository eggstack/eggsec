# Performance evidence

Status: Executed (roadmap `plans/performance-resource-efficiency-roadmap-2026-09-21.md`;
A–F implementation `4fb021d`; closure-polish corrective pass
`plans/performance-closure-polish-corrective-pass-2026-09-21.md`, completion
record in that plan).

This document is the retained before/after evidence surface for the
performance and resource-efficiency campaign. Raw generated logs are ignored
artifacts; only summarized medians/ranges and structural counters are kept
here. Measured throughput/latency numbers below are the campaign evidence;
connection/handshake/auth counters in Phases E/F and the polish addendum are
structural correctness evidence, not universal network-latency benchmarks.

## Baseline reference

- Baseline SHA: `1fae91ec489a4fb553c1dfb26e184b5c61ea4adb`
- Campaign HEAD at creation: `d0b93683178c21b061bf366a1a8ad831a0810827`
- Host OS: Ubuntu 24.04.5 LTS, kernel 6.8.0-139-generic, x86_64
- CPU: Intel Core i9-9900K @ 3.60GHz, 16 logical threads
- Rust: `rustc 1.98.1`, `cargo 1.98.1` (MSRV for merge gates remains 1.89;
  performance runs use the local toolchain and are informational only)
- Harness: `scripts/perf-profile.sh` + ignored profile targets
  (`cargo test --release ... -- --ignored --nocapture`)

## Methodology

- All fixtures bind loopback/local resources only (`127.0.0.1`, Unix loopback
  wiremock, in-memory/fake transports, plaintext distributed loopback where
  noted). No Internet scanning.
- One unreported warm-up trial, then at least five measured trials per case.
  Tables report median and observed range, never a single sample.
- Release mode is required for throughput comparisons
  (`cargo test --release`); debug runs are correctness-only.
- Profile tests emit stable `key=value` / JSON-line summaries to stdout and
  never assert on timing. Correctness invariants (ordering checksums,
  result counts, peak-live-work bounds where applicable) may be hard
  assertions; RPS/latency numbers are informational and must not become
  merge gates.
- Normal `make check` does not execute long timing workloads: every profile
  target is `#[ignore]` and the shell harness is manual-only.
- Known noise sources: shared CI/dev host scheduling jitter, CPU frequency
  scaling, wiremock loopback listener setup, Tokio background threads, RSS
  sampling granularity (`/proc/self/statm` where available, otherwise
  `/usr/bin/time -v` maximum RSS when the host provides it).
- H1 and H2 fixtures are measured separately; H2 is never inferred from H1.
- Load-test latency includes response-body drain through EOF and the
  aggregate timeout extends through EOF. That definition did not change in
  this campaign.

## Workload matrix (Phase A harness)

| Area | Fixture | Sizes / concurrencies | Reported |
|------|---------|----------------------|----------|
| Load-test request materialization (CPU-only) | `RequestTemplate::scoped_request` repeat | 20k requests, single thread | ops/sec, ns/op |
| Load-test loopback (fake transport, no network) | `LoadTestExecutor` + `RecordingFakeTransport` | concurrency 1/10/50/100, 5k requests each | RPS, p50/p95/p99, wall, errors, hops |
| Load-test loopback (wiremock H1) | `LoadTestRunner` + loopback scope | concurrency 1/10/50, 200 requests each | RPS, latency, wall |
| Async fan-out (synthetic scheduler proof) | spawn-per-item + semaphore vs bounded JoinSet | 10k items, concurrency 50 | wall, peak live, checksum |
| Port scan fan-out | `scan_ports` vs loopback listeners/closed ports | small + full-range candidate counts | wall, retained-handle shape, checksum |
| Endpoint scan fan-out | `scan_endpoints` vs loopback server | 1k/10k paths | wall, retained-handle shape, checksum |
| Subdomain fan-out | synthetic resolver fixture + unit scheduler | candidate sets | wall, peak live, membership |
| Distributed worker | plaintext loopback coordinator + no-op tasks | max_concurrency N, short + long tasks | observed `tasks_in_progress`, requested/assigned, TCP accepts, auth exchanges, heartbeats, results, wall |
| ProxyPool | synthetic large proxy set | 5k entries | sort wall, allocation shape |
| Metrics | hot-counter repeat | 100k records + merge | ns/op |

Existing load-test qualification evidence is linked, not duplicated:
`architecture/loadtest.md` (repeated loopback evidence),
`architecture/transport_eggfetch.md` (adapter parity),
`crates/eggsec-transport-eggfetch/tests/parity.rs` (52 adapter tests),
`crates/eggsec-transport-eggfetch/tests/h2_mux.rs` (5 H2 local),
`crates/eggsec-transport-eggfetch/tests/socks5_local.rs` (4 SOCKS5-local).

## Phase A baseline (before any optimization)

Commands (release, ignored-only):

```bash
bash scripts/perf-profile.sh --suite all --trials 5 --warmup 1
# or per-suite:
# bash scripts/perf-profile.sh --suite loadtest --trials 5 --warmup 1
# bash scripts/perf-profile.sh --suite fanout --trials 5 --warmup 1
# bash scripts/perf-profile.sh --suite distributed --trials 5 --warmup 1
```

Direct target form:

```bash
cargo test --release -p eggsec --test perf_baseline -- --ignored --nocapture
cargo test --release -p eggsec-web-proxy --test perf_pool_baseline -- --ignored --nocapture
```

### A1. Request materialization (CPU-only, 20k iterations, release)

Measured 2026-09-21 (5 trials + 1 warm-up):

| Metric | Median | Range (5 trials) |
|--------|--------|------------------|
| `scoped_request` throughput | ~1.13M ops/sec | 1.11M–1.14M ops/sec |
| ns/op | ~882ns | 876–904ns |

Shape note: each call re-parses method + URL, builds a temporary
`HashMap<String,String>`, converts headers, parses `User-Agent`, and clones
the body `Vec<u8>`. This is the Phase C target.

### A2. Load-test executor through the fake (5k requests, release)

Measured 2026-09-21 (5 trials + 1 warm-up; sub-ms fake latencies read as 0ms):

| Concurrency | RPS (median) | p50 | p95 | p99 | Hops |
|-------------|--------------|-----|-----|-----|------|
| 1 | ~692k | 0ms | 0ms | 0ms | 5000 |
| 10 | ~698k | 0ms | 0ms | 0ms | 5000 |
| 50 | ~692k | 0ms | 0ms | 0ms | 5000 |
| 100 | ~681k | 0ms | 0ms | 0ms | 5000 |

Fake-transport runs isolate executor/DTO cost from network cost (flat across
concurrency because the fake performs no I/O; per-request DTO construction
dominates). Network loopback numbers are lower and noisier; see A3.

### A3. Wiremock H1 loopback (`LoadTestRunner` Reqwest-backend shape, 200 requests, release)

Measured 2026-09-21 (5 trials + 1 warm-up):

| Concurrency | RPS (median) | Observed behavior |
|-------------|--------------|-------------------|
| 1 | ~19k | p50/p95 sub-ms–1ms |
| 10 | ~36k | peak on this host/fixture |
| 50 | ~26–29k | wiremock-side contention lowers RPS past c=10 |

This exercises the Reqwest-backend runner shape, not the production Eggfetch
backend (covered by the retained parity evidence linked above). RPS varies
run to run (±10%); ranges not single samples carry the claim.

Accepted-connection counts after warm-up are fixture-dependent and are
reported by the harness where the fixture exposes them; wiremock reuses
connections opportunistically so this column is informational.

### A4. Async fan-out structural baseline

Current retained-handle shape grows with total work even when active network
work is semaphore-bounded:

- fuzzer `run_concurrent_inner`: one `tokio::spawn` + `Vec<JoinHandle>`
  entry per payload; semaphore acquired inside the task. 10k payloads retain
  10k handles until `join_all`.
- port scan `scan_ports`: permit acquired before spawn (active work bounded)
  but one handle retained per port until the end; shared `DashMap` + atomics.
- endpoint scan `scan_endpoints`: same retained-handle shape + shared
  `DashMap`/atomics.
- subdomain `verify_subdomains`/`bruteforce`: one task per candidate with an
  internal semaphore; large candidate sets retain O(candidates) suspended
  tasks.

Synthetic scheduler proof, measured 2026-09-21 (10k items, concurrency 50,
release, 2 trials + 1 warm-up):

| Scheduler | Wall (median) | Peak live / retained | Checksum |
|-----------|---------------|----------------------|----------|
| spawn-per-item + semaphore | ~5ms | ~10k retained handles (50 active) | `0x447e38cca94c4058` |
| bounded JoinSet (Phase B shape) | ~3–4ms | <= 50 | `0x447e38cca94c4058` (identical) |

Real-path loopback samples, measured 2026-09-21 (release):

| Workload | Wall (median) | Result |
|----------|---------------|--------|
| port scan small (12 candidates, c=100) | <1ms | open count is host-dependent (this host: 4); checksum stable per host |
| port scan sweep (200 closed loopback ports, c=100) | ~3ms | 0 open, checksum stable |
| endpoint scan (1002 paths, c=20, wiremock) | ~23ms | found=2, checksum stable |
| subdomain synthetic (2000 candidates, c=50) | ~1ms | peak live 50, found=1000, checksum stable |

The required Phase B success criterion is structural (peak in-flight state
O(concurrency)), not a wall-time threshold.

### A5. Distributed worker structural baseline

Using plaintext loopback fixtures (isolated lab only; TLS behavior
unchanged), measured 2026-09-21 (3 trials + 1 warm-up, 4 messages per trial
to stay under the production 60/min/IP loopback rate limit):

| Case | Configured `max_concurrency` | Observed peak `tasks_in_progress` | Task-request calls while saturated |
|------|------------------------------|-----------------------------------|------------------------------------|
| short tasks | 10 | can exceed 10 under burst (spawn-per-item, no permit) | fixed 5 tasks / 5s regardless of capacity |
| long tasks | 2 | can exceed 2; fixed polling reserves beyond budget | same fixed polling |

Control-plane setup, measured: 12 fresh client setups (4 heartbeats +
4 task-requests + 4 results) take ~496ms median (~41ms per TCP + PSK-auth +
request/response setup on loopback plaintext). Phase D makes capacity
truthful; Phase E reuses the session.

### A6. Secondary candidates (deferred to Phase F)

- Load-test `Metrics`: double map lookups (`entry` then `get`) and
  per-failure `kind.as_str().to_string()` allocation.
- `ProxyPool` sorting/stat lookup: repeated `to_log_key()` `String`
  creation, repeated `DashMap` stats lookups inside sort comparators, entry
  cloning while deriving healthy/sorted sets. Baseline measured 2026-09-21
  (5k synthetic entries, release, 5 trials + 1 warm-up): latency sort
  ~14.1ms median, success-rate sort ~8.6ms median, ordering checksum stable.
- `TaskQueue` state transitions: split `RwLock`s for pending/in_progress/
  completed; `dequeue` holds pending while awaiting in_progress and clones
  `Task` into the map.
- Global `eggsec-runtime` state mutex: theoretical cross-session contention
  point; explicitly deferred unless Phase A/F measurements demonstrate a
  real bottleneck. No restructuring in this campaign without that evidence.

## Phase B — bounded async fan-out and fuzzer lock scope

Measured 2026-09-21 (same host; warm-up 1 + 5 trials unless noted).
Checksums are stable across trials within a run; port-scan absolute values
vary run to run because the fixture binds ephemeral ports.

| Workload | Before (peak retained) | After (peak live) | Wall before → after | Checksum |
|----------|------------------------|-------------------|---------------------|----------|
| fuzzer 25 payloads loopback, concurrency 4 | O(25) handles | <= 4 (admission bound) | deterministic pass (debug) | index/order identical |
| port scan sweep200, concurrency 100 | O(200) handles | <= 100 (admission bound) | ~3ms → ~3–4ms | stable within run |
| endpoint 1002 paths, concurrency 20 | O(1002) handles | <= 20 (admission bound) | ~23ms → ~24ms | `0x28021f95e6d8be97` identical across phases |
| subdomain 2000 candidates, concurrency 50 | O(2000) tasks | <= 50 | ~1ms → ~1ms | stable within run |

Fuzzer `TimingAnalyzer` lock: guard now spans `timing.record()` only; a
slow response-body path no longer holds the analyzer mutex. Regression test
`test_timing_lock_released_before_body_read` fails deterministically against
the old shape (verified by temporary revert) and passes with the fix; it
uses loopback body-delay instrumentation, not wall-time thresholds.

`recon/dns_enhanced.rs`: no change. Its wordlist is hard-capped at 1024
entries, it already acquires the permit before spawn (active work bounded),
and Phase A shows no measurable retained-task cost at that cap. The
high-cardinality unbounded-candidate paths (`verify_subdomains`,
`bruteforce`) were converted.

## Phase C — load-test request hot path

Measured 2026-09-21 (same host; warm-up 1 + 5 trials):

| Metric | Before | After |
|--------|--------|-------|
| `scoped_request` rebuild ns/op (CPU-only, 20k iters) | ~882ns (~1.13M ops/sec) | ~890ns (prototype-build path, unchanged work) |
| per-request prototype clone ns/op (CPU-only, 20k iters) | n/a (rebuilt every request) | ~106ns (~9.4M ops/sec), ~8.4× cheaper |
| fake-transport executor RPS, 5k requests | ~692k/698k/692k/681k at c=1/10/50/100 | ~1207k/1204k/1029k/1141k (~1.6× on the DTO-dominated path) |
| H1 loopback RPS, 200 requests (Reqwest-backend runner shape) | ~18.9k/36.3k/25.8k at c=1/10/50 | ~21.3k/40.3k/29.8k (network-dominated; ±10% run noise, directionally consistent) |
| H2 local fixture | retained parity evidence (52 parity + 5 H2 + 4 SOCKS5 tests green) | unchanged behavior, suites green |
| Accepted connections after warm-up | fixture-reported | unchanged shape |

Prototype request compiled once per run; per-request work is a cheap DTO
clone. Authorization timing preserved: every clone still passes through
`HttpTransport::execute` with the same `NetworkAuthority` and per-hop
checkpoint sequence (denial test: 5 requests → 5 `policy_denied`, hops
counted per request). No DNS/peer/redirect/proxy/TLS decision is cached.

Workstream C4 (drain-without-retain): measured no-change. The only narrow
upstream primitive (`eggfetch-core` `bytes_stream()`) still requires
duplicating the adapter's manual authorized redirect loop (per-hop
authorization, aggregate deadline through EOF, redirect classification) or
distorting the shared `HttpTransport` trait with a single-consumer
defaulted method — both rejected by the workstream gate. Body-through-EOF
and deadline-through-EOF semantics are preserved, as is the shared
full-body response contract (guarded by a new 256KB large-body EOF test).
Drain time dominates latency either way; only transient peak bytes for
large bodies would differ, which does not justify a second code path
through the 52 parity + 5 interop tests.

## Phase D — distributed worker capacity

Implemented 2026-09-21 (same host). Primary success criterion is bounded
resource use and truthful capacity, not no-op-task RPS.

| Metric | Before | After |
|--------|--------|-------|
| Configured vs observed peak `tasks_in_progress` (short tasks) | can exceed N (spawn-per-item, no permit) | <= N (admission bound + CAS reservations; processor integration test: 12 tasks at cap 3, peak observed <= 3, final `failed=12/in_progress=0/reserved=0`) |
| Locally reserved (queued + executing) | unbounded (channel 100, fixed 5/5s polling) | <= `max_concurrency` (single `CapacityTracker`; 10 concurrent reserve-5 ticks on cap 10 admit exactly 10) |
| Task-request calls while saturated | fixed polling (5 tasks / 5s regardless) | skipped when `available == 0`; `min(5, available)` otherwise |
| Peak RSS (control-plane fixture) | ~6.1MB VmHWM | ~6.1MB VmHWM (unchanged shape) |
| Throughput on short tasks | fast-fail drain | 12 tasks drain terminally with exact accounting |
| Wire serialization | — | unchanged (`WorkerConfig` roundtrip test) |

`max_concurrency == 0` is a structured configuration error (`start()` fails
before registration), never silently coerced. Over-delivery is admitted
only up to room and logged as a protocol anomaly; excess assignments are
left for existing stale-task recovery, never executed beyond the limit and
never silently dropped. `TASK_PROCESSING_TIMEOUT` (300s), result submission
behavior, task dispatch, enforcement, and TLS/PSK behavior are unchanged.

## Phase E — coordinator session reuse

Implemented 2026-09-21 (same host). Fixed control workload before/after;
steady-state reuse is proven over both plaintext and verified local TLS
(the closure-polish pass replaced the earlier plaintext structural
inference with a real TLS fixture; see the polish addendum below):

| Counter | Before (one-shot client per message) | After (steady state, shared session) |
|---------|--------------------------------------|--------------------------------------|
| Coordinator TCP accepts | 6 for register+3 heartbeats+request+result (12 for the 12-op fixture, ~496ms) | 1 (plaintext `session_reuses_single_connection_in_steady_state` and TLS `session_reuses_single_tls_connection_in_steady_state`: 6 ops, 1 accept, 1 handshake where TLS, 1 auth, 1 live connection) |
| TLS handshakes | 1 per message | 1 per healthy connection lifetime (verified by the TLS listener's handshake counter, not inferred) |
| PSK auth exchanges | 1 per message | 1 per connection (+1 per reconnect) |
| Heartbeats / task-requests / results | N each | N each (same workload, fewer setups) |
| Wall (6-op steady workload) | ~248ms in setups alone (extrapolated from the 12-op fixture) | ~246ms total incl. RTTs (loopback-RTT-dominated; the count improvement 6→1 setups is the durable claim) |
| CPU/RSS | ~6.1MB VmHWM | unchanged shape |

Session architecture: one actor task owns the `LineWriter` for its whole
lifetime (`CoordinatorSession`, crate-internal; one-shot `RemoteClient`
methods unchanged for CLI/tool callers). Commands serialize through a
bounded (64) mpsc channel with `oneshot` replies; the queue bound is a
pinned constant plus burst-liveness coverage. Reconnect performs TCP +
TLS + PSK auth plus worker re-registration before connection-local
heartbeat state is relied upon (proven by the outage-recovery test, which
drives re-establishment + registration replay with a heartbeat alone and
no explicit re-register call, and by the deterministic sever fixture in
the polish addendum, which observes Register replay on the fresh
connection before the post-reconnect heartbeat).

Retry matrix: heartbeat retried once after reconnect (idempotent); task
acquisition never transparently retried (lost-response-after-dequeue risk
→ error surfaces, next poll recovers via stale tasks); result submission
never transparently retried (`complete()` appends → replay could duplicate
→ error surfaces, matching pre-session behavior); `Execute` untouched (no
broad retries). The polish pass adds explicit no-retry regression tests
(one wire `RequestTasks` / one wire `Result` per caller call on failure,
second explicit call recovers). Reconnect pacing: windowed rate limiting
(1s/2s/5s by consecutive failures) with fail-fast inside the window — no
actor sleeps, no tight loop, shutdown trivially responsive. No backoff
state on the wire.

Authentication/TLS unchanged: fewer handshakes, not weaker ones (no cert
verification change, no plaintext widening, no cross-connection auth
caching, same timeouts, same DNS-cache TTL semantics held by the actor's
single owned client; the TLS fixture uses verified test trust, never the
`insecure-tls` bypass).

### Closure-polish addendum (2026-09-21)

Short-lived `rcgen` localhost material, the production
`TlsServer::from_pem` accept path, and test-only verified trust
(`TlsClient::with_test_root` / `RemoteClient::with_test_root` /
`CoordinatorSession::spawn_with_test_root`; no public constructor, no
feature flag, PEM files under a test tempdir):

- TLS steady state (`session_reuses_single_tls_connection_in_steady_state`):
  register + 3 heartbeats + task request + result over one `CoordinatorSession`
  → 1 TCP accept / 1 TLS handshake / 1 PSK auth / 1 live connection.
- Established-session sever (`session_severed_tls_connection_reconnects_with_reregister`):
  scripted TLS coordinator accepts A, authenticates, handles Register + one
  live heartbeat, then deliberately drops A; the next heartbeat observes the
  loss, reconnects as B with fresh TCP/TLS/auth, replays Register before the
  heartbeat is handled as the worker's, and the idempotent retry succeeds.
  Observed: accepts 2, handshakes 2, auths 2, peer addrs differ, event order
  `conn0 register → conn0 heartbeat → conn0 severed → conn1 register →
  conn1 heartbeat`, zero unauthenticated commands.
- Retry dispositions (`session_request_tasks_never_retried_on_failure`,
  `session_result_never_retried_on_failure`): a dropped exchange surfaces
  exactly one wire message per caller call; the next explicit call recovers
  (second message, fresh setup). Heartbeat remains the only retried control
  op (proven by the sever test's successful retry).

The former "deterministic mid-session sever requires root/netns"
limitation is withdrawn: the sever/reconnect/re-auth/re-register sequence
is now covered by the loopback fixture above without privileged networking.

## Phase F — secondary hot spots and closure

Final re-measurement 2026-09-21 (same host; warm-up 1 + 5 trials; release):

| Workload | Baseline (Phase A) | Final (all phases) |
|----------|-------------------|---------------------|
| request rebuild ns/op | ~882ns | ~890ns (prototype-build path, once per run) |
| per-request clone ns/op | n/a | ~106ns (~8.4× cheaper) |
| fake executor RPS c=1/10/50/100 | ~692k/698k/692k/681k | ~1207k/1204k/1029k/1141k |
| H1 loopback RPS c=1/10/50 | ~18.9k/36.3k/25.8k | ~21.3k/40.3k/29.8k (±10% run noise) |
| port sweep200 wall | ~3ms | ~3–4ms, same open counts |
| endpoint 1002 paths wall | ~23ms | ~24–25ms, identical checksum |
| subdomain 2000 candidates | ~1ms, unbounded tasks | ~1ms, peak live 50 |
| distributed 12 fresh setups | ~496ms | ~495ms (local scheduling only; session reuse measured separately: 6 ops → 1 accept/1 auth, 1 handshake where TLS) |
| pool 5k sorts latency/rate | ~14.1ms / ~8.6ms | ~3.1ms / ~3.3ms (snapshot sort) |

| Candidate | Disposition | Evidence |
|-----------|-------------|----------|
| Metrics micro-cleanup | implemented (Phase C) | counts identical under success/failure/cancel/merge; serialized shape byte-compatible (legacy test green) |
| ProxyPool snapshot sort | implemented (this phase) | latency 14.1ms → 3.1ms, rate 8.6ms → 3.3ms; health/priority/latency/rate/round-robin semantics pinned by 26 pool tests incl. tie-ordering |
| TaskQueue lock layout | implemented as option B (this phase) | split locks acquired in opposite orders on different paths (dequeue pending→in_progress vs stale-recovery in_progress→pending); unified state removes the inversion; the owned-return/in-progress representation still requires one task clone at dequeue; 23 distributed tests incl. 200-task hammer + stale-cycle preservation |
| Global runtime mutex | rejected without contention evidence | no multi-session contention measured; lifecycle-ordering risk exceeds any hypothetical gain |
| Pipeline dependency waves | rejected (bounded cardinality) | wave size bounded by the dependency graph; not a scaling path |
| Global allocator (mimalloc/jemalloc) | rejected without allocation-profile + packaging review | no profile evidence; dependency-surface cost |
| PGO | future release engineering only | representative workloads now stable; not mandatory for normal builds |
| `target-cpu=native` | rejected for portable artifacts | release portability requirement stands |

## Limitations

- Numbers are host- and toolchain-specific; cross-host numeric claims are
  not made. Structural counters (peak live work, connection/auth counts,
  retained-handle shape) carry across hosts; RPS/latency do not.
- Short or current-only samples do not establish version-to-version
  regression parity (per `AGENTS.md` eggfetch qualification rule).
- Performance claims stay limited to the measurements recorded here; the
  harness remains informational and never a merge gate.
