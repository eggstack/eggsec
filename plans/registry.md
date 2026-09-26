# Eggsec Active Planning Registry

This file is the compact control surface for planning. Detailed requirements and completed history remain in source roadmaps, implementation plans, closure evidence, and Git history. It links to source documents rather than duplicating their content.

Canonical direction remains in:

- `plans/000-long-term-specification.md`
- `plans/001-terminology-and-domain-model.md`
- `plans/002-long-term-roadmap.md`
- `plans/003-planning-process.md`

Durable decisions: `plans/adrs/ADR-0001-scoped-transport-eggfetch-backend.md`, `plans/adrs/ADR-0002-eggress-selective-reuse-boundary.md`.

Flat-era files (2026-07 – 2026-09) are grandfathered at the `plans/` top level with inline `Status: Executed` markers and appended completion records. They are immutable historical records referenced by `architecture/*.md`; they are NOT rewritten to the new templates. All new work follows `plans/003-planning-process.md` with new files under `plans/subsystems/`, `plans/implementation/<subsystem>/`, and `plans/closure/<subsystem>/`.

## Status vocabulary

- **proposed** — roadmap or plan exists but is not approved for execution.
- **ready** — dependencies and interfaces are satisfied; plan may be handed off.
- **active** — implementation or closure work is in progress.
- **blocked** — a named dependency or evidence requirement prevents progress.
- **closing** — implementation landed and closure evidence is being gathered.
- **closed** — closure record accepted (flat-era: inline completion record).
- **conditionally closed** — substantial work landed, but a named correctness or operational evidence condition remains.
- **superseded** — replaced by another document.
- **archived** — no longer active and retained for traceability.

## Subsystem standing

| Subsystem | Status | Roadmap(s) | Latest milestone / evidence |
|---|---|---|---|
| architecture-convergence | closed | `plans/architecture-convergence-roadmap-2026-09-08.md` | Phase G closure/measurement; completion records in each phase file |
| dependency-architecture-simplification | closed | `plans/dependency-architecture-simplification-roadmap.md` | Phase J + dispatch-profile parity corrective pass; closure report |
| crate-boundary-ownership | closed | `plans/crate-boundary-consolidation-roadmap-2026-09-16.md` | Phase D closes the roadmap |
| network-transport-egress | closed | `plans/network-dependency-hardening-roadmap-2026-09-11.md`; `plans/eggress-1.0.8-adoption-roadmap-2026-09-22.md` | Eggress 1.0.10 adoption + metadata closure (2026-09-25, implementation `fe5ec2d2`); ADR-0001, ADR-0002 controlling |
| frontend-runtime-tui | closed | `plans/frontend-runtime-convergence-roadmap-2026-09-10.md`; `plans/tui-terminal-ownership-corrective-roadmap-2026-09-19.md` | Phase B lifecycle closure + confirmation-intent corrective pass |
| python-programmability | closed | `plans/python-library-roadmap.md`; `plans/python-api-completion-roadmap.md`; `plans/python-api-high-value-roadmap.md`; `plans/python-api-release-5-roadmap.md` | Release 5 phase F compatibility/performance/release closure |
| ci-verification-release | closed | `plans/ci-verification-release-simplification-roadmap.md`; `plans/ci-release-simplification-corrective-closure-index.md` | Phase K evidence-toolchain polish |
| performance-resource-efficiency | closed | `plans/performance-resource-efficiency-roadmap-2026-09-21.md` | Phase F + closure-polish corrective pass |
| daemon-protocol-agent | closed | (single-plan workstream) `plans/ws6-daemon-schema-parity.md` | Executed |
| nse-runtime-extraction | active | `plans/subsystems/nse-runtime-extraction-roadmap.md` | Milestones 001-002 closed; Milestone 003 standalone extraction/cross-repo qualification ready for handoff |

## Grandfathered file index

### architecture-convergence (closed)

- `plans/architecture-convergence-roadmap-2026-09-08.md`
- `plans/architecture-convergence-phase-a-scope-contract-unification.md`
- `plans/architecture-convergence-phase-b-feature-build-verification-reconciliation.md`
- `plans/architecture-convergence-phase-c-operation-dispatch-runtime-convergence.md`
- `plans/architecture-convergence-phase-d-protocol-agent-boundaries-hotspot-decomposition.md`
- `plans/architecture-convergence-phase-e-programmability-parity-browser-daemon-proxy.md`
- `plans/architecture-convergence-phase-f-platform-integration-maturity.md`
- `plans/architecture-convergence-phase-g-closure-measurement-documentation.md`

### dependency-architecture-simplification (closed)

- `plans/dependency-architecture-simplification-roadmap.md`
- `plans/dependency-architecture-phase-a-authorization-target-binding.md` through `plans/dependency-architecture-phase-j-measurement-and-closure.md`
- `plans/dependency-architecture-simplification-closure-report.md`
- `plans/dependency-architecture-corrective-closure-pass.md`
- `plans/dependency-architecture-final-polish-pass.md`
- `plans/dependency-architecture-post-polish-corrective-pass.md`
- `plans/dependency-architecture-dispatch-profile-parity-corrective-pass.md`

### crate-boundary-ownership (closed)

- `plans/crate-boundary-consolidation-roadmap-2026-09-16.md`
- `plans/crate-boundary-consolidation-phase-a-ownership-and-primitive-cleanup.md`
- `plans/crate-boundary-consolidation-phase-b-report-model-extraction.md`
- `plans/crate-boundary-consolidation-phase-c-policy-enforcement-extraction.md`
- `plans/crate-boundary-consolidation-phase-d-loadtest-resilience-reuse-closure.md`

### network-transport-egress (closed)

- `plans/network-dependency-hardening-roadmap-2026-09-11.md`
- `plans/network-dependency-phase-a-baseline-invariants.md` through `plans/network-dependency-phase-g-closure-measurement.md`
- `plans/eggfetch-0.1.7-adoption-and-direct-route-simplification-2026-09-18.md`
- `plans/eggfetch-0.1.7-post-adoption-qualification-corrective-pass-2026-09-19.md`
- `plans/eggfetch-0.1.7-final-qualification-deep-check-corrective-pass-2026-09-19.md`
- `plans/eggfetch-0.2.0-adoption-and-requalification-2026-09-22.md`
- `plans/eggfetch-0.2.0-planning-record-duplication-cleanup-2026-09-22.md`
- `plans/eggress-1.0.8-adoption-roadmap-2026-09-22.md`
- `plans/eggress-1.0.8-phase-a-proxy-engine-adoption-2026-09-22.md`
- `plans/eggress-1.0.8-phase-b-health-qualification-and-closure-2026-09-22.md`
- `plans/eggress-1.0.8-post-adoption-compatibility-corrective-pass-2026-09-22.md`
- `plans/eggress-1.0.10-adoption-and-metadata-closure-2026-09-24.md`
- `plans/loadtest-authorization-transport-corrective-pass-2026-09-16.md`
- `plans/loadtest-multi-address-socket-binding-corrective-pass-2026-09-17.md`

### frontend-runtime-tui (closed)

- `plans/frontend-runtime-convergence-roadmap-2026-09-10.md`
- `plans/frontend-runtime-phase-0-binding-and-parity-guards.md` through `plans/frontend-runtime-phase-3-runtime-contract-hotspot-closure.md`
- `plans/frontend-runtime-tui-full-profile-corrective-pass.md`
- `plans/tui-terminal-ownership-corrective-roadmap-2026-09-19.md`
- `plans/tui-terminal-ownership-phase-a-single-writer-logging-boundary.md`
- `plans/tui-terminal-ownership-phase-b-lifecycle-process-output-closure.md`
- `plans/tui-broad-profile-warning-debt-cleanup-2026-09-22.md`
- `plans/tui-confirmation-test-intent-corrective-pass-2026-09-22.md`

### python-programmability (closed)

- `plans/python-library-roadmap.md`
- `plans/python-bindings-phase-a-foundation.md` through `plans/python-bindings-phase-f-major-tool-expansion.md`
- `plans/python-bindings-corrective-verification-pass.md`
- `plans/python-bindings-final-validation-polish-pass.md`
- `plans/python-bindings-active-api-scope-ci-pass.md`
- `plans/python-api-completion-roadmap.md`
- `plans/python-api-high-value-roadmap.md`
- `plans/python-api-milestone-a-engine-operation-model.md` through `plans/python-api-milestone-g-extensibility-stabilization.md`
- `plans/python-api-release-1-api-convergence.md`
- `plans/python-api-release-1-2-closure-pass.md`
- `plans/python-api-release-1-2-corrective-integration-pass.md`
- `plans/python-api-release-1-4-final-polish-pass.md`
- `plans/python-api-release-1-4-final-readiness-follow-up.md`
- `plans/python-api-release-1-4-operational-correction-pass.md`
- `plans/python-api-release-2-network-programmability.md`
- `plans/python-api-release-3-major-subsystem-completion.md`
- `plans/python-api-release-4-stateful-remote-execution.md`
- `plans/python-api-release-5-roadmap.md`
- `plans/python-api-release-5-phase-a-tool-schema-integration.md` through `plans/python-api-release-5-phase-f-compatibility-performance-release-closure.md`
- `plans/python-api-release-5-corrective-closure.md`
- `plans/python-api-release-closure-pass.md`
- `plans/python-api-corrective-integration-pass.md`
- `plans/python-api-final-integration-release-readiness.md`

### ci-verification-release (closed)

- `plans/ci-verification-release-simplification-roadmap.md`
- `plans/ci-verification-release-simplification-closure-report.md`
- `plans/ci-simplification-phase-a-policy-baseline.md` through `plans/ci-simplification-phase-f-documentation-and-closure.md`
- `plans/ci-release-simplification-corrective-closure-index.md`
- `plans/ci-release-simplification-corrective-phase-g-publishability.md` through `plans/ci-release-simplification-corrective-phase-k-evidence-toolchain-polish.md`
- `plans/ci-failure-remediation-2026-07-27.md`
- `plans/phase-b-gap-closure-performance-benchmarks-guards.md`

### performance-resource-efficiency (closed)

- `plans/performance-resource-efficiency-roadmap-2026-09-21.md`
- `plans/performance-phase-a-baseline-and-measurement-harness.md` through `plans/performance-phase-f-secondary-hotspots-and-closure.md`
- `plans/performance-closure-polish-corrective-pass-2026-09-21.md`

### daemon-protocol-agent (closed)

- `plans/ws6-daemon-schema-parity.md`

## Dependency-ready implementation plans

- `plans/implementation/nse-runtime-extraction/003-standalone-repository-extraction.md` — **ready for handoff**. Create `eggstack/eggsec-nse`, make metadata/CI/provenance standalone, perform real SSH-backed qualification, then switch Eggsec to one exact external Git revision and qualify both repositories. Publication/provider inversion remain deferred.
- `plans/implementation/nse-runtime-extraction/002-runtime-dependency-decoupling.md` — **closed** (`plans/closure/nse-runtime-extraction/002-closure.md`). Runtime crate has zero `eggsec-*` dependencies; report bridge and scoped-transport HTTP adapter moved into the engine; only `eggsec` directly depends on `eggsec-nse`; TUI/Python consume through `eggsec::nse`; guards 144/145/146 enforce it.
- `plans/implementation/nse-runtime-extraction/001-canonical-execution-report-convergence.md` — **closed**. One runtime-owned resolver/execution/report pipeline; runtime CLI, Eggsec manual dispatch/TUI, and Python migrated.

## Blocked work

- None currently registered for NSE runtime extraction. Milestone 003 is dependency-ready; later Milestones 004-005 remain sequenced behind its closure rather than treated as blocked implementation plans.

## Closure work and current control points

All closure evidence for flat-era work lives inline in the files above (appended completion records) and in `architecture/` deep-dives. Current foundation evidence relevant to future work includes:

| Area | Controlling evidence |
|---|---|
| Enforcement/dispatch ownership | architecture-convergence Phase C + closure report; `architecture/dispatch.md`, `architecture/config.md` |
| Crate boundaries | crate-boundary Phase D; `architecture/overview.md` dependency map |
| Transport/egress | Eggress 1.0.10 closure; ADR-0001, ADR-0002; `architecture/transport_eggfetch.md`, `architecture/network_dependency_closure.md` |
| Frontend/runtime | frontend-runtime Phase 3 + TUI terminal-ownership Phase B; `architecture/tui.md`, `architecture/runtime_bridge.md` |
| Python API | Release 5 phase F; `architecture/python_api.md` |
| CI/release | Phase K; `docs/VERIFICATION.md` |
| Performance | Phase F + polish corrective; `architecture/performance.md` |
| NSE runtime extraction | `plans/subsystems/nse-runtime-extraction-roadmap.md`; `plans/implementation/nse-runtime-extraction/003-standalone-repository-extraction.md`; Milestone 003 is the current handoff boundary |
