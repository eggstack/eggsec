---
name: authentication_security_testing
description: "Authentication security testing including brute force, credential stuffing, and MFA bypass"
triggers:
  - auth
  - authentication
  - login
  - brute force
  - credential stuffing
  - password
  - mfa
  - totp
  - bypass
  - session
  - session fixation
metadata:
  category: authentication
  tools: [auth_test]
  scope: targets
---

## Overview

Authentication testing evaluates the security of login mechanisms including resistance to brute force attacks, credential stuffing, MFA bypass techniques, and session security.

## Capabilities

- Login form brute force testing
- Credential stuffing with breach databases
- Username enumeration detection
- Account lockout testing
- MFA/2FA bypass techniques
- OTP brute forcing
- Session fixation detection
- Session timeout testing
- "Remember me" security
- Password policy enforcement

## Usage

### Basic Brute Force Test

```bash
eggsec auth-test --target https://example.com/login --brute-force
eggsec auth-test --target https://example.com/login --wordlist /path/to/passwords.txt
```

### Credential Stuffing

```bash
eggsec auth-test --target https://example.com/login --credential-file /path/to/breach.txt
```

### MFA Bypass Test

```bash
eggsec auth-test --target https://example.com/login --test-mfa
```

## Common Vulnerabilities

| Issue | Description | Severity |
|-------|-------------|----------|
| No Rate Limiting | Brute force possible | Critical |
| No Account Lockout | Unlimited guesses | High |
| Username Enumeration | User ID discovery | Medium |
| Weak MFA | OTP can be bruteforced | High |
| Session Fixation | Session ID not regenerated | Medium |
| Predictable Tokens | Session hijacking | Critical |

## Triggers

Keywords: auth, authentication, login, brute force, credentials, password, mfa, totp, bypass, session, cookie, token, credential stuffing, lockout, otp, single sign on, sso

## Best Practices

1. Always test rate limiting on login endpoints
2. Check for username enumeration via error messages
3. Test MFA with OTP codes (6-digit = 1M possibilities)
4. Verify session IDs are regenerated after login
5. Check "remember me" token security (long-lived cookies)