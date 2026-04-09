---
name: oauth_oidc_testing
description: "OAuth 2.0 and OIDC security testing for authentication and authorization flaws"
triggers:
  - oauth
  - oidc
  - openid
  - authentication
  - authorization
  - token
  - jwt
  - s256
  - pkce
  - csrf
  - state parameter
metadata:
  category: api_testing
  tools: [fuzzer]
  scope: targets
---

## Overview

OAuth/OIDC testing discovers vulnerabilities in authentication flows including token validation issues, redirect URI bypass, CSRF in authorization code flow, and JWT manipulation.

## Capabilities

- Authorization code flow testing
- Implicit flow testing
- Client credentials testing
- PKCE bypass detection
- Redirect URI validation testing
- State parameter CSRF testing
- JWT algorithm manipulation
- Token introspection testing
- Refresh token rotation testing
- Scope escalation testing

## Usage

### Basic OAuth Test

```bash
slapper oauth --target https://auth.example.com --test-flow authorization_code
```

### JWT Security Testing

```bash
slapper oauth --target https://api.example.com --test-jwt
```

### PKCE Bypass Test

```bash
slapper oauth --target https://auth.example.com --test-pkce
```

## Common Vulnerabilities

| Issue | Description | Severity |
|-------|-------------|----------|
| State Missing | CSRF vulnerability | High |
| Redirect URI Bypass | Authorization code theft | Critical |
| Weak JWT Algo | Algorithm confusion attack | High |
| Token Not Revoked | Session fixation | Medium |
| Scope Overlap | Privilege escalation | High |

## JWT Attack Patterns

**Algorithm Confusion:**
```json
{"alg": "HS256"} -> {"alg": "none"}
```

**Key Confusion (RS256 -> HS256):**
Use the public RSA key as HMAC secret.

**Null Signature:**
```json
{"alg": "none", "typ": "JWT"}
```

## Triggers

Keywords: oauth, oidc, openid, authentication, authorization, token, jwt, bearer, pkce, s256, state, csrf, redirect, authorize, access token, refresh token

## Best Practices

1. Always verify state parameter presence and randomness
2. Test redirect URI with variations (subdomain, path)
3. Never accept "alg: none" in JWT validation
4. Check token expiration and revocation
5. Verify scope restrictions are enforced