# CI Verification & Release Simplification — Closure Report

## Status

Corrective Phase J local and hosted closure complete 2026-07-31 for implementation
commit `b91d9f9`. Hosted CI run
[`30636819135`](https://github.com/eggstack/eggsec/actions/runs/30636819135)
passed Rust, Python, macOS, and Windows jobs. CodeQL run
[`30636818358`](https://github.com/eggstack/eggsec/actions/runs/30636818358)
also passed. The CI simplification and Phase H workflow contract remain
complete. Manual publication was not run.

The earlier Phase H evidence is retained below as historical context. It is not
evidence for Phase I.

## Corrective scope

Corrective Phases G and H supersede the earlier completion claim. Phase G made
the intended crates publishable, derived the release order from Cargo metadata,
and reduced `release-check` to deterministic local validation. Phase H repairs
specialist Make targets, merges Python CI into `ci.yml`, removes duplicate deep
checks, and makes the verification and release documentation executable.

No runtime behavior, enforcement posture, public API, or feature semantics are
changed by this closure work. Phases I and J add only release-integrity
validation, documentation, and evidence corrections.

## Corrective Phase J — Cargo-native archive evidence

The active Rust release path is owned by `scripts/release-package-graph.py` and
uses exactly:

```bash
cargo package --workspace --no-verify --target-dir <isolated-target> \
  --exclude eggsec-cli --exclude eggsec-tui --exclude eggsec-python
```

Cargo generated the exact 12-package set:
`eggsec-core`, `eggsec-agent`, `eggsec-output`, `eggsec-db-lab`,
`eggsec-mobile-lab`, `eggsec-nse`, `eggsec-runtime`, `eggsec-tool-core`,
`eggsec-ui-model`, `eggsec-web-proxy`, `eggsec`, and `eggsec-daemon`.
The helper emitted JSONL records containing package, version, absolute archive
path, size, and SHA-256; each archive passed content/manifest inspection and
standalone `cargo metadata --no-deps --offline` outside the source workspace.
No Python archive writer or shell archive selector remains.

| Gate | Status | Evidence |
|---|---|---|
| package helper and Cargo fixture tests | `PASS` | 29 tests; unpublished dependency, workspace inheritance, alias, optional/target dependency, private member, failure status |
| real workspace graph validation/order | `PASS` | 12-package acyclic graph |
| Cargo-native archive generation and exact set | `PASS` | `make release-check`, commit `130c233` |
| standalone Cargo metadata and archive inspection | `PASS` | all 12 archives |
| Rust archive size/SHA-256 inventory | `PASS` | JSONL inventory under isolated target |
| `make check` | `PASS` | Linux x86_64 |
| `make check-python` | `PASS` | Linux x86_64 |
| `make check-full` | `PASS` | Linux x86_64 |
| `make release-check` | `PASS` | Rust archives, wheel/sdist, fresh-wheel smoke |
| registry preflight | `SKIPPED` | separate staged maintainer operation |
| Cargo 1.80 package proof | `FAIL` | Cargo 1.80.1 cannot complete the workspace package graph for unpublished internal dependencies; no compatibility claim is made |
| publication | `NOT RUN` | explicitly excluded |

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

### Corrective Phase I

Evidence was collected on Linux x86_64 against implementation commit
`b65a68a993de0e9a5d8733f9cbd9bb43af70c0fc`.

| Command or gate | Status | Host / evidence |
|---|---|---|
| `python3 scripts/test_release_package_graph.py` | `PASS` | 28 tests |
| package graph `validate` and `order` | `PASS` | 12-package acyclic order |
| `cargo metadata --locked` and workspace locked check | `PASS` | Linux x86_64 |
| `make check` | `PASS` | Rust format, lint, tests, and architecture guards |
| `make check-python` | `PASS` | Canonical Python verification |
| `make check-full` | `PASS` | Advisories, licenses, bans, sources, and feature profiles |
| `make release-check` | `PASS` | 12 deterministic archives inspected; wheel/sdist and fresh-wheel smoke passed |
| registry preflight | `SKIPPED` | Separate staged manual process; no registry publication |
| Rust 1.80 MSRV check | `NOT RUN` | Toolchain unavailable in validation environment |
| hosted CI [30632663714](https://github.com/eggstack/eggsec/actions/runs/30632663714) | `PASS` | Rust, Python, macOS, and Windows jobs all passed |
| branch-protection settings | `NOT VERIFIED` | Repository settings unavailable to the implementation environment |
| package/release publication | `NOT RUN` | Explicitly excluded by Phase I |

The retained lockfile delta is limited to the targeted `event-listener` 5.4.1
to 5.4.2 security correction after `cargo deny` rejected the older version.
The package graph now diagnoses stale internal versions at file/dependency-key
locations, and failed archive operations remain failures.

### Historical Phase H evidence

The earlier evidence below is retained against implementation commit `4c94186`
on Linux x86_64 for continuity; it is superseded for closure purposes by the
Phase I table above.
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

The workflow and stale-reference searches passed. Corrective Phase I local gates
and the remote mandatory CI jobs completed with `PASS` for the pushed
implementation commit above.
