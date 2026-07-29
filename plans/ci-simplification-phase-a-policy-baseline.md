# Phase A Plan: Verification Policy and Baseline Contract

## Status

Planned. This phase is inventory and policy work only. It must not remove workflows, alter release behavior, or reduce test coverage.

## Objective

Create a precise, repository-backed contract for what Eggsec considers mandatory merge verification, optional diagnostic validation, and manual release preparation. Measure the current apparatus, classify every existing job and script, and define the target command surface before implementation begins.

This phase exists to prevent the simplification effort from becoming an unreviewed sequence of YAML deletions. Later phases must be able to point to an explicit disposition for every current check.

## Required outcome

At completion, maintainers and implementation agents must be able to answer all of the following without interpreting multiple workflows:

- Which commands must pass before an ordinary change is merged?
- Which checks are Linux-only, and which portability checks run on macOS or Windows?
- Which checks are optional, slow, privileged, system-dependent, scheduled, or release-only?
- Which current checks are duplicates?
- Which scripts are behavioral tests, architecture guards, release-process evidence, or obsolete process artifacts?
- Which critical invariants must remain protected after the CI graph is collapsed?
- Which package registries are used, and which final publish commands remain manual?

## Scope

Inspect and classify at minimum:

```text
.github/workflows/test.yml
.github/workflows/deep-checks.yml
.github/workflows/security-scan.yml
.github/workflows/python-wheels.yml
.github/workflows/testpypi-rehearsal.yml
.github/workflows/release.yml
.gitlab-ci.yml
Makefile
AGENTS.md
scripts/check-architecture-guards.sh
scripts/check_python_types.sh
scripts/check-python-capability-matrix.py
scripts/check-python-architecture-guards.py
scripts/check_python_stub_parity.py
scripts/validate_python_profiles.py
scripts/run_python_profile.py
scripts/build_python_release_evidence.py
scripts/check_maturity_guard.py
scripts/python_skip_budget.py
scripts/check_python_compatibility.py
scripts/generate_python_compatibility_baseline.py
scripts/validate_python_release_candidate.sh
crates/eggsec-python/validation/profiles.json
crates/eggsec-python/wheel-profiles.json
docs/python/packaging.md
docs/python/versioning.md
docs/python/README_1_0_CHECKLIST.md
```

Also inspect repository branch protection or documented required status names if accessible. Do not assume current workflow job names are unreferenced externally.

## Deliverables

### 1. Verification contract document

Add a canonical document, recommended path:

```text
docs/VERIFICATION.md
```

It must define:

- `make check` as the mandatory Rust/Linux contributor contract;
- `make check-python` as the mandatory Python/Linux contract for Python-facing changes;
- narrow macOS and Windows portability expectations;
- `make check-full` as optional broad validation;
- which changes require Python checks;
- which changes require optional feature/system checks before release;
- the distinction between merge readiness and release readiness;
- that release publication is always manual and is not part of CI.

The document should be concise enough to remain the authoritative entry point. Detailed commands can live behind Make targets, but their ownership must be explicit.

### 2. Current-job disposition table

Add a temporary or durable planning table to this plan or a companion document. For every job in every current workflow, record:

```text
workflow
job name
trigger
current command(s)
platform
estimated duplicate builds
protected defect class
proposed disposition
replacement command/job
reason
```

Allowed dispositions:

- `mandatory-retain`
- `mandatory-merge`
- `optional-move`
- `release-local`
- `example-relocate`
- `remove-obsolete`

Do not use ambiguous outcomes such as “review later.” If evidence is insufficient, choose `optional-move` and state what future evidence would justify making it mandatory.

### 3. Script ownership table

Classify each verification/release script as one of:

- behavioral test runner;
- static architecture guard;
- metadata consistency check;
- packaging smoke check;
- manual release helper;
- historical evidence/process helper;
- obsolete.

Record whether the script requires an installed extension, a built wheel, JUnit artifacts, Git metadata, credentials, network access, system packages, or platform-specific dependencies.

### 4. Critical invariant list

Identify the smallest direct checks that protect Eggsec's high-value invariants. The list must include an explicit disposition for:

- workspace no-default compilation;
- central enforcement behavior;
- strict-surface scope semantics;
- operation metadata consistency;
- command registry consistency;
- tool registration consistency;
- enforced-dispatch regression coverage;
- output report-envelope stability;
- architecture dependency boundaries;
- Python API/stub parity;
- Python feature/capability metadata consistency;
- Python error/redaction behavior;
- installed-package importability.

The phase must distinguish static grep guards from behavioral tests. A static guard should remain mandatory only when it protects a boundary that is not cheaply expressible as a Rust or Python test.

### 5. Baseline measurements

Record the current state using reproducible measurements where GitHub data is available:

- number of workflow files;
- number of jobs in each workflow;
- number of Rust matrix entries;
- number of Python extension builds per ordinary Python-related push;
- number of artifact upload/download handoffs;
- number of scheduled workflows;
- number of publishing-capable jobs;
- number of unique mandatory tools installed in CI;
- approximate critical-path duration from recent representative runs, if available;
- common CI-only repair categories from recent commits.

Do not manufacture timing data. Where historical run timing is unavailable, record structural counts and mark duration as unavailable.

### 6. Target workflow sketch

Document the intended mandatory workflow shape before implementation. Recommended logical structure:

```yaml
jobs:
  rust:
    # fmt, check, clippy, core tests, selected invariant tests

  python:
    # one venv, one maturin develop, pytest and static/API checks

  portability:
    # narrow macOS/Windows checks
```

The sketch must state that job decomposition is for operating-system isolation or meaningful dependency separation only—not one status check per command.

## Implementation steps

1. Enumerate workflow jobs and matrix expansions from the checked-in YAML.
2. Trace each workflow command to its Make target, script, test file, or release action.
3. Identify exact duplicate compilations and tests across jobs.
4. Review recent CI-fix commits to identify failures caused by orchestration rather than product defects.
5. Define mandatory defect classes and map one retained check to each.
6. Draft `docs/VERIFICATION.md` and the disposition tables.
7. Verify all referenced commands exist in the current repository; proposed future commands must be clearly marked as targets to be implemented in later phases.
8. Review the classification for accidental weakening of safety/enforcement checks.
9. Commit only documentation/planning changes in this phase.

## Decision rules

Use these rules consistently:

- A feature configuration is not automatically a separate product. Retain representative topology coverage rather than one job per feature name.
- A release artifact test is not required on every commit if `maturin develop` and installed-package tests protect the same code path sufficiently for iteration.
- Coverage generation is diagnostic unless the repository enforces a meaningful, stable threshold.
- Multiple advisory tools do not provide proportional value when they consume the same advisory data.
- Evidence generation is not behavioral verification.
- A skip-count budget is not a substitute for explicitly scoped tests and documented feature availability.
- External network scans are not repository correctness tests.
- A previously published binary cannot validate the current commit.
- Cross-platform release builds are release preparation, not routine portability smoke tests.

## Validation

This phase is validated primarily by completeness and internal consistency.

Run repository searches such as:

```bash
find .github/workflows -maxdepth 1 -type f -print
rg -n "maturin (develop|build|publish)|twine upload|cargo publish|gh release|pypi|testpypi" .github .gitlab-ci.yml Makefile scripts docs AGENTS.md
rg -n "build_python_release_evidence|check_maturity_guard|python_skip_budget|validate_python_release_candidate" .
rg -n "cargo (check|test|clippy|build)" .github/workflows Makefile
```

If branch protection status names are not accessible, record that as an explicit pre-implementation operational check for Phase B rather than guessing.

## Acceptance criteria

- `docs/VERIFICATION.md` exists and names one canonical mandatory Rust command and one canonical Python command.
- Every existing workflow job has one explicit disposition.
- Every release or evidence script has an explicit owner/disposition.
- The baseline records structural counts without unsupported timing claims.
- The critical invariant list maps each retained defect class to a direct check.
- Publishing-capable paths are fully enumerated, including PyPI, TestPyPI, crates.io, and GitHub Releases where applicable.
- The target workflow sketch contains no publishing or release-artifact generation.
- No workflow or script is deleted in this phase.
- No runtime, enforcement, public API, or feature behavior changes are made.
- Later phases can be executed without rediscovering the current CI graph.

## Out of scope

- Editing or deleting workflow YAML.
- Changing branch protection.
- Rewriting the Makefile.
- Removing Python evidence scripts.
- Publishing any package.
- Fixing unrelated failing tests.
- Adding new CI providers or build systems.

## Handoff notes

This phase should be executable by a smaller model or junior maintainer because it is observational. Require exact file references and command ownership. Do not accept a generic recommendation document that lacks a one-to-one disposition map; later deletion work depends on that map.