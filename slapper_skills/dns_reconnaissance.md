---
name: dns_reconnaissance
description: "DNS reconnaissance for discovering domain infrastructure, subdomains, and DNS records"
triggers:
  - dns
  - dns lookup
  - domain enumeration
  - subdomain discovery
  - nameserver
  - mx lookup
  - txt records
  - zone transfer
metadata:
  category: reconnaissance
  tools: [recon]
  scope: targets
---

## Overview

DNS reconnaissance is a critical first step in security assessments. It reveals the attack surface by enumerating domains, subdomains, and infrastructure components.

## Capabilities

- DNS record lookup (A, AAAA, MX, TXT, NS, CNAME, PTR)
- Subdomain enumeration via wordlist and dictionary attacks
- NSEC/NSEC3 walking for zone enumeration
- DNSSEC validation checking
- CAA/CSP record discovery
- Reverse DNS lookup for IP ranges
- SRV record enumeration

## Usage

### Basic DNS Lookup

```bash
slapper recon --target example.com --dns
```

### Subdomain Discovery

```bash
slapper recon subdomains --target example.com
slapper recon subdomains --target example.com --wordlist /path/to/wordlist.txt
```

### Full DNS Enumeration

```bash
slapper recon --target example.com --all
```

### Zone Transfer Testing

```bash
slapper recon dns --target example.com --zone-transfer
```

### Reverse DNS Lookup

```bash
slapper recon dns --target 192.168.1.0/24 --reverse-lookup
```

## Configuration

DNS recon can be configured in `config.toml`:

```toml
[recon]
dns_timeout = 10
dns_concurrency = 50
dns_wordlist = "/path/to/subdomains.txt"
```

## Output

Results include:
- Discovered subdomains with IP addresses
- DNS record types and values
- Name server information
- Mail server configurations
- SPF/DKIM/DMARC records for email security

## Triggers

Keywords that activate this skill: dns, lookup, enumerate, subdomain, zone, nameserver, mx, txt, a record, aaaa record, cname, ptr, srv, recon, reconnaissance

## Best Practices

1. Start with passive DNS before active queries
2. Use multiple wordlists for comprehensive subdomain enumeration
3. Check for dangling DNS entries that may reveal infrastructure
4. Verify DNSSEC signatures when present
5. Document all findings for later correlation