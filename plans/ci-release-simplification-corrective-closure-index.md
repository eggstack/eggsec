# CI and Manual Release Simplification — Corrective Closure Index

## Status

Open. The original six-phase simplification substantially reduced CI and release-process complexity, but the line of work is not closed. The retained closure report records useful structural outcomes, yet its completion claim is superseded by this corrective index until the plans below are implemented and validated.

## Purpose

This index isolates the remaining work discovered after implementation. The broad reduction effort should not be reopened. The remaining defects are narrow and fall into two ownership areas:

1. the manual Rust release path is not yet demonstrably publishable or reliably validated;
2. the compact CI and local command contract contains several consistency, duplication, and evidence-reporting defects.

No new roadmap expansion is authorized. The objective is to close the existing simplification work with direct, reproducible evidence.

## Current accepted baseline

The following outcomes are considered landed and must be preserved:

- automated PyPI, TestPyPI, crates.io, and GitHub Release publication is absent from hosted CI;
- tag pushes have no release side effects;
- the external-target self-scan workflow is deleted;
- the root GitLab consumer pipeline is relocated under `examples/`;
- routine Rust CI is consolidated under `make check`;
- routine Python CI builds the extension once;
- evidence-bundle, maturity-gate, skip-budget, and synthetic release-gate orchestration is removed;
- one optional non-publishing deep-check workflow remains;
- publication cadence is manual and maintainer-controlled.

Corrective implementation must not restore deleted workflow graphs or publishing automation.

## Remaining blockers

### Manual crates.io release path

The documentation lists a multi-crate crates.io release, but workspace crates currently use local path-only dependencies in their manifests. Publishable crates require registry-resolvable dependency specifications. The documented dependency order is also not derived from the actual workspace graph.

The local `release-check` script has not completed successfully. It duplicates `cargo package` and `cargo publish --dry-run`, discovers crates in filesystem order rather than dependency order, suppresses useful diagnostics, and contains portability assumptions that are not suitable for the primary macOS maintainer environment.

### CI and command contract

Several specialist Make targets were mechanically translated from nextest syntax and are invalid under `cargo test`. The optional deep workflow executes feature-profile checks twice. Python CI duplicates the commands in `scripts/check-python.sh` rather than invoking the canonical local target. The roadmap required one mandatory workflow, while the implementation retains separate Rust and Python workflows without documenting that deviation as an intentional amendment.

Operational documentation still states release-readiness requirements that are not implemented by current commands. The closure report marks unmet criteria as complete and must be corrected after direct validation.

## Corrective sequence

### Corrective Phase G — Publishability and release-check closure

Plan: [`ci-release-simplification-corrective-phase-g-publishability.md`](ci-release-simplification-corrective-phase-g-publishability.md)

Primary outcome:

- every crate intentionally published to crates.io has publishable manifest metadata;
- the publish order is generated or validated from the actual dependency graph;
- local dry-run packaging succeeds in that order;
- `make release-check` completes on Linux and macOS without publishing;
- release documentation matches the validated package set and process.

This phase is a blocker for any release-readiness claim.

### Corrective Phase H — CI contract, command, and closure consistency

Plan: [`ci-release-simplification-corrective-phase-h-ci-contract-closure.md`](ci-release-simplification-corrective-phase-h-ci-contract-closure.md)

Primary outcome:

- all advertised Make targets execute valid Cargo commands;
- hosted Python CI invokes the same canonical command as local validation;
- optional diagnostics do not duplicate work;
- workflow structure and documentation agree;
- release-readiness requirements are executable rather than aspirational;
- the closure report records only commands that actually completed.

Phase H depends on Phase G's final release-check interface and evidence.

## Sequencing rules

1. Implement Phase G first.
2. Do not amend the closure report to `Complete` until the complete Phase G validation sequence succeeds.
3. Phase H may correct independent Makefile and workflow issues while Phase G is in progress, but final documentation and closure evidence must be based on the completed Phase G interface.
4. Keep implementation commits narrowly scoped. Manifest publishability changes should not be mixed with unrelated dependency upgrades.
5. Do not publish any crate or Python package while executing either corrective plan.
6. Do not push a release tag as a validation technique.

## Roadmap-level corrective acceptance criteria

The corrective sequence is complete only when all of the following are true:

1. Every crate listed as publishable has a valid crates.io package manifest.
2. Every internal normal/build dependency of a publishable crate includes a compatible registry version as well as its local path, unless the dependent crate is intentionally excluded from publication.
3. The documented crate publication order is topologically valid for the actual package graph.
4. `cargo package` or `cargo publish --dry-run` succeeds for every intended Rust package in validated order.
5. `make release-check` completes end-to-end without publishing.
6. `make release-check` works on both Linux and macOS, or the repository explicitly designates and documents one supported release host with a justified reason.
7. Package-validation output retains enough Cargo diagnostics to identify the failing crate and cause.
8. `make test-ci`, `make test-integration`, and `make test-slow` use valid `cargo test` syntax and pass or are removed from the advertised interface.
9. `deep-checks.yml` does not execute representative feature profiles twice.
10. Python CI invokes `make check-python` or `scripts/check-python.sh` rather than maintaining a divergent duplicate command list.
11. The workflow inventory either satisfies the original one-mandatory-workflow requirement or explicitly amends that requirement with rationale and matching documentation.
12. `docs/VERIFICATION.md`, `docs/RELEASING.md`, `AGENTS.md`, the Makefile, workflow YAML, and the closure report describe the same executable contract.
13. The closure report does not mark partial or timed-out validation as passed.
14. Hosted CI remains non-publishing and no external-target scan workflow returns.
15. No runtime behavior, enforcement posture, Python API maturity, or feature semantics are changed to close process defects.

## Explicit exclusions

This corrective sequence does not include:

- publishing a release;
- changing release cadence back to automation;
- reintroducing wheel matrices on ordinary pushes;
- fixing unrelated all-feature compilation defects unless they are explicitly retained as release gates;
- broad dependency upgrades;
- reorganizing crate boundaries;
- adding provenance, attestation, signing, or release-bot infrastructure;
- adding another evidence framework;
- expanding product functionality.

## Handoff requirement

Implementation agents must treat command execution results as evidence, not documentation assertions. Any unavailable command must be recorded as unavailable or failed. A timeout is not a pass. The final closure update must include the exact package set, dependency order, validation host, commands, and outcomes.