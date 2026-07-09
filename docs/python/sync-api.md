# Sync API Reference

The Python bindings provide a synchronous API over the async Rust engine. The GIL is released during all network I/O, so other Python threads run freely while scans execute.

## Functions

### `eggsec.scan_ports(target, ports, scope, *, concurrency=100, timeout_ms=5000)`

Convenience function for a single scoped port scan. Internally creates an ephemeral `Client`.

**Parameters:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `target` | `str` | *(required)* | Hostname or IP to scan |
| `ports` | `list[int]` | *(required)* | Port numbers to scan |
| `scope` | `Scope` | *(required)* | Authorization scope |
| `concurrency` | `int` | `100` | Max concurrent connections |
| `timeout_ms` | `int` | `5000` | Connection timeout (ms) |

**Returns:** `PortScanResult`

**Raises:**

- `EnforcementError` — target or a port is outside scope
- `ScanError` — scan failed (network error, DNS resolution, etc.)

```python
import eggsec

scope = eggsec.Scope.allow_hosts(["127.0.0.1"])
result = eggsec.scan_ports("127.0.0.1", [80, 443], scope)
```

## Classes

### `Client(scope, *, mode="manual", concurrency=100, timeout_ms=5000)`

Reusable scan client bound to a scope. Preferred for repeated scans.

**Constructor parameters:**

| Name | Type | Default | Description |
|------|------|---------|-------------|
| `scope` | `Scope` | *(required)* | Authorization scope |
| `mode` | `str` | `"manual"` | `"manual"` or `"automation"` |
| `concurrency` | `int` | `100` | Default max concurrent connections |
| `timeout_ms` | `int` | `5000` | Default connection timeout (ms) |

**Raises:** `ValueError` — if `mode` is not `"manual"` or `"automation"`.

#### `Client.scan_ports(target, ports, *, concurrency=None, timeout_ms=None) -> PortScanResult`

Scan ports on a target. Keyword-only parameters override client defaults for this call.

**Raises:** `EnforcementError`, `ScanError`

#### `Client.scope -> Scope`

Read-only property. Returns the client's scope.

#### `Client.mode -> str`

Read-only property. Returns the client's execution mode.

```python
client = eggsec.Client(
    scope=eggsec.Scope.allow_hosts(["10.0.0.0/24"]),
    concurrency=200,
)
result = client.scan_ports("10.0.0.1", [22, 80])
```

### `Scope`

Frozen scope configuration. Created via static factory methods, not a constructor.

#### `Scope.allow_hosts(hosts: list[str]) -> Scope`

Allow specific hostnames or CIDRs.

```python
scope = eggsec.Scope.allow_hosts(["example.com", "10.0.0.0/8"])
```

#### `Scope.allow_cidrs(cidrs: list[str]) -> Scope`

Allow only CIDR ranges.

```python
scope = eggsec.Scope.allow_cidrs(["192.168.0.0/16", "10.0.0.0/8"])
```

#### `Scope.deny_all() -> Scope`

Deny all targets. Useful as a safety default.

#### `Scope.from_file(path: str) -> Scope`

Load scope from a TOML or YAML file. Raises `ScopeError` on parse failure.

#### `Scope.is_target_allowed(target: str) -> bool`

Check if a target is within scope.

#### `Scope.is_port_allowed(port: int) -> bool`

Check if a port is within scope.

### `PortScanResult`

Immutable scan result. Returned by `scan_ports()` and `Client.scan_ports()`.

| Field | Type | Description |
|-------|------|-------------|
| `target` | `str` | Scanned host |
| `open_ports` | `list[OpenPort]` | Open ports found |
| `scanned_ports` | `int` | Total ports attempted |
| `elapsed_ms` | `int` | Scan duration in ms |
| `stats` | `ScanStats` | Aggregate stats |

Methods:

- `to_dict() -> dict` — convert to a plain Python dictionary
- `to_json() -> str` — serialize to a JSON string

### `OpenPort`

| Field | Type | Description |
|-------|------|-------------|
| `port` | `int` | Port number |
| `protocol` | `str` | Always `"tcp"` (Phase B) |
| `service` | `str` | Service name (e.g. `"http"`, `"ssh"`) |
| `banner` | `str \| None` | Banner text, if captured |
| `confidence` | `float` | Service detection confidence (0.0–1.0) |

### `ScanStats`

| Field | Type | Description |
|-------|------|-------------|
| `ports_scanned` | `int` | Ports attempted |
| `total_open` | `int` | Open ports found |
| `elapsed_ms` | `int` | Duration in ms |

### `PortRange`

Helper for building port lists. Use static factory methods.

```python
ports = eggsec.PortRange.list([22, 80, 443])
ports = eggsec.PortRange.range(1, 1024)
ports = eggsec.PortRange.top_100()

# Access the list via the .ports property
client.scan_ports("10.0.0.1", ports.ports)
```

### `TimingPreset`

Scan timing profiles. Not yet wired to scan parameters in Phase B; provided for API completeness.

```python
preset = eggsec.TimingPreset.normal()
```

Available: `paranoid`, `sneaky`, `polite`, `normal`, `aggressive`, `insane`.

## Exception Hierarchy

```
EggsecError
├── ConfigError
├── ScopeError
├── EnforcementError
├── NetworkError
├── ScanError
├── TimeoutError
├── FeatureUnavailableError
├── SerializationError
└── InternalError
```

All exceptions inherit from `EggsecError`, which inherits from Python's built-in `Exception`.
