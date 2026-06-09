---
name: log_injection_testing
description: "Log injection and log falsification attack testing"
triggers:
  - log injection
  - log falsification
  - newline injection
  - \n injection
  - crlf injection
  - log forging
  - log tampering
  - audit trail
metadata:
  category: fuzzing
  tools: [fuzzer]
  scope: targets
---

## Overview

Log injection attacks exploit applications that incorporate user input into log files without proper sanitization. Attackers can inject fake log entries by using newline (`\n`) and carriage return (`\r`) characters, potentially:
- Evading audit trails
- Corrupting log analysis
- Implicating other users in malicious activity
- Bypassing security monitoring

## Capabilities

- Newline injection testing (`\n`, `\r`)
- CRLF injection detection
- Log forgery detection
- Structured log parsing bypass
- Multi-line log entry injection

## Payloads

**Newline Injection:**
```
test\n[2026-04-14 INFO] Auth succeeded for admin
```

**CRLF Injection:**
```
test\r\n[2026-04-14 INFO] Auth succeeded for admin
```

**Tab for Alignment Bypass:**
```
test\t\t\t[2026-04-14 INFO] Auth succeeded for admin
```

**Real World Simulation:**
```
\n[ALERT] Security bypass attempted by user: admin
```

## Usage

### Test Log Generation

```bash
eggsec fuzz --target https://example.com/api/trace --type log-injection --param user
```

### Test User-Agent Logging

```bash
eggsec fuzz --target https://example.com/api/log --type log-injection --header User-Agent
```

### Custom Log Payloads

```bash
eggsec fuzz --target https://example.com/debug --payloads ./log_injection.txt
```

## Triggers

Keywords: log injection, log falsification, newline, crlf, \\n, \\r, log forging, log tampering, audit, \t, tab, injection, forge

## Best Practices

1. **Test inputs that appear in logs**: username, User-Agent, Referer, API keys
2. **Check log parsers**: Some apps parse structured logs (JSON, syslog)
3. **Verify sanitization**: Input should strip `\n`, `\r`, or escape them
4. **Look for timestamp injection**: Can attacker control timestamps in logs?
5. **Check log rotation**: Do injected entries survive log rotation?

## Secure Logging Patterns

**Insecure (vulnerable):**
```rust
writeln!(log_file, "User: {}", user_input);  // Allows injection
```

**Secure (fixed):**
```rust
fn sanitize_for_logging(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() && c != '\t' { '?' } else { c })
        .collect()
}
writeln!(log_file, "User: {}", sanitize_for_logging(user_input));
```

## References

- CWE-117: Improper Output Neutralization for Logs
- OWASP: Log Injection
- CWE-93: CRLF Injection
