---
name: endpoint_discovery
description: "Web endpoint and path discovery for HTTP/HTTPS targets"
triggers:
  - endpoint
  - endpoints
  - path
  - paths
  - directory
  - directories
  - fuzz
  - dirb
  - dirbuster
  - gobuster
  - spider
  - crawl
  - web discovery
metadata:
  category: scanning
  tools: [scanner]
  scope: targets
---

## Overview

Endpoint discovery finds hidden paths, directories, and files on web servers. This reveals administrative interfaces, backup files, configuration files, and other attack surface not immediately visible.

## Capabilities

- Path bruteforcing with wordlists
- File extension enumeration
- HTTP method discovery (GET, POST, PUT, DELETE)
- Response analysis (status codes, content-length)
- Delta encoding detection (compressed/encoded responses)
- Servlet enumeration for Java applications
- API endpoint discovery
- Nested path traversal

## Usage

### Basic Endpoint Scan

```bash
eggsec scan endpoints --target https://example.com
```

### With Custom Wordlist

```bash
eggsec scan endpoints --target https://example.com --wordlist /path/to/paths.txt
```

The wordlist is a plain text file with one endpoint per line. Lines starting with `#` are treated as comments. Paths are automatically normalized to start with `/`.

### PHP Extension Scan

```bash
eggsec scan endpoints --target https://example.com --extensions php,asp,aspx,jsp
```

### API Endpoint Discovery

```bash
eggsec scan endpoints --target https://api.example.com --api
```

## Common Paths Reference

| Path | Purpose |
|------|---------|
| /admin | Administrative interfaces |
| /api | API endpoints |
| /backup | Backup files |
| /.git | Git repository |
| /config | Configuration files |
| /debug | Debug endpoints |
| /env | Environment variables |
| /login | Login pages |
| /phpmyadmin | Database admin |
| /wp-admin | WordPress admin |

## Configuration

```toml
[scan]
default_concurrency = 50
rate_limit_per_second = 100
follow_redirects = true
```

## Triggers

Keywords: endpoint, endpoints, path, paths, directory, directories, fuzz, discover, spider, crawl, bruteforce, enumerate, hidden, robots.txt, sitemap.xml

## Best Practices

1. Always check robots.txt and sitemap.xml
2. Use multiple wordlists for comprehensive coverage
3. Look for API documentation files (swagger, openapi)
4. Identify default file patterns (index.php, admin, login)
5. Test for configuration backup files (.bak, .old)