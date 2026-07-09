# Python Bindings Corrective Verification Pass

## Objective

Stabilize the Python binding work after the rapid Phase A-F implementation sequence. This pass should not add new user-facing tools. It should verify that the new `eggsec-python` crate builds, imports, packages, enforces scope, gates optional features correctly, and behaves predictably under sync and async use.

The main risk is not architecture direction. The architecture is directionally correct: a separate PyO3/maturin crate exposing Eggsec as a host-language Python library. The risk is that Phase F expanded the API surface very quickly across many feature-gated domains. This pass should reduce that risk by proving the default wheel and every optional feature profile are coherent.

## Scope

In scope:

- Rust compile validation for `eggsec-python` default, selected feature sets, and all features.
- Maturin build/install validation.
- Python import/export validation for default and feature-gated modules.
- Type stub consistency checks.
- CI workflow correction for wheel artifact selection and platform-specific testing.
- Async bridge validation and cleanup if needed.
- Enforcement/scope tests for Phase F's higher-risk modules.
- Feature availability behavior: missing optional features should fail cleanly.
- Documentation/status correction where implementation claims exceed verified support.

Out of scope:

- New bindings for additional Eggsec tools.
- Major redesign of the Python API unless required to fix import/build/runtime breakage.
- Publishing to PyPI before validation is green.
- Adding broad new dependencies to the default wheel.

## Current state summary

Recent commits indicate all planned phases landed:

- Phase A: `eggsec-python` foundation crate, PyO3/maturin scaffolding, exceptions, feature/build metadata.
- Phase B: scoped synchronous port scanning, `Scope`, `Client`, DTOs, JSON/dict serialization.
- Phase C: `AsyncClient`, async bridge, endpoint discovery, service fingerprinting, context managers.
- Phase D: findings/reporting, passive recon, TLS/technology detection, WAF detection.
- Phase E: type stubs, docs, examples, wheel CI, TestPyPI/PyPI workflow.
- Phase F: 13 expansion tracks: WAF validation/fuzzing, load testing, WebSocket, Git secrets, SBOM, database, proxy, mobile, container, packet inspection, stress, NSE, daemon client.

There were no visible GitHub status checks or workflow runs for the latest implementation commit through the connector. Treat the implementation as unverified until this pass completes.

## Workstream 1: Rust compile matrix

### Tasks

Run and fix:

```bash
cargo check -p eggsec-python
cargo test -p eggsec-python
```

Then validate feature combinations:

```bash
cargo check -p eggsec-python --no-default-features
cargo check -p eggsec-python --features websocket
cargo check -p eggsec-python --features git-secrets
cargo check -p eggsec-python --features sbom
cargo check -p eggsec-python --features container
cargo check -p eggsec-python --features full-no-system
cargo check -p eggsec-python --features db-pentest
cargo check -p eggsec-python --features web-proxy
cargo check -p eggsec-python --features mobile
cargo check -p eggsec-python --features packet-inspection
cargo check -p eggsec-python --features stress-testing
cargo check -p eggsec-python --features nse
cargo check -p eggsec-python --features daemon-client
cargo check -p eggsec-python --all-features
```

If some features cannot compile together because of upstream/native/system constraints, document the incompatibility and update feature definitions accordingly. Do not leave silent broken combinations.

### Likely issues to inspect

- Optional dependency feature names that pass through to `eggsec` but do not include the matching optional crate dependency.
- PyO3 registration of types/functions under `#[cfg(feature = ...)]` while Python re-exports assume they exist.
- Type names in `__init__.py` or `.pyi` that do not match Rust `#[pyclass(name = ...)]` names.
- Engine feature names that differ from Python crate feature names.
- `eggsec-mobile-lab` dependency declared but not actually wired into the relevant feature.
- `daemon-client` needing `tokio-util`, transport features, or daemon crate features beyond the optional dependency.

### Acceptance criteria

Default `cargo check -p eggsec-python` passes.

Each documented feature either passes `cargo check` or is explicitly documented as unsupported/pending with the feature removed from public claims.

`--all-features` either passes or has a documented and tested replacement matrix if mutually exclusive native dependencies prevent all-features builds.

No feature-gated Rust module is registered unconditionally.

## Workstream 2: Maturin build and local installation

### Tasks

Run and fix:

```bash
cd crates/eggsec-python
python -m venv .venv
source .venv/bin/activate
pip install -U pip maturin pytest
maturin develop
python -c "import eggsec; print(eggsec.__version__, eggsec.features())"
pytest -q
```

Build a release wheel locally:

```bash
maturin build --release
python -m venv /tmp/eggsec-wheel-test
source /tmp/eggsec-wheel-test/bin/activate
pip install target/wheels/*.whl
python -c "import eggsec; print(eggsec.build_info())"
```

If the wheel path differs, use the actual maturin output path.

### Acceptance criteria

`maturin develop` succeeds for the default feature set.

The default wheel installs into a clean virtual environment without requiring Rust at install time.

`import eggsec` succeeds from an installed wheel.

`eggsec.features()` and `eggsec.has_feature(...)` work after wheel installation.

## Workstream 3: Python import/export correctness

### Tasks

Add or update tests that verify the public Python namespace is coherent.

Required default-build checks:

```python
import eggsec

required = [
    "__version__",
    "features",
    "has_feature",
    "build_info",
    "Scope",
    "Client",
    "AsyncClient",
    "scan_ports",
    "async_scan_ports",
    "scan_endpoints",
    "async_scan_endpoints",
    "fingerprint_services",
    "async_fingerprint_services",
    "Report",
    "Finding",
    "Evidence",
    "Severity",
    "detect_waf",
]

for name in required:
    assert hasattr(eggsec, name), name
```

Optional modules must be tested both ways:

- In a default build, unavailable optional functions/classes should either be absent or raise `FeatureUnavailableError` through a documented shim. They must not break `import eggsec`.
- In a feature-enabled build, the corresponding functions/classes should be present and minimally callable against local fixtures or with validation-only constructors.

Audit `crates/eggsec-python/python/eggsec/__init__.py` for this pattern:

```python
try:
    optional_symbol = _core.optional_symbol
except AttributeError:
    pass
```

This is acceptable for `_core` symbols, but imports such as `from .websocket import ...` must not import non-existent runtime modules or unconditionally reference unavailable `_core` symbols. If per-module `.pyi` files exist without corresponding `.py` modules, ensure the package does not import them at runtime.

### Corrective options

Preferred pattern:

- Keep `__init__.py` runtime exports sourced from `_core` only.
- Keep `.pyi` files for typing, but do not runtime-import `.pyi` modules.
- For optional modules, either expose nothing when unavailable or expose thin Python shims that raise `FeatureUnavailableError` with a clear message.

### Acceptance criteria

`import eggsec` works in default build.

`from eggsec import *` works in default build.

Every name in `eggsec.__all__` exists in default build.

Optional feature names do not appear in `__all__` unless they actually exist or are documented shims.

Feature-enabled builds expose their corresponding symbols.

## Workstream 4: Type stub consistency

### Tasks

Validate that `.pyi` files match runtime exports.

Add a small checker script, for example:

```text
scripts/check_eggsec_python_stubs.py
```

The script should:

1. import `eggsec` from the installed local build;
2. inspect `eggsec.__all__`;
3. parse `__init__.pyi` for top-level public names;
4. report names present in stubs but missing at runtime for the active feature profile;
5. report names in `__all__` missing from stubs.

Optionally run mypy or pyright against examples if adding either dependency is acceptable.

### Acceptance criteria

Default-build stubs align with runtime exports.

Optional-feature stubs are either clearly guarded/documented or split into feature-specific stub files that do not create false expectations in default installs.

Examples type-check or at least import-check under the default wheel.

## Workstream 5: CI wheel workflow correction

### Problem to address

The wheel test workflow appears to download all matrix wheel artifacts and install the first wheel returned by `ls dist/*.whl | head -1`. That can select a wheel for the wrong platform when artifacts are merged.

### Tasks

Update `.github/workflows/python-wheels.yml` so each test job installs the matching wheel for its platform.

Acceptable strategies:

1. Build and test per platform in the same matrix job.
2. Preserve artifact names by target and download only the matching target for each test job.
3. Select wheels by Python tag/platform tag using a small Python script, not `head -1`.

Add explicit jobs for:

- Linux x86_64 default wheel.
- macOS arm64 default wheel.
- Linux source/maturin develop smoke.
- Optional: Linux `full-no-system` feature profile.

Keep PyPI publish manual-only. Do not auto-publish on ordinary pushes.

### Acceptance criteria

CI installs the wheel matching the runner platform.

Default wheel import smoke test runs from the installed wheel.

Localhost scanner smoke test runs from the installed wheel.

Report serialization smoke test runs from the installed wheel.

The workflow fails on pytest failures instead of using `|| true` for the primary test suite. If some tests are known flaky/networked, mark and exclude them rather than ignoring failures globally.

## Workstream 6: Async runtime and cancellation audit

### Tasks

Inspect `runtime_async.rs` and `AsyncClient`.

Verify:

- Returned object is awaitable under normal Python `await` syntax.
- Exceptions raised inside Rust futures propagate as Python exceptions.
- Dropping/cancelling the Python future does not panic.
- Repeated async calls do not leak unbounded threads.
- The implementation does not require users to poll manually unless clearly documented.

Add tests:

```python
@pytest.mark.asyncio
async def test_async_scan_ports_awaitable(): ...

@pytest.mark.asyncio
async def test_async_scope_error_propagates(): ...

@pytest.mark.asyncio
async def test_async_cancellation_does_not_panic(): ...

@pytest.mark.asyncio
async def test_many_async_calls_do_not_hang(): ...
```

If the custom `PyFuture` is not a true asyncio awaitable, either:

- fix it to implement Python await protocol correctly, or
- rename/document it as a pollable future and defer `AsyncClient` public claims until true awaitability is implemented.

Preferred long-term option remains using a mature bridge such as `pyo3-async-runtimes` if it simplifies correctness.

### Acceptance criteria

`await eggsec.async_scan_ports(...)` works in a normal asyncio program.

`async with eggsec.AsyncClient(...)` works if advertised.

Scope errors from async calls raise `eggsec.EnforcementError`.

Cancellation is tested and documented.

No unbounded thread/runtime creation is observed in repeated small async calls.

## Workstream 7: Scope and enforcement audit for Phase F modules

### Tasks

For each Phase F module, add or verify enforcement tests.

Modules:

- WAF validation / HTTP fuzzing.
- Load testing.
- WebSocket testing.
- Database probes.
- Proxy and web proxy APIs.
- Mobile analysis where file paths are involved.
- Container scanning where local paths/images are involved.
- Packet inspection.
- Stress testing.
- NSE execution.
- Daemon client.

Network-targeted APIs must enforce target scope before making network calls.

Path-targeted APIs must validate filesystem paths and avoid path confusion in examples/tests.

Dangerous/active APIs must require explicit caps:

- duration;
- max requests/messages/packets;
- concurrency;
- rate limit;
- explicit target;
- explicit scope.

Automation mode must not honor manual override behavior.

### Required tests

Add one denied-scope test per network module where feasible:

```python
scope = eggsec.Scope.allow_hosts(["allowed.local"])
client = eggsec.Client(scope, mode="automation")
with pytest.raises(eggsec.EnforcementError):
    client.<module_action>("not-allowed.local", ...)
```

Add cap validation tests for active modules:

```python
with pytest.raises(ValueError):
    client.load_test_http(url, duration_s=0, ...)

with pytest.raises(ValueError):
    client.fuzz_http(url, max_requests=None, ...)
```

Add default-feature unavailable tests:

```python
if not eggsec.has_feature("stress-testing"):
    assert not hasattr(eggsec, "stress_test") or raises FeatureUnavailableError
```

### Acceptance criteria

Every Phase F module has at least one import/availability test.

Every network-active Phase F module has at least one denied-scope test.

Every active/stress/fuzz/load module has cap validation tests.

Stress, packet, NSE, daemon, DB, proxy, and other optional modules are unavailable by default unless intentionally compiled in.

## Workstream 8: Documentation/status cleanup

### Tasks

Audit docs and README for claims that imply production readiness or full PyPI availability before CI proves it.

Update language to:

- `experimental` or `alpha` for Python bindings;
- `default wheel candidate` rather than published wheel unless actually published;
- `feature-gated experimental modules` for Phase F tools;
- explicit unsupported/untested platform notes.

Ensure docs distinguish:

- Python library binding;
- CLI/TUI manual operation;
- daemon/API/MCP/agent operation;
- Lua/NSE compatibility;
- optional dangerous/active testing features.

### Acceptance criteria

Docs do not claim PyPI publication unless it has occurred.

Docs do not claim Phase F modules are stable unless validated.

Feature-gated modules list required Cargo/Python build features.

Examples do not scan public third-party targets by default.

## Workstream 9: Release gating checklist

Add a checked-in release checklist:

```text
crates/eggsec-python/RELEASE_CHECKLIST.md
```

Include:

```text
[ ] cargo check -p eggsec-python
[ ] cargo test -p eggsec-python
[ ] cargo check -p eggsec-python --features full-no-system
[ ] selected optional feature checks
[ ] maturin develop
[ ] pytest default suite
[ ] maturin build --release
[ ] clean venv wheel install
[ ] import smoke
[ ] scanner smoke
[ ] report smoke
[ ] stub consistency check
[ ] docs reviewed
[ ] TestPyPI dry run
[ ] install from TestPyPI
[ ] PyPI publish only after all above pass
```

### Acceptance criteria

Release checklist exists and matches the actual commands used by CI.

PyPI publish workflow points to the checklist or references the same gates.

## Suggested implementation order

1. Run default `cargo check -p eggsec-python` and fix immediate compile errors.
2. Run `maturin develop` and fix import/export breakage.
3. Fix `__init__.py` / optional module runtime import behavior.
4. Add default import and `__all__` tests.
5. Fix wheel CI artifact selection and remove `|| true` from primary pytest path.
6. Validate feature-gated compile matrix.
7. Add enforcement/cap tests for Phase F modules.
8. Audit async awaitability/cancellation.
9. Clean up docs/status and add release checklist.

This order catches highest-probability breakage first: compile failures, import failures, and broken CI packaging.

## Final acceptance criteria

The corrective pass is complete when:

- `cargo check -p eggsec-python` passes by default.
- `cargo test -p eggsec-python` passes by default.
- `maturin develop` succeeds.
- `import eggsec` succeeds in a clean environment.
- `from eggsec import *` succeeds in a clean environment.
- Default pytest suite passes without ignored failures.
- Wheel CI installs the correct platform wheel.
- Optional features are either tested or explicitly marked pending.
- Missing optional features fail cleanly.
- Async APIs are truly awaitable or their public claims are corrected.
- Phase F active/dangerous modules have enforcement and cap validation tests.
- Docs accurately reflect experimental status and feature availability.

## Notes for implementer

Prefer deleting or deferring unstable Phase F exports over leaving broken public API names. It is better for default `import eggsec` to be reliable with fewer symbols than to expose an expansive namespace that fails under common build profiles.

Do not publish to PyPI from this pass unless all release gates pass. TestPyPI is acceptable only after local wheel install and CI smoke tests are green.
