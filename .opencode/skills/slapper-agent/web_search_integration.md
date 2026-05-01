---
name: web_search_integration
description: "Unified web search for vulnerability research using SearXNG, OSV.dev, NVD, and security sources"
triggers:
  - search
  - web search
  - searxng
  - cve
  - cve lookup
  - vulnerability
  - osv
  - nvd
  - exploitdb
  - exploit
  - github advisories
  - threat intel
metadata:
  category: search
  tools: [search]
  scope: targets
---

## Overview

The search tool provides unified access to multiple security research sources. It enables vulnerability research, exploit discovery, and threat intelligence gathering through a single interface.

## Capabilities

- **SearXNG**: Meta-search engine aggregating Google, Bing, DuckDuckGo
- **OSV.dev**: Google Vulnerability Database with package-specific advisories
- **NVD**: National Vulnerability Database with CVEs and CVSS scores
- **GitHub Advisories**: Security advisories from GitHub Repos
- **ExploitDB**: Public exploit database integration
- **CVE Lookup**: Cross-referenced vulnerability information

## Usage

### Web Search via SearXNG

```bash
slapper search --query "example.com vulnerability 2024"
slapper search --query "apache struts exploit" --source searxng
```

### CVE Lookup via OSV.dev

```bash
slapper search --query "CVE-2024-1234" --source osv
slapper search --query "RUST: GHSA-xxxx-xxxx" --source osv
```

### NVD Vulnerability Search

```bash
slapper search --query "buffer overflow" --source nvd
slapper search --query "CVE-2024" --source nvd
```

### Multi-Source Search

```bash
slapper search --query "log4j" --source all
```

## Output Format

```json
{
  "results": [
    {
      "title": "CVE-2024-1234: Buffer Overflow in X",
      "url": "https://nvd.nist.gov/vuln/detail/CVE-2024-1234",
      "source": "nvd",
      "cvss_score": 9.8,
      "description": "A buffer overflow vulnerability..."
    }
  ]
}
```

## Triggers

Keywords: search, web search, searxng, cve, vulnerability, osv, nvd, exploit, github advisory, threat, intel, research, look up, find exploit

## Best Practices

1. Start with OSV.dev for package-specific vulnerabilities
2. Use NVD for comprehensive CVE information
3. Cross-reference findings from multiple sources
4. Check ExploitDB for public exploits
5. Use SearXNG for discovering recent vulnerability disclosures

## Configuration

```toml
[search]
searxng_url = "http://localhost:8888"
cache_ttl_seconds = 3600
timeout = 30
```