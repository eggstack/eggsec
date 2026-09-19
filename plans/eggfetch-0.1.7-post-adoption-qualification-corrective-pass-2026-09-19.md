# Eggfetch 0.1.7 post-adoption qualification corrective pass

Status: Ready for handoff
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
