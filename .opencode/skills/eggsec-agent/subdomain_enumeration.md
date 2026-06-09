---
name: subdomain_enumeration
description: "Comprehensive subdomain discovery using multiple techniques and sources"
triggers:
  - subdomain
  - subdomains
  - subomain enumeration
  - virtual host
  - vhost
  - domain discovery
metadata:
  category: reconnaissance
  tools: [recon]
  scope: targets
---

## Overview

Subdomain enumeration reveals hidden attack surfaces that may be less protected than main domains. This skill uses multiple techniques to discover subdomains through DNS, search engines, certificate logs, and wordlists.

## Capabilities

- DNS bruteforce with wordlists
- Permutation and alteration of known subdomains
- Search engine scraping (via SearXNG)
- Certificate Transparency log analysis
- DNS zone transfer attempts
- DNSSEC zone walking
- Virtual host discovery on known IPs
- Resolution with multiple DNS servers
- Integration with crt.sh and similar services

## Usage

### Basic Subdomain Enumeration

```bash
eggsec recon subdomains --target example.com
```

### With Custom Wordlist

```bash
eggsec recon subdomains --target example.com --wordlist /path/to/subdomains.txt
```

### Using Certificate Logs

```bash
eggsec recon subdomains --target example.com --crt-search
```

### Permutation Mode

```bash
eggsec recon subdomains --target example.com --permutate
```

### Find Virtual Hosts

```bash
eggsec recon subdomains --target 192.168.1.1 --vhost
```

## Wordlist Sources

Default wordlists include common prefixes:
- www, mail, ftp, localhost, webmail, smtp
- pop, ns1, webdisk, ns2, cdn
- csrf, sandbox, api, dev, www2

## Configuration

```toml
[recon.subdomains]
wordlist = "~/.config/eggsec/wordlists/subdomains.txt"
concurrent = 100
timeout = 5
```

## Triggers

Keywords: subdomain, subdomains, virtual host, vhost, enumerate, bruteforce, dns, wordlist, permutation, crt.sh, certspotter, discovery

## Best Practices

1. Always start with certificate transparency logs (passive)
2. Use permutation for variations of found subdomains
3. Validate all discovered subdomains are actually reachable
4. Look for abandoned subdomains (dangling DNS)
5. Correlate findings with port scan results