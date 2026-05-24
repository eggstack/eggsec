# Slapper Architecture Overview

Slapper is a high-performance, async-first security testing toolkit built in Rust. It provides capabilities from reconnaissance to advanced fuzzing, distributed scanning, and autonomous agent-driven assessments.

**Quick Facts:**
- 41 modules in `crates/slapper/src/`
- 743 source files
- 1324 base tests (1469+ with full features)
- 31 payload types for fuzzing
- 29 TUI tabs
- 20+ feature flags

---

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
             ┌─────────────────┼─────────────────┬────────────────────┐
             ▼                 ▼                  ▼                    ▼
      ┌─────────────┐   ┌─────────────┐    ┌─────────────┐     ┌─────────────┐
      │   cli/      │   │ commands/   │    │    tui/     │     │    tool/    │
      │  Parsing    │   │  Handlers   │    │   (TUI)     │     │    MCP      │
      └─────────────┘   └──────┬──────┘    └─────────────┘     └─────────────┘
                               │
             ┌─────────────────┼─────────────────┬────────────────┐
             ▼                 ▼                  ▼                ▼
      ┌─────────────┐   ┌─────────────┐   ┌─────────────┐  ┌─────────────┐
      │   scanner/  │   │   fuzzer/   │   │    recon/   │  │  pipeline/  │
      │ Port scan   │   │  Fuzzing    │   │   (intel)   │  │  Stages    │
      └─────────────┘   └─────────────┘   └─────────────┘  └─────────────┘
             │                 │                  │                │
             │                 │                  │                │
             ▼                 ▼                  ▼                ▼
      ┌─────────────┐   ┌─────────────┐   ┌─────────────┐  ┌─────────────┐
      │    waf/     │   │  loadtest/  │   │   output/    │  │ distributed/│
      │  Detection  │   │  Benchmark  │   │   Reports    │  │   Cluster   │
      │   Bypass    │   │             │   │              │  │             │
      └─────────────┘   └─────────────┘   └─────────────┘  └─────────────┘
```

---

## Module Index (Deep Dives)

Each major area links to a detailed `.md` file in this directory. Modules without links are candidates for future documentation.

### Core Infrastructure

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `cli/` | `crates/slapper/src/cli/` | Clap-based argument parsing; `Commands` enum (35+ variants) | [cli_commands.md](cli_commands.md) |
| `commands/` | `crates/slapper/src/commands/` | Command dispatch via `handle_command()`; handlers | [cli_commands.md](cli_commands.md) |
| `config/` | `crates/slapper/src/config/` | TOML/YAML config loading, scope enforcement | [config.md](config.md) |
| `types.rs` | `crates/slapper/src/types.rs` | Canonical `Severity` enum (Critical/High/Medium/Low/Info) | - |
| `error/` | `crates/slapper/src/error/` | Core error types (`SlapperError`, `Result`) | - |

### Security Testing

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `scanner/` | `crates/slapper/src/scanner/` | Port scanning (TCP/SYN), service fingerprinting, endpoint discovery | [scanner.md](scanner.md) |
| `fuzzer/` | `crates/slapper/src/fuzzer/` | Security fuzzing engine with 31 payload types, mutation, grammar | [fuzzer.md](fuzzer.md) |
| `recon/` | `crates/slapper/src/recon/` | Passive/active recon: DNS, WHOIS, SSL, subdomain, CVE mapping, cloud | [recon.md](recon.md) |
| `waf/` | `crates/slapper/src/waf/` | WAF detection (34 products) and bypass techniques | [waf.md](waf.md) |
| `auth/` | `crates/slapper/src/auth/` | Authentication testing: brute force, credential stuffing, MFA, SSH, SMTP | - |
| `browser/` | `crates/slapper/src/browser/` | Headless Chrome for DOM XSS and SPA crawling | - |
| `websocket/` | `crates/slapper/src/websocket/` | WebSocket security testing | - |
| `wireless/` | `crates/slapper/src/wireless/` | Wireless security testing | - |
| `hunt/` | `crates/slapper/src/hunt/` | Intelligent vulnerability hunting | - |

### Assessment Orchestration

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `pipeline/` | `crates/slapper/src/pipeline/` | Stage-based chained assessment with pause/resume (11 profiles) | [pipeline.md](pipeline.md) |
| `workflow/` | `crates/slapper/src/workflow/` | Finding management and SLA tracking | - |
| `vuln/` | `crates/slapper/src/vuln/` | Vulnerability triage and lifecycle management | - |

### AI & Autonomous Agents

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `ai/` | `crates/slapper/src/ai/` | AI/LLM integration: adaptive fuzzing, WAF bypass, payload generation, planning | [ai_agents.md](ai_agents.md) |
| `agent/` | `crates/slapper/src/agent/` | Autonomous security agent with scheduled scans, memory, alert routing | [ai_agents.md](ai_agents.md) |
| `tool/` | `crates/slapper/src/tool/` | SecurityTool trait, ToolRegistry, MCP/REST/gRPC integration | [ai_agents.md](ai_agents.md) |

### Performance & Load

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `loadtest/` | `crates/slapper/src/loadtest/` | HTTP load testing with HDR histogram metrics | [loadtest.md](loadtest.md) |
| `stress/` | `crates/slapper/src/stress/` | SYN/UDP/HTTP/ICMP flood testing (feature-gated) | [networking.md](networking.md) |

### Networking & Packets

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `packet/` | `crates/slapper/src/packet/` | Packet capture (libpcap), crafting (pnet), parsing | [networking.md](networking.md) |
| `proxy/` | `crates/slapper/src/proxy/` | SOCKS/HTTP/Tor proxy pool with health checks | - |

### Data & Reporting

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `output/` | `crates/slapper/src/output/` | Report generation: JSON, HTML, CSV, SARIF, JUnit, PDF, Markdown | [output.md](output.md) |
| `storage/` | `crates/slapper/src/storage/` | SQLx-based persistence for findings, history, configuration | - |

### Integration & Compliance

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `integrations/` | `crates/slapper/src/integrations/` | Jira, GitHub, GitLab connectors | - |
| `compliance/` | `crates/slapper/src/compliance/` | OWASP, PCI-DSS, HIPAA, SOC2 scanning and reporting | - |
| `container/` | `crates/slapper/src/container/` | Kubernetes and Docker security checks | - |
| `supply_chain/` | `crates/slapper/src/supply_chain/` | SBOM generation and analysis | - |

### User Interface

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `tui/` | `crates/slapper/src/tui/` | Interactive Terminal UI (ratatui + crossterm), 29 tabs | [tui.md](tui.md) |

### Notifications & Utilities

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `notify/` | `crates/slapper/src/notify/` | Webhook notifications: Slack, Discord, Teams, email | - |
| `logging/` | `crates/slapper/src/logging/` | Structured logging with tracing | - |
| `utils/` | `crates/slapper/src/utils/` | Circuit breaker, formatting, rate limiting, regex caching | - |
| `constants.rs` | `crates/slapper/src/constants.rs` | Centralized magic number elimination | - |

---

## Module Interconnections

### Typical Assessment Flow

```
┌──────────┐     ┌───────────┐     ┌───────────┐     ┌─────────┐
│  Recon   │────▶│  Scanner  │────▶│ Endpoint  │────▶│  Fuzzer │
│ (intel)  │     │ (ports)   │     │  (paths)  │     │  (bugs) │
└──────────┘     └───────────┘     └───────────┘     └─────────┘
     │               │                  │                  │
     │               │                  │                  │
     ▼               ▼                  ▼                  ▼
┌──────────┐     ┌───────────┐     ┌───────────┐     ┌─────────┐
│   WAF    │◀────│  Pipeline │────▶│  LoadTest │     │   AI    │
│ (bypass) │     │(orchestra)│     │   (perf)  │     │ (adapt) │
└──────────┘     └───────────┘     └───────────┘     └─────────┘
                        │
                        ▼
                  ┌───────────┐
                  │  Output   │
                  │ (reports) │
                  └───────────┘
```

### Key Dependencies

| From | To | Purpose |
|------|----|---------|
| `cli/` | `commands/` | Parsed args → handler dispatch |
| `commands/handlers/` | `pipeline/` | Pipeline execution |
| `pipeline/` | `scanner/`, `fuzzer/`, `recon/`, `waf/`, `loadtest/` | Stage orchestration |
| `scanner/` | `waf/` | WAF detection during discovery |
| `fuzzer/` | `waf/` | Bypass detection during fuzzing |
| `fuzzer/` | `ai/` | Adaptive payload generation |
| `agent/` | `tool/` | Autonomous scanning via tool abstraction |
| `tool/` | All modules | MCP/REST API exposure |
| `output/` | All modules | Report generation from any findings |

---

## Design Patterns

### 1. Feature-Gated Compilation

```rust
#[cfg(feature = "stress-testing")]  // Raw sockets, IP spoofing, DoS
#[cfg(feature = "packet-inspection")]  // Packet capture
#[cfg(feature = "python-plugins")]  // Python plugins via pyo3
#[cfg(feature = "nse")]  // Nmap Scripting Engine
#[cfg(feature = "ai-integration")]  // AI planner, autonomous agents
#[cfg(feature = "full")]  // All features
```

### 2. Builder Pattern

```rust
Pipeline::from_args(args)
FuzzEngine::new(args)
LoadTestRunner::new(url, total, concurrency, timeout)
SmartWafBypass::new(client, config)
SarifBuilder::new()
```

### 3. Trait-Based Tool Abstraction

```rust
pub trait SecurityTool: Send + Sync {
    fn name(&self) -> &str;
    async fn execute(&self, target: &Target, args: Value) -> Result<Value>;
    fn capabilities(&self) -> Vec<Capability>;
}
```

`ToolRegistry` manages dynamic registration for MCP/REST/gRPC exposure.

### 4. Scope Enforcement

`Scope` struct in `config/scope.rs` enforces target restrictions:
- `is_target_allowed(target)` - checks allow/block rules
- `validate_url(url)` - validates URL's host
- `is_port_allowed(port)` - port allowlist/blocklist
- **Private IP blocking**: Direct IPs (`127.0.0.1`, `169.254.169.254`) blocked via `TargetScope::parse()`

### 5. Performance Collections

Use `rustc_hash::FxHashMap` and `FxHashSet` instead of std collections for hot paths:
- Scanner results, fuzzing state, recon caches, WAF signatures
- All 14 architecture documents confirm this pattern

### 6. Async-First Concurrency

Built on `tokio` for high concurrency:
- `tokio::join!` for parallel recon tasks
- `tokio::sync::Semaphore` for concurrency control
- `JoinSet` for load test workers
- `DashMap` for lock-free concurrent collection

---

## Workspace Crates

| Crate | Location | Purpose |
|-------|----------|---------|
| `slapper` | `crates/slapper/` | Core toolkit |
| `slapper-plugin` | `crates/slapper-plugin/` | Python plugin system via `pyo3` |
| `slapper-nse` | `crates/slapper-nse/` | Full Nmap Scripting Engine (NSE) via `mlua` |
| `slapper-ruby` | `crates/slapper-ruby/` | Ruby bridge and Metasploit RPC integration |

**NSE Integration**: Full Lua VM with 164 NSE-style library modules (stdnse, nmap, http, socket, dns, ssl, ssh, mysql, postgres, redis, mongodb, ldap, snmp, smb, vulns, etc.). See [plugins_nse.md](plugins_nse.md).

---

## Key Data Types

| Type | Location | Purpose |
|------|----------|---------|
| `SlapperConfig` | `config/settings.rs` | Main configuration struct |
| `Scope` | `config/scope.rs` | Target allow/block enforcement |
| `Severity` | `types.rs` | Unified severity (Critical, High, Medium, Low, Info) |
| `PayloadType` | `fuzzer/payloads/mod.rs` | 31 payload categories |
| `SlapperError` | `error/mod.rs` | Unified error via `thiserror` |
| `TabError` | `tui/app/tab_error.rs` | Structured error with recovery categories |
| `SecurityTool` | `tool/traits.rs` | Trait for tool abstraction |
| `ToolRegistry` | `tool/registry.rs` | Dynamic tool registration |
| `AiClient` | `ai/client.rs` | LLM client with provider abstraction |
| `SmartWafBypass` | `ai/waf_bypass.rs` | WAF bypass with knowledge base |
| `AiPlanner` | `ai/planner.rs` | AI-driven execution planning |
| `FuzzEngine` | `fuzzer/engine/mod.rs` | Core fuzzing orchestration |
| `PipelineContext` | `pipeline/context.rs` | Inter-stage data passing |
| `Stage` | `pipeline/stage.rs` | Pipeline stage enum (11 profiles) |
| `WafDetector` | `waf/detector/mod.rs` | 34 WAF product detection |
| `LoadTestRunner` | `loadtest/runner.rs` | HTTP benchmarking |

---

## Index of Detailed Documentation

| Document | Modules Covered |
|----------|-----------------|
| [ai_agents.md](ai_agents.md) | `ai/`, `agent/`, `tool/` - AI/LLM integration and autonomous agents |
| [cli_commands.md](cli_commands.md) | `cli/`, `commands/` - CLI parsing and command dispatch |
| [config.md](config.md) | `config/` - Configuration system and scope enforcement |
| [distributed.md](distributed.md) | `distributed/` - Worker/coordinator cluster architecture |
| [fuzzer.md](fuzzer.md) | `fuzzer/` - Fuzzing engine and payload types |
| [loadtest.md](loadtest.md) | `loadtest/` - HTTP load testing and benchmarking |
| [networking.md](networking.md) | `packet/`, `stress/` - Packet capture/crafting and stress testing |
| [output.md](output.md) | `output/` - Reporting formats and deduplication |
| [pipeline.md](pipeline.md) | `pipeline/` - Stage orchestration and session management |
| [plugins_nse.md](plugins_nse.md) | `slapper-plugin/`, `slapper-nse/`, `slapper-ruby/` - Plugin systems and NSE |
| [recon.md](recon.md) | `recon/` - Reconnaissance modules and runner |
| [scanner.md](scanner.md) | `scanner/` - Port scanning and fingerprinting |
| [tui.md](tui.md) | `tui/` - Terminal UI, 29 tabs, components, workers |
| [waf.md](waf.md) | `waf/` - WAF detection and bypass techniques |

---

## Undocumented Modules

The following modules lack dedicated architecture documentation (candidates for future deep dives):

| Module | Purpose | Priority |
|--------|---------|----------|
| `auth/` | Authentication testing (brute force, credential stuffing, MFA, SSH, SMTP) | Medium |
| `browser/` | Headless Chrome for DOM XSS and SPA crawling | Medium |
| `compliance/` | Compliance scanning (OWASP, PCI-DSS, HIPAA, SOC2) | Low |
| `container/` | Kubernetes and Docker security checks | Medium |
| `hunt/` | Intelligent vulnerability hunting | Medium |
| `integrations/` | Issue tracker connectors (Jira, GitHub, GitLab) | Low |
| `notify/` | Webhook notifications (Slack, Discord, Teams, email) | Low |
| `proxy/` | SOCKS/HTTP/Tor proxy pool with health checks | Medium |
| `storage/` | SQLx-based persistence for findings, history, configuration | Medium |
| `supply_chain/` | SBOM generation and analysis | Low |
| `vuln/` | Vulnerability triage and lifecycle management | Medium |
| `websocket/` | WebSocket security testing | Medium |
| `wireless/` | Wireless security testing | Low |
| `workflow/` | Finding management and SLA tracking | Low |

---

## Quick Reference

| Metric | Value |
|-------|-------|
| Total modules | 41 modules in `crates/slapper/src/` |
| Architecture docs | 14 documents in `architecture/` |
| Workspace crates | 4 (slapper, slapper-plugin, slapper-nse, slapper-ruby) |
| Payload types | 31 (defined in `fuzzer/payloads/mod.rs`) |
| WAF products | 34 (defined in `waf/data/patterns.rs`) |
| TUI tabs | 29 |
| Pipeline profiles | 11 |
| Feature flags | 20+ |
| NSE libraries | 164 (in `slapper-nse/src/libraries/`) |

---

*This overview serves as the entry point to the architecture documentation. Each linked document provides a deep dive into a specific component or domain.*