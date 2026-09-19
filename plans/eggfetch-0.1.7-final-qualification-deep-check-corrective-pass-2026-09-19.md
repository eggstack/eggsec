# Eggfetch 0.1.7 final qualification and deep-check corrective pass

Status: Ready for hosted verification
Date: 2026-09-19
Planning baseline: 612b543b23ba83c43382276505f6c4c6cdefaf2c
Predecessor implementation: 7f4a92ca4660c4a68e69fa5a8c5d494cda972676
Predecessor CI-record commit: 612b543b23ba83c43382276505f6c4c6cdefaf2c
Predecessor corrective plan: plans/eggfetch-0.1.7-post-adoption-qualification-corrective-pass-2026-09-19.md
Related executed TUI corrective: plans/frontend-runtime-tui-full-profile-corrective-pass.md

## Purpose

Close the residual verification and truthfulness gaps discovered after the
Eggfetch 0.1.7 post-adoption qualification pass. The transport implementation
itself is not being reopened: logical-URL plus singular authorized resolved
routing, manual per-hop redirect authorization, aggregate timeout through body
EOF, explicit proxy intent, supported pinned CONNECT/local-resolution SOCKS5,
fail-closed unsupported proxy shapes, H1/H2, no H3, no backend retries, and no
environment-derived proxy routing remain the required architecture.

The predecessor pass added useful Eggsec-local H2 and SOCKS5 fixtures and its
ordinary hosted CI is green. It also truthfully recorded that the exhaustive
feature sweep still had three Rust compile failures. Those failures mean the
repository's scheduled/manual Deep Checks contract is not actually green yet,
and the predecessor acceptance mapping is therefore too strong where it treats
local partial deep evidence as equivalent to a successful Deep Checks run.

This pass also tightens two evidence-quality issues: the H2 socket-isolation
fixture currently changes the logical URL port at the same time as the selected
socket, so it does not isolate the resolved-address component of the route key;
and the single-run loopback performance measurements are too short/noisy to
support a categorical "no throughput/latency regression" statement.

## Confirmed baseline findings

At the planning baseline:

- Main is `612b543b23ba83c43382276505f6c4c6cdefaf2c`; the transport qualification implementation is
  `7f4a92ca4660c4a68e69fa5a8c5d494cda972676`.
- Ordinary CI `35430396972` and Code Quality `35430396524` succeeded for
  the implementation SHA.
- `cargo test -p eggsec-transport-eggfetch` is recorded as 66 passing tests,
  including four local H2 tests and four local-resolution SOCKS5 tests.
- `make check-features-individual` is recorded as 82 PASS / 4 SKIP / 3 FAIL.
  The failures are `eggsec/full`, `eggsec-tui/packet-inspection`, and
  `eggsec-tui/full`.
- The common Rust failure is stale privilege-helper ownership in
  `crates/eggsec/src/packet/cli.rs`: packet paths import
  `crate::utils::is_root`, while `is_root` is now exported by
  `crate::platform`.
- `.github/workflows/deep-checks.yml` runs `make check-full` and
  `make check-features-individual` in its Deep Checks job. A source compile
  failure in those profiles is FAIL, not an environment SKIP.
- The existing TUI full-profile corrective is still registered as active.
  Reuse/reconcile that plan rather than creating a second TUI architecture
  campaign.
- `h2_mux.rs::h2_selected_socket_change_does_not_reuse_old_connection`
  changes the URL port along with the physical selected socket, so logical
  origin and selected-address isolation are confounded.
- The H2 fixture decrements its live-stream counter once in the
  `send_response` error branch and again after the match. A response-send
  failure can therefore double-decrement the `AtomicUsize` test counter.
- The recorded 0.1.7 versus 0.1.5 performance samples are very short loopback
  runs and are mixed by concurrency; they are useful smoke evidence, not a
  statistically defensible no-regression result.
- The predecessor corrective file still begins with `Status: Ready for handoff`
  even though its completion record and `plans/README.md` call it executed.

## Invariants

- Do not change `NetworkAuthority` semantics or widen scope.
- Direct traffic remains logical URL plus exactly one socket-authorized
  `SocketAddr`; the approved DNS vector is not backend failover permission.
- No origin DNS fallback.
- Host, TLS identity/SNI, redirects, and route-cache origin identity remain
  logical-URL based.
- Redirect authorization remains manual above Eggfetch and occurs before I/O
  for every hop.
- Proxy peer and ultimate target remain independent authorization decisions.
- Supported proxied routes remain singular per leg.
- SOCKS5H remote resolution and plaintext forward-proxy shapes that cannot
  enforce the selected ultimate address remain fail closed.
- Environment proxy variables remain unable to influence this backend.
- H3 and backend retries remain disabled.
- Keep the current SNI compatibility hint; SNI-hint removal is separate work.
- No new production dependency, transport pool, resolver abstraction, or crate
  is expected from this pass.
- A failing verification gate must be reported as failing. Do not convert a
  Rust compile error into SKIP or claim an equivalent green gate that did not
  run.

## Workstream 0 — freeze and reproduce

1. Record current HEAD, Rust/MSRV, Eggfetch version/features, and the exact
   predecessor CI/deep-gate evidence.
2. Re-run the three known failing profiles before editing:
   - `cargo check -p eggsec --features full`
   - `cargo check -p eggsec-tui --features packet-inspection`
   - `cargo check -p eggsec-tui --features full`
3. Provision the native build prerequisites used by Deep Checks before
   classifying results: `protobuf-compiler`, `libpcap-dev`,
   `libssl-dev`, `libssh2-1-dev`, and `pkg-config` on Ubuntu-equivalent
   hosts.
4. Distinguish source failures from missing-prerequisite skips. Preserve the
   original diagnostics in the completion record.

If main has moved, reconcile intervening commits first. Do not blindly apply
the baseline diagnosis to a changed packet/platform ownership surface.

## Workstream 1 — repair privilege-helper ownership and reconcile TUI full

### 1.1 Packet privilege helper

Replace stale packet-path imports of `crate::utils::is_root` with the canonical
platform owner, `crate::platform::is_root`. Audit all packet-module imports
rather than changing only the first compiler-reported line.

Do not re-export `is_root` from `utils` as a compatibility shim. The prior
ownership cleanup deliberately moved privilege/prerequisite detection to
`platform`; restoring a utils alias would recreate the boundary drift that
the feature sweep caught.

Add or retain compile coverage sufficient to make this ownership error visible
under both engine and TUI packet/full profiles.

### 1.2 Reconcile the active TUI corrective

After the packet import repair, re-run:

- `cargo check -p eggsec-tui --features packet-inspection`
- `cargo check -p eggsec-tui --features full`
- `make check-feature-profiles`
- `make check-features-individual`

If additional TUI source failures remain, execute the still-applicable portions
of `plans/frontend-runtime-tui-full-profile-corrective-pass.md` rather than
duplicating its request-builder/feature-sweep work here. Re-baseline that plan
against current main and distinguish already-landed portions from remaining
work.

When `eggsec-tui --features full` and the exhaustive TUI sweep are genuinely
green on a provisioned host, append a completion record to the TUI corrective
and change its registered status from active/ready-for-handoff to executed.
Do not mark it executed merely because this plan references it.

## Workstream 2 — strengthen the H2 route-key proof

### 2.1 Selected-address-only isolation

Replace or supplement
`h2_selected_socket_change_does_not_reuse_old_connection` with a fixture that
changes the physical resolved address while keeping the logical origin
unchanged: same scheme, hostname, and port.

Preferred deterministic shape:

1. Bind two H2-over-TLS loopback listeners on different loopback addresses but
   the same TCP port (for example `127.0.0.1:P` and `127.0.0.2:P`) when the
   supported CI host permits it.
2. Use one logical URL such as `https://h2.local:P/`.
3. Make the resolver/authorized selection return the first address for the
   first request and the second address for the next authorization cycle.
4. Prove each physical listener receives exactly its selected route and the
   first connection is not reused for the second selected-address snapshot.
5. Revisit the first snapshot and prove its own route may reuse its existing
   connection if still cached.

If a portable same-port two-address fixture needs a different loopback
mechanism, use it, but do not change the logical origin as part of the
selected-address isolation assertion. Keep the existing logical-origin
isolation test as a separate dimension.

The test must still exercise `EggfetchTransport`, not `eggfetch-core`
directly, and must not use system DNS.

### 2.2 Fix live-stream accounting

Restructure the H2 response task so `current` is decremented exactly once for
each increment regardless of response-send success/failure. Do not add a
production dependency for this. Prefer simple single-exit accounting or a
small test-local guard.

Expose/assert a zero live-stream count after fixture quiescence where practical
so a future accounting regression is visible. Keep the peak-concurrency proof
(`max_concurrent >= 2`) intact.

### 2.3 Preserve existing H2 evidence

Re-run all four existing H2 behaviors plus the strengthened address-only case:

- ALPN `h2`;
- concurrent multiplexing on one warmed connection;
- sequential reuse;
- logical-origin isolation;
- selected-resolved-address isolation.

Do not require a cold burst to use one TCP connection; the existing warm-route
qualification correctly avoids conflating Hyper's connection-establishment race
with H2 multiplexing.

## Workstream 3 — make performance claims match the evidence

The current single-run 10–30 ms loopback samples are not sufficient to claim
there is no throughput/latency regression at every concurrency. Do one of the
following, in priority order.

### 3.1 Preferred: collect bounded repeatable measurements

Use the same production `EggfetchTransport` harness and the reproducible
pre-adoption checkout `49cd4bf70efe5c8cb24a8199e04d28b51244f4f9`.

For concurrency 1, 10, 50, and 100:

- choose request counts large enough that each measured sample runs for at
  least roughly 1–2 seconds on the qualification host rather than a few
  milliseconds;
- perform one unreported warm-up;
- record at least five measured trials per version/profile;
- keep host, kernel, CPU/container allocation, Rust/Cargo versions, build
  profile, protocol, target, and harness identical;
- record per-trial RPS, p50/p95/p99, wall time, accepted physical connection
  count, errors, and timeouts;
- summarize median plus range or another simple dispersion measure;
- keep connection-count/error invariants separate from noisy throughput.

Do not introduce a permanent benchmark framework solely for this closure pass.

### 3.2 Acceptable fallback: narrow the claim

If repeatable measurements are not practical, retain the existing numbers as
smoke evidence but change documentation to say performance is inconclusive at
those run lengths. State only what is supported: zero observed request errors,
bounded/reused connections, and no correctness regression in the measured
transport behavior.

In either case, remove the categorical sentence that the single-run data shows
"no throughput/latency regression versus" 0.1.5 unless the repeated data
actually supports it. Do not invent a hard performance threshold after seeing
the results.

## Workstream 4 — restore truthful deep/release closure

### 4.1 Local/provisioned gates

Run the final implementation SHA through:

```text
cargo fmt --all -- --check
cargo check -p eggsec-transport
cargo test -p eggsec-transport -- --test-threads=1
cargo check -p eggsec-transport-eggfetch
cargo test -p eggsec-transport-eggfetch -- --test-threads=1
cargo test -p eggsec-transport-eggfetch --test h2_mux -- --test-threads=1
cargo test -p eggsec --lib loadtest -- --test-threads=1
cargo test -p eggsec --test network_policy_invariants -- --test-threads=1
cargo test -p eggsec --test enforced_dispatch_regression -- --test-threads=1
cargo check -p eggsec --features packet-inspection
cargo check -p eggsec --features full
cargo check -p eggsec-tui --features packet-inspection
cargo check -p eggsec-tui --features full
cargo tree -p eggsec-transport-eggfetch -e features
cargo tree -d
make check-msrv
make check
make check-deps
make check-feature-profiles
make check-full
make check-features-individual
```

On the provisioned qualification host, the target state for
`make check-features-individual` is zero Rust compile FAIL entries. A named
SKIP is acceptable only when the documented native prerequisite is genuinely
absent from that particular local host; the hosted Deep Checks environment
already provisions the principal Linux build prerequisites and must not use
those local SKIPs as closure evidence.

### 4.2 Clean release check

`make release-check` requires a clean tree. Commit the implementation/docs
candidate first, run `make release-check` on that clean implementation SHA,
and record the exact result. If the release check exposes an in-scope defect,
fix it, create a new implementation SHA, and rerun. Do not call a dirty-tree
failure a release qualification.

Run `make check-python` when touched files or release-check policy require it;
otherwise record why it was not applicable.

### 4.3 Actual hosted Deep Checks

Trigger the repository's real `Deep Checks` workflow with
`workflow_dispatch` on the final implementation SHA. Require truthful results
for at least:

- Deep Checks job (`make check-full` + exhaustive feature sweep);
- MSRV 1.89 job;
- macOS portability;
- Windows portability;
- platform-integration fixture layer, with live-probe SKIPs interpreted
  according to the workflow's documented policy.

A successful ordinary push CI run is not a substitute for this workflow.
A local feature sweep is not a substitute either.

If the workflow exposes an unrelated pre-existing failure, identify the exact
job/profile and owning plan. Do not mark the Deep Checks acceptance item green
until the workflow's required jobs have a truthful acceptable disposition.

## Workstream 5 — reconcile records and documentation

After implementation and hosted verification:

1. Change this plan to Executed and append exact final implementation SHA,
   record-only SHA if any, ordinary CI/Code Quality run IDs, Deep Checks run ID,
   release-check result, feature-sweep counts, H2 connection/stream evidence,
   and performance disposition.
2. Correct the predecessor post-adoption plan's top-level status to
   `Status: Executed` without rewriting its historical completion evidence.
3. Append a short corrective addendum to the predecessor plan explaining that
   this pass resolved the three feature failures, ran actual hosted Deep
   Checks, strengthened address-only H2 isolation, and reconciled performance
   wording/measurement.
4. Update `plans/README.md` so this pass is executed and the TUI full-profile
   corrective is no longer described as active once its own acceptance
   criteria are actually satisfied.
5. Update `architecture/transport_eggfetch.md` with the selected-address-only
   H2 route-key proof and corrected test counts.
6. Update `architecture/loadtest.md` with repeated performance statistics or
   the narrowed/inconclusive wording. Preserve raw prior measurements as
   historical evidence rather than silently replacing them.
7. If the privilege-helper repair changes a durable ownership statement,
   update the relevant architecture/agent documentation; do not add a grep
   guard for a single import path when compile coverage already owns the
   invariant.

A final documentation-only record commit may follow the verified implementation
SHA. If so, explicitly identify it as record-only and link all CI/deep evidence
to the implementation SHA it describes.

## Non-goals

- No Eggfetch version bump beyond 0.1.7.
- No Git dependency or Eggfetch fork.
- No transport authorization redesign.
- No multi-address backend failover.
- No automatic redirects.
- No environment proxy discovery.
- No H3 or backend retries.
- No SNI-hint removal.
- No custom-CA contract expansion.
- No new production SOCKS/H2 server code.
- No broad packet subsystem refactor.
- No new TUI architecture roadmap; reuse the existing corrective.
- No unrelated Reqwest/NSE migration.
- No benchmark threshold added to normal CI from noisy loopback data.
- No package publication.

## Acceptance criteria

This corrective is complete only when all of the following are true:

1. `crate::utils::is_root` is no longer used by packet code; privilege
   detection resolves through the canonical `platform` owner.
2. `cargo check -p eggsec --features full`,
   `cargo check -p eggsec-tui --features packet-inspection`, and
   `cargo check -p eggsec-tui --features full` succeed on a correctly
   provisioned supported host.
3. `make check-features-individual` has zero Rust compile FAIL entries on the
   provisioned qualification/Deep Checks host.
4. The existing TUI full-profile corrective is reconciled and marked executed
   only after its remaining acceptance criteria are actually met.
5. H2 selected-address isolation is proven with unchanged logical origin; the
   test no longer relies on a simultaneous origin-port change for that claim.
6. H2 live-stream accounting cannot double-decrement and the multiplexing,
   sequential reuse, origin-isolation, and selected-address-isolation tests are
   green through `EggfetchTransport`.
7. Existing direct pinning, Host/SNI, redirect, timeout-through-EOF, H1 reuse,
   CONNECT, SOCKS5-local, unsupported-proxy fail-closed, hostile-environment,
   and route-isolation tests remain green.
8. Performance documentation either contains repeated, sufficiently long,
   same-harness 0.1.5/0.1.7 measurements with dispersion or explicitly labels
   the existing short samples inconclusive; no unsupported no-regression claim
   remains.
9. `make check-msrv`, `make check`, `make check-deps`,
   `make check-feature-profiles`, `make check-full`, and clean-tree
   `make release-check` are green.
10. The actual hosted Deep Checks workflow is run for the final implementation
    SHA and its required jobs have a truthful acceptable disposition; ordinary
    push CI/local evidence is not substituted for it.
11. Ordinary hosted CI and Code Quality are green on the final implementation
    SHA.
12. Predecessor/corrective plan statuses and `plans/README.md` agree with the
    evidence.
13. Eggfetch remains published 0.1.7 with the intended H1/H2/Rustls/proxy
    feature graph, Rust 1.89 baseline, H3 off, retries off, and no environment
    proxy activation.
14. No security invariant is weakened and no new production dependency,
    connection pool, crate, or backend fallback path is introduced.

## Suggested handoff order

1. Freeze/reproduce the three feature failures.
2. Repair packet `is_root` ownership.
3. Re-run packet/full engine and TUI profiles.
4. Reconcile/finish the existing TUI full-profile corrective if anything
   remains.
5. Strengthen H2 selected-address isolation and fix stream accounting.
6. Run focused transport/security suites.
7. Repeat or narrow the performance evidence.
8. Run mandatory/MSRV/full/exhaustive gates on a provisioned host.
9. Commit the implementation candidate and run clean `make release-check`.
10. Trigger actual hosted Deep Checks on that implementation SHA.
11. Fix only genuine in-scope failures; rerun affected gates.
12. Reconcile architecture docs, predecessor addendum, TUI plan status, and
    this completion record.
13. Push a record-only follow-up if needed, clearly separating it from the
    verified implementation SHA.

## Exit criterion

Close this pass only when the Eggfetch 0.1.7 transport remains unchanged in its
security architecture, the repository's advertised full/TUI feature profiles
compile, the exhaustive feature sweep and actual hosted Deep Checks have been
truthfully exercised, the H2 route-key fixture independently proves physical
selected-address isolation, and performance/documentation claims are no
stronger than the measurements support.
