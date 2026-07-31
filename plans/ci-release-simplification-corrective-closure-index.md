# CI and Manual Release Simplification — Corrective Closure Index

## Status

Corrective Phase J local implementation complete; hosted verification pending
for the pushed commit.

Corrective Phase I completed its CI, Python parity, lockfile, versioning, and hosted-evidence corrections. Phase J replaced the handwritten Rust archive stage with Cargo-native workspace packaging and standalone package validation. Hosted verification remains the final evidence gate.

Publication remains manual and has not been run.

## Purpose

This index isolates defects discovered after implementation. The broad CI reduction effort must not be reopened. The only remaining work belongs to Rust release-package integrity.

No new roadmap expansion is authorized. The objective is to close the existing simplification work with direct, reproducible evidence and without restoring deleted automation.

## Accepted baseline

The following outcomes are landed and must be preserved:

- automated PyPI, TestPyPI, crates.io, and GitHub Release publication is absent from hosted CI;
- tag pushes have no release side effects;
- the external-target self-scan workflow is deleted;
- the root GitLab consumer pipeline is relocated under `examples/`;
- `.github/workflows/ci.yml` is the single mandatory workflow;
- mandatory Rust CI is consolidated under `make check`;
- mandatory Python CI invokes `make check-python` and builds the extension once;
- `.github/workflows/deep-checks.yml` is the only optional diagnostic workflow;
- evidence-bundle, maturity-gate, skip-budget, and synthetic release-gate orchestration is removed;
- specialist Make targets use valid Cargo syntax;
- Python sync/async parity again includes `is_vulnerable` and meaningful semantic outcomes;
- `Cargo.lock` is minimized relative to the pre-Phase-G baseline except for the justified `event-listener` security patch;
- internal publishable dependencies use local paths plus registry versions;
- the package order is derived from Cargo metadata;
- the version-bump procedure includes internal dependency versions;
- hosted CI run evidence is recorded with an actual run ID and job conclusions;
- publication cadence is manual and maintainer-controlled.

Corrective implementation must not restore deleted workflow graphs, publishing automation, evidence bureaucracy, or runtime scope expansion.

## Completed corrective phases

### Corrective Phase G — Publishability and release-check restructuring

Plan: [`ci-release-simplification-corrective-phase-g-publishability.md`](ci-release-simplification-corrective-phase-g-publishability.md)

Accepted outcomes retained:

- private crates have explicit `publish = false`;
- publishable internal dependencies have local paths plus registry versions;
- package ordering is derived from Cargo metadata;
- the release check separates default validation from optional registry preflight;
- release publication remains manual.

### Corrective Phase H — CI contract, command, and documentation consolidation

Plan: [`ci-release-simplification-corrective-phase-h-ci-contract-closure.md`](ci-release-simplification-corrective-phase-h-ci-contract-closure.md)

Accepted outcomes:

- Rust, Python, and portability jobs are consolidated in `ci.yml`;
- `test.yml` is deleted;
- hosted Python verification invokes the canonical local command;
- optional feature profiles have one owner;
- invalid specialist Make targets are corrected;
- active documentation reflects the compact workflow shape.

Phase J must not redesign this accepted workflow contract.

### Corrective Phase I — Release result integrity, parity, lockfile, and evidence

Plan: [`ci-release-simplification-corrective-phase-i-release-integrity.md`](ci-release-simplification-corrective-phase-i-release-integrity.md)

Accepted outcomes:

- the previous `PACKAGE_FIRST_RELEASE` false-success branch is removed;
- failed package-stage commands are no longer matched by error string and converted into success;
- registry preflight is reported as `SKIPPED` when not run;
- `is_vulnerable` is restored to semantic Python parity comparison;
- the lockfile delta is reduced to the targeted `event-listener` security patch;
- version-bump guidance includes all version-qualified internal path dependencies;
- hosted workflow run `30632663714` records successful Rust, Python, macOS, and Windows jobs;
- branch-protection state remains honestly recorded as `NOT VERIFIED`.

Post-implementation review invalidated only the claim that the custom generated tar files are Cargo-valid publishable archives. Phase I's other outcomes remain accepted.

## Remaining blocker

### Corrective Phase J — Cargo-native package archive closure

Plan: [`ci-release-simplification-corrective-phase-j-cargo-native-packaging.md`](ci-release-simplification-corrective-phase-j-cargo-native-packaging.md)

Primary required outcomes:

- Cargo itself creates every `.crate` archive used for release validation;
- custom tar creation and regex manifest normalization are removed;
- workspace-inherited package and dependency metadata is normalized by Cargo;
- each extracted archive passes standalone `cargo metadata --no-deps --offline` outside the source workspace;
- the exact expected archive set is enforced;
- aliases, optional dependencies, target-specific dependencies, feature references, private packages, metadata, README/license files, and prohibited entries are inspected;
- every Rust archive records path, size, and SHA-256;
- Cargo packaging failures remain failures;
- the shell no longer owns archive selection through `mapfile` or stale `find` results;
- active documentation describes the command that actually runs;
- registry preflight remains a separate staged maintainer operation;
- closure evidence is collected only against the final implementation commit.

This is the only remaining blocker for the CI/manual-release simplification line.

## Current evidence classification

Until Phase J completes, use:

```text
CI workflow simplification: PASS
Python semantic parity: PASS
Lockfile minimization: PASS
Version-bump procedure: PASS
Hosted CI run 30632663714: PASS
Branch protection: NOT VERIFIED
Cargo-native Rust archives: PASS (12/12)
make release-check release-archive criterion: PASS
Registry preflight: SKIPPED
Publication: NOT RUN
```

The prior `make release-check` execution remains useful evidence for Rust/Python checks, wheel/sdist construction, and fresh-wheel smoke. It is not sufficient evidence that the 12 handwritten tar files are Cargo publishable archives.

## Sequencing rules

1. Implement Phase J only; do not reopen the hosted CI or Make command design.
2. Begin with a synthetic workspace proof of the exact Cargo package command.
3. Do not write additional documentation claims until Cargo-native archive creation is demonstrated.
4. Remove the handwritten archive writer even if the Cargo-native proof exposes a limitation.
5. If Cargo cannot produce all archives before registry publication, reduce the contract honestly rather than constructing substitutes.
6. Do not classify a failed, skipped, timed-out, blocked, or unverified operation as `PASS`.
7. Keep registry preflight separate from default local archive validation.
8. Review `Cargo.lock` after implementation and reject unrelated dependency changes.
9. Run the complete Phase J validation sequence on the final implementation commit.
10. Record an actual hosted workflow run URL or ID before making a new remote CI claim.
11. Do not publish any crate, Python package, tag, or GitHub Release while executing the plan.
12. Mark the closure report complete only after every blocking Phase J criterion returns `PASS`.

## Corrective acceptance criteria

The corrective sequence is complete only when all of the following are true:

1. The closure report is reopened before Phase J implementation evidence is claimed.
2. Cargo, not Python `tarfile`, creates every `.crate` archive used for validation.
3. No regex-based publish-manifest normalization remains.
4. The Cargo-native command is proven with a synthetic workspace containing unpublished internal dependencies.
5. The fixture includes `[workspace.package]` and `[workspace.dependencies]` inheritance.
6. The publishable package set is derived from Cargo metadata.
7. Private packages are excluded explicitly.
8. Exactly one expected Cargo-generated archive exists per package/version.
9. Missing, duplicate, or unexpected archives return failure.
10. Each archive is extracted outside the source workspace.
11. Each extracted manifest passes standalone `cargo metadata --no-deps --offline`.
12. No packaged manifest retains `workspace = true`.
13. No runtime/build/target runtime/build dependency retains a local path.
14. Internal dependency versions match the release version policy.
15. Aliased internal dependencies are checked by actual package name.
16. Optional dependency flags are preserved.
17. Feature references resolve to valid features or optional dependencies.
18. Private crates are absent from publish-facing runtime/build dependency graphs.
19. Package metadata and configured README/license files are validated.
20. Prohibited archive entries are rejected.
21. Every Rust archive records path, size, and SHA-256.
22. Cargo package failure remains non-zero and prevents a PASS summary.
23. Registry preflight is separate and labeled `SKIPPED` when not run.
24. `release-check.sh` no longer uses `mapfile` for archive selection.
25. Active documentation states the exact packaging command actually used.
26. Unsupported macOS or Rust 1.80 claims are removed or backed by direct evidence.
27. Package-helper unit and real-Cargo fixture tests pass.
28. The real workspace package graph validates and orders successfully.
29. `make check` passes.
30. `make check-python` passes.
31. `make check-full` passes.
32. `make release-check` passes using only Cargo-generated Rust archives.
33. Hosted CI success includes a real workflow run URL/ID and per-job conclusions.
34. Branch protection remains honestly labeled if unavailable.
35. `Cargo.lock` contains no unrelated dependency churn.
36. No hosted workflow publishes, triggers on tags, scans external targets, or recreates evidence bundles.
37. No runtime behavior, enforcement posture, public API, feature scope, or crate architecture is changed.
38. No package or release is published while executing the corrective phase.

## Explicit exclusions

This corrective sequence does not include:

- publishing a release;
- changing release cadence back to automation;
- reintroducing wheel matrices on ordinary pushes;
- adding a local registry service without separate authorization;
- adding Docker or a registry container;
- adding provenance, attestation, signing, SBOM, or release bots;
- broad dependency upgrades;
- reorganizing crate boundaries;
- adding another evidence framework;
- expanding product functionality;
- redesigning the accepted Phase H CI workflow;
- claiming macOS or Rust 1.80 release support without direct execution evidence.

## Handoff requirement

Implementation agents must treat command results as evidence, not documentation assertions.

Use only:

```text
PASS
FAIL
SKIPPED
NOT RUN
NOT VERIFIED
BLOCKED
TIMEOUT
```

A failed, skipped, timed-out, blocked, or unverified operation is never a pass. The final closure update must include the exact Cargo packaging command, package set, dependency order, implementation commit, validation host, archive inventory, local command outcomes, hosted workflow evidence, and any retained lockfile changes.
