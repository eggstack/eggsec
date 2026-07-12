# Eggsec Python Package: Versioning and Governance

This document defines the versioning policy, schema management, and governance model for the `eggsec` Python package.

## 1. Semantic Versioning Policy

The Python package follows [Semantic Versioning 2.0.0](https://semver.org/) independently from the Rust engine version.

```
MAJOR.MINOR.PATCH
```

- **MAJOR**: Breaking changes to the public API (removed/renamed symbols, changed signatures, changed default behavior).
- **MINOR**: New functionality added in a backwards-compatible manner.
- **PATCH**: Backwards-compatible bug fixes.

### Current version

| Component | Version | Notes |
|-----------|---------|-------|
| Python package (`eggsec`) | 0.1.0 | Experimental/alpha |
| Rust engine (`eggsec`) | workspace version | Internal, not directly exposed |
| Finding schema | 1.0 | See §2 |
| Protocol version | 1.0.0 | Daemon/gRPC wire protocol |
| ABI version | 1 | Native extension interface |

### Stability guarantees

| Stability class | Guarantee |
|-----------------|-----------|
| **stable** | No breaking changes within a major version. Deprecation window before removal. |
| **beta** | May change within a minor version. Deprecation notice provided. |
| **experimental** | May change or be removed at any time. No deprecation window. |

The package is currently at **0.x** (pre-1.0). During 0.x, minor version bumps may include breaking changes with advance notice. After 1.0, the full semver contract applies.

## 2. Schema Versioning

Finding schema version is tracked by the `FINDING_SCHEMA_VERSION` constant (currently `"1.0"`).

- Schema version increments when the `VersionedFinding` structure changes field names, types, or semantics.
- Schema changes are always **backwards-compatible** within a major schema version (new optional fields, new enum variants).
- Breaking schema changes increment the major version and require migration support.
- The `finding_schema` module exposes `schema_version()` on `VersionedFinding` for runtime checks.

### Machine-readable schema metadata

```python
import eggsec
eggsec.build_info()  # includes schema_version field
eggsec.FINDING_SCHEMA_VERSION  # "1.0"
```

## 3. Experimental/Stable Classification

Every public API is classified into one of three stability levels:

| Level | Marker | Example |
|-------|--------|---------|
| **stable** | Default (no marker) | `scan_ports`, `Severity`, `Finding` |
| **beta** | `@beta` decorator (Python) or `_beta` suffix | `hunt_test`, `graphql_test` |
| **experimental** | `eggsec.experimental` namespace | New APIs under active development |

### Rules

1. New APIs enter as **experimental** or **beta**.
2. After at least one minor version as beta, an API can be promoted to **stable**.
3. Stable APIs require a deprecation window before removal (see §4).
4. Experimental APIs may be removed without deprecation in a minor version.

## 4. Deprecation Policy

| Policy | Details |
|--------|---------|
| **Minimum deprecation window** | 2 minor versions (e.g., deprecated in 0.3, removed in 0.5) |
| **Warning type** | `DeprecationWarning` (visible by default in dev, suppressed in production) |
| **Migration guide** | Required for every deprecation; included in release notes |
| **Alternative** | A recommended replacement must be documented |

### Deprecated symbol lifecycle

1. **Deprecated**: `DeprecationWarning` emitted on import and use.
2. **Soft-removed**: Symbol still importable but emits warning; removed from `__all__`.
3. **Hard-removed**: Symbol no longer importable; raises `ImportError` with migration message.

## 5. Supported Python Versions

| Python Version | Support Level |
|----------------|---------------|
| 3.9 | Minimum supported |
| 3.10 | Supported |
| 3.11 | Supported |
| 3.12 | Fully tested (primary CI target) |
| 3.13 | Supported (forward-compatible) |

- Drop support for a Python version only in a minor version bump.
- CI tests against 3.9, 3.10, 3.11, 3.12, 3.13.

## 6. Supported OS/Architectures

| Platform | Architecture | Support Level |
|----------|-------------|---------------|
| Linux | x86_64 | Primary (CI tested) |
| Linux | aarch64 | Supported |
| macOS | arm64 (Apple Silicon) | Primary (CI tested) |
| macOS | x86_64 (Intel) | Supported |
| Windows | x86_64 | Experimental (no CI) |

Wheel availability depends on the target platform. The `build_info()` function returns the target triple used for compilation.

## 7. Wheel Feature Profiles

Wheels are distributed with different feature sets to minimize dependency footprint:

| Profile | Features Included | System Deps |
|---------|-------------------|-------------|
| **default** | core, scanner, async-api, endpoint-discovery, service-fingerprinting, waf-detection, waf-validation, http-fuzzing, load-testing, findings-reporting | None |
| **full-no-system** | default + websocket, git-secrets, sbom, container | None |
| **full** | Everything | Varies (libpcap, libssl, ADB) |

Install examples:

```bash
pip install eggsec                    # default profile
pip install eggsec[full-no-system]    # all non-system-dependent features
pip install eggsec[packet-inspection] # + libpcap
pip install eggsec[nse]               # + libssl (NSE Lua support)
```

## 8. Rust/Python ABI Expectations

- The native extension (`_core`) is a cdylib compiled against the Python stable ABI where possible.
- pyo3 0.22 is used; ABI compatibility follows pyo3's stability guarantees.
- The `ABI_VERSION` constant (`"1"`) is incremented when the native interface changes incompatibly.
- Wheels are built per-platform; cross-platform ABI compatibility is not guaranteed.

### ABI versioning contract

| ABI Version | Meaning |
|-------------|---------|
| 1 | Initial native interface (pyo3 0.22) |
| 2+ | Reserved for pyo3 major upgrades or breaking native API changes |

## 9. Machine-Readable Version Metadata

The `build_info()` function returns a dict with all version metadata:

```python
import eggsec

info = eggsec.build_info()
# {
#     "version": "0.1.0",
#     "rust_crate_version": "0.1.0",
#     "package_name": "eggsec",
#     "target_triple": "x86_64-unknown-linux-gnu",
#     "binding_version": "0.1.0",
# }
```

Additionally, `api_surface_version()` returns a comprehensive version snapshot:

```python
import eggsec

version = eggsec.api_surface_version()
# {
#     "package_version": "0.1.0",
#     "schema_version": "1.0",
#     "protocol_version": "1.0.0",
#     "abi_version": "1",
#     "features_list": ["core", "scanner", ...],
# }
```

Module-level constants are also available:

```python
eggsec.__version__          # "0.1.0"
eggsec.__schema_version__   # "1.0"
eggsec.__protocol_version__ # "1.0.0"
```
