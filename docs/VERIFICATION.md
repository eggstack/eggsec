# Verification Contract

This document defines the mandatory, optional, and release-only verification surface for Eggsec. It is the single authoritative entry point for understanding what must pass before a change is merged and what is reserved for release preparation.

## Mandatory Rust contributor contract

The single canonical command for ordinary Rust/Linux changes:

```bash
make check
```

This expands to:

```bash
cargo fmt --all --check
cargo check --workspace --no-default-features
cargo clippy --lib -p eggsec -- -D warnings
cargo test --lib -p eggsec
cargo test -p eggsec --test metadata_consistency
cargo test -p eggsec --test command_registry
cargo test -p eggsec --test tool_registration --features rest-api
cargo test -p eggsec --test feature_matrix
cargo test -p eggsec --test enforcement_matrix
cargo test -p eggsec --test enforced_dispatch_regression
cargo test -p eggsec-output --test report_envelope
bash scripts/check-architecture-guards.sh
```

No `cargo-nextest` is required. Any pull request touching Rust source, workspace configuration, or architecture documentation must pass all of these checks locally before pushing.

## Mandatory Python contributor contract

For changes touching `crates/eggsec-python/`, `scripts/`, or `docs/python/`, additionally require:

```bash
# Build and install
cd crates/eggsec-python && maturin develop && cd ../..

# Python checks
python scripts/check-python-capability-matrix.py
python scripts/check-python-architecture-guards.py
python scripts/check_python_stub_parity.py
bash scripts/check_python_types.sh
pytest crates/eggsec-python/tests/ -v --timeout=60
```

Or via the Makefile (requires `rtk` for some targets):

```bash
make test-python-phase-f
```

## Platform portability

Rust checks run on Linux in CI (`ci.yml` `rust` job). Narrow portability is validated by:

- `cargo check -p eggsec` on macos-latest and windows-latest (in `ci.yml` `portability` job)
- Python wheel builds on linux x86_64 and macos universal2 (in `python-wheels.yml`)

macOS and Windows builds are **not** required for every PR. Contributors should test locally on their target platform when making platform-specific changes.

## Optional broad validation

These checks are valuable but not required for every merge:

| Check | Trigger | Command |
|-------|---------|---------|
| Full workspace all-features | Weekly schedule | `cargo check --workspace --all-features && cargo test --workspace --all-features` |
| Feature profile checks | Push to main | `make check-feature-profiles` |
| Code coverage | Push to main | `cargo tarpaulin -p eggsec --features rest-api,nse` |
| Security audit | Every PR | `cargo audit --deny warnings` |
| Cargo deny | Every PR | `cargo deny check advisories && cargo deny check licenses && cargo deny check bans` |
| Dependency review | PR only | GitHub dependency-review-action |
| Secret scanning | PR only | gitleaks |

## Which changes require Python checks

Changes require Python verification when they touch:

- `crates/eggsec-python/` (any file)
- `scripts/` (any Python or shell script)
- `docs/python/` (any documentation)
- `crates/eggsec-core/` (shared types used by Python bindings)
- `crates/eggsec/src/` (engine code affecting Python dispatch)

Changes to `eggsec-tui`, `eggsec-cli`, `eggsec-daemon`, or `eggsec-runtime` alone do not require Python checks.

## Which changes require optional feature/system checks before release

Before a release tag is created, verify:

1. All feature-gated crates compile: `cargo check -p eggsec --features <feature>` for each feature in the feature matrix
2. Representative feature profiles pass: `make check-feature-profiles`
3. Deep checks pass: `cargo check --workspace --all-features && cargo test --workspace --all-features`
4. Python wheel builds succeed on all target platforms
5. TestPyPI rehearsal succeeds

## Merge readiness vs release readiness

**Merge readiness** requires:
- `make check` passes (in `ci.yml`)
- Python checks pass (if Python files changed)
- No clippy warnings
- Format check passes

**Release readiness** additionally requires:
- All optional feature profiles compile
- Deep checks pass
- Python wheels build on all platforms
- TestPyPI upload and install succeeds
- Evidence bundle generated
- Maturity guard passes

## Release publication is always manual

Release publication is never part of CI. The release workflow (`release.yml`) is triggered by:
- Pushing a `v*` tag
- Manual `workflow_dispatch` with a tag input

The final `publish-pypi` job requires manual approval via GitHub Environments. PyPI publication is never automatic.

## Package registries

| Registry | Package | Publication trigger |
|----------|---------|-------------------|
| PyPI | `eggsec` (Python wheel) | Manual approval after TestPyPI validation |
| TestPyPI | `eggsec` (pre-release) | Manual `workflow_dispatch` or tag-driven release |
| crates.io | `eggsec`, `eggsec-core`, etc. | Not yet automated (planned) |
| GitHub Releases | Binary + evidence bundle | Tag-driven release workflow |

## Make targets reference

| Target | Purpose | When required |
|--------|---------|---------------|
| `make test` | Unit tests only | Default local check |
| `make check` | Full mandatory Rust CI contract (no nextest) | Every PR/push |
| `make check-full` | Optional broad validation (full-no-system + full features) | Pre-release |
| `make clippy` | Lint | Every PR/push |
| `make fmt` | Format check | Every PR/push |
| `make check-no-default` | No-default-features build | Every PR/push (part of `make check`) |
| `make check-feature-profiles` | Representative feature profiles | Pre-release |
| `make test-feature-matrix` | Feature metadata validation | Every PR/push (part of `make check`) |
| `make test-python-phase-f` | Python compat + budgets + redaction | Python changes |
| `make build-python-evidence` | Generate evidence bundle | Pre-release |
| `make build` | Release build of CLI binary | Release only |
