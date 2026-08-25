# Phase E Plan: Manual Release Workflow and Publication Removal

## Status


Status: Executed.
Planned. This phase removes all package/release mutation from hosted CI and replaces it with a local maintainer-controlled validation and publication procedure.

## Objective

Make release cadence and publication explicitly manual. Delete tag-triggered and workflow-dispatch publishing paths for PyPI, TestPyPI, crates.io, and GitHub Releases. Provide one local release-check command that validates repository state and built artifacts but stops before any irreversible upload.

The release process should be deliberate, inspectable, and simple enough that maintainers can execute it without a CI state machine.

## Non-negotiable release policy

- No GitHub Actions workflow publishes any package.
- No GitLab pipeline publishes any package.
- No tag triggers artifact construction, publication, or GitHub Release creation.
- No repository workflow requires registry credentials or trusted-publishing OIDC permissions.
- The final publish command is typed explicitly by a maintainer in a local or maintainer-controlled environment.
- Validation and publication are separate steps.
- Failed publication is handled according to registry immutability rules; an already published version is never overwritten.
- The procedure supports the registries actually used by Eggsec: crates.io for publishable Rust crates and PyPI for the Python package. TestPyPI may be used manually when a maintainer deliberately chooses a rehearsal, but it is not a required workflow gate.

## Files to remove or rewrite

Delete:

```text
.github/workflows/release.yml
.github/workflows/testpypi-rehearsal.yml
```

Remove publishing jobs and inputs from:

```text
.github/workflows/python-wheels.yml
```

Preferred final disposition for `python-wheels.yml`:

- delete it if routine Python validation is fully covered by Phase C and manual artifact construction is covered locally; or
- reduce it to a manual, non-publishing artifact smoke workflow only if maintainers have a demonstrated need for hosted cross-platform wheel construction.

A retained manual wheel workflow must:

- use only `workflow_dispatch`;
- have no PyPI/TestPyPI environment;
- have no `id-token: write`;
- have no package credentials;
- upload artifacts only for inspection;
- not create a GitHub Release;
- not be required for ordinary merges or releases.

The default recommendation is deletion to keep release ownership local.

## Local release artifacts

Add a canonical release document, recommended path:

```text
docs/RELEASING.md
```

Add a local validation script or Make target, recommended paths:

```text
scripts/release-check.sh
make release-check
```

The script must use strict shell behavior:

```bash
set -euo pipefail
```

It must validate without publishing.

## Release-check requirements

### 1. Repository state

Verify:

- the working tree is clean;
- the current branch/commit is intentional;
- all required version files are aligned;
- the requested version is not empty or malformed;
- generated source-controlled metadata that remains part of the package is current;
- no local untracked package artifact is accidentally being included.

The script may accept an expected version argument:

```bash
scripts/release-check.sh 0.2.0
```

It must not infer that a release should occur merely because a tag exists.

### 2. Version alignment

Validate the exact current package layout. At minimum inspect:

```text
Cargo.toml
crates/eggsec-python/Cargo.toml
crates/eggsec-python/pyproject.toml
```

Also inspect publishable workspace crates whose versions are independently declared or inherited.

Use a small Python/TOML-aware helper where practical rather than fragile `grep '^version'` extraction. Do not add a heavy dependency solely for this; Python 3.11+ `tomllib` is sufficient for local tooling.

The check must distinguish workspace package version inheritance from a literal crate version.

### 3. Mandatory verification

Run:

```bash
make check
make check-python
```

Optionally run `make check-full` when the release includes feature/system surfaces covered there. `docs/RELEASING.md` must state when the optional full check is expected.

Do not generate an evidence bundle to prove these commands ran. Their exit status and operator review are sufficient.

### 4. Rust package dry-run

For each publishable Rust crate, use the appropriate local checks:

```bash
cargo package -p <crate>
cargo publish -p <crate> --dry-run
```

Where workspace dependency ordering matters, document the order explicitly. Do not run actual `cargo publish` from the script.

If some workspace crates are intentionally `publish = false`, assert that they are excluded.

### 5. Python artifact build

From the Python binding package:

```bash
cd crates/eggsec-python
maturin build --release
maturin sdist --out dist
python -m twine check dist/*
```

Use the repository's actual maturin output paths consistently. Clean stale artifacts before building so validation cannot accidentally inspect a prior version.

### 6. Fresh-environment installation

Create a temporary virtual environment outside the source package, install the newly built wheel, and run a compact release smoke suite:

- import `eggsec`;
- verify version/build info;
- verify `py.typed` and stubs are installed;
- verify capability metadata loads;
- run report serialization/redaction smoke;
- run one deterministic loopback operation if supported by the default wheel.

The smoke command must import from the installed wheel, not the workspace source tree. Print and validate `eggsec.__file__`.

### 7. Artifact inventory

Print filenames, sizes, and SHA-256 hashes for operator inspection. This output may remain local. Do not require uploading a generated evidence manifest to GitHub.

The script should stop with a clear message such as:

```text
Release validation passed. No artifacts were published.
```

## Manual publication procedure

`docs/RELEASING.md` must separate Rust and Python publication.

### Rust crates / crates.io

Document:

1. authenticate locally with crates.io using the maintainer's chosen secure mechanism;
2. publish crates in dependency order;
3. wait for registry availability where dependent crates require it;
4. verify the published version;
5. create/push a tag only after the maintainer decides the publication state warrants it;
6. never attempt to overwrite an immutable version—bump the version for corrections.

Example commands may include:

```bash
cargo publish -p <crate>
```

Do not include real tokens or encourage storing credentials in repository files.

### Python package / PyPI

Document one preferred explicit command:

```bash
cd crates/eggsec-python
maturin publish
```

or:

```bash
python -m twine upload dist/*
```

Choose one as canonical based on the package layout; document the alternative only when it serves a real recovery or signing need.

Optional manual TestPyPI rehearsal:

```bash
python -m twine upload --repository testpypi dist/*
```

This is opt-in, not a release gate. If used, the rehearsal version must be unique because package indexes are immutable.

### GitHub tags/releases

Tagging and GitHub Release notes are optional manual follow-up activities. They must not publish registry packages or trigger release workflows. If maintainers create a GitHub Release manually, document it as metadata/distribution convenience after registry publication, not the source of release cadence.

## Remove hosted release permissions and secrets

Search and remove:

- `id-token: write` used for package publication;
- `environment: pypi`;
- `environment: testpypi`;
- `MATURIN_PYPI_TOKEN`;
- `PYPI_API_TOKEN` or equivalent;
- `TEST_PYPI_TOKEN`;
- crates.io tokens in workflow environment;
- `pypa/gh-action-pypi-publish`;
- `maturin upload` or `maturin publish` in workflows;
- `cargo publish` in workflows;
- GitHub release actions or `gh release` commands;
- tag-triggered `on.push.tags` release automation.

Repository-level secrets may remain configured outside source control, but documentation should state they are no longer required by CI. Do not claim they were deleted unless repository settings were actually changed.

## Documentation reconciliation

Update at minimum:

```text
AGENTS.md
docs/VERIFICATION.md
docs/RELEASING.md
docs/python/packaging.md
docs/python/versioning.md
crates/eggsec-python/README.md
README.md
```

Remove claims that:

- pushing a tag releases packages;
- TestPyPI rehearsal is required;
- GitHub Actions validates or publishes release artifacts;
- release evidence bundles are required;
- GitHub environments approve PyPI promotion;
- CI determines release cadence.

State clearly that version bumps and release dates are maintainer decisions.

## Implementation steps

1. Inventory all hosted publishing commands, permissions, environments, secrets references, and tag triggers.
2. Add `docs/RELEASING.md` with the manual policy and immutable-version recovery rule.
3. Implement `scripts/release-check.sh` and `make release-check`.
4. Validate the script on the current version without publication.
5. Delete `release.yml` and `testpypi-rehearsal.yml`.
6. Delete or strip publishing from `python-wheels.yml`; prefer deletion unless hosted artifact builds have a documented owner.
7. Remove release/evidence Make targets superseded by `release-check`.
8. Remove documentation references to hosted release gates.
9. Search the full repository for publication commands and permissions.
10. Push a non-release commit and verify no release workflow runs.
11. Push or create a temporary local tag only if needed for local validation; do not push a test tag to the shared repository merely to prove absence.

## Validation commands

```bash
make release-check
```

Repository searches:

```bash
rg -n "cargo publish|maturin (publish|upload)|twine upload|gh release|action-gh-release|gh-action-pypi-publish" .github .gitlab-ci.yml Makefile scripts docs AGENTS.md README.md
rg -n "id-token:\s*write|environment:\s*(pypi|testpypi)|PYPI|TEST_PYPI|CRATES_IO" .github .gitlab-ci.yml
rg -n "tags:\s*$|v\*" .github/workflows
rg -n "release\.yml|testpypi-rehearsal\.yml|python-wheels\.yml" .
```

Expected workflow inventory after this phase:

```text
.github/workflows/ci.yml
.github/workflows/deep-checks.yml   # optional; may be absent
```

## Acceptance criteria

- `.github/workflows/release.yml` is deleted.
- `.github/workflows/testpypi-rehearsal.yml` is deleted.
- `.github/workflows/python-wheels.yml` is deleted or contains no publishing and is manual-only.
- No workflow triggers on `v*` tags.
- No workflow has package-publishing OIDC permission or package-index credentials.
- No workflow invokes `cargo publish`, `maturin publish/upload`, `twine upload`, or GitHub Release mutation.
- `docs/RELEASING.md` exists and declares manual maintainer-controlled cadence.
- `make release-check` performs validation and artifact smoke tests but cannot publish.
- Version alignment uses TOML-aware parsing or an equivalently robust method.
- Fresh-environment wheel installation verifies the built artifact rather than workspace imports.
- The procedure documents crates.io and PyPI separately and accurately.
- The procedure states that published versions are immutable and corrections require a version bump.
- TestPyPI is optional and manual.
- A normal push and a tag push have no publication side effects.
- No release credentials are required for CI.

## Explicit non-goals

- Publishing a release during implementation.
- Changing package names or ownership.
- Adding a release PR bot.
- Automating changelog generation.
- Adding signing, provenance, SBOM attestation, or supply-chain frameworks unless already required by a registry.
- Building every cross-platform wheel in GitHub Actions.
- Deleting historical release notes or retained plans.

## Rollback strategy

If local cross-platform artifact construction proves impractical, add a manual non-publishing artifact-build workflow with no credentials and no release triggers. Do not restore automated promotion or publication. If a local release check is too broad, split validation into readable subcommands while keeping publication outside all scripts.

## Handoff notes

The implementation report must include the final repository-wide publication-command search and the exact files remaining under `.github/workflows/`. It must explicitly state that no package was published while executing this phase.