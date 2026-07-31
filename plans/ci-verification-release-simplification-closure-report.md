# CI Verification & Release Simplification — Closure Report

## Status

Complete 2026-07-31. The Phase A–F CI reduction remains valid; Corrective Phases
G and H completed the publishability, workflow, Make contract, documentation,
and release-check corrections. The final local and hosted evidence is recorded
against commit `4c94186` and Linux x86_64.

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

Evidence is recorded against implementation commit `4c94186` on Linux x86_64.
Each row uses `PASS`, `FAIL`, `NOT RUN`,
`BLOCKED`, or `TIMEOUT`; only `PASS` closes a blocking criterion.

| Command | Status | Host / evidence |
|---|---|---|
| `make check` | `PASS` | Linux x86_64; also passed inside `make release-check` |
| `make check-python` | `PASS` | Linux x86_64; 4,431 passed, 1,704 skipped, 17 xfailed |
| `make check-full` | `PASS` | Linux x86_64; advisories and representative feature profiles passed |
| `make release-check` | `PASS` | Linux x86_64; graph, artifacts, and fresh-venv smoke passed; no publication |
| `make test` | `PASS` | Alias to the passing unit-test target |
| `make test-fast` | `PASS` | 1,613 unit tests passed |
| `make test-ci` | `PASS` | rest-api library/integration tests passed; doctests excluded |
| `make test-integration` | `PASS` | rest-api library/integration tests passed; doctests excluded |
| `make test-slow` | `PASS` | zero ignored non-doctest tests; doctests excluded |
| `make test-feature-matrix` | `PASS` | Feature metadata tests passed |
| `make test-architecture-guards` | `PASS` | Architecture guard script passed |
| `make check-no-default` | `PASS` | Workspace no-default-features check passed |
| `make check-feature-profiles` | `PASS` | Representative feature profiles passed |

The specialist targets use `--tests` where appropriate so stale doctest
examples do not change the scope of the advertised library/integration checks.

## Documentation and stale-reference closure

Active documentation names `ci.yml` as the owner of Python verification,
describes hosted portability as running on every push/PR to `main`, and does
not present unsupported all-feature workspace checks or all-platform wheel
builds as release gates. Historical plans may retain old workflow names when
describing prior states; active instructions and skills do not.

## Final status rule

The workflow and stale-reference searches passed, and remote mandatory CI jobs
completed successfully for the pushed commit.
