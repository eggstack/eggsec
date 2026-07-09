# Python Bindings Active API Scope and CI Verification Pass

## Objective

Complete the remaining stabilization work for `eggsec-python` after the corrective verification pass. The previous pass improved compilation issues, docs honesty, CI artifact selection, feature-gating tests, and release checklist coverage, but the repo still has two release-blocking issues:

1. Some active Python APIs expose load testing, WAF validation, and HTTP fuzzing without accepting or enforcing Python-side `Scope`.
2. The wheel/CI path exists but has not been proven green, and the PyPI workflow still needs a final safety cleanup.

This pass should not add new Eggsec capabilities. It should make the existing Python API safer, testable, and release-gated.

## Non-goals

Do not add new tool bindings.

Do not publish to PyPI.

Do not broaden default wheel features.

Do not redesign the entire Python API unless required to enforce scope correctly.

Do not paper over failures with skipped tests or `|| true`.

## Workstream 1: Add Python-side scope enforcement for active APIs

### Problem

`tests/test_enforcement.py` currently documents that `load_test_http`, `validate_waf`, and `fuzz_http` do not accept a `scope` parameter at the Python binding level. This is not acceptable for a host-language binding. The Python library is not the CLI, and it cannot rely on CLI-layer enforcement for functions that can generate significant traffic or active test payloads.

### Required API changes

Add scope-aware client methods as the canonical API:

```python
client = eggsec.Client(scope=eggsec.Scope.allow_hosts(["staging.example.com"]), mode="manual")

client.load_test_http(
    "https://staging.example.com",
    total_requests=100,
    concurrency=5,
    timeout_secs=5,
)

client.validate_waf(
    "https://staging.example.com",
    profile="owasp-light",
    max_requests=100,
    rate_limit_per_sec=2,
)

client.fuzz_http(
    "https://staging.example.com/search",
    parameter="q",
    payload_set="xss-basic",
    max_requests=100,
    rate_limit_per_sec=5,
)
```

Add async equivalents on `AsyncClient` if the corresponding top-level async functions already exist:

```python
await async_client.load_test_http(...)
await async_client.validate_waf(...)
await async_client.fuzz_http(...)
```

For top-level functions, choose one of these strategies:

Preferred strategy:

```python
eggsec.load_test_http(url, *, scope=scope, ...)
eggsec.validate_waf(url, *, scope=scope, ...)
eggsec.fuzz_http(url, *, scope=scope, ...)
```

The top-level functions must require `scope=` for active APIs. Do not provide an unrestricted default.

Alternative strategy if changing signatures is too disruptive:

- Deprecate or hide top-level active functions.
- Keep only `Client` / `AsyncClient` active methods as public documented API.
- If top-level functions remain for compatibility, require explicit `allow_unscoped_manual=True` and emit `DeprecationWarning`; this is less preferred.

### Enforcement requirements

Before invoking engine work:

- parse URL/target;
- extract host;
- enforce host against `Scope`;
- validate any port implied by the URL if scope tracks ports;
- reject denied targets with `eggsec.EnforcementError`;
- in automation mode, do not honor any manual override-like path.

Apply this to:

- `load_test_http`;
- `validate_waf`;
- `fuzz_http`;
- async equivalents;
- any aliases added in Phase F for these functions.

### Cap validation requirements

Active functions must require bounded execution:

`load_test_http`:

- `total_requests > 0`;
- `concurrency > 0`;
- `timeout_secs > 0`;
- optional duration must be > 0 if supported;
- no implicit unlimited mode.

`validate_waf`:

- explicit profile or documented safe default;
- `max_requests > 0`;
- `rate_limit_per_sec > 0` or conservative default;
- scoped target required.

`fuzz_http`:

- explicit parameter or target component;
- explicit payload set or bounded generated payload list;
- `max_requests > 0`;
- `rate_limit_per_sec > 0` or conservative default;
- scoped target required.

### Tests

Replace the current “no scope parameter” tests with denial tests:

```python
def test_load_test_http_out_of_scope():
    scope = eggsec.Scope.allow_hosts(["allowed.local"])
    client = eggsec.Client(scope)
    with pytest.raises(eggsec.EnforcementError):
        client.load_test_http("http://evil.local", total_requests=1, concurrency=1, timeout_secs=1)
```

Add equivalent tests for:

- top-level `load_test_http(..., scope=scope)` denied;
- `Client.load_test_http` denied;
- `AsyncClient.load_test_http` denied if async API exists;
- `validate_waf` top-level/client/async denied;
- `fuzz_http` top-level/client/async denied.

Add cap validation tests for top-level and client methods.

Add positive tests only against localhost/local fixtures, not public targets.

### Acceptance criteria

No Python-exposed active API can run without explicit scope or a scoped client.

Denied active API calls raise `eggsec.EnforcementError` before network activity.

The old tests asserting absence of scope parameters are removed.

Docs show scoped client usage for active APIs.

## Workstream 2: Repair and harden Python public namespace

### Tasks

Run a default-build import audit:

```python
import eggsec
for name in eggsec.__all__:
    assert hasattr(eggsec, name), name
```

Ensure active APIs in `__all__` match the new scoped signatures.

Ensure optional feature symbols remain absent by default unless implemented as documented shims that raise `FeatureUnavailableError`.

If type stubs advertise active APIs, update signatures to require `scope` or show only `Client`/`AsyncClient` methods.

### Acceptance criteria

`import eggsec` passes in default build.

`from eggsec import *` passes in default build.

Every `__all__` symbol exists.

Type stubs match runtime signatures for active APIs.

## Workstream 3: CI workflow final cleanup

### Problem

The Python wheel workflow was improved, but the publish section still needs final safety cleanup. The tail of the workflow should not contain an empty or malformed `with:` block. The workflow should avoid accidental PyPI publication before all release gates are green.

### Tasks

Audit `.github/workflows/python-wheels.yml`.

Fix any malformed YAML at the final PyPI publish step. If `with:` is empty, remove it.

Prefer splitting final publish from TestPyPI smoke validation:

Option A: separate manual workflows:

- `python-wheels.yml`: build/test wheels only.
- `python-testpypi.yml`: publish to TestPyPI and validate install.
- `python-pypi.yml`: publish to PyPI after explicit manual dispatch.

Option B: one workflow with explicit manual input:

```yaml
workflow_dispatch:
  inputs:
    publish_pypi:
      type: boolean
      default: false
```

Then only run final PyPI publish when `publish_pypi == true` and TestPyPI smoke has passed.

Ensure the test job installs the exact matching platform wheel. The current `wheel-${{ matrix.target }}` artifact approach is acceptable; keep it.

Remove `pip install "$WHEEL[dev]"` unless the wheel actually defines a `dev` extra. If it does not, install the wheel directly and then install pytest separately.

### Acceptance criteria

Workflow YAML is valid.

Final PyPI publish cannot happen accidentally during normal push/PR.

TestPyPI install smoke must pass before PyPI publish path is available.

Default wheel test job installs the matching platform wheel.

No primary test path ignores pytest failures.

## Workstream 4: Prove builds locally and in CI

### Required local commands

Run and fix:

```bash
cargo check -p eggsec-python
cargo test -p eggsec-python
cargo check -p eggsec-python --features full-no-system
```

Run selected active/optional feature checks:

```bash
cargo check -p eggsec-python --features websocket
cargo check -p eggsec-python --features git-secrets
cargo check -p eggsec-python --features sbom
cargo check -p eggsec-python --features container
cargo check -p eggsec-python --features nse
cargo check -p eggsec-python --features stress-testing
cargo check -p eggsec-python --features packet-inspection
cargo check -p eggsec-python --features daemon-client
```

If a feature fails because of unavailable system dependencies, document it in the release checklist and feature matrix instead of silently ignoring it.

Run Python build/install smoke:

```bash
cd crates/eggsec-python
python -m venv .venv
source .venv/bin/activate
pip install -U pip maturin pytest
maturin develop
python -c "import eggsec; print(eggsec.__version__, eggsec.features())"
pytest -q
maturin build --release
python -m venv /tmp/eggsec-wheel-test
source /tmp/eggsec-wheel-test/bin/activate
pip install target/wheels/eggsec-*.whl
python -c "import eggsec; print(eggsec.build_info())"
```

### CI proof

After fixes, confirm GitHub Actions status for the relevant workflow. If the connector cannot see workflow runs, record the absence honestly in the final implementation note and ensure local commands are recorded in `RELEASE_CHECKLIST.md` or a verification log.

### Acceptance criteria

Default Rust and Python builds pass.

`full-no-system` passes or is corrected.

Feature failures are documented with reasons.

Wheel install smoke passes from a clean venv.

CI workflow is green, or local validation is explicitly recorded if CI did not trigger.

## Workstream 5: Async behavior verification

### Tasks

Given the current known limitation of the hand-rolled `PyFuture`, add pragmatic tests that prove the advertised API works.

Required tests:

```python
@pytest.mark.asyncio
async def test_async_scan_ports_awaitable(): ...

@pytest.mark.asyncio
async def test_async_active_api_scope_error(): ...

@pytest.mark.asyncio
async def test_async_exception_propagates(): ...
```

If cancellation is not natively propagated, keep docs honest. Do not claim full asyncio cancellation semantics.

If `await eggsec.async_scan_ports(...)` does not work with normal Python await syntax, either fix the await protocol or rename/re-document the API as pollable rather than asyncio-native.

### Acceptance criteria

Normal `await` syntax works for documented async APIs.

Async denied-scope errors propagate as `eggsec.EnforcementError`.

Docs accurately describe cancellation limitations.

## Workstream 6: Documentation updates

### Tasks

Update docs that mention active APIs:

- `docs/python/waf.md`;
- `docs/python/sync-api.md`;
- `docs/python/async-api.md`;
- `docs/python/scope-and-safety.md`;
- `docs/python/api-reference.md`;
- `crates/eggsec-python/README.md`.

Every active API example must show a scoped client or explicit `scope=` parameter.

Remove or rewrite any language implying load/fuzz/WAF validation can be run as unscoped top-level helpers.

Keep the experimental/not-yet-PyPI status until release gates are actually complete.

### Acceptance criteria

Docs match runtime signatures.

Examples do not scan public third-party targets by default.

Active APIs always show scope.

Known async cancellation limitation remains documented.

## Suggested implementation order

1. Fix active API signatures and client methods.
2. Replace enforcement tests that assert missing scope with actual denied-scope tests.
3. Update type stubs and docs for active API scope.
4. Run default `cargo check` and fix compile errors.
5. Run `maturin develop` and Python tests.
6. Fix Python namespace / `__all__` / stub mismatches.
7. Clean up `python-wheels.yml` publish tail and final publish gating.
8. Run local wheel build/install smoke.
9. Record validation results in release checklist or implementation notes.

## Final acceptance criteria

This pass is complete when:

- active Python APIs cannot execute without scope;
- `load_test_http`, `validate_waf`, and `fuzz_http` have client-level scope enforcement;
- top-level active APIs either require `scope=` or are not documented as preferred;
- denied-scope tests exist for active APIs;
- cap validation tests pass;
- default import and `from eggsec import *` pass;
- type stubs match active API signatures;
- Python wheel workflow YAML is valid and guarded;
- local `cargo check`, `cargo test`, `maturin develop`, pytest, and wheel install smoke are green or failures are documented precisely;
- docs remain honest about experimental status and PyPI unavailability.
