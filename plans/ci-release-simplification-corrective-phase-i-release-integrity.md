# Corrective Phase I Plan: Release Integrity and Evidence Closure

## Status

Ready for implementation. This phase reopens the CI/manual-release simplification closure only for the narrow defects identified after Corrective Phases G and H.

Corrective Phase H is accepted as structurally complete. The compact workflow and Make command surface must be preserved. The remaining work is confined to release-validation integrity, one Python parity-test regression, lockfile review, version-bump correctness, and truthful closure evidence.

## Objective

Finish the manual-release simplification without reintroducing automation or verification bloat.

The final state must ensure that:

- `make release-check` cannot convert a failed Rust package operation into success;
- every intended crates.io archive is created and inspected deterministically before publication;
- registry-dependent first-release limitations are reported accurately and handled through the staged manual publication procedure;
- Python sync/async parity tests continue to compare meaningful semantic outcomes;
- `Cargo.lock` contains no unexplained third-party dependency churn;
- the next release version can be updated without leaving stale internal dependency requirements;
- closure evidence identifies the exact commit, host, command outcomes, and hosted CI run evidence actually observed.

No package may be published while executing this plan.

## Accepted baseline that must not regress

The following outcomes are already correct and are out of scope for redesign:

- `.github/workflows/ci.yml` is the single mandatory workflow;
- the mandatory workflow owns Rust, Python, and narrow macOS/Windows portability jobs;
- `.github/workflows/deep-checks.yml` is the only optional workflow;
- Python CI invokes `make check-python` rather than duplicating its implementation;
- `test.yml`, release workflows, wheel-publish workflows, TestPyPI rehearsal automation, and external-target scan workflows remain deleted;
- hosted workflows do not publish packages, trigger releases from tags, or require registry credentials;
- `make check`, `make check-python`, and `make check-full` remain the canonical local verification commands;
- specialist Make targets use valid Cargo syntax;
- release cadence and publication remain manual maintainer decisions;
- no runtime, enforcement, API, or feature expansion is authorized.

Do not reopen Phases A through H except where this plan explicitly corrects their evidence or release behavior.

## Confirmed defects

### 1. False-positive Rust package validation

`scripts/release-check.sh` currently runs `cargo package` for each publishable crate. When Cargo reports `no matching package named`, the script treats the failed command as an expected first-release condition, increments a counter, continues, and ultimately reports successful Rust package validation.

This is not a successful package operation. It proves only that the failure string matched an expected registry-state limitation.

The current behavior creates two integrity problems:

- a failed package command is converted into an overall pass;
- the closure report states that every package was validated even though some package commands did not succeed.

A command returning non-zero must never be represented as `PASS`.

### 2. Pre-first-release archive validation and registry verification are conflated

A multi-crate first release has two distinct concerns:

1. deterministic local archive and manifest validation;
2. registry-dependent build/publish verification after internal dependencies are visible on crates.io.

Dependent crates may not complete Cargo's registry-backed package verification before their internal dependencies have been published. That limitation does not justify ignoring failures. The release process must separate the two concerns explicitly.

The local default release check should prove everything that can be proven without registry mutation. The staged manual publication procedure should prove registry resolution layer by layer before each actual publish command.

### 3. Python semantic parity test was weakened outside scope

`crates/eggsec-python/tests/test_stable_core_fixtures.py` was changed so normalized sync/async comparison ignores `is_vulnerable` in addition to timing-derived counters.

`is_vulnerable` is a semantic result, not a raw timing measurement. Removing it from parity comparison weakens the API contract. The accompanying comment claims aggregate counters cover it, but the same normalization also removes `waf_bypasses` and `time_anomalies`, so the meaningful outcome can disappear entirely from the comparison.

The test must again verify stable semantic equivalence. Timing values may remain normalized out; vulnerability and policy outcomes may not.

### 4. Large, unexplained `Cargo.lock` rewrite

Corrective Phase G changed a large portion of `Cargo.lock`, including:

- lockfile format version change;
- third-party package version changes;
- broad dependency-list rewrites.

Some lockfile format change may be justified by the declared Rust 1.80 MSRV, but adding registry versions to local path dependencies should not silently become a dependency-upgrade pass.

The final lockfile must be reviewed against pre-Phase-G commit `f26942dc37783ee302ffc5c67e58810cfdcc3578`. Every third-party version/source/checksum change must either be removed or explicitly justified as unavoidable for MSRV-compatible lockfile regeneration.

### 5. Version-bump instructions omit internal dependency versions

The release documentation currently instructs maintainers to update the workspace version and Python `pyproject.toml` version. Publishable manifests now contain repeated internal dependency versions such as:

```toml
eggsec-core = { path = "../eggsec-core", version = "0.1.0" }
```

On the next release, changing only the workspace and Python versions will leave stale dependency requirements and cause the package-graph validator to fail.

The procedure must include every internal path-plus-version entry, with a deterministic validation command that identifies any stale value.

### 6. Closure evidence exceeds independently recorded evidence

The closure report states that remote mandatory CI jobs completed successfully. The retained evidence does not identify a workflow run ID, URL, or job conclusions.

Direct local command output may establish local validation. It cannot establish hosted workflow success or branch-protection state. Hosted claims require hosted evidence. Unavailable evidence must be recorded as `NOT VERIFIED`, not inferred from a successful push or a documentation statement.

## Scope

Primary implementation files:

```text
scripts/release-check.sh
scripts/release-package-graph.py
scripts/test_release_package_graph.py
crates/eggsec-python/tests/test_stable_core_fixtures.py
Cargo.lock
docs/RELEASING.md
docs/VERIFICATION.md
AGENTS.md
crates/eggsec-python/VALIDATION.md
plans/ci-release-simplification-corrective-closure-index.md
plans/ci-verification-release-simplification-closure-report.md
plans/ci-release-simplification-corrective-phase-g-publishability.md
plans/ci-release-simplification-corrective-phase-h-ci-contract-closure.md
```

Manifest files under `crates/*/Cargo.toml` are in scope only if package inspection exposes a remaining publishability defect or the version-bump procedure requires a narrowly scoped correction.

Workflow YAML and the Makefile should not change unless validation reveals a direct regression from the accepted Phase H state.

## Workstream 1 — Reopen closure accurately

At the start of implementation, amend the corrective index and closure report.

Required status language:

```text
Reopened for Corrective Phase I. The CI simplification and Phase H workflow
contract remain complete. Final manual-release closure is pending correction of
the package-validation false positive, semantic parity regression, lockfile
review, version-bump procedure, and hosted evidence record.
```

Preserve prior evidence as historical context. Do not delete or rewrite earlier findings to make the sequence appear linear.

The report must distinguish:

- structural CI completion;
- deterministic local release validation;
- registry-dependent staged verification;
- actual publication, which remains outside this plan;
- hosted CI evidence;
- branch-protection evidence.

## Workstream 2 — Define the exact Rust release-validation contract

Document and implement three explicit verification levels.

### Level A: deterministic local archive validation

This is part of default `make release-check` and must not require internal crates to already exist on crates.io.

For every crates.io-publishable package in topological order:

1. create the package archive without registry-backed build verification;
2. fail if archive creation itself fails;
3. locate exactly one archive matching package name and version;
4. inspect the archive's normalized `Cargo.toml`;
5. verify package metadata and dependency rewrite invariants;
6. inspect the archive file list for excluded build/repository artifacts;
7. record archive path, size, and SHA-256.

Preferred lightweight Cargo command:

```bash
cargo package -p <crate> --no-verify
```

Before adopting this command, prove with a focused synthetic two-crate fixture that it creates the dependent archive when the dependency version is not present on crates.io. Record the observed Cargo behavior in tests or a concise code comment.

If the installed Cargo version still performs registry resolution during `--no-verify`, do not suppress the failure. Use the smallest deterministic alternative that produces Cargo's normalized package archive or package file list without adding a local registry service. Acceptable fallback components include:

- `cargo package --list` for package-content selection;
- Cargo metadata for dependency graph and versions;
- inspection of a successfully generated `.crate` archive where Cargo supports it;
- a temporary package-copy verification step owned by the existing helper.

Do not introduce a private registry daemon, containerized registry, or new release framework merely to close this phase.

### Level B: registry-sensitive dry-run verification

This is optional in the default local check and mandatory immediately before publishing each dependency layer.

Use:

```bash
cargo publish -p <crate> --dry-run
```

or an equivalent Cargo-supported registry preflight.

Rules:

- process crates in the topological order generated by the helper;
- only verify a dependent layer after its internal dependencies are visible on crates.io;
- a failure, timeout, or unavailable registry remains a failure or unavailable result;
- never translate `no matching package named` into success;
- retain full Cargo output in a readable log;
- never publish from the preflight command.

### Level C: manual publication

Actual `cargo publish` commands remain explicit maintainer actions and are never invoked by `release-check.sh`, tests, GitHub Actions, or this plan.

The documentation must show the staged sequence:

```text
validate local archives for all crates
publish dependency layer
wait for registry visibility
run dry-run for next layer
publish next layer
repeat
```

## Workstream 3 — Make archive inspection authoritative

Extend `scripts/release-package-graph.py` or add one small standard-library helper owned by it. Prefer extending the existing helper rather than creating another standalone subsystem.

Recommended commands:

```bash
python3 scripts/release-package-graph.py validate
python3 scripts/release-package-graph.py order
python3 scripts/release-package-graph.py inspect-archive <path-to-crate>
```

The exact command name may differ, but archive inspection must be deterministic and testable.

### Required packaged-manifest checks

For each generated `.crate` archive, parse the normalized packaged `Cargo.toml` and verify:

- package name equals the expected Cargo package;
- package version equals the workspace release version;
- no normal, build, target-specific normal, or target-specific build dependency retains a local `path` key;
- every internal dependency retained in the package has a registry version;
- internal dependency versions match the release policy;
- no publishable package depends on `eggsec-cli`, `eggsec-tui`, or `eggsec-python`;
- optional internal dependencies remain optional where intended;
- feature definitions still reference valid dependency names;
- package metadata identifies the expected repository, license, and Rust version.

### Required package-content checks

At minimum reject archive entries containing:

```text
.git/
target/
.venv/
.venv-ci/
dist/
*.pcap
*.pcapng
exports/
```

Do not apply a generic secret scanner or large evidence framework. The objective is direct package hygiene, not supply-chain process expansion.

Require the configured README and license files to be present when Cargo includes them.

### Dependency-section coverage

The validator currently inspects top-level dependency tables. Review and support or explicitly reject internal path dependencies under:

```toml
[target.'cfg(...)'.dependencies]
[target.'cfg(...)'.build-dependencies]
```

Also handle dependency aliases using `package = "eggsec-..."` correctly. Graph ownership must follow the actual package name, while diagnostics should show the manifest key and package name.

Do not add broad TOML rewriting; validation is sufficient.

## Workstream 4 — Correct `release-check.sh` result semantics

Remove the success path that catches `no matching package named` from a failed `cargo package` command and increments an "expected" counter.

Required behavior:

- every command executed by the script has an explicit outcome;
- non-zero archive creation stops the release check;
- optional registry preflight non-zero exits stop that preflight and return non-zero;
- skipped registry preflight is labeled `SKIPPED`, not `PASS`;
- no line says "all packages validated" unless all required deterministic archive checks passed;
- the final summary states exactly what was validated.

Recommended summary shape:

```text
Rust package archives: PASS (12/12 created and inspected)
Registry preflight: SKIPPED (enable explicitly; required during staged publish)
Python wheel/sdist: PASS
Fresh-wheel smoke: PASS
No artifacts were published.
```

### Artifact isolation

Use one temporary release root for Rust package archives, logs, Python artifacts, and smoke virtual environments where practical.

Do not inspect stale archives under a prior `target/package` or `dist` directory. Before each package operation:

- remove or isolate prior output for that package/version;
- identify the newly created archive deterministically;
- fail on zero or multiple unexpected matches.

`EGGSEC_RELEASE_KEEP_ARTIFACTS=1` may preserve the temporary root, but the script must print its path before exiting.

### Exit status and logging

Under `set -euo pipefail`:

- preserve pipeline failure status;
- retain full logs through `tee` where useful;
- print the failing stage, package, command class, and log path;
- do not use broad `grep` error classification as an alternative to exit status.

## Workstream 5 — Add focused release-integrity tests

Extend `scripts/test_release_package_graph.py` with narrow tests. Do not add pytest or another dependency solely for this helper.

Required tests:

1. archive inspection accepts a normalized internal registry dependency;
2. archive inspection rejects a retained internal `path` dependency;
3. archive inspection rejects an internal dependency version mismatch;
4. target-specific internal dependencies are detected;
5. aliased internal dependencies resolve to the correct package node;
6. optional internal dependencies remain represented in ordering and inspection;
7. prohibited archive entries are rejected;
8. missing expected README/license metadata is reported clearly when applicable;
9. deterministic archive selection rejects stale or ambiguous matches;
10. a simulated failed package command cannot produce a successful release summary;
11. `no matching package named` remains non-zero and is never classified as pass;
12. the real workspace graph still validates and orders successfully.

Add a synthetic first-release fixture that demonstrates the selected deterministic archive-generation method. The fixture should use temporary manifests and package names that cannot accidentally resolve to real crates.io packages.

Do not make tests contact crates.io.

## Workstream 6 — Restore semantic Python parity

In `crates/eggsec-python/tests/test_stable_core_fixtures.py`:

1. remove `is_vulnerable` from the globally ignored normalization fields;
2. run the focused sync/async parity test repeatedly to identify the operation producing nondeterminism;
3. keep raw durations, rates, latency percentiles, and other truly time-dependent measurements excluded;
4. preserve comparison of semantic booleans, policy outcomes, finding categories, counts, and target identity;
5. make the fixture deterministic for the affected operation where possible;
6. if one timing-sensitive operation cannot produce a stable exact dictionary, give it a targeted semantic assertion rather than removing the semantic field globally.

Examples of acceptable targeted assertions:

```text
sync.is_vulnerable == async.is_vulnerable
sync target == async target
sync finding categories == async finding categories
both results satisfy the same success/failure policy outcome
raw elapsed/latency fields may differ
```

If sync and async paths genuinely return different semantic vulnerability outcomes under the same deterministic fixture, treat that as a product bug and fix the narrow dispatch/result issue. Do not mask it in normalization.

Required validation:

```bash
cd crates/eggsec-python
pytest tests/test_stable_core_fixtures.py -q
pytest tests/test_stable_core_fixtures.py::test_stable_core_sync_async_normalized_equivalence -q --count-or-loop-equivalent
```

Do not add a repeat-test plugin. Repeat with a small shell loop if needed:

```bash
for i in 1 2 3 4 5; do
  pytest tests/test_stable_core_fixtures.py::test_stable_core_sync_async_normalized_equivalence -q || exit 1
done
```

Then run the canonical full Python command:

```bash
make check-python
```

## Workstream 7 — Review and minimize `Cargo.lock` changes

Use pre-Phase-G commit `f26942dc37783ee302ffc5c67e58810cfdcc3578` as the comparison baseline.

Produce a package-level comparison containing:

```text
package name
old version/source/checksum
new version/source/checksum
reason for change
```

### Allowed changes

- lockfile format change required to preserve compatibility with the declared Rust 1.80 MSRV;
- workspace-package dependency-list normalization caused directly by the new internal dependency metadata;
- a third-party change proven unavoidable when regenerating an MSRV-compatible lockfile, with explicit documentation.

### Disallowed changes

- opportunistic third-party upgrades;
- dependency refresh caused by running unrestricted `cargo update`;
- checksum/source changes without a package-version reason;
- unrelated feature or dependency edits.

### Preferred correction procedure

1. restore the pre-Phase-G lockfile into a temporary comparison copy;
2. determine whether only the lockfile format prevents Rust 1.80 use;
3. regenerate with the intended MSRV-compatible Cargo toolchain or a toolchain that honors `rust-version`;
4. preserve previous third-party versions wherever resolution permits;
5. use targeted `cargo update -p <package> --precise <old-version>` only when needed to restore an unintended change;
6. run all checks with `--locked` where supported.

Do not blindly restore lockfile format version 4 if Cargo 1.80 cannot consume it. The final choice must honor the declared MSRV and minimize dependency movement.

Required validation:

```bash
cargo metadata --locked --format-version 1 --no-deps >/dev/null
cargo check --workspace --no-default-features --locked
make check
make check-python
```

Record any retained third-party version change in the closure report. A statement such as "Cargo.lock regenerated" is not sufficient evidence.

## Workstream 8 — Correct the version-bump workflow

Keep the process lightweight. Do not introduce a release bot or TOML-writing framework.

Update `docs/RELEASING.md` so a version bump includes:

1. workspace package version in root `Cargo.toml`;
2. Python project version in `crates/eggsec-python/pyproject.toml`;
3. every version-qualified internal path dependency in publishable manifests;
4. lockfile regeneration only as required;
5. package-graph validation;
6. deterministic archive validation;
7. canonical release checks.

Extend `release-package-graph.py validate` diagnostics so a stale internal dependency version reports:

```text
manifest path
manifest dependency key
actual internal package name
found version
expected release version
```

Optionally add a read-only command:

```bash
python3 scripts/release-package-graph.py version-locations
```

It may print every file and line requiring update. Prefer a read-only inventory over an auto-rewriter unless maintainers explicitly request automation later.

Add a fixture test that changes the synthetic workspace version and proves stale internal dependency versions are rejected until updated.

Acceptance does not require performing a real repository version bump.

## Workstream 9 — Make closure evidence auditable

The closure report must use only:

```text
PASS
FAIL
SKIPPED
NOT RUN
NOT VERIFIED
BLOCKED
TIMEOUT
```

Only `PASS` satisfies a blocking local criterion.

### Local evidence

For each command record:

- exact command;
- final implementation commit SHA;
- operating system and architecture;
- exit status;
- concise result summary;
- relevant artifact/log location when retained.

Required local commands:

```bash
python3 scripts/test_release_package_graph.py
python3 scripts/release-package-graph.py validate
python3 scripts/release-package-graph.py order
make check
make check-python
make check-full
make release-check
```

Also rerun the corrected focused Python parity test five consecutive times.

### Hosted evidence

After pushing the final implementation commit, record:

- GitHub Actions workflow run URL or numeric run ID;
- commit SHA evaluated by the run;
- `Rust` job conclusion;
- `Python` job conclusion;
- macOS portability conclusion;
- Windows portability conclusion.

If the API or UI evidence cannot be obtained, use `NOT VERIFIED`. Do not infer hosted success from local results or from the existence of a commit.

### Branch protection

Branch-protection state is separate from workflow success. Record it only when settings were actually inspected. Otherwise state:

```text
Branch protection: NOT VERIFIED; repository settings were not available to the implementation environment.
```

Do not block this narrow technical closure on unavailable branch-protection API access unless maintainers explicitly require it, but do not claim it passed.

## Implementation sequence

1. Reopen the corrective index and closure report for Phase I.
2. Reproduce the current `release-check` false-positive path and retain a concise failure example.
3. Add the synthetic first-release archive-generation fixture.
4. Choose and document the deterministic archive-generation command.
5. Extend package-graph/archive inspection and its focused tests.
6. Rewrite Rust package validation in `release-check.sh` so failures remain failures.
7. Separate default archive validation from staged registry preflight.
8. Update release documentation with the three-level contract and staged first-release sequence.
9. Restore semantic comparison of `is_vulnerable`; make the affected fixture deterministic or add targeted semantic assertions.
10. Compare and minimize `Cargo.lock` against `f26942dc37783ee302ffc5c67e58810cfdcc3578`.
11. Correct version-bump documentation and validator diagnostics.
12. Run helper tests and focused parity tests.
13. Run the full local validation sequence on the final implementation commit.
14. Push the implementation commit.
15. Inspect and record hosted CI run evidence using an actual run URL or ID.
16. Update the closure report with exact outcomes.
17. Mark the corrective index and closure report complete only if all blocking criteria pass.

## Required validation sequence

### Package graph and helper tests

```bash
python3 scripts/test_release_package_graph.py
python3 scripts/release-package-graph.py list
python3 scripts/release-package-graph.py validate
python3 scripts/release-package-graph.py order
```

### Package archives

Run the final deterministic archive command for every generated package in helper order. Then inspect every archive through the helper.

Expected result:

```text
12/12 intended crates.io archives created
12/12 normalized manifests inspected
0 retained internal path dependencies
0 private runtime/build dependencies
0 version mismatches
0 prohibited archive entries
```

Use the actual intended package count from metadata if the package set changes deliberately; do not hard-code twelve into implementation logic.

### Python parity

```bash
for i in 1 2 3 4 5; do
  (cd crates/eggsec-python && \
   pytest tests/test_stable_core_fixtures.py::test_stable_core_sync_async_normalized_equivalence -q) || exit 1
done
```

Then:

```bash
make check-python
```

### Lockfile/MSRV

```bash
cargo metadata --locked --format-version 1 --no-deps >/dev/null
cargo check --workspace --no-default-features --locked
```

Run with Rust 1.80 if that toolchain is available in the implementation environment. If it is unavailable, record `NOT RUN` for the direct MSRV execution and do not claim it passed.

### Canonical commands

```bash
make check
make check-python
make check-full
make release-check
```

### Static release-policy searches

```bash
rg -n 'cargo publish|maturin publish|twine upload|id-token:\s*write|tags:' .github/workflows
rg -n 'no matching package named' scripts/release-check.sh scripts/release-package-graph.py
rg -n 'is_vulnerable' crates/eggsec-python/tests/test_stable_core_fixtures.py
rg -n 'version\s*=\s*"0\.1\.0"' crates/*/Cargo.toml docs/RELEASING.md
```

Interpret matches rather than requiring zero for documentation/manual publication commands. Hosted workflow publication searches must remain empty.

## Acceptance criteria

### Release-validation integrity

- A failed `cargo package`, archive-generation, archive-inspection, or registry-preflight command cannot produce an overall success result.
- `no matching package named` is never treated as `PASS`.
- Default `make release-check` creates and inspects an archive for every intended crates.io package without publishing.
- Every generated archive's normalized manifest has no retained local path for normal/build/target runtime dependencies.
- Every retained internal dependency has the expected registry version.
- Private crates do not appear as published runtime/build dependencies.
- Package archives contain no prohibited repository/build artifacts.
- Archive selection cannot silently use stale artifacts.
- Registry preflight is clearly separated, optional in the default local check, and required during staged manual publication.
- Registry-preflight failures and timeouts remain non-zero outcomes.
- The final release-check summary distinguishes archive validation from registry preflight.
- No package is published during implementation or validation.

### Package-graph helper

- Focused helper tests pass without network access.
- Target-specific internal dependencies are validated or explicitly rejected.
- Dependency aliases resolve to actual package names.
- Real-workspace validation and ordering pass.
- Version-mismatch diagnostics identify manifest, dependency key, package, found version, and expected version.

### Python parity

- `is_vulnerable` is not globally removed from semantic sync/async comparison.
- Raw timing fields may remain normalized out.
- Five consecutive focused parity-test runs pass.
- `make check-python` passes.
- No runtime/API behavior is changed merely to silence the test unless a genuine semantic bug is identified and narrowly fixed.

### Lockfile

- `Cargo.lock` is compared against pre-Phase-G commit `f26942dc37783ee302ffc5c67e58810cfdcc3578` at package level.
- Unrelated third-party dependency upgrades are reverted.
- Any retained third-party change has an explicit technical reason.
- Final lockfile format is compatible with the declared MSRV or the MSRV claim is separately corrected with maintainer approval.
- `cargo metadata --locked` and no-default workspace checking pass.

### Version-bump procedure

- `docs/RELEASING.md` includes internal dependency version updates.
- The graph validator catches stale internal dependency versions after a synthetic version change.
- The procedure remains manual and dependency-light.
- No release bot or auto-publish mechanism is introduced.

### Evidence and closure

- The corrective index and closure report are reopened before implementation claims completion.
- Local evidence references the final implementation commit and host.
- Hosted CI success claims include an actual workflow run URL or ID and job conclusions.
- Unavailable hosted or branch-protection evidence is labeled `NOT VERIFIED`.
- `make check`, `make check-python`, `make check-full`, and `make release-check` all return zero on the final implementation commit.
- The final report does not represent skipped registry preflight or unavailable evidence as pass.
- The report may return to `Complete` only after every blocking Phase I criterion is `PASS`.

## Explicit non-goals

This phase does not include:

- publishing any crate or Python package;
- reintroducing automated crates.io or PyPI publication;
- changing release cadence;
- adding a local registry service or registry container unless maintainers separately authorize it after the lightweight approach is proven impossible;
- adding provenance, signing, SBOM, attestation, or release-bot infrastructure;
- broad dependency upgrades;
- changing crate boundaries;
- expanding CI matrices;
- adding cross-platform Python wheel automation;
- fixing unrelated compiler warnings;
- redesigning Python APIs;
- weakening enforcement or scope policy;
- changing product semantics to satisfy release tooling;
- creating another generalized evidence framework.

## Rollback strategy

If deterministic archive creation cannot be achieved with the installed Cargo behavior without registry access:

1. keep the current path-plus-version manifest corrections;
2. remove the false-success classification;
3. fail `make release-check` honestly for the unsupported stage;
4. document staged per-layer dry-run verification as the only supported initial-release path;
5. leave the closure report reopened rather than adding registry infrastructure automatically.

If lockfile minimization cannot preserve a specific prior third-party version while retaining Rust 1.80 compatibility, document the exact resolver constraint and retained change. Do not hide the difference.

If the parity test exposes a real sync/async semantic discrepancy, isolate and fix that defect in a separate narrow implementation commit, then rerun this phase's validation. Do not re-add semantic fields to the ignore list.

## Handoff guidance

Implement this phase in small commits with immediate validation:

1. closure reopening;
2. archive/helper tests;
3. release-check result correction;
4. Python parity correction;
5. lockfile minimization;
6. documentation/version workflow;
7. final evidence.

A smaller implementation model should not attempt to solve all workstreams in one edit. Run the focused test after each workstream. Preserve the accepted Phase H workflow shape. Prefer deletion of false-success logic and direct archive inspection over new abstractions.

The key closure rule is simple:

> A failed, skipped, timed-out, or unverified operation is never a pass.
