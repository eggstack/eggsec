---
name: nosql_injection
description: "NoSQL injection testing for MongoDB, Redis, CouchDB, and Elasticsearch"
triggers:
  - nosql injection
  - mongodb injection
  - redis injection
  - couchdb injection
  - elasticsearch injection
  - nosql fuzzing
metadata:
  category: fuzzing
  tools: [fuzzer]
  scope: web_targets
---

## Overview

Eggsec supports NoSQL injection testing for MongoDB, Redis, CouchDB, and Elasticsearch databases. NoSQL injection exploits vulnerabilities in applications that build queries using user input without proper sanitization.

## Supported Databases

| Database | Payloads | Notes |
|----------|----------|-------|
| MongoDB | 28+ | Standard operators, $where, $function |
| Redis | 10+ | Command injection, CONFIG, KEYS |
| CouchDB | 5+ | View injection, _all_docs |
| Elasticsearch | 5+ | Query DSL injection |
| Bypass | 6+ | Unicode, casing, operators |

## Usage

```bash
# Basic NoSQL injection scan
eggsec fuzzer --target http://target.com/api --payload-type nosql

# With custom wordlist
eggsec fuzzer --target http://target.com/api --payload-type nosql --wordlist custom_nosql.txt

# Enable detection only (no exploitation)
eggsec fuzzer --target http://target.com/api --payload-type nosql --detect-only
```

## Payload Categories

### MongoDB Operators
- `$where` clauses
- `$ne` (not equal)
- `$gt` / `$lt` (comparison)
- `$regex` (regex injection)
- `$exists` (presence check)
- `$size` (array size)
- `$all` (array matching)
- `$in` / `$nin` (set matching)

### Redis Commands
- `CONFIG GET/SET`
- `KEYS` pattern matching
- `FLUSHDB`
- `EVAL` Lua scripting
- `MULTI/EXEC`

### Bypass Techniques
- Unicode normalization
- Case variation
- Operator encoding
- JSON wrapping

## Triggers

Keywords: nosql injection, mongodb, redis, couchdb, elasticsearch, nosql fuzzing, database injection, document database

## References

- `fuzzer/payloads/nosql.rs` - Payload implementations
- `PayloadType::Nosql` - Integration point
