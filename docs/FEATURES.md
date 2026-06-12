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
| `stress-testing` | DoS testing tools, proxy management, ICMP, IP spoofing | pnet, pnet_packet, socket2, nix, libc, surge-ping |
| `packet-inspection` | Live packet capture, advanced packet tools | pnet, pnet_packet, libc |
| `nse` | Nmap Scripting Engine support - run Lua NSE scripts | tool-api, eggsec-nse |
| `nse-sandbox` | NSE sandbox mode - restrict dangerous Lua operations | nse, eggsec-nse/sandbox |
| `mobile` | Mobile app static analysis (APK/IPA manifest & config checks for authorized lab/defense use only). Dynamic mobile (Android ADB + logcat + Phase 2a proxy + traffic-capture + runtime-permission operations + correlation) shipped under `mobile-dynamic`; future phases per `plans/dynamic-mobile-testing-loadout-design-plan.md`. | zip, plist (optional under feature) |
| `full` | All features combined (excludes `grpc-api`, `ws-api`, `pdf`) | stress-testing, packet-inspection, rest-api, nse, ai-integration, websocket, headless-browser, database, container, sbom, advanced-hunting, compliance, external-integrations, finding-workflow, vuln-management, wireless, mobile |

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

Includes all features:
- Core functionality
- Stress testing tools
- Packet inspection
- REST API server
- NSE (Nmap Scripting Engine) support
- Mobile static analysis (APK/IPA manifest & config checks)

### With Mobile Static Analysis

```bash
cargo build --release --features mobile
```

Adds:
- `eggsec mobile <path.{apk,ipa}>` - Static security analysis of Android APKs and iOS IPAs (authorized lab/defense use only).
- Coverage: manifest attributes, permissions (normal/dangerous/signature), transport security (cleartext/ATS), secrets in assets, debug/backup/exported flags, signing/provisioning notes, custom URL schemes.
- Phase 1: static-only (no execution, no device interaction). Pure-Rust ZIP/plist + bounded AXML extraction. Requires `--features mobile` (or `--features full`, which includes it). See `crates/eggsec/src/mobile/{mod,apk,ipa}.rs` and `docs/CAPABILITIES.md` (Mobile App Security section). Dynamic mobile (Phase 1 + Phase 2a + final polish + close-out polish, complete 2026-06-12) is shipped under `--features mobile-dynamic`; future phases per `plans/dynamic-mobile-testing-loadout-design-plan.md`.

## Feature Hierarchy

```
full
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
│       └── sandbox (optional)
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
└── wireless

grpc-api (standalone, NOT in full)
└── tool-api
    ├── tonic
    └── prost, prost-build

ws-api (standalone, NOT in full)

pdf (standalone, NOT in full)
```

## API Server Features

### REST API

```bash
cargo build --release --features rest-api
```

Adds:
- REST API server with Axum
- MCP (Model Context Protocol) endpoints for AI agent integration
- SSE streaming support
- Health check and metrics endpoints

### gRPC API

```bash
cargo build --release --features grpc-api
```

Adds:
- gRPC API server with Tonic
- Protocol Buffers message definitions
- Bidirectional streaming support

## NSE (Nmap Scripting Engine)

### Basic NSE Support

```bash
cargo build --release --features nse
```

Adds:
- Run Lua NSE scripts
- NSE script loading and execution
- Integration with Eggsec's scanning pipeline

**Note:** `eggsec-nse` uses `native-tls` (OpenSSL) for TLS support. This is intentional — Nmap NSE scripts expect OpenSSL-based TLS behavior. Do not migrate to `rustls`.

### NSE Sandbox Mode

```bash
cargo build --release --features nse-sandbox
```

Adds:
- Restricted Lua environment for untrusted NSE scripts
- Blocks dangerous operations: `io.popen`, `os.setenv`, filesystem access
- Safe subset of NSE libraries

## Build Time Impact

| Feature | Approx. Compile Time Impact | Binary Size Impact |
|---------|---------------------------|-------------------|
| `tool-api` | Minimal | Minimal |
| `rest-api` | Low (axum + tower) | Medium |
| `grpc-api` | Medium (tonic + prost) | Medium |
| `stress-testing` | Medium (pnet + nix) | Medium |
| `packet-inspection` | Low (pnet) | Low |
| `nse` | Medium (mlua + Lua) | Medium |
| `full` | High (all combined) | High |

## Verifying Enabled Features

To see which features are enabled in your current build:

```bash
./eggsec --version
```

To check available commands:

```bash
eggsec --help
```

If a command is not available, rebuild with the required feature flag.

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
| `mobile` | `mobile` |
| `vuln` | `vuln-management` |
| `storage` | `database` |
| `config` | Default |
| `doctor` | Default |
| `report` | Default |
