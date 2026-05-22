# Slapper Architecture Overview

Slapper is a high-performance, async-first security testing toolkit built in Rust. It is designed for penetration testers and security researchers, offering capabilities from reconnaissance to advanced fuzzing, distributed scanning, and autonomous agent-driven assessments.

## Module Map

This document provides a bird's-eye view of Slapper's architecture and serves as an index to detailed component documentation. Each major area links to a dedicated `.md` file in this directory.

| Module | Source Location | Description | Doc |
|--------|---------------|-------------|-----|
| `agent/` | `crates/slapper/src/agent/` | Autonomous security agent with scheduled scans, memory, and alert routing | [ai_agents.md](ai_agents.md) |
| `ai/` | `crates/slapper/src/ai/` | AI/LLM integration: adaptive fuzzing, payload generation, WAF bypass, planning | [ai_agents.md](ai_agents.md) |
| `auth/` | `crates/slapper/src/auth/` | Authentication testing: brute force, credential stuffing, MFA, SSH, SMTP | - |
| `browser/` | `crates/slapper/src/browser/` | Headless Chrome for DOM XSS and SPA crawling | - |
| `cli/` | `crates/slapper/src/cli/` | Clap-based argument parsing; defines `Commands` enum and per-command args | [cli_commands.md](cli_commands.md) |
| `commands/` | `crates/slapper/src/commands/` | Command dispatch via `handle_command()`; handlers in `handlers/` | [cli_commands.md](cli_commands.md) |
| `compliance/` | `crates/slapper/src/compliance/` | OWASP, PCI-DSS, HIPAA, SOC2 scanning and reporting | - |
| `config/` | `crates/slapper/src/config/` | TOML/YAML config loading, scope enforcement, validation | [config.md](config.md) |
| `container/` | `crates/slapper/src/container/` | Kubernetes and Docker security checks | - |
| `distributed/` | `crates/slapper/src/distributed/` | Worker/coordinator cluster, task queue, TLS, PSK auth | [distributed.md](distributed.md) |
| `fuzzer/` | `crates/slapper/src/fuzzer/` | Security fuzzing engine with 30 payload types, mutation, grammar, diffing | [fuzzer.md](fuzzer.md) |
| `integrations/` | `crates/slapper/src/integrations/` | Jira, GitHub, GitLab connectors | - |
| `loadtest/` | `crates/slapper/src/loadtest/` | HTTP load testing with HDR histogram metrics | [loadtest.md](loadtest.md) |
| `notify/` | `crates/slapper/src/notify/` | Webhook notifications: Slack, Discord, Teams, email | - |
| `output/` | `crates/slapper/src/output/` | Report generation: JSON, HTML, CSV, SARIF, JUnit, PDF, Markdown | [output.md](output.md) |
| `packet/` | `crates/slapper/src/packet/` | Packet capture (libpcap), crafting (pnet), parsing | [networking.md](networking.md) |
| `pipeline/` | `crates/slapper/src/pipeline/` | Stage-based chained assessment orchestration with pause/resume | [pipeline.md](pipeline.md) |
| `proxy/` | `crates/slapper/src/proxy/` | SOCKS/HTTP/Tor proxy pool with health checks | - |
| `recon/` | `crates/slapper/src/recon/` | Passive/active recon: DNS, WHOIS, SSL, subdomain, CVE mapping, cloud | [recon.md](recon.md) |
| `scanner/` | `crates/slapper/src/scanner/` | Port scanning, service fingerprinting, endpoint discovery, CMS detection | [scanner.md](scanner.md) |
| `storage/` | `crates/slapper/src/storage/` | SQLx-based persistence for findings, history, configuration | - |
| `stress/` | `crates/slapper/src/stress/` | SYN/UDP/HTTP/ICMP flood testing (feature-gated `stress-testing`) | [networking.md](networking.md) |
| `supply_chain/` | `crates/slapper/src/supply_chain/` | SBOM generation and analysis | - |
| `tool/` | `crates/slapper/src/tool/` | REST API / MCP / gRPC integration layer; `SecurityTool` trait | - |
| `tui/` | `crates/slapper/src/tui/` | Interactive Terminal UI (ratatui + crossterm), 29 tabs | [tui.md](tui.md) |
| `vuln/` | `crates/slapper/src/vuln/` | Vulnerability triage and lifecycle management | - |
| `waf/` | `crates/slapper/src/waf/` | WAF detection (34 products) and bypass techniques | [waf.md](waf.md) |
| `websocket/` | `crates/slapper/src/websocket/` | WebSocket security testing | - |
| `wireless/` | `crates/slapper/src/wireless/` | Wireless security testing | - |
| `workflow/` | `crates/slapper/src/workflow/` | Finding management and SLA tracking | - |

## Workspace Crates

| Crate | Location | Purpose |
|-------|----------|---------|
| `slapper` | `crates/slapper/` | Core toolkit |
| `slapper-plugin` | `crates/slapper-plugin/` | Python plugin system via `pyo3` |
| `slapper-nse` | `crates/slapper-nse/` | Full Nmap Scripting Engine (NSE) via `mlua` |
| `slapper-ruby` | `crates/slapper-ruby/` | Ruby bridge and Metasploit RPC integration |

## Design Principles

- **Async-First**: Built on `tokio` for high concurrency
- **Modular & Extensible**: Feature flags gate modules; robust plugin system
- **Security-Focused**: Built-in WAF bypass, 30 payload types, scope enforcement
- **Standardized Output**: SARIF, SPDX, JUnit for CI/CD integration
- **Performance-Conscious**: Uses `rustc_hash::FxHashMap`/`FxHashSet` instead of std collections

## Key Architectural Patterns

1. **Feature-gated compilation** — `#[cfg(feature = "...")]` gates modules, commands, and dependencies
2. **Consistent command pattern** — Every command: `handler(ctx, args) → module::run_cli(args, config)`
3. **Async-first** — Tokio runtime throughout, `async_trait` for tool interfaces
4. **Builder pattern** — `Pipeline::from_args()`, `FuzzEngine::new()`, `SarifBuilder`
5. **Trait-based tool abstraction** — `SecurityTool` trait enables polymorphic registration for API/MCP
6. **Scope enforcement** — `Scope` with allowed/excluded targets, CIDR matching; direct IPs blocked
7. **Session persistence** — Scans can be saved/resumed via JSON session files
8. **Centralized constants** — `constants.rs` eliminates magic numbers

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

## Index of Detailed Documentation

| Document | Area |
|----------|------|
| [ai_agents.md](ai_agents.md) | AI module, autonomous agent, MCP integration |
| [cli_commands.md](cli_commands.md) | CLI parsing, command dispatch, handler patterns |
| [config.md](config.md) | Configuration loading, scope enforcement |
| [distributed.md](distributed.md) | Worker/coordinator cluster architecture |
| [fuzzer.md](fuzzer.md) | Fuzzing engine, payload types, detection |
| [loadtest.md](loadtest.md) | HTTP load testing and benchmarking |
| [networking.md](networking.md) | Packet capture/crafting/parsing, stress testing |
| [output.md](output.md) | Reporting formats, deduplication, templates |
| [pipeline.md](pipeline.md) | Stage orchestration, session resume |
| [plugins_nse.md](plugins_nse.md) | Python/Ruby plugins, NSE integration |
| [recon.md](recon.md) | Reconnaissance modules and runner |
| [scanner.md](scanner.md) | Port scanning, fingerprinting, endpoints |
| [tui.md](tui.md) | Terminal UI, 29 tabs, components, workers |
| [waf.md](waf.md) | WAF detection, bypass profiles, smuggling |

## Quick Reference

- **Feature flags**: `stress-testing`, `packet-inspection`, `python-plugins`, `ruby-plugins`, `nse`, `ai-integration`, `rest-api`, `grpc-api`, `ws-api`, `full`
- **Severity**: Single canonical definition in `types.rs`; `Severity` enum re-exported everywhere
- **Error type**: `SlapperError` in `error/mod.rs` with `thiserror`
- **Key crates**: `tokio`, `clap`, `ratatui`, `rustc_hash`, `sqlx`, `serde`, `tracing`

---
*This overview serves as a guide to the architecture documentation. See individual `.md` files for deep dives.*