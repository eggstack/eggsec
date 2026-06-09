---
name: security_compliance_checks
description: "Security compliance verification including HTTPS, headers, cookies, and CORS"
triggers:
  - compliance
  - header
  - headers
  - security header
  - cors
  - cookie
  - hsts
  - csp
  - x-frame-options
  - x-content-type
  - referrer-policy
  - permissions-policy
  - https redirect
metadata:
  category: compliance
  tools: [compliance]
  scope: targets
---

## Overview

Security compliance checks verify that web applications implement recommended security controls for HTTPS enforcement, security headers, cookie attributes, and CORS policies.

## Capabilities

| Check | Description | Severity |
|-------|-------------|----------|
| HTTPS Enforcement | Redirect HTTP to HTTPS | High |
| HSTS | Strict-Transport-Security header | High |
| X-Content-Type-Options | MIME sniffing protection | Medium |
| X-Frame-Options | Clickjacking protection | Medium |
| CSP | Content Security Policy | Medium |
| Referrer-Policy | Referrer information control | Low |
| Permissions-Policy | Browser feature restrictions | Low |
| Cache-Control | Sensitive page caching | Medium |
| HttpOnly Cookie | XSS cookie protection | High |
| Secure Cookie | HTTPS-only cookie | High |
| SameSite Cookie | CSRF cookie protection | Medium |
| CORS Wildcard | Excessive CORS permissions | High |
| X-XSS-Protection | Legacy XSS filter (deprecated) | Low |

## Usage

### Run Compliance Checks

```bash
eggsec compliance --target https://example.com
```

### Specific Check

```bash
eggsec compliance --target https://example.com --check hsts,csp
```

### Full Report

```bash
eggsec compliance --target https://example.com --format json --output compliance.json
```

## Header Requirements

```http
Strict-Transport-Security: max-age=31536000; includeSubDomains
X-Content-Type-Options: nosniff
X-Frame-Options: DENY or SAMEORIGIN
Content-Security-Policy: default-src 'self'
Referrer-Policy: strict-origin-when-cross-origin
Permissions-Policy: geolocation=(), microphone=()
```

## Cookie Security

| Attribute | Purpose |
|-----------|---------|
| HttpOnly | Prevents JavaScript access |
| Secure | HTTPS-only transmission |
| SameSite=Strict | Prevents CSRF |
| SameSite=Lax | Most restrictive after initial use |

## Triggers

Keywords: compliance, header, headers, security, cors, cookie, hsts, csp, x-frame, x-content, referrer, cache, https, redirect, security header, check, audit

## Best Practices

1. Always enforce HTTPS before checking security headers
2. Implement HSTS with includeSubDomains and preload
3. Use strict CSP policy (avoid unsafe-inline if possible)
4. Set all cookie security attributes
5. Review CORS configuration for least privilege