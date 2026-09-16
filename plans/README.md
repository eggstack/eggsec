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

## Load-test authorization and transport corrective pass (ready for handoff)

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

## TUI full-profile corrective closure (active)

Corrective pass:
[`frontend-runtime-tui-full-profile-corrective-pass.md`](frontend-runtime-tui-full-profile-corrective-pass.md)

This bounded pass closes the remaining frontend/runtime verification gap after
Phases 0-3: `eggsec-tui --features full` is advertised as the maximum-capability
TUI aggregate but currently fails to compile because several feature-gated task
builders lag the runtime DTO contract. The same pass adds mechanical TUI feature
coverage to the individual-feature sweep and a dependency-light broad TUI
profile to routine verification so the defect cannot silently recur.

It does not reopen canonical request, approval-binding, dispatch ownership,
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
semantic mappings, and recorded closure evidence. The active corrective pass
above addresses the one remaining TUI feature-profile verification defect.

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
maintenance, wiring, and runtime-contract work, while the active TUI full-profile
corrective pass closes its remaining feature-verification gap.

Package publication and release cadence remain manual maintainer actions.
