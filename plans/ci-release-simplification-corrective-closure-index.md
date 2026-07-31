# CI and Manual Release Simplification — Corrective Closure Index

## Status

Reopened for Corrective Phase I. The original CI reduction and Corrective Phase H workflow/command consolidation remain accepted. Final manual-release closure is pending correction of the package-validation false positive, Python semantic parity regression, lockfile review, version-bump procedure, and hosted evidence record.

## Purpose

This index isolates defects discovered after implementation. The broad reduction effort must not be reopened. The remaining work is narrow and belongs to release-validation integrity rather than CI expansion.

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
- publication cadence is manual and maintainer-controlled.

Corrective implementation must not restore deleted workflow graphs, publishing automation, or evidence bureaucracy.

## Completed corrective phases

### Corrective Phase G — Publishability and release-check restructuring

Plan: [`ci-release-simplification-corrective-phase-g-publishability.md`](ci-release-simplification-corrective-phase-g-publishability.md)

Landed outcomes retained by Phase I:

- private crates have explicit `publish = false`;
- publishable internal dependencies have local paths plus registry versions;
- package ordering is derived from Cargo metadata;
- the release check separates default validation from optional registry preflight;
- release publication remains manual.

Post-implementation review found that failed `cargo package` commands can still be classified as expected first-release conditions and converted into overall success. Phase I corrects that result-integrity defect without reverting the valid manifest work.

### Corrective Phase H — CI contract, command, and documentation consolidation

Plan: [`ci-release-simplification-corrective-phase-h-ci-contract-closure.md`](ci-release-simplification-corrective-phase-h-ci-contract-closure.md)

Accepted outcomes:

- Rust, Python, and portability jobs are consolidated in `ci.yml`;
- `test.yml` is deleted;
- hosted Python verification invokes the canonical local command;
- optional feature profiles have one owner;
- invalid specialist Make targets are corrected;
- active documentation reflects the compact workflow shape.

Phase I must not redesign this accepted workflow contract.

## Remaining blocker

### Corrective Phase I — Release integrity and evidence closure

Plan: [`ci-release-simplification-corrective-phase-i-release-integrity.md`](ci-release-simplification-corrective-phase-i-release-integrity.md)

Primary outcomes required:

- failed Rust package operations remain failures;
- every intended crates.io archive is created and inspected deterministically before publication;
- registry-dependent first-release verification is handled through an explicit staged manual process;
- `is_vulnerable` and other meaningful Python semantic outcomes remain covered by sync/async parity tests;
- unrelated `Cargo.lock` dependency churn is removed or individually justified;
- version-bump instructions include all internal dependency requirements;
- local and hosted closure evidence is recorded only when actually observed.

This phase is the only remaining blocker for the CI/manual-release simplification line.

## Sequencing rules

1. Reopen the retained closure report before implementation claims begin.
2. Preserve the current two-workflow inventory and Make command contract.
3. Correct package-result semantics before collecting new release evidence.
4. Do not treat `no matching package named`, a timeout, a skipped registry check, or unavailable hosted evidence as pass.
5. Restore semantic Python parity before rerunning the full Python suite.
6. Review lockfile changes against pre-Phase-G commit `f26942dc37783ee302ffc5c67e58810cfdcc3578` before accepting the final dependency graph.
7. Run the complete Phase I validation sequence on the final implementation commit.
8. Record an actual hosted workflow run URL or ID before claiming remote CI success.
9. Do not publish any crate, Python package, tag, or GitHub Release while executing the plan.
10. Mark the closure report complete only after every blocking Phase I criterion returns `PASS`.

## Corrective acceptance criteria

The corrective sequence is complete only when all of the following are true:

1. Every intended crates.io package has valid path-plus-version internal dependency metadata.
2. The package graph validates and produces an acyclic topological order.
3. Default `make release-check` deterministically creates and inspects an archive for every intended crates.io package.
4. Failed archive creation or inspection returns non-zero and stops the release check.
5. `no matching package named` is never classified as successful validation.
6. Packaged manifests retain no local path keys for normal/build/target runtime dependencies.
7. Packaged internal dependency versions match the release version policy.
8. Private crates are absent from published runtime/build dependency graphs.
9. Registry preflight is clearly separate and is run layer by layer during manual publication.
10. Skipped registry preflight is recorded as `SKIPPED`, not `PASS`.
11. `is_vulnerable` is restored to meaningful Python sync/async semantic comparison.
12. The focused parity test passes repeatedly and `make check-python` passes.
13. `Cargo.lock` contains no unexplained third-party version/source/checksum changes relative to the pre-Phase-G baseline.
14. The final lockfile remains compatible with the declared Rust MSRV or any MSRV correction is explicitly approved and documented.
15. Release version-bump instructions include workspace, Python, and internal dependency versions.
16. The graph validator identifies stale internal dependency versions with file-level diagnostics.
17. `make check`, `make check-python`, `make check-full`, and `make release-check` pass on the final implementation commit.
18. Hosted CI success claims include an actual workflow run URL or ID and per-job conclusions.
19. Unavailable branch-protection or hosted evidence is labeled `NOT VERIFIED`.
20. No hosted workflow publishes, triggers on tags, scans external targets, or recreates evidence bundles.
21. No runtime behavior, enforcement posture, public API, or feature scope is weakened to close process defects.
22. No package or release is published while executing the corrective plan.

## Explicit exclusions

This corrective sequence does not include:

- publishing a release;
- changing release cadence back to automation;
- reintroducing wheel matrices on ordinary pushes;
- adding a local registry service unless separately authorized after the lightweight archive approach is proven impossible;
- broad dependency upgrades;
- reorganizing crate boundaries;
- adding provenance, attestation, signing, SBOM, or release-bot infrastructure;
- adding another evidence framework;
- expanding product functionality;
- redesigning the accepted Phase H CI workflow.

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

A failed, skipped, timed-out, blocked, or unverified operation is never a pass. The final closure update must include the exact package set, dependency order, implementation commit, validation host, local command outcomes, hosted workflow run evidence, and any retained lockfile changes.
