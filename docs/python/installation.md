# Python Bindings Installation

Eggsec provides Python bindings via [PyO3](https://pyo3.rs) and [maturin](https://github.com/PyO3/maturin).

## Architecture

Python is a **host-language binding** over the Rust engine, not an internal plugin runtime. The core engine is compiled into a Python extension module. See `architecture/overview.md` for workspace layout.

## Development Setup

```bash
cd crates/eggsec-python
python -m venv .venv
source .venv/bin/activate
pip install maturin pytest
maturin develop
python -c "import eggsec; print(eggsec.__version__)"
```

## Running Tests

```bash
cd crates/eggsec-python
pip install maturin pytest
maturin develop
pytest
```

## Building Wheels

```bash
cd crates/eggsec-python
pip install maturin
maturin build --release
```

## Status

**Experimental / Alpha** — Phase A foundation only. Core metadata, feature flags, and build info are exposed. Scanner, proxy, and other tool APIs are not yet available from Python.

## Notes

- Requires Python >= 3.9
- The binding crate does not depend on `eggsec-cli` or `eggsec-tui`
- Feature flags are reported at runtime via `eggsec.features()` and `eggsec.has_feature(name)`
