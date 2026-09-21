# Performance Phase C — load-test request hot path

Status: Executed

Date: 2026-09-21

Baseline: 1fae91ec489a4fb553c1dfb26e184b5c61ea4adb

Parent roadmap:
performance-resource-efficiency-roadmap-2026-09-21.md

Depends on:
performance-phase-a-baseline-and-measurement-harness.md

## Purpose

Reduce CPU work and allocations inside the high-RPS load-test loop without
changing transport policy, timing semantics, result schema, or backend
capability.

The worker-owned Metrics architecture and CAS GlobalPacer are retained.

## Workstream C1 — Compile immutable request state once per run

Current RequestTemplate::scoped_request parses/builds an entire
ScopedHttpRequest for every request.

Keep the public RequestTemplate fields and constructor behavior compatible, but
move immutable conversion out of WorkerCtx::run.

Preferred shape:

1. At LoadTestExecutor::run start, construct one validated prototype
   ScopedHttpRequest from RequestTemplate + LoadTestPlan.
2. Fail the run before spawning workers if this compilation fails, preserving
   the same error category/message intent.
3. Share or clone the prototype into workers.
4. For each request, clone the already-parsed ScopedHttpRequest and pass the
   clone to HttpTransport::execute.

This removes per-request:

- method string parsing;
- URL parsing;
- temporary HashMap<String,String> construction;
- header string parsing;
- User-Agent HeaderName parsing;
- Vec<u8> body copy.

RequestBody already uses bytes::Bytes, so after the prototype is built a body
clone should be refcount-cheap.

Do not change ScopedHttpRequest public fields solely for this optimization.

## Workstream C2 — Preserve authorization timing

Precompiling the transport DTO must not pre-authorize the request.

Every dispatched clone must still pass through HttpTransport::execute with the
same NetworkAuthority and the same per-hop authorization/checkpoint sequence.

Do not cache:

- DNS authorization decisions;
- selected physical peers;
- redirect authorization;
- proxy authorization;
- TLS consistency decisions that are intentionally performed by the transport.

The optimization is request representation caching, not policy-result caching.

## Workstream C3 — Tighten Metrics hot counters

In crates/eggsec/src/loadtest/metrics.rs, remove avoidable double map lookups.

Examples:

- mutate status_codes.entry(code).or_insert(0) directly;
- mutate error-kind counters directly;
- avoid allocating kind.as_str().to_string() on every failure if an internal
  FxHashMap<LoadTestErrorKind,u64> can be converted to the existing public
  FxHashMap<String,u64> only in to_results.

The serialized/public LoadTestResults shape remains exactly unchanged.

Preserve saturating arithmetic and the first-1000 error sample cap.

## Workstream C4 — Benchmark-gated response drain without retention

The production Eggfetch adapter currently drains response bytes through EOF and
returns them in ScopedHttpResponse. LoadTestExecutor then uses only status.

Investigate whether the current eggfetch-core public API can consume a response
body to EOF without accumulating the full body in memory.

This workstream has a hard semantic gate:

- total request timeout must still extend through EOF;
- the body must still be fully consumed so connection reuse behavior remains
  valid;
- redirects must behave identically;
- status/error classification must remain identical;
- cancellation must still stop future hops/work;
- normal HttpTransport::execute callers must retain their current full-body
  response contract.

Preferred options, in order:

1. an existing upstream Eggfetch drain/stream primitive that can be used by a
   narrow load-test-specific internal path;
2. an additive defaulted transport method that preserves all downstream
   implementors and is justified by more than one consumer;
3. no implementation if the only option is to distort the shared transport
   API or duplicate the adapter.

Do not modify the shared public transport trait with a new required method.

If the benchmark shows negligible gain for realistic body sizes, record a
measured no-change decision.

## Workstream C5 — Keep existing good architecture

Do not replace:

- worker-private HDR histograms;
- final histogram merge;
- AtomicU64 issuance;
- CAS GlobalPacer;
- cancellation token behavior;
- production Eggfetch backend;
- manual authorized redirect loop;
- H1/H2 behavior.

Do not restore Reqwest as a direct production benchmark path.

## Tests

Add/extend tests proving:

1. invalid method/URL/header/template still fails before network I/O;
2. prototype clones are semantically equal to per-request construction;
3. request body/header mutations by one transport execution cannot leak into a
   subsequent request clone;
4. authority is invoked for every request/hop as before;
5. redirect/proxy/TLS parity tests remain green;
6. LoadTestResults serialization is unchanged;
7. metrics counts remain identical under success, HTTP failure, transport
   error, cancellation, and merge.

If drain-without-retain lands, add body-size/deadline fixtures proving EOF is
still consumed.

## Performance evidence

Using Phase A methodology, record:

- isolated request-materialization operations/sec or ns/op before/after;
- allocations if the available tooling can measure them reproducibly;
- H1 loopback RPS/latency at concurrency 1/10/50/100;
- H2 local fixture measurements separately;
- 0-byte/small-body/large-body cases if C4 is evaluated;
- accepted connection count after warm-up.

Use repeated trials and medians/ranges. Do not claim an Eggfetch version
regression/improvement from unmatched historical samples.

## Verification

Run at minimum:

- cargo fmt --all --check
- cargo test -p eggsec --test loadtest_tests
- cargo test -p eggsec-transport
- cargo test -p eggsec-transport-eggfetch
- cargo test -p eggsec-transport-eggfetch --test parity
- cargo test -p eggsec-transport-eggfetch --test h2_mux
- cargo test -p eggsec-transport-eggfetch --test socks5_local
- make check
- make check-python
- bash scripts/check-architecture-guards.sh

Run the release performance profile separately.

## Completion record

Status: Executed

- Starting SHA: roadmap baseline `1fae91ec489a4fb553c1dfb26e184b5c61ea4adb`
  (built on the Phase A/B HEAD); final SHA recorded at commit time.
- Files changed:
  - `crates/eggsec/src/loadtest/executor.rs` (C1: `run()` compiles one
    prototype `ScopedHttpRequest` before spawning workers and fails fast
    with the historical "invalid request: ..." message intent;
    `WorkerCtx` carries the prototype and dispatches cheap clones; +4
    tests: fail-fast-before-I/O, prototype/clone equality, mutation
    isolation, 256KB large-body EOF);
  - `crates/eggsec/src/loadtest/adapter.rs` (doc: prototype ownership);
  - `crates/eggsec/src/loadtest/metrics.rs` (C3: single-lookup `entry`
    counters; error kinds keyed by `LoadTestErrorKind` internally with the
    `String` map materialized once in `to_results`; serialized shape,
    saturating arithmetic, and the 1000-error cap unchanged);
  - `crates/eggsec/tests/perf_baseline.rs` (harness: clone-cost timing
    alongside rebuild timing);
  - `architecture/performance.md` (Phase C evidence table).
- Request-materialization evidence: rebuild ~890ns/op vs clone ~106ns/op
  (~8.4×); fake executor ~690k → ~1.1M RPS (~1.6×); H1 loopback
  ~18.9k/36.3k/25.8k → ~21.3k/40.3k/29.8k at c=1/10/50 (network-dominated,
  within ±10% run noise).
- C4 disposition: measured no-change (only narrow primitive is
  `bytes_stream()`, which still forces adapter duplication or a
  single-consumer trait method; body/deadline-through-EOF and the shared
  full-body contract preserved; 256KB EOF test guards the semantics).
- Transport authorization/deadline invariants: unchanged (denial,
  redirect/proxy/TLS parity suites green: transport 18, eggfetch parity
  52, H2 5, SOCKS5 4; `loadtest_tests` 29; loadtest lib 37).
- Verification: `cargo test -p eggsec --lib loadtest::` (37 passed);
  `--test loadtest_tests` (29 passed); transport + eggfetch suites green;
  `cargo clippy -p eggsec --no-default-features` clean;
  `cargo fmt --all --check` clean; release profiles re-run.
- Kept architecture: worker-private histograms, histogram merge, atomic
  issuance, CAS pacer, cancellation, Eggfetch backend, manual redirect
  loop, H1/H2 behavior (per C5).
