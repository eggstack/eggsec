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
     │ Port scan   │    │  Fuzzing    │   │  DNS,WHOIS  │    │   Stages    │
     │ Fingerprint │    │  Payloads   │   │             │    │             │
     └─────────────┘    └─────────────┘   └─────────────┘    └─────────────┘
            │                  │                  │                  │
            │                  │                  │                  │
            ▼                  ▼                  ▼                  ▼
     ┌─────────────┐    ┌─────────────┐   ┌─────────────┐    ┌─────────────┐
     │    waf/     │    │  loadtest/  │   │  output/    │    │ distributed/│
     │  Detection  │    │  Benchmark  │   │  Reporting  │    │   Cluster   │
     │   Bypass    │    │             │   │ SARIF,PDF   │    │             │
     └─────────────┘    └─────────────┘   └─────────────┘    └─────────────┘
            │                  │                  │                  │
            └──────────────────┼──────────────────┼──────────────────┘
                               │                  │
                               ▼                  ▼
                      ┌────────────────┐   ┌────────────────┐
                      │    notify/    │   │    storage/    │
                      │   Webhooks    │   │   SQLx DB      │
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
| `types.rs` | `crates/slapper/src/types.rs` | Canonical `Severity` enum; shared types re-exported project-wide | - |
| `error/` | `crates/slapper/src/error/` | Central error types (`SlapperError`, `Result`) | - |

### Security Testing Modules

| Module | Source Location | Description | Doc |
|--------|---------------|-------------|-----|
| `scanner/` | `crates/slapper/src/scanner/` | Port scanning (TCP/SYN), service fingerprinting, endpoint discovery, CMS detection | [scanner.md](scanner.md) |
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
| `workflow/` | `crates/slapper/src/workflow/` | Finding management and SLA tracking | - |
| `vuln/` | `crates/slapper/src/vuln/` | Vulnerability triage and lifecycle management | - |

### AI & Intelligence

| Module | Source Location | Description | Doc |
|--------|---------------|-------------|-----|
| `ai/` | `crates/slapper/src/ai/` | AI/LLM integration: adaptive fuzzing, payload generation, WAF bypass, planning | [ai_agents.md](ai_agents.md) |
| `agent/` | `crates/slapper/src/agent/` | Autonomous security agent with scheduled scans, memory, and alert routing | [ai_agents.md](ai_agents.md) |

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

### Tool Abstraction Layer

| Module | Source Location | Description | Doc |
|--------|---------------|-------------|-----|
| `tool/` | `crates/slapper/src/tool/` | REST API / MCP / gRPC integration; `SecurityTool` trait, `ToolRegistry` | [ai_agents.md](ai_agents.md) |

### User Interface

| Module | Source Location | Description | Doc |
|--------|---------------|-------------|-----|
| `tui/` | `crates/slapper/src/tui/` | Interactive Terminal UI (ratatui + crossterm), 29 tabs | [tui.md](tui.md) |

### Notifications & Utilities

| Module | Source Location | Description | Doc |
|--------|---------------|-------------|-----|
| `notify/` | `crates/slapper/src/notify/` | Webhook notifications: Slack, Discord, Teams, email | - |
| `logging/` | `crates/slapper/src/logging/` | Structured logging with tracing | - |
| `utils/` | `crates/slapper/src/utils/` | Circuit breaker, formatting, rate limiting, regex caching | - |
| `constants.rs` | `crates/slapper/src/constants.rs` | Centralized magic number elimination | - |

## Workspace Crates

| Crate | Location | Purpose |
|-------|----------|---------|
| `slapper` | `crates/slapper/` | Core toolkit |
| `slapper-plugin` | `crates/slapper-plugin/` | Python plugin system via `pyo3` |
| `slapper-nse` | `crates/slapper-nse/` | Full Nmap Scripting Engine (NSE) via `mlua` |
| `slapper-ruby` | `crates/slapper-ruby/` | Ruby bridge and Metasploit RPC integration |

**NSE Integration**: Full Lua VM with 164 NSE-style library modules (stdnse, nmap, http, socket, dns, ssl, ssh, mysql, postgres, redis, mongodb, ldap, snmp, smb, vulns, etc.). Supports sandboxed execution with path canonicalization and network allowlisting. See [plugins_nse.md](plugins_nse.md) for details.

## Module Interconnections

Understanding how modules work together is essential for effective security assessments:

### Typical Assessment Flow

```
┌──────────┐     ┌───────────┐     ┌───────────┐     ┌─────────┐
│  Recon   │────▶│  Scanner  │────▶│ Endpoint  │────▶│  Fuzzer │
│ (intel)  │     │ (ports)   │     │ (paths)   │     │ (bugs)  │
└──────────┘     └───────────┘     └───────────┘     └─────────┘
     │               │                  │                 │
     │               │                  │                 │
     ▼               ▼                  ▼                 ▼
┌──────────┐     ┌───────────┐     ┌───────────┐     ┌─────────┐
│   WAF    │◀────│  Pipeline  │────▶│  LoadTest │     │   AI    │
│ (bypass) │     │(orchestra.)│     │ (perf)    │     │(adapt)  │
└──────────┘     └───────────┘     └───────────┘     └─────────┘
                       │
                       ▼
                 ┌───────────┐
                 │  Output   │
                 │ (reports) │
                 └───────────┘
```

### Key Module Dependencies

| From | To | Purpose |
|------|-----|---------|
| `cli/` | `commands/` | Parsed args → handler dispatch |
| `commands/handlers/` | `pipeline/` | Pipeline execution |
| `pipeline/` | `scanner/`, `fuzzer/`, `recon/`, `waf/`, `loadtest/` | Stage orchestration |
| `scanner/` | `waf/` | WAF detection during discovery |
| `fuzzer/` | `waf/` | Bypass detection during fuzzing |
| `fuzzer/` | `ai/` | Adaptive payload generation |
| `agent/` | `tool/` | Autonomous scanning via tool abstraction |
| `tool/` | All modules | MCP/REST API exposure |
| `output/` | All modules | Report generation from any findings |

## Design Principles

- **Async-First**: Built on `tokio` for high concurrency across all network operations
- **Modular & Extensible**: Feature flags gate modules; robust plugin system via Python/Ruby/NSE
- **Security-Focused**: Built-in WAF bypass, 31 payload types, scope enforcement
- **Standardized Output**: SARIF, SPDX, JUnit for CI/CD integration
- **Performance-Conscious**: Uses `rustc_hash::FxHashMap`/`FxHashSet` instead of std collections

## Key Architectural Patterns

### 1. Feature-Gated Compilation

`#[cfg(feature = "...")]` gates modules, commands, and dependencies:

| Feature | Enables |
|---------|---------|
| `stress-testing` | Raw sockets, IP spoofing, DoS tools |
| `packet-inspection` | Packet capture, traceroute |
| `python-plugins` / `ruby-plugins` | Plugin language support |
| `nse` | Nmap NSE script support |
| `ai-integration` | AI planner, script generation, autonomous agent skills |
| `rest-api` / `grpc-api` | API server integration |
| `ws-api` | WebSocket pub/sub |
| `database` | SQLx-based persistence |
| `cloud` | Cloud security scanning (AWS, GCP, Azure) |
| `container` | Kubernetes/Docker security checks |
| `advanced-hunting` | Advanced threat hunting |
| `compliance` | OWASP, PCI-DSS, HIPAA, SOC2 reporting |
| `pdf` | PDF report generation |
| `full` | All features combined |

### 2. Consistent Command Pattern

Every command follows this flow:

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

### 3. Builder Pattern

Used throughout for fluent initialization:

```rust
Pipeline::from_args(args)
FuzzEngine::new(args)
SarifBuilder::new()
LoadTestRunner::new(url, total, concurrency, timeout)
SmartWafBypass::new(client, config)
```

### 4. Trait-Based Tool Abstraction

`SecurityTool` trait enables polymorphic registration for API/MCP integration:

```rust
pub trait SecurityTool: Send + Sync {
    fn name(&self) -> &str;
    async fn execute(&self, target: &Target, args: Value) -> Result<Value>;
    fn capabilities(&self) -> Vec<Capability>;
}
```

`ToolRegistry` manages dynamic registration and lookup. See [ai_agents.md](ai_agents.md) for MCP/OpenAI protocol adapters.

### 5. Scope Enforcement

`Scope` struct in `config/scope.rs` enforces target restrictions:

- `is_target_allowed(target)` - checks if target passes scope rules
- `validate_url(url)` - validates URL's host via scope rules
- `is_port_allowed(port)` - checks port allowlist/blocklist
- **Private IP blocking**: Direct IP addresses (e.g., `127.0.0.1`) blocked via `TargetScope::parse()`
- CIDR notation supported for range-based scope rules

See [config.md](config.md) for detailed scope rule evaluation.

### 6. Session Persistence

Scans can be saved/resumed via JSON session files:

- `PipelineSession` in `pipeline/session.rs`
- Checkpoints written only when output path matches `*.session` or `*.session.json`
- `PipelineContext` serialization preserves inter-stage data

### 7. Centralized Constants

`constants.rs` eliminates magic numbers across modules, providing named constants for thresholds, timeouts, buffer sizes, and scoring weights.

## Key Data Types

| Type | Location | Purpose |
|------|----------|---------|
| `SlapperConfig` | `config/settings.rs` | Main configuration struct |
| `Scope` | `config/scope.rs` | Target allow/block enforcement |
| `Severity` | `types.rs` | Unified severity enum (Critical, High, Medium, Low, Info) |
| `PayloadType` | `fuzzer/payloads/mod.rs` | 31 payload categories for fuzzing |
| `SlapperError` | `error/mod.rs` | Unified error type via `thiserror` |
| `TabError` | `tui/app/tab_error.rs` | Structured error type with recovery categories |
| `SecurityTool` | `tool/traits.rs` | Trait for tool abstraction |
| `ToolRegistry` | `tool/registry.rs` | Dynamic tool registration |
| `AiClient` | `ai/client.rs` | LLM client with provider abstraction |
| `SmartWafBypass` | `ai/waf_bypass.rs` | WAF bypass with knowledge base |
| `AiPlanner` | `ai/planner.rs` | AI-driven execution planning |
| `FuzzEngine` | `fuzzer/engine/mod.rs` | Core fuzzing orchestration |
| `PipelineContext` | `pipeline/context.rs` | Inter-stage data passing |
| `Stage` | `pipeline/stage.rs` | Pipeline stage enum with 11 profiles |

## Index of Detailed Documentation

| Document | Area | Modules Covered |
|----------|------|-----------------|
| [ai_agents.md](ai_agents.md) | AI/LLM integration and autonomous agents | `ai/`, `agent/`, `tool/` |
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

The following modules currently lack dedicated architecture documentation (candidates for future deep dives):

| Module | Purpose |
|--------|---------|
| `auth/` | Authentication testing (brute force, credential stuffing, MFA, SSH, SMTP) |
| `browser/` | Headless Chrome for DOM XSS and SPA crawling |
| `compliance/` | Compliance scanning and reporting (OWASP, PCI-DSS, HIPAA, SOC2) |
| `container/` | Kubernetes and Docker security checks |
| `hunt/` | Intelligent vulnerability hunting |
| `integrations/` | Issue tracker connectors (Jira, GitHub, GitLab) |
| `notify/` | Webhook notifications (Slack, Discord, Teams, email) |
| `proxy/` | SOCKS/HTTP/Tor proxy pool with health checks |
| `storage/` | SQLx-based persistence for findings, history, configuration |
| `supply_chain/` | SBOM generation and analysis |
| `vuln/` | Vulnerability triage and lifecycle management |
| `websocket/` | WebSocket security testing |
| `wireless/` | Wireless security testing |
| `workflow/` | Finding management and SLA tracking |

## Quick Reference

| Item | Value |
|------|-------|
| Total modules | 41 modules in `crates/slapper/src/` |
| Detailed docs | 14 architecture documents in `architecture/` |
| Workspace crates | 4 (slapper, slapper-plugin, slapper-nse, slapper-ruby) |
| Payload types | 31 (defined in `fuzzer/payloads/mod.rs`) |
| WAF products | 34 (defined in `waf/data/patterns.rs`) |
| TUI tabs | 29 |
| Pipeline profiles | 11 |
| Feature flags | 20+ |
| NSE libraries | 164 (in `slapper-nse/src/libraries/`) |

---

*This overview serves as the entry point to the architecture documentation. See individual `.md` files for deep dives into each component.*