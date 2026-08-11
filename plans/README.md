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

Ordered implementation plans:

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

The A–J roadmap is implemented. The first corrective closure pass is also
implemented and resolved the active PyO3/quick-xml advisories, simplified routine
CI, and replaced estimated artifact measurements with observed values:

[`dependency-architecture-corrective-closure-pass.md`](dependency-architecture-corrective-closure-pass.md)

Final review found a small set of remaining closure/polish defects: the canonical
Python check was recorded as PASS despite one failing test, the Python binding
still enables the engine `cli` feature, progress/UI dependency ownership needs
one final graph-level disposition, and several active docs/scope-helper remnants
still describe the pre-corrective state.

Active final handoff:
[`dependency-architecture-final-polish-pass.md`](dependency-architecture-final-polish-pass.md)

This final pass is intentionally bounded to genuine Python CI success, Python/CLI
dependency separation, small dependency/documentation cleanup, and exact-final-SHA
validation. It does not reopen authorization, scope/DNS semantics, metadata,
daemon topology, dependency modernization, or CI architecture. Package
publication and release cadence remain manual maintainer actions.
