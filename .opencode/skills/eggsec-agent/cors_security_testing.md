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

CORS analysis runs inside the recon pipeline (`CorsAnalyzer`, `recon/cors.rs`);
there is no `cors` fuzz payload type:

```bash
# Full pipeline includes the CORS configuration check
eggsec recon https://api.example.com

# CORS-focused run: skip unrelated stages
eggsec recon https://api.example.com \
  --no-tech --no-geo --no-whois --no-subdomains --no-ssl \
  --no-dns-records --no-js --no-content --no-cloud \
  --no-wayback --no-threat --no-cve --no-email-security

# Header-manipulation probes that exercise origin reflection
eggsec fuzz "https://api.example.com/api" -t headers
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