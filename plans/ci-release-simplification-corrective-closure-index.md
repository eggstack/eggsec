# CI and Manual Release Simplification — Corrective Closure Index

## Status


Status: Executed.
Reopened for Corrective Phase K.

Corrective Phase J implementation and hosted verification are complete. The
Cargo-native implementation commit is
`b91d9f91499ff7cabfc34a8bd9eed0e64e86af43`; hosted CI run
[`30636819135`](https://github.com/eggstack/eggsec/actions/runs/30636819135)
passed Rust, Python, macOS, and Windows jobs, and CodeQL run
[`30636818358`](https://github.com/eggstack/eggsec/actions/runs/30636818358)
passed Python analysis.

Corrective Phase K is a documentation-only polish pass. It must bind the final
local validation record to an accessible remote commit and distinguish the Rust
1.80 code MSRV from the separately validated Cargo release-tooling requirement.

Publication remains manual and has not been run.

## Purpose

This index tracks the narrow corrective sequence that followed the original CI
and release simplification roadmap. The implementation is complete. The only
remaining work is closure-record provenance and release-toolchain clarity.

No new roadmap expansion is authorized. Phase K must not reopen the accepted
workflow, package, runtime, Python API, dependency, or release architecture.

## Accepted baseline

The following outcomes are complete and must be preserved:

- automated PyPI, TestPyPI, crates.io, and GitHub Release publication is absent
  from hosted CI;
- tag pushes have no release side effects;
- the external-target self-scan workflow is deleted;
- the root GitLab consumer pipeline is relocated under `examples/`;
- `.github/workflows/ci.yml` is the single mandatory workflow;
- mandatory Rust CI is consolidated under `make check`;
- mandatory Python CI invokes `make check-python` and builds the extension once;
- `.github/workflows/deep-checks.yml` is the only optional diagnostic workflow;
- evidence-bundle, maturity-gate, skip-budget, and synthetic release-gate
  orchestration is removed;
- specialist Make targets use valid Cargo syntax;
- Python sync/async parity includes `is_vulnerable` and meaningful semantic
  outcomes;
- `Cargo.lock` is minimized relative to the pre-Phase-G baseline except for the
  justified `event-listener` security patch;
- internal publishable dependencies use local paths plus registry versions;
- the package order is derived from Cargo metadata;
- the version-bump procedure includes internal dependency versions;
- Cargo creates all release-validation `.crate` archives;
- the exact 12-package archive set is inventoried with path, size, and SHA-256;
- each archive passes standalone `cargo metadata --no-deps --offline` outside
  the source workspace;
- custom tar creation, regex manifest normalization, and shell-owned archive
  selection are removed;
- hosted CI and CodeQL evidence use actual run IDs and conclusions;
- publication cadence is manual and maintainer-controlled.

Corrective work must not restore deleted workflow graphs, publishing automation,
evidence bureaucracy, or runtime scope expansion.

## Completed corrective phases

### Corrective Phase G — Publishability and release-check restructuring

Plan: [`ci-release-simplification-corrective-phase-g-publishability.md`](ci-release-simplification-corrective-phase-g-publishability.md)

Accepted outcomes:

- private crates have explicit `publish = false`;
- publishable internal dependencies have local paths plus registry versions;
- publication order is derived from Cargo metadata;
- default local validation is separated from registry-sensitive preflight;
- release publication remains manual.

### Corrective Phase H — CI contract and command consolidation

Plan: [`ci-release-simplification-corrective-phase-h-ci-contract-closure.md`](ci-release-simplification-corrective-phase-h-ci-contract-closure.md)

Accepted outcomes:

- Rust, Python, and portability jobs are consolidated in `ci.yml`;
- `test.yml` is deleted;
- hosted Python verification invokes the canonical local command;
- optional feature profiles have one owner;
- invalid specialist Make targets are corrected;
- active documentation reflects the compact workflow shape.

### Corrective Phase I — Result integrity, parity, lockfile, and evidence

Plan: [`ci-release-simplification-corrective-phase-i-release-integrity.md`](ci-release-simplification-corrective-phase-i-release-integrity.md)

Accepted outcomes:

- the prior `PACKAGE_FIRST_RELEASE` false-success branch is removed;
- failed package-stage commands are never converted into success through error
  string matching;
- registry preflight is reported as `SKIPPED` when not run;
- `is_vulnerable` is restored to semantic Python parity comparison;
- the lockfile delta is reduced to the targeted `event-listener` security patch;
- version-bump guidance includes all version-qualified internal path
  dependencies;
- hosted workflow run `30632663714` records successful Rust, Python, macOS, and
  Windows jobs;
- branch-protection state remains honestly recorded as `NOT VERIFIED`.

Phase I's handwritten archive fallback was superseded by Phase J. Its other
outcomes remain accepted.

### Corrective Phase J — Cargo-native package archive closure

Plan: [`ci-release-simplification-corrective-phase-j-cargo-native-packaging.md`](ci-release-simplification-corrective-phase-j-cargo-native-packaging.md)

Accepted outcomes:

- Cargo itself creates every `.crate` archive used for release validation;
- the active command is:

  ```bash
  cargo package --workspace --no-verify --target-dir <isolated-target> \
    --exclude eggsec-cli --exclude eggsec-tui --exclude eggsec-python
  ```

- custom tar creation and regex manifest normalization are removed;
- workspace-inherited package and dependency metadata is normalized by Cargo;
- each extracted archive passes standalone
  `cargo metadata --no-deps --offline`;
- the exact expected archive set is enforced;
- aliases, optional dependencies, target-specific dependencies, feature
  references, private packages, metadata, README/license files, and prohibited
  entries are inspected;
- every Rust archive records path, size, and SHA-256;
- Cargo packaging failures remain failures;
- `release-check.sh` no longer owns archive selection through `mapfile` or stale
  filesystem discovery;
- active documentation describes the command that actually runs;
- registry preflight remains a separate staged maintainer operation;
- hosted CI run `30636819135` and CodeQL run `30636818358` are verified.

Phase J implementation is complete. Post-implementation review found only a
closure-record SHA inconsistency and release-tooling documentation gap.

## Remaining polish phase

### Corrective Phase K — Final evidence and release-toolchain polish

Plan: [`ci-release-simplification-corrective-phase-k-evidence-toolchain-polish.md`](ci-release-simplification-corrective-phase-k-evidence-toolchain-polish.md)

Primary required outcomes:

- run the complete final local validation sequence against one clean, exact
  commit that exists in remote history;
- record the full validation SHA consistently;
- remove the inaccessible `130c233` reference from active final evidence;
- record `rustc --version --verbose`, `cargo --version --verbose`, and the
  relevant Python version for the final evidence run;
- retain `b91d9f91499ff7cabfc34a8bd9eed0e64e86af43` as the Phase J implementation
  commit;
- retain hosted CI run `30636819135` and CodeQL run `30636818358` as verified
  hosted evidence;
- correct final closure prose so Phase J/Phase K, not Phase I, owns the final
  gate record;
- document Rust 1.80 as the code MSRV separately from the Cargo version required
  for the validated maintainer release operation;
- retain the observed Cargo 1.80.1 workspace-packaging failure without treating
  it as a code-MSRV failure;
- keep Linux as the tested release host and macOS release-script compatibility
  as unverified;
- make no implementation, workflow, manifest, lockfile, dependency, or runtime
  changes;
- publish nothing.

Phase K is the only remaining closure item. It is not an implementation blocker.

## Current evidence classification

Until Phase K records the final local evidence against an accessible commit, use:

```text
CI workflow simplification: PASS
Python semantic parity: PASS
Lockfile minimization: PASS
Version-bump procedure: PASS
Cargo-native Rust archive implementation: PASS
Cargo-native Rust archives: PASS (12/12 in prior local run)
Final local validation commit provenance: NOT VERIFIED
Hosted CI run 30636819135: PASS (Rust, Python, macOS, and Windows)
CodeQL run 30636818358: PASS
Branch protection: NOT VERIFIED
Registry preflight: SKIPPED
Publication: NOT RUN
```

The `NOT VERIFIED` classification applies only to the final local evidence SHA
record. It does not invalidate the accepted Phase J implementation or hosted
run conclusions.

## Phase K sequencing rules

1. Reopen the closure report before claiming replacement final evidence.
2. Do not modify the accepted Phase J implementation.
3. Select one exact clean commit for final local validation.
4. Record full Git, Rust, Cargo, Python, host, and architecture information.
5. Run the package-helper tests, graph validation, locked metadata, `make check`,
   `make check-python`, `make check-full`, and `make release-check` against that
   same commit.
6. Remove `130c233` from active final evidence.
7. Distinguish the Rust 1.80 code MSRV from the tested release-tool Cargo
   version.
8. Do not raise the code MSRV or add a Cargo gate to ordinary builds.
9. Keep registry preflight `SKIPPED` unless explicitly run.
10. Keep publication `NOT RUN`.
11. Mark Phase K complete only after all blocking local gates pass and the final
    documentation is internally consistent.
12. Do not create another phase absent a newly demonstrated implementation
    defect.

## Phase K acceptance criteria

The corrective line is fully closed only when all of the following are true:

1. Phase J implementation files remain unchanged.
2. Final local validation is executed against one exact clean commit.
3. The validation commit exists in remote history.
4. The full validation SHA is recorded consistently.
5. `130c233` is absent from active final evidence.
6. Git working-tree cleanliness is recorded before validation.
7. Host and architecture are recorded.
8. `rustc --version --verbose` is recorded.
9. `cargo --version --verbose` is recorded.
10. `python3 --version` is recorded where Python gates are claimed.
11. Package-helper tests pass.
12. The real package graph validates and orders successfully.
13. Locked Cargo metadata succeeds.
14. `make check` passes.
15. `make check-python` passes.
16. `make check-full` passes.
17. `make release-check` passes.
18. The release check confirms 12/12 Cargo-generated archives.
19. Registry preflight is `SKIPPED` unless actually run.
20. Publication is `NOT RUN`.
21. Hosted CI run `30636819135` remains accurately recorded.
22. CodeQL run `30636818358` remains accurately recorded.
23. Branch protection remains `NOT VERIFIED` unless directly inspected.
24. Final closure prose identifies Phase J/Phase K as the final gate owner.
25. `docs/RELEASING.md` distinguishes the Rust 1.80 code MSRV from the
    maintainer release-tooling Cargo requirement.
26. The exact successful Cargo version is documented.
27. Cargo 1.80.1's package-command limitation is recorded without being
    misrepresented as a code-MSRV failure.
28. Linux remains the tested release host.
29. macOS release-script compatibility remains unclaimed.
30. No workflow, implementation script, manifest, lockfile, dependency,
    runtime, API, or feature changes are introduced.
31. No package, tag, or GitHub Release is published.
32. The closure report and this index are marked complete only after the final
    evidence table is updated.

## Explicit exclusions

Phase K does not include:

- changing the Rust code MSRV;
- adding a Cargo version gate to ordinary builds or CI;
- modifying the Phase J package helper or release script;
- adding or redesigning hosted workflows;
- publishing a release;
- running registry preflight solely for additional evidence;
- adding a local registry, Docker, signing, provenance, SBOM, attestation, or
  release bots;
- dependency upgrades;
- crate reorganization;
- runtime or public API changes;
- another evidence framework;
- broader test matrices.

## Handoff requirement

Implementation agents must treat command results as evidence, not documentation
assertions.

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

A failed, skipped, timed-out, blocked, or unverified operation is never a pass.
The final closure update must include the full implementation SHA, full local
validation SHA, validation host, Rust/Cargo/Python versions, exact local command
outcomes, package count, hosted run evidence, branch-protection classification,
and confirmation that no publication occurred.
