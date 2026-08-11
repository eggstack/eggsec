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
cargo test -p eggsec --features rest-api --tests --no-fail-fast
cargo test -p eggsec-output --tests
bash scripts/check-architecture-guards.sh
```

The package-level test commands automatically include all integration tests. Newly added tests run without Makefile maintenance. No `cargo-nextest` is required. Any pull request touching Rust source, workspace configuration, or architecture documentation must pass all of these checks locally before pushing.

### Defect classes covered

| Command | Defect class | Why merge-time |
|---------|-------------|----------------|
| `cargo fmt --all --check` | Style inconsistency | Mechanical; blocks clean diffs |
| `cargo check --workspace --no-default-features` | Missing feature gates, broken no-default build | Catches regressions in optional-feature boundaries |
| `cargo clippy --lib -p eggsec -- -D warnings` | Code quality, API misuse, common bugs | Low-cost static analysis on primary engine |
| `cargo test -p eggsec --features rest-api --tests` | Behavioral regressions across all integration tests | Exercises MCP, REST, enforcement, dispatch, scanner, fuzzer, agent, NSE, and more |
| `cargo test -p eggsec-output --tests` | Report envelope roundtrip | Output crate is leaf; distinct defect class |
| `bash scripts/check-architecture-guards.sh` | Architecture drift (dependency boundaries, stale terminology, bypass patterns) | Static grep checks catch regressions not covered by types/tests |

## Mandatory Python contributor contract

For changes touching `crates/eggsec-python/`, `scripts/`, or `docs/python/`, additionally require:

```bash
make check-python
```

This runs `scripts/check-python.sh` which builds the extension once and runs behavioral tests, capability/architecture checks, stub parity, and type checks in a single virtual environment.

## Platform portability

Rust checks run on Linux in CI (`ci.yml` `rust` job). The `msrv` and
`portability` jobs have been moved to `deep-checks.yml` (weekly schedule or
manual trigger) to keep routine PR CI lightweight. The declared MSRV is 1.88.

- MSRV validation: `deep-checks.yml` `msrv` job (Rust 1.88, `--no-default-features`)
- macOS/Windows portability: `deep-checks.yml` `portability` job

Contributors generally need local platform testing only for platform-specific
changes.

## Code MSRV vs release-tool Cargo

The declared code MSRV (1.88) is the minimum Rust compiler version for building
the project. The `cargo package` and `cargo publish` commands may require a
newer Cargo version than the MSRV — these are release-tooling operations, not
code compilation. The release-tool Cargo requirement is validated separately
during `make release-check` and is documented in `docs/RELEASING.md`.

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
| Known advisories | `cargo deny check advisories` | `deny.toml` + `docs/DEPENDENCY_EXCEPTIONS.md` | Every `check-full` run |
| Disallowed licenses | `cargo deny check licenses` | `deny.toml` (allow list) | Every `check-full` run |
| Banned/duplicate dependencies | `cargo deny check bans` | `deny.toml` (warn on multiples) | Every `check-full` run |
| Secret introduction | GitHub-native secret scanning | Repository settings | Every push (GitHub-managed) |

`cargo audit` is available locally as a secondary advisory check but is not run in CI to avoid tool duplication with `cargo deny`. Both tools share the same advisory ignore list; `deny.toml` is the canonical source. Detailed exception documentation lives in `docs/DEPENDENCY_EXCEPTIONS.md`.

## Which changes require Python checks

Changes require Python verification when they touch:

- `crates/eggsec-python/` (any file)
- `scripts/` (any Python or shell script)
- `docs/python/` (any documentation)
- `crates/eggsec-core/` (shared types used by Python bindings)
- `crates/eggsec/src/` (engine code affecting Python dispatch)

Changes to `eggsec-tui`, `eggsec-cli`, `eggsec-daemon`, or `eggsec-runtime` alone do not require Python checks.

## Which changes require optional feature/system checks before release

Before a release tag is created, the Linux release host must pass:

```bash
make check
make check-python
make check-full
make release-check
```

`make check-full` covers the selected representative feature profiles, not
every possible `--all-features` combination. Unsupported or currently broken
all-feature combinations are not release gates. Python wheel validation is
limited to the artifacts built by the manual release process; cross-platform
wheel production is not claimed unless it is separately performed and recorded
on each target platform.

## Merge readiness vs release readiness

**Merge readiness** requires:
- `make check` passes (in `ci.yml`)
- Python checks pass (if Python files changed)
- No clippy warnings
- Format check passes

**Release readiness** additionally requires:
- `make check-full` passes (advisories and representative profiles)
- `make release-check` passes end-to-end on the supported Linux release host
- all intended Rust archives are created by Cargo's workspace package command,
  recorded with size/SHA-256, and inspected with standalone Cargo metadata;
  registry preflight is a separate staged-maintainer operation

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
| `make check-msrv` | MSRV compile check | Deep checks only (requires `rustup toolchain install 1.88`) |
| `make check-feature-profiles` | Representative feature profiles | Pre-release |
| `make release-check` | Release validation (no publication) | Pre-release |
| `make test-feature-matrix` | Feature metadata validation | Every PR/push (part of `make check`) |
| `make build` | Release build of CLI binary | Release only |
