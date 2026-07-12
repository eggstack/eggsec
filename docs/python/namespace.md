# Eggsec Python Package: Namespace and Import Stability

This document defines the namespace architecture, import stability guarantees, and deprecation policy for the `eggsec` Python package.

## 1. Namespace Architecture

The `eggsec` package uses a **flat re-export** pattern:

- All public symbols are re-exported from `eggsec/__init__.py`.
- Users import directly from the top-level namespace: `from eggsec import scan_ports`.
- Internal module structure (`eggsec.scanner`, `eggsec.recon`, etc.) is an implementation detail.
- The `eggsec._core` native module is a private implementation detail and must not be imported directly.

### Why flat namespace?

1. **Discoverability**: All APIs are visible via `dir(eggsec)` and `help(eggsec)`.
2. **Simplicity**: Single import location reduces cognitive load.
3. **Stability**: Internal module reorganization does not affect user code.

## 2. Feature-Gated Import Behavior

Feature-gated symbols use a `try/except AttributeError` pattern to handle unavailable features gracefully:

```python
# Always available
scan_ports = _core.scan_ports

# Feature-gated (only available when compiled with --features mobile)
try:
    analyze_apk = _core.analyze_apk
    async_analyze_apk = _core.async_analyze_apk
except AttributeError:
    pass
```

### Behavior for users

- **Available feature**: Symbol is importable and functional.
- **Unavailable feature**: Symbol is not present in the namespace. Accessing it raises `AttributeError`.
- **Check availability**: Use `eggsec.has_feature("mobile")` or `eggsec.features()` before importing.

```python
import eggsec

if eggsec.has_feature("mobile"):
    eggsec.analyze_apk(...)
else:
    raise RuntimeError("mobile feature not available in this build")
```

## 3. Experimental Namespace Policy

New APIs under active development are placed in the `eggsec.experimental` namespace:

```python
import eggsec
from eggsec import experimental

# Experimental APIs may change or be removed without notice
experimental.new_scanner(...)
```

### Rules for experimental APIs

1. **No stability guarantee**: APIs may change signature or be removed in any release.
2. **No deprecation window**: Removal requires only a changelog entry.
3. **Promotion path**: After stability validation, promote to `eggsec` top-level with beta/stable classification.
4. **Documentation**: Experimental APIs must be documented with `.. warning::` directives.

### Implementation

The `experimental` namespace is a Python subpackage (`eggsec/experimental/`) that re-exports from `_core` with unstable markers:

```python
# eggsec/experimental/__init__.py
"""
Experimental APIs — may change or be removed without notice.
"""
from eggsec._core import experimental_feature  # example
```

## 4. Deprecation Policy and Warnings

When an API is deprecated:

1. **Import-time warning**: Importing the deprecated name emits `DeprecationWarning`.
2. **Runtime warning**: Calling the deprecated function emits `DeprecationWarning` with migration guidance.
3. **`__all__` removal**: The deprecated name is removed from `__all__` (no longer part of public API).
4. **Documentation**: Deprecation is noted in the API reference and release notes.

### Deprecation warning helper

The package provides a helper for emitting deprecation warnings:

```python
import warnings

def _deprecated(name: str, replacement: str | None = None) -> None:
    msg = f"{name} is deprecated"
    if replacement:
        msg += f"; use {replacement} instead"
    warnings.warn(msg, DeprecationWarning, stacklevel=3)
```

### Migration timeline

| Phase | Version | Effect |
|-------|---------|--------|
| Deprecated | N.0 | Warning emitted; symbol still functional |
| Soft-removed | N.1 | Symbol removed from `__all__`; warning on import |
| Hard-removed | N.2 | Symbol no longer importable; `ImportError` raised |

Minimum window: 2 minor versions (deprecated in 0.3 → removed in 0.5).

## 5. Import Stability Guarantees

| Guarantee | Scope |
|-----------|-------|
| **Stable symbols** | No breaking changes within a major version |
| **Feature-gated symbols** | Always importable when feature is compiled; never present otherwise |
| **Internal modules** | `eggsec._core` and submodules are private; may change without notice |
| **`__all__` list** | Authoritative list of public exports; checked in CI |
| **Type stubs** | `__init__.pyi` maintained in sync with `__init__.py` |

### What is NOT guaranteed

- Internal module paths (`eggsec.scanner` vs `eggsec._core.scanner`).
- Order of imports in `__init__.py`.
- Exact error messages for missing features.
- Behavior of feature-gated APIs when the feature is compiled out.

## 6. Recommended Import Patterns

### Pattern 1: Direct import (preferred)

```python
import eggsec

result = eggsec.scan_ports("example.com", [80, 443])
```

### Pattern 2: Selective import

```python
from eggsec import scan_ports, Severity, Finding

result = scan_ports("example.com", [80, 443])
```

### Pattern 3: Feature check before use

```python
import eggsec

if eggsec.has_feature("db-pentest"):
    result = eggsec.db_probe("localhost", 5432)
else:
    print("Database pentest not available in this build")
```

### Pattern 4: Async usage

```python
import asyncio
import eggsec

async def main():
    result = await eggsec.async_scan_ports("example.com", [80, 443])

asyncio.run(main())
```

### Pattern 5: Configuration

```python
from eggsec import EggsecConfig, ScanConfig, Scope

config = EggsecConfig(scan=ScanConfig(timeout=30))
```

### Anti-patterns to avoid

```python
# DON'T: Import from internal modules
from eggsec._core import scan_ports  # private, may break

# DON'T: Import feature-gated symbols without checking
from eggsec import analyze_apk  # may raise ImportError

# DON'T: Use wildcard imports
from eggsec import *  # pollutes namespace, poor IDE support
```

## 7. API Surface Introspection

The package provides machine-readable API surface information:

```python
import eggsec

# Get full API surface with stability levels
surface = eggsec.api_surface()
# {
#     "scan_ports": {"stability": "stable", "deprecated": None},
#     "analyze_apk": {"stability": "stable", "deprecated": None, "requires_feature": "mobile"},
#     ...
# }

# Check if a specific name is available and stable
info = eggsec.api_surface().get("scan_ports")
# {"stability": "stable", "deprecated": None}
```

## 8. Version Constants

Module-level constants provide version information:

```python
eggsec.__version__            # "0.1.0" — Python package version
eggsec.__schema_version__     # "1.0"   — Finding schema version
eggsec.__protocol_version__   # "1.0.0" — Daemon/gRPC protocol version
eggsec.FINDING_SCHEMA_VERSION # "1.0"   — Same as __schema_version__
```

## 9. Feature Matrix

The `features()` function returns a dict of all known features and their availability:

```python
import eggsec

features = eggsec.features()
# {
#     "core": True,
#     "scanner": True,
#     "mobile": False,  # not compiled in
#     "nse": False,
#     ...
# }
```

The `feature_matrix()` function returns richer metadata per feature:

```python
matrix = eggsec.feature_matrix()
# {
#     "mobile": {
#         "available": False,
#         "description": "APK/IPA static analysis",
#         "requires_system_deps": False,
#     },
#     ...
# }
```
