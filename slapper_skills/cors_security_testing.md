---
name: cors_security_testing
description: "Cross-Origin Resource Sharing security testing and misconfiguration detection"
triggers:
  - cors
  - cross origin
  - origin
  - access-control
  - ACAO
  - wildcard
  - preflight
  - cross-domain
metadata:
  category: api_testing
  tools: [fuzzer]
  scope: targets
---

## Overview

CORS testing identifies misconfigurations in Cross-Origin Resource Sharing that could allow unauthorized access to sensitive data or enable attacks from malicious origins.

## Capabilities

- Access-Control-Allow-Origin analysis
- Credential transmission assessment
- Preflight request testing
- Wildcard origin detection
- Method and header testing
- Null origin security
- Integration with other origins
- Same-site cookie security

## Usage

### Basic CORS Test

```bash
slapper fuzz --target https://api.example.com --type cors
```

### Test with Credentials

```bash
slapper fuzz --target https://api.example.com/auth --type cors --credentials
```

### Test Preflight Handling

```bash
slapper fuzz --target https://api.example.com/api --type cors --preflight
```

## Security Issues

| Configuration | Risk | Severity |
|--------------|------|----------|
| `Access-Control-Allow-Origin: *` | Data exposure | High |
| `Access-Control-Allow-Credentials: true` + wildcard | Token theft | Critical |
| `Access-Control-Allow-Origin: null` | Sandbox escape | High |
| No restrictions | CSRF + data theft | Medium |

## Triggers

Keywords: cors, cross origin, origin, access-control, acao, acac, wildcard, preflight, cross-domain, OPTIONS, csrf, jsonp

## Best Practices

1. Never use `Access-Control-Allow-Origin: *` with credentials
2. Validate origins against allowlist
3. Use SameSite cookies for sensitive data
4. Limit allowed methods and headers
5. Implement CORS early in development