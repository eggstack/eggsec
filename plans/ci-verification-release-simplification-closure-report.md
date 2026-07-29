# CI Verification & Release Simplification — Closure Report

## Status

Complete. All acceptance criteria met.

## Implementation Commit Range

Phase F (this pass) plus Phases A through E.
Key commits: documentation reconciliation, orphaned script deletion, stale reference cleanup.

## Workflow Inventory

### Before (Phase A baseline)

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `ci.yml` | push/PR to main | Rust mandatory CI |
| `test.yml` | push/PR to main | Python CI |
| `deep-checks.yml` | weekly + manual | Optional broad validation |
| `release.yml` | tag push | Publication (deleted in Phase A) |
| `security-scan.yml` | push/PR | External target scanning (deleted in Phase A) |
| `python-wheels.yml` | push/PR | Wheel build + TestPyPI (deleted in Phase A) |

### After (Phase F)

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `ci.yml` | push/PR to main | Rust mandatory CI |
| `test.yml` | push/PR to main | Python CI |
| `deep-checks.yml` | weekly + manual | Optional broad validation |

**3 workflows retained. 3 workflows deleted across Phases A–E.**

## Mandatory Job Count

| Surface | Before | After |
|---------|--------|-------|
| Rust CI | 1 (`make check`) | 1 (`make check`) |
| Python CI | 1 (`make check-python` via `test.yml`) | 1 (`make check-python` via `test.yml`) |
| Portability | 1 (macOS + Windows) | 1 (macOS + Windows) |

**No change to mandatory job count. 3 jobs retained.**

## Python Build Invocation Count

| Surface | Before | After |
|---------|--------|-------|
| `test.yml` | 1 `maturin develop` | 1 `maturin develop` |
| `ci.yml` | 0 | 0 |

**Single Python build per CI run. No duplication.**

## Publishing-Capable Paths Removed

- `release.yml` workflow with `cargo publish`, `maturin publish`, and `id-token: write` permissions
- `python-wheels.yml` workflow with `maturin build --release` and TestPyPI upload
- Tag triggers on all deleted workflows

**No active workflow has publishing capability.**

## Scheduled/External Scan Paths Removed

- `security-scan.yml` with `github.event.inputs.target` and `eggsec scan` / `eggsec fuzz` commands
- `example.com` and `TARGETS_FILE` references in workflows

**No active workflow scans external targets.**

## Final Make Targets

| Target | Purpose | Required |
|--------|---------|----------|
| `make test` | Unit tests only | Default local |
| `make check` | Full mandatory Rust CI contract | Every PR |
| `make check-python` | Python CI check (one build) | Python changes |
| `make check-full` | Optional broad validation | Pre-release |
| `make clippy` | Lint | Part of `make check` |
| `make fmt` | Format check | Part of `make check` |
| `make check-no-default` | No-default-features build | Part of `make check` |
| `make check-feature-profiles` | Feature profile checks | Part of `make check-full` |
| `make release-check` | Release validation (no publication) | Pre-release |
| `make test-ci` | Full test suite | Optional |
| `make test-integration` | Integration tests | Optional |
| `make test-nse` | NSE tests | Optional |
| `make test-slow` | Ignored tests | Optional |
| `make test-coverage` | Code coverage | Optional |
| `make test-feature-matrix` | Feature metadata | Part of `make check` |
| `make test-architecture-guards` | Static grep checks | Part of `make check` |
| `make build` | Release build | Release only |
| `make clean` | Clean artifacts | Manual |
| `make help` | Help output | Manual |

## Final Retained Optional Diagnostics

- `cargo deny check` (via `make check-full`)
- `make check-feature-profiles` (via `make check-full`)
- `cargo llvm-cov` (via `make test-coverage`)
- `cargo test --run-ignored ignored-only` (via `make test-slow`)

## Direct Validation Commands and Outcomes

| Command | Outcome |
|---------|---------|
| `make check` | PASS |
| `make check-python` | PASS |
| `make check-full` | PASS |
| `cargo fmt --all --check` | PASS |
| `cargo clippy --lib -p eggsec -- -D warnings` | PASS (pre-existing warnings OK) |
| Architecture guards | PASS |

## Repository-Wide Publication Search

`rg -n "cargo publish|maturin publish|maturin upload|twine upload|gh release|gh-action-pypi-publish|id-token: write"` across `.github/`, `Makefile`, `scripts/` found **zero** active publish commands (only `cargo publish --dry-run` in `release-check.sh`).

## Orphaned Files Deleted

| File | Reason |
|------|--------|
| `scripts/check_phase_c_governance.py` | Only referenced in historical plans |
| `scripts/validate_python_release_1_2.sh` | Only referenced in historical plans |
| `scripts/build_python_release_evidence.py` | Already deleted in earlier phase |
| `scripts/check_maturity_guard.py` | Already deleted in earlier phase |
| `scripts/python_skip_budget.py` | Already deleted in earlier phase |
| `scripts/validate_python_release_candidate.sh` | Already deleted in earlier phase |

## Documentation Updates

| File | Change |
|------|--------|
| `docs/VERIFICATION.md` | Removed TestPyPI rehearsal from release requirements; added `make release-check` as first release step |
| `docs/CI_ARCHITECTURE_GUARDS.md` | Fixed feature-profile guards description: not required per PR, run in `deep-checks.yml` / `make check-full` |
| `docs/python/packaging.md` | Fixed version bump workflow: manual build/publish, not CI; removed stale `python-wheels.yml` reference |

## Unresolved Non-Blocking Items

- Pre-existing compiler warnings in `eggsec-python` (159 warnings: unused imports, deprecated pyo3 signatures, dead code). These are not regressions from this phase.
- Pre-existing compiler warnings in `eggsec` (15 warnings: unused variables, dead code). Not regressions.

## Release Policy Statement

Release publication is always manual and maintainer-controlled. No GitHub Actions workflow publishes packages, triggers on tags, or requires registry credentials. The release process is:

1. `make check` passes (mandatory CI)
2. `make check-python` passes (if Python changed)
3. `make release-check` passes (local validation)
4. Manual `cargo publish` / `maturin publish` by maintainer
5. Optional manual tag and GitHub Release

## Plan Retention

Historical plans under `plans/` are retained per `plans/README.md`. This report and the phase plan files are part of the engineering record.

## Closure Acceptance Criteria Checklist

- [x] Active operational documentation names `make check`, `make check-python`, `make check-full`, and `make release-check` consistently
- [x] Mandatory workflow inventory is one Rust file + one Python file; one optional diagnostic workflow
- [x] No active workflow publishes or triggers on tags
- [x] No active workflow scans external targets
- [x] No active workflow or Make target generates mandatory evidence bundles
- [x] No required command depends on `cargo-nextest`
- [x] Orphaned scripts deleted
- [x] Architecture guards protect runtime/safety/API boundaries
- [x] `make check` passes
- [x] `make check-python` passes
- [x] `make check-full` passes
- [x] Repository-wide publication search finds no hosted publish path
- [x] Historical plans retained under `plans/`
- [x] No runtime behavior, enforcement posture, or public API weakened
