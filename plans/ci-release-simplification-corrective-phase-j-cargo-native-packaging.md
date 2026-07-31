# Corrective Phase J Plan: Cargo-Native Package Archive Closure

## Status

Ready for implementation.

Corrective Phase I fixed the prior false-success branch, restored Python semantic parity, minimized the lockfile, corrected version-bump guidance, and recorded real hosted CI evidence. Post-implementation review found one remaining release-integrity defect: the default release check constructs handwritten tar archives rather than validating Cargo-generated package archives.

This phase reopens only the manual Rust package-archive portion of the CI/release simplification closure. Corrective Phase H and the compact hosted CI contract remain accepted and must not be redesigned.

## Objective

Replace the handwritten package-archive fallback with a Cargo-native, registry-independent packaging path that produces the same normalized `.crate` artifacts Cargo would publish.

The final state must prove all of the following without publishing:

- every intended crates.io package is assembled by Cargo, not by a custom tar writer;
- workspace-inherited package and dependency metadata is concretized by Cargo;
- every generated archive is a valid standalone Cargo package manifest;
- generated archive selection is exact, isolated, and free of stale artifacts;
- archive inspection covers dependency aliases, target-specific dependencies, optional dependencies, feature references, package metadata, prohibited files, size, and SHA-256;
- any Cargo packaging failure remains a failure;
- registry-dependent verification remains a separate staged maintainer operation;
- closure documentation describes the implementation that actually runs.

No package, tag, or GitHub Release may be published while executing this plan.

## Accepted baseline that must not regress

The following outcomes are complete and out of scope for redesign:

- `.github/workflows/ci.yml` is the single mandatory workflow;
- `.github/workflows/deep-checks.yml` is the only optional workflow;
- Rust, Python, macOS portability, and Windows portability jobs remain compact;
- hosted Python CI invokes `make check-python`;
- hosted workflows do not publish, trigger releases from tags, or scan external targets;
- specialist Make targets use valid Cargo syntax;
- `is_vulnerable` remains part of Python sync/async semantic parity;
- the lockfile remains minimized relative to the pre-Phase-G baseline, except for the justified `event-listener` security patch;
- internal publishable dependencies retain `path` plus `version` metadata;
- publication order remains derived from Cargo metadata;
- publication cadence remains manual;
- no runtime, enforcement, API, feature, or crate-boundary expansion is authorized.

Do not reopen Phases A through I except where this plan corrects package-archive claims and final closure evidence.

## Confirmed defects

### 1. The current `.crate` files are handwritten archives

`scripts/release-package-graph.py create-archive` currently:

1. runs `cargo package --list`;
2. reads the selected source files;
3. rewrites selected lines in `Cargo.toml` with regular expressions;
4. creates a gzip tar archive with Python `tarfile`.

This is not Cargo package assembly. It does not include all of Cargo's normalization behavior, package metadata generation, lockfile handling, VCS metadata, symlink behavior, or manifest transformation rules.

### 2. Workspace-inherited dependencies remain unresolved

The custom normalizer expands selected package fields such as:

```toml
version.workspace = true
license.workspace = true
```

but it does not resolve dependency entries such as:

```toml
serde = { workspace = true }
tokio = { workspace = true }
rustc-hash = { workspace = true }
```

The generated archive does not contain the workspace root that defines `[workspace.dependencies]`. Its packaged manifest is therefore not a valid standalone package manifest even though the custom inspector accepts it.

### 3. The inspector does not ask Cargo to parse the extracted package

The current inspector parses TOML itself and checks selected invariants. It does not extract the archive and run Cargo metadata against the packaged manifest. As a result, unresolved workspace inheritance and other Cargo-level manifest defects are not detected.

### 4. Documentation claims a different implementation

Active documentation says Level A uses `cargo package --no-verify` and inspects Cargo-normalized archives. The implementation uses `cargo package --list` plus handwritten archive generation.

The architecture and release documentation therefore overstate what is proven.

### 5. Rust archive inventory is incomplete

The release check reports size and SHA-256 for Python artifacts but not for each generated Rust archive, despite Phase I requiring archive path, size, and digest evidence.

### 6. Focused release-integrity tests remain incomplete

The current tests do not fully cover:

- workspace-inherited dependencies inside generated archives;
- Cargo standalone parsing of extracted archives;
- target-specific dependencies inside packaged manifests;
- aliased internal dependencies inside packaged manifests;
- optional dependency and feature-reference preservation;
- stale or ambiguous archive selection;
- simulated Cargo packaging failure propagating through the actual package command path;
- missing license-file behavior;
- Rust archive size and digest inventory;
- macOS stock Bash compatibility or an explicit Linux-only policy.

### 7. The script uses `mapfile`

`release-check.sh` uses Bash `mapfile` to select archives. The repository designates Linux as the supported release host, but active documentation also says the script should work on macOS. Stock macOS Bash 3.2 does not provide `mapfile`.

Either remove the unsupported portability claim or remove the `mapfile` dependency. Prefer moving archive enumeration into the Python helper so the shell no longer owns this detail.

## Scope

Primary implementation files:

```text
scripts/release-check.sh
scripts/release-package-graph.py
scripts/test_release_package_graph.py
docs/RELEASING.md
docs/VERIFICATION.md
architecture/overview.md
AGENTS.md
.opencode/skills/eggsec-python/SKILL.md
crates/eggsec-python/VALIDATION.md
docs/python/packaging.md
plans/ci-release-simplification-corrective-closure-index.md
plans/ci-verification-release-simplification-closure-report.md
plans/ci-release-simplification-corrective-phase-i-release-integrity.md
```

`Cargo.toml`, `Cargo.lock`, and `crates/*/Cargo.toml` are in scope only when Cargo-native packaging exposes an actual manifest defect. Do not perform dependency upgrades or broad manifest cleanup.

Workflow YAML, Make targets, runtime source, and Python API implementation are out of scope unless a direct regression is discovered.

## Workstream 1 — Reopen closure accurately

Before implementation claims begin, update the corrective index and closure report to state:

```text
Reopened for Corrective Phase J. Corrective Phase I completed its CI, parity,
lockfile, versioning, and evidence corrections, but post-implementation review
found that the default Rust archive stage constructs handwritten tar files rather
than validating Cargo-generated package archives. Manual Rust release closure is
pending Cargo-native archive generation and standalone package validation.
```

Preserve the Phase I hosted workflow evidence and local command evidence as historical results. Do not erase them. Reclassify only the Rust archive and `make release-check` closure claim that depended on the handwritten archives.

Required evidence status until completion:

```text
Cargo-native archives: NOT VERIFIED
make release-check release-archive criterion: BLOCKED
registry preflight: SKIPPED
publication: NOT RUN
```

## Workstream 2 — Prove the Cargo-native packaging command

The implementation must use Cargo itself to create `.crate` files.

### Preferred command shape

First test one workspace-level invocation in an isolated target directory:

```bash
cargo package \
  --workspace \
  --no-verify \
  --target-dir "$TMP_ROOT/rust-target" \
  --exclude eggsec-cli \
  --exclude eggsec-tui \
  --exclude eggsec-python
```

Add `--allow-dirty` only when running focused fixture tests or when the release script has already established and documented why the worktree state is acceptable. The normal release path already requires a clean worktree and should not need it.

The purpose of the workspace-level command is to let Cargo understand the complete internal dependency graph while producing all publishable archives in one packaging operation.

### Synthetic proof fixture

Before replacing the current path, add a temporary two- or three-package workspace fixture that includes:

- one unpublished internal dependency with a unique package name;
- `[workspace.package]` inheritance;
- `[workspace.dependencies]` inheritance;
- an aliased internal dependency using `package = "..."`;
- an optional internal dependency;
- a target-specific dependency;
- a private `publish = false` member excluded from packaging.

Run the candidate Cargo command against that fixture without contacting or publishing to crates.io.

The fixture must prove that Cargo creates archives whose normalized manifests:

- contain concrete package metadata;
- contain concrete dependency specifications instead of `workspace = true`;
- omit local `path` keys from publish-facing runtime/build dependencies;
- retain correct package aliases, versions, optional flags, and target tables;
- can be parsed as standalone packages.

Do not assume the command works because `cargo package --list` succeeds.

### Command fallback sequence

Use this bounded decision sequence:

1. Try workspace-level `cargo package --workspace --no-verify` with exact private-package exclusions.
2. If Cargo fails only because of package lockfile generation or registry state, test the same fixture with `--exclude-lockfile` and document the tradeoff.
3. If workspace packaging is unsupported on the declared Rust MSRV, test the smallest Cargo-supported per-layer command that still creates Cargo-normalized archives.
4. Do not fall back to handwritten manifest rewriting or Python tar construction.
5. Do not add a local registry service, containerized registry, or release framework without a separate maintainer decision.

If no Cargo-native local method can produce all dependent archives before registry publication, record that fact accurately and change the contract. In that case, local validation may prove manifest graph correctness and package file selection, while actual Cargo archive creation becomes a staged dependency-layer operation. Do not call custom tar files crates.io-valid archives.

### Supported toolchain

Run the proof on:

- the repository's current stable Cargo;
- Rust/Cargo 1.80 if the toolchain is available.

If 1.80 remains unavailable, record `NOT RUN`. Do not claim MSRV packaging compatibility from current-stable behavior alone.

## Workstream 3 — Replace handwritten archive creation

### Remove custom package assembly

Delete or retire:

```text
cmd_create_archive
_normalized_manifest_text
manual tarfile archive construction
regex-based removal of path keys
manual expansion of workspace package fields
```

The helper may continue to use `tarfile` for read-only inspection of Cargo-generated archives. It must not use `tarfile` to construct release-validation archives.

### Add one Cargo packaging owner

Prefer one helper command owned by `scripts/release-package-graph.py`, for example:

```bash
python3 scripts/release-package-graph.py package-workspace <target-dir>
```

Required behavior:

1. derive the exact publishable package set from Cargo metadata;
2. derive the exact private exclusions;
3. run one documented Cargo packaging command;
4. preserve full stdout/stderr;
5. return Cargo's non-zero status unchanged;
6. locate the Cargo-generated `target/package` directory under the isolated target root;
7. require exactly one `<name>-<version>.crate` file for each expected package;
8. reject unexpected publishable archives and missing archives;
9. emit a machine-readable manifest of archive records.

Recommended output record fields:

```json
{
  "package": "eggsec-core",
  "version": "0.1.0",
  "archive": "/tmp/.../eggsec-core-0.1.0.crate",
  "size": 12345,
  "sha256": "..."
}
```

Use JSON Lines or one JSON array. Do not parse human-formatted table output in the shell.

### Remove shell archive selection

`release-check.sh` should consume the helper's machine-readable archive inventory. Remove `mapfile`, `find`-based exact-match counting, and shell-owned archive ambiguity logic.

This also eliminates the macOS Bash incompatibility in this section.

## Workstream 4 — Make Cargo standalone parsing authoritative

For each Cargo-generated archive:

1. inspect the tar structure and normalized `Cargo.toml`;
2. extract into a fresh temporary directory;
3. run Cargo against the extracted manifest:

```bash
cargo metadata \
  --manifest-path <extracted>/Cargo.toml \
  --format-version 1 \
  --no-deps \
  --offline
```

The command must return zero. This is the authoritative check that the packaged manifest no longer depends on a missing workspace root.

Do not allow the source repository workspace to be discovered accidentally. The extracted package must live outside the repository tree or in a temporary directory with no ancestor workspace manifest.

### Required normalized-manifest checks

The existing inspector should retain and extend direct checks for clear diagnostics:

- expected package name and version;
- no `workspace = true` inheritance remains anywhere in the packaged manifest;
- no local `path` remains in normal, build, target-normal, or target-build dependencies;
- internal dependency versions match the release version policy;
- private packages are absent from runtime/build dependency tables;
- aliased dependencies preserve the correct `package` value;
- optional dependencies preserve `optional = true`;
- target-specific dependency tables are preserved and valid;
- every feature reference names an existing feature or optional dependency in the normalized manifest;
- repository, license/license-file, readme, edition, and rust-version are concrete and correct;
- no `[workspace]`, `[patch]`, or `[replace]` section remains in the publish-facing manifest;
- Cargo's generated metadata files are present when expected.

Do not rely on regular-expression rewriting. Inspection may parse TOML and compare against Cargo metadata and source manifests.

### Cargo-generated archive structure

Record and test the expected Cargo archive structure, including where present:

```text
Cargo.toml
Cargo.toml.orig
Cargo.lock
.cargo_vcs_info.json
README/license files
package sources
```

Do not require optional files that Cargo legitimately omits for a given package. Base assertions on package configuration and observed Cargo behavior.

### Package-content hygiene

Retain focused rejection of:

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

Do not add generic secret scanning, SBOM generation, provenance, or signing.

## Workstream 5 — Correct release-check integration

### Rust packaging stage

Replace the per-crate handwritten loop with:

1. one Cargo-native packaging helper invocation;
2. one machine-readable archive inventory;
3. one inspection pass over the exact inventory;
4. one standalone Cargo metadata parse for every extracted archive;
5. archive size and SHA-256 output for every Rust package.

Recommended summary:

```text
Rust Cargo archives: PASS (12/12 Cargo-generated, parsed, and inspected)
Registry preflight: SKIPPED (required during staged publication)
Python wheel/sdist: PASS
Fresh-wheel smoke: PASS
No artifacts were published.
```

Do not say `Cargo-generated` unless the archives came directly from Cargo's package command.

### Failure semantics

Any of the following must return non-zero immediately:

- Cargo package command failure;
- missing expected archive;
- unexpected duplicate archive;
- unexpected publishable package archive;
- standalone Cargo metadata failure;
- manifest invariant failure;
- prohibited file entry;
- size/hash inventory failure.

Do not classify an error by matching `no matching package named`. Preserve Cargo's status and output.

### Temporary artifacts

Use one temporary root for:

```text
rust-target/
extracted-crates/
logs/
python-dist/
smoke-venv/
archive-inventory.jsonl
```

If `EGGSEC_RELEASE_KEEP_ARTIFACTS=1` is set, print the retained temporary root in the final summary.

Do not inspect repository `target/package` or stale `dist/` contents.

### Platform contract

Choose one truthful contract:

- make the shell path compatible with Linux and macOS Bash 3.2 by keeping archive enumeration in Python; or
- retain Linux as the only supported release host and remove statements that the complete script should work on macOS.

Preferred: remove `mapfile` and other Bash-4-only operations while keeping Linux as the only validated release host. Documentation may say macOS is unverified, not supported-by-assumption.

## Workstream 6 — Complete focused tests

Extend `scripts/test_release_package_graph.py` without adding third-party Python test dependencies.

### Cargo-native fixture tests

Required real-Cargo fixture tests:

1. workspace packaging creates archives for all selected publishable members;
2. private `publish = false` members are excluded;
3. unpublished internal dependency names do not prevent the chosen Cargo-native archive command;
4. `[workspace.package]` inheritance is concretized;
5. `[workspace.dependencies]` inheritance is concretized;
6. aliased internal dependency metadata is normalized correctly;
7. optional dependency metadata remains optional;
8. target-specific dependencies remain present and concrete;
9. extracted package passes standalone `cargo metadata --no-deps --offline`;
10. no archive manifest retains `workspace = true` or local runtime/build paths.

The fixture must use package names that cannot collide with real crates.io packages.

### Helper behavior tests

Required tests:

11. simulated Cargo non-zero status is returned unchanged by `package-workspace`;
12. missing expected archive is rejected;
13. duplicate archive selection is rejected;
14. unexpected archive is rejected;
15. inventory records include package, version, path, size, and SHA-256;
16. corrupt archive is rejected;
17. prohibited archive entry is rejected;
18. missing configured README is rejected;
19. missing configured license-file is rejected;
20. invalid feature reference is rejected;
21. target-specific retained path is rejected;
22. aliased private-package dependency is rejected;
23. version mismatch is rejected;
24. real workspace graph still validates and orders successfully.

### Release-script contract tests

Avoid a new shell framework. Use one of these lightweight approaches:

- unit-test the Python packaging helper and keep the shell as a thin caller; or
- run `release-check.sh` in a bounded test mode with a fake Cargo executable placed first on `PATH`.

At minimum prove:

- Cargo packaging failure prevents the final PASS summary;
- a skipped registry preflight is labeled `SKIPPED`;
- Rust archive PASS cannot appear unless the inventory count equals the expected package count;
- no custom archive-creation command remains.

### Full validation

Run:

```bash
python3 scripts/test_release_package_graph.py
python3 scripts/release-package-graph.py validate
python3 scripts/release-package-graph.py order
cargo metadata --locked --format-version 1 >/tmp/eggsec-metadata.json
make check
make check-python
make check-full
make release-check
```

Do not run publication commands.

## Workstream 7 — Align documentation with actual behavior

Update all active documentation that currently says `cargo package --no-verify` creates the archives when that is not what the script executes.

After implementation, active documentation must state the exact Cargo command used, including whether packaging is workspace-level, whether `--exclude-lockfile` is used, and which packages are excluded.

Required files to search:

```text
README.md
AGENTS.md
architecture/
docs/
crates/eggsec-python/
.opencode/skills/
plans/ci-verification-release-simplification-closure-report.md
plans/ci-release-simplification-corrective-closure-index.md
```

Required distinctions:

- Cargo-native local archive assembly;
- standalone manifest parsing;
- registry-sensitive dry-run;
- actual publication;
- supported release host;
- hosted CI evidence;
- branch-protection evidence.

Do not describe registry preflight as completed when it was skipped.

## Workstream 8 — Final evidence and closure

Collect evidence only against the final implementation commit.

Required local evidence table:

| Gate | Required status |
|---|---|
| package helper unit/fixture tests | `PASS` |
| real workspace graph validation | `PASS` |
| Cargo-native archive generation | `PASS` |
| expected archive set | `PASS` |
| standalone Cargo metadata for every archive | `PASS` |
| archive manifest/content inspection | `PASS` |
| Rust archive size/SHA-256 inventory | `PASS` |
| `make check` | `PASS` |
| `make check-python` | `PASS` |
| `make check-full` | `PASS` |
| `make release-check` | `PASS` |
| registry preflight | `SKIPPED` or actual result |
| Rust 1.80 package proof | `PASS` or `NOT RUN` |
| hosted CI run | actual run URL/ID and job conclusions |
| branch protection | `PASS`, `FAIL`, or `NOT VERIFIED` |
| publication | `NOT RUN` |

Only set the corrective index and closure report to `Complete` when all blocking local gates are `PASS` and hosted CI evidence is recorded accurately.

A Rust 1.80 result may remain `NOT RUN` if the toolchain is unavailable, but the documentation must not claim package-command compatibility with Rust 1.80 in that case.

## Implementation sequence

1. Reopen the corrective index and closure report for Phase J.
2. Add the synthetic workspace fixture with workspace-inherited dependencies.
3. Prove the exact Cargo-native package command on current stable Cargo.
4. Test the command on Rust 1.80 if available.
5. Implement `package-workspace` or the equivalent in the existing Python helper.
6. Remove custom tar creation and regex manifest normalization.
7. Emit an exact machine-readable archive inventory.
8. Add extraction and standalone Cargo metadata validation.
9. Extend manifest/content checks for aliases, targets, optional deps, and features.
10. Integrate the helper into `release-check.sh` and remove `mapfile`/shell archive selection.
11. Add Rust archive size and SHA-256 reporting.
12. Complete focused unit, fixture, failure-propagation, and real-workspace tests.
13. Run all primary local verification commands.
14. Push the implementation commit and record the actual hosted workflow run.
15. Update active documentation with the exact implementation.
16. Mark Phase J and the closure report complete only after evidence is recorded.

## Validation commands

### Static removal checks

```bash
rg -n 'cmd_create_archive|_normalized_manifest_text|tarfile\.open\(.*w:gz|create-archive' scripts
rg -n 'mapfile' scripts/release-check.sh
```

Expected after implementation:

- no custom release archive writer remains;
- `tarfile` write mode is absent from the package helper;
- `mapfile` is absent from the release script.

### Cargo packaging evidence

```bash
python3 scripts/release-package-graph.py package-workspace /tmp/eggsec-package-proof
cat /tmp/eggsec-package-proof/archive-inventory.jsonl
```

Verify exact package count and filenames against:

```bash
python3 scripts/release-package-graph.py order
```

### Archive standalone checks

For each inventory record, the helper must execute an equivalent of:

```bash
cargo metadata \
  --manifest-path /tmp/extracted/<package>-<version>/Cargo.toml \
  --format-version 1 \
  --no-deps \
  --offline
```

### Active documentation searches

```bash
rg -n 'cargo package --no-verify|cargo package --list|deterministic local archive|Cargo-generated|create-archive|mapfile' \
  README.md AGENTS.md architecture docs crates/eggsec-python .opencode/skills \
  plans/ci-verification-release-simplification-closure-report.md \
  plans/ci-release-simplification-corrective-closure-index.md
```

Every retained statement must match the final implementation.

### No-publication search

```bash
rg -n 'cargo publish|maturin publish|twine upload|id-token:\s*write|tags:' .github/workflows
```

No hosted publication behavior may appear.

## Acceptance criteria

- The corrective index and closure report are reopened before implementation evidence is claimed.
- The packaging path uses Cargo to create every `.crate` archive.
- The release helper no longer constructs gzip tar archives.
- The release helper no longer rewrites Cargo manifests with regular expressions.
- The chosen Cargo command is proven with a synthetic workspace containing unpublished internal dependencies.
- The synthetic fixture includes `[workspace.package]` and `[workspace.dependencies]` inheritance.
- The exact publishable package set is derived from Cargo metadata.
- Private packages are excluded explicitly.
- Exactly one Cargo-generated archive exists for every intended package/version.
- Missing, duplicate, or unexpected archives fail validation.
- Every extracted archive passes standalone `cargo metadata --no-deps --offline` outside the repository workspace.
- No packaged manifest retains `workspace = true`.
- No packaged runtime/build dependency retains a local path.
- Target-specific runtime/build dependencies are inspected.
- Aliased internal dependencies are resolved and checked by actual package name.
- Optional dependency flags and valid feature references are preserved.
- Private crates are absent from publish-facing runtime/build dependency graphs.
- Package metadata, README, and license configuration are validated.
- Prohibited archive entries are rejected.
- Rust archive path, size, and SHA-256 are recorded for every package.
- Cargo packaging failure remains non-zero and prevents a PASS summary.
- Registry preflight remains separate and is labeled `SKIPPED` when not run.
- `release-check.sh` no longer uses `mapfile` for archive selection.
- Active documentation states the exact packaging command actually used.
- Unsupported macOS or MSRV claims are removed or backed by direct evidence.
- Package-helper tests pass.
- `make check` passes.
- `make check-python` passes.
- `make check-full` passes.
- `make release-check` passes using only Cargo-generated Rust archives.
- Hosted CI has an actual run URL/ID and all mandatory job conclusions are recorded.
- Branch protection remains honestly labeled when unavailable.
- No package, tag, or GitHub Release is published.
- No runtime behavior, public API, enforcement posture, workflow graph, or feature scope changes are introduced.

## Explicit non-goals

- Publishing the initial crates.io release.
- Publishing the Python package.
- Adding automated release workflows.
- Adding a local registry service without separate authorization.
- Adding Docker or a registry container.
- Adding provenance, attestation, signing, SBOM, or release bots.
- Reintroducing broad CI matrices.
- Expanding runtime functionality.
- Reorganizing crates.
- Upgrading dependencies unrelated to a proven package defect.
- Fixing unrelated all-feature compilation issues.
- Claiming macOS or Rust 1.80 release support without direct execution evidence.

## Rollback strategy

If Cargo cannot generate all dependent archives locally on the supported toolchain:

1. remove the handwritten archive writer anyway;
2. retain deterministic graph and source-manifest validation locally;
3. define Cargo archive creation as a staged dependency-layer operation;
4. require Cargo-native archive inspection immediately before each layer is published;
5. record dependent archive status as `NOT VERIFIED` until that staged operation runs;
6. do not mark the overall manual Rust release path fully closed;
7. seek a separate maintainer decision before introducing a local registry.

The fallback must reduce claims rather than fabricate Cargo-equivalent artifacts.

## Handoff notes

This phase is narrow but exacting. The key requirement is not more validation volume; it is replacing a false abstraction boundary with Cargo's own package implementation.

Implementation agents should begin with the synthetic workspace proof and stop if the proposed Cargo command does not produce standalone normalized archives. Do not spend time expanding documentation or archive checks until the Cargo-native creation path is demonstrated. Once demonstrated, keep the shell thin and place package enumeration, Cargo invocation, archive inventory, and inspection in the existing standard-library Python helper.