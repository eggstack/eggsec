# Phase B Plan: Core Rust CI Collapse

## Status

Planned. This phase implements the compact mandatory Rust CI contract defined in Phase A. It must preserve critical correctness and enforcement coverage while removing duplicated job orchestration.

## Objective

Replace the Rust portion of the current monolithic `test.yml` with a small Linux-first workflow and narrow macOS/Windows portability checks. Consolidate commands into a reproducible `make check` target, remove routine release builds and broad matrices from the mandatory path, and ensure each retained check owns a distinct defect class.

The goal is not to make CI green by weakening code. The goal is to make failures high-signal, locally reproducible, and proportionate to the repository.

## Preconditions

Phase A must be complete. The implementation agent must have:

- the current-job disposition table;
- the critical invariant list;
- the canonical `docs/VERIFICATION.md` contract;
- knowledge of any branch-protection checks that reference current job names.

If branch protection requires status names that will disappear, coordinate their replacement as part of the cutover. Do not leave `main` permanently requiring deleted checks.

## Target structure

Create or reduce to one mandatory workflow, recommended path:

```text
.github/workflows/ci.yml
```

The workflow should trigger on:

```yaml
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
```

Do not include tag triggers, scheduled triggers, package publication permissions, workflow-dispatch release inputs, or release environments.

Logical jobs:

### `rust`

One Ubuntu job should run the canonical Rust/core contract. The final command list must derive from Phase A, but the expected baseline is:

```bash
cargo fmt --all -- --check
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

This list is intentionally explicit for handoff, but implementation should remove a command if Phase A demonstrates exact duplication and records its replacement. Conversely, do not add every historical test target merely because it existed in `test.yml`.

The Ubuntu job may call `make check` rather than repeating commands in YAML, provided the Make target is transparent and contains no release/evidence side effects.

### `portability`

Use a small operating-system matrix:

```yaml
strategy:
  matrix:
    os: [macos-latest, windows-latest]
```

Run only a narrow compile or unit-test smoke command such as:

```bash
cargo check -p eggsec
```

or, if Phase A identifies workspace portability as essential and sufficiently fast:

```bash
cargo test -p eggsec --lib
```

Do not run release builds, all-feature builds, coverage, full integration tests, or Python wheel matrices in this job.

## Makefile changes

Refactor `Makefile` so routine verification does not require `cargo-nextest`.

Required target shape:

```make
.PHONY: check check-python check-full

check:
	cargo fmt --all -- --check
	cargo check --workspace --no-default-features
	cargo clippy --lib -p eggsec -- -D warnings
	cargo test --lib -p eggsec
	# selected invariant tests
	bash scripts/check-architecture-guards.sh
```

Existing specialist targets may remain when useful, but `make test` and `make check` must not have surprising differences. Recommended policy:

- `make test`: ordinary core unit tests using `cargo test`;
- `make check`: full mandatory Rust/Linux CI contract;
- `make check-full`: optional broad validation implemented fully in later phases;
- nextest-specific targets: retain only under explicit names such as `test-nextest` and mark them optional.

Remove claims that contributors require `cargo-nextest` for CI parity.

## Workflow reductions

Within the current `test.yml`, remove or migrate the following from mandatory Rust CI:

- the ten-entry `check` matrix;
- the five-entry `test-features` matrix where covered by retained core tests;
- routine code coverage generation and Codecov upload;
- three-platform `cargo build --release`;
- standalone `cargo audit` when dependency policy is retained elsewhere;
- three separate `cargo deny` invocations as mandatory per-push jobs;
- dependency review and secret scanning only if Phase D will immediately re-home them; otherwise leave them temporarily until Phase D;
- the nine-entry feature-profile matrix;
- duplicate library tests inside architecture jobs;
- any Rust release gate or artifact creation.

Do not delete the Python section in this phase unless the replacement Python job from Phase C lands in the same atomic change. Prefer a staged cutover that avoids losing validation.

## Feature coverage policy

Replace feature-name enumeration with representative dependency-topology coverage.

Expected optional profiles, subject to Phase A findings:

```bash
cargo check -p eggsec --features full-no-system
cargo check -p eggsec --features full
```

If `full` requires system dependencies or combines incompatible/platform-sensitive capabilities, do not alter code semantics to make it mandatory. Instead define a small representative set under `make check-full` and the optional diagnostic workflow.

Potential representative categories:

- default/no-default core;
- one API/protobuf profile;
- one extracted domain crate profile;
- one system-dependent profile;
- one hazardous or privilege-sensitive compile-only profile.

A category should have one owner unless multiple configurations exercise genuinely different dependency graphs.

## Caching and setup

Keep workflow setup simple:

- `actions/checkout@v4`;
- one pinned or stable Rust setup action consistent with repository MSRV policy;
- `Swatinem/rust-cache@v2` or equivalent existing cache;
- install `ripgrep` only if architecture guards still require it;
- install `protobuf-compiler` only in the optional profile that compiles gRPC.

Do not install coverage, advisory, license, packaging, or release tools in mandatory Rust CI.

## Failure behavior

- `fail-fast` may remain enabled for a single job's sequential commands. There is no value in running later expensive commands after formatting or compilation fails unless the implementation uses grouped shell commands with clear summaries.
- Portability matrix entries may use `fail-fast: false` so both operating systems report.
- Do not use `continue-on-error` for mandatory checks.
- Do not upload JUnit or build artifacts from mandatory Rust CI unless a specific debugging need is documented.

## Implementation steps

1. Create the new `make check` target using Phase A's retained command list.
2. Run `make check` locally before changing workflow triggers.
3. Create `.github/workflows/ci.yml` or reduce `test.yml` to the target structure.
4. Add the narrow portability job.
5. Remove duplicated Rust jobs and matrices from the old workflow.
6. Ensure ordinary pushes trigger only one copy of the Rust contract.
7. Update branch protection or document the exact required-status migration.
8. Update `AGENTS.md` quick verification to use `make check` and standard Cargo prerequisites.
9. Run a repository search for old Rust job names and CI-parity claims.
10. Commit the cutover atomically enough that mandatory validation is never absent.

## Validation commands

Run locally on Linux or the primary development platform:

```bash
make check
cargo test -p eggsec --test enforcement_matrix
cargo test -p eggsec --test enforced_dispatch_regression
cargo test -p eggsec-output --test report_envelope
```

Inspect workflow syntax with an available YAML parser or GitHub workflow linter. At minimum:

```bash
python - <<'PY'
from pathlib import Path
import yaml
for path in Path('.github/workflows').glob('*.yml'):
    yaml.safe_load(path.read_text())
    print(path)
PY
```

If PyYAML is not a project dependency, use an ephemeral environment or another existing YAML parser; do not add a runtime dependency solely for this validation.

After push, verify:

- one Ubuntu Rust job runs;
- one macOS portability job runs;
- one Windows portability job runs;
- no release build or package artifact job is triggered;
- required status checks match branch protection.

## Acceptance criteria

- One mandatory workflow owns routine Rust verification.
- `make check` reproduces the Ubuntu Rust job without `cargo-nextest`.
- The mandatory Rust job does not use a feature matrix.
- macOS and Windows perform only narrow portability checks.
- No mandatory job runs `cargo build --release`.
- No mandatory job generates coverage artifacts.
- No mandatory job installs both `cargo-audit` and `cargo-deny`.
- The retained tests directly cover enforcement, metadata/registry consistency, dispatch regression, report envelope, and architecture boundaries.
- The workflow contains no tag, schedule, package publish, or release environment configuration.
- Normal pushes no longer execute duplicate `cargo test -p eggsec --lib` runs in separate jobs.
- `AGENTS.md` and `docs/VERIFICATION.md` agree on the canonical command.
- Branch protection does not require deleted job names.
- Runtime and public API behavior are unchanged.

## Explicit non-goals

- Making every optional feature compile on every operating system.
- Achieving a coverage percentage target.
- Replacing Cargo with another build system.
- Adding reusable composite actions.
- Building release artifacts.
- Fixing unrelated pre-existing warnings by weakening `-D warnings` policy without review.
- Deleting Python/release workflows before their replacement phases are ready.

## Rollback strategy

If the replacement workflow misses a critical invariant, add the smallest direct command to `make check`; do not restore the entire old matrix. If branch protection blocks merging because of renamed checks, correct the protection configuration rather than reintroducing duplicate compatibility jobs indefinitely.

## Handoff notes

The implementation agent should report a before/after job count and identify each removed Rust matrix entry's disposition from Phase A. A claim of simplification is incomplete if the YAML becomes smaller but `make check` invokes a new script that reproduces the old matrix internally.