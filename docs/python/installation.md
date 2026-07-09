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

**Experimental / Alpha** — Phase F complete. Default wheel includes: port scanning, endpoint discovery, service fingerprinting, DNS/TLS/tech recon, WAF detection, WAF validation/bypass, HTTP fuzzing, load testing, findings/reporting, and scope enforcement. Optional feature-gated modules: WebSocket testing, git secrets, SBOM generation, database pentesting, proxy management, mobile app analysis, container scanning, packet inspection, stress testing, NSE scripts, and daemon client.

## Notes

- Requires Python >= 3.9
- The binding crate does not depend on `eggsec-cli` or `eggsec-tui`
- Feature flags are reported at runtime via `eggsec.features()` and `eggsec.has_feature(name)`
