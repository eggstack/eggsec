# Implementation plan retention

The repository intentionally retains implementation and handoff plans under
`plans/`. They are part of the engineering record and may be referenced by
architecture reviews, release validation, or later corrective work.

The architecture guard therefore checks that this policy is documented and
that the directory still contains plan files. It does not require historical
plan filenames from an older branch and it does not treat Markdown plans as
generated artifacts. Generated reports, build output, and temporary evidence
belong outside this directory or in ignored paths.

When a plan is completed, preserve it and record the outcome in the plan or in
the associated release/validation document. Do not delete useful handoff
history solely to satisfy a static guard.

## TUI confirmation-test intent corrective pass (ready for handoff 2026-09-22)

Plan:
[tui-confirmation-test-intent-corrective-pass-2026-09-22.md](tui-confirmation-test-intent-corrective-pass-2026-09-22.md)

This narrow follow-up corrects the remaining test-contract mismatch from the
completed warning-debt cleanup. The out-of-scope confirmation test now compares
`preflight()` with raw `EnforcementContext::evaluate()`, but its name/comment/
locals/assertion still describe the second branch as "execution."

The pass preserves the targeted `RequireConfirmation` case, renames/reframes
it as preflight-vs-raw-evaluation coverage, strengthens both sides to assert the
expected confirmation classification independently, and does not reintroduce a
side-effecting `try_approve` call merely to preserve the old wording. No
production enforcement/API/feature behavior is intended to change.

## Post-adoption record and TUI warning cleanup (executed 2026-09-22)

Ordered plans (both executed; each carries its completion record):

1. [eggfetch-0.2.0-planning-record-duplication-cleanup-2026-09-22.md](eggfetch-0.2.0-planning-record-duplication-cleanup-2026-09-22.md)
2. [tui-broad-profile-warning-debt-cleanup-2026-09-22.md](tui-broad-profile-warning-debt-cleanup-2026-09-22.md)

The first pass is documentation-only: remove the duplicated `## Exit criterion`
from the executed Eggfetch 0.2.0 adoption record without altering its
qualification evidence or conclusions.

Executed: duplicate block removed in `b1faff04` (9 deletions, single exit
criterion retained, completion record byte-identical).

The second pass closes the warning debt recorded during the same qualification.
It targets the 18 warnings emitted by the representative broad
`eggsec-tui --features db-pentest,web-proxy,c2` lib-test profile plus the two
adjacent `mobile/mod.rs` feature-profile import warnings. The pass requires
real cleanup rather than `allow`/underscore suppression, preserves TUI
rendering/runtime/enforcement behavior and mobile feature semantics, and does
not expand into unrelated NSE or workspace-wide warning debt.

Executed: broad TUI `--tests` profile now shows 0 owned warnings (only
third-party sqlx future-incompat remains), mobile/mobile-dynamic profiles show
0 warnings, with no suppression attributes added. Implementation `1acb0c65`;
record in the pass plan. An additional `truncate_str` unused import observed
locally in the same owned surface was also cleaned (total 19 owned warnings
observed at implementation start, all removed).

## Eggfetch 0.2.0 adoption and requalification (executed 2026-09-22)

Plan:
[eggfetch-0.2.0-adoption-and-requalification-2026-09-22.md](eggfetch-0.2.0-adoption-and-requalification-2026-09-22.md)

This bounded adoption moves the already-qualified scoped HTTP backend from the
published `eggfetch-core 0.1.7` line to the newly published `0.2.0` line.
It is intentionally a dependency/requalification pass rather than a transport
redesign: logical-URL + singular authorized resolved routing, manual per-hop
redirect authorization, aggregate body-through-EOF deadlines, explicit pinned
proxy routing, fail-closed unsupported proxy shapes, H1/H2, no H3/retries,
no environment-derived proxy routing, and decompression-off response semantics
remain mandatory.

The pass explicitly updates the pre-1.0 Cargo requirement and lockfile, advances
the exact-version architecture guard, compares the feature/dependency graph,
and reruns the existing direct/H2/CONNECT/SOCKS5/redirect/timeout/security
qualification. Upstream issue #24 is fixed in 0.2.0, but Eggsec must not use
that as a reason to enable automatic decompression or compression features.
Historical 0.1.7 plans remain unchanged.

Executed: manifest/lockfile moved to published `eggfetch-core 0.2.0` /
`eggfetch-http-connect 0.2.0` with no adapter semantic change; guard
Check 134 advanced to the 0.2.0 line; current-state docs name 0.2.0;
completion record appended to the plan.

## Performance and resource-efficiency optimization campaign (executed 2026-09-21)

Roadmap:
[performance-resource-efficiency-roadmap-2026-09-21.md](performance-resource-efficiency-roadmap-2026-09-21.md)

Ordered implementation plans:

1. [performance-phase-a-baseline-and-measurement-harness.md](performance-phase-a-baseline-and-measurement-harness.md)
2. [performance-phase-b-bounded-async-fanout-and-fuzzer-locking.md](performance-phase-b-bounded-async-fanout-and-fuzzer-locking.md)
3. [performance-phase-c-loadtest-request-hotpath.md](performance-phase-c-loadtest-request-hotpath.md)
4. [performance-phase-d-distributed-worker-capacity.md](performance-phase-d-distributed-worker-capacity.md)
5. [performance-phase-e-coordinator-session-reuse.md](performance-phase-e-coordinator-session-reuse.md)
6. [performance-phase-f-secondary-hotspots-and-closure.md](performance-phase-f-secondary-hotspots-and-closure.md)

This bounded campaign targets measured runtime performance/resource hot paths
without reopening the completed transport, crate-boundary, or frontend/runtime
architecture work. Phase A establishes reproducible local-only evidence. Phase
B bounds high-cardinality async fan-out and narrows the fuzzer timing lock.
Phase C removes repeated load-test request materialization and evaluates
body-drain allocation only behind a semantic/measurement gate. Phase D makes
WorkerConfig.max_concurrency an actual capacity contract. Phase E reuses the
coordinator's existing multi-command authenticated connection lifecycle. Phase
F re-measures, applies only evidence-supported secondary cleanup, and performs
compatibility/closure qualification.

The campaign must preserve public Rust/Python/CLI surfaces, distributed wire
formats unless an explicitly additive compatibility-safe change is separately
justified, transport authorization checkpoints, singular physical-route
pinning, redirect/proxy/TLS semantics, no automatic retries, HTTP/3-off policy,
and load-test body-through-EOF timing.

### Performance campaign closure-polish corrective follow-up (executed 2026-09-21)

Corrective pass:
[`performance-closure-polish-corrective-pass-2026-09-21.md`](performance-closure-polish-corrective-pass-2026-09-21.md)

This narrow post-closure pass reconciles the remaining evidence defects without
reopening the completed A-F optimization campaign. It corrects the stale
performance-document status and overstated TaskQueue clone claim, replaces the
plaintext-only inference for coordinator-session reuse with verified local TLS
coverage, and adds a deterministic established-connection sever/reconnect/
re-authentication/re-registration fixture that does not require root or netns.
Production APIs, wire DTOs, TLS trust behavior, worker capacity, retry policy,
and performance architecture remain unchanged.

Executed: implementation `71336632` (verified TLS steady-state, TLS
sever/reconnect, no-retry regressions, doc corrections) + docs/record
`855d03cd`; completion record in the pass plan. Hosted CI for `855d03cd`:
CI `35637506642` success, Code Quality `35637504761` success (see the pass
plan; the recording follow-up's own runs are verified before closure).

## TUI terminal ownership corrective campaign (executed 2026-09-20)

Roadmap:
[`tui-terminal-ownership-corrective-roadmap-2026-09-19.md`](tui-terminal-ownership-corrective-roadmap-2026-09-19.md)

Ordered implementation plans (both executed; each carries its completion record):

1. [`tui-terminal-ownership-phase-a-single-writer-logging-boundary.md`](tui-terminal-ownership-phase-a-single-writer-logging-boundary.md)
2. [`tui-terminal-ownership-phase-b-lifecycle-process-output-closure.md`](tui-terminal-ownership-phase-b-lifecycle-process-output-closure.md)

This bounded corrective campaign addresses TUI rendering corruption caused by
multiple writers touching the controlling terminal while Ratatui owns the
alternate screen. Phase A establishes an explicit no-console logging policy for
rich TUI execution, removes direct TUI terminal writes, routes recoverable
messages through UI state, and pins the single-writer invariant. Phase B makes
terminal teardown cleanup-safe across errors/panics, propagates fatal errors
only after restoration, audits all TUI-reachable subprocess output, and adds a
PTY smoke test plus closure guards.

The campaign is closed. Phase A implementation landed at
`5698501457bc9f05363ffa39fb38416f35c6a1a2`; Phase B landed at
`71bef7acfa92f9c2a9ff2bb1569b024c4ac95b8b`, with the final Phase B
completion record at `c0ae180d64f7cb7d6bf91e09526997f4b534a857`. Closure
evidence includes the mandatory Rust checks, architecture guards 138/139, the
Unix/Linux PTY smoke, broad/full TUI feature checks, and MSRV verification.

The campaign did not redesign TUI layout, remove tracing diagnostics, create
default persistent TUI log retention, or reopen scope/dispatch/runtime
semantics.

## Eggfetch 0.1.7 final qualification/deep-check corrective pass (executed 2026-09-19)

Corrective pass:
[eggfetch-0.1.7-final-qualification-deep-check-corrective-pass-2026-09-19.md](eggfetch-0.1.7-final-qualification-deep-check-corrective-pass-2026-09-19.md)

This final bounded closure pass addresses the residual issues found after the
0.1.7 post-adoption qualification: the three real Rust compile failures in the
exhaustive feature sweep (including stale packet `is_root` ownership), the stale
TUI full-profile record, lack of an actual hosted Deep Checks
run for the qualified implementation, a confounded H2 selected-socket isolation
fixture, H2 test-counter cleanup, and overly strong conclusions from very short
single-run loopback performance samples.

It does not reopen the Eggfetch transport architecture. The pass preserves
logical-URL + singular authorized resolved routing, manual per-hop redirect
authorization, aggregate body-through-EOF deadlines, explicit pinned proxy
routing, fail-closed unsupported proxy shapes, H1/H2, no H3/retries, and no
environment-derived proxy routing.

## Eggfetch 0.1.7 post-adoption qualification corrective pass (executed 2026-09-19)

Corrective pass:
[eggfetch-0.1.7-post-adoption-qualification-corrective-pass-2026-09-19.md](eggfetch-0.1.7-post-adoption-qualification-corrective-pass-2026-09-19.md)

This bounded follow-up closed the remaining evidence gaps after the successful
0.1.7 migration: Eggsec-local H2 ALPN/multiplex/reuse proof (`h2_mux`, 5
tests), supported local-resolution SOCKS5 success and singular two-leg
pinning proof (`socks5_local`, 4 tests), completion of the deep
feature/release gates, and bounded concurrency 1/10/50/100 measurement with
same-harness pre-adoption comparison. Documentation now distinguishes local
correctness, upstream qualification, and measured performance (see the plan's
completion record + `architecture/transport_eggfetch.md` +
`architecture/loadtest.md`).

The pass did not reopen the transport architecture. Logical-URL + singular
authorized resolved routing, manual per-hop redirect authorization, explicit
proxy intent, aggregate body-through-EOF deadlines, no environment proxies,
fail-closed unsupported proxy shapes, H3 off, and backend retries off remain
mandatory invariants.

## Eggfetch 0.1.7 adoption and direct-route simplification (executed 2026-09-19)

Plan:
[`eggfetch-0.1.7-adoption-and-direct-route-simplification-2026-09-18.md`](eggfetch-0.1.7-adoption-and-direct-route-simplification-2026-09-18.md)

This bounded follow-up consumes the published `eggfetch-core 0.1.7` release.
It raises the current 0.1.5 lock, replaces the production direct adapter's
IP-literal wire-URL/SNI shim with logical-URL + singular
`resolved_addresses()` routing, qualifies the new bounded H1/H2 resolved-route
reuse, and adopts the corrected total deadline through response-body EOF while
preserving Eggsec's manual per-hop authorization loop.

The pass explicitly does **not** reopen completed scope/proxy policy: singular
socket-authorized pins remain mandatory, environment proxy discovery remains
disabled, SOCKS5H/plain forward-proxy strict cases remain fail closed, H3 and
automatic retries remain disabled, and the new Eggfetch HTTPS-downgrade policy
does not change Eggsec semantics without a separate neutral-contract decision.

## Load-test authorization and transport corrective pass (executed)

Corrective pass:
[`loadtest-authorization-transport-corrective-pass-2026-09-16.md`](loadtest-authorization-transport-corrective-pass-2026-09-16.md)

This bounded post-crate-boundary pass closes the security debt exposed by the
Phase D load-test decoupling. It removes the wildcard scope fallback, carries
the approval scope snapshot through strict/canonical/tool execution, makes the
temporary Reqwest backend fail closed, adds physical proxy-peer checkpoints to
the shared transport contract, and makes the existing Eggfetch adapter the
production direct load-test backend after H1/H2/performance qualification.

Full proxy migration is release-gated rather than reimplemented locally:
Eggfetch `main` has completed and qualified pinned proxy-peer plus pinned
CONNECT/SOCKS5 destination routing, but those APIs are newer than the latest
published `eggfetch-core v0.1.4`. The pass must consume a published qualifying
release or leave unsupported proxied load-test routes fail-closed; it must not
add a floating sibling Git dependency. The same pass reconciles Eggsec's
workspace MSRV with Eggfetch's Rust 1.89 requirement and closes the remaining
Python load-test result parity/documentation guards.

## Crate-boundary consolidation (executed 2026-09-16)

Roadmap:
[`crate-boundary-consolidation-roadmap-2026-09-16.md`](crate-boundary-consolidation-roadmap-2026-09-16.md)

This is a bounded ownership pass over the already substantially decomposed
workspace: it fixes `eggsec-output` report-DTO/formatter ownership, separates
policy/enforcement semantics from configuration loading, shrinks the `utils`
catch-all, decouples load testing behind the scoped transport seam, and
promotes code to a new crate only on measured dependency payoff.

Ordered implementation plans (all executed; each plan carries its completion
record):

1. [`crate-boundary-consolidation-phase-a-ownership-and-primitive-cleanup.md`](crate-boundary-consolidation-phase-a-ownership-and-primitive-cleanup.md)
2. [`crate-boundary-consolidation-phase-b-report-model-extraction.md`](crate-boundary-consolidation-phase-b-report-model-extraction.md)
3. [`crate-boundary-consolidation-phase-c-policy-enforcement-extraction.md`](crate-boundary-consolidation-phase-c-policy-enforcement-extraction.md)
4. [`crate-boundary-consolidation-phase-d-loadtest-resilience-reuse-closure.md`](crate-boundary-consolidation-phase-d-loadtest-resilience-reuse-closure.md)

Phase A creates no crate (scheduling/session leave `eggsec-output`,
rate-limit/circuit-breaker semantics hardened, Reqwest pool removed). Phase B
extracts `eggsec-report-model` (four domain crates drop `eggsec-output`).
Phase C extracts `eggsec-policy` (deterministic kernel + engine bridge).
Phase D decouples load testing internally (transport-neutral core, scoped
Reqwest backend, structured progress) and rejects both `eggsec-loadtest`
(single consumer) and `eggsec-resilience` (no second consumer), closing the
roadmap with no new members in this phase.

## Network dependency and supply-chain hardening (executed 2026-09-13)

Roadmap:
[`network-dependency-hardening-roadmap-2026-09-11.md`](network-dependency-hardening-roadmap-2026-09-11.md)

This is a post-convergence dependency/capability pass. It does not reopen the
completed A-J dependency roadmap or frontend/runtime convergence. It targets
residual duplicate outbound HTTP/TLS ownership, establishes a scope-aware
transport seam, evaluates `eggfetch` as the shared HTTP substrate and narrow
`eggress` reuse, narrows process/runtime capability inheritance, and hardens
Cargo/GitHub Actions supply-chain policy.

Ordered implementation plans:

1. [`network-dependency-phase-a-baseline-invariants.md`](network-dependency-phase-a-baseline-invariants.md)
2. [`network-dependency-phase-b-scoped-transport-contract.md`](network-dependency-phase-b-scoped-transport-contract.md)
3. [`network-dependency-phase-c-eggfetch-readiness-adapter.md`](network-dependency-phase-c-eggfetch-readiness-adapter.md)
4. [`network-dependency-phase-d-outbound-client-migration.md`](network-dependency-phase-d-outbound-client-migration.md)
5. [`network-dependency-phase-e-egress-and-capability-segregation.md`](network-dependency-phase-e-egress-and-capability-segregation.md)
6. [`network-dependency-phase-f-supply-chain-ci-hardening.md`](network-dependency-phase-f-supply-chain-ci-hardening.md)
7. [`network-dependency-phase-g-closure-measurement.md`](network-dependency-phase-g-closure-measurement.md)

Phases A-C establish the baseline, mandatory scoped transport contract, and
Eggfetch resolver/redirect hooks before production consumers migrate. Phase D
moves ordinary outbound clients while retaining specialized interception and
protocol ownership. Phase E makes measured Egress adopt/reject decisions and
narrows CLI/Tokio/crate capability boundaries. Phase F hardens dependency and
CI supply-chain policy. Phase G is the final measurement/security closure pass.

All seven phases are executed (each plan carries its completion record).
Retained closure report:
[`../architecture/network_dependency_closure.md`](../architecture/network_dependency_closure.md)
(final graphs, fixture results, remaining-owner dispositions, debt, acceptance
mapping); measurement addendum is §10 of
[`../architecture/network_dependency_baseline.md`](../architecture/network_dependency_baseline.md).

## TUI full-profile corrective closure (executed 2026-09-11)

Corrective pass:
[`frontend-runtime-tui-full-profile-corrective-pass.md`](frontend-runtime-tui-full-profile-corrective-pass.md)

This bounded pass closed the remaining frontend/runtime verification gap after
Phases 0-3: `eggsec-tui --features full` is advertised as the maximum-capability
TUI aggregate but currently fails to compile because several feature-gated task
builders lag the runtime DTO contract. The same pass adds mechanical TUI feature
coverage to the individual-feature sweep and a dependency-light broad TUI
profile to routine verification so the defect cannot silently recur.

It did not reopen canonical request, approval-binding, dispatch ownership,
TUI surface-model, or runtime-contract architecture unless implementation
uncovers a concrete regression in those completed invariants.

## Frontend/runtime convergence (executed)

Roadmap:
[`frontend-runtime-convergence-roadmap-2026-09-10.md`](frontend-runtime-convergence-roadmap-2026-09-10.md)

This was a post-convergence corrective pass against the executed 2026-09-08
architecture roadmap. Canonical typed operation requests/defaults/validation
remain in place; the work resolved residual dispatch ownership overlap, CLI/TUI
metadata and feature drift, runtime wire/domain mirrors, approval-cache binding,
and frontend/runtime maintenance hotspots.

Ordered implementation plans (all executed; Phase 3 contains the closure record):

1. [`frontend-runtime-phase-0-binding-and-parity-guards.md`](frontend-runtime-phase-0-binding-and-parity-guards.md)
2. [`frontend-runtime-phase-1-canonical-dispatch-ownership.md`](frontend-runtime-phase-1-canonical-dispatch-ownership.md)
3. [`frontend-runtime-phase-2-tui-surface-wiring-convergence.md`](frontend-runtime-phase-2-tui-surface-wiring-convergence.md)
4. [`frontend-runtime-phase-3-runtime-contract-hotspot-closure.md`](frontend-runtime-phase-3-runtime-contract-hotspot-closure.md)

Phase 0 converted surface/binding assumptions into executable guards. Phase 1
established one engine-owned operation execution seam. Phase 2 completed TUI
metadata/action/feature wiring against that seam. Phase 3 resolved runtime
wire/domain ownership, proved embedded/daemon lifecycle parity, removed parallel
semantic mappings, and recorded closure evidence. The executed corrective pass
above records closure of the remaining TUI feature-profile verification defect.

## Architecture convergence and capability maturity (executed)

Roadmap:
[`architecture-convergence-roadmap-2026-09-08.md`](architecture-convergence-roadmap-2026-09-08.md)

This is a corrective convergence roadmap against the post-simplification
architecture. It does not reopen or replace the historical dependency/
architecture A-J roadmap. It targets residual dual scope semantics, feature/build
contract drift, incomplete operation/dispatch migration, protocol/agent coupling,
programmability parity, and platform integration maturity.

Ordered implementation plans (all executed; closure validated — see the Phase G
completion record for measurements and hosted CI confirmation):

1. [`architecture-convergence-phase-a-scope-contract-unification.md`](architecture-convergence-phase-a-scope-contract-unification.md)
2. [`architecture-convergence-phase-b-feature-build-verification-reconciliation.md`](architecture-convergence-phase-b-feature-build-verification-reconciliation.md)
3. [`architecture-convergence-phase-c-operation-dispatch-runtime-convergence.md`](architecture-convergence-phase-c-operation-dispatch-runtime-convergence.md)
4. [`architecture-convergence-phase-d-protocol-agent-boundaries-hotspot-decomposition.md`](architecture-convergence-phase-d-protocol-agent-boundaries-hotspot-decomposition.md)
5. [`architecture-convergence-phase-e-programmability-parity-browser-daemon-proxy.md`](architecture-convergence-phase-e-programmability-parity-browser-daemon-proxy.md)
6. [`architecture-convergence-phase-f-platform-integration-maturity.md`](architecture-convergence-phase-f-platform-integration-maturity.md)
7. [`architecture-convergence-phase-g-closure-measurement-documentation.md`](architecture-convergence-phase-g-closure-measurement-documentation.md)

Phases A and B are foundational. Phase C converges request/dispatch ownership;
Phase D then completes protocol/agent boundaries against that stable service
surface. Phase E closes existing programmable API execution gaps. Phase F adds
reproducible platform-sensitive integration evidence without expanding hazardous
capability. Phase G is a bounded closure/reconciliation pass and must run last.

## Dependency, architecture, and verification simplification

Roadmap:
[`dependency-architecture-simplification-roadmap.md`](dependency-architecture-simplification-roadmap.md)

Ordered implementation plans (all executed):

1. [`dependency-architecture-phase-a-authorization-target-binding.md`](dependency-architecture-phase-a-authorization-target-binding.md)
2. [`dependency-architecture-phase-b-scope-resolution-correctness.md`](dependency-architecture-phase-b-scope-resolution-correctness.md)
3. [`dependency-architecture-phase-c-feature-registry.md`](dependency-architecture-phase-c-feature-registry.md)
4. [`dependency-architecture-phase-d-metadata-consolidation.md`](dependency-architecture-phase-d-metadata-consolidation.md)
5. [`dependency-architecture-phase-e-advisory-dependency-remediation.md`](dependency-architecture-phase-e-advisory-dependency-remediation.md)
6. [`dependency-architecture-phase-f-engine-application-boundary.md`](dependency-architecture-phase-f-engine-application-boundary.md)
7. [`dependency-architecture-phase-g-binary-topology-and-tls.md`](dependency-architecture-phase-g-binary-topology-and-tls.md)
8. [`dependency-architecture-phase-h-upstream-msrv-native-deps.md`](dependency-architecture-phase-h-upstream-msrv-native-deps.md)
9. [`dependency-architecture-phase-i-ci-verification-simplification.md`](dependency-architecture-phase-i-ci-verification-simplification.md)
10. [`dependency-architecture-phase-j-measurement-and-closure.md`](dependency-architecture-phase-j-measurement-and-closure.md)

The A-J roadmap and the first three corrective passes are implemented and remain
part of the engineering record:

- [`dependency-architecture-corrective-closure-pass.md`](dependency-architecture-corrective-closure-pass.md)
- [`dependency-architecture-final-polish-pass.md`](dependency-architecture-final-polish-pass.md)
- [`dependency-architecture-post-polish-corrective-pass.md`](dependency-architecture-post-polish-corrective-pass.md)

The final dispatch-profile parity corrective pass is complete:

- [`dependency-architecture-dispatch-profile-parity-corrective-pass.md`](dependency-architecture-dispatch-profile-parity-corrective-pass.md)

This pass corrected dispatch and tool-API pipeline construction to use
`Pipeline::from_profile()` as the canonical parser-independent constructor,
ensuring `ScanProfile` is the single source of truth for stage selection, risk
budget, and profile-specific validation. The historical roadmap is closed; the
completed frontend/runtime convergence roadmap above addresses the later
maintenance, wiring, and runtime-contract work, including the executed TUI
full-profile corrective closure.

Package publication and release cadence remain manual maintainer actions.
