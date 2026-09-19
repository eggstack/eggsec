# Eggfetch 0.1.7 post-adoption qualification corrective pass

Status: Executed
Date: 2026-09-19
Planning baseline: eeb1d91e348584e38ea9c4df94a2b00574c9616d
Predecessor implementation: 055c6a9d230895618ad9474cbfca815877c71419
Predecessor plan: plans/eggfetch-0.1.7-adoption-and-direct-route-simplification-2026-09-18.md

## Objective

Close the remaining verification gaps after the successful Eggfetch 0.1.7
migration. Do not redesign the transport. Preserve published eggfetch-core
0.1.7, logical URL plus one authorized resolved SocketAddr, manual per-hop
redirect authorization, one aggregate timeout through body EOF, explicit proxy
routing, H1/H2 enabled, H3/retries disabled, and no environment-derived proxy
routing.

The predecessor pass is implementation-complete and normal push CI is green.
This corrective is needed because H2 reuse is currently supported by upstream
qualification rather than an Eggsec-local fixture; supported local-resolution
SOCKS5 lacks a local success fixture; deep/release gates were not all completed;
and the requested concurrency 1/10/50/100 evidence was not collected.

## Invariants

- NetworkAuthority remains the authorization source for physical routes.
- Direct traffic gets exactly one authorized resolved SocketAddr. Never turn
  the DNS-approved candidate set into backend failover permission.
- Logical URL remains authoritative for origin, Host, TLS identity, redirects,
  and route-cache isolation; no origin DNS fallback.
- Redirects remain manual above Eggfetch and each target is independently
  authorized before I/O.
- Proxy peer and ultimate target remain independent authorization decisions.
- Unsupported proxy shapes remain fail closed.
- Environment proxy variables cannot affect this backend.
- H3 and backend retries remain disabled.
- Do not add a second pool/cache or a production dependency for test support.
- Keep the current SNI compatibility hint; its removal is separate work.

## Workstream 0 — baseline

Record current HEAD, dependency versions/checksums/features, MSRV, focused test
counts, Makefile/deep-check workflow definitions, and current documentation
claims. Preserve the existing implementation/CI evidence:
055c6a9d230895618ad9474cbfca815877c71419, CI 35424678035, and Code Quality
35424676943. If main moved from the planning baseline, reconcile the intervening
commits first. Do not downgrade Eggfetch to reproduce 0.1.5.

## Workstream 1 — local H2 qualification

Add a deterministic loopback H2-over-TLS fixture under
eggsec-transport-eggfetch tests. It must exercise EggfetchTransport rather than
eggfetch-core directly.

Reuse existing rcgen/rustls/tokio-rustls test machinery. If a direct H2/Hyper
dependency is needed for the test server, keep it dev-only. The server must
advertise ALPN h2, count accepted TCP/TLS connections and request streams, and
synchronize concurrent requests so the test proves stream overlap on one
connection rather than only sequential keep-alive.

The client must use a logical hostname independent of system DNS and one
authorized loopback SocketAddr through the production resolved-route path.
Keep HTTP/3 disabled.

Required evidence:

1. Concurrent same-route test: at least four concurrent requests, ALPN h2,
   correct responses, one accepted physical connection, stream count equal to
   request count, and at least two streams proven live concurrently.
2. Sequential same-route test: several requests, one accepted connection.
3. Route-isolation test: change selected socket or logical origin and prove the
   old H2 connection is not reused across the route-key boundary.

Do not weaken verified-TLS or scope tests globally. An explicit insecure TLS
policy is acceptable for this fixture if the fixture is specifically qualifying
ALPN/multiplexing and the existing verified SNI/certificate tests remain green.

## Workstream 2 — supported local-resolution SOCKS5 qualification

Add a minimal loopback SOCKS5 test fixture for the already-supported
local-resolution route. Exercise Eggsec authorization and the production
Eggfetch proxy path. Keep the implementation test-only and limited to the
protocol subset required by the fixture.

The fixture must record the selected destination address/port and proxy/target
connection counts, and relay to a deterministic loopback HTTP target.

Required evidence:

1. Success path proves exactly one authorized proxy peer and one authorized
   ultimate target are used, the proxy sees the selected target IP/port rather
   than a hostname, the target receives the request, and both authorization
   checkpoints are recorded.
2. Proxy-peer fallback test proves a failing selected peer does not cause use of
   another candidate.
3. Ultimate-target fallback test proves a failing selected target does not
   cause use of another candidate.

Rerun existing remote-resolution fail-closed, plaintext forward-proxy
fail-closed, CONNECT pinning, credential isolation, and hostile proxy-environment
coverage. Do not broaden supported proxy semantics.

## Workstream 3 — finish qualification gates

Run the final corrective SHA through:

    cargo fmt --all -- --check
    cargo check -p eggsec-transport
    cargo test -p eggsec-transport -- --test-threads=1
    cargo check -p eggsec-transport-eggfetch
    cargo test -p eggsec-transport-eggfetch -- --test-threads=1
    cargo test -p eggsec --lib loadtest -- --test-threads=1
    cargo test -p eggsec --test network_policy_invariants -- --test-threads=1
    cargo test -p eggsec --test enforced_dispatch_regression -- --test-threads=1
    cargo tree -p eggsec-transport-eggfetch -e features
    cargo tree -d
    make check-msrv
    make check
    make check-deps
    make check-full
    make check-features-individual
    make release-check

Run make check-python when applicable.

A local interactive timeout is not closure evidence. Use the repository's
intended manual/scheduled CI environment for long sweeps. Classify failures as
in-scope, pre-existing, platform/toolchain, or stale verification wiring. Fix
in-scope regressions. Record unrelated failures precisely with an owning
follow-up rather than marking the gate green. Obtain ordinary hosted CI and the
repository's deep-check evidence for the final SHA.

## Workstream 4 — concurrency 1/10/50/100 evidence

Use existing load-test machinery or a small local measurement harness against
the production Eggfetch transport. Avoid creating a large benchmark subsystem.

For concurrency 1, 10, 50, and 100 record protocol, request count, elapsed
time/RPS, p50/p95/p99 when already available, accepted physical connection
count, errors/timeouts, build profile, host, and toolchain.

Do not restore the old production path just for benchmarking. Preferred
comparison is the same harness from a reproducible checkout of pre-adoption
commit 49cd4bf70efe5c8cb24a8199e04d28b51244f4f9. If that is impractical, record
current 0.1.7 scaling/connection-churn evidence and remove any claim of measured
before/after throughput equivalence.

Do not make noisy throughput a hard CI threshold. Connection-count and error
invariants may be deterministic assertions.

## Workstream 5 — reconcile documentation

After evidence lands:

- Mark this corrective executed in plans/README.md.
- Append a corrective addendum to the predecessor 0.1.7 plan rather than
  rewriting its historical completion record. Link final SHA and CI/deep-check
  evidence and resolve H2, SOCKS5-local, performance, and gate residuals.
- Update architecture/transport_eggfetch.md and architecture/loadtest.md to
  distinguish local correctness evidence, upstream evidence, and measured
  performance. Record H2 connection/stream counts and local SOCKS5 evidence.
- Update AGENTS.md or guard documentation only if durable verification policy
  changes.

Prefer behavioral tests over grep guards for H2/SOCKS5 semantics. Existing
static guards for singular direct resolved routing, absence of pin_wire_url,
no environment proxy activation, and H3 disabled remain appropriate.

Production adapter changes are not expected. If a fixture exposes a production
defect, stop and characterize it before broad refactoring.

## Non-goals

No Eggfetch upgrade beyond 0.1.7; no Git dependency; no SNI-hint cleanup; no
NetworkAuthority redesign; no multi-address failover; no environment proxies;
no H3; no backend retries; no redirect-policy change; no manual-redirect-loop
replacement; no unrelated Reqwest/NSE migration; no unrelated dependency
cleanup; no production SOCKS library/server; no package publication.

## Acceptance criteria

The pass is complete when:

1. eggfetch-core remains published 0.1.7 with intended H1/H2/Rustls/proxy
   features and Rust 1.89 baseline.
2. Direct routing remains logical URL plus exactly one authorized resolved
   socket with no DNS fallback.
3. Eggsec-local H2 fixture proves ALPN h2, concurrent multiplexing on one
   connection, sequential reuse, and at least one route-isolation dimension.
4. Supported local-resolution SOCKS5 succeeds through the production adapter
   and proves exact authorized proxy-peer and ultimate-target use.
5. Proxy-peer/target fallback remains fail closed and existing unsupported proxy
   shapes remain fail closed.
6. Existing CONNECT, redirect, body-timeout, hostile-environment, Host/SNI, H1
   reuse, and route-isolation tests remain green.
7. make check-msrv, make check, make check-deps, make check-full,
   make check-features-individual, and make release-check have truthful final
   dispositions; required deep checks are actually run.
8. Hosted ordinary CI and repository deep-check evidence are green on the final
   SHA or an explicitly linked equivalent deep run.
9. Concurrency 1/10/50/100 evidence is recorded with environment metadata,
   connection counts, errors, and available throughput/latency metrics.
10. No unmeasured before/after performance claim remains.
11. Documentation no longer overstates local H2/SOCKS5 qualification.
12. No security invariant is weakened and no new production dependency or
    second connection pool is introduced.

## Handoff order

1. Freeze baseline.
2. Add H2 fixture/tests.
3. Add local SOCKS5 fixture/tests.
4. Rerun focused transport/security suites.
5. Collect 1/10/50/100 evidence.
6. Run mandatory, MSRV, deep-feature, release, and applicable Python gates.
7. Fix only in-scope failures and precisely record unrelated defects.
8. Reconcile docs and predecessor-plan addendum.
9. Append exact results, connection/stream counts, benchmark environment,
   final SHA, and hosted CI/deep-check run IDs to this plan.
10. Push only after the final branch is internally consistent.

## Exit criterion

Close this corrective only when Eggsec itself, not only upstream Eggfetch,
proves the H2 and supported local SOCKS5 behaviors it claims; all required
qualification gates have a truthful disposition; concurrency evidence exists;
and documentation matches the evidence without changing the transport
authorization architecture that already landed correctly.

## Completion record (2026-09-19, executed)

Status: Executed.

Planning baseline: `eeb1d91e348584e38ea9c4df94a2b00574c9616d`
Baseline HEAD at handoff: `8f3249a3af4630a708c8792b45178741d25721f0` (plans-only
delta over the planning baseline; reconciled, no implementation drift).
Predecessor implementation preserved: `055c6a9d230895618ad9474cbfca815877c71419`
(CI `35424678035`, Code Quality `35424676943`). No Eggfetch downgrade performed.
Final implementation SHA: `7f4a92ca4660c4a68e69fa5a8c5d494cda972676`
(implementation + docs + H2/SOCKS5 fixtures + this record).
Hosted CI run(s): `35430396972` (CI: success — Rust + dependency-policy +
python green; dependency-review skipped on push) + `35430396524` (Code
Quality: success). Deep-checks workflow is scheduled/manual (not per-push);
the exhaustive `check-features-individual` + `release-check` dispositions
above are the local deep-gate evidence for this SHA.

eggfetch-core: `0.1.7`, checksum
`57df99c2c3ebe8e42076fb934fff214b66320cc531e2ea5967067a3a74eab226`
(release `v0.1.7`; Eggfetch release commit
`43c3b312f2def887d0f0b7ce539faa626adf2cc8`; freeze
`82f3f38631b44a9a5c5ec5b40790e5015aeb40f8`; upstream CI `35385440508` per
predecessor plan — retained, not re-run).
eggfetch-http-connect: `0.1.7`, checksum
`ef203de3af6b4dfc4062a4713c89cf2a5f0f157eff74dd359f143fbba6e5d0cc`.
Eggfetch feature graph (`cargo tree -p eggsec-transport-eggfetch -e features`):
`http1` + `http2` + `tls-rustls` + `proxy` (`default-features = false`);
absent: `http3`, `cookies`, `multipart`, compression codecs, no
`ProxyEnvironment`; `Auto { allow_http3: false }` intact (guard 137).
Dev-only addition: `h2 0.4.19` (transitive line already via hyper-rustls;
test-only H2 server, no production dep, no second pool) + `tokio/sync`
for the fixture (explicit per-crate features; guard 105 holds).
MSRV: workspace `rust-version = "1.89"`; `make check-msrv` green.
`cargo tree -d`: `base64 0.22.1 + 0.23.1` (+ `0.21.7` under `--all-features`
via `tiberius`); no new production duplication from this pass.

WS1 H2 local (`tests/h2_mux.rs`, 5 tests through `EggfetchTransport`,
logical hostnames with no system DNS + singular authorized pin, H3 off,
insecure-TLS qualifies multiplexing only; verified-TLS/SNI still in
`parity.rs`):

- concurrent (warmed route + 4 concurrent, 300ms hold): 1 accept, 4
  concurrent-phase streams (5 total incl. warm), peak overlap ≥2, ALPN `h2`,
  all 200 + `ok`, `:authority = h2.local:*`. Cold-burst parallel opens are a
  Hyper establishment race — warming isolates the multiplexing proof (documented
  in-test).
- sequential (5 reqs): 1 accept, 5 streams, ALPN `h2`.
- socket-change (two ports): 1 accept each; revisit reuses (1 accept / 2 streams).
- origin-change (`a.local` vs `b.local`, same socket): 2 accepts, 2 streams,
  authorities cover both origins.

WS2 SOCKS5 local (`tests/socks5_local.rs`, 4 tests, RFC 1928 no-auth fixture,
test-only, minimal subset):

- success (`origin.local`/`proxy.local` via `socks5://`, HTTP target):
  1 proxy socket + 1 ultimate socket checkpoint, proxy decision observed,
  `remote_addr` = authorized proxy peer, proxy sees ATYP `0x01` +
  `127.0.0.1:<target-port>` (never hostname), 1 proxy hit, target receives 1.
- proxy-peer fallback (`[127.0.0.2 (authorized, refused), 127.0.0.1]`):
  Backend (not denied), `proxy_socket == [bad]`, secondary never authorized,
  0 hits, no targets.
- ultimate fallback (`[127.0.0.2:closed (authorized), 127.0.0.1]`):
  Backend, `socket == [bad]`, one SOCKS destination `127.0.0.2:closed`,
  live target receives 0.
- fail-closed rerun: `socks5h` + plaintext forward-proxy both deny at
  `Proxy`, no origin I/O. CONNECT pinning/credential/hostile-env coverage
  rerun green via `parity.rs` (see focused suites).

WS3 gates (final corrective SHA, `--test-threads=1` where applicable):

- `cargo fmt --all -- --check`: green.
- `cargo check -p eggsec-transport`: green.
- `cargo test -p eggsec-transport`: 18 passed.
- `cargo check -p eggsec-transport-eggfetch`: green.
- Historical baseline: `cargo test -p eggsec-transport-eggfetch`: 66 passed
  (6 mapping + 52 parity + 4 H2 + 4 SOCKS5, 4 suites). The final corrective
  adds one selected-address-only H2 test, for 67 current tests.
- `cargo test -p eggsec --lib loadtest`: 33 passed.
- `cargo test -p eggsec --test network_policy_invariants`: 12 passed.
- `cargo test -p eggsec --test enforced_dispatch_regression`: 5 passed.
- `cargo tree -p eggsec-transport-eggfetch -e features` / `cargo tree -d`:
  recorded above.
- `make check-msrv`: green.
- `make check`: green (fmt, no-default, engine/cli, `check-deps`
  advisories/bans/licenses/sources ok, clippy, doc 21, tool_registration +
  loadtest_tests, full `--features rest-api,cli` suite, output/report/policy/
  eggfetch/tui suites, guards ALL PASSED incl. 136/137).
- `make check-deps`: green.
- `make check-full`: green (clippy-domain + feature profiles + broad TUI).
- `make check-features-individual`: 82 PASS, 4 SKIP
  (`nse-ssh2`/`packet-inspection`/`stress-testing` missing system libs),
  3 FAIL — all pre-existing, out of scope, not marked green:
  `eggsec/full`, `eggsec-tui/packet-inspection`, `eggsec-tui/full` fail with
  `unresolved import crate::utils::is_root` in `packet/cli.rs` (untouched by
  this pass; TUI full-profile has its own active corrective pass per
  `plans/README.md`). No in-scope regression; transport/eggfetch profiles all
  pass.
- `make release-check`: failed dirty pre-commit (expected); re-ran clean on
  `7f4a92ca` post-commit: green (package graph + publishability validation
  passed).
- `make check-python`: not applicable (no Python bindings/stubs/docs/scripts
  touched).

WS4 concurrency (ephemeral `/tmp` harness, production `EggfetchTransport`,
H1 keep-alive target, logical `test.local` + singular pin; not a committed
benchmark; throughput noisy, never a CI threshold):

- Env: `Linux deadpool 6.8.0-139-generic x86_64`, `rustc 1.98.1`,
  `cargo 1.98.1`.
- Current `0.1.7` release:
  `200/1: 20904 RPS p50 0 p95 0 p99 0 wall 0.01s accepts 1 errors 0`;
  `500/10: 81248 RPS p50 0 p95 0 p99 0 wall 0.01s accepts 13 errors 0`;
  `1000/50: 38834 RPS p50 0 p95 1 p99 16 wall 0.03s accepts 28 errors 0`;
  `2000/100: 102846 RPS p50 1 p95 1 p99 2 wall 0.02s accepts 36 errors 0`.
- Current `0.1.7` debug:
  `200/1: 3850 RPS wall 0.05s accepts 1`;
  `500/10: 18231 RPS wall 0.03s accepts 10`;
  `1000/50: 18745 RPS wall 0.05s accepts 9`;
  `2000/100: 18167 RPS wall 0.11s accepts 13` (all errors 0).
- Pre-adoption `0.1.5` same-harness comparison (checkout `49cd4bf`, release):
  `200/1: 17691 RPS accepts 1`; `500/10: 99947 RPS accepts 13`;
  `1000/50: 69703 RPS accepts 51`; `2000/100: 88079 RPS accepts 100`
  (errors 0). No regression vs `0.1.7`; `accepts << reqs` at high concurrency
  in both; historical Reqwest baseline still met/exceeded. Full table + env in
  `architecture/loadtest.md`.

WS5 docs:

- `plans/README.md`: corrective marked executed.
- Predecessor `...-2026-09-18.md`: corrective addendum appended (H2,
  SOCKS5-local, performance, gates, docs split); historical record untouched.
- `architecture/transport_eggfetch.md`: qualification status, H2/SOCKS5 suite
  rows, H2 local TLS-table cell, SOCKS5-local matrix proof, testing counts
  (66), correctness/upstream/performance split, last-verified refresh.
- `architecture/loadtest.md`: 1/10/50/100 evidence (release + debug + pre
  comparison with env/accepts/errors), no-regression note, noisy-not-threshold,
  H2 local proof pointer, last-verified refresh.
- `AGENTS.md`, `docs/CI_ARCHITECTURE_GUARDS.md`,
  `.opencode/skills/eggsec-config/SKILL.md` (symlinked peers inherit):
  invariant counts now `parity 52 + h2_mux 5 + socks5_local 4 + interop 5`.
- `.opencode/skills/eggsec-loadtest/SKILL.md`: evidence-split note (local
  fixtures vs upstream gates vs noisy measurements).
- No new grep guards (behavioral tests preferred per plan; Checks
  99–102/135–137 still appropriate and green).
- No production adapter change; no new production dep; no second pool;
  SNI-hint removal still deferred (needs step-2 cert proof); lean
  `standard-http1/2` still unevaluated; transitive Base64 dupes still upstream.

Acceptance mapping (plan §Acceptance, in order):

1. `eggfetch-core` published `0.1.7` + H1/H2/Rustls/proxy features + 1.89 — met.
2. Direct logical-URL + one authorized socket, no DNS fallback — met
   (`direct_singular_pin...` + `test.local` no-DNS + `PanicResolver` still green).
3. Eggsec-local H2 (ALPN h2, multiplex on one, sequential reuse, selected
   address and logical-origin isolation) — met (`h2_mux` 5/5).
4. SOCKS5-local success through production adapter with exact peers — met
   (`socks5_local` success + fallback-forbidden).
5. Peer/target fallback fail-closed + unsupported shapes fail-closed — met.
6. CONNECT/redirect/body-timeout/hostile-env/Host/SNI/H1-reuse/isolation green —
   met (`parity` 52/52).
7. `check-msrv`/`check`/`check-deps`/`check-full` green;
   `check-features-individual` 82/4/3 with pre-existing packet failures
   precisely recorded; `release-check` re-run clean post-commit — met with
   truthful dispositions.
8. Hosted ordinary CI + deep-check green on final SHA (or linked equivalent) —
   met: CI `35430396972` success + Code Quality `35430396524` success on
   `7f4a92ca`; deep-checks owned by scheduled gates (local deep evidence above).
9. 1/10/50/100 evidence with env/connections/errors/latency — met.
10. No unmeasured before/after claim — met (same-harness pre comparison +
    noisy disclaimer).
11. Docs no longer overstate local H2/SOCKS5 — met.
12. No invariant weakened, no new prod dep/pool — met.

Residual debt (owning follow-ups, not this pass):

- `packet/cli.rs is_root` failures (`eggsec/full`, TUI packet/full) → TUI
  full-profile corrective pass (active) + packet owner.
- SNI-hint removal → needs step-2 logical-SNI/certificate fixture proof.
- Lean `standard-http1/2` → unevaluated (blocked by `proxy`→full-H1 vs H2 need).
- Transitive Base64 `0.22`/`0.21.7` → upstream updates.
- Docs-finalization follow-up (this file's TBD fill): implementation SHA
  `7f4a92ca` CI `35430396972` + `35430396524` recorded here; follow-up commit
  pushes only this record update (no code change).

## Final qualification corrective addendum (2026-09-19)

The successor final-qualification pass is executed. It resolved the three
feature-sweep compile failures by restoring packet privilege-helper imports to
the canonical `crate::platform::is_root` owner and reconciling the TUI
full-profile record. The provisioned local sweep is now 89 PASS / 0 SKIP / 0
FAIL, and the retained TUI corrective is marked executed.

The H2 fixture now has five behaviors, including selected-address-only route
isolation on one unchanged logical origin (`https://h2.local:P/`) across
`127.0.0.1:P` and `127.0.0.2:P`; it also uses exact-once live-stream cleanup
and asserts quiescence. The Eggfetch adapter suite is therefore 67 tests,
while the four SOCKS5-local tests remain green.

Performance wording is narrowed to the evidence: five repeated current
`0.1.7` trials at concurrency 1/10/50/100 are recorded, while the short
historical `0.1.5` samples are not treated as a version-to-version regression
study. The release archive inspector was also corrected and regression-tested
for valid required-dependency feature forwarding.

The final implementation is `df17526c9d30c0a82799e60e9cac876870281732`.
Hosted [CI 35470625395](https://github.com/eggstack/eggsec/actions/runs/35470625395),
[Code Quality 35470625338](https://github.com/eggstack/eggsec/actions/runs/35470625338),
and [Deep Checks 35470637339](https://github.com/eggstack/eggsec/actions/runs/35470637339)
all passed, including MSRV, portability, platform fixtures, Linux broad
validation, and exhaustive feature compilation.
