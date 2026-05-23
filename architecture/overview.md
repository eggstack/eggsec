# Slapper Architecture Overview

Slapper is a high-performance, async-first security testing toolkit built in Rust. Designed for penetration testers and security researchers, it offers capabilities from reconnaissance to advanced fuzzing, distributed scanning, and autonomous agent-driven assessments.

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              main.rs                                        │
│                    CLI Parsing → Config Loading → Scope                     │
└─────────────────────────────┬───────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                    CommandContext (global state)                            │
│              SlapperConfig + Scope + Output + Logging                       │
└─────────────────────────────┬───────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                      handle_command()                                        │
│                    Command Dispatch Layer                                   │
└─────────────────────────────┬───────────────────────────────────────────────┘
                              │
          ┌───────────────────┼───────────────────┬────────────────────┐
          ▼                   ▼                   ▼                    ▼
   ┌─────────────┐     ┌─────────────┐     ┌─────────────┐      ┌─────────────┐
   │   cli/      │     │ commands/   │     │    tui/     │      │  tool/      │
   │  Parsing    │     │  Handlers   │     │   (TUI)     │      │   MCP/API   │
   └─────────────┘     └──────┬──────┘     └─────────────┘      └─────────────┘
                             │
          ┌──────────────────┼──────────────────┬──────────────────┐
          ▼                  ▼                  ▼                  ▼
   ┌─────────────┐    ┌─────────────┐   ┌─────────────┐    ┌─────────────┐
   │   scanner/  │    │   fuzzer/   │   │    recon/   │    │   pipeline/ │
   │   Port scan │    │   Fuzzing   │   │   Recon     │    │  Orchestrat.│
   │ Fingerprint│    │  Payloads   │   │   DNS,WHOIS │    │   Stages    │
   └─────────────┘    └─────────────┘   └─────────────┘    └─────────────┘
          │                  │                  │                  │
          │                  │                  │                  │
          ▼                  ▼                  ▼                  ▼
   ┌─────────────┐    ┌─────────────┐   ┌─────────────┐    ┌─────────────┐
   │    waf/     │    │  loadtest/  │   │  output/    │    │ distributed/│
   │Detection   │    │  Benchmark  │   │  Reporting  │    │  Cluster    │
   │  Bypass     │    │             │   │  SARIF,PDF  │    │             │
   └─────────────┘    └─────────────┘   └─────────────┘    └─────────────┘
          │                  │                  │                  │
          └──────────────────┼──────────────────┼──────────────────┘
                             │                  │
                             ▼                  ▼
                    ┌────────────────┐   ┌────────────────┐
                    │    storage/    │   │    notify/     │
                    │   SQLx DB      │   │  Webhooks      │
                    └────────────────┘   └────────────────┘
```

## Module Map

This document provides a bird's-eye view of Slapper's architecture and serves as an index to detailed component documentation. Each major area with a dedicated `.md` file is linked.

### Core Modules

| Module | Source Location | Description | Doc |
|--------|---------------|-------------|-----|
| `cli/` | `crates/slapper/src/cli/` | Clap-based argument parsing; `Commands` enum (35+ variants), per-command args | [cli_commands.md](cli_commands.md) |
| `commands/` | `crates/slapper/src/commands/` | Command dispatch via `handle_command()`; handlers in `handlers/` | [cli_commands.md](cli_commands.md) |
| `config/` | `crates/slapper/src/config/` | TOML/YAML config loading, scope enforcement, validation | [config.md](config.md) |
| `tool/` | `crates/slapper/src/tool/` | REST API / MCP / gRPC integration; `SecurityTool` trait, `ToolRegistry` | - |

### Security Testing Modules

| Module | Source Location | Description | Doc |
|--------|---------------|-------------|-----|
| `scanner/` | `crates/slapper/src/scanner/` | Port scanning, service fingerprinting, endpoint discovery, CMS detection | [scanner.md](scanner.md) |
| `fuzzer/` | `crates/slapper/src/fuzzer/` | Security fuzzing engine with 31 payload types, mutation, grammar, diffing | [fuzzer.md](fuzzer.md) |
| `recon/` | `crates/slapper/src/recon/` | Passive/active recon: DNS, WHOIS, SSL, subdomain, CVE mapping, cloud | [recon.md](recon.md) |
| `waf/` | `crates/slapper/src/waf/` | WAF detection (34 products) and bypass techniques | [waf.md](waf.md) |
| `auth/` | `crates/slapper/src/auth/` | Authentication testing: brute force, credential stuffing, MFA, SSH, SMTP | - |
| `browser/` | `crates/slapper/src/browser/` | Headless Chrome for DOM XSS and SPA crawling | - |
| `websocket/` | `crates/slapper/src/websocket/` | WebSocket security testing | - |
| `wireless/` | `crates/slapper/src/wireless/` | Wireless security testing | - |
| `hunt/` | `crates/slapper/src/hunt/` | Intelligent vulnerability hunting | - |

### Assessment Orchestration

| Module | Source Location | Description | Doc |
|--------|---------------|-------------|-----|
| `pipeline/` | `crates/slapper/src/pipeline/` | Stage-based chained assessment orchestration with pause/resume | [pipeline.md](pipeline.md) |
| `agent/` | `crates/slapper/src/agent/` | Autonomous security agent with scheduled scans, memory, and alert routing | [ai_agents.md](ai_agents.md) |
| `workflow/` | `crates/slapper/src/workflow/` | Finding management and SLA tracking | - |
| `vuln/` | `crates/slapper/src/vuln/` | Vulnerability triage and lifecycle management | - |

### AI & Intelligence

| Module | Source Location | Description | Doc |
|--------|---------------|-------------|-----|
| `ai/` | `crates/slapper/src/ai/` | AI/LLM integration: adaptive fuzzing, payload generation, WAF bypass, planning | [ai_agents.md](ai_agents.md) |

### Performance & Load

| Module | Source Location | Description | Doc |
|--------|---------------|-------------|-----|
| `loadtest/` | `crates/slapper/src/loadtest/` | HTTP load testing with HDR histogram metrics | [loadtest.md](loadtest.md) |
| `stress/` | `crates/slapper/src/stress/` | SYN/UDP/HTTP/ICMP flood testing (feature-gated `stress-testing`) | [networking.md](networking.md) |

### Networking & Packets

| Module | Source Location | Description | Doc |
|--------|---------------|-------------|-----|
| `packet/` | `crates/slapper/src/packet/` | Packet capture (libpcap), crafting (pnet), parsing | [networking.md](networking.md) |
| `proxy/` | `crates/slapper/src/proxy/` | SOCKS/HTTP/Tor proxy pool with health checks | - |

### Data & Reporting

| Module | Source Location | Description | Doc |
|--------|---------------|-------------|-----|
| `output/` | `crates/slapper/src/output/` | Report generation: JSON, HTML, CSV, SARIF, JUnit, PDF, Markdown | [output.md](output.md) |
| `storage/` | `crates/slapper/src/storage/` | SQLx-based persistence for findings, history, configuration | - |

### Integration & Compliance

| Module | Source Location | Description | Doc |
|--------|---------------|-------------|-----|
| `integrations/` | `crates/slapper/src/integrations/` | Jira, GitHub, GitLab connectors | - |
| `compliance/` | `crates/slapper/src/compliance/` | OWASP, PCI-DSS, HIPAA, SOC2 scanning and reporting | - |
| `container/` | `crates/slapper/src/container/` | Kubernetes and Docker security checks | - |
| `supply_chain/` | `crates/slapper/src/supply_chain/` | SBOM generation and analysis | - |

### User Interface

| Module | Source Location | Description | Doc |
|--------|---------------|-------------|-----|
| `tui/` | `crates/slapper/src/tui/` | Interactive Terminal UI (ratatui + crossterm), 29 tabs | [tui.md](tui.md) |

### Notifications & Utilities

| Module | Source Location | Description | Doc |
|--------|---------------|-------------|-----|
| `notify/` | `crates/slapper/src/notify/` | Webhook notifications: Slack, Discord, Teams, email | - |
| `logging/` | `crates/slapper/src/logging/` | Structured logging with tracing | - |
| `error/` | `crates/slapper/src/error/` | Central error types (`SlapperError`) | - |
| `utils/` | `crates/slapper/src/utils/` | Circuit breaker, formatting, rate limiting, regex caching | - |

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
- **Security-Focused**: Built-in WAF bypass, 31 payload types, scope enforcement
- **Standardized Output**: SARIF, SPDX, JUnit for CI/CD integration
- **Performance-Conscious**: Uses `rustc_hash::FxHashMap`/`FxHashSet` instead of std collections

## Key Architectural Patterns

### 1. Feature-Gated Compilation
`#[cfg(feature = "...")]` gates modules, commands, and dependencies:
- `stress-testing` - Raw sockets, IP spoofing
- `packet-inspection` - Packet capture
- `python-plugins` / `ruby-plugins` - Plugin language support
- `nse` - Nmap NSE script support
- `ai-integration` - AI planner, script generation, autonomous agent skills
- `ws-api` - WebSocket pub/sub
- `rest-api` / `grpc-api` - API server integration
- `full` - All features combined

### 2. Consistent Command Pattern
Every command follows: `handler(ctx, args) → scope check → module::run_cli(args, config)`

### 3. Builder Pattern
- `Pipeline::from_args()`
- `FuzzEngine::new(args).run()`
- `SarifBuilder::new()`
- `LoadTestRunner::new(url, total, concurrency, timeout)`

### 4. Trait-Based Tool Abstraction
`SecurityTool` trait enables polymorphic registration for API/MCP integration:
```rust
pub trait SecurityTool: Send + Sync {
    fn name(&self) -> &str;
    async fn execute(&self, target: &Target, args: Value) -> Result<Value>;
    fn capabilities(&self) -> Vec<Capability>;
}
```

### 5. Scope Enforcement
`Scope` struct with allowed/excluded targets, CIDR matching; direct IPs blocked via `TargetScope::parse()`.

### 6. Session Persistence
Scans can be saved/resumed via JSON session files with `PipelineContext` serialization.

### 7. Centralized Constants
`constants.rs` eliminates magic numbers across modules.

## Command Flow

```
main.rs
  → Cli::parse()
  → load_config()
  → load_scope()
  → CommandContext::new()
  → handle_command()
    → handler (e.g., handle_fuzz)
      → scope check (ensure_scope_url)
      → module::run_cli(args, config)
        → e.g., FuzzEngine::new(args).run()
```

## Key Data Types

| Type | Location | Purpose |
|------|----------|---------|
| `SlapperConfig` | `config/settings.rs` | Main configuration struct |
| `Scope` | `config/scope.rs` | Target allow/block enforcement |
| `Severity` | `types.rs` | Unified severity enum (Critical, High, Medium, Low, Info) - single canonical definition |
| `SlapperError` | `error/mod.rs` | Unified error type via `thiserror` |
| `TabError` | `tui/app/tab_error.rs` | Structured error type with categories |
| `PayloadType` | `types.rs` | 31 payload categories |
| `SecurityTool` | `tool/traits.rs` | Trait for tool abstraction |
| `ToolRegistry` | `tool/registry.rs` | Dynamic tool registration |
| `AiClient` | `ai/client.rs` | LLM client with provider abstraction |
| `SmartWafBypass` | `ai/waf_bypass.rs` | WAF bypass with knowledge base |
| `AiPlanner` | `ai/planner.rs` | AI-driven execution planning |

## Index of Detailed Documentation

| Document | Area | Modules Covered |
|----------|------|-----------------|
| [ai_agents.md](ai_agents.md) | AI/LLM integration and autonomous agents | `ai/`, `agent/`, `tool/agents/` |
| [cli_commands.md](cli_commands.md) | CLI parsing and command dispatch | `cli/`, `commands/` |
| [config.md](config.md) | Configuration system and scope enforcement | `config/` |
| [distributed.md](distributed.md) | Worker/coordinator cluster architecture | `distributed/` |
| [fuzzer.md](fuzzer.md) | Fuzzing engine and payload types | `fuzzer/` |
| [loadtest.md](loadtest.md) | HTTP load testing and benchmarking | `loadtest/` |
| [networking.md](networking.md) | Packet capture/crafting and stress testing | `packet/`, `stress/` |
| [output.md](output.md) | Reporting formats and deduplication | `output/` |
| [pipeline.md](pipeline.md) | Stage orchestration and session management | `pipeline/` |
| [plugins_nse.md](plugins_nse.md) | Python/Ruby plugins and NSE integration | `slapper-plugin/`, `slapper-nse/`, `slapper-ruby/` |
| [recon.md](recon.md) | Reconnaissance modules and runner | `recon/` |
| [scanner.md](scanner.md) | Port scanning and fingerprinting | `scanner/` |
| [tui.md](tui.md) | Terminal UI, 29 tabs, components, workers | `tui/` |
| [waf.md](waf.md) | WAF detection and bypass techniques | `waf/` |

## Modules Without Detailed Docs

The following modules currently lack dedicated architecture documentation (candidates for future reviews):

- `auth/` - Authentication testing
- `browser/` - Headless Chrome
- `compliance/` - Compliance scanning
- `container/` - Container security
- `hunt/` - Vulnerability hunting
- `integrations/` - Issue tracker connectors
- `notify/` - Webhook notifications
- `proxy/` - Proxy pool management
- `storage/` - SQLx persistence
- `supply_chain/` - SBOM analysis
- `tool/` (core) - Tool abstraction (partially covered in ai_agents.md)
- `vuln/` - Vulnerability management
- `websocket/` - WebSocket testing
- `wireless/` - Wireless security
- `workflow/` - Finding management

## Quick Reference

| Item | Value |
|------|-------|
| Feature flags | `stress-testing`, `packet-inspection`, `python-plugins`, `ruby-plugins`, `nse`, `ai-integration`, `rest-api`, `grpc-api`, `ws-api`, `full` |
| Severity | Single canonical definition in `types.rs`; `Severity` enum re-exported everywhere |
| Error type | `SlapperError` in `error/mod.rs` with `thiserror` |
| Key crates | `tokio`, `clap`, `ratatui`, `rustc_hash`, `sqlx`, `serde`, `tracing` |
| Total modules | 41 modules in `crates/slapper/src/` |
| Detailed docs | 14 architecture documents in `architecture/` |

---

*This overview serves as a guide to the architecture documentation. See individual `.md` files for deep dives into each component.*