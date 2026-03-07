# Slapper Build Features

This document explains the available build configurations and feature flags for Slapper.

## Feature Flags Overview

Slapper uses Cargo feature flags to enable optional capabilities. This allows users to build a minimal binary or include all features depending on their needs.

| Feature | Description |
|---------|-------------|
| `default` | Core functionality: load testing, port scanning, fuzzing, WAF testing |
| `stress-testing` | DoS testing tools, proxy management, ICMP |
| `packet-inspection` | Live packet capture, advanced packet tools |
| `python-plugins` | Python plugin support |
| `ruby-plugins` | Ruby plugin support + Metasploit RPC |
| `all-plugins` | Both Python and Ruby plugin support |
| `full` | All features combined |

## Available Builds

### Default Build

```bash
cargo build --release
```

Includes:
- Load testing (HTTP)
- Port scanning
- Service fingerprinting
- Endpoint discovery
- Security fuzzing (SQLi, XSS, SSRF, etc.)
- WAF detection and bypass testing
- GraphQL, OAuth/OIDC, JWT, WebSocket, gRPC testing
- Reconnaissance (DNS, WHOIS, tech detection, CVE mapping)
- Interactive TUI
- Cluster mode
- Notifications

### With Stress Testing

```bash
cargo build --release --features stress-testing
```

Adds:
- `stress` - SYN, UDP, HTTP, TCP, ICMP flood testing
- `proxy` - Proxy pool management (SOCKS4, SOCKS5, HTTP, HTTPS, Tor)
- `icmp` - ICMP ping probes
- `traceroute` - Network path tracing

**Warning**: Stress testing tools should only be used on systems you own or have explicit written permission to test.

### With Packet Inspection

```bash
cargo build --release --features packet-inspection
```

Adds:
- `packet capture` - Live packet capture from network interfaces
- `packet send` - Craft and send custom packets
- `packet hexdump` - Analyze packet captures

**Note**: Live packet capture requires root/sudo privileges.

### With Python Plugins

```bash
cargo build --release --features python-plugins
```

Adds:
- Python plugin support
- Run custom Python-based security scanners

Requires Python 3.8+ development headers.

### With Ruby Plugins

```bash
cargo build --release --features ruby-plugins
```

Adds:
- Ruby plugin support
- Metasploit Framework RPC integration
- Full Metasploit module access from Ruby plugins

Requires Ruby 3.0+ development headers and Metasploit Framework (optional, for RPC functionality).

### Full Build (Recommended for Pentesting)

```bash
cargo build --release --features full
```

Includes all features:
- Core functionality
- Stress testing tools
- Packet inspection
- Python and Ruby plugin support

## Command Feature Requirements

The following commands require specific build features:

| Command | Required Feature |
|---------|-----------------|
| `load` | Default |
| `scan-ports` | Default |
| `scan-endpoints` | Default |
| `fingerprint` | Default |
| `fuzz` | Default |
| `waf` | Default |
| `waf-stress` | Default |
| `scan` | Default |
| `recon` | Default |
| `graphql` | Default |
| `oauth` | Default |
| `packet dump` | Default |
| `packet traceroute` | Default |
| `packet interfaces` | Default |
| `cluster` | Default |
| `notify` | Default |
| `resume` | Default |
| `report` | Default |
| `remote` | Default |
| `exec` | Default |
| `stress` | `stress-testing` |
| `proxy` | `stress-testing` |
| `icmp` | `stress-testing` |
| `traceroute` | `stress-testing` |
| `packet capture` | `packet-inspection` |
| `packet send` | `packet-inspection` |
| `plugin` | `python-plugins` or `ruby-plugins` |

## WAF Detection Support

Slapper detects 26 WAF products and supports targeted bypass profiles:

| WAF | Detection Method | Bypass Profile |
|-----|------------------|----------------|
| Cloudflare | Headers, cookies, IP ranges | Yes |
| Akamai | Headers, IP ranges | Yes |
| AWS WAF | Headers, IP ranges | Yes |
| Azure WAF | Headers, IP ranges | Yes |
| Google Cloud Armor | Headers, IP ranges | No |
| Fastly | Headers, IP ranges | No |
| Imperva | Headers, cookies, IP ranges | Yes |
| Sucuri | Headers, IP ranges | No |
| CloudFront | Headers, IP ranges | Yes |
| F5 BIG-IP | Headers, cookies | No |
| Barracuda | Headers, cookies | No |
| Fortinet | Headers | No |
| Citrix NetScaler | Headers, cookies | No |
| ModSecurity | Body patterns | No |
| Wordfence | Headers, cookies | No |
| DataDome | Headers, cookies, IP ranges | No |
| PerimeterX | Headers, cookies | No |
| Nginx | Headers, body patterns | No |
| Traefik | Headers | No |
| Kong | Headers | No |
| Varnish | Headers | No |
| Radware | Headers | No |
| Signal Sciences | Headers, IP ranges | No |
| Wallarm | Headers, IP ranges | No |
| Reblaze | Headers, IP ranges | No |

### WAF Bypass Profiles

Use `--profile` flag with `waf` command for targeted bypass:

```bash
slapper waf https://example.com --profile cloudflare --bypass
slapper waf https://example.com --profile akamai --bypass
slapper waf https://example.com --profile imperva --bypass
slapper waf https://example.com --profile aws-waf --bypass
slapper waf https://example.com --profile azure-waf --bypass
```

Available profiles: `cloudflare`, `akamai`, `aws-waf`, `azure-waf`, `imperva`, `f5-asm`, `cloudfront`, `sucuri`, `auto`

## Verifying Available Features

To see which features are enabled in your current build:

```bash
./slapper --version
```

To check available commands:

```bash
slapper --help
```

If a command is not available, rebuild with the required feature flag.
