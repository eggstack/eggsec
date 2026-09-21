# Performance Phase E — coordinator session reuse

Status: Executed

Date: 2026-09-21

Baseline: 1fae91ec489a4fb553c1dfb26e184b5c61ea4adb

Parent roadmap:
performance-resource-efficiency-roadmap-2026-09-21.md

Depends on:
performance-phase-d-distributed-worker-capacity.md

## Purpose

Remove repeated TCP/TLS/PSK setup from steady-state distributed worker control
traffic while preserving the existing JSON protocol and failure semantics.

This phase is connection-lifecycle work and must remain independently
reviewable from Phase D capacity enforcement.

## Confirmed protocol property

The coordinator authenticates a connection once and then enters a command loop
that can process multiple CommandMessage values on the same LineWriter.

The current client/worker usage does not exploit this fully: heartbeat,
task-request, and result paths can construct fresh RemoteClient instances and
open/authenticate new connections repeatedly.

Connection-local worker identity also matters: the coordinator sets
connected_worker_id when Register is received on that connection and heartbeat
updates use that connection-local association.

A reused registered session therefore matches the server's current design
better than independent one-shot heartbeat connections.

## Workstream E1 — Define persistent session state

Extend RemoteClient internals or add a private CoordinatorSession owner that
tracks:

- host/port;
- TLS configuration/domain;
- authenticated LineWriter when connected;
- remembered worker registration metadata after register_worker succeeds;
- connection generation/reconnect state;
- DNS cache as currently supported.

Public RemoteClient method signatures should remain source-compatible.

Prefer one owner of the LineWriter. Do not wrap a socket in unrelated mutexes
throughout the codebase.

## Workstream E2 — Serialize command/response exchange safely

The line protocol is request/response and does not currently carry a general
multiplexing correlation implementation suitable for arbitrary concurrent
writes on one stream.

Therefore steady-state commands should be serialized by one connection owner.

Preferred worker architecture:

- one bounded mpsc command channel;
- one coordinator-session task owns RemoteClient/LineWriter;
- heartbeat, task acquisition, and result submission send typed internal
  requests to the owner;
- oneshot response channels return method results where needed.

An Arc<tokio::sync::Mutex<RemoteClient>> is acceptable only if review proves
there is no lock-order/lifecycle complexity and no unrelated lock is held while
awaiting I/O. The actor model is preferred because it makes connection ownership
and reconnect sequencing explicit.

Bound the command queue. Result submission must not become an unbounded
memory sink.

## Workstream E3 — Reconnect and registration recovery

On EOF, connection error, TLS failure, or protocol parse failure:

1. discard the broken LineWriter;
2. reconnect using the existing timeout/TLS rules;
3. perform PSK authentication;
4. if this worker was previously registered, replay Register using remembered
   id/hostname/capabilities before relying on connection-local worker identity;
5. retry the original idempotent control operation only when doing so cannot
   duplicate task execution/result semantics.

Do not add broad automatic retries to arbitrary Execute operations.

Heartbeat is safe to retry after reconnect.

Task acquisition needs explicit duplicate-assignment analysis: a lost response
after the coordinator dequeues tasks must not cause hidden task loss or
duplicate execution. If the existing protocol cannot prove safe transparent
retry for RequestTasks, surface the error and let the next normal poll recover
through existing stale-task behavior rather than guessing.

Result submission likewise needs an idempotency analysis around task_id before
any transparent retry. Prefer existing complete semantics over speculative
replay.

Document each method's retry disposition.

## Workstream E4 — Keep authentication and TLS unchanged

Do not:

- weaken certificate verification;
- widen plaintext opt-in;
- cache PSK auth across distinct TCP connections;
- skip authentication on reconnect;
- bypass connect timeout;
- make DNS cache authoritative beyond its current TTL semantics.

Persistent reuse means fewer handshakes, not weaker handshakes.

## Workstream E5 — Worker integration

After Phase D, the worker should share one coordinator session for:

- initial registration;
- heartbeat;
- capacity-aware RequestTasks;
- task-result submission.

Avoid constructing a new TLS client for every task result.

Shutdown must:

- stop accepting new session commands;
- resolve/fail pending callers;
- close/drop the connection owner;
- leave no detached reconnect loop.

## Workstream E6 — Backoff without responsiveness regression

A broken coordinator must not cause a tight reconnect loop.

Use bounded reconnect backoff consistent with existing polling cadence and
shutdown responsiveness. Do not sleep while holding a public lock.

Keep shutdown cancellation selectable so the worker can terminate promptly.

Do not add long persistent backoff state to the wire protocol.

## Tests

Add loopback TLS tests proving:

1. register + multiple heartbeats + task requests + results use one accepted
   connection in steady state;
2. only one PSK authentication occurs on that steady-state connection;
3. forced connection close triggers a new TLS/auth exchange;
4. reconnect re-registers the worker before a heartbeat depends on
   connected_worker_id;
5. malformed/failed auth never falls back to an unauthenticated session;
6. shutdown cancels reconnect and pending command waiters;
7. command queue is bounded;
8. response association remains correct under concurrent callers;
9. RequestTasks/result retry disposition avoids duplicate execution;
10. public RemoteClient/Worker serialization/API remains compatible.

## Performance evidence

Record before/after for a fixed control workload:

- coordinator TCP accepts;
- TLS handshakes;
- PSK auth exchanges;
- heartbeats;
- task-request calls;
- result submissions;
- wall time;
- CPU/RSS if useful.

For a stable worker session, accepted connections/TLS/auth should approach one
per healthy connection lifetime rather than one per control message.

## Verification

Run at minimum:

- cargo fmt --all --check
- targeted distributed remote/worker integration tests
- cargo test -p eggsec --lib
- make check
- make check-python
- make check-feature-profiles
- make check-msrv
- bash scripts/check-architecture-guards.sh

If daemon/runtime protocol fixtures cover the same DTOs, run them as well.

## Completion record

Status: Executed

- Starting SHA: roadmap baseline `1fae91ec489a4fb553c1dfb26e184b5c61ea4adb`
  (built on the Phase A–D HEAD); implementation commit `4fb021d`.
- Session architecture: `CoordinatorSession` (crate-internal, `remote.rs`)
  — one actor task owns the `LineWriter`; bounded 64-command mpsc channel
  with `oneshot` replies (actor model, not a mutex-wrapped socket);
  remembered registration metadata replayed on every (re)connect; the
  actor owns a single `RemoteClient` so DNS cache persists across
  reconnects within TTL. One-shot `RemoteClient` methods are byte-identical
  for existing CLI/tool callers.
- Files changed:
  - `crates/eggsec/src/distributed/remote.rs` (E1/E2: session config/state/
    actor/`exchange`/`establish`/windowed reconnect; E3: reconnect +
    registration replay + documented retry matrix; E6: windowed pacing
    without actor sleeps; test-only accept/auth/completed counters;
    `ConnectionDeps` for the clippy arg-count lint; 7 session tests);
  - `crates/eggsec/src/distributed/worker.rs` (E5: registration, heartbeat,
    capacity-aware acquisition, and result submission all multiplex over the
    shared session; session-first shutdown ordering; no fresh TLS client
    per message);
  - `scripts/perf-profile.sh` (`session` suite);
  - `architecture/performance.md` (Phase E evidence table).
- Connection/auth counts before/after: 6-op steady workload 6 accepts →
  1 accept / 1 auth / 1 live connection (plaintext); closure-polish adds
  the verified-TLS counterpart with 1 accept / 1 handshake / 1 auth
  (`session_reuses_single_tls_connection_in_steady_state`); 12-op fixture
  ~496ms of setups → single setup per healthy lifetime.
- Reconnect evidence: outage-then-recovery test drives re-establishment +
  registration replay via heartbeat alone; closure-polish adds the
  deterministic established-session sever fixture
  (`session_severed_tls_connection_reconnects_with_reregister`: A Register
  + heartbeat, server-side drop of A, fresh B with new TCP/TLS/auth,
  Register replayed before heartbeat, retry succeeds; accepts 2,
  handshakes 2, auths 2, peer addrs differ, zero unauthenticated
  commands); wrong-PSK test proves no unauthenticated fallback (0 auths,
  0 live connections); shutdown test proves prompt caller failure with no
  detached loop; burst test proves queue liveness past the bound;
  concurrency test proves response association across 5 parallel results.
- Retry matrix: heartbeat once (idempotent; proven by the sever test's
  successful retry); RequestTasks/result never transparently retried
  (dequeue-loss and duplicate-complete analysis in code comments, plus
  explicit regression tests `session_request_tasks_never_retried_on_failure`
  and `session_result_never_retried_on_failure`: one wire message per
  caller call on failure, next explicit call recovers); `Execute`
  untouched.
- Checks (A–E): `cargo test -p eggsec --lib distributed::` (25 passed);
  `--test distributed_tests` (21 passed); full lib 1466 passed;
  `cargo clippy -p eggsec --no-default-features` + `--features cli` clean;
  `cargo fmt --all --check` clean; release session suite green. Post-polish
  re-validation is recorded in the closure-polish plan, not cited here.
- Former limitation withdrawn: deterministic mid-session sever no longer
  requires root/netns — the loopback sever fixture above covers
  sever/reconnect/re-auth/re-register without privileged networking
  (see `architecture/performance.md` polish addendum).
- Compatibility: no wire-format change (`CommandMessage`/`ResponseMessage`
  untouched), no public API removal/signature change (session types are
  crate-internal; one-shot client methods unchanged; test-only trust
  constructors are `#[cfg(test)]` crate-private), auth/TLS semantics
  unchanged (verified test trust, never the `insecure-tls` bypass).
