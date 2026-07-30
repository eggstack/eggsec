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
make check-python
```

This runs `scripts/check-python.sh` which builds the extension once and runs behavioral tests, capability/architecture checks, stub parity, and type checks in a single virtual environment.

## Platform portability

Rust checks run on Linux in CI (`ci.yml` `rust` job). Narrow portability is validated by:

- `cargo check -p eggsec` on macos-latest and windows-latest (in `ci.yml` `portability` job)

macOS and Windows builds are **not** required for every PR. Contributors should test locally on their target platform when making platform-specific changes.

## Optional broad validation

These checks are valuable but not required for every merge. They run in the optional `deep-checks.yml` workflow (weekly schedule or manual trigger) or locally via `make check-full`.

| Check | Command | Purpose |
|-------|---------|---------|
| Full mandatory contract | `make check` (included in `check-full`) | Baseline correctness |
| Advisory/license/ban policy | `cargo deny check` | Dependency policy enforcement |
| Representative feature profiles | `make check-feature-profiles` | Feature coherence |

### Security tool ownership

Each defect class has one primary tool and owner:

| Defect class | Tool | Configuration | Cadence |
|-------------|------|---------------|---------|
| Known advisories | `cargo deny check advisories` | `deny.toml` (advisory ignore list) | Every `check-full` run |
| Disallowed licenses | `cargo deny check licenses` | `deny.toml` (allow list) | Every `check-full` run |
| Banned/duplicate dependencies | `cargo deny check bans` | `deny.toml` (warn on multiples) | Every `check-full` run |
| Secret introduction | GitHub-native secret scanning | Repository settings | Every push (GitHub-managed) |

`cargo audit` is available locally as a secondary advisory check but is not run in CI to avoid tool duplication with `cargo deny`. Both tools share the same advisory ignore list; `deny.toml` is the canonical source.

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

1. `make release-check` passes (local validation, no publication)
2. `python scripts/release-package-graph.py validate` passes (publishability invariants)
3. All feature-gated crates compile: `cargo check -p eggsec --features <feature>` for each feature in the feature matrix
4. Representative feature profiles pass: `make check-feature-profiles`
5. Deep checks pass: `cargo check --workspace --all-features && cargo test --workspace --all-features`
6. Python wheel builds succeed on all target platforms

## Merge readiness vs release readiness

**Merge readiness** requires:
- `make check` passes (in `ci.yml`)
- Python checks pass (if Python files changed)
- No clippy warnings
- Format check passes

**Release readiness** additionally requires:
- `make release-check` passes (local validation, no publication)
- All optional feature profiles compile
- Deep checks pass

## Release publication is always manual

Release publication is never part of CI. No workflow triggers on tags or
publishes packages. The release process is manual and maintainer-controlled.
See [docs/RELEASING.md](RELEASING.md) for the full procedure.

## Package registries

| Registry | Package | Publication method |
|----------|---------|-------------------|
| PyPI | `eggsec` (Python wheel) | Manual: `maturin publish` or `twine upload` |
| TestPyPI | `eggsec` (pre-release) | Optional manual rehearsal |
| crates.io | Rust workspace crates | Manual: `cargo publish` in dependency order |
| GitHub Releases | (optional metadata) | Manual, after registry publication |

## Make targets reference

| Target | Purpose | When required |
|--------|---------|---------------|
| `make test` | Unit tests only | Default local check |
| `make check` | Full mandatory Rust CI contract (no nextest) | Every PR/push |
| `make check-python` | Python CI check (one build, all checks) | Python changes |
| `make check-full` | Optional broad validation (advisories + feature profiles) | Pre-release |
| `make clippy` | Lint | Every PR/push |
| `make fmt` | Format check | Every PR/push |
| `make check-no-default` | No-default-features build | Every PR/push (part of `make check`) |
| `make check-feature-profiles` | Representative feature profiles | Pre-release |
| `make release-check` | Release validation (no publication) | Pre-release |
| `make test-feature-matrix` | Feature metadata validation | Every PR/push (part of `make check`) |
| `make build` | Release build of CLI binary | Release only |
