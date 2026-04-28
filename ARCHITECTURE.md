# Slapper Architecture

## Workspace Structure

```
slapper/                         # Workspace root
├── Cargo.toml                   # Workspace config, release profile (LTO, opt-level 3)
├── crates/
│   ├── slapper/                 # Main binary crate (CLI + all core logic)
│   ├── slapper-plugin/          # Python plugin support (PyO3)
│   ├── slapper-nse/             # Nmap Scripting Engine compat layer (mlua)
│   └── slapper-ruby/            # Ruby plugin support + Metasploit RPC (magnus)
├── plugins/                     # Example plugins
├── examples/                    # Chain files, configs, payloads
├── wordlists/                   # Default wordlists for scanning
└── templates/                   # Report templates
```

## Module Map (crates/slapper/src/)

| Module | Purpose |
|--------|---------|
| `cli/` | Clap-based CLI argument parsing, defines `Commands` enum and per-command arg structs |
| `commands/` | Command dispatch (`handle_command()`), per-command handlers |
| `config/` | TOML/YAML config loading, scope enforcement (`SlapperConfig`, `Scope`) |
| `constants/` | Centralized magic numbers and default values |
| `scanner/` | TCP port scanning, endpoint discovery, service fingerprinting, UDP fingerprinting |
| `fuzzer/` | Fuzz engine with 30 payload types, mutation, grammar, diffing, session handling |
| `waf/` | WAF detection (30+ products), bypass techniques (headers, smuggling, evasion) |
| `recon/` | Passive recon: DNS, WHOIS, SSL, subdomain enum, tech detection, CVE mapping, CORS, cloud |
| `loadtest/` | HTTP load testing with HDR histogram metrics |
| `pipeline/` | Stage-based chained assessment, session resume |
| `tui/` | Interactive terminal UI (ratatui + crossterm) |
| `output/` | Report generation: JSON, HTML, CSV, SARIF, JUnit |
| `distributed/` | Worker/coordinator cluster, task queue, TLS |
| `proxy/` | SOCKS/HTTP/Tor proxy pool with health checks |
| `stress/` | SYN/UDP/HTTP/ICMP flood testing (feature-gated) |
| `packet/` | Packet capture (libpcap), crafting (pnet), hexdump, traceroute |
| `notify/` | Webhook notifications (Slack, Discord, Teams) |
| `tool/` | REST API / MCP / gRPC integration layer (feature-gated) |
| `utils/` | HTTP client creation, URL parsing, stealth, rate limiting, scope checking |
| `error/` | `SlapperError` with `thiserror`, `From` impls for common error types |

## Feature Flags

| Feature | Enables | Dependencies |
|---------|---------|-------------|
| `default` | Core scanning, fuzzing, WAF, load testing | — |
| `stress-testing` | DoS tools, proxy management | pnet, socket2, nix, surge-ping |
| `packet-inspection` | Live capture, traceroute | pnet, libc |
| `python-plugins` | Python plugin support | pyo3 |
| `ruby-plugins` | Ruby + Metasploit integration | magnus |
| `rest-api` | REST API + MCP server | axum, tower |
| `grpc-api` | gRPC server | tonic, prost |
| `nse` | Nmap NSE script support | mlua |
| `full` | All features | all of the above |

## Feature Flag Groupings

Features are organized into logical groupings for easier reference and composite feature configuration:

```
# Plugin System
all-plugins = ["python-plugins", "ruby-plugins"]

# API & Protocol Integration
api-integration = ["rest-api", "grpc-api", "ws-api", "tool-api"]

# AI & Agent Capabilities
ai-capabilities = ["ai-integration", "advanced-hunting"]

# DevSecOps & Compliance
devsecops = ["external-integrations", "database", "sbom", "vuln-management", "compliance", "finding-workflow"]

# Network & Packet Analysis
network-analysis = ["stress-testing", "packet-inspection", "wireless"]

# Browser & Application Testing
app-testing = ["headless-browser", "websocket", "api-schema"]

# Cloud & Container Security
cloud-security = ["cloud", "container"]

# NSE Scripting
nse-scripting = ["nse", "nse-sandbox"]

# Security Research
security-research = ["git-secrets", "pdf"]
```

### Composite Feature Sets

| Group | Features Included |
|-------|-------------------|
| `all-plugins` | `python-plugins`, `ruby-plugins` |
| `api-integration` | `rest-api`, `grpc-api`, `ws-api`, `tool-api` |
| `ai-capabilities` | `ai-integration`, `advanced-hunting` |
| `devsecops` | `external-integrations`, `database`, `sbom`, `vuln-management`, `compliance`, `finding-workflow` |
| `network-analysis` | `stress-testing`, `packet-inspection`, `wireless` |
| `app-testing` | `headless-browser`, `websocket`, `api-schema` |
| `cloud-security` | `cloud`, `container` |
| `nse-scripting` | `nse`, `nse-sandbox` |
| `security-research` | `git-secrets`, `pdf` |

Note: The `full` feature enables all features except `insecure-tls` (which is intentionally excluded due to security risks).

## Command Flow

```
main.rs
  → Cli::parse()
  → load_config()
  → load_scope()
  → CommandContext::new()
  → handle_command()
    → handler (e.g., handle_fuzz)
      → scope check
      → module::run_cli(args, config)
        → e.g., FuzzEngine::new(args).run()
```

## Key Design Patterns

1. **Feature-gated compilation** — `#[cfg(feature = "...")]` gates modules, commands, and dependencies
2. **Consistent command pattern** — Every command: `handler(ctx, args) → module::run_cli(args, config)`
3. **Async-first** — Tokio runtime throughout, `async_trait` for tool interfaces
4. **Builder pattern** — `Pipeline::from_args()`, `FuzzEngine::new()`, `SarifBuilder`
5. **Trait-based tool abstraction** — `SecurityTool` trait enables polymorphic registration for API/MCP
6. **Scope enforcement** — Configurable `Scope` with allowed/excluded targets, CIDR matching
7. **Session persistence** — Scans can be saved/resumed via JSON session files
8. **Centralized constants** — `constants.rs` eliminates magic numbers

## Testing

- **19 integration test files** in `crates/slapper/tests/`
- **WireMock** for HTTP mock servers (`tests/common/wiremock_helpers.rs`)
- **Criterion** for benchmarks, **proptest** for property-based tests
- Inline `#[cfg(test)]` modules for unit tests
- NSE tests require `feature = "nse"`, stress tests require `feature = "stress-testing"`

```bash
# Run all tests
cargo test -p slapper

# Run specific test file
cargo test -p slapper --test proxy_tests

# Run with all features
cargo test -p slapper --features full
```

## Adding a New Command

1. Add variant to `Commands` enum in `cli/mod.rs`
2. Create arg struct in appropriate `cli/` submodule
3. Add handler in `commands/handlers/`
4. Implement logic in the target module
5. Add feature gate if needed: `#[cfg(feature = "...")]`

## Adding a New Fuzz Payload Type

1. Create `fuzzer/payloads/<name>.rs` with `Payload` generation
2. Add variant to `PayloadType` enum in `fuzzer/payloads/mod.rs`
3. Register in `get_payloads()` match
4. Add detection patterns in `fuzzer/detection/`

## Adding a WAF Signature

1. Add signature entry in `waf/detector.rs`
2. Add bypass headers in `waf/bypass/headers.rs` if needed
3. Add tests in `tests/waf_detector_tests.rs`
