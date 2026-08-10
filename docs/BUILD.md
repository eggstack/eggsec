# Build Features and System Dependencies

## System Dependencies

| Feature | Required Packages | Install (Ubuntu/Debian) |
|---------|-------------------|--------------------------|
| `packet-inspection` | `libpcap-dev` | `sudo apt-get install libpcap-dev` |
| `wireless` | `wireless-tools` | `sudo apt-get install wireless-tools` (provides `iwlist` scanner) |
| `nse` | `libssl-dev` | `sudo apt-get install libssl-dev` |
| `nse-ssh2` | `libssh2-dev` | `sudo apt-get install libssh2-dev` |
| `grpc-api` | `protobuf-compiler` | `sudo apt-get install protobuf-compiler` (protoc for reflection descriptor; Rust proto code is checked-in) |

```bash
# Ubuntu/Debian (all features)
sudo apt-get install libpcap-dev libssl-dev wireless-tools libssh2-dev protobuf-compiler

# Fedora/RHEL
sudo dnf install libpcap-devel openssl-devel wireless-tools libssh2-devel protobuf-compiler
```

## Build Features

| Feature | Description | Status |
|---------|-------------|--------|
| `stress-testing` | SYN/UDP/ICMP floods, proxy management, IP spoofing | Lab-only |
| `packet-inspection` | Live packet capture, traceroute | Experimental |
| `nse` | Nmap NSE script compatibility | Experimental |
| `nse-ssh2` | NSE with SSH2/libssh2 support | Experimental |
| `nse-sandbox` | Restrict dangerous Lua operations | Experimental |
| `api-schema` | OpenAPI v3 schema-based fuzzing | Stable |
| `sbom` | SBOM generation (CycloneDX, SPDX) | Stable |
| `rest-api` | REST API server for agent integration | Experimental |
| `grpc-api` | gRPC API server | Experimental |
| `ws-api` | WebSocket pub/sub | Experimental |
| `ai-integration` | AI planner, script generation, autonomous agent | Experimental |
| `websocket` | WebSocket security testing | Stable |
| `headless-browser` | DOM XSS and SPA crawling | Stable |
| `web-proxy` | MITM proxy for HTTP/HTTPS traffic interception in authorized lab environments | Stable |
| `web-proxy-mcp` | MCP tool exposure for web proxy (12 tools). Requires `web-proxy`. | Stable |
| `database` | SQLx-based PostgreSQL persistence | Stable |
| `container` | Kubernetes/Docker security scanning | Stable |
| `mobile` | Mobile app static analysis (APK/IPA) | Stable |
| `mobile-dynamic` | Mobile dynamic testing (ADB + Frida). Requires `mobile`. | Stable |
| `cloud` | AWS/GCP/Azure asset discovery | Marker (planned) |
| `git-secrets` | Git secrets scanning | Marker (planned) |
| `wireless` | WiFi passive recon and security analysis | Stable |
| `wireless-advanced` | Wireless active attacks (deauth, disassoc). Requires `wireless`. | Stable |
| `evasion` | Evasion technique detection (MITRE ATT&CK mapped) | Stable |
| `postex` | Post-exploitation and LOTL simulation | Stable |
| `c2` | C2 framework for purple teaming. Depends on postex + evasion. | Stable |
| `c2-mcp` | MCP tool exposure for C2. Requires `c2`. | Marker (planned) |
| `db-pentest-mssql-tiberius` | MSSQL driver for db-pentest. Requires `db-pentest`. | Stable |
| `db-pentest-mongodb` | MongoDB driver for db-pentest. Requires `db-pentest`. | Stable |
| `db-pentest-redis` | Redis driver for db-pentest. Requires `db-pentest`. | Stable |
| `db-pentest-mcp` | MCP tool exposure for db-pentest. Requires `db-pentest`. | Marker (planned) |
| `transparent-proxy` | Transparent proxy mode for web-proxy | Marker (planned) |
| `dynamic-plugins` | Dynamic plugin loading system | Marker (planned) |
| `pdf` | PDF report generation | Marker (planned) |
| `advanced-hunting` | Advanced threat hunting | Marker (planned) |
| `compliance` | Compliance scanning (OWASP, PCI, HIPAA, SOC2) | Marker (planned) |
| `external-integrations` | Jira, GitHub, GitLab connectors | Marker (planned) |
| `finding-workflow` | Finding lifecycle management | Marker (planned) |
| `vuln-management` | Vulnerability triage and CVSS scoring | Marker (planned) |
| `full` | Most non-default features combined (excludes several markers and experimental features) | — |

### CLI-Level Features

These are on the `eggsec-cli` crate:

| Feature | Description | Default |
|---------|-------------|---------|
| `tui` | Terminal UI adapter (`eggsec-tui`) | Yes |
| `daemon-client` | Daemon client CLI commands (`eggsec-daemon` client library) | No |
| `headless` | Marker for headless/CI builds (no TUI, no daemon client) | No |

## Build Examples

```bash
# Default build - load testing, scanning, fuzzing, WAF testing
cargo build --release -p eggsec-cli

# With stress testing (controlled flood testing, proxy pool)
cargo build --release -p eggsec-cli --features stress-testing

# With packet inspection (live capture)
cargo build --release -p eggsec-cli --features packet-inspection

# With NSE support
cargo build --release -p eggsec-cli --features nse

# With mobile static analysis
cargo build --release -p eggsec-cli --features mobile

# With wireless passive recon
cargo build --release -p eggsec-cli --features wireless

# With wireless active attacks (requires wireless feature)
cargo build --release -p eggsec-cli --features wireless-advanced

# With evasion detection
cargo build --release -p eggsec-cli --features evasion

# With post-exploitation simulation
cargo build --release -p eggsec-cli --features postex

# With C2 framework (depends on postex + evasion)
cargo build --release -p eggsec-cli --features c2

# With web proxy MCP tools (requires web-proxy)
cargo build --release -p eggsec-cli --features web-proxy-mcp

# Full build - all features
cargo build --release -p eggsec-cli --features full

# Headless build - no TUI, no daemon client (CI/scripting)
cargo build --release -p eggsec-cli --no-default-features

# Daemon client build - CLI commands without TUI
cargo install --path crates/eggsec-cli --no-default-features --features daemon-client

# Daemon with HTTP/SSE transport
cargo build --release -p eggsec-daemon --features http-api
```

## Installing to PATH

```bash
# Install to ~/.cargo/bin/eggsec
cargo install --path crates/eggsec-cli

# With all features
cargo install --path crates/eggsec-cli --features full

# Verify
eggsec --version
```

Note: `cargo install` must use `--path crates/eggsec-cli` because the workspace root is a virtual manifest.
