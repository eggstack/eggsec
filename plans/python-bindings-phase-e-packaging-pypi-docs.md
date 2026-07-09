# Phase E Plan: Packaging, PyPI, Type Stubs, and Documentation Hardening

## Objective

Prepare the Eggsec Python library for public or semi-public distribution as prebuilt wheels. This phase should harden packaging, CI, documentation, examples, smoke tests, type hints, and release process so `pip install eggsec` works on supported platforms without requiring users to compile Rust locally.

This phase is not primarily about adding new Eggsec capabilities. It is about making the existing Python API usable, documented, typed, and releasable.

## Dependencies

This phase assumes Phase D is complete:

- Core binding crate exists.
- Sync and async scanner APIs work.
- Endpoint discovery and service fingerprinting work.
- Reporting, passive recon, and WAF detection work.
- Scope/enforcement semantics are documented.
- Python examples exist.

## Packaging goals

Use maturin for wheel builds.

Publish a default wheel with a conservative feature set:

- core binding
- scope/client model
- scanner
- endpoint discovery
- service fingerprinting
- reporting
- passive recon
- WAF detection

Do not include by default:

- NSE
- raw packet inspection
- stress testing
- headless browser automation
- database pentest native drivers
- wireless tooling
- cloud SDK-heavy features
- SSH2/OpenSSL-heavy optional paths unless already required by default

The default wheel should prioritize reliable installability.

## Supported platforms

Initial wheel targets:

- macOS arm64
- macOS x86_64
- Linux x86_64
- Linux aarch64

Windows x86_64 should be evaluated, but it can be deferred if network behavior, socket permissions, dependency issues, or CI complexity make it risky.

Use manylinux-compatible builds for Linux where possible.

## PyPI naming decision

Confirm the distribution name before publishing.

Preferred:

```text
eggsec
```

Fallbacks if unavailable or if early stabilization prefers a less final name:

```text
eggsec-rs
eggsec-py
```

The Python import name should remain:

```python
import eggsec
```

Avoid publishing under a temporary import name unless unavoidable.

## Versioning policy

Use the workspace version initially, but document whether Python API versioning is tied to the Rust crate version.

Before Python API `1.0`, minor releases may adjust APIs but must include migration notes.

After Python API `1.0`, breaking Python API changes require a major version bump.

Add `DeprecationWarning` paths for renamed public Python APIs once the API starts stabilizing.

## Type stubs and typing

Ship `py.typed`.

Add `.pyi` stubs for public classes and functions if PyO3-generated signatures are not enough:

```text
crates/eggsec-python/python/eggsec/
  __init__.pyi
  client.pyi
  scanner.pyi
  reports.pyi
  recon.pyi
  waf.pyi
  errors.pyi
  py.typed
```

The public type surface should cover:

- `Client`
- `AsyncClient`
- `Scope`
- scanner functions
- endpoint functions
- fingerprinting functions
- report/finding types
- recon/WAF types
- exception hierarchy

Run a lightweight type-checking example with pyright or mypy if adding either tool is acceptable. If not, include a stub syntax check and basic import tests.

## Documentation hardening

Complete the Python docs tree:

```text
docs/python/index.md
docs/python/installation.md
docs/python/quickstart.md
docs/python/sync-api.md
docs/python/async-api.md
docs/python/scope-and-safety.md
docs/python/scanner.md
docs/python/endpoint-discovery.md
docs/python/fingerprinting.md
docs/python/reports.md
docs/python/recon.md
docs/python/waf.md
docs/python/packaging.md
docs/python/api-reference.md
```

Every documented function should have a tested example or at least a smoke-tested snippet.

Add a clear comparison section:

- Python library: compose Eggsec from Python scripts.
- CLI/TUI: direct human operation.
- Daemon/API/MCP/agent surfaces: automated integration with stricter enforcement.
- NSE/Lua: Nmap script compatibility.

Document feature availability and optional modules:

```python
eggsec.features()
eggsec.has_feature("nse")
```

Document the default wheel feature set and what is not included.

## Examples hardening

Ensure these examples are present and runnable:

```text
examples/python/basic_port_scan.py
examples/python/async_multi_target_scan.py
examples/python/endpoint_discovery.py
examples/python/service_fingerprint.py
examples/python/recon_report.py
examples/python/waf_detection.py
examples/python/scan_to_json.py
examples/python/scan_to_pandas.py
```

Examples should default to localhost fixtures or require users to pass a target explicitly. Avoid examples that scan public targets by default.

Each example should include a short scope/authorization comment.

## CI and release workflow

Add a GitHub Actions workflow for Python wheels, for example:

```text
.github/workflows/python-wheels.yml
```

Workflow stages:

1. Build wheels with maturin.
2. Install each wheel in a clean virtual environment.
3. Run import smoke test.
4. Run localhost scanner smoke test.
5. Run serialization/report smoke test.
6. Upload wheels as artifacts.

Add a separate publish workflow or manual release job:

1. Build wheels.
2. Publish to TestPyPI.
3. Install from TestPyPI in a clean environment.
4. Run smoke tests.
5. Publish to PyPI.

Use trusted publishing if feasible.

## Smoke tests

Add a minimal smoke suite that can run after installing a wheel:

```bash
python - <<'PY'
import eggsec
print(eggsec.__version__)
print(eggsec.features())
assert eggsec.has_feature("core")
PY
```

Add a local TCP fixture scan:

```bash
python examples/python/basic_port_scan.py --target 127.0.0.1 --local-fixture
```

Add a serialization smoke test:

```bash
python examples/python/scan_to_json.py --target 127.0.0.1 --local-fixture
```

## README and package metadata

The PyPI README should include:

- short description
- install command
- quickstart
- supported platforms
- default included features
- excluded/planned feature modules
- safety/scope note
- link to docs
- license

Package classifiers should reflect security tooling carefully and avoid implying offensive-only usage.

## Validation commands

Local validation:

```bash
cd crates/eggsec-python
maturin build --release
python -m venv /tmp/eggsec-wheel-test
source /tmp/eggsec-wheel-test/bin/activate
pip install target/wheels/*.whl
python -c "import eggsec; print(eggsec.__version__)"
pytest python/tests
```

CI validation:

```bash
gh workflow run python-wheels.yml
```

Release validation:

```bash
maturin publish --repository testpypi
pip install --index-url https://test.pypi.org/simple/ eggsec
python -c "import eggsec; print(eggsec.features())"
```

Adjust exact commands to the repository's release policy.

## Acceptance criteria

Prebuilt wheels build for the selected initial platforms.

A wheel installs into a clean virtual environment without requiring Rust.

Import, feature introspection, scanner smoke, and serialization smoke tests pass from installed wheels.

Type stubs and `py.typed` are included.

Python docs cover installation, quickstart, sync/async APIs, scope/safety, scanner, endpoints, fingerprinting, reports, recon, and WAF detection.

Examples are runnable and do not scan public third-party targets by default.

TestPyPI publication is documented and validated before PyPI publication.

The default wheel feature set is clearly documented.

## Out of scope

Adding new major Eggsec tools is out of scope for this phase except where needed to stabilize docs or examples. NSE, stress testing, raw packet features, database, cloud, mobile, proxy, and daemon client expansion are handled in Phase F or later detailed plans.
