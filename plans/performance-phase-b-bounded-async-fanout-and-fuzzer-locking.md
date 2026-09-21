# Performance Phase B — bounded async fan-out and fuzzer locking

Status: Ready for implementation

Date: 2026-09-21

Baseline: 1fae91ec489a4fb553c1dfb26e184b5c61ea4adb

Parent roadmap:
performance-resource-efficiency-roadmap-2026-09-21.md

Depends on:
performance-phase-a-baseline-and-measurement-harness.md

## Purpose

Remove avoidable serialization in the fuzzer and make high-cardinality async
fan-out scale with configured concurrency rather than total work count.

This phase changes scheduling internals only. Result DTOs, callback behavior,
timeouts, ordering, and public concurrency options remain stable.

## Workstream B1 — Narrow TimingAnalyzer lock scope

In crates/eggsec/src/fuzzer/engine/utils.rs, change send_payload_async so the
TimingAnalyzer mutex guard exists only for the mutation that records the
request timing.

Required semantic shape:

- await request.send();
- measure response_time;
- enter a small lexical block;
- lock TimingAnalyzer;
- call timing.record(response_time);
- copy/move the TimingResult out;
- drop the guard before any response-body await, pattern scan, leak formatting,
  severity calculation, or FuzzResult construction.

Do not change TimingAnalyzer classification semantics in this phase.

Add a concurrency regression test that can prove a slow response-body path does
not hold the analyzer mutex. Prefer instrumentation/fake response behavior over
sleep-only timing assertions when practical.

## Workstream B2 — Bound fuzzer tasks by concurrency

In crates/eggsec/src/fuzzer/engine/execution.rs, replace the current
one-spawn-per-payload plus semaphore plus Vec<JoinHandle> design with a bounded
scheduler.

Preferred implementation:

- JoinSet or FuturesUnordered;
- at most args.concurrency payload futures/tasks in flight;
- admit a new item as one completes;
- carry the original payload index through completion;
- keep a Vec<Option<FuzzResult>> or equivalent indexed result buffer so output
  remains deterministic;
- preserve the existing per-task 300-second project timeout;
- preserve the existing fallback FuzzResult for worker failure/cancellation;
- preserve progress increments at the same semantic completion point.

Once fan-out itself is bounded, a second semaphore is unnecessary unless
another invariant requires it.

Do not use an unbounded channel as a substitute for an unbounded task set.

## Workstream B3 — Bound port-scan task retention

In crates/eggsec/src/scanner/ports/mod.rs, replace the
permit-before-spawn plus retain-all-handles pattern with at most concurrency
live tasks.

Prefer tasks returning their result to the parent instead of mutating a shared
DashMap. If the parent can preserve all existing semantics, simplify:

- Arc<DashMap<u16, PortResult>>;
- results_count atomics used only for shared insertion;
- Arc::try_unwrap failure path.

Parent aggregation must preserve:

- total_matches_count;
- max_results behavior;
- progress events;
- final sort by port;
- MAX_SCAN_RESULTS truncation;
- spoof/raw-socket path unchanged;
- existing connect and outer task timeouts.

If max_results depends on completion order today, preserve completion-order
selection rather than silently changing it to input-order selection.

## Workstream B4 — Bound endpoint-scan task retention

Apply the same bounded-scheduler pattern to
crates/eggsec/src/scanner/endpoints.rs.

Preserve:

- one shared reqwest Client;
- redirect policy and TLS verification behavior;
- spoof headers;
- include_404 semantics;
- content-length/redirect extraction;
- is_interesting behavior;
- current timeout/error behavior;
- max_results completion-order semantics;
- final sorting: interesting first, then status, then path;
- total_endpoints_matched and interesting_findings.

Task-return aggregation should replace DashMap/atomics where it is genuinely
simpler. Do not alter public EndpointScanResults fields.

## Workstream B5 — Bound subdomain fan-out

In crates/eggsec/src/recon/subdomain.rs, convert verify_subdomains and
bruteforce from one-task-per-candidate with an internal semaphore to bounded
in-flight work.

Preserve:

- configured concurrency;
- per-query timeout;
- resolver behavior;
- MX/TXT/CNAME/IP checks;
- returned source names and result DTOs.

Candidate counts may be large, so the scheduler must not retain one suspended
Tokio task per candidate.

Review recon/dns_enhanced.rs after this conversion. Its wordlist is already
hard-capped, so convert it only if the same helper/pattern materially reduces
complexity or Phase A shows measurable retained-task cost. Record a no-change
decision if not.

## Workstream B6 — Do not over-generalize

Use one consistent scheduling pattern, but do not create a new public crate or
generic concurrency framework merely to deduplicate a few loops.

A small private helper is acceptable only if it improves readability and can
preserve each domain's result/error semantics without callback boxing or type
erasure.

Do not rewrite pipeline dependency-wave join_all in this phase. Wave cardinality
is small and dependency-defined; it is not part of the confirmed
high-cardinality fan-out problem.

## Tests

Add focused tests for:

1. peak live work <= configured concurrency;
2. concurrency==0 continues to fail where currently rejected;
3. fuzzer output index/order is unchanged;
4. worker timeout produces the same fallback/error representation;
5. port result sorting and max_results;
6. endpoint sorting/include_404/max_results;
7. subdomain result membership under bounded scheduling;
8. cancellation/drop leaves no detached tasks where the owning API supports
   cancellation.

Avoid flaky wall-time assertions as the primary correctness proof.

## Performance acceptance

Using the Phase A harness, record before/after:

- peak live tasks/work;
- peak RSS when available;
- wall time;
- result counts/checksums.

The required performance success criterion is structural: peak in-flight
scheduler state must be O(concurrency), not O(total work). Wall-time improvement
is desirable but not required if semantics and memory scalability improve.

## Verification

Run at minimum:

- cargo fmt --all --check
- cargo test -p eggsec --lib
- targeted scanner/fuzzer/recon integration tests
- make check
- make check-python
- bash scripts/check-architecture-guards.sh

Also run the affected Phase A profile cases in release mode.

## Completion record

Append starting/final SHA, files changed, peak-live-work before/after, wall/RSS
measurements, and any fan-out site intentionally left unchanged with reason.
