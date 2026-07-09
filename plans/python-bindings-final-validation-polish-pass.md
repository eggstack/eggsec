# Python Bindings Final Validation and Polish Pass

## Objective

Run the final validation/polish pass for the `eggsec-python` track after the active API scope-enforcement fix. The current shape is good alpha-quality architecture: Python bindings are separated into `crates/eggsec-python`, active APIs now require scope, feature-gated modules are absent by default, docs are honest about experimental status, and CI workflow shape has improved.

This pass should prove the work. It should not add new feature surface. The goal is to convert the Python binding track from "appears structurally correct" to "validated experimental package candidate" by closing test, workflow, async, and documentation gaps.

## Current state

Recent implementation claims:

- `load_test_http`, `validate_waf`, `fuzz_http`, and async variants now require a `Scope` at the Python layer.
- `Client` and `AsyncClient` methods enforce the client's internal scope before dispatch.
- Type stubs and docs were updated for the new active API signatures.
- CI no longer installs a phantom `[dev]` extra and now runs both Rust-adjacent and Python package tests.
- Enforcement tests now include denied-scope tests for active APIs and cap validation.
- Implementation reported `159 tests pass, 6 pre-existing network failures unchanged`.

Remaining uncertainty:

- GitHub Actions status was not visible through the connector.
- The 6 network failures are not triaged.
- Async awaitability and cancellation semantics are still not fully proven.
- Release checklist gates are not fully marked/recorded.
- Optional feature matrix needs explicit pass/fail documentation.

## Non-goals

Do not add new Python-exposed Eggsec tools.

Do not publish to PyPI.

Do not expand the default wheel feature set.

Do not weaken scope enforcement for convenience.

Do not hide failures with broad skips or `|| true`.

Do not change the public API unless tests reveal a real bug or signature mismatch.

## Workstream 1: Run and record Rust validation matrix

### Required commands

Run from the repository root:

```bash
cargo check -p eggsec-python
cargo test -p eggsec-python
cargo check -p eggsec-python --features full-no-system
```

Run selected optional feature checks:

```bash
cargo check -p eggsec-python --features websocket
cargo check -p eggsec-python --features git-secrets
cargo check -p eggsec-python --features sbom
cargo check -p eggsec-python --features container
cargo check -p eggsec-python --features db-pentest
cargo check -p eggsec-python --features web-proxy
cargo check -p eggsec-python --features mobile
cargo check -p eggsec-python --features packet-inspection
cargo check -p eggsec-python --features stress-testing
cargo check -p eggsec-python --features nse
cargo check -p eggsec-python --features daemon-client
```

Optionally run:

```bash
cargo check -p eggsec-python --all-features
```

If `--all-features` fails because of mutually incompatible or system-dependent features, document that fact explicitly and prefer the supported feature matrix over pretending all-features is a required gate.

### Output artifact

Add or update:

```text
crates/eggsec-python/VALIDATION.md
```

Include:

- command;
- result: pass/fail/skipped;
- date;
- platform;
- short failure reason if failed;
- whether failure blocks default wheel candidate.

### Acceptance criteria

Default Rust checks pass.

`full-no-system` passes or is corrected.

Every optional feature has an explicit documented status.

No feature is described as supported in docs unless it compiles or is clearly labeled pending/experimental with known blockers.

## Workstream 2: Python local build, import, and wheel smoke

### Required commands

From `crates/eggsec-python`:

```bash
python -m venv .venv
source .venv/bin/activate
pip install -U pip maturin pytest pytest-timeout
maturin develop
python -c "import eggsec; print(eggsec.__version__, eggsec.features())"
pytest -q tests/ python/tests/
maturin build --release
```

Then test the built wheel in a clean environment:

```bash
python -m venv /tmp/eggsec-wheel-test
source /tmp/eggsec-wheel-test/bin/activate
pip install target/wheels/eggsec-*.whl
python - <<'PY'
import eggsec
print(eggsec.__version__)
print(eggsec.build_info())
assert eggsec.has_feature("core")
PY
```

Run installed-wheel smoke checks:

```bash
python - <<'PY'
import eggsec
for name in eggsec.__all__:
    assert hasattr(eggsec, name), name
print("__all__ OK")
PY
```

Run a report serialization smoke test from the installed wheel.

Run a localhost scanner smoke test from the installed wheel with a local TCP fixture if possible. Avoid depending on ports 22/80/443 being open.

### Acceptance criteria

`maturin develop` works.

Default pytest suite passes, excluding explicitly marked external-network tests if necessary.

Release wheel builds.

Clean venv wheel install succeeds.

Installed-wheel import, `__all__`, scanner, and report smoke tests pass.

## Workstream 3: Triage the 6 network failures

### Problem

The latest implementation reported `159 tests pass, 6 pre-existing network failures unchanged`. Those failures need triage before the package can be considered a stable experimental candidate.

### Tasks

Identify the failing tests by name.

Classify each failure:

1. real bug;
2. flaky external-network dependency;
3. environment-dependent DNS/TLS behavior;
4. test expectation mismatch;
5. intentionally unsupported platform behavior.

Fix real bugs.

For external-network tests, mark them explicitly, for example:

```python
@pytest.mark.network
```

Do not run external-network tests in the default local or wheel smoke suite. Provide a separate command:

```bash
pytest -m network
```

If tests rely on `example.com`, public TLS state, public DNS records, or external WAF behavior, prefer local fixtures or deterministic mocks.

### Acceptance criteria

Default pytest suite has zero failures.

Network tests are either fixed, isolated behind `pytest -m network`, or replaced with deterministic local fixtures.

`VALIDATION.md` records the network-test status.

Docs do not imply external-network tests are part of the default release gate.

## Workstream 4: Async API proof

### Tasks

Audit `runtime_async.rs`, `AsyncClient`, and top-level async functions.

Add tests that prove normal Python usage works:

```python
@pytest.mark.asyncio
async def test_async_scan_ports_awaitable(): ...

@pytest.mark.asyncio
async def test_async_load_test_denied_scope_awaitable(): ...

@pytest.mark.asyncio
async def test_async_validate_waf_denied_scope_awaitable(): ...

@pytest.mark.asyncio
async def test_async_fuzz_http_denied_scope_awaitable(): ...

@pytest.mark.asyncio
async def test_async_exception_propagates_as_eggsec_error(): ...
```

If `await eggsec.async_scan_ports(...)` is not valid Python syntax against the current returned object, either fix the await protocol or update docs and names to avoid claiming asyncio-native behavior.

Cancellation semantics:

- Do not overpromise full cancellation propagation.
- Add a small cancellation smoke test if possible.
- Keep docs explicit that the hand-rolled `PyFuture` has limited cancellation propagation if that remains true.

### Acceptance criteria

Documented async examples execute.

Async denied-scope calls fail before network dispatch.

Async exceptions propagate as Python exceptions.

Docs accurately describe awaitability and cancellation limits.

## Workstream 5: Public namespace and stubs polish

### Tasks

Run import namespace checks from both `maturin develop` and installed wheel:

```python
import eggsec
missing = [name for name in eggsec.__all__ if not hasattr(eggsec, name)]
assert not missing, missing
```

Check type stubs against runtime signatures for changed active APIs:

- `load_test_http`;
- `async_load_test_http`;
- `validate_waf`;
- `async_validate_waf`;
- `fuzz_http`;
- `async_fuzz_http`;
- `Client.load_test_http`;
- `Client.validate_waf`;
- `Client.fuzz_http`;
- `AsyncClient.load_test_http`;
- `AsyncClient.validate_waf`;
- `AsyncClient.fuzz_http`.

If feasible, add a lightweight script:

```text
scripts/check_eggsec_python_exports.py
```

The script should import the installed package and verify:

- `__all__` exists;
- every `__all__` name exists;
- feature-gated symbols are absent by default;
- required default symbols are present;
- active top-level APIs require `scope`.

### Acceptance criteria

No `__all__` drift.

Stubs match runtime signatures.

Default optional feature symbols remain absent unless enabled.

Export checker is documented or included in validation commands.

## Workstream 6: GitHub Actions verification

### Tasks

Confirm `.github/workflows/python-wheels.yml` is valid YAML.

Ensure primary wheel test path:

- builds default wheels;
- downloads only matching wheel artifact;
- installs exact matching wheel;
- runs import smoke;
- runs scanner smoke;
- runs report smoke;
- runs pytest without ignored failures.

Ensure publish path remains manual-only and safe.

If final PyPI publish remains in the same workflow as TestPyPI, require an explicit manual input gate before final PyPI publish. Prefer separate TestPyPI and PyPI workflows if time allows.

Run or trigger the workflow if possible.

Record results in `VALIDATION.md`:

- workflow name;
- commit SHA;
- run URL if available;
- pass/fail;
- failed job if any.

### Acceptance criteria

Workflow YAML validates.

No accidental PyPI publish path exists on push/PR.

Wheel workflow passes or failure is explicitly documented with next fix.

## Workstream 7: Release checklist closure

### Tasks

Update `crates/eggsec-python/RELEASE_CHECKLIST.md` so it reflects the actual validation commands.

Do not check boxes for commands that were not run.

If `VALIDATION.md` records passing results, reference it from the checklist.

Add a `Not yet PyPI-ready` section if any gate remains open.

### Acceptance criteria

Release checklist is accurate.

Checklist distinguishes completed gates from remaining gates.

PyPI publish remains blocked until all pre-release gates are complete.

## Workstream 8: Documentation polish

### Tasks

Review and correct:

- root `README.md` Python section;
- `crates/eggsec-python/README.md`;
- `docs/python/index.md`;
- `docs/python/installation.md`;
- `docs/python/quickstart.md`;
- `docs/python/sync-api.md`;
- `docs/python/async-api.md`;
- `docs/python/scope-and-safety.md`;
- `docs/python/waf.md`;
- `docs/python/packaging.md`;
- `docs/python/api-reference.md`.

Ensure:

- PyPI status remains pre-release/not published unless release actually happened;
- active APIs always show scope or scoped client usage;
- examples do not scan public third-party targets by default;
- network tests are described separately from default tests;
- async cancellation limitations are explicit;
- optional feature modules remain labeled experimental/feature-gated.

### Acceptance criteria

Docs match runtime signatures.

Docs match validation status.

No example encourages unscoped active testing.

## Suggested implementation order

1. Run default Rust checks and fix any immediate breakage.
2. Run `maturin develop` and pytest; identify the 6 network failures.
3. Isolate/fix network tests so default pytest is clean.
4. Add async awaitability and denied-scope tests.
5. Run wheel build and clean venv install smoke.
6. Add/update `VALIDATION.md` with command results.
7. Run namespace/stub checker and fix drift.
8. Verify workflow YAML and publish gating.
9. Update release checklist and docs.

## Final acceptance criteria

This pass is complete when:

- default Rust checks pass;
- default Python tests pass without external-network failures;
- wheel build/install smoke passes in a clean venv;
- active APIs remain scoped and tested;
- async examples are proven or docs are corrected;
- namespace/stub drift is resolved;
- CI workflow is valid and either green or has precisely documented remaining failures;
- release checklist accurately reflects gate status;
- docs remain honest about experimental/not-yet-PyPI status.

At that point, `eggsec-python` can be considered a validated experimental package candidate. It should still not be published to PyPI until the release checklist is fully completed and reviewed.
