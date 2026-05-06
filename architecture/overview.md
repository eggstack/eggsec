# Slapper Architectural Overview

Slapper is a high-performance, async-first security testing toolkit built in Rust. It is designed for penetration testers and security researchers, offering a wide range of capabilities from reconnaissance to advanced fuzzing and distributed scanning.

## Core Module Groups

- **[CLI & Commands](cli_commands.md)**: Command-line argument parsing and command dispatch.
- **[Configuration](config.md)**: TOML/YAML configuration loading and scope enforcement.
- **[Scanner](scanner.md)**: Port scanning, service fingerprinting, and endpoint discovery.
- **[Fuzzer](fuzzer.md)**: Advanced security fuzzing engine with support for 22 payload types.
- **[WAF](waf.md)**: Web Application Firewall detection and bypass techniques.
- **[Reconnaissance](recon.md)**: Passive and active recon (DNS, WHOIS, SSL, CVE mapping).
- **[Load Testing](loadtest.md)**: High-performance HTTP load testing with real-time metrics.
- **[Pipeline](pipeline.md)**: Orchestration of chained security assessment profiles.
- **[AI & Agents](ai_agents.md)**: AI-driven analysis, payload generation, and autonomous agent integration via MCP.
- **[TUI](tui.md)**: Real-time Terminal User Interface for interactive scanning.
- **[Output & Reporting](output.md)**: Support for multiple formats including JSON, SARIF, and PDF.
- **[Distributed](distributed.md)**: Scalable worker/coordinator architecture for large-scale assessments.
- **[Networking](networking.md)**: Packet capture, crafting, and low-level stress testing.
- **[Plugins & NSE](plugins_nse.md)**: Extensibility via Python, Ruby, and Nmap Scripting Engine.

## Specialized Modules

- **Authentication (`auth`)**: Support for various authentication mechanisms (Basic, Bearer, OAuth, Custom).
- **Headless Browser (`browser`)**: Integration with headless Chrome for DOM XSS and SPA crawling.
- **Compliance (`compliance`)**: Scanning and reporting against security standards (e.g., OWASP, PCI-DSS).
- **Container Security (`container`)**: Kubernetes and Docker-specific security checks.
- **Integrations (`integrations`)**: Connectors for Jira, GitHub, GitLab, and other external tools.
- **Storage (`storage`)**: Persistence layer for findings, history, and configuration using SQLx.
- **Supply Chain (`supply_chain`)**: Tools for generating and analyzing SBOMs (Software Bill of Materials).
- **Vulnerability Management (`vuln`, `workflow`)**: Triage, prioritization, and lifecycle management of discovered vulnerabilities.

## Workspace Crates

- **`slapper`**: The core toolkit crate containing the majority of the logic.
- **`slapper-plugin`**: A flexible plugin system supporting Python and Ruby extensions, allowing for easy custom scanner development.
- **`slapper-nse`**: A full integration of the Nmap Scripting Engine (NSE), allowing Slapper to run thousands of existing Nmap scripts.
- **`slapper-ruby`**: Specialized bridge for Ruby-based tools and Metasploit RPC integration.

## Design Principles

- **Async-First**: Built on top of `tokio` for high concurrency and performance.
- **Modular & Extensible**: Heavy use of feature flags and a robust plugin system.
- **Security-Focused**: Built-in WAF bypass, payload generation, and threat hunting features.
- **Standardized**: Support for industry-standard formats like SARIF and SPDX.

## Module Map

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

## Adding New Components

- **[Adding a New Command](cli_commands.md)** — Add variant to `Commands` enum, create arg struct, add handler
- **[Adding a New Fuzz Payload Type](fuzzer.md)** — Create payload file, add variant to `PayloadType`, register
- **[Adding a WAF Signature](waf.md)** — Add signature entry, bypass headers if needed

---
*This overview serves as an index for detailed component documentation.*
