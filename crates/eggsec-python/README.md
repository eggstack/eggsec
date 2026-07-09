# eggsec-python

Python bindings for the [Eggsec](https://github.com/anomalyco/eggsec) security assessment engine.

## Status

**Experimental / Alpha** — This is Phase A (foundation) of the Python bindings.
Only core metadata, feature flags, and build info are exposed.

## Architecture

Eggsec is a Rust-native security assessment engine. These bindings use [PyO3](https://pyo3.rs) and [maturin](https://github.com/PyO3/maturin) to expose the engine as a Python-native library.

This is a **host-language binding**, not an internal plugin runtime. The Rust engine is compiled into a Python extension module.

## Installation (development)

```bash
cd crates/eggsec-python
python -m venv .venv
source .venv/bin/activate
pip install maturin pytest
maturin develop
python -c "import eggsec; print(eggsec.__version__)"
pytest
```

## Usage

```python
import eggsec

print(eggsec.__version__)
print(eggsec.features())
print(eggsec.has_feature("core"))
print(eggsec.build_info())
```

## Available Exceptions

- `EggsecError` — Base exception for all eggsec errors
- `ConfigError` — Configuration errors
- `ScopeError` — Scope enforcement errors
- `EnforcementError` — Policy enforcement errors
- `NetworkError` — Network-related errors
- `ScanError` — Scan execution errors
- `TimeoutError` — Timeout errors
- `FeatureUnavailableError` — Feature not available
- `SerializationError` — Serialization/deserialization errors
- `InternalError` — Internal engine errors

## License

MIT
