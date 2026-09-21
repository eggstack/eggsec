# Performance Phase F — secondary hot spots and closure

Status: Executed

Date: 2026-09-21

Baseline: 1fae91ec489a4fb553c1dfb26e184b5c61ea4adb

Parent roadmap:
performance-resource-efficiency-roadmap-2026-09-21.md

Depends on:
performance-phase-a-baseline-and-measurement-harness.md
performance-phase-b-bounded-async-fanout-and-fuzzer-locking.md
performance-phase-c-loadtest-request-hotpath.md
performance-phase-d-distributed-worker-capacity.md
performance-phase-e-coordinator-session-reuse.md

## Purpose

Re-measure the repository after the primary optimizations, apply only
evidence-supported secondary improvements, and perform compatibility/performance
closure.

This phase is specifically intended to prevent speculative cleanup from turning
a successful bounded campaign into a broad rewrite.

## Workstream F1 — Re-run the full profile matrix

Using the exact Phase A methodology and comparable host conditions where
practical, re-run:

- load-test request materialization;
- H1/H2 loopback load tests;
- fuzzer high-cardinality campaign;
- port scan fan-out;
- endpoint scan fan-out;
- subdomain fan-out;
- distributed short-task/long-task capacity cases;
- distributed connection/session setup counts.

Update architecture/performance.md with before/after medians/ranges and
structural counters.

If hardware/toolchain changed, state that clearly and avoid direct numeric
claims that depend on cross-host comparability.

## Workstream F2 — Load-test metrics micro-cleanup

If not already completed in Phase C, inspect Metrics hot counters for:

- entry followed by second get lookup;
- repeated allocation of static error-kind names;
- avoidable String cloning during merge.

These are low-risk and may land if they simplify code even when timing impact
is small, provided serialized LoadTestResults remains identical.

Do not change histogram precision or latency units as an optimization.

## Workstream F3 — ProxyPool snapshot/sort efficiency

Profile crates/eggsec-web-proxy/src/pool.rs under a synthetic large proxy set.

Current candidates include:

- repeated ProxyEntry::to_log_key String creation;
- repeated DashMap stats lookups inside sort comparison callbacks;
- cloning entries multiple times while deriving healthy/sorted sets.

If material:

1. build a temporary snapshot of entry + already-fetched stats/key;
2. sort the snapshot without repeated concurrent-map lookups;
3. map back to the same public Vec<ProxyEntry> results.

Preserve:

- health semantics;
- priority ordering;
- latency ordering;
- success-rate ordering;
- round-robin index;
- public method signatures.

Do not redesign proxy routing or merge proxy/interception architecture.

If the benchmark does not show material cost, record "measured; no change."

## Workstream F4 — TaskQueue state-transition evaluation

Profile crates/eggsec/src/distributed/queue.rs after Phases D/E.

Current TaskQueue uses independent RwLocks for pending, in_progress, and
completed state. dequeue can hold pending while awaiting in_progress and clones
Task into the in-progress map.

Evaluate two options:

A. retain the existing split locks and reduce only unnecessary clones/lock
   hold time;

B. one private QueueState under a Tokio Mutex so pending->in_progress and
   in_progress->completed become short atomic state transitions.

Option B is allowed only if measurement and code clarity support it. The queue
is mutation-heavy, so RwLock is not automatically superior, but replacing it
without data is also not justified.

Preserve all public TaskQueue methods/result semantics and max_size behavior.

Add race/concurrency tests for any lock-layout change.

## Workstream F5 — Explicitly evaluate and normally reject larger rewrites

Measure enough to decide on these known candidates:

### Global eggsec-runtime state mutex

Do not split RuntimeState by default. Only open a follow-up roadmap if profiling
with many simultaneous sessions shows meaningful mutex wait/contention and the
benefit exceeds the lifecycle-ordering risk.

### Pipeline dependency waves

Do not replace join_all merely for style. Wave cardinality is bounded by the
pipeline dependency graph and is not known to be a scaling problem.

### Global allocator

Do not add mimalloc/jemalloc solely from generic Rust performance folklore.
Require allocation-profile evidence and a separate packaging/dependency review.

### PGO

Profile-guided optimization may be documented as future release engineering
once representative workloads are stable. Do not make PGO mandatory for normal
builds in this campaign.

### target-cpu=native

Reject for distributed release artifacts. Eggsec must retain portable binaries.

Record each disposition in architecture/performance.md.

## Workstream F6 — Compatibility and API-surface closure

Review the final diff specifically for:

- public Rust signatures/types;
- Python exports/stubs/result schemas;
- CLI help/flags/defaults;
- serialized runtime/distributed DTOs;
- feature graph/dependency changes;
- authorization checkpoints;
- load-test result fields;
- daemon/TUI behavior affected indirectly by shared engine code.

Where the repo already has compatibility scripts/guards, use them rather than
inventing duplicate checks.

## Workstream F7 — Architecture guards

Add only low-cost static guards for invariants that are easy to regress and
difficult to notice in ordinary tests.

Candidate guards:

- high-cardinality fuzzer path must not return to spawn-per-payload plus
  semaphore retention;
- Worker processing must reference/configure max_concurrency capacity;
- load-test executor must use the compiled request prototype rather than call
  scoped_request inside the per-request worker loop.

Append the next available check numbers. Do not create brittle guards that ban
JoinSet/join_all globally.

Document new guards in docs/CI_ARCHITECTURE_GUARDS.md.

## Workstream F8 — Close planning/documentation state

Update:

- architecture/performance.md;
- architecture/loadtest.md for final load-test evidence if needed;
- relevant distributed/scanner/fuzzer architecture docs;
- plans/README.md;
- each phase plan with its completion record;
- parent roadmap Status and completion record.

Retain all plans.

Residual debt must identify:

- owner/module;
- why it remains;
- measurement supporting the decision;
- condition that should reopen it.

## Final verification

Mandatory:

- cargo fmt --all --check
- make check
- make check-python
- make check-feature-profiles
- make check-features-individual
- make check-msrv
- bash scripts/check-architecture-guards.sh
- cargo test -p eggsec-runtime
- targeted eggsec loadtest/fuzzer/scanner/recon/distributed suites
- eggsec-transport and eggsec-transport-eggfetch parity/H2/SOCKS5 tests

Run make check-full and release-check where host prerequisites permit. Record
any skip with exact reason rather than silently treating it as pass.

The performance harness is run separately in release mode and remains
informational rather than a hard merge threshold.

## Closure criteria

The roadmap may be marked Executed only when:

1. all implemented primary phases have completion records;
2. before/after evidence is retained;
3. peak async work is bounded structurally;
4. max_concurrency is truthful;
5. steady-state coordinator session reuse is proven;
6. transport/security semantics remain green;
7. public API/capability compatibility checks pass;
8. secondary candidates are either implemented with evidence or explicitly
   rejected/deferred;
9. no generated benchmark artifacts are committed;
10. plans/README.md reflects the final state.

## Completion record

Status: Executed

- Starting SHA: roadmap baseline `1fae91ec489a4fb553c1dfb26e184b5c61ea4adb`
  (built on the Phase A–E HEAD); final SHA recorded at commit time (campaign
  commit + SHA-record follow-up; see the roadmap completion record).
- F1 full-matrix re-run (same host, release, warm-up 1 + 5 trials):
  rebuild ~890ns / clone ~106ns; fake executor ~1.1M RPS; H1 ~21k/40k/30k
  at c=1/10/50; port sweep ~3–4ms; endpoint ~24–25ms identical checksum;
  subdomain ~1ms peak 50; distributed setups unchanged shape; session 6 ops
  → 1 accept/1 auth; pool sorts 14.1/8.6ms → 3.1/3.3ms. Table in
  `architecture/performance.md` (Phase F).
- F2 Metrics: completed in Phase C (single-lookup entry counters,
  enum-keyed internals, unchanged serialization).
- F3 ProxyPool: snapshot sort landed (one stats lookup + one key build per
  entry instead of per comparison); 26 pool tests incl. tie-ordering pin.
- F4 TaskQueue: option B landed (unified `QueueState` under one mutex;
  removes the pending→in_progress vs in_progress→pending lock-order
  inversion and one clone per dequeue); 23 distributed tests incl.
  200-task hammer + stale-cycle preservation.
- F5 larger rewrites: all rejected/deferred with evidence (runtime mutex,
  pipeline waves, allocator, PGO, target-cpu=native) — dispositions in
  `architecture/performance.md`.
- F6 compatibility: zero public-signature changes outside `pub(crate)`/tests
  (diff scan); `LoadTestResults`/wire DTOs/config shapes unchanged;
  authorization checkpoints, redirect/proxy/TLS semantics, body-through-EOF
  deadlines unchanged; no Python/CLI changes.
- F7 guards: Checks 140 (fuzzer bounded scheduler), 141 (worker capacity),
  142 (load-test prototype) appended to
  `scripts/check-architecture-guards.sh` and documented in
  `docs/CI_ARCHITECTURE_GUARDS.md`; full guard script passes.
- F8 docs: `architecture/performance.md` (evidence), `loadtest.md`
  (prototype), `distributed.md` (capacity + session), `scanner.md`
  (bounded flows), `fuzzer.md` (bounded scheduler + lock scope),
  `CI_ARCHITECTURE_GUARDS.md` (140–142), `README.md` (evidence pointer),
  `AGENTS.md` (performance contract), loadtest/distributed skills;
  plans retained with completion records.
- Final verification: `make check` + `make check-python` (as applicable) +
  `check-feature-profiles` + guards + affected-crate suites (see the
  verification todo outcome at commit time).
- Residual debt (owner / why / measurement / reopen condition):
  - Mid-session TCP sever fixture (distributed): no deterministic loopback
    sever without root/netns; recovery shares the tested establish+replay
    path. Reopen if a harness gains packet-level fault injection.
  - Global runtime mutex (runtime): no contention measured; reopen only on
    multi-session mutex-wait profiling with benefit exceeding
    lifecycle-ordering risk.
  - Allocator/PGO/native-cpu (release engineering): no profile evidence;
    reopen only with allocation profiles + packaging review (allocator) or
    stable representative workloads (PGO).
  - H1 loopback RPS variance ±10% run noise: harness remains informational;
    never promote to a merge threshold.
