# Phase F Plan: Documentation, Guard Reconciliation, and Closure

## Status


Status: Executed.
Planned. This is the final closure phase. It must not introduce a new CI framework or reopen feature/release expansion.

## Objective

Reconcile repository documentation, Make targets, scripts, architecture guards, and branch-protection expectations with the simplified CI and manual release model. Remove dead references and orphaned process artifacts, then produce a compact closure report based on direct verification and structural evidence.

The line of work is complete only when the repository tells one coherent story: routine correctness is checked by a small local/CI contract, broad diagnostics are optional, and publication is a manual maintainer action.

## Preconditions

Phases A through E are implemented and the replacement mandatory workflow has completed successfully at least once on `main` or a representative pull request.

The expected workflow inventory is:

```text
.github/workflows/ci.yml
.github/workflows/deep-checks.yml   # optional; may be absent
```

If additional workflows remain, each must have a documented purpose, non-overlapping ownership, no publication capability, and explicit approval under this roadmap. “Historical” is not sufficient justification for an active workflow.

## Workstream 1 — Canonical documentation

Review and update at minimum:

```text
README.md
AGENTS.md
docs/VERIFICATION.md
docs/RELEASING.md
docs/BUILD.md
docs/python/packaging.md
docs/python/versioning.md
docs/python/README_1_0_CHECKLIST.md
crates/eggsec-python/README.md
plans/README.md
```

### `AGENTS.md`

The quick verification section should be operational and short:

```bash
make check
make check-python   # when Python-facing code, bindings, stubs, or docs change
```

Document `make check-full` as optional before broad feature/release work. Remove:

- requirements to reproduce a large historical CI matrix;
- evidence-bundle commands;
- Phase F maturity/skip-budget gate descriptions;
- workflow lists that include deleted files;
- claims that wheel publication or TestPyPI rehearsal occurs in CI;
- mandatory `cargo-nextest` prerequisite language.

Do not turn `AGENTS.md` into a release manual; link to `docs/RELEASING.md`.

### `docs/VERIFICATION.md`

Confirm it remains the canonical explanation of:

- mandatory merge checks;
- change-to-check mapping;
- portability smoke scope;
- optional diagnostic scope;
- system/feature prerequisites;
- local reproduction commands;
- distinction between merge and release readiness.

### `docs/RELEASING.md`

Confirm it states:

- manual cadence;
- local validation first;
- explicit crates.io/PyPI publication commands as applicable;
- immutable version handling;
- no GitHub Actions publication;
- optional manual TestPyPI rehearsal;
- optional manual tag/GitHub Release follow-up.

### Python documentation

Remove release maturity claims that were coupled to generated evidence or old workflow status. Keep actual API maturity documentation where it describes compatibility promises, not CI bureaucracy.

## Workstream 2 — Makefile and command surface cleanup

The final Makefile should have a small primary surface:

```text
make test
make check
make check-python
make check-full
make release-check
make clean
make help
```

Specialist targets may remain, but help output must distinguish primary commands from optional specialist diagnostics.

Required properties:

- `make check` uses standard Cargo tooling and no credentials;
- `make check-python` builds once and uses one environment;
- `make check-full` is optional and non-publishing;
- `make release-check` validates and stops before publication;
- no target generates mandatory release evidence bundles;
- no target invokes a publish command;
- no target unexpectedly scans external targets;
- deprecated targets either become aliases with a removal note for one transition or are deleted with all references updated.

Avoid indefinite alias accumulation. This is a cleanup phase.

## Workstream 3 — Script and manifest garbage collection

Use call-site searches to identify orphaned files after workflow consolidation.

Candidates include, subject to actual retained ownership:

```text
historical Python evidence, maturity, and skip-budget helpers (removed)
scripts/generate_python_compatibility_baseline.py
the historical Python release-candidate helper (removed)
scripts/run_python_profile.py
scripts/validate_python_profiles.py
crates/eggsec-python/validation/profiles.json
crates/eggsec-python/wheel-profiles.json
```

Do not delete a file only because it appears in this list. For each candidate:

1. search all call sites;
2. inspect whether it is part of runtime/package behavior or only old CI process;
3. retain and document it if it has a named active use case;
4. otherwise delete it and update references/tests.

Remove ignored generated-output directories and artifact-retention docs that no longer have producers.

## Workstream 4 — Architecture guard reconciliation

Inspect:

```text
scripts/check-architecture-guards.sh
```

Some architecture guards may refer to historical plan retention, deprecated terminology, removed CI files, or process artifacts. Classify each guard:

- runtime architecture invariant;
- safety/enforcement invariant;
- public API/metadata invariant;
- documentation consistency check;
- historical/process guard.

Retain the first three when they are not better expressed as tests. Keep documentation checks only when they protect a canonical source of truth. Remove historical/process guards that force obsolete files or release machinery to remain.

Specific requirements:

- preserve the general `plans/` retention policy without pinning this roadmap to active status forever;
- do not require deleted workflow filenames;
- do not require evidence bundles or release-candidate artifacts;
- do not use grep guards to freeze old phase terminology;
- ensure guard failures identify the invariant and file involved.

Run architecture guards through `make check` only if their runtime is small and they protect active boundaries.

## Workstream 5 — Workflow and branch-protection closure

Verify:

- workflow triggers are limited to intended branches/manual schedule;
- no tag trigger remains;
- no publishing permission remains;
- no deleted job is still required by branch protection;
- no duplicate workflow runs on both `main` and obsolete `master` unless both branches are active;
- optional diagnostics are not required statuses;
- workflow concurrency/cancellation settings, if used, are simple and prevent stale duplicate runs without hiding failures.

A reasonable optional addition to mandatory CI is:

```yaml
concurrency:
  group: ci-${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true
```

Use only if it reduces duplicate runs without harming intentional parallel PR validation. This is optional, not a new workstream.

## Workstream 6 — Closure report

Add a concise retained report, recommended path:

```text
plans/ci-verification-release-simplification-closure-report.md
```

It must record:

- implementation commit range;
- workflow inventory before and after;
- mandatory job count before and after;
- Python build invocation count before and after;
- publishing-capable paths removed;
- scheduled/external scan paths removed;
- final Make targets;
- final retained optional diagnostics;
- direct validation commands and outcomes;
- unresolved non-blocking items, if any;
- explicit release policy statement.

Do not generate a machine evidence bundle. The report is a human-readable engineering record referencing direct commands and source paths.

## Repository-wide stale-reference search

Search for at minimum:

```text
test.yml
deep-checks.yml
security-scan.yml
python-wheels.yml
release.yml
testpypi-rehearsal.yml
python-release-gate
python-evidence-bundle
python-maturity-guard
python-skip-budget
build-python-evidence
validate_python_release_candidate
TestPyPI rehearsal
publish_pypi
id-token: write
cargo-nextest required
```

Each match must be:

- an active accurate reference;
- a retained historical plan/report clearly marked historical; or
- removed.

Historical plans under `plans/` may mention old files. Do not rewrite the entire retained engineering record. Closure searches should distinguish historical content from active operational documentation and scripts.

## Implementation steps

1. Inventory active docs and command references after Phase E.
2. Simplify `AGENTS.md` and Makefile help output.
3. Reconcile `docs/VERIFICATION.md` and `docs/RELEASING.md` with actual commands.
4. Delete orphaned scripts/manifests after call-site review.
5. Reclassify and simplify architecture guards.
6. Inspect branch-protection required checks and workflow triggers.
7. Run the complete direct validation sequence.
8. Add the closure report with measured before/after counts.
9. Perform repository-wide stale-reference searches.
10. Commit closure changes without unrelated code modifications.

## Required validation

Run:

```bash
make check
make check-python
make check-full
make release-check
```

`make check-full` may require documented optional system dependencies. If unavailable, run each supported subset and record the exact limitation in the closure report. Do not claim it passed when it did not run.

Confirm release-check non-mutation:

```bash
git status --short
```

The working tree may contain ignored build artifacts, but source-controlled files must not be modified unexpectedly.

Workflow inventory:

```bash
find .github/workflows -maxdepth 1 -type f -print | sort
```

Publishing search:

```bash
rg -n "cargo publish|maturin (publish|upload)|twine upload|gh release|gh-action-pypi-publish|id-token:\s*write" .github .gitlab-ci.yml Makefile scripts
```

External scan search:

```bash
rg -n "github\.event\.inputs\.target|example\.com|TARGETS_FILE|eggsec scan|eggsec fuzz|eggsec load" .github/workflows .gitlab-ci.yml
```

Python build duplication search:

```bash
rg -n "maturin develop|maturin build" .github/workflows
```

Documentation search:

```bash
rg -n "release\.yml|security-scan\.yml|testpypi-rehearsal\.yml|python-release-gate|build-python-evidence" README.md AGENTS.md docs Makefile scripts
```

## Closure acceptance criteria

- Active operational documentation names `make check`, `make check-python`, `make check-full`, and `make release-check` consistently.
- The mandatory workflow inventory is one file; at most one optional diagnostic workflow remains.
- No active workflow publishes or triggers on tags.
- No active workflow scans external targets or downloads a released Eggsec binary for self-validation.
- No active workflow or Make target generates mandatory evidence bundles.
- No required command depends on `cargo-nextest`.
- Orphaned evidence, maturity, skip-budget, and release-candidate scripts are deleted or have a documented active owner.
- Architecture guards protect current runtime/safety/API boundaries rather than historical process files.
- Branch protection references only existing mandatory status checks.
- `make check` passes.
- `make check-python` passes.
- `make release-check` passes and explicitly reports that nothing was published.
- `make check-full` passes where supported, or exact optional dependency limitations are documented without blocking routine iteration.
- The closure report contains measured before/after structural counts.
- A repository-wide publication search finds no hosted publish path.
- Historical plans remain retained under `plans/` in accordance with `plans/README.md`.
- No runtime behavior, enforcement posture, or public API was weakened to satisfy the simplification.

## Definition of done

This line of work is done when an ordinary contributor can clone Eggsec, run one Rust command and one Python command, receive the same high-value feedback as mandatory CI, and continue iterating without encountering release-evidence or package-publication machinery. A maintainer can separately validate a release locally and then choose whether and when to publish it.

The project must not require a future corrective pass merely to explain which workflow is authoritative.

## Explicit non-goals

- Rewriting historical plans to erase old process references.
- Adding new CI dashboards or metrics services.
- Adding a release automation replacement.
- Expanding test coverage unrelated to the simplification.
- Refactoring product code for style.
- Publishing a package.

## Handoff notes

This phase should be assigned as a narrow evidence-and-cleanup pass after implementation, not combined with feature work. The agent must report failed or unavailable commands honestly. Closure is based on direct behavior and source structure, not on the existence of a generated success artifact.
