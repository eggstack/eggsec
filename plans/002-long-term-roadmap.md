# Eggsec Long-Term Implementation Roadmap

Status: execution roadmap for `plans/000-long-term-specification.md`

Terminology: `plans/001-terminology-and-domain-model.md`

This roadmap orders the work needed to reach and hold the long-term Eggsec architecture. Each phase MUST leave the repository in a coherent state and MUST include focused implementation plans, migrations, tests, documentation, and closure evidence before the next dependent phase is treated as available.

The roadmap is dependency-ordered, not calendar-ordered. Phases 0–6 are executed and closed; their evidence lives in the grandfathered flat-era plans mapped by `plans/registry.md`. Phase 7 is the standing maintenance posture for all future work.

## Cross-phase execution rules

Every phase MUST:

1. preserve the `EnforcementContext::evaluate()` pre-dispatch gate on all surfaces;
2. keep strict surfaces fail-closed through `EnforcedDispatcher::dispatch_execution()` with `ApprovedExecution` bundles;
3. use `LoadedScope` (never raw `Scope`) for strict authorization, with fail-closed `scope_from_spec` conversion;
4. keep `OperationMetadata` the single source of truth for operation policy;
5. maintain backward-compatible migrations or document an intentional break;
6. keep `eggsec-transport` light and implementation-neutral (never `eggsec-policy` → `eggsec-transport`);
7. keep TLS ring-only and Tokio `default-features = false` per-crate;
8. update architecture documentation and static ownership guards with code;
9. record explicit exit evidence in the implementation plan or closure record;
10. leave `make check` green with dependency policy and MSRV intact.

## Phase 0 — Enforcement, scope, and dispatch foundation

Objective: unify scope semantics, reconcile the feature/build/verification contract, and converge operation/dispatch ownership on the canonical execution seam.

Status: **closed**. Evidence: `architecture-convergence-roadmap-2026-09-08.md` plus phases A–G (A scope-contract unification, B feature-build-verification reconciliation, C operation-dispatch-runtime convergence, D protocol/agent boundaries, E programmability parity, F platform integration maturity, G closure/measurement), and the dependency-architecture A–J simplification (authorization-target binding, scope-resolution correctness, feature registry, metadata consolidation, advisory remediation, engine/application boundary, binary topology + TLS, upstream MSRV/native deps, CI verification simplification, measurement and closure) with its corrective passes.

## Phase 1 — Crate boundaries and reusable-library ownership

Objective: fix report-DTO/formatter ownership (`eggsec-report-model` extraction), separate policy/enforcement semantics from configuration loading (`eggsec-policy` extraction), shrink the `utils` catch-all, and decouple load testing behind the scoped transport seam — promoting code to a new crate only on measured dependency payoff.

Status: **closed**. Evidence: `crate-boundary-consolidation-roadmap-2026-09-16.md` plus phases A–D.

## Phase 2 — Scoped transport, Eggfetch backend, and Eggress reuse

Objective: establish the mandatory scoped transport contract, migrate ordinary outbound clients onto it, adopt the published Eggfetch backend with requalification, and make measured Eggress adopt/reject decisions while narrowing process/runtime capability inheritance and hardening supply-chain policy.

Status: **closed**. Evidence: `network-dependency-hardening-roadmap-2026-09-11.md` (phases A–G); Eggfetch 0.1.7 adoption + two qualification corrective passes; Eggfetch 0.2.0 adoption + record-dedup pass; Eggress 1.0.8 adoption roadmap + phases A/B + compatibility corrective pass; Eggress 1.0.10 adoption and socket-metadata closure; load-test authorization/transport and multi-address socket-binding corrective passes.

## Phase 3 — Frontend and runtime convergence

Objective: resolve dispatch ownership overlap, converge CLI/TUI metadata and feature wiring on the canonical execution seam, unify runtime wire/domain contracts with approval-cache binding, and pin the single-terminal-writer/logging boundary with lifecycle-safe teardown.

Status: **closed**. Evidence: `frontend-runtime-convergence-roadmap-2026-09-10.md` (phases 0–3) + TUI full-profile corrective pass; `tui-terminal-ownership-corrective-roadmap-2026-09-19.md` (phases A/B); TUI warning-debt cleanup; confirmation-test intent corrective pass; `ws6-daemon-schema-parity.md`.

## Phase 4 — Python programmability parity

Objective: bring Python bindings from foundation to full engine-operation, policy/config, assessment-domain, findings/reporting, and lab-domain coverage with packaging, docs, and compatibility/performance closure.

Status: **closed**. Evidence: `python-library-roadmap.md`; `python-bindings-phase-a-foundation.md` through `-phase-f-major-tool-expansion.md` plus corrective/validation passes; `python-api-completion-roadmap.md` and milestones A–G; `python-api-release-1-*` through `python-api-release-5-*` (roadmap + phases A–F) plus closure/corrective passes.

## Phase 5 — CI, verification, and release simplification

Objective: collapse core CI, Python verification, and optional security workflows; make packaging cargo-native; harden publishability, release integrity, and evidence tooling.

Status: **closed**. Evidence: `ci-verification-release-simplification-roadmap.md` + closure report; `ci-simplification-phase-a-policy-baseline.md` through `-phase-f-documentation-and-closure.md`; `ci-release-simplification-corrective-closure-index.md` + phases G–K; `ci-failure-remediation-2026-07-27.md`; `phase-b-gap-closure-performance-benchmarks-guards.md`.

## Phase 6 — Performance and resource-efficiency closure

Objective: bound high-cardinality async fan-out, remove load-test request hotpath waste, make worker concurrency a real capacity contract, reuse coordinator sessions, and close secondary hotspots — all behind measurement gates with public-surface and wire-format compatibility preserved.

Status: **closed**. Evidence: `performance-resource-efficiency-roadmap-2026-09-21.md` (phases A–F) + closure-polish corrective pass.

## Phase 7 — Standing maintenance and future capability (open)

Objective: hold all closed-phase invariants while landing future capability through the governance in `plans/003-planning-process.md`.

Standing rules:

- New outbound-HTTP behavior goes through the `eggsec-transport` contract and qualified backend; no second HTTP abstraction or retry owner.
- New authorization semantics go through `eggsec-policy` + engine bridge with an ADR when cross-cutting.
- New crates only on measured dependency payoff (Phase 1 gate restated).
- New subsystem roadmaps live in `plans/subsystems/`; milestone plans in `plans/implementation/<subsystem>/`; closure records in `plans/closure/<subsystem>/`; all registered in `plans/registry.md`.
- Known deferred items (not commitments): Eggress typed detailed failure classification (explicitly deferred at 1.0.10 adoption); broader proxy capability expansion (routing, embed/runtime/server, H2, SSH, QUIC/H3, UDP, pproxy compatibility, insecure TLS) unless a new ADR reopens them.

Exit criteria: none — this phase never closes. Individual milestones close via their closure records.
