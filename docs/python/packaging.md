# Packaging & Release

Build, test, and publish the `eggsec` Python package.

## Building wheels with maturin

The Python bindings use [maturin](https://github.com/PyO3/maturin) as the
build backend. The extension module compiles the Rust engine into a
`cdylib` that Python loads via the standard extension mechanism.

### Development build (in-tree)

```bash
cd crates/eggsec-python
python -m venv .venv
source .venv/bin/activate
pip install maturin pytest
maturin develop
python -c "import eggsec; print(eggsec.__version__)"
```

### Release wheel (portable)

```bash
cd crates/eggsec-python
maturin build --release
```

Wheels are written to `target/wheels/`. Each wheel is a platform-specific
`.whl` file tagged with the target OS, architecture, and Python version.

### Universal (Python-only) wheel

Not applicable -- the package contains a compiled Rust extension. There is
no pure-Python fallback.

## Supported platforms

| OS | Architecture | Triple | Status |
|---|---|---|---|
| macOS | ARM64 (Apple Silicon) | `aarch64-apple-darwin` | Supported |
| macOS | x86_64 (Intel) | `x86_64-apple-darwin` | Supported |
| Linux | x86_64 | `x86_64-unknown-linux-gnu` | Supported |
| Linux | aarch64 | `aarch64-unknown-linux-gnu` | Supported |
| Windows | x86_64 | `x86_64-pc-windows-msvc` | Not currently built |

Linux builds require `manylinux`-compatible hosts (or CI containers) for
PyPI compatibility. Use `maturin build --manylinux auto` or specify the
target explicitly:

```bash
maturin build --release --manylinux 2_28 --target x86_64-unknown-linux-gnu
```

## Default wheel feature set

The default wheel compiles the engine with **no optional features
enabled**. This keeps the binary small and avoids pulling in system
dependencies (`libssl-dev`, `libpcap-dev`, etc.) that users may not have.

**Note:** PyPI publication has not yet occurred. The wheel is a default
wheel candidate — not a published artifact.

The default wheel includes:

- Core engine (`eggsec-core`, `eggsec-tool-core`)
- Scanner (TCP port scan, endpoint discovery)
- Service fingerprinting
- Reconnaissance (DNS, TLS inspection, technology detection)
- WAF detection
- Findings & reporting (`Finding`, `FindingSet`, `Report`)
- Scope enforcement

## What is NOT included by default

The following features require custom builds or future optional extras:

| Feature | Build flag | System dependency |
|---|---|---|
| Nmap NSE scripts | `--features nse` | `libssl-dev` |
| Stress testing / raw sockets | `--features stress-testing` | Root / `CAP_NET_RAW` |
| Packet inspection | `--features packet-inspection` | `libpcap-dev` |
| Headless browser | `--features headless-browser` | Chromium / Playwright |
| Database persistence | `--features database` | SQLx drivers |
| Cloud integration | `--features cloud` | AWS/GCP/Azure SDKs |
| SBOM generation | `--features sbom` | -- |
| WebSocket testing | `--features websocket` | -- |

To build with specific features:

```bash
maturin build --release --features nse,packet-inspection
```

## PyPI naming

The primary package name is **`eggsec`**. If that name is taken or
conflicts, the following fallback names are reserved:

| Name | Use case |
|---|---|
| `eggsec` | Primary (preferred) |
| `eggsec-rs` | Fallback if `eggsec` is taken |
| `eggsec-py` | Fallback if both above are taken |

The import name is always `import eggsec` regardless of the published
package name (controlled by `module-name = "eggsec._core"` in
`pyproject.toml`).

## Versioning policy

- The Python package version is **tied to the workspace version** in
  `Cargo.toml`. Both are bumped together.
- The package is currently pre-1.0 (`0.1.0`). Pre-1.0 releases may
  contain breaking API changes between minor versions without notice.
- The `__version__` attribute and `__version_info__` tuple are compiled
  into the extension module from `CARGO_PKG_VERSION`.
- Binding-layer version (`binding_version` in `build_info()`) tracks the
  Python-specific API surface independently of the engine version.

### Version bump workflow

1. Update `version` in the workspace `Cargo.toml` (or use `cargo-edit`).
2. Update `version` in `crates/eggsec-python/pyproject.toml` to match.
3. Tag the release: `git tag python-v<version>`.
4. CI builds wheels and publishes to PyPI (see below).

## TestPyPI workflow

TestPyPI is used for validation before a production release.

### 1. Build

```bash
cd crates/eggsec-python
maturin build --release --manylinux auto
```

### 2. Upload to TestPyPI

```bash
pip install twine
twine upload --repository testpypi target/wheels/*.whl
```

### 3. Install from TestPyPI

```bash
pip install --index-url https://test.pypi.org/simple/ --extra-index-url https://pypi.org/simple/ eggsec
```

The `--extra-index-url` fallback pulls transitive dependencies from
production PyPI since TestPyPI may not have them.

### 4. Smoke test

```bash
python -c "
import eggsec
print('version:', eggsec.__version__)
print('features:', eggsec.features())
print('has scanner:', eggsec.has_feature('scanner'))
info = eggsec.build_info()
print('build:', info)
"
```

## Publishing to PyPI

### Prerequisites

- PyPI account with API token configured (`~/.pypirc` or environment
  variable `PYPI_TOKEN`).
- `twine` installed (`pip install twine`).
- All wheels built for target platforms.

### Steps

```bash
# Clean previous builds
rm -rf target/wheels/

# Build for current platform
cd crates/eggsec-python
maturin build --release --manylinux auto

# Upload
twine upload target/wheels/*.whl
```

### CI publishing (GitHub Actions)

Add a workflow triggered on tags:

```yaml
name: Publish Python package
on:
  push:
    tags: ["python-v*"]

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with:
          python-version: "3.12"
      - name: Install maturin
        run: pip install maturin twine
      - name: Build wheel
        run: |
          cd crates/eggsec-python
          maturin build --release --manylinux auto
      - name: Publish to PyPI
        env:
          TWINE_USERNAME: __token__
          TWINE_PASSWORD: ${{ secrets.PYPI_TOKEN }}
        run: twine upload crates/eggsec-python/target/wheels/*.whl
```

## Smoke test commands

After installation, verify the package works:

```bash
# Basic import and version check
python -c "import eggsec; print(eggsec.__version__)"

# Feature check
python -c "import eggsec; print(eggsec.features())"

# Build info
python -c "import eggsec; print(eggsec.build_info())"

# Scope creation
python -c "
from eggsec import Scope
s = Scope.allow_hosts(['example.com'])
print(s)
print('target allowed:', s.is_target_allowed('example.com'))
print('port allowed:', s.is_port_allowed(80))
"

# Port scan (requires network access)
python -c "
import eggsec
scope = eggsec.Scope.allow_hosts(['127.0.0.1'])
result = eggsec.scan_ports('127.0.0.1', [22, 80, 443], scope)
print(result)
print('open ports:', result.open_ports)
"

# Run pytest suite (if tests are present)
cd crates/eggsec-python && pytest -v
```
