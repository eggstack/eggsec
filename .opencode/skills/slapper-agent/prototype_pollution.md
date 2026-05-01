---
name: prototype_pollution
description: "JavaScript prototype pollution detection and testing"
triggers:
  - prototype pollution
  - js pollution
  - javascript injection
  - __proto__
  - constructor prototype
metadata:
  category: fuzzing
  tools: [fuzzer]
  scope: web_targets
---

## Overview

Slapper supports prototype pollution testing for JavaScript applications. Prototype pollution vulnerabilities allow attackers to inject properties into existing objects, potentially leading to RCE or data manipulation.

## Payload Categories

| Category | Count | Description |
|----------|-------|-------------|
| JS Pollution | 10+ | Direct `__proto__` injection |
| Value Pollution | 5+ | `constructor` property injection |
| Merge Pollution | 5+ | Deep merge exploitation |
| Bypass | 4+ | Path encoding, null bytes |
| Node-specific | 6+ | require(), process access |

## Usage

```bash
# Basic prototype pollution scan
slapper fuzzer --target http://target.com/api --payload-type prototype

# SPA target
slapper fuzzer --target http://target.com/ --payload-type prototype
```

## Injection Techniques

### Basic Pollution
```javascript
{"__proto__": {"polluted": "yes"}}
{"constructor": {"prototype": {"polluted": "yes"}}}
```

### Merge Exploitation
```javascript
{"__proto__": {}}
{"a": "123", "__proto__": {"polluted": "true"}}
```

### Node.js Specific
```javascript
{"__proto__": {"exports": {"id": "malicious"}}}
{"constructor": {"prototype": {"require": "global"}}}
```

## Detection Methods

1. **Property Injection** - Check if injected properties appear in new objects
2. **Deep Merge** - Target recursive merge functions
3. **Function Override** - Exploit callback replacements

## Triggers

Keywords: prototype pollution, js pollution, javascript injection, __proto__, constructor prototype, node.js security

## References

- `fuzzer/payloads/prototype.rs` - Payload implementations
- `PayloadType::Prototype` - Integration point
