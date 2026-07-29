# Release Procedure

This document defines the manual, maintainer-controlled release process for
Eggsec. Release cadence and publication are explicit maintainer decisions.

## Non-negotiable policy

- No GitHub Actions workflow publishes any package.
- No tag triggers artifact construction, publication, or GitHub Release creation.
- No workflow requires registry credentials or trusted-publishing OIDC permissions.
- The final publish command is typed explicitly by a maintainer in a local
  or maintainer-controlled environment.
- Validation and publication are separate steps.
- Failed publication is handled according to registry immutability rules;
  an already published version is never overwritten.

## Registries

| Registry | Package | Publication method |
|----------|---------|-------------------|
| crates.io | `eggsec-core`, `eggsec`, `eggsec-nse`, `eggsec-output`, `eggsec-tool-core`, `eggsec-agent`, `eggsec-runtime`, `eggsec-db-lab`, `eggsec-web-proxy`, `eggsec-mobile-lab`, `eggsec-daemon`, `eggsec-ui-model` | `cargo publish` (manual) |
| PyPI | `eggsec` | `maturin publish` (manual) |
| TestPyPI | `eggsec` | Optional, manual rehearsal |

Note: `eggsec-cli`, `eggsec-tui`, and `eggsec-python` have `publish = false`
and are not published to crates.io.

## Pre-release validation

Run the local release check script:

```bash
make release-check
# or directly:
scripts/release-check.sh <version>
```

This validates without publishing:

1. Clean working tree
2. Version alignment across Cargo.toml, pyproject.toml, and optional argument
3. `make check` (mandatory Rust CI contract)
4. `make check-python` (mandatory Python CI contract)
5. Rust crate package dry-runs
6. Python wheel and sdist build
7. Fresh-environment wheel installation and smoke test
8. Artifact inventory (filenames, sizes, SHA-256 hashes)

Optional full validation (pre-release, not required for every merge):

```bash
make check-full
```

## Rust crates / crates.io

### Prerequisites

- Authenticated with crates.io using the maintainer's chosen mechanism
  (e.g., `cargo login` with an API token).
- All version files aligned and clean working tree.

### Dependency order

Publish crates in dependency order. Dependent crates must be available on
crates.io before their dependents are published:

1. `eggsec-core` (no internal dependencies)
2. `eggsec-tool-core` (depends on `eggsec-core`)
3. `eggsec-runtime` (depends on `eggsec-core`)
4. `eggsec` (depends on `eggsec-core`, `eggsec-tool-core`, `eggsec-runtime`)
5. `eggsec-output` (depends on `eggsec-core`)
6. `eggsec-nse` (depends on `eggsec-core`, `eggsec`)
7. `eggsec-agent` (depends on `eggsec-core`)
8. `eggsec-db-lab` (depends on `eggsec-core`)
9. `eggsec-web-proxy` (depends on `eggsec-core`)
10. `eggsec-mobile-lab` (depends on `eggsec-core`)
11. `eggsec-ui-model` (depends on `eggsec-core`)
12. `eggsec-daemon` (depends on `eggsec-runtime`)

Wait for each crate to appear on crates.io before publishing dependents.

### Commands

```bash
cargo publish -p eggsec-core
# wait for availability...
cargo publish -p eggsec-tool-core
# wait for availability...
cargo publish -p eggsec-runtime
# wait for availability...
cargo publish -p eggsec
cargo publish -p eggsec-output
cargo publish -p eggsec-nse
cargo publish -p eggsec-agent
cargo publish -p eggsec-db-lab
cargo publish -p eggsec-web-proxy
cargo publish -p eggsec-mobile-lab
cargo publish -p eggsec-ui-model
cargo publish -p eggsec-daemon
```

### Post-publication

- Never attempt to overwrite an immutable version. Bump the version for corrections.
- Tag only after the maintainer decides the publication state warrants it.

## Python package / PyPI

### Prerequisites

- Authenticated with PyPI (API token in `~/.pypirc` or environment).
- `maturin` and `twine` installed.
- Release check passed.

### Build

```bash
cd crates/eggsec-python
rm -rf dist/ target/wheels/
maturin build --release --out dist
maturin sdist --out dist
python -m twine check dist/*
```

### Publish

Preferred command:

```bash
cd crates/eggsec-python
maturin publish
```

Or manually:

```bash
python -m twine upload dist/*
```

### Optional TestPyPI rehearsal

TestPyPI is opt-in, not a release gate. If used, the rehearsal version must
be unique because package indexes are immutable.

```bash
python -m twine upload --repository testpypi dist/*
```

## GitHub tags/releases

Tagging and GitHub Release notes are optional manual follow-up activities.
They must not publish registry packages or trigger release workflows.

If maintainers create a GitHub Release manually, document it as
metadata/distribution convenience after registry publication, not the source
of release cadence.

## Version bump workflow

1. Update `version` in workspace `Cargo.toml`.
2. Update `version` in `crates/eggsec-python/pyproject.toml` to match.
3. Run `make release-check` to validate alignment.
4. Publish crates and/or Python package as described above.
5. Tag the release (optional, after publication): `git tag v<version>`.
6. Push tag: `git push origin v<version>`.

## Recovery from failed publication

- If a version is partially published (some crates succeeded, others failed),
  complete the remaining publishes if possible. If the failing crate cannot
  be fixed without a code change, bump the version and start fresh.
- Never overwrite an immutable published version.
