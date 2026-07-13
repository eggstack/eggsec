# Network Programmability Guide

Release 2 exposes reusable network and protocol primitives for building custom scanners, probes, and traffic-analysis workflows.

## Quick Start

### Target Resolution

```python
from eggsec import TargetPy, resolve_target_sync

target = TargetPy("example.com", port=443, scheme="https")
resolved = resolve_target_sync(target)
print(resolved.resolved_ips)  # ['93.184.216.34']
```

### TCP Banner Probe

```python
from eggsec import banner_probe

result = banner_probe("example.com", 80, timeout_ms=3000)
if result.banner_text:
    print(f"Banner: {result.banner_text}")
```

### DNS Query

```python
from eggsec import dns_query

result = dns_query("example.com", record_types=["A", "MX", "TXT"])
for record in result.records:
    print(f"{record.record_type}: {record.data}")
```

### TLS Inspection

```python
from eggsec import tls_probe

result = tls_probe("example.com", port=443)
if result.certificate:
    print(f"Issuer: {result.certificate.issuer}")
    print(f"Expires: {result.certificate.valid_until}")
```

### HTTP Client

```python
from eggsec import HttpClientPy, HttpClientConfigPy, HttpRequestPy

config = HttpClientConfigPy(verify_tls=True, timeout_ms=10000)
client = HttpClientPy(config)

req = HttpRequestPy(method="GET", url="https://example.com/api")
response = client.request(req)
print(f"Status: {response.status_code}")
print(f"Headers: {response.headers}")
```

### WebSocket Session

```python
from eggsec import WebSocketSessionPy, WebSocketSessionConfigPy

config = WebSocketSessionConfigPy(url="wss://echo.websocket.org")
session = WebSocketSessionPy(config)
handshake = session.connect()
print(f"Connected: {handshake.status_code}")

session.send_text("Hello, WebSocket!")
message = session.recv()
print(f"Received: {message.text_content}")

session.close()
```

## Managed Sessions

All session types (`TcpSessionPy`, `UdpSocketPy`, `WebSocketSessionPy`, `HttpClientPy`) implement Python context managers for deterministic cleanup:

```python
from eggsec import TcpSessionPy, TcpConfigPy

config = TcpConfigPy("example.com", 80)
with TcpSessionPy(config) as session:
    result = session.connect()
    session.write(b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n")
    data = session.read(4096)
    print(data.data)
# Session closed automatically
```

## Async API

All operations have async counterparts:

```python
import asyncio
from eggsec import AsyncHttpClientPy, HttpClientConfigPy, HttpRequestPy

async def main():
    config = HttpClientConfigPy()
    async with AsyncHttpClientPy(config) as client:
        req = HttpRequestPy(method="GET", url="https://example.com")
        response = await client.async_request(req)
        print(f"Status: {response.status_code}")

asyncio.run(main())
```

## Transcript and Evidence

Network sessions optionally capture transcripts for forensic analysis:

```python
from eggsec import TcpSessionPy, TcpConfigPy

config = TcpConfigPy("example.com", 80)
session = TcpSessionPy(config)
result = session.connect()

# Transcript is captured automatically
session.write(b"...")
data = session.read(4096)
session.close()

# Access transcript
for entry in session.transcript:
    print(f"[{entry.direction}] {entry.size} bytes")
```

## Redaction

HTTP responses support automatic redaction of sensitive headers:

```python
from eggsec import RedactConfigPy

redact = RedactConfigPy(
    redact_headers=["Authorization", "Cookie", "X-API-Key"],
    redact_query_params=["token", "api_key"]
)
# Response.redacted_headers() masks sensitive values
```
