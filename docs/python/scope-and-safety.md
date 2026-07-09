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

## Best Practices

1. **Be specific** — allow only the hosts and ports you need. Start with `deny_all()` and add rules.
2. **Use `allow_cidrs()`** for network ranges, `allow_hosts()` for individual targets.
3. **Load scope from files** for team-shared configurations rather than hardcoding in scripts.
4. **Catch `EnforcementError`** explicitly to distinguish scope violations from network errors.
5. **Prefer `Client` over the convenience function** when scanning multiple targets — the scope is validated once at construction, and the client object makes it clear what's authorized.
6. **Never disable scope** — the `Scope` class has no "allow all" constructor. If you need to scan broad ranges, use `Scope.allow_cidrs(["0.0.0.0/0"])` and accept the authorization responsibility.

## Exception Reference

| Exception | Cause |
|-----------|-------|
| `ScopeError` | Scope file could not be read or parsed |
| `EnforcementError` | Target or port is outside the allowed scope |
| `ScanError` | Network failure during scanning |
| `TimeoutError` | Connection exceeded the timeout |

All inherit from `EggsecError` → `Exception`.
