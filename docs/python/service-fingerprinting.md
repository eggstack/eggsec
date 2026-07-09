# Service Fingerprinting Guide

Service fingerprinting identifies running services on open ports by analyzing banners, response patterns, and protocol behavior.

## Scanning

### Using `Client`

```python
scope = eggsec.Scope.allow_hosts(["10.0.0.1"])
client = eggsec.Client(scope)
result = client.fingerprint_services("10.0.0.1", [22, 80, 443])
```

### Convenience Function

```python
result = eggsec.fingerprint_services("10.0.0.1", [22, 80, 443], scope)
```

### Async

```python
future = client.fingerprint_services("10.0.0.1", [22, 80, 443])
for r in future:
    if r is not None:
        result = r
```

### Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `host` | `str` | *(required)* | Target host |
| `ports` | `list[int]` | *(required)* | Ports to fingerprint |
| `concurrency` | `int` | `50` | Max concurrent probes |
| `timeout_ms` | `int` | `5000` | Probe timeout (ms) |

## Results

### `FingerprintScanResult`

| Field | Type | Description |
|-------|------|-------------|
| `target` | `str` | Scanned host |
| `scanned` | `int` | Ports probed |
| `identified` | `int` | Services identified |
| `total` | `int` | Total service matches |
| `elapsed_ms` | `int` | Scan duration (ms) |
| `services` | `list[ServiceFingerprintResult]` | Individual results |

Methods: `to_dict()`, `to_json()`

### `ServiceFingerprintResult`

| Field | Type | Description |
|-------|------|-------------|
| `port` | `int` | Port number |
| `service` | `str` | Detected service name |
| `banner` | `str \| None` | Raw banner text |
| `version` | `str \| None` | Detected version |
| `product` | `str \| None` | Product name |
| `extra` | `str \| None` | Additional info |
| `confidence` | `int` | Detection confidence (0–100) |

Methods: `to_dict()`, `to_json()`

## Example

```python
import eggsec

scope = eggsec.Scope.allow_hosts(["10.0.0.1"])
client = eggsec.Client(scope)

result = client.fingerprint_services(
    "10.0.0.1",
    [22, 80, 443, 3306, 5432, 8080],
    timeout_ms=5000,
)

print(f"Identified {result.identified}/{result.scanned} services in {result.elapsed_ms}ms")

for svc in result.services:
    version = f" {svc.version}" if svc.version else ""
    product = f" ({svc.product})" if svc.product else ""
    print(f"  {svc.port}/tcp — {svc.service}{product}{version} [{svc.confidence}%]")
```

## Combining with Port Scanning

A common pattern is to first discover open ports, then fingerprint them:

```python
scope = eggsec.Scope.allow_hosts(["10.0.0.1"])
client = eggsec.Client(scope)

# Step 1: Find open ports
ports_result = client.scan_ports("10.0.0.1", eggsec.PortRange.top_100().ports)
open_ports = [p.port for p in ports_result.open_ports]

if open_ports:
    # Step 2: Fingerprint discovered services
    fp_result = client.fingerprint_services("10.0.0.1", open_ports)
    for svc in fp_result.services:
        print(f"  {svc.port}: {svc.service} {svc.version or ''}")
```
