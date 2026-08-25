# CI, Verification, and Manual Release Simplification Roadmap

## Status


Status: Executed.
Completed 2026-07-31. This document remains a handoff roadmap record; corrective closure was completed without runtime behavior changes.

### Corrective closure amendment

Corrective Phase G closed the manual package graph and release-check path. Corrective
Phase H then consolidated mandatory Rust and Python checks in `ci.yml`, removed the
duplicated Python workflow, corrected specialist Make targets, and reconciled the
verification documentation. The final evidence is retained in
[`ci-verification-release-simplification-closure-report.md`](ci-verification-release-simplification-closure-report.md).

## Purpose

Eggsec's current continuous-integration, verification, and release apparatus has grown beyond what is justified by the project's size and present release model. Ordinary pushes and pull requests trigger overlapping Rust feature matrices, repeated Python extension builds, release-evidence generation, maturity and skip-budget gates, package artifact construction, dependency scanners, security scanning workflows, and release-oriented orchestration.

The resulting system creates three material problems:

1. iteration is slowed by a large job graph whose failures often reflect workflow plumbing rather than product defects;
2. contributors and implementation agents cannot identify one canonical local command that predicts merge readiness;
3. GitHub Actions has become entangled with release cadence and package publication even though releases are intended to be deliberate, manual maintainer actions.

This roadmap reduces the apparatus to the smallest set that preserves high-value correctness signals. It explicitly moves package publication outside GitHub CI and codifies a manual local crates.io/PyPI release procedure. Eggsec currently publishes Rust crates and a Python package surface; the implementation must preserve the repository's actual package destinations while ensuring GitHub Actions publishes neither. For the Python binding, the applicable package index is PyPI. For any publishable Rust crates, the applicable registry is crates.io.

## Non-negotiable policy

The completed system must satisfy all of the following:

- GitHub Actions does not publish to crates.io, PyPI, TestPyPI, or GitHub Releases.
- Tag creation does not trigger artifact building, publishing, or release creation.
- Release cadence is chosen manually by maintainers.
- Release credentials are not required by repository CI.
- The mandatory CI path is small enough to run on every pull request and push to `main` without materially impeding iteration.
- A contributor can reproduce mandatory CI with documented local commands.
- Expensive, platform-sensitive, privileged, networked, exhaustive, and release-only checks are not mandatory merge gates.
- Correctness checks are retained when they protect observable behavior or critical architectural invariants.
- Process artifacts such as evidence bundles, maturity promotion records, and skip-budget orchestration do not substitute for behavioral tests.
- No phase changes Eggsec enforcement semantics, scope policy, tool behavior, public API behavior, or feature maturity merely to make CI easier.

## Current baseline

The current repository has the following overlapping surfaces:

- `.github/workflows/test.yml`: formatting, Clippy, Rust check and test matrices, coverage, cross-platform release builds, advisory and license tooling, dependency review, secret scanning, architecture guards, feature profiles, repeated Python builds, Python capability/maturity/type/stub checks, evidence generation, skip-budget enforcement, and a synthetic release gate.
- `.github/workflows/deep-checks.yml`: scheduled and manual all-feature workspace checks and tests.
- `.github/workflows/security-scan.yml`: downloads a previously released Eggsec binary and runs network scans from GitHub-hosted infrastructure.
- `.github/workflows/python-wheels.yml`: builds and tests wheels on normal development events and contains TestPyPI/PyPI publishing paths.
- `.github/workflows/testpypi-rehearsal.yml`: publishes rehearsal artifacts to TestPyPI.
- `.github/workflows/release.yml`: tag-driven release validation, wheel/sdist construction, artifact evidence, and downstream release actions.
- `.gitlab-ci.yml`: consumer-style scan examples represented as repository CI.
- `Makefile`: overlapping nextest, architecture, feature-profile, Phase F, compatibility, redaction, resource-budget, and evidence targets.
- `scripts/`: release-candidate, evidence, compatibility, profile, skip-budget, and architecture validation scripts with mixed ownership.
- `AGENTS.md` and Python packaging documentation: multiple definitions of required verification and release readiness.

## Target end state

### Mandatory GitHub CI

One mandatory workflow, preferably `.github/workflows/ci.yml`, with no more than three conceptual jobs:

1. `rust`: formatting, workspace compile coverage, targeted Clippy, core tests, and the small set of architecture/enforcement tests that protect critical invariants;
2. `python`: one default Python 3.12 environment, one `maturin develop` build, Python tests, stub/API/type checks executed in the same environment;
3. `portability`: narrow macOS and Windows compile or unit-test smoke checks, with Linux as the comprehensive environment.

The exact job split may be two to four jobs if required by runner operating systems, but the logical contract must remain small and obvious. Jobs must not be split merely to create separate status checks.

### Optional diagnostics

At most one optional workflow may remain for slow or broad validation. It must be manual or scheduled, non-publishing, and non-required for routine merges. It may exercise representative feature profiles, all-feature compilation where valid, slow tests, dependency policy, or coverage.

### Local verification

The repository exposes three clear commands:

- `make check`: mandatory Rust/core verification and the canonical default contributor command;
- `make check-python`: one Python binding build followed by required Python behavioral and static checks;
- `make check-full`: optional slow, broad, system-dependent, or release-preparation checks.

Standard Cargo and Python tooling should be preferred. `cargo-nextest` may remain an optional accelerator but must not be required to reproduce mandatory CI.

### Manual release

Release preparation is a local operator workflow. A release-check script may validate a clean tree, version alignment, tests, package metadata, artifact construction, and fresh-environment installation. It must stop before publication. Maintainers run the final `cargo publish`, `maturin publish`, or `twine upload` command explicitly.

### Documentation

One document defines routine verification. One document defines manual release preparation and publication. Other docs link to those canonical sources rather than restating divergent command matrices.

## Principles for deciding what remains mandatory

A check belongs in mandatory CI only when all of the following are true:

- it protects behavior, compilation, safety enforcement, or a durable architectural boundary;
- it is deterministic on a hosted runner;
- it does not require credentials, privileged devices, external targets, or mutable third-party state;
- it runs frequently enough that delayed feedback would materially increase correction cost;
- it is not already covered by another mandatory command;
- its failure message identifies a corrective action without requiring release-artifact archaeology.

Checks that do not meet this standard must move to local optional validation, a scheduled diagnostic workflow, or removal.

## Phase sequence

### Phase A — Verification policy and baseline contract

Plan: [`ci-simplification-phase-a-policy-baseline.md`](ci-simplification-phase-a-policy-baseline.md)

Establish a measured baseline and write the canonical policy before deleting workflows. Inventory every CI job, script, Make target, required status, artifact, and documentation reference. Classify each as mandatory, optional, release-only, example-only, or obsolete. Define the exact mandatory commands and record protected invariants so later phases do not accidentally trade correctness for speed.

Exit condition: the repository has one reviewed verification contract and an explicit migration map for every current job and script.

### Phase B — Core Rust CI collapse

Plan: [`ci-simplification-phase-b-core-ci-collapse.md`](ci-simplification-phase-b-core-ci-collapse.md)

Replace the Rust portion of `test.yml` with a compact Linux-first CI contract plus narrow portability smoke checks. Remove duplicated feature matrices, per-check job fragmentation, routine coverage builds, and redundant security tools from the mandatory path. Preserve enforcement, registry, metadata, report-envelope, no-default, and architecture-boundary protection where evidence shows they are distinct.

Exit condition: routine Rust CI is reproducible with `make check`, has a small job graph, and no longer performs release builds or exhaustive feature permutations.

### Phase C — Python verification collapse and evidence retirement

Plan: [`ci-simplification-phase-c-python-verification-collapse.md`](ci-simplification-phase-c-python-verification-collapse.md)

Build the default Python extension once per Linux CI run, execute behavioral and static checks in that environment, and eliminate the multi-job evidence/maturity/skip-budget release DAG. Preserve installed-package smoke tests, stub parity, API metadata consistency, typing, redaction, and bounded resource tests only where they protect real behavior.

Exit condition: Python CI has one primary build/test job, no artifact-transfer gate chain, and no repeated `maturin develop` jobs.

### Phase D — Optional, security, and consumer workflow cleanup

Plan: [`ci-simplification-phase-d-optional-security-workflow-cleanup.md`](ci-simplification-phase-d-optional-security-workflow-cleanup.md)

Delete the repository self-scan workflow, remove or relocate GitLab consumer examples, and reduce broad scheduled checks to one non-blocking diagnostic workflow. Consolidate dependency/advisory/license/secret checks according to distinct value rather than tool count.

Exit condition: the repository has at most one optional diagnostic workflow and no CI that scans arbitrary external targets or tests previously published binaries as a proxy for the current commit.

### Phase E — Manual release workflow and publication removal

Plan: [`ci-simplification-phase-e-manual-release-workflow.md`](ci-simplification-phase-e-manual-release-workflow.md)

Remove tag-triggered release automation, PyPI/TestPyPI publishing jobs, TestPyPI rehearsal automation, GitHub release creation, and release credentials from CI. Add a local release-check procedure that validates but never publishes. Document explicit maintainer commands for crates.io and PyPI as applicable.

Exit condition: no GitHub or GitLab workflow can publish, a tag has no release side effects, and a maintainer can perform a verified release locally using one documented sequence.

### Phase F — Documentation, guard reconciliation, and closure

Plan: [`ci-simplification-phase-f-documentation-and-closure.md`](ci-simplification-phase-f-documentation-and-closure.md)

Remove stale CI/release references, simplify `AGENTS.md` and the Makefile help surface, delete orphaned scripts and manifests, validate branch protection expectations, and produce closure evidence based on actual commands and workflow structure rather than generated evidence bundles.

Exit condition: repository documentation, scripts, Make targets, and workflow files describe one coherent model with no dead references or hidden publishing paths.

## Ordering and dependency rules

Phases must be implemented in order.

- Phase A is documentation and classification only. It prevents later deletions from becoming ad hoc.
- Phase B may create the new mandatory workflow but should not yet delete Python or release workflows unless necessary to avoid duplicate triggers.
- Phase C depends on the command contract from Phase A and the workflow structure from Phase B.
- Phase D should occur after mandatory Rust and Python coverage is visibly present in the replacement workflow.
- Phase E must not publish a rehearsal package while being implemented; validation uses local artifact construction only.
- Phase F is a closure pass, not an opportunity to add new verification frameworks.

Each phase must leave `main` in a usable state. Do not create an intermediate commit in which all mandatory validation is absent.

## Deliberate exclusions

This roadmap does not include:

- changing Eggsec runtime architecture;
- weakening scope or enforcement policy;
- reducing behavioral test coverage solely to shorten CI;
- publishing a new package version;
- changing package names or registry ownership;
- adding a release bot, changelog bot, release PR service, or external CI provider;
- introducing Nix, Bazel, Earthly, custom container orchestration, or another build meta-system;
- adding self-hosted runners;
- making all-feature tests pass by altering product semantics;
- preserving historical process machinery merely because prior plans mention it.

## Roadmap-level acceptance criteria

The roadmap is complete only when all of the following are true:

1. `.github/workflows/` contains one mandatory CI workflow and at most one optional non-publishing diagnostic workflow.
2. No workflow has `id-token: write` for package publication, package-index credentials, `cargo publish`, `maturin publish`, `twine upload`, `gh release`, or equivalent release mutation commands.
3. Pushes of ordinary commits do not build distributable wheel matrices or release artifacts.
4. Pushing a `v*` tag does not trigger a release workflow.
5. Mandatory CI builds the Python extension no more than once per operating-system/profile combination actually required by the contract.
6. `make check` reproduces mandatory Rust/Linux verification with standard toolchain prerequisites.
7. `make check-python` reproduces mandatory Python/Linux verification in one virtual environment.
8. `make check-full` is clearly optional and does not publish.
9. Linux runs comprehensive validation; macOS and Windows run narrow portability checks only.
10. Critical enforcement, metadata, registry, dispatch, and report-envelope invariants retain direct tests or guards.
11. Network security scans against external targets are absent from project CI.
12. Release documentation states that cadence and publication are manual maintainer decisions.
13. Local release validation stops before publication and succeeds without GitHub Actions.
14. A repository-wide search finds no stale references to deleted workflow names, release gates, evidence bundles, or required TestPyPI rehearsal.
15. The final pull request or implementation commit demonstrates a materially smaller job graph and documents which checks moved, merged, or were removed.

## Handoff guidance

Implementation agents should favor deletion and consolidation over translation of existing machinery into a new wrapper. A smaller YAML file that invokes a sprawling meta-script is not simplification. Likewise, moving evidence-bundle generation from CI into `make check` would preserve the original problem.

When deciding whether to retain an existing check, identify the concrete defect class it catches and whether another retained test catches the same class. If ownership cannot be stated clearly, move the check out of the mandatory path pending evidence.

The intended outcome is not minimal testing. It is a minimal, high-signal verification system whose cost is proportional to Eggsec's size and whose release process remains under explicit maintainer control.
