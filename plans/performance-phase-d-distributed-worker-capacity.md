# Performance Phase D — distributed worker capacity

Status: Ready for implementation

Date: 2026-09-21

Baseline: 1fae91ec489a4fb553c1dfb26e184b5c61ea4adb

Parent roadmap:
performance-resource-efficiency-roadmap-2026-09-21.md

Depends on:
performance-phase-a-baseline-and-measurement-harness.md

## Purpose

Make WorkerConfig.max_concurrency a real resource limit and prevent the worker
from reserving substantially more coordinator work than it can execute.

This phase is intentionally local to worker scheduling. It does not change the
distributed JSON wire format or connection lifecycle; persistent session reuse
belongs to Phase E.

## Confirmed current behavior

WorkerConfig exposes max_concurrency (default 10), but
start_task_processing_loop spawns a Tokio task for every Task received from the
local channel.

The independent task-request loop asks for MAX_TASKS_PER_REQUEST (5) every five
seconds regardless of current running/reserved work.

Therefore the configured concurrency value does not currently bound:

- spawned task count;
- locally queued assigned tasks;
- resource pressure from long-running operations.

## Workstream D1 — Validate worker capacity

At Worker::start or construction/start boundary, reject max_concurrency == 0
with a structured configuration error.

Do not silently coerce zero to one.

Preserve WorkerConfig serialization and defaults.

## Workstream D2 — Introduce one capacity accounting model

Create one internal capacity mechanism shared by acquisition and execution.

Preferred model:

- Arc<Semaphore> for execution permits, plus
- a small atomic/locked reserved-work count that includes work assigned by the
  coordinator but not yet terminal.

An alternative actor-owned counter is acceptable if it makes invariants easier
to prove.

Required invariant:

reserved_not_terminal <= max_concurrency

where reserved includes queued plus actively executing assignments.

Do not count completed work awaiting only local statistics cleanup as active.

## Workstream D3 — Make task acquisition capacity-aware

Before RequestTasks, calculate available assignment capacity.

Request:

min(MAX_TASKS_PER_REQUEST, available_capacity)

If available capacity is zero, skip the network task-request operation for that
tick.

When tasks are returned, reserve capacity before exposing them to the processing
loop. The design must avoid a race where two loops both observe the same free
permit/counter and over-reserve.

If the coordinator returns more tasks than requested, fail safely rather than
spawning beyond the configured limit. Record/log the protocol anomaly and
handle tasks according to existing queue durability semantics; do not silently
drop assigned work.

Do not change CommandMessage::RequestTasks in this phase.

## Workstream D4 — Bound task processing

Replace detached one-task-per-received-item spawning with a JoinSet or another
owned bounded task set.

Each running task must:

- own a capacity reservation/permit;
- release it exactly once on success, execution error, timeout, or cancellation;
- update WorkerStats exactly once;
- preserve TASK_PROCESSING_TIMEOUT;
- preserve result submission behavior;
- remain owned so shutdown can wait/abort according to existing Worker
  lifecycle semantics rather than leaving detached tasks.

If the existing Worker shutdown path currently does not await all processing
children, close that lifecycle gap as part of the capacity implementation
without changing public methods.

## Workstream D5 — Make statistics derive from the same truth

WorkerStats.tasks_in_progress should correspond to the actual executing
capacity state.

Avoid maintaining one semaphore count and a separately racy stats count that
can diverge under timeout/error.

Tests should assert:

tasks_in_progress <= max_concurrency

through all terminal paths.

Completed/failed counters remain saturating/compatible.

## Workstream D6 — Preserve scheduling semantics

Do not change:

- task type dispatch;
- enforcement context;
- operation policy;
- per-task payload/result schema;
- coordinator queue ownership;
- stale-task timeout;
- heartbeat format;
- TLS/PSK behavior.

Do not add work-stealing or priority scheduling here.

## Tests

Add deterministic tests for:

1. max_concurrency 1 with many queued tasks never exceeds one live execution;
2. max_concurrency N never exceeds N;
3. long-running tasks cause task requests to pause when capacity is full;
4. completion frees one slot and permits the next request/execution;
5. execution error frees the slot;
6. 300-second timeout path (using controllable test timeout/helper rather than
   sleeping 300 seconds) frees the slot and increments failed once;
7. shutdown leaves no detached processing task;
8. stats remain consistent;
9. max_concurrency==0 fails explicitly;
10. wire serialization is unchanged.

Use test-only barriers/notifies instead of timing races wherever possible.

## Performance/efficiency evidence

From Phase A distributed fixture, record before/after:

- configured vs observed peak tasks_in_progress;
- locally reserved task count;
- number of task-request calls while saturated;
- peak RSS if available;
- throughput on short tasks;
- behavior on long tasks.

The primary success criterion is bounded resource use and truthful capacity,
not maximal no-op-task RPS.

## Verification

Run at minimum:

- cargo fmt --all --check
- targeted distributed worker/queue tests
- cargo test -p eggsec --lib
- make check
- make check-python
- make check-feature-profiles
- bash scripts/check-architecture-guards.sh

If distributed paths need broader feature activation, use the existing
representative feature profile rather than inventing a new production feature.

## Completion record

Append baseline/final SHA, capacity model chosen, peak observed concurrency,
network request-count changes, timeout/shutdown evidence, and exact validation
commands.
