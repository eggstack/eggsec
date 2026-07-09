# WAF Detection Guide

Web Application Firewall (WAF) detection identifies whether a target is protected by a WAF and which product is in use.

## Detection

### Using `Client`

```python
scope = eggsec.Scope.allow_hosts(["example.com"])
client = eggsec.Client(scope, mode="manual")
result = client.detect_waf("https://example.com")
```

### Convenience Function

```python
scope = eggsec.Scope.allow_hosts(["example.com"])
result = eggsec.detect_waf("https://example.com", scope)
```

### Async

```python
scope = eggsec.Scope.allow_hosts(["example.com"])
future = eggsec.async_detect_waf("https://example.com", scope)
for r in future:
    if r is not None:
        result = r
```

## WAF Validation

Active WAF validation tests bypass techniques against a target. Requires scope enforcement.

### Using `Client`

```python
scope = eggsec.Scope.allow_hosts(["example.com"])
client = eggsec.Client(scope, mode="manual")
result = client.validate_waf("https://example.com", bypass=True)
```

### Convenience Function

```python
scope = eggsec.Scope.allow_hosts(["example.com"])
result = eggsec.validate_waf("https://example.com", scope, bypass=True)
```

### Async

```python
scope = eggsec.Scope.allow_hosts(["example.com"])
future = eggsec.async_validate_waf("https://example.com", scope, bypass=True)
for r in future:
    if r is not None:
        result = r
```

## HTTP Fuzzing

HTTP fuzzing sends crafted payloads to identify WAF behavior and weaknesses. Requires scope enforcement.

### Using `Client`

```python
scope = eggsec.Scope.allow_hosts(["example.com"])
client = eggsec.Client(scope, mode="manual")
result = client.fuzz_http("https://example.com", payload_type="xss")
```

### Convenience Function

```python
scope = eggsec.Scope.allow_hosts(["example.com"])
result = eggsec.fuzz_http("https://example.com", scope, payload_type="xss")
```

### Async

```python
scope = eggsec.Scope.allow_hosts(["example.com"])
future = eggsec.async_fuzz_http("https://example.com", scope, payload_type="xss")
for r in future:
    if r is not None:
        result = r
```

## Results

### `WafDetectionResult`

| Field | Type | Description |
|-------|------|-------------|
| `detected` | `bool` | WAF detected |
| `waf_name` | `str \| None` | WAF product name |
| `confidence` | `int` | Detection confidence (0-100) |
| `request_error` | `str \| None` | HTTP request error if any |
| `matched_headers` | `list[str]` | Headers that matched WAF signatures |
| `matched_cookies` | `list[str]` | Cookies that matched WAF signatures |
| `matched_patterns` | `list[str]` | Body patterns that matched |
| `server_header` | `str \| None` | Server response header |
| `status_code` | `int` | HTTP status code |

### Output Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `to_dict()` | `dict` | Python dictionary |
| `to_json()` | `str` | JSON string |

## Supported WAF Products

Detection signatures cover 18+ WAF products:

- Cloudflare
- Akamai
- AWS WAF
- Imperva / Incapsula
- F5 ASM
- Azure WAF
- FortiWeb
- ModSecurity
- Sucuri
- Barracuda
- DenyAll
- Radware
- Safe3
- DotDefender
- StackPath
- Fastly
- CloudFront
- Generic ModSecurity

## Example

```python
import json
import eggsec

scope = eggsec.Scope.allow_hosts(["example.com"])
client = eggsec.Client(scope, mode="manual", timeout_ms=5000)

result = client.detect_waf("https://example.com")

if result.detected:
    print(f"WAF detected: {result.waf_name} ({result.confidence}% confidence)")
    if result.matched_headers:
        print(f"Matched headers: {result.matched_headers}")
else:
    print("No WAF detected")

# Export
print(json.dumps(result.to_dict(), indent=2))
```
