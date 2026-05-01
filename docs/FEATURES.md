# Slapper Build Features

This document explains the available build configurations and feature flags for Slapper.

## Feature Flags Overview

Slapper uses Cargo feature flags to enable optional capabilities. This allows users to build a minimal binary or include all features depending on their needs.

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `default` | Core functionality: load testing, port scanning, fuzzing, WAF testing | None |
| `tool-api` | Tool abstraction layer (SecurityTool trait, ToolRegistry) | None |
| `rest-api` | REST API server with MCP (Model Context Protocol) for AI agent integration | `tool-api`, axum, tower |
| `grpc-api` | gRPC API server for external tool integration | `tool-api`, tonic, prost |
| `stress-testing` | DoS testing tools, proxy management, ICMP, IP spoofing | pnet, pnet_packet, socket2, nix, libc, surge-ping |
| `packet-inspection` | Live packet capture, advanced packet tools | pnet, pnet_packet, libc |
| `python-plugins` | Python plugin support (PyO3) | slapper-plugin (pyo3, dirs) |
| `ruby-plugins` | Ruby plugin support + Metasploit RPC | slapper-plugin (magnus), slapper-ruby |
| `all-plugins` | Both Python and Ruby plugin support | python-plugins, ruby-plugins |
| `nse` | Nmap Scripting Engine support - run Lua NSE scripts | tool-api, slapper-nse |
| `nse-sandbox` | NSE sandbox mode - restrict dangerous Lua operations | nse, slapper-nse/sandbox |
| `full` | All features combined | python-plugins, ruby-plugins, stress-testing, packet-inspection, rest-api, nse |

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
- REST API server
- NSE (Nmap Scripting Engine) support

## Feature Hierarchy

```
full
├── python-plugins
│   └── slapper-plugin (pyo3)
├── ruby-plugins
│   ├── slapper-plugin (magnus)
│   └── slapper-ruby
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
└── nse
    ├── tool-api
    └── slapper-nse
        └── sandbox (optional)

grpc-api (standalone)
└── tool-api
    ├── tonic
    └── prost, prost-build
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
- Integration with Slapper's scanning pipeline

**Note:** `slapper-nse` uses `native-tls` (OpenSSL) for TLS support. This is intentional — Nmap NSE scripts expect OpenSSL-based TLS behavior. Do not migrate to `rustls`.

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
| `python-plugins` | Medium (pyo3 + Python) | Medium |
| `ruby-plugins` | High (magnus + Ruby) | High |
| `nse` | Medium (mlua + Lua) | Medium |
| `full` | High (all combined) | High |

## Verifying Enabled Features

To see which features are enabled in your current build:

```bash
./slapper --version
```

To check available commands:

```bash
slapper --help
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
| `plugin` | `python-plugins` or `ruby-plugins` |
| `nse` | `nse` |
