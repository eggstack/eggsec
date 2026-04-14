---
name: port_scanning
description: "Network port scanning and service fingerprinting for network discovery"
triggers:
  - port scan
  - ports
  - scanning
  - nmap
  - service detection
  - version detection
  - syn scan
  - tcp scan
  - network discovery
metadata:
  category: scanning
  tools: [scanner]
  scope: targets
---

## Overview

Port scanning reveals exposed network services and their versions, providing critical information for attack surface assessment. Slapper supports various scan types including SYN, TCP connect, and spoofed scans.

## Capabilities

- TCP port scanning (full range 1-65535)
- SYN scan (requires root, feature-gated)
- UDP port scanning
- Service version detection
- OS fingerprinting
- IP spoofing for stealth scanning
- Decoy scan with multiple fake sources
- Idle/Zombie scan for anonymity
- Custom port ranges
- Concurrent scanning

## Usage

### Basic Port Scan

```bash
slapper scan ports --target 192.168.1.1
```

### Full Range Scan

```bash
slapper scan ports --target 192.168.1.1 --ports 1-65535
```

### Service Detection

```bash
slapper scan ports --target 192.168.1.1 --service-detect
```

### With IP Spoofing

```bash
slapper scan ports --target 192.168.1.1 --spoof-ip 10.0.0.1
```

### Custom Port List

```bash
slapper scan ports --target 192.168.1.1 --ports 22,80,443,8080,8443
```

### Fast Scan (Top Ports)

```bash
slapper scan ports --target 192.168.1.1 --top-ports 100
```

## Common Port Reference

| Port | Service | Risk |
|------|---------|------|
| 21 | FTP | High (cleartext) |
| 22 | SSH | Low (but brute force) |
| 23 | Telnet | Critical (cleartext) |
| 25 | SMTP | Medium |
| 53 | DNS | Medium |
| 80 | HTTP | High |
| 443 | HTTPS | Medium |
| 445 | SMB | Critical |
| 3306 | MySQL | High |
| 5432 | PostgreSQL | High |
| 6379 | Redis | Critical |
| 8080 | HTTP-Alt | High |
| 27017 | MongoDB | High |

## Configuration

```toml
[scan]
timeout = 1000
concurrency = 100
max_retries = 3
```

## Triggers

Keywords: port, ports, scan, scanning, syn, tcp, udp, network, service, version, fingerprint, detect, discover, open port, closed port, filtered

## Best Practices

1. Start with top ports to quickly identify exposed services
2. Use SYN scan when elevated privileges are available
3. Check for filtered ports that may indicate firewall rules
4. Correlate findings with endpoint discovery
5. Document all services found for attack planning