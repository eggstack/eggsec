# Performance Phase A — baseline and measurement harness

Status: Ready for implementation

Date: 2026-09-21

Baseline: 1fae91ec489a4fb553c1dfb26e184b5c61ea4adb

Parent roadmap:
performance-resource-efficiency-roadmap-2026-09-21.md

## Purpose

Create a durable, local-only performance measurement surface before modifying
runtime hot paths.

Eggsec already has useful retained load-test measurements, but the latest
high-volume comparison harness was ephemeral. The goal here is reproducibility,
not a microbenchmark framework for its own sake.

## Design constraints

1. Do not add a runtime dependency for benchmarking.
2. Prefer existing workspace dependencies and standard-library timing.
3. Profile fixtures must bind only loopback/local resources and must not
   perform Internet scanning.
4. Normal cargo test/make check must remain deterministic and must not execute
   long timing workloads.
5. Timing results are informational/manual. Correctness invariants may be CI
   gates; RPS/latency numbers may not.
6. Generated raw results are ignored artifacts. Only summarized evidence is
   retained in architecture/performance.md.
7. Release-mode runs are required for meaningful throughput comparisons.
8. Every benchmark must identify warm-up count, measured trial count, workload
   size, concurrency, and environment.

## Workstream A1 — Add performance evidence document

Create architecture/performance.md.

It must contain:

- baseline SHA;
- host OS/kernel/architecture;
- rustc and cargo versions;
- CPU model/core count when available;
- benchmark commands;
- warm-up and repetition policy;
- measured medians and ranges rather than single-run claims;
- known noise sources;
- a section per optimization phase;
- explicit statements when a comparison is inconclusive.

Link the existing load-test qualification evidence rather than duplicating it.

## Workstream A2 — Add a manual profiling harness

Add a small script such as scripts/perf-profile.sh plus focused ignored
integration/profile targets under the owning crates.

Preferred execution model:

- cargo test --release ... -- --ignored --nocapture for Rust-owned fixtures;
- a shell/Python orchestrator may repeat trials and capture wall/RSS when the
  host provides /usr/bin/time;
- profile tests emit stable machine-readable key=value or JSON-line summaries;
- no benchmark output is committed automatically.

Do not introduce Criterion/Divan merely to time asynchronous end-to-end
network operations. A dev-only benchmark dependency is acceptable only if a
specific CPU-only microbenchmark proves it materially improves repeatability.

## Workstream A3 — Load-test baseline

Retain/reproduce the current production Eggfetch path using loopback fixtures.

Measure at minimum:

- concurrency 1, 10, 50, 100;
- sufficient requests per trial to run for at least about one second on the
  host rather than relying on millisecond samples;
- one unreported warm-up and at least five measured trials;
- RPS;
- p50/p95/p99;
- wall duration;
- errors;
- accepted connection count after warm-up where the fixture exposes it.

Keep H1 and H2 fixtures separate. Do not infer H2 from an H1 benchmark.

Add a CPU-only request-materialization profile that repeatedly executes the
current RequestTemplate::scoped_request path so Phase C can distinguish local
DTO construction cost from network cost.

## Workstream A4 — Async fan-out baseline

Add deterministic local fixtures for:

- fuzzer: at least 1k, 10k, and one larger payload campaign if memory permits;
- port scan: representative small and full-range candidate counts against
  loopback listeners/closed ports;
- endpoint scan: at least 1k and 10k local paths;
- subdomain brute-force/verification: synthetic resolver fixture where
  practical, otherwise a deterministic fake/unit scheduler test.

Instrument peak live worker count explicitly with test-only atomics. Do not
infer peak tasks solely from requested concurrency.

Record:

- wall time;
- peak live work;
- total scheduled items;
- result count;
- ordering checksum or equivalent deterministic-output proof;
- peak RSS when portable tooling is available.

The most important baseline property is that current peak retained task/handle
state grows with total work even when active network work is semaphore-bounded.

## Workstream A5 — Distributed-worker baseline

Using loopback TLS fixtures already compatible with the distributed protocol,
measure a no-op/lightweight task workload.

Record:

- configured max_concurrency;
- observed maximum tasks_in_progress;
- tasks requested/assigned;
- completed/failed;
- coordinator TCP accepts;
- TLS handshakes if fixture instrumentation can expose them;
- auth exchanges;
- heartbeats;
- result submissions;
- total wall time.

Include a long-running task case to show whether current fixed polling can
reserve more work than max_concurrency.

Do not weaken TLS or PSK behavior for the benchmark.

## Workstream A6 — Correctness counters for future phases

Add reusable test-only counters/helpers only where they make later changes
provable:

- live/peak task counter for bounded schedulers;
- accepted-connection/auth counter for coordinator fixtures;
- load-test request-construction count if useful;
- deterministic result ordering/hash helpers.

Keep these helpers test-only and dependency-light.

## Acceptance criteria

Phase A is complete when:

1. architecture/performance.md exists with baseline methodology and numbers.
2. The profiling harness is committed and documented.
3. Normal make check does not run long timing trials.
4. Loopback-only load-test baseline is reproducible.
5. Async fan-out fixtures report peak live work.
6. Distributed fixtures report observed concurrency and connection/auth setup.
7. Baseline results are repeated, not single-sample claims.
8. Generated result files are ignored/not committed.
9. No production behavior or public API changes in this phase.

## Verification

Run at minimum:

- cargo fmt --all --check
- make check
- cargo test -p eggsec --lib
- cargo test -p eggsec --test loadtest_tests
- cargo test -p eggsec-runtime
- bash scripts/check-architecture-guards.sh

Run the new profile harness separately in release mode and record the exact
command/output summary in architecture/performance.md.

## Completion record

When implemented, append baseline/final SHAs, exact profile commands,
environment, measurement table, and any unavailable metric with reason.
