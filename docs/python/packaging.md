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

## Source distribution (sdist)

The project publishes an sdist alongside wheels. The sdist includes all
workspace crates required to build the Python extension from source.

**Build**: `maturin sdist --out dist --manifest-path crates/eggsec-python/Cargo.toml`

**Requirements to build from sdist**: Rust toolchain (>= 1.80), Cargo, and a
C compiler. The sdist build will fail with actionable diagnostics if these
are missing.

**What's included**: All `crates/` directories in the workspace,
`pyproject.toml`, `Cargo.toml` (workspace root), and generated files
(`wheel-profiles.json`, type stubs). The build system (maturin) resolves
the workspace dependency graph automatically.

**When to use**: The sdist is useful for package managers that build from
source (e.g., `pip install --no-binary :all:`) or for auditing the full
source. Most users should prefer pre-built wheels for faster installation.

CI builds and validates the sdist in the `build-sdist` job of
`python-wheels.yml`. If the sdist build fails due to missing Rust tooling,
the CI documents the failure without blocking the wheel-based release.

## Platform support

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

## ABI compatibility

The native ABI version is tracked by the `ABI_VERSION` constant (currently
`"1"`). ABI-breaking changes include:

- Removing or renaming a `#[pyclass]` or `#[pyfunction]`.
- Changing the Python signature of an existing function.
- Modifying the memory layout of a `#[pyclass]`.

Non-breaking additions (new classes, new methods, new optional parameters)
do not bump the ABI version. Consumers should check `ABI_VERSION` before
loading a wheel compiled against a different version.

The `api_surface_version()` function returns all version metadata in one
call:

```python
>>> eggsec.api_surface_version()
{'package_version': '0.1.0', 'schema_version': '1.0', 'protocol_version': '1.0.0', 'abi_version': '1', ...}
```

## Wheel profiles

Three canonical wheel profiles are defined. The machine-readable manifest is
at `crates/eggsec-python/wheel-profiles.json`. Use
`eggsec.wheel_profile()` to detect the installed profile at runtime.

### Core/default wheel

Compiled with no optional features. Suitable for most users who need
scanning, fingerprinting, recon, WAF detection, and reporting.

```bash
maturin build --release
```

### Full-no-system wheel

Compiled with optional features that do not require external system
libraries (`websocket`, `git-secrets`, `sbom`, and `container`). This is the
portable second profile used by the release checks.

```bash
maturin build --release --features full-no-system
```

The repository-level helpers build both profiles and validate each in a clean
virtual environment:

```bash
bash scripts/build_wheel_profiles.sh
for wheel in target/python-wheels/*.whl; do
  bash scripts/validate_wheel.sh "$wheel"
done
```

Not all features can be combined in a single wheel. Features such as
`packet-inspection`, `nse`, and `wireless` require system libraries or tools
that may not be available on all platforms.

### Runtime profile detection

```python
import eggsec

# Detect the installed wheel profile
profile = eggsec.wheel_profile()
print(profile)  # "core", "full-no-system", or "custom"

# Enhanced build info with diagnostics
info = eggsec.build_info()
print(info["wheel_profile"])      # compiled profile
print(info["compiled_features"])  # list of enabled features
print(info["python_version"])     # Python interpreter version
print(info["schema_version"])     # schema version
print(info["abi_version"])        # native ABI version
```

## Feature matrix

| Feature | System Dep | Default Wheel | full-no-system Wheel |
|---|---|---|---|
| `core` | -- | Yes | Yes |
| `scanner` | -- | Yes | Yes |
| `async-api` | -- | Yes | Yes |
| `endpoint-discovery` | -- | Yes | Yes |
| `service-fingerprinting` | -- | Yes | Yes |
| `waf-detection` | -- | Yes | Yes |
| `waf-validation` | -- | Yes | Yes |
| `http-fuzzing` | -- | Yes | Yes |
| `load-testing` | -- | Yes | Yes |
| `findings-reporting` | -- | Yes | Yes |
| `websocket` | -- | No | Yes |
| `git-secrets` | -- | No | Yes |
| `sbom` | -- | No | Yes |
| `container` | -- | No | Yes |
| `db-pentest` | -- | No | Yes |
| `mobile` | -- | No | Yes |
| `stress-testing` | -- | No | Yes |
| `evasion` | -- | No | Yes |
| `postex` | -- | No | Yes |
| `c2` | -- | No | Yes |
| `headless-browser` | Chromium | No | Yes |
| `packet-inspection` | `libpcap-dev` | No | Yes |
| `nse` | `libssl-dev` | No | Yes |
| `wireless` | wireless-tools | No | Yes |
| `web-proxy` | -- | No | Yes |

## Python version support

The package requires **Python >= 3.9**. The minimum is enforced in
`pyproject.toml` and tested in CI.

| Python | Status |
|---|---|
| 3.9 | Minimum supported |
| 3.10 | Supported |
| 3.11 | Supported |
| 3.12 | Supported |
| 3.13 | Supported |

Python 3.8 and earlier are not supported due to reliance on `|` union
type syntax in type annotations and `importlib.metadata` features.

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

The first-release stable guarantee covers local `Engine` and `AsyncEngine`
execution against the twenty-two stable-core operations. Daemon-client execution is
provisional until transport parity, checkpoint portability, and reconnect
semantics have their own release gate.

### 1. Build and validate locally

```bash
bash scripts/validate_python_release_candidate.sh
```

The required fixture suite starts only managed loopback services and does not
use public DNS, HTTP, or TLS. Its fixture manager sets
`EGGSEC_ALLOW_LOOPBACK_FIXTURE=1` only while those explicit release fixtures
are active; ordinary package resolution continues to enforce the normal
policy-approved path.

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

The repository workflow builds and tests platform wheels on pushes and pull
requests. Manual dispatch first publishes to TestPyPI and verifies a clean
installation. Production publication is a separate `publish_pypi` input and
protected environment, so a TestPyPI dry run cannot publish to PyPI
implicitly.

For a separate tag-driven setup:

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
        uses: pypa/gh-action-pypi-publish@release/v1
        with:
          packages-dir: crates/eggsec-python/target/wheels/
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
