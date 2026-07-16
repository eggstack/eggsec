# Python Namespace Governance

Phase C establishes intentional package structure that separates stable
application APIs, reusable primitives, provisional managed subsystems,
and hazardous experimental capabilities.

## Package Structure

```text
eggsec/
  __init__.py          stable core: engine, operations, scope, config, events
  _feature_guard.py    feature availability detection and structured errors
  net/                 targets, transports, probes, HTTP, WebSocket (provisional)
  sessions/            browser, mobile, database, proxy, capture (provisional)
  storage/             finding/assessment repositories, artifacts (provisional)
  reporting/           reporters, streaming, baselines, formats (provisional)
  daemon/              daemon client and parity contracts (provisional)
  experimental/        wireless, evasion, postex, C2, hunt, AI, stress, etc.
```

## Maturity Levels

| Level | Description | Import Impact |
|-------|-------------|---------------|
| **stable** | 22-operation engine registry, core DTOs, config, events | Always available in default wheel |
| **provisional** | Session contracts, network types, storage, reporting | Available in default wheel; may require features for full functionality |
| **experimental** | Wireless, evasion, postex, C2, hunt, AI, stress | Feature-gated; isolated under `eggsec.experimental` |
| **deprecated** | Legacy names retained for backward compatibility | Emit `DeprecationWarning` on access |

## Import Rules

1. `import eggsec` must not initialize experimental, browser, database, container, packet, mobile, or AI dependencies
2. `from eggsec import X` where X is experimental should work (lazy) or raise `FeatureUnavailableError`
3. `from eggsec.experimental import X` should work if the feature is compiled
4. Feature-gated modules fail with structured `FeatureUnavailableError`, not `AttributeError`

## Canonical Naming

| Old Name (Py-suffixed) | Canonical Name | Location |
|------------------------|----------------|----------|
| `TargetPy` | `Target` | `eggsec.net.Target` |
| `TcpSessionPy` | `TcpSession` | `eggsec.net.TcpSession` |
| `WebSocketSessionPy` | `WebSocketSession` | `eggsec.net.WebSocketSession` |
| `HttpClientPy` | `HttpClient` | `eggsec.net.HttpClient` |
| `DatabaseSessionStatePy` | `DatabaseSessionState` | `eggsec.sessions.DatabaseSessionState` |
| `BrowserSessionPy` | `BrowserSession` | `eggsec.sessions.BrowserSession` |
| `DaemonClientPy` | `DaemonClient` | `eggsec.daemon.DaemonClient` |
| `WirelessNetworkPy` | `WirelessNetwork` | `eggsec.experimental.WirelessNetwork` |

Py-suffixed names are retained at the top level for backward compatibility
but emit `DeprecationWarning` when accessed. New code should use canonical
names from the appropriate submodule.

## Deprecation Policy (Pre-1.0)

- Stable names are not removed without a documented migration path
- Canonical renames retain aliases for at least one minor release
- Deprecated access emits `DeprecationWarning` with replacement and removal floor
- Deprecations are listed in `api_surface()` machine-readable metadata
- Removal requires compatibility-baseline update and release notes

## Feature Availability

Use `eggsec._feature_guard.list_unavailable_features()` to introspect
which features are not compiled into the current wheel. This does not
import any feature-gated native code.

```python
from eggsec._feature_guard import list_unavailable_features
unavailable = list_unavailable_features()
for f in unavailable:
    print(f"{f['feature']}: {f['install_hint']}")
```

## Migration Guide

### Before (flat namespace)

```python
from eggsec import TargetPy, TcpSessionPy, HttpClientPy
from eggsec import wireless_scan, evasion_scan
```

### After (organized submodules)

```python
from eggsec.net import Target, TcpSession, HttpClient
from eggsec.experimental import wireless_scan, evasion_scan
```

### Backward-compatible (deprecated)

```python
from eggsec import TargetPy  # works but emits DeprecationWarning
from eggsec import wireless_scan  # works if feature compiled
```
