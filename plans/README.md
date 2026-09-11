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
