# Endpoint Discovery Guide

Endpoint discovery scans a web server for known paths and directories. It sends HTTP requests to a list of endpoints and identifies which ones exist.

## Configuration

### `EndpointScanConfig(base_url, endpoints, *, concurrency=50, timeout_ms=5000, include_404=False, verify_tls=True)`

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `base_url` | `str` | *(required)* | Target URL (e.g. `"http://10.0.0.1"`) |
| `endpoints` | `list[str]` | *(required)* | Paths to probe (e.g. `["/admin", "/login"]`) |
| `concurrency` | `int` | `50` | Max concurrent requests |
| `timeout_ms` | `int` | `5000` | Request timeout (ms) |
| `include_404` | `bool` | `False` | Include 404 responses in results |
| `verify_tls` | `bool` | `True` | Verify TLS certificates |

```python
config = eggsec.EndpointScanConfig(
    base_url="http://10.0.0.1",
    endpoints=["/", "/admin", "/login", "/api/v1", "/robots.txt"],
    concurrency=100,
    timeout_ms=3000,
)
```

## Scanning

### Using `Client`

```python
scope = eggsec.Scope.allow_hosts(["10.0.0.1"])
client = eggsec.Client(scope)
result = client.scan_endpoints(config)
```

### Convenience Function

```python
result = eggsec.scan_endpoints(config, scope)
```

### Async

```python
future = client.scan_endpoints(config)
for r in future:
    if r is not None:
        result = r
```

## Results

### `EndpointScanResult`

| Field | Type | Description |
|-------|------|-------------|
| `target` | `str` | Base URL scanned |
| `scanned` | `int` | Endpoints probed |
| `found` | `int` | Endpoints that returned non-404 |
| `matched` | `int` | Total matches (including duplicates) |
| `interesting` | `int` | Endpoints flagged as interesting |
| `elapsed_ms` | `int` | Scan duration (ms) |
| `findings` | `list[EndpointFinding]` | Individual endpoint results |

Methods: `to_dict()`, `to_json()`

### `EndpointFinding`

| Field | Type | Description |
|-------|------|-------------|
| `path` | `str` | Endpoint path |
| `status` | `int` | HTTP status code |
| `status_text` | `str` | Status text (e.g. `"OK"`) |
| `content_length` | `int \| None` | Response body size |
| `response_ms` | `int` | Response time (ms) |
| `redirect` | `str \| None` | Redirect URL if any |
| `interesting` | `bool` | Engine flagged as interesting |

Methods: `to_dict()`, `to_json()`

## Example

```python
import eggsec

scope = eggsec.Scope.allow_hosts(["web.example.com"])
client = eggsec.Client(scope)

config = eggsec.EndpointScanConfig(
    base_url="https://web.example.com",
    endpoints=[
        "/", "/admin", "/login", "/api", "/robots.txt",
        "/.env", "/backup", "/config", "/debug",
    ],
    timeout_ms=5000,
)

result = client.scan_endpoints(config)

print(f"Found {result.found}/{result.scanned} endpoints in {result.elapsed_ms}ms")

for finding in result.findings:
    marker = " [INTERESTING]" if finding.interesting else ""
    print(f"  {finding.status} {finding.path}{marker}")
```
