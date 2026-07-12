# Scope and Safety

Eggsec enforces scope at every scan call. This prevents accidental scanning of unauthorized targets — even if you write incorrect code, the engine blocks it before any network connection is made.

## Why Scope Matters

- **Authorization** — you should only scan systems you own or have written permission to test.
- **Safety** — scope enforcement is a hard gate, not advisory. `EnforcementError` is raised before I/O.
- **Audit** — scope is attached to every client and result, providing a clear record of what was authorized.

## Creating Scopes

### Allow specific hosts

```python
scope = eggsec.Scope.allow_hosts([
    "example.com",
    "10.0.0.1",
    "192.168.1.0/24",  # CIDRs are supported here too
])
```

Hostnames are matched exactly. CIDR notation (containing `/`) triggers CIDR matching.

### Allow CIDR ranges only

```python
scope = eggsec.Scope.allow_cidrs([
    "10.0.0.0/8",
    "172.16.0.0/12",
])
```

### Deny everything

```python
scope = eggsec.Scope.deny_all()
```

Returns a scope with no allowed targets. Any scan attempt raises `EnforcementError`.

### Load from file

```python
scope = eggsec.Scope.from_file("scope.toml")
```

Supports TOML and YAML. Raise `ScopeError` if the file is missing or unparseable.

## Scope File Format

```toml
# scope.toml
require_explicit_scope = true

[[allowed_targets]]
pattern = "example.com"
description = "Primary web app"

[[allowed_targets]]
cidr = "10.0.0.0/24"
description = "Internal network"

[[allowed_targets]]
pattern = "*.staging.example.com"
description = "Staging subdomains"
```

## How Enforcement Works

Every call to `scan_ports()` or `Client.scan_ports()` performs two checks **before** any network I/O:

1. **Target check** — `Scope.is_target_allowed(target)` validates the hostname or IP against allowed patterns and CIDR ranges.
2. **Port check** — `Scope.is_port_allowed(port)` validates each requested port number.

If either check fails, an `EnforcementError` is raised immediately. No connections are opened.

```python
scope = eggsec.Scope.allow_hosts(["10.0.0.0/24"])

# EnforcementError: target outside scope
eggsec.scan_ports("192.168.1.1", [80], scope)

# EnforcementError: port outside scope
eggsec.scan_ports("10.0.0.1", [3306], scope)  # if port not allowed
```

## Active API Scope Enforcement

The following APIs perform **mandatory scope enforcement at the Python layer** before any engine work is dispatched. Scope is a required positional parameter for standalone functions; `Client`/`AsyncClient` methods use the client's internal scope:

| API | Standalone Signature | Client Method |
|-----|---------------------|---------------|
| `load_test_http` | `(url, total_requests, concurrency, timeout_secs, scope, *, method="GET")` | `Client.load_test_http(url, total_requests, concurrency, timeout_secs, *, method="GET")` |
| `validate_waf` | `(url, scope, *, bypass=False, test_type=None)` | `Client.validate_waf(url, *, bypass=False, test_type=None)` |
| `fuzz_http` | `(url, scope, payload_type="all", *, method="GET", param=None, concurrency=10, timeout=30)` | `Client.fuzz_http(url, payload_type="all", *, method="GET", param=None, concurrency=10, timeout=30)` |

**Enforcement flow:**

1. The Python binding extracts the host from the target URL.
2. `Scope.is_target_allowed(host)` is called. If the host is outside scope, `EnforcementError` is raised immediately.
3. For `load_test_http`, additional validation ensures `total_requests > 0`, `concurrency > 0`, and `timeout_secs > 0`. Violations raise `ValueError`.
4. Only after scope enforcement passes does the call dispatch to the engine.

This means even if you bypass the `Client` and use standalone functions, scope is still enforced. There is no way to perform active operations against out-of-scope targets.

```python
scope = eggsec.Scope.allow_hosts(["example.com"])

# EnforcementError: host outside scope
eggsec.validate_waf("https://evil.com", scope)
eggsec.fuzz_http("https://evil.com", scope)
eggsec.load_test_http("https://evil.com", 100, 10, 30, scope)

# ValueError: caps violated
eggsec.load_test_http("https://example.com", 0, 10, 30, scope)  # total_requests=0
```

## Best Practices

1. **Be specific** -- allow only the hosts and ports you need. Start with `deny_all()` and add rules.
2. **Use `allow_cidrs()`** for network ranges, `allow_hosts()` for individual targets.
3. **Load scope from files** for team-shared configurations rather than hardcoding in scripts.
4. **Catch `EnforcementError`** explicitly to distinguish scope violations from network errors.
5. **Prefer `Client` over the convenience function** when scanning multiple targets -- the scope is validated once at construction, and the client object makes it clear what's authorized.
6. **Never disable scope** -- the `Scope` class has no "allow all" constructor. If you need to scan broad ranges, use `Scope.allow_cidrs(["0.0.0.0/0"])` and accept the authorization responsibility.
7. **Use `DomainRegistry`** to discover available capability domains before dispatching operations.

## Domain Descriptors

Domains group operations under logical capability areas. Each domain
declares what operations it provides, what feature gate controls it,
and whether it is available in the current build.

```python
from eggsec import DomainRegistry

# All known domains (including unavailable ones)
all_domains = DomainRegistry.all_domains()

# Only domains available in this build
available = DomainRegistry.available_domains()

# Find a specific domain
db = DomainRegistry.find("db-pentest")
if db and db.is_available:
    print(f"DB pentest available: {db.operations}")
```

### DomainDescriptor fields

| Property | Type | Description |
|---|---|---|
| `id` | `str` | Domain identifier (e.g. `"db-pentest"`, `"mobile-static"`). |
| `display_name` | `str` | Human-readable name. |
| `description` | `str` | Brief purpose. |
| `category` | `str` | Classification (e.g. `"standard-assessment"`, `"defense-lab"`). |
| `required_feature` | `str \| None` | Cargo feature flag, or None if always available. |
| `operations` | `list[str]` | Operation IDs provided by this domain. |
| `is_available` | `bool` | Whether the domain is compiled into this build. |

## OperationRegistry Enhanced Methods

The `OperationRegistry` provides static methods for querying operation
metadata. Beyond the basic `all_operations()`, `find()`, and
`find_by_tool_id()`, the following methods are available:

```python
from eggsec import OperationRegistry

# Count all operations
total = OperationRegistry.operation_count()

# Operations requiring a specific feature
db_ops = OperationRegistry.operations_for_feature("db-pentest")

# Operations supporting a specific surface
cli_ops = OperationRegistry.operations_for_surface("cli")

# All operation IDs
ids = OperationRegistry.operation_ids()

# All display names
names = OperationRegistry.operation_names()
```

These methods are useful for building dynamic UIs or validating that
required operations are available before dispatching.

## Event Schema Versioning

All events produced by the engine are wrapped in `EventEnvelope` with a
`schema_version` field. This enables backward-compatible evolution:

- New event types can be added without breaking existing consumers.
- Consumers should ignore unknown `event_type` values.
- The `schema_version` field indicates the event schema version
  (currently `"1.0.0"`).

```python
import eggsec

# Check the current event schema version
print(eggsec.EVENT_SCHEMA_VERSION)  # "1.0.0"

# Consume events safely by checking the type
for event in stream:
    if event["event_type"] == "progress":
        handle_progress(event["payload"])
    elif event["event_type"] == "finding":
        handle_finding(event["payload"])
    # Unknown types are silently ignored
```

## Exception Reference

| Exception | Cause |
|-----------|-------|
| `ScopeError` | Scope file could not be read or parsed |
| `EnforcementError` | Target or port is outside the allowed scope |
| `ScanError` | Network failure during scanning |
| `TimeoutError` | Connection exceeded the timeout |

All inherit from `EggsecError` → `Exception`.
