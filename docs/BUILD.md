# Build Features and System Dependencies

## Native Dependency Inventory

Each native or external runtime requirement is documented with its owning
feature/artifact, platform scope, build vs runtime requirement, fallback
behavior, and reason for retention.

| Dependency | Owning Feature/Artifact | Platforms | Build-time | Runtime | Fallback | Reason for Retention |
|-----------|------------------------|-----------|------------|---------|----------|---------------------|
| `libpcap` | `packet-inspection` | Linux, macOS | Yes (`libpcap-dev`) | Yes | Compilation fails without dev package | Live packet capture requires pcap bindings |
| `wireless-tools` | `wireless` | Linux | No | Yes (`iwlist`) | WiFi scan commands fail | WiFi scanning needs `iwlist` scanner |
| `libssl-dev` | `nse` | All | Yes | No | NSE TLS scripts fail at runtime | OpenSSL needed for NSE TLS protocol scripts |
| `libssh2-dev` | `nse-ssh2` | All | Yes | No | NSE SSH scripts fail at compile time | libssh2 needed for NSE SSH2 support |
| `protobuf-compiler` | `grpc-api` | All | Yes (descriptor only) | No | tonic-reflection descriptor not generated | protoc generates reflection descriptor set; Rust code is checked-in |
| `ring` | `rustls` (via tokio-rustls) | All | Yes (Rust) | No | TLS compilation fails | Rust TLS library; pure Rust with assembly |
| `aws-lc-rs` | (not used) | — | — | — | — | Explicitly excluded; ring-only TLS policy |

### Build System Notes

- **libpcap**: `pnet` and `pnet_packet` require `libpcap-dev` at build time. Feature-gated behind `packet-inspection`.
- **OpenSSL**: `native-tls` with OpenSSL backend required for NSE TLS scripts. Optional behind `nse` feature. The `openssl` crate with `vendored` feature is used, which bundles OpenSSL source — but still requires a C compiler and Perl at build time.
- **libssh2**: Required for `ssh2` crate. Optional behind `nse-ssh2` feature. System library required.
- **protoc**: Only needed when `grpc-api` feature is enabled, and only for generating the tonic-reflection descriptor set (`tool_descriptor.bin`). The Rust proto code is checked in at `crates/eggsec/src/generated/eggsec.tool.v1.rs` and compiled via `include!()` — no protoc needed for ordinary builds.

### Retained protoc requirement justification (acceptance criterion 12)

The `grpc-api` feature retains a `protoc` build dependency because tonic-reflection
requires a binary file descriptor set (`tool_descriptor.bin`) that cannot be
checked in as Rust source. The descriptor is a protobuf-encoded binary that
tonic-reflection loads at runtime to serve gRPC server reflection. Generating
it from `.proto` source is the only supported path. This is a build-time-only
requirement — it does not affect CI compilation of unchecked-in code, and the
Rust proto code is checked in separately. The descriptor is regenerated only
when the proto schema changes (a maintainer task).

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
