# CI Verification & Release Simplification — Closure Report

## Status

Reopened for corrective closure. The Phase A–F CI reduction remains valid and
Phase G completed the publishability corrections, but the final closure is
pending Phase H validation on the commit that contains the unified workflow,
Make contract, documentation, and release-check evidence.

## Corrective scope

Corrective Phases G and H supersede the earlier completion claim. Phase G made
the intended crates publishable, derived the release order from Cargo metadata,
and reduced `release-check` to deterministic local validation. Phase H repairs
specialist Make targets, merges Python CI into `ci.yml`, removes duplicate deep
checks, and makes the verification and release documentation executable.

No runtime behavior, enforcement posture, public API, or feature semantics are
changed by this closure work.

## Structural baseline and final state

The baseline below is the Phase A baseline commit `5331f667`, not the later
post-roadmap snapshot used by the superseded report.

| Measure | Phase A baseline (`5331f667`) | Final Phase H state |
|---|---:|---:|
| Active workflow files | 6 | 2 |
| Optional retained workflows | 1 (`deep-checks.yml`) | 1 (`deep-checks.yml`) |
| Logical mandatory CI jobs | 25 in `test.yml` plus separate Python/wheel surfaces | 3 (`rust`, `python`, `portability`) |
| Python `maturin develop` invocations in the main test workflow | 9-profile matrix plus duplicated checker jobs | 1 shared invocation in `scripts/check-python.sh` |
| Publishing-capable workflows | 4 jobs across `python-wheels.yml` and `release.yml`, plus rehearsal | 0 hosted jobs |
| External-target scan workflows | 1 (`security-scan.yml`) | 0 |
| Package-release helper scripts | mixed release/evidence helpers | `release-check.sh` and `release-package-graph.py` |

The Phase A inventory recorded 7 workflow files only when the relocated
`.gitlab-ci.yml` example was counted alongside the six GitHub Actions files; it
is not an active GitHub workflow in either column.

## Final workflow contract

`.github/workflows/ci.yml` is the single mandatory workflow. It contains:

- `Rust`: `make check` on Linux;
- `Python`: `make check-python` on Python 3.12/Linux;
- `Portability (macos-latest)` and `Portability (windows-latest)`: narrow
  `cargo check -p eggsec` jobs.

`.github/workflows/deep-checks.yml` is optional, scheduled/manual, and runs
only `make check-full`; representative profiles therefore have one owner.
There are no tag triggers, publication permissions, external-target variables,
release artifacts, or package publication commands in hosted workflows.

## Command contract

The advertised specialist targets now use valid Cargo syntax:

| Target | Contract |
|---|---|
| `test-fast` | Alias for `test-unit` |
| `test-ci` | `cargo test -p eggsec --features rest-api --tests --no-fail-fast` (library and integration test targets; doctests excluded) |
| `test-integration` | `cargo test -p eggsec --features rest-api --tests --no-fail-fast` |
| `test-slow` | `cargo test -p eggsec --features rest-api --tests -- --ignored` |
| `check-full` | `check`, `cargo deny check`, then `$(MAKE) check-feature-profiles` |

`make check`, `make check-python`, `make check-full`, and `make release-check`
are the release-readiness commands. The supported and tested release host is
Linux; macOS and Windows are compile-portability platforms, not claimed
cross-platform wheel-release hosts.

## Validation evidence

Evidence is recorded against the final implementation commit and host after
the validation sequence completes. Each row uses `PASS`, `FAIL`, `NOT RUN`,
`BLOCKED`, or `TIMEOUT`; only `PASS` closes a blocking criterion.

| Command | Status | Host / evidence |
|---|---|---|
| `make check` | `NOT RUN` | To be run on final commit |
| `make check-python` | `NOT RUN` | To be run on final commit |
| `make check-full` | `NOT RUN` | To be run on final commit |
| `make release-check` | `NOT RUN` | Linux; to be run end-to-end on final commit |
| `make test` | `NOT RUN` | Specialist validation pending |
| `make test-fast` | `NOT RUN` | Specialist validation pending |
| `make test-ci` | `NOT RUN` | Specialist validation pending |
| `make test-integration` | `NOT RUN` | Specialist validation pending |
| `make test-slow` | `NOT RUN` | Specialist validation pending; zero ignored tests is valid |
| `make test-feature-matrix` | `NOT RUN` | Specialist validation pending |
| `make test-architecture-guards` | `NOT RUN` | Specialist validation pending |
| `make check-no-default` | `NOT RUN` | Specialist validation pending |
| `make check-feature-profiles` | `NOT RUN` | Specialist validation pending |

The `NOT RUN` placeholders must be replaced with exact final-commit evidence
before this report is marked complete.

## Documentation and stale-reference closure

Active documentation names `ci.yml` as the owner of Python verification,
describes hosted portability as running on every push/PR to `main`, and does
not present unsupported all-feature workspace checks or all-platform wheel
builds as release gates. Historical plans may retain old workflow names when
describing prior states; active instructions and skills do not.

## Final status rule

Change this report to `Complete` only after every blocking command above is
`PASS`, the final commit SHA and Linux host are recorded, the workflow and
stale-reference searches pass, and remote mandatory CI jobs complete
successfully. Until then the report intentionally remains reopened.
