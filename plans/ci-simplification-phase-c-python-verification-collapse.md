# Phase C Plan: Python Verification Collapse and Evidence Retirement

## Status

Planned. This phase consolidates Python binding verification after the core CI structure from Phase B exists.

## Objective

Replace the current multi-job Python verification graph with one primary Linux/Python 3.12 job that builds the default extension once and runs behavioral, typing, API-consistency, and packaging-smoke checks in the same environment. Remove release-evidence, maturity-gate, skip-budget, and artifact-transfer orchestration from mandatory CI.

The retained system must test observable Python package behavior. It must not preserve process bureaucracy merely because prior release plans introduced it.

## Current problem to eliminate

The current workflow repeatedly performs variants of:

```bash
python -m venv .venv
pip install maturin
maturin develop
```

in separate jobs for capability metadata, architecture guards, stub parity, typing, maturity consistency, feature metadata, evidence generation, maturity guarding, skip budgets, and multiple feature profiles. Jobs exchange JUnit files and generated evidence artifacts, then a synthetic release gate evaluates job status.

This creates repeated compilation, runner setup, artifact plumbing, and failure modes without corresponding behavioral isolation.

## Target Python CI contract

Add one primary job to the mandatory workflow, recommended name:

```text
python
```

Runner and interpreter:

```yaml
runs-on: ubuntu-latest
python-version: "3.12"
```

Expected setup:

```bash
python -m venv .venv
source .venv/bin/activate
python -m pip install --upgrade pip
pip install maturin pytest pytest-timeout mypy pyright
maturin develop --manifest-path crates/eggsec-python/Cargo.toml
```

The job must build the extension only once. All checks that require the installed module execute afterward in the same virtual environment.

## Required checks

Phase A determines the final exact set. The expected retained checks are:

### Behavioral test suite

```bash
EGGSEC_ALLOW_LOOPBACK_FIXTURE=1 \
pytest crates/eggsec-python/tests \
       crates/eggsec-python/python/tests \
       --timeout=60 \
       --tb=short
```

The implementation may split fast and slow tests only within the same job or through explicit pytest markers. Do not create one job per test category.

### Capability and feature metadata

```bash
python scripts/check-python-capability-matrix.py
python scripts/check-python-architecture-guards.py
```

If these scripts duplicate one another, merge their checks into one clearly named script and delete the duplicate. The retained script must fail with direct descriptions of metadata drift.

### Stub parity

```bash
python scripts/check_python_stub_parity.py
```

Retain because a PyO3 module can import successfully while its checked-in type surface drifts.

### Type checks

```bash
bash scripts/check_python_types.sh
```

The type-check script should use the same environment and installed package. Avoid reinstalling the package or creating nested virtual environments.

### Installed-package smoke

At least one test must confirm that imports and resource files work from an installed build rather than from accidental source-tree imports. `maturin develop` is acceptable for routine CI if the test verifies the resolved module path. A clean wheel installation belongs in optional/release validation unless Phase A identifies a defect class that cannot be caught otherwise.

Minimum assertions:

- `import eggsec` succeeds;
- `eggsec.__version__` and `build_info()` are coherent;
- `eggsec.features()` returns the documented type;
- `__init__.pyi` and `py.typed` are installed where expected;
- one representative stable operation can execute against a deterministic loopback fixture;
- report serialization and redaction behavior are exercised.

## Checks to retire from mandatory CI

Remove the following job concepts from the mandatory graph unless Phase A documents a distinct behavioral defect class and consolidates it into the one Python job:

- `python-capability-matrix` as a separate extension build;
- `python-architecture-guards` as a separate extension build;
- `python-stub-parity` as a separate extension build;
- `python-type-check` as a separate extension build;
- `python-maturity-consistency` as a separate extension build;
- `python-feature-metadata` as a separate extension build;
- `python-profile-manifest` as a separate job;
- `python-evidence-bundle`;
- `python-maturity-guard`;
- `python-skip-budget`;
- `python-release-gate`;
- JUnit upload/download solely to feed evidence or skip-budget scripts;
- evidence artifact retention;
- per-feature `maturin develop` matrix entries in routine CI.

A lightweight profile-manifest JSON schema/consistency check may run in the primary job if the manifest remains useful for local builds. It must not justify a separate runner.

## Evidence and maturity script disposition

Review and normally remove or archive the following when they exist solely to support the retired CI process:

```text
scripts/build_python_release_evidence.py
scripts/check_maturity_guard.py
scripts/python_skip_budget.py
scripts/generate_python_compatibility_baseline.py
scripts/validate_python_release_candidate.sh
```

For each script:

1. identify whether it protects observable runtime/API behavior;
2. move any valuable assertion into a direct Rust/Python test or a retained metadata checker;
3. delete the orchestration/evidence shell once no callers remain;
4. remove generated artifact paths and documentation references.

Do not keep a script as “optional” solely to avoid deletion. Optional checks must have a named maintainer use case.

## Compatibility policy

`check_python_compatibility.py` may remain under `make check-full` or manual release validation if it compares a durable stable API against an intentionally maintained baseline. It must not be a mandatory per-commit gate if:

- the package is pre-1.0 and the baseline changes frequently;
- baseline regeneration is routine rather than exceptional;
- failures merely indicate that generated metadata was not refreshed;
- the same breakage is caught by stub parity or behavioral tests.

If retained, document who updates the baseline and under what versioning policy. Otherwise convert high-value compatibility assertions into explicit tests and remove the baseline machinery.

## Resource and redaction checks

Redaction correctness is security-relevant and should remain directly tested. Resource-budget tests require more discrimination.

Retain in mandatory CI only resource tests that are:

- deterministic on GitHub-hosted Linux runners;
- short;
- based on generous regression thresholds rather than host-specific absolute performance;
- associated with known leak classes such as file descriptors, threads, sockets, or temporary directories.

Move memory-growth, repository-scale, long concurrency, platform-sensitive, and performance budget tests to `make check-full` or the optional diagnostic workflow.

Do not enforce resource behavior by counting skipped tests.

## Feature-profile policy

Routine Python CI builds only the default supported profile. Optional features should be covered through one of:

- Rust compile checks under `make check-full`;
- a small manually dispatched wheel-profile smoke set;
- direct tests that do not require rebuilding the extension for every feature;
- local pre-release validation for the specific artifact being published.

Do not rebuild Python for `nse`, `db-pentest`, `web-proxy`, `mobile`, `headless-browser`, `daemon-client`, `packet-inspection`, and `stress-testing` on every relevant push merely to prove each feature imports.

If a feature profile is claimed as a published artifact, test that profile during the manual release for that artifact.

## Makefile changes

Implement:

```make
check-python:
	python -m venv .venv-ci
	. .venv-ci/bin/activate && pip install ...
	. .venv-ci/bin/activate && maturin develop --manifest-path crates/eggsec-python/Cargo.toml
	. .venv-ci/bin/activate && pytest ...
	. .venv-ci/bin/activate && python scripts/check-python-capability-matrix.py
	. .venv-ci/bin/activate && python scripts/check-python-architecture-guards.py
	. .venv-ci/bin/activate && python scripts/check_python_stub_parity.py
	. .venv-ci/bin/activate && bash scripts/check_python_types.sh
```

A helper script such as `scripts/check-python.sh` is acceptable if it is readable, uses one environment/build, has strict shell error handling, and is called both locally and in CI. Do not hide multiple profile builds inside it.

Remove Phase F/evidence-oriented Make targets whose only purpose was the old gate graph:

- `build-python-evidence`;
- `test-python-phase-f` if it is process-oriented rather than a useful test group;
- compatibility baseline generation targets without a durable policy;
- skip-budget targets.

Retain direct redaction/resource test targets only when they remain useful to maintainers.

## Implementation steps

1. Map all current Python jobs to retained commands or removal using Phase A.
2. Create one virtual-environment setup path and prove all retained checks run after one `maturin develop`.
3. Add `make check-python` or the canonical helper script.
4. Add the one Python job to the mandatory workflow.
5. Remove the old Python job DAG from `test.yml`.
6. Delete artifact upload/download steps used only for evidence or skip budgets.
7. Move high-value assertions out of evidence/maturity scripts into direct tests.
8. Delete scripts, manifests, baselines, and generated-output configuration with no remaining caller.
9. Update Python docs and `AGENTS.md` to use `make check-python`.
10. Run repository-wide searches for old job names and evidence commands.

## Validation commands

```bash
make check-python
```

Then verify one-build behavior. A simple implementation-time check may use clean logs:

```bash
rm -rf target/debug target/maturin .venv-ci
make check-python 2>&1 | tee /tmp/eggsec-python-check.log
rg -n "maturin develop|Compiling eggsec-python" /tmp/eggsec-python-check.log
```

The exact compilation log is cache-dependent; the requirement is that the command invokes `maturin develop` once, not that Cargo prints one compile line.

Repository searches:

```bash
rg -n "python-evidence-bundle|python-maturity-guard|python-skip-budget|python-release-gate" .github Makefile scripts docs AGENTS.md
rg -n "build_python_release_evidence|check_maturity_guard|python_skip_budget" .
rg -n "maturin develop" .github/workflows
```

## Acceptance criteria

- Mandatory Python CI contains one primary Linux job.
- The job invokes `maturin develop` exactly once.
- Behavioral tests, capability/architecture checks, stub parity, and type checks run in the same virtual environment.
- No Python job downloads JUnit results from another job.
- No mandatory job generates or uploads a release evidence bundle.
- No maturity guard, skip-budget gate, or synthetic release-gate job remains.
- Default-profile Python tests still exercise enforcement/error/redaction behavior and representative stable operations.
- Type stubs and `py.typed` installation are verified.
- Long or platform-sensitive resource tests are optional rather than silently skipped and budget-counted.
- Python feature profiles are not rebuilt as a routine matrix.
- `make check-python` reproduces the CI job locally.
- Deleted scripts have no remaining references.
- Public Python behavior and maturity classifications are not changed merely to satisfy CI.

## Explicit non-goals

- Building all release wheels.
- Publishing to TestPyPI or PyPI.
- Changing Python support-version policy.
- Converting the project to `abi3`.
- Promoting provisional APIs.
- Removing substantive redaction, enforcement, or API-parity tests.
- Introducing tox, nox, cibuildwheel, or another orchestration framework unless the current simple command cannot meet a demonstrated requirement.

## Rollback strategy

If consolidation reveals that one check genuinely requires an isolated environment, add a second step or at most one narrowly scoped job with documented ownership. Do not restore the prior one-job-per-script model. If a retired evidence script contained a unique behavioral assertion, recover that assertion as a direct test rather than restoring evidence generation.

## Handoff notes

The implementation report must include a before/after count of `maturin develop` or wheel-build invocations for an ordinary Python-related push. It must also list every removed Python gate and the retained test, if any, that protects its former defect class.