# Scanner Module Guide

Phase B exposes scoped TCP port scanning from Python. This guide covers how scanning works, port specification, result interpretation, and common patterns.

## How It Works

1. **Scope enforcement** — before any connection is made, the engine validates the target and every port against the provided `Scope`. Violations raise `EnforcementError` immediately, without network I/O.

2. **Concurrent TCP probes** — connections are attempted in parallel up to the configured concurrency limit. The GIL is released during I/O so other Python threads continue.

3. **Service detection** — open ports are fingerprinted using protocol banners and response patterns. Each `OpenPort` includes a service name and confidence score.

4. **Result aggregation** — results are collected into a `PortScanResult` with structured stats.

## Port Specification

Pass port numbers as a plain `list[int]`, or use `PortRange` helpers:

```python
import eggsec

# Explicit list
ports = [22, 80, 443, 8080]

# Range (inclusive)
ports = eggsec.PortRange.range(1, 1024).ports

# Top 100 most common ports
ports = eggsec.PortRange.top_100().ports

# Custom list via PortRange
ports = eggsec.PortRange.list([80, 443, 8000, 8443]).ports
```

`PortRange` is a convenience wrapper. The `.ports` property returns a `list[int]` suitable for passing to `scan_ports()`.

## Timing Presets

`TimingPreset` defines scan speed profiles (paranoid through insane). In Phase B these are not yet wired to the `concurrency`/`timeout_ms` parameters — they are exposed for API completeness and forward compatibility.

## Result Structure

```python
result = eggsec.scan_ports("10.0.0.1", [22, 80], scope)

# result.open_ports -> list[OpenPort]
for p in result.open_ports:
    print(p.port, p.service, p.confidence)

# result.stats -> ScanStats
print(result.stats.ports_scanned)   # 2
print(result.stats.total_open)      # 1
print(result.stats.elapsed_ms)      # 340

# Serialize
d = result.to_dict()
j = result.to_json()
```

## Common Patterns

### Scan a single host

```python
scope = eggsec.Scope.allow_hosts(["web.example.com"])
result = eggsec.scan_ports("web.example.com", [80, 443], scope)
```

### Scan an internal network

```python
scope = eggsec.Scope.allow_cidrs(["10.0.0.0/24"])
client = eggsec.Client(scope, concurrency=200)

for i in range(1, 255):
    result = client.scan_ports(f"10.0.0.{i}", [22, 80, 443])
    if result.open_ports:
        print(f"10.0.0.{i}: {[p.port for p in result.open_ports]}")
```

### Scan with custom timeout

```python
result = eggsec.scan_ports(
    "10.0.0.1",
    [80, 443],
    scope=scope,
    timeout_ms=10000,  # 10 second timeout
)
```

### Export results to JSON

```python
import json

result = eggsec.scan_ports("10.0.0.1", [22, 80], scope)
data = json.loads(result.to_json())

with open("scan-results.json", "w") as f:
    json.dump(data, f, indent=2)
```

## Error Handling

```python
import eggsec

scope = eggsec.Scope.allow_hosts(["10.0.0.0/24"])

try:
    result = eggsec.scan_ports("192.168.1.1", [80], scope)
except eggsec.EnforcementError as e:
    print(f"Target outside scope: {e}")
except eggsec.ScanError as e:
    print(f"Scan failed: {e}")
except eggsec.TimeoutError as e:
    print(f"Connection timed out: {e}")
```

All scanner errors inherit from `eggsec.EggsecError`.
