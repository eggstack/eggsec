# Eggsec Build Features

This document explains the available build configurations and feature flags for Eggsec.

## Feature Flags Overview

Eggsec uses Cargo feature flags to enable optional capabilities. This allows users to build a minimal binary or include all features depending on their needs.

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `default` | Core functionality: load testing, port scanning, fuzzing, WAF testing | None |
| `tool-api` | Tool abstraction layer (SecurityTool trait, ToolRegistry) | None |
| `rest-api` | REST API server with MCP (Model Context Protocol) for AI agent integration | `tool-api`, axum, tower |
| `grpc-api` | gRPC API server for external tool integration | `tool-api`, tonic, prost |
| `ws-api` | WebSocket pub/sub API | axum/ws |
| `stress-testing` | DoS testing tools, proxy management, ICMP, IP spoofing | pnet, pnet_packet, socket2, nix, libc, surge-ping |
| `packet-inspection` | Live packet capture, advanced packet tools | pnet, pnet_packet, libc |
| `nse` | Nmap Scripting Engine support - run Lua NSE scripts | tool-api, eggsec-nse |
| `nse-ssh2` | NSE with SSH2/libssh2 support | nse, ssh2 |
| `nse-sandbox` | NSE sandbox mode - restrict dangerous Lua operations | nse, eggsec-nse/sandbox |
| `ai-integration` | AI/LLM integration for analysis and planning | tool-api, eventsource-stream, semver |
| `websocket` | WebSocket security testing | tokio-tungstenite |
| `headless-browser` | DOM XSS and SPA crawling | headless_chrome |
| `database` | SQLx-based persistence for findings and scan history | sqlx |
| `container` | Kubernetes/Docker scanning | kube, k8s-openapi |
| `cloud` | AWS/GCP/Azure asset discovery | None |
| `sbom` | SBOM generation (CycloneDX, SPDX) | cyclonedx-bom, spdx, walkdir |
| `git-secrets` | Git secrets scanning | None |
| `advanced-hunting` | Advanced threat hunting | None |
| `compliance` | Compliance scanning (OWASP, PCI, HIPAA, SOC2) | None |
| `external-integrations` | Jira, GitHub, GitLab connectors | None |
| `finding-workflow` | Finding lifecycle management | None |
| `vuln-management` | Vulnerability triage and CVSS scoring | None |
| `wireless` | Passive WiFi scanning and security analysis | None |
| `wireless-advanced` | Active wireless attacks (deauth/disassoc, lab-only) | `wireless` |
| `mobile` | Mobile app static analysis (APK/IPA) | zip, plist |
| `mobile-dynamic` | Mobile dynamic testing (Android ADB + Frida) | `mobile` |
| `db-pentest` | Database security assessment (Postgres/MySQL/MSSQL/MongoDB/Redis) | sqlx |
| `db-pentest-mssql-tiberius` | Real MSSQL client via tiberius | tiberius |
| `db-pentest-mongodb` | Real MongoDB client | mongodb, bson |
| `db-pentest-redis` | Real Redis client | redis |
| `db-pentest-mcp` | MCP tool exposure for db-pentest | `db-pentest` |
| `web-proxy` | Interactive web proxy (HTTP/HTTPS/WebSocket/HTTP2/gRPC) | tokio-tungstenite, h2, http, prost, prost-types |
| `web-proxy-mcp` | MCP tool exposure for web proxy (12 tools) | `web-proxy` |
| `transparent-proxy` | Transparent proxy mode (Linux iptables/nftables REDIRECT) | `web-proxy` |
| `dynamic-plugins` | Dynamic plugin loading from shared libraries (.so/.dylib) | `web-proxy` |
| `api-schema` | OpenAPI v3 schema-based fuzzing (marker-only) | None |
| `pdf` | PDF report generation | printpdf |
| `full` | All features combined (21 sub-features) | See below |

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

### Full Build (Recommended)

```bash
cargo build --release --features full
```

Includes all 21 sub-features:
- Core functionality
- Stress testing tools
- Packet inspection
- REST API server
- NSE (Nmap Scripting Engine) support
- AI integration
- WebSocket security testing
- Headless browser testing
- Database persistence
- Container scanning
- SBOM generation
- Advanced threat hunting
- Compliance scanning
- External integrations
- Finding workflow
- Vulnerability management
- Wireless scanning (passive + active)
- Mobile testing (static + dynamic)
- Database pentesting
- Web proxy interception

### With Mobile Static Analysis

```bash
cargo build --release --features mobile
```

Adds:
- `eggsec mobile <path.{apk,ipa}>` - Static security analysis of Android APKs and iOS IPAs (authorized lab/defense use only).

### With Database Pentesting

```bash
cargo build --release --features db-pentest
```

Adds:
- `eggsec db pentest <connection-string>` - Direct database security assessment (Postgres, MySQL, MSSQL, MongoDB, Redis).

### With Web Proxy

```bash
cargo build --release --features web-proxy
```

Adds:
- `eggsec proxy intercept` - Interactive MITM web proxy for HTTP/HTTPS/WebSocket/HTTP2/gRPC traffic interception.

## Feature Hierarchy

```
full (21 sub-features)
├── stress-testing
│   ├── pnet, pnet_packet
│   ├── socket2, nix, libc
│   └── surge-ping
├── packet-inspection
│   ├── pnet, pnet_packet
│   └── libc
├── rest-api
│   ├── tool-api
│   ├── axum, tower
│   └── async-stream
├── nse
│   ├── tool-api
│   └── eggsec-nse
├── ai-integration
├── websocket
├── headless-browser
├── database
├── container
├── sbom
├── advanced-hunting
├── compliance
├── external-integrations
├── finding-workflow
├── vuln-management
├── wireless
├── wireless-advanced (depends on wireless)
├── mobile
├── mobile-dynamic (depends on mobile)
├── db-pentest
└── web-proxy

grpc-api (standalone, NOT in full)
├── tool-api
├── tonic
└── prost, prost-build

ws-api (standalone, NOT in full)
pdf (standalone, NOT in full)
```

## Command Feature Requirements

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
| `config` | Default |
| `doctor` | Default |
| `stress` | `stress-testing` |
| `proxy` | `stress-testing` |
| `icmp` | `stress-testing` |
| `traceroute` | `stress-testing` |
| `packet capture` | `packet-inspection` |
| `packet send` | `packet-inspection` |
| `nse` | `nse` |
| `serve` | `rest-api` |
| `mcp-serve` | `rest-api` |
| `codegg-mcp` | `rest-api` |
| `agent` | `rest-api` |
| `ai-analyze` | `ai-integration` |
| `browser` | `headless-browser` |
| `hunt` | `advanced-hunting` |
| `compliance` | `compliance` |
| `sbom` | `sbom` |
| `grpc` | `grpc-api` |
| `wireless` | `wireless` |
| `wireless deauth/disassoc` | `wireless-advanced` |
| `mobile` | `mobile` |
| `mobile dynamic` | `mobile-dynamic` |
| `vuln` | `vuln-management` |
| `storage` | `database` |
| `proxy intercept` | `web-proxy` |
| `db pentest` | `db-pentest` |
