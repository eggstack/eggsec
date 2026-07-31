# Corrective Phase H Plan: CI Contract, Command, and Closure Consistency

## Status

Executed 2026-07-31. This is the final corrective phase for the CI and manual-release simplification roadmap. It depends on the finalized release-check contract and evidence from Corrective Phase G.

## Objective

Remove the remaining inconsistencies between hosted workflows, Make targets, contributor documentation, release documentation, and the retained closure report. All advertised commands must be executable, hosted CI must invoke the same canonical local commands, optional diagnostics must not duplicate work, and closure status must reflect only completed validation.

This phase is deliberately narrow. Do not expand the CI graph or add new verification frameworks.

## Confirmed defects to correct

### Invalid specialist Make targets

The current Makefile contains nextest-shaped arguments after conversion to `cargo test`:

```make
test-ci:
	cargo test -p eggsec --retries 0 --no-fail-fast

test-integration:
	cargo test -p eggsec --test '*.rs'

test-slow:
	cargo test -p eggsec --run-ignored ignored-only
```

These targets are advertised in help and documentation but do not represent valid Cargo test invocations.

### Divergent Python CI implementation

`scripts/check-python.sh` states that it is the canonical local and CI command, while `.github/workflows/test.yml` duplicates its environment creation, dependency installation, `maturin develop`, pytest, metadata, stub, and type-check commands.

This creates future drift and violates the one-command reproducibility objective.

### Workflow count deviation

The original roadmap required one mandatory CI workflow plus at most one optional diagnostic workflow. The implementation retains:

```text
.github/workflows/ci.yml
.github/workflows/test.yml
.github/workflows/deep-checks.yml
```

The simplest closure is to merge the Python job into `ci.yml` and delete `test.yml`. If maintainers intentionally choose two mandatory files, the roadmap and all closure criteria must be explicitly amended with a technical justification. The preferred implementation is one mandatory file.

### Duplicate optional diagnostics

`deep-checks.yml` invokes both:

```bash
make check-full
make check-feature-profiles
```

but `make check-full` already invokes `make check-feature-profiles`. The same feature profiles therefore run twice.

### Documentation and closure overstatement

`docs/VERIFICATION.md` contains requirements that current commands do not implement, including broad all-feature workspace checks and all-platform Python wheel builds. It also describes portability checks as not required per pull request even though the workflow runs them on every pull request.

The retained closure report says all acceptance criteria are met while recording a partial/timed-out `release-check`. It also uses before/after counts that do not consistently refer to the pre-roadmap baseline.

## Scope

Primary files:

```text
.github/workflows/ci.yml
.github/workflows/test.yml
.github/workflows/deep-checks.yml
Makefile
scripts/check-python.sh
AGENTS.md
CONTRIBUTING.md
docs/VERIFICATION.md
docs/RELEASING.md
docs/CI_ARCHITECTURE_GUARDS.md
crates/eggsec-python/README.md
crates/eggsec-python/VALIDATION.md
plans/ci-verification-release-simplification-roadmap.md
plans/ci-verification-release-simplification-closure-report.md
plans/ci-release-simplification-corrective-closure-index.md
```

Also search all active documentation and scripts for the corrected target/workflow names.

## Workstream 1 — Repair and test the Make command surface

### Primary targets

Preserve these primary commands:

```text
make test
make check
make check-python
make check-full
make release-check
make clean
make help
```

Corrective Phase G owns the final `release-check` implementation.

### Repair `test-fast`

The target appears in `.PHONY` but currently has no implementation. Either add:

```make
test-fast: test-unit
```

or remove it from `.PHONY` and all documentation. Prefer the alias only if contributors use the name.

### Repair `test-ci`

Do not attempt to emulate nextest retries with unsupported Cargo flags.

Recommended implementation:

```make
test-ci:
	cargo test -p eggsec --no-fail-fast
```

Update help text to:

```text
Run all eggsec package tests without Cargo fail-fast
```

Do not claim retry behavior.

If the workspace, rather than package-only tests, is intended, use:

```make
test-ci:
	cargo test --workspace --no-fail-fast
```

Only choose workspace scope after measuring runtime and confirming it remains an optional specialist command. Do not silently broaden it.

### Repair `test-integration`

Choose one clear semantic contract.

Preferred simple option:

```make
test-integration:
    cargo test -p eggsec --features rest-api --tests --no-fail-fast
```

Document that Cargo's `--tests` selects test targets and may include the library test target according to Cargo target selection. If maintainers need integration binaries only, add a small script that enumerates named integration test targets from Cargo metadata; do not pass a quoted wildcard to `--test`.

Given the simplification objective, use the valid simple command unless duplicate unit execution is materially costly.

### Repair `test-slow`

Recommended implementation:

```make
test-slow:
    cargo test -p eggsec --features rest-api -- --ignored
```

If ignored tests require feature flags, document or create explicit feature-specific slow targets. Do not hide broad feature activation in the default slow target.

### `.PHONY` consistency

Ensure every non-file Make target is declared once, including:

```text
check-architecture-ci
```

Remove obsolete names. Keep `check-architecture-ci: check` only as a temporary compatibility alias if active documentation or tooling still calls it; otherwise delete it during this phase.

### Help validation

`make help` must list only implemented targets and accurately describe their scope.

Add a lightweight Makefile contract check if useful, for example a shell test that extracts help targets and confirms `make -n <target>` succeeds. Do not introduce a parser framework.

## Workstream 2 — Unify hosted and local Python verification

Merge the Python job into `.github/workflows/ci.yml` and invoke the canonical command:

```yaml
python:
  name: Python
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - uses: Swatinem/rust-cache@v2
    - uses: actions/setup-python@v5
      with:
        python-version: "3.12"
    - name: Install system dependencies
      run: sudo apt-get update && sudo apt-get install -y ripgrep
    - name: Run canonical Python verification
      run: make check-python
```

Delete:

```text
.github/workflows/test.yml
```

Do not duplicate virtual-environment or maturin commands in YAML. `scripts/check-python.sh` remains the implementation shared by Make and CI.

### CI environment behavior

`scripts/check-python.sh` must remain suitable for both local and clean CI environments:

- create its virtual environment if absent;
- install only required development dependencies;
- run exactly one `maturin develop`;
- use the active `python`/`pip` consistently after activation;
- fail clearly when `python3`, a virtualenv module, Rust, or required system tools are absent;
- avoid relying on a pre-created CI virtual environment;
- quote paths;
- not publish or build release artifacts.

Optional optimization: support an environment variable that skips dependency reinstall only when the caller guarantees the environment. Do not complicate the default path.

### Path filtering decision

Do not add a third-party path-filter action merely to avoid Python checks on documentation-only changes. The primary goal is a simple authoritative workflow.

Default policy:

- run Rust, Python, and portability jobs for all pushes/PRs to `main`;
- document that local contributors need `make check-python` only for Python-facing/engine changes.

If hosted cost later proves material, path-based job gating is separate follow-up work and must preserve a required aggregate status. It is out of scope here.

## Workstream 3 — Remove optional diagnostic duplication

Reduce `.github/workflows/deep-checks.yml` to one canonical command:

```yaml
- name: Optional broad validation
  run: make check-full
```

Remove the separate `make check-feature-profiles` step because `check-full` already owns it.

Alternatively, change `check-full` not to include profiles and retain separate explicit steps. Do not split ownership across both. Preferred ownership is:

```make
check-full: check
	cargo deny check
	$(MAKE) check-feature-profiles
```

Use `$(MAKE)` instead of literal `make` for recursive invocation so flags and executable selection propagate correctly.

Review other recursive Make calls for the same correction.

The optional workflow remains weekly/manual and non-required. Do not add Python release builds, all-feature matrices, or publication.

## Workstream 4 — Decide and codify the workflow contract

Preferred final inventory:

```text
.github/workflows/ci.yml          # mandatory: Rust, Python, portability
.github/workflows/deep-checks.yml # optional: make check-full
```

This satisfies the original roadmap criterion.

`ci.yml` should contain exactly three conceptual jobs:

```text
rust
python
portability (macOS/Windows matrix)
```

No release, security-scan, artifact-publishing, or evidence jobs may return.

### Workflow validation

Verify:

- push and pull-request triggers target `main`;
- no tag trigger;
- no `workflow_call` release reuse;
- no package registry environment;
- no `id-token: write`;
- no external target variables;
- no `continue-on-error` on mandatory jobs;
- branch protection references `Rust`, `Python`, and both portability contexts or the intended aggregate names.

If branch protection cannot be inspected programmatically, document the exact manual settings check and do not claim it was verified.

### Concurrency

A simple concurrency group may be added:

```yaml
concurrency:
  group: ci-${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true
```

This is optional. Add it only if maintainers want stale-run cancellation. It must not be used to mask failing runs.

## Workstream 5 — Align verification and release documentation

### `docs/VERIFICATION.md`

Correct the platform statement. Since portability jobs run on every pull request, say so directly:

```text
Hosted CI runs narrow cargo check portability jobs on macOS and Windows for every push/PR to main. Contributors generally need local platform testing only for platform-specific changes.
```

Replace aspirational release requirements with executable commands.

Recommended release-readiness contract after Phase G:

```text
make check
make check-python
make check-full
make release-check
```

Then document:

- `make check-full` covers the selected representative feature profiles, not every possible `--all-features` combination;
- unsupported or currently broken all-feature combinations are not release gates unless separately fixed and adopted;
- Python wheel validation covers the artifacts actually built by the manual release process;
- cross-platform wheel production is not claimed unless maintainers perform and record it on each target platform.

Remove these requirements unless implemented as real commands:

```text
cargo check --workspace --all-features
cargo test --workspace --all-features
Python wheel builds on all target platforms
```

Do not weaken a proven requirement silently. If maintainers require all-platform wheels, create a separate packaging roadmap rather than pretending the current manual process satisfies it.

### `docs/RELEASING.md`

Use Corrective Phase G's validated package set, order, release host, and command interface. Make clear that `make release-check` must complete, not partially pass.

### `AGENTS.md` and contributor docs

Keep the quick surface compact:

```bash
make check
make check-python  # when Python-facing code changes
```

List specialist targets only when they execute successfully. Update workflow inventory to two files.

### Roadmap amendment

Update the original roadmap status or add an addendum linking this corrective index. Do not rewrite historical phase descriptions. State that the original closure was reopened for the specific G/H defects.

## Workstream 6 — Correct the closure report

The existing closure report is an engineering record and should be corrected transparently rather than deleted.

### Reopened status

Before final validation, set status to:

```text
Reopened for corrective closure.
```

Add a section explaining:

- CI reduction outcomes remain valid;
- manual release validation did not complete;
- specialist target and documentation inconsistencies were found;
- corrective plans G and H supersede the previous completion claim.

### Baseline counts

Recalculate before/after counts from the actual pre-roadmap base commit and current head.

At minimum record:

- workflow files before and after;
- logical mandatory jobs before and after;
- Python extension build invocations before and after;
- deleted publishing-capable workflows;
- deleted external scan workflows;
- retained optional workflows;
- active Make targets;
- package-release helper scripts.

Do not describe a post-Phase-B state as the original “before” state.

### Validation evidence

Use exact status values:

```text
PASS
FAIL
NOT RUN
BLOCKED
TIMEOUT
```

Only `PASS` satisfies an acceptance criterion.

Record:

- command;
- host operating system and architecture;
- commit SHA;
- outcome;
- relevant log path or concise failure reason.

Do not mark a checkbox complete with parenthetical timeout language.

### Final status

Set the closure report back to `Complete` only when every blocking criterion in the corrective index passes. Non-blocking warnings may remain listed separately.

## Workstream 7 — Add focused command-contract verification

Add a small regression test or script to prevent invalid Make commands from returning unnoticed.

Recommended path:

```text
scripts/check-make-targets.sh
```

Possible checks:

```bash
make -n test
make -n test-fast
make -n test-ci
make -n test-integration
make -n test-slow
make -n check
make -n check-python
make -n check-full
make -n release-check
```

Dry-run parsing catches missing targets but not invalid Cargo flags. Add direct focused execution in the Phase H validation sequence:

```bash
make test-fast
make test-ci
make test-integration
make test-slow
```

`test-slow` may pass with zero ignored tests; record the observed behavior. If full optional suites are too expensive for mandatory CI, keep the script out of `make check` and run it during closure. The target definitions themselves must still be syntactically valid.

A static grep guard may reject known nextest-only fragments in Cargo targets:

```bash
rg -n -- '--retries|--run-ignored|--test '\''\*\.rs' Makefile
```

Use careful quoting or implement the check in Python/shell. This is optional if direct tests and review are sufficient.

## Implementation steps

1. Reopen the closure report and link the corrective index.
2. Repair all advertised Make targets and `.PHONY` declarations.
3. Run each corrected specialist target directly.
4. Move the Python job into `ci.yml` and change it to call `make check-python`.
5. Delete `test.yml`.
6. Remove duplicated feature-profile execution from `deep-checks.yml`.
7. Validate YAML syntax and workflow triggers.
8. Inspect/update branch protection required status names.
9. Correct verification, release, agent, contributor, and Python docs.
10. Complete Corrective Phase G and obtain the final release-check interface/results.
11. Run the full closure validation sequence on the final commit.
12. Recalculate structural before/after counts from the original baseline.
13. Update the closure report with exact evidence and set status accurately.
14. Perform stale-reference and publication searches.

## Required validation sequence

### Primary commands

```bash
make check
make check-python
make check-full
make release-check
```

All four must return zero for closure.

### Specialist Make targets

```bash
make test
make test-fast
make test-ci
make test-integration
make test-slow
make test-feature-matrix
make test-architecture-guards
make check-no-default
make check-feature-profiles
```

If one target is intentionally removed, repository searches must show no active documentation or help reference.

### Make target inventory

```bash
make help
make -qp | sed -n 's/^\([^.#%][^$#[:space:]]*\):.*/\1/p' | sort -u
```

Review rather than treating the raw parser output as authoritative; GNU/BSD make output can differ.

### Workflow inventory

```bash
find .github/workflows -maxdepth 1 -type f -print | sort
```

Expected:

```text
.github/workflows/ci.yml
.github/workflows/deep-checks.yml
```

### Workflow content searches

```bash
rg -n 'maturin develop|pytest|check_python_stub_parity|check_python_types' .github/workflows
rg -n 'make check-python' .github/workflows/ci.yml
rg -n 'make check-feature-profiles' .github/workflows/deep-checks.yml Makefile
rg -n 'tags:|id-token:\s*write|maturin publish|twine upload|cargo publish' .github/workflows
```

Expected:

- Python implementation commands appear in `scripts/check-python.sh`, not duplicated in workflow YAML;
- `ci.yml` calls `make check-python` once;
- feature profiles have one optional owner;
- no publication or tag trigger appears.

### Documentation searches

```bash
rg -n 'test\.yml|all-features|all target platforms|release-check.*timeout|All acceptance criteria met' README.md AGENTS.md CONTRIBUTING.md docs Makefile plans/ci-verification-release-simplification-closure-report.md
```

Every match must be accurate or explicitly historical.

### Commit status

After push, inspect the final commit's workflow results. Required mandatory jobs must complete successfully. If connector/API limitations prevent inspection, record that limitation and use the GitHub UI/manual evidence; do not claim verified branch protection or status success without evidence.

## Acceptance criteria

- `test-fast` is implemented or removed from `.PHONY` and documentation.
- `test-ci` uses valid Cargo syntax and its description no longer claims retries.
- `test-integration` uses a valid target-selection mechanism.
- `test-slow` passes ignored-test arguments after `--`.
- Every target shown by `make help` exists and executes its documented command.
- `.PHONY` declarations match active targets.
- `ci.yml` contains Rust, Python, and portability jobs.
- Python CI invokes `make check-python` or the canonical shared script exactly once.
- `.github/workflows/test.yml` is deleted.
- `.github/workflows/deep-checks.yml` does not rerun feature profiles already owned by `make check-full`.
- Final active workflow inventory is one mandatory workflow plus at most one optional diagnostic workflow.
- No hosted workflow publishes, triggers on tags, scans external targets, or regenerates evidence bundles.
- `docs/VERIFICATION.md` describes actual hosted portability cadence.
- Release-readiness documentation contains only executable, supported commands.
- Unsupported all-feature combinations are not presented as completed release gates.
- Cross-platform wheel claims match actual validation scope.
- Branch protection references only existing mandatory job names, or verification limitations are honestly documented.
- The closure report is marked reopened until all blocking commands pass.
- Before/after structural counts use the true pre-roadmap baseline.
- Partial, timed-out, blocked, or unrun commands are not recorded as passed.
- `make check` passes.
- `make check-python` passes.
- `make check-full` passes.
- `make release-check` passes end-to-end after Corrective Phase G.
- Corrected specialist Make targets pass or are intentionally removed.
- The final closure report references the implementation commit and validation host.
- No product runtime, enforcement, or public API behavior changes are introduced.

## Explicit non-goals

- Reintroducing one job per Python checker.
- Adding path-filter actions or change-detection frameworks.
- Making optional deep checks mandatory.
- Fixing unrelated feature compilation failures.
- Adding cross-platform wheel automation.
- Publishing packages.
- Rewriting historical plans wholesale.
- Eliminating compiler warnings unrelated to these commands.

## Rollback strategy

If merging workflows creates a transient branch-protection issue, update required checks promptly; do not retain duplicate workflows indefinitely as compatibility shims. If invoking `make check-python` exposes a CI-only environment assumption, fix the shared script so local and hosted execution converge. Do not restore duplicated YAML command lists.

## Handoff notes

A smaller implementation model should complete workstreams in order and run the affected target immediately after each edit. The final task is evidence correction, not prose polishing: do not set the closure report to complete until the full four-command primary sequence and all retained specialist targets have actually returned success.
