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

DNS reconnaissance runs inside the single `recon` command (positional target; no subcommands):

### Full Recon Pipeline (includes DNS records + subdomain enumeration)

```bash
eggsec recon example.com
```

### Skip Non-DNS Stages

```bash
# DNS-focused run: keep dns/subdomains, skip the rest
eggsec recon example.com \
  --no-tech --no-geo --no-whois --no-ssl --no-js \
  --no-content --no-cloud --no-wayback --no-cors \
  --no-threat --no-cve --no-email-security
```

### Custom Subdomain Wordlist

Subdomain wordlists are configured via `[recon]` settings or engine defaults, not a per-run CLI flag.

### Zone Transfer Testing

```bash
# Zone transfer checks run inside the full recon pipeline
eggsec recon example.com
```

### Reverse DNS Lookup

```bash
# Reverse DNS runs inside the full recon pipeline
eggsec recon 192.168.1.0/24
```

## Configuration

DNS recon concurrency is configured in `config.toml` (API keys for passive sources live under `[recon.apis]`):

```toml
[recon]
dns_concurrency = 50
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