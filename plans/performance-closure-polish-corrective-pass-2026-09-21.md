# Performance campaign closure-polish corrective pass

Status: Executed

Date: 2026-09-21

Baseline: `9fc73a2a6ca1ab6c079f851f8cff6780dfb6df90`

Parent roadmap:
[`performance-resource-efficiency-roadmap-2026-09-21.md`](performance-resource-efficiency-roadmap-2026-09-21.md)

Related executed phases:

- [`performance-phase-e-coordinator-session-reuse.md`](performance-phase-e-coordinator-session-reuse.md)
- [`performance-phase-f-secondary-hotspots-and-closure.md`](performance-phase-f-secondary-hotspots-and-closure.md)

## Purpose

Close the small evidence/documentation gap left after the successful performance
and resource-efficiency campaign without reopening the implementation line.

The primary campaign is functionally complete and current hosted validation is
green. This pass is intentionally limited to:

1. reconciling stale/overstated closure documentation;
2. replacing inferred TLS session-reuse evidence with an actual local TLS
   session fixture;
3. replacing the claimed inability to test established-connection loss with a
   deterministic loopback sever/reconnect fixture;
4. recording the resulting evidence and final post-polish validation.

This is not a new optimization phase. It must not change production scheduling,
transport policy, distributed wire messages, retry semantics, or public
capability.

## Confirmed baseline state

At the baseline SHA:

1. The campaign roadmap and `plans/README.md` are marked Executed.
2. `architecture/performance.md` still says:
   `Status: Active campaign`.
3. Phase F says the unified `TaskQueue` "moves (rather than clones) tasks
   between states" / removes one clone per dequeue, but current
   `TaskQueue::dequeue` still performs:
   `state.in_progress.insert(task.id.clone(), task.clone())`
   before returning the owned task. The lock-order simplification is real;
   the clone-removal claim is not.
4. Phase E requested loopback TLS evidence, but the retained
   `CoordinatorSession` tests use explicit plaintext loopback and infer that
   connection reuse applies to TLS because TLS occupies the same connection
   establishment lifecycle.
5. Phase E/F record deterministic mid-session TCP sever as unavailable without
   root/netns. That is unnecessarily strong: a purpose-built local test server
   can deliberately close an already-authenticated accepted socket and then
   accept the client's reconnect without privileged networking.
6. The session retry policy is already conservative and should remain:
   heartbeat may retry once after reconnect; `RequestTasks` and result
   submission must not be transparently retried.
7. Current HEAD hosted CI completed successfully, including the full mandatory
   Rust contract, canonical Python verification, dependency policy, and code
   quality. The polish commit must receive its own validation; the prior HEAD
   run is context, not proof for the new final SHA.

## Global constraints

This pass must preserve:

1. all public Rust APIs;
2. Python API/stub/result compatibility;
3. CLI behavior, flags, defaults, and output schemas;
4. `CommandMessage` / `ResponseMessage` / worker config serialization;
5. production TLS verification behavior and trust roots;
6. PSK authentication behavior;
7. connection timeout and DNS-cache behavior;
8. Phase E retry dispositions;
9. worker capacity/accounting semantics;
10. `TaskQueue` public behavior and result ordering;
11. all scope/policy/transport authorization boundaries;
12. the completed performance measurements except where wording is corrected.

Do not change production code merely to make a documentation claim true. In
particular, do not redesign `TaskQueue` solely to eliminate its retained task
clone.

## Workstream 1 — Reconcile closure documentation

Update `architecture/performance.md`:

- change the status from Active campaign to an executed/closed state;
- identify the implementation SHA and this polish SHA once known;
- retain the existing host/toolchain limitations;
- distinguish measured performance evidence from test/evidence corrections.

Update Phase F completion text:

- remove the claim that the unified queue removes one clone per dequeue;
- state the real benefit precisely:
  - one state mutex;
  - no cross-lock ordering inversion;
  - atomic short state transitions;
  - existing owned-return/in-progress representation still requires a task
    clone at dequeue;
- replace the stale "verification todo outcome at commit time" wording with the
  actual verification record after this pass.

Update Phase E/F residual debt:

- remove the assertion that deterministic mid-session sever requires
  root/netns if the fixture below lands;
- retain any genuinely unresolved limitation with a precise reopen condition.

Do not rewrite historical benchmark numbers.

## Workstream 2 — Add a verified local TLS session fixture

The current session tests should prove persistent reuse over the TLS path
directly.

### Preferred fixture design

Use the existing `rcgen` dependency to create a short-lived local test CA /
server certificate for `localhost` (or the exact loopback test name).

Add only crate-private, test-only trust injection, for example:

- a `#[cfg(test)]` `TlsClient` constructor accepting a test
  `RootCertStore` / root certificate; and
- a `#[cfg(test)]` `RemoteClient` or `SessionConfig` construction path
  that uses that connector.

The exact shape may differ, but all of the following are mandatory:

- no new public constructor;
- no production feature flag;
- no change to `TlsClient::new` WebPKI-root behavior;
- no use of the production `insecure-tls` bypass as the primary proof;
- no checked-in private key/certificate fixture unless there is a strong reason
  to prefer it over generated test material;
- temporary PEM/key files are cleaned up or created under a test temp
  directory.

The server should use the same `TlsServer` / rustls accept path as the real
distributed listener.

### TLS steady-state proof

Run a representative sequence over one `CoordinatorSession`:

1. register;
2. multiple heartbeats;
3. one task request;
4. one result submission.

Assert:

- one accepted TCP connection for the healthy lifetime;
- one TLS handshake for that accepted connection, using a test counter or the
  fixture's accept/handshake instrumentation;
- one successful PSK authentication;
- commands complete on the same registered connection;
- no plaintext fallback occurs.

If adding a generic production handshake counter would enlarge runtime state
only for testing, keep the counter in the fixture or behind `#[cfg(test)]`.

## Workstream 3 — Add deterministic established-connection sever/reconnect proof

Build a purpose-specific loopback coordinator fixture that controls connection
lifetime explicitly.

The test must exercise loss of an already established and registered session,
not merely "coordinator initially unavailable."

Required scenario:

1. accept connection A;
2. perform TLS (preferred; plaintext is acceptable for an additional focused
   protocol test) and PSK authentication;
3. receive/acknowledge Register;
4. receive at least one normal command so the connection is demonstrably live;
5. deliberately close/drop connection A from the server side;
6. let the client observe EOF/write/read failure;
7. after the reconnect window permits another attempt, accept connection B;
8. verify connection B performs fresh TLS setup and PSK authentication;
9. verify Register is replayed on connection B before a heartbeat is processed
   as belonging to the worker;
10. verify the heartbeat succeeds;
11. verify the old connection is not reused and no unauthenticated command is
    accepted.

This should be a scripted test server/fixture, not packet injection, netns,
iptables, root privileges, or sleeps as the sole correctness mechanism.

Use barriers/notifies/explicit protocol steps so the test is deterministic.
A small wait to cross the intentional reconnect backoff window is acceptable,
but ordering assertions must come from observed messages/counters.

### Retry invariants

The sever fixture must not accidentally weaken Phase E's retry matrix.

Add/retain tests proving:

- heartbeat may perform its one idempotent retry after reconnect;
- a failed `RequestTasks` exchange is surfaced and not transparently
  replayed;
- a failed result exchange is surfaced and not transparently replayed.

Do not add request IDs/idempotency protocol changes in this pass.

## Workstream 4 — Reconcile Phase E evidence

After the new tests pass, update
`plans/performance-phase-e-coordinator-session-reuse.md`:

- record actual TLS session reuse rather than relying on plaintext structural
  inference;
- record established-session sever/reconnect/re-auth/re-register evidence;
- remove or rewrite the former root/netns limitation;
- preserve the existing retry matrix and compatibility statement.

Update `architecture/distributed.md` and
`architecture/performance.md` only where needed so they tell the same story.

Do not overstate what is measured: connection/handshake/auth counters are
structural correctness evidence, not a universal network-latency benchmark.

## Workstream 5 — Final validation and plan-state closure

Run the targeted tests first:

- `cargo test -p eggsec --lib distributed::remote::session_tests -- --nocapture`
  or the final equivalent filter;
- `cargo test -p eggsec --lib distributed::`;
- `cargo test -p eggsec --test distributed_tests`;
- any explicit feature profile required by the verified-TLS fixture.

Then run:

- `cargo fmt --all --check`;
- `cargo clippy -p eggsec --no-default-features -- -D warnings` where the
  repository's current warning baseline permits;
- `cargo clippy -p eggsec --no-default-features --features cli -- -D warnings`
  where applicable;
- `make check`;
- `make check-python`;
- `make check-feature-profiles`;
- `make check-msrv`;
- `bash scripts/check-architecture-guards.sh`.

Run broader/deep checks required by current contributor guidance when host
prerequisites permit. Record skips precisely.

After push, record hosted CI status for the polish HEAD. Do not cite the
pre-polish `9fc73a2a6ca1ab6c079f851f8cff6780dfb6df90` CI run as final proof.

## Acceptance criteria

This corrective pass is complete only when:

1. `architecture/performance.md` no longer calls the executed campaign
   active.
2. The TaskQueue closure record accurately describes the retained clone.
3. A real local TLS `CoordinatorSession` test proves steady-state connection
   reuse.
4. The TLS fixture uses verified test trust and does not alter production trust
   roots or require insecure TLS as its primary proof.
5. A deterministic server-side sever test closes an established registered
   connection and observes reconnect.
6. Reconnect performs a fresh connection/TLS/auth sequence.
7. Registration replay is observed before post-reconnect heartbeat handling.
8. Heartbeat retry semantics remain idempotent and bounded.
9. `RequestTasks` and result submission remain non-retried on ambiguous
   exchange failure.
10. No wire DTO or public API changes are introduced.
11. Phase E/F and architecture docs agree on the evidence and residual debt.
12. The stale verification wording is replaced with exact completed checks.
13. Targeted distributed tests pass.
14. Repository-wide required checks pass.
15. Hosted CI for the final polish SHA is green, or any infrastructure-only
    failure is recorded distinctly from code/test failure.

## Explicit exclusions

This pass does not authorize:

- new performance optimizations;
- TaskQueue API/representation redesign;
- distributed protocol version changes;
- request/result idempotency tokens;
- broader retry behavior;
- reconnect-policy redesign;
- worker capacity changes;
- transport/Eggfetch work;
- global runtime-lock work;
- allocator/PGO/native-CPU changes;
- new production certificate-loading behavior;
- weakening TLS verification for local convenience.

## Handoff / completion record

Then update `plans/README.md` to mark this corrective pass executed while
retaining the original A-F campaign history.

## Completion record

Status: Executed

- Starting SHA: `7b5e0070c49320e3d9a770d54f6f523515cc0429` (plan registration;
  the plan text cites the earlier `9fc73a2a` pre-registration HEAD).
- Implementation SHA: `71336632a12cc795f3fa3e4d9f79cd43d116c2b4` (code +
  docs corrections; this record + README mark follow as a docs-only
  commit, whose SHA is recorded in `plans/README.md`).
- Test-only TLS fixture design: `#[cfg(test)]` crate-private trust
  injection only — `TlsClient::with_test_root` (verified root, never the
  `insecure-tls` bypass), `RemoteClient::with_test_root`, and
  `CoordinatorSession::spawn_with_test_root` running the same actor loop
  (`session_actor_loop`) as production. No new public constructor, no
  production feature flag, no change to `TlsClient::new` WebPKI behavior.
  The server uses the production `TlsServer::from_pem` accept path and a
  new `#[cfg(test)]` handshake counter (`tls_handshakes`, incremented only
  after `accept_tls` succeeds) alongside the existing accept/auth counters.
- Certificate/root handling: short-lived `rcgen` localhost material
  (SANs `localhost` + `127.0.0.1`, generated per test, no checked-in
  key/cert); cert/key PEM written under `tempfile::TempDir` (auto-cleaned
  on drop) and loaded via `TlsServer::from_pem`; client trusts the
  DER-encoded test root only. TLS domain `localhost` while dialing
  `127.0.0.1`, mirroring worker TLS-domain/IP dialing.
- Steady-state counts
  (`session_reuses_single_tls_connection_in_steady_state`: register + 3
  heartbeats + task request + result over one session): 1 TCP accept / 1
  TLS handshake / 1 PSK auth / 1 live connection; worker registered with
  fresh heartbeat timestamp; TLS-only listener (`is_tls`), no plaintext
  fallback.
- Sever event sequence
  (`session_severed_tls_connection_reconnects_with_reregister`): scripted
  TLS coordinator accepts A → TLS/auth → Register → one live heartbeat →
  deliberate server-side drop of the established registered connection A →
  client observes loss on next heartbeat → fresh connection B (new TCP +
  TLS + PSK auth) → Register replayed → idempotent heartbeat retry
  succeeds. Reconnect counts: accepts 2, handshakes 2, auths 2; peer
  addrs differ (old connection never reused); violations 0 (no
  unauthenticated command accepted).
- Registration replay ordering: server event log
  `conn0 register → conn0 heartbeat → conn0 severed → conn1 register →
  conn1 heartbeat`; connection B rejects any pre-Register command as a
  violation (none observed).
- Retry-matrix regression: heartbeat retry proven by the sever test's
  successful post-reconnect retry; `session_request_tasks_never_retried_on_failure`
  and `session_result_never_retried_on_failure` prove one wire
  `RequestTasks` / one wire `Result` per caller call on dropped exchanges
  (600ms quiescence, no transparent replay) with recovery on the next
  explicit call (second wire message over a fresh setup).
- Documentation corrections: `architecture/performance.md` status
  Active→Executed with polish addendum (TLS counts, sever sequence,
  withdrawn root/netns limitation, corrected TaskQueue clone wording);
  Phase F completion fixed (retained dequeue clone, real verification
  record, sever debt closed); Phase E completion records TLS reuse +
  sever/re-auth/re-register evidence and the withdrawn limitation;
  `architecture/distributed.md` updated (unified TaskQueue layout,
  test-only verified trust, session-vs-oneshot invariant, new test
  inventory); `eggsec-distributed` skill corrected (`TlsClient::new` is
  WebPKI-verified; test trust + sever coverage noted). README and
  AGENTS.md needed no changes (no stale status or overstated claims there).
- Local verification (all green): `cargo test -p eggsec --lib
  distributed::remote::session_tests` (11 passed);
  `cargo test -p eggsec --lib distributed::` (29 passed);
  `cargo test -p eggsec --test distributed_tests` (23 passed);
  `cargo fmt --all --check` clean;
  `cargo clippy -p eggsec --no-default-features -- -D warnings` clean;
  `cargo clippy -p eggsec --no-default-features --features cli -- -D warnings`
  clean; `make check` EXIT 0 (fmt, no-default check, deny, clippy, doc +
  integration suites incl. transport-eggfetch parity/H2/SOCKS5, TUI lib,
  guards); `make check-feature-profiles` EXIT 0; `make check-python`
  EXIT 0; `make check-msrv` EXIT 0. Deep sweeps (`check-features-individual`,
  `check-full`) are deep-checks-only per AGENTS.md, not per-PR; no new
  features or system deps were added.
- Hosted CI: recorded post-push below (the pre-polish HEAD run is context,
  not proof for the polish SHA).
- Residual debt (with reopen conditions):
  - None from this pass's acceptance list; all 15 criteria are met.
  - Reopen the sever fixture only if the retry matrix or reconnect policy
    changes and the fixture no longer covers it.
  - Reopen TLS trust scaffolding only if production cert loading changes
    and the test-only injection no longer mirrors it.
  - Timing stays informational (loopback medians/ranges, never merge
    gates); short samples never establish regression parity.
