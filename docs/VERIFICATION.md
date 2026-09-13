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
cargo check -p eggsec
cargo check -p eggsec-cli
cargo check -p eggsec-cli --no-default-features
make check-deps  # cargo deny --workspace --all-features check (advisories + bans + licenses + sources)
make clippy   # engine lib (empty default + cli) + leaf crates (-D warnings)
cargo test -p eggsec --doc
cargo test -p eggsec --no-default-features --test tool_registration --test loadtest_tests --no-fail-fast
cargo test -p eggsec --features rest-api,cli --tests --no-fail-fast
cargo test -p eggsec-output --tests
bash scripts/check-architecture-guards.sh
```

The package-level test commands automatically include all integration tests. Newly added tests run without Makefile maintenance. No `cargo-nextest` is required. Any pull request touching Rust source, workspace configuration, or architecture documentation must pass all of these checks locally before pushing.

### Defect classes covered

| Command | Defect class | Why merge-time |
|---------|-------------|----------------|
| `cargo fmt --all --check` | Style inconsistency | Mechanical; blocks clean diffs |
| `cargo check --workspace --no-default-features` | Missing feature gates, broken no-default build | Catches regressions in optional-feature boundaries |
| `make check-deps` (`cargo deny --workspace --all-features check`: advisories + bans + licenses + sources) | Known advisories, disallowed licenses, banned/wildcard deps, unexpected sources | Supply-chain policy must fail PRs, not weekly jobs; all-features closure covers optional deps (pdf, db-pentest, mssql) |
| `make clippy` (engine lib empty-default + `cli` + `eggsec-core`, `eggsec-tool-core`, `eggsec-output`, `eggsec-runtime`, `eggsec-ui-model`, `eggsec-agent`, `eggsec-transport`, `eggsec-transport-eggfetch`, `-D warnings`) | Code quality, API misuse, common bugs | Low-cost static analysis on engine and leaf crates |
| `cargo test -p eggsec --features rest-api,cli --tests` | Behavioral regressions across all integration tests | Exercises MCP, REST, enforcement, dispatch, scanner, fuzzer, agent, NSE, and more |
| `cargo test -p eggsec-transport-eggfetch --tests` | Adapter parity/adversarial regressions (33 local-fixture tests) | Proves approved-IP pinning, authorized redirects, TLS/timeout mapping without production wiring |
| `cargo test -p eggsec-output --tests` | Report envelope roundtrip | Output crate is leaf; distinct defect class |
| `bash scripts/check-architecture-guards.sh` | Architecture drift (dependency boundaries, stale terminology, bypass patterns) | Static grep checks catch regressions not covered by types/tests |

## Mandatory Python contributor contract

For changes touching `crates/eggsec-python/`, `scripts/`, or `docs/python/`, additionally require:

```bash
make check-python
```

This runs `scripts/check-python.sh` which builds the extension once and runs behavioral tests, capability/architecture checks, stub parity, and type checks in a single virtual environment.

## Platform portability and integration (Phase F)

Rust checks run on Linux in CI (`ci.yml` `rust` job). The `msrv` and
`portability` jobs have been moved to `deep-checks.yml` (weekly schedule or
manual trigger) to keep routine PR CI lightweight. The declared MSRV is 1.88.

Platform-sensitive fixture suites are hermetic (no root/hardware) and run
locally without privilege:

```bash
cargo run -p eggsec-cli -- doctor
bash scripts/check_platform.sh
```

Live layers (netns, emulator, RF) are isolated, may SKIP with the named
prerequisite, and run only via the scheduled/manual `platform-integration`
job in `deep-checks.yml` — never in routine PR CI. Real RF/hardware stays a
maintainer procedure. See [PLATFORM.md](PLATFORM.md) for the matrix, fixture
vs live commands, expected skips, and release-gating.

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
| Full mandatory contract | `make check` (included in `check-full`; already runs `check-deps`) | Baseline correctness + dependency policy |
| Domain/platform lint | `make clippy-domain` | Lint extracted implementation crates with relevant features |
| Representative feature profiles | `make check-feature-profiles` | Feature coherence (engine profiles + broad TUI `db-pentest,web-proxy,c2` check and lib tests) |
| Exhaustive per-feature sweep | `make check-features-individual` | Every engine and TUI feature compiled in its minimum set (`full` aggregates are curated, not exhaustive) |

### Security tool ownership

Each defect class has one primary tool and owner:

| Defect class | Tool | Configuration | Cadence |
|-------------|------|---------------|---------|
| Known advisories | `cargo deny check advisories` | `deny.toml` + `docs/DEPENDENCY_EXCEPTIONS.md` | Every PR (`dependency-policy` job + `make check`) |
| Disallowed licenses | `cargo deny check licenses` | `deny.toml` (allow list + `auto_generate_cdp` build-time exception) | Every PR (`dependency-policy` job + `make check`) |
| Banned/duplicate dependencies | `cargo deny check bans` | `deny.toml` (deny `aws-lc-rs`, deny wildcards, warn on multiples) | Every PR (`dependency-policy` job + `make check`) |
| Unexpected sources | `cargo deny check sources` | `deny.toml` (`unknown-registry/git = "deny"`, git rev required) | Every PR (`dependency-policy` job + `make check`) |
| Manifest diff vulnerabilities | GitHub Dependency Review | `ci.yml` `dependency-review` job, fail on moderate+ | Every PR (diff only) |
| Secret introduction | GitHub-native secret scanning | Repository settings | Every push (GitHub-managed) |

`cargo deny` (`deny.toml`) is the canonical dependency-policy source. `cargo
audit` is diagnostic only and is not run in CI: `.cargo/audit.toml` was
removed in Phase F because it suppressed a larger stale advisory set with no
owner metadata. Do not reintroduce it; record new exceptions in `deny.toml` +
`docs/DEPENDENCY_EXCEPTIONS.md` with owner and review-by date. Detailed
exception documentation lives in `docs/DEPENDENCY_EXCEPTIONS.md`.

Supply-chain workflow hardening (Phase F): all GitHub Actions are pinned to
immutable full-length commit SHAs with human-readable version comments;
workflows run with least-privilege `permissions: contents: read`;
`.github/dependabot.yml` proposes weekly `cargo` + `github-actions` updates
(no auto-merge); Dependency Review fails PRs on moderate+ vulnerabilities with
read-only permissions. Checks 109–112 in
`scripts/check-architecture-guards.sh` enforce these boundaries.

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
make check-features-individual
make release-check
```

`make check-full` covers domain lint and the selected representative feature
profiles, not every possible `--all-features` combination. Exhaustive
per-feature coverage comes from `make check-features-individual`
(weekly/manual; missing system prerequisites are reported as SKIP, not FAIL).
Unsupported or currently broken all-feature combinations are not release gates.
Python wheel validation is limited to the artifacts built by the manual release
process; cross-platform wheel production is not claimed unless it is separately
performed and recorded on each target platform.

## Merge readiness vs release readiness

**Merge readiness** requires:
- `make check` passes (in `ci.yml` `rust` job: fmt, no-default checks, `check-deps`, clippy, tests, guards)
- `dependency-policy` job passes (same `make check-deps` as an independent gate signal)
- `dependency-review` job passes on PRs (moderate+ vulnerabilities fail)
- Python checks pass (if Python files changed)
- No clippy warnings
- Format check passes

**Release readiness** additionally requires:
- `make check-full` passes (domain lint, representative profiles; `check-deps` already covered via `make check`)
- `make check-features-individual` passes (or reports only documented system-prerequisite SKIP entries)
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
| `make check` | Full mandatory Rust CI contract (format, checks, `check-deps`, clippy, tests, guards) | Every PR/push |
| `make check-deps` | Dependency advisory/license/ban/source policy (`cargo deny --workspace --all-features check`) | Every PR/push (part of `make check`, plus independent `dependency-policy` CI job) |
| `make check-python` | Python CI check (one build, all checks) | Python changes |
| `make check-full` | Optional broad validation (domain lint + feature profiles; `check-deps` via `check`) | Pre-release |
| `make clippy` | Lint | Every PR/push |
| `make fmt` | Format check | Every PR/push |
| `make check-no-default` | No-default-features build | Every PR/push (part of `make check`) |
| `make check-msrv` | MSRV compile check | Deep checks only (requires `rustup toolchain install 1.88`) |
| `make check-feature-profiles` | Representative feature profiles | Pre-release |
| `make release-check` | Release validation (no publication) | Pre-release |
| `make test-feature-matrix` | Feature metadata validation | Every PR/push (part of `make check`) |
| `make clippy-domain` | Lint domain/platform crates | Pre-release (part of `make check-full`) |
| `make check-features-individual` | Exhaustive per-feature compile sweep | Pre-release / weekly (deep checks) |
| `make build` | Release build of CLI binary | Release only |
