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

Final behavioral review found one narrow remaining pipeline-construction issue:
dispatch and tool-API callers do not consistently preserve the requested
`ScanProfile`, canonical stage set, and risk budget.

Active corrective handoff:
[`dependency-architecture-dispatch-profile-parity-corrective-pass.md`](dependency-architecture-dispatch-profile-parity-corrective-pass.md)

This handoff is limited to canonical profile construction, dispatch/tool-API
profile parity, output/session parameter disposition, focused behavioral tests,
and final exact-SHA closure evidence. The broader roadmap remains closed.

Package publication and release cadence remain manual maintainer actions.
