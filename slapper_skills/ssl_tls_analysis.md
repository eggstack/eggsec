---
name: ssl_tls_analysis
description: "SSL/TLS certificate analysis and configuration testing for secure communications"
triggers:
  - ssl
  - tls
  - certificate
  - https
  - sslscan
  - sslyze
  - heartbleed
  - poodle
  - cipher
metadata:
  category: reconnaissance
  tools: [recon]
  scope: targets
---

## Overview

SSL/TLS analysis reveals cryptographic configurations, certificate details, and potential vulnerabilities in HTTPS endpoints. This is essential for assessing the security of encrypted communications.

## Capabilities

- Certificate chain validation and analysis
- Supported cipher suites enumeration
- TLS version detection (SSLv2, SSLv3, TLS 1.0-1.3)
- Signature algorithm assessment
- Key exchange strength evaluation
- Certificate expiration monitoring
- Self-signed certificate detection
- Wildcard certificate analysis
- OCSP stapling verification
- CRL checking

## Usage

### Basic SSL Scan

```bash
slapper recon ssl --target https://example.com
```

### Detailed Cipher Analysis

```bash
slapper recon ssl --target https://example.com --detailed
```

### Check Certificate Expiration

```bash
slapper recon ssl --target https://example.com --check-expiry
```

### Test for Vulnerabilities

```bash
slapper recon ssl --target https://example.com --vulnerabilities
```

## Configuration

```toml
[recon.ssl]
timeout = 30
verify_cert = true
check_ revocation = true
```

## Common Vulnerabilities

| Vulnerability | Description | Severity |
|--------------|-------------|----------|
| Heartbleed | OpenSSL heartbeat leak | Critical |
| POODLE | SSLv3 fallback attack | High |
| BEAST | TLS 1.0 CBC attack | Medium |
| FREAK | Export cipher weakness | Medium |
| Logjam | DH key exchange weak | Medium |
| ROBOT | RSA padding oracle | High |

## Triggers

Keywords: ssl, tls, certificate, https, cipher, crypto, heartbleed, poodle, beast, freak, robot, weak crypto, insecure cipher, expired cert, self-signed

## Best Practices

1. Always verify certificate chains completely
2. Check for certificate pinning implementations
3. Test for TLS 1.3 support (latest secure version)
4. Validate that weak ciphers are disabled
5. Ensure forward secrecy is configured