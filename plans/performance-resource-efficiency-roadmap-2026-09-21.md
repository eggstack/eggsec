# Performance and resource-efficiency optimization roadmap

Status: Ready for implementation

Date: 2026-09-21

Baseline: 1fae91ec489a4fb553c1dfb26e184b5c61ea4adb

## Purpose

Improve Eggsec runtime throughput, allocation behavior, peak memory use, and
distributed-worker efficiency without changing the supported API surface,
command behavior, Python surface, transport authorization semantics, or
security capability.

The current repository has already completed substantial crate-boundary,
transport, frontend/runtime, and dependency work. This campaign therefore does
not reopen those architecture lines. It targets measured runtime hot paths that
remain inside the established boundaries.

The campaign is deliberately evidence-driven. It must distinguish:

- throughput from scheduler/memory scalability;
- useful concurrency from merely spawning more Tokio tasks;
- local CPU/allocation costs from remote network latency;
- control-plane connection overhead from task execution time;
- benchmark evidence from correctness requirements.

No noisy throughput threshold belongs in normal merge CI.

## Confirmed baseline findings

At the baseline SHA:

1. Fuzzer concurrency is partially serialized in
   crates/eggsec/src/fuzzer/engine/utils.rs because the TimingAnalyzer mutex
   guard remains in scope after timing.record(...) while successful responses
   are subsequently read and scanned.
2. Fuzzer concurrent execution in
   crates/eggsec/src/fuzzer/engine/execution.rs creates one Tokio task per
   payload and has each task wait on a semaphore. Concurrency is bounded, but
   scheduler state and JoinHandle retention scale with total payload count.
3. Port scanning in crates/eggsec/src/scanner/ports/mod.rs acquires a permit
   before spawning, so active work is bounded, but it retains one JoinHandle
   per scanned port until join_all at the end. It also uses shared DashMap and
   atomic result state where task-return aggregation can potentially suffice.
4. Endpoint scanning in crates/eggsec/src/scanner/endpoints.rs has the same
   retained-handle shape and shared DashMap/atomic aggregation.
5. Subdomain verification/bruteforce in
   crates/eggsec/src/recon/subdomain.rs creates one task per candidate and
   gates inside the task with a semaphore, so large candidate sets retain
   O(total candidates) suspended tasks.
6. The load-test worker architecture itself is already good: each worker owns
   a private HDR histogram/status accumulator and merges at completion.
   However, RequestTemplate::scoped_request reconstructs an otherwise
   immutable request on every operation: method parsing, URL parsing,
   temporary String HashMap construction, header conversion, body Vec cloning,
   and policy/hint cloning all occur in the request hot loop.
7. The production Eggfetch load-test path drains response bodies through EOF
   and retains them in ScopedHttpResponse even though the load-test executor
   consumes only the status. Body-through-EOF semantics are intentional and
   must remain; retaining the bytes may be avoidable only if the transport can
   drain without accumulating.
8. WorkerConfig.max_concurrency is part of the public distributed-worker
   configuration but start_task_processing_loop currently spawns every
   dequeued task without enforcing that limit.
9. The worker task-request loop polls a fixed MAX_TASKS_PER_REQUEST every five
   seconds independently of current local execution capacity, so assigned work
   can grow beyond the configured concurrency budget.
10. RemoteClient caches DNS only within one instance, while worker heartbeat,
    task-request, and task-result paths repeatedly construct clients and
    reconnect. Each operation can therefore pay TCP connect, TLS handshake,
    PSK authentication, and request/response setup again even though the
    coordinator already supports multiple CommandMessage values on one
    authenticated connection.
11. Secondary allocation/contention candidates remain in load-test metrics,
    ProxyPool sorting/stat lookup, and TaskQueue state transitions. These are
    not first-pass rewrite targets until profiling shows material payoff.
12. The global eggsec-runtime state mutex is a theoretical cross-session
    contention point, but current evidence does not justify restructuring it.
    It is explicitly deferred unless Phase A/F measurements demonstrate a real
    bottleneck.
13. architecture/loadtest.md retains useful repeated loopback evidence, but
    the performance harness used for the latest qualification was ephemeral
    rather than a committed reproducible profiling target.

## Global invariants

Every phase must preserve all of the following:

1. No public Rust API removal or signature change solely for performance.
2. No CLI flag, default, exit-status, output-schema, or command-capability
   regression.
3. No Python callable/result-schema regression.
4. No operation-ID, runtime TaskKind, daemon message, or serialized DTO change
   unless a phase explicitly proves an additive compatibility-safe extension
   is required. The planned phases do not require one.
5. Scope enforcement and the existing transport checkpoint order remain
   unchanged.
6. Logical URL identity, singular authorized physical-route pinning,
   same-host/authority redirect rules, proxy authorization, TLS consistency,
   no automatic retries, and HTTP/3-off behavior remain unchanged.
7. Load-test latency continues to include response-body drain through EOF and
   the aggregate timeout continues through EOF.
8. Fuzzer output ordering, payload/result association, timing classification,
   leak scanning, and error representation remain behaviorally compatible.
9. Scanner result sorting, max-results behavior, progress contracts, timeouts,
   and spoofing behavior remain compatible.
10. Distributed tasks remain bounded by the existing task timeout and no task
    may become detached from shutdown/cancellation merely to improve
    throughput.
11. Release artifacts remain portable; do not add target-cpu=native or a
    platform-specific runtime dependency to obtain benchmark wins.
12. Generated benchmark output belongs under target/, /tmp, or another ignored
    location. Retain summarized evidence in architecture/performance.md, not
    raw generated logs in plans/.

## Ordered implementation sequence

### Phase A — Baseline and reproducible measurement harness

Plan:
performance-phase-a-baseline-and-measurement-harness.md

Create a deterministic, local-only performance harness and record baseline
throughput, latency, peak live work, connection counts, and memory where
portable. The harness must be useful for before/after comparisons without
turning noisy timing into a merge gate.

Exit condition: every later optimization has a reproducible baseline and a
clear semantic invariant to protect.

### Phase B — Bounded async fan-out and fuzzer lock scope

Plan:
performance-phase-b-bounded-async-fanout-and-fuzzer-locking.md

Narrow the fuzzer timing mutex to timing mutation only, then replace
O(total-work) spawned/retained task sets in the high-cardinality fuzzer,
scanner, and subdomain paths with O(concurrency) in-flight scheduling while
preserving deterministic outputs and existing timeouts.

Exit condition: peak live Tokio work and retained JoinHandle state are bounded
by configured concurrency rather than total payload/port/endpoint/candidate
count.

### Phase C — Load-test request hot-path optimization

Plan:
performance-phase-c-loadtest-request-hotpath.md

Compile the immutable transport request once per run, use cheap DTO clones in
workers, remove avoidable metrics-map work, and investigate a drain-without-
retain response path only if it can preserve transport semantics without
contorting the shared contract.

Exit condition: request materialization no longer reparses/rebuilds immutable
state per request, and before/after loopback evidence is retained.

### Phase D — Distributed worker capacity enforcement

Plan:
performance-phase-d-distributed-worker-capacity.md

Make WorkerConfig.max_concurrency a real execution/resource contract and make
task acquisition capacity-aware without changing the wire format.

Exit condition: running plus locally reserved work cannot exceed configured
capacity, including timeout/error/shutdown paths.

### Phase E — Persistent coordinator session reuse

Plan:
performance-phase-e-coordinator-session-reuse.md

Reuse authenticated coordinator connections across registration, heartbeats,
task acquisition, and result submission, with reconnect/re-auth/re-register
behavior explicitly tested.

Exit condition: steady-state worker control traffic no longer performs one
TCP/TLS/auth setup per message, while failure recovery remains correct.

### Phase F — Secondary hot spots and closure qualification

Plan:
performance-phase-f-secondary-hotspots-and-closure.md

Re-measure. Apply only evidence-supported low-risk secondary optimizations in
metrics, ProxyPool, and TaskQueue; explicitly reject speculative runtime/pipeline
rewrites when the data does not support them; then run compatibility and
release qualification and close the roadmap.

Exit condition: retained before/after evidence shows the effect of each landed
change, all compatibility gates pass, and residual performance debt has an
owner/removal criterion.

## Ordering rationale

Phase A lands first so optimization claims are reproducible.

Phase B and Phase C are the highest-confidence local hot-path work and may be
implemented independently after Phase A. Phase D should land before Phase E so
connection/session measurements are taken against a worker whose execution
capacity is already truthful. Phase F depends on all implemented phases and is
the only phase allowed to turn secondary candidates into code changes.

Do not combine Phase D and Phase E into one large distributed rewrite. Capacity
enforcement is a local resource-control correction; persistent session reuse is
a connection-lifecycle change and deserves independent review and rollback.

## Roadmap acceptance criteria

The line is complete only when:

1. A committed local-only profiling harness can reproduce the key workloads.
2. Fuzzer TimingAnalyzer locking never spans response-body await/scanning.
3. High-cardinality fan-out keeps O(concurrency) work in flight.
4. Fuzzer output remains deterministically associated with input payloads.
5. Port/endpoint/subdomain result and timeout semantics remain compatible.
6. Load-test request parsing/header/body materialization is moved out of the
   per-request hot loop where immutable.
7. Existing load-test body-through-EOF deadline semantics remain unchanged.
8. Worker max_concurrency is enforced under success, failure, timeout, and
   shutdown.
9. Worker task requests reflect local available capacity rather than fixed
   polling alone.
10. Steady-state worker coordinator traffic reuses authenticated sessions.
11. Reconnect performs the required authentication and worker-registration
   recovery before connection-local heartbeat state is relied upon.
12. No new unbounded queue/task set is introduced.
13. architecture/performance.md records environment, commands, repetitions,
   warm-up, before/after ranges/medians, and limitations.
14. make check, make check-python, representative feature checks, architecture
   guards, MSRV verification, and targeted affected-crate tests pass.
15. The public API/capability surface is unchanged except for strictly additive
   internals/documentation explicitly recorded in completion evidence.

## Explicit exclusions

This roadmap does not authorize:

- another crate-extraction campaign;
- replacing Eggfetch or reopening the completed transport migration;
- changing authorization or proxy-routing semantics;
- changing load-test latency definition to headers-only;
- dropping response bodies before EOF merely to report higher RPS;
- adding automatic retries or HTTP/3;
- changing public result DTOs to reduce allocations;
- changing distributed wire messages merely to optimize local scheduling;
- replacing Tokio;
- target-cpu=native release artifacts;
- allocator replacement without profile evidence;
- restructuring the global runtime mutex without demonstrated contention;
- pipeline dependency-wave redesign without demonstrated contention;
- hard throughput thresholds in ordinary CI.

## Handoff record requirements

Each phase must append a completion record containing:

- starting and final SHAs;
- exact benchmark/verification commands;
- test environment and relevant CPU/OS/Rust versions;
- before/after measurements with repetitions and warm-up stated;
- peak-live-work or connection-count evidence where relevant;
- compatibility tests run;
- any candidate investigated but intentionally not implemented, with reason.

Do not mark the roadmap executed because one benchmark improved. Closure
requires compatibility evidence and the complete ordered campaign disposition.
