---
name: race_condition
description: "Race condition and TOCTOU (Time-of-Check-Time-of-Use) testing"
triggers:
  - race condition
  - toctou
  - concurrent testing
  - time of check
  - http concurrent
metadata:
  category: fuzzing
  tools: [fuzzer]
  scope: web_targets
---

## Overview

Eggsec supports race condition testing using concurrent request attacks. TOCTOU (Time-of-Check-Time-of-Use) vulnerabilities occur when there's a gap between when a system checks a condition and when it uses the result.

## Payload Categories

| Category | Count | Description |
|----------|-------|-------------|
| Time-of-Check | 10+ | State change between check and use |
| HTTP Concurrent | 6+ | Parallel request timing attacks |
| Race Primitives | 6+ | Basic race primitives |
| Header Races | 5+ | HTTP header timing attacks |
| Auth Races | 6+ | Authentication race conditions |

## Usage

```bash
# Basic race condition scan
eggsec fuzzer --target http://target.com/api --payload-type race

# With high concurrency
eggsec fuzzer --target http://target.com/api --payload-type race --concurrency 50

# Token reuse scenario
eggsec fuzzer --target http://target.com/api --payload-type race --payload-file race_tokens.txt
```

## Testing Techniques

### Time-of-Check Manipulation
Send requests that modify state between authentication check and resource access:
```
Request 1: GET /transfer?to=alice&amount=100  [with valid token]
Request 2: GET /transfer?to=bob&amount=1000   [token reuse]
```

### HTTP Concurrent Attacks
Issue multiple requests simultaneously to exploit:
- Session fixation
- OAuth token reuse
- CSRF token bypass
- Rate limit bypass

### Header Race Conditions
```
Request 1: X-Forwarded-For: 10.0.0.1
Request 2: X-Forwarded-For: 10.0.0.2
Request 3: X-Forwarded-For: 10.0.0.3
```

## Triggers

Keywords: race condition, toctou, concurrent testing, time of check, http concurrent, parallel requests, timing attack

## References

- `fuzzer/payloads/race.rs` - Payload implementations
- `PayloadType::Race` - Integration point
