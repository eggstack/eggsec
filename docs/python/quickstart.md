# Quick Start

Get scanning in under a minute.

## Install

```bash
# Development install (editable)
cd crates/eggsec-python
pip install maturin pytest
maturin develop

# Or from a built wheel
pip install dist/eggsec-*.whl
```

Verify:

```bash
python -c "import eggsec; print(eggsec.__version__)"
```

## Minimal Example

```python
import eggsec

# 1. Define scope — only scan what you own
scope = eggsec.Scope.allow_hosts(["127.0.0.1"])

# 2. Scan
result = eggsec.scan_ports(
    target="127.0.0.1",
    ports=[22, 80, 443],
    scope=scope,
)

# 3. Inspect results
print(result)
# Scan of 127.0.0.1: 2 open ports (3 scanned in 42ms)

for port in result.open_ports:
    print(f"  {port}")
    # 22/tcp - ssh
    # 80/tcp - http
```

## Client API

For repeated scans against the same scope, create a `Client` to avoid re-validating scope each time:

```python
import eggsec

scope = eggsec.Scope.allow_hosts(["10.0.0.0/24"])
client = eggsec.Client(scope, concurrency=200, timeout_ms=3000)

result = client.scan_ports("10.0.0.1", [22, 80, 443, 8080])
print(result.stats)
# ScanStats(scanned=4, open=2, elapsed_ms=1200)
```

## Result Structure

Every scan returns a `PortScanResult` with:

| Field | Type | Description |
|-------|------|-------------|
| `target` | `str` | Hostname or IP scanned |
| `open_ports` | `list[OpenPort]` | Open ports found |
| `scanned_ports` | `int` | Total ports attempted |
| `elapsed_ms` | `int` | Wall-clock time in milliseconds |
| `stats` | `ScanStats` | Aggregate statistics |

Each `OpenPort` has: `port`, `protocol`, `service`, `banner` (optional), `confidence`.

Use `to_dict()` or `to_json()` to export results for serialization:

```python
data = result.to_dict()
json_str = result.to_json()
```

## Async API

For non-blocking scans, use `AsyncClient`:

```python
import eggsec

scope = eggsec.Scope.allow_hosts(["127.0.0.1"])

async def scan():
    async with eggsec.AsyncClient(scope) as client:
        future = client.scan_ports("127.0.0.1", [80, 443])
        result = await future
        print(result)

# Or poll manually in a thread
future = eggsec.async_scan_ports("127.0.0.1", [80, 443], scope)
for result in future:
    if result is not None:
        print(result)
```

## Endpoint Discovery

Scan a web server for known paths:

```python
config = eggsec.EndpointScanConfig(
    base_url="http://127.0.0.1",
    endpoints=["/", "/admin", "/login", "/robots.txt"],
)
result = eggsec.scan_endpoints(config, scope)
print(f"Found {result.found} endpoints")
```

## Service Fingerprinting

Identify services on open ports:

```python
result = eggsec.fingerprint_services("127.0.0.1", [22, 80, 443], scope)
for svc in result.services:
    print(f"  {svc.port}: {svc.service} {svc.version or ''}")
```

## Next Steps

- [Sync API Reference](sync-api.md) — full function and class docs
- [Async API](async-api.md) — non-blocking scan operations
- [Scanner Guide](scanner.md) — port ranges, timing, common patterns
- [Endpoint Discovery](endpoint-discovery.md) — HTTP endpoint scanning
- [Service Fingerprinting](service-fingerprinting.md) — service identification
- [Scope & Safety](scope-and-safety.md) — authorization and enforcement details
