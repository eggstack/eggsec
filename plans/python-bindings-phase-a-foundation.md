# Phase A Plan: Python Binding Foundation

## Objective

Create the foundation for exposing Eggsec as a Python-native library backed by the existing Rust engine. This phase should produce an importable local Python extension module, a binding crate, packaging metadata, a minimal public API, and enough tests/docs to let later phases add real tools without reworking the foundation.

This phase must not expose major scanner behavior yet except possibly a trivial smoke function. The main deliverable is the binding and packaging substrate.

## Architectural constraints

Python is a host-language binding, not a script runtime inside Eggsec.

The new binding crate must not depend on `eggsec-cli` or `eggsec-tui`.

The binding crate should depend only on library/domain crates required for the foundation: likely `eggsec`, `eggsec-core`, `eggsec-tool-core`, `eggsec-runtime`, `tokio`, `serde`, `serde_json`, and PyO3/maturin dependencies.

Do not enable broad Eggsec feature sets by default. Keep the initial crate feature profile minimal.

Use a facade layer. Do not expose arbitrary Rust modules directly to Python.

## Deliverables

Add a new workspace member:

```text
crates/eggsec-python/
```

Add this crate to root `Cargo.toml` workspace members.

Add `crates/eggsec-python/Cargo.toml` with:

```toml
[package]
name = "eggsec-python"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
publish = false

[lib]
name = "eggsec"
crate-type = ["cdylib"]
```

Use PyO3 with `extension-module`. Add `pyo3-async-runtimes` only if needed during this phase; otherwise add it in Phase C.

Add `crates/eggsec-python/pyproject.toml` using maturin as the build backend. Include package metadata, Python version policy, license, classifiers, and readme metadata.

Add a first source layout:

```text
crates/eggsec-python/src/
  lib.rs
  error.rs
  features.rs
  version.rs
```

Add Python package scaffolding if using a mixed Rust/Python package layout:

```text
crates/eggsec-python/python/eggsec/
  __init__.py
  py.typed
```

Expose from Python:

```python
eggsec.__version__
eggsec.features()
eggsec.has_feature(name: str) -> bool
eggsec.build_info() -> dict
```

Define Python exceptions:

```python
EggsecError
ConfigError
ScopeError
EnforcementError
NetworkError
ScanError
TimeoutError
FeatureUnavailableError
SerializationError
InternalError
```

The exception types may be generated in Rust with PyO3. They should be exported from the module and documented.

## Implementation steps

1. Add `crates/eggsec-python` to the workspace.

2. Add PyO3/maturin configuration. Keep the module name `eggsec` unless packaging conflicts force `eggsec_py` temporarily.

3. Implement `#[pymodule] fn eggsec(...)` in `src/lib.rs`.

4. Register exception classes in `error.rs`.

5. Implement `features()` and `has_feature()` using compile-time feature flags. At minimum, report:

```text
core
scanner
async-api
nse
stress-testing
packet-inspection
headless-browser
database
cloud
sbom
websocket
```

For this phase, most optional flags may return false. The function should still exist.

6. Implement `build_info()` returning version, Rust crate version, enabled binding features, target triple if easily available, and package name.

7. Add local tests under:

```text
crates/eggsec-python/tests/
```

or:

```text
crates/eggsec-python/python/tests/
```

Tests should verify import, version, features, `has_feature`, and exception class availability.

8. Add a short local development doc:

```text
docs/python/installation.md
```

Include:

```bash
cd crates/eggsec-python
python -m venv .venv
source .venv/bin/activate
pip install maturin pytest
maturin develop
python -c "import eggsec; print(eggsec.__version__)"
pytest
```

9. Add a root or docs link to indicate Python bindings are planned/experimental.

## Design notes

The binding crate should not become a dumping ground. It should have one module per Python-facing domain. Each module should convert from Rust internal types into stable Python DTOs.

Do not expose `anyhow::Error` directly. Establish the exception conversion mechanism now, even if most mappings initially fall back to `InternalError`.

If PyO3 `abi3` is easy to enable, consider `abi3-py39` or `abi3-py310`. If this creates friction, defer ABI-stable wheels to Phase E.

## Tests and validation

Run:

```bash
cargo check -p eggsec-python
cd crates/eggsec-python && maturin develop
python -c "import eggsec; print(eggsec.__version__)"
python - <<'PY'
import eggsec
assert isinstance(eggsec.features(), dict)
assert eggsec.has_feature('core') is True
assert issubclass(eggsec.EggsecError, Exception)
PY
pytest
```

If local maturin or Python tooling is not available in the environment, document the blocker and ensure `cargo check -p eggsec-python` still passes.

## Acceptance criteria

`crates/eggsec-python` exists and is part of the workspace.

The Python extension module builds locally with maturin.

`import eggsec` works in a clean virtual environment after `maturin develop`.

`eggsec.__version__`, `eggsec.features()`, `eggsec.has_feature()`, and `eggsec.build_info()` work.

The exception hierarchy is importable from Python.

The binding crate does not depend on CLI or TUI crates.

Documentation explains that Python is a host-language binding over Rust, not an internal plugin runtime.

## Out of scope

Port scanning, endpoint scanning, service fingerprinting, WAF detection, reports, NSE, packet tools, stress tools, PyPI publishing, and wheel CI are out of scope for this phase.
