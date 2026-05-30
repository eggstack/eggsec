# Slapper Architecture Overview

Slapper is a Rust-native security assessment and defense-validation engine designed for scoped, repeatable security testing of live systems.

**Quick Facts:**
- 39 modules in `crates/slapper/src/`
- 526 source files
- 1324 base tests (1469+ with full features)
- 30 payload types for fuzzing
- 28 TUI tabs
- 34 WAF products detected

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
              ▼                 ▼                  ▼                ▼
       ┌─────────────┐   ┌─────────────┐   ┌─────────────┐  ┌─────────────┐
       │    waf/     │   │  loadtest/  │   │   output/    │  │ distributed/│
       │  Detection  │   │  Benchmark  │   │   Reports    │  │   Cluster   │
       │   Bypass    │   │             │   │              │  │             │
       └─────────────┘   └─────────────┘   └─────────────┘  └─────────────┘
```

---

## Module Index

Each module links to a detailed `.md` file in this directory. Modules without links are documented inline or lack dedicated deep-dive documentation.

### Entry Point

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `main.rs` | `crates/slapper/src/main.rs` | Binary entry, CLI parsing, config loading, command dispatch | [cli_commands.md](cli_commands.md) |

### Core Infrastructure

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `cli/` | `crates/slapper/src/cli/` | Clap-based argument parsing; `Commands` enum with 35+ variants | [cli_commands.md](cli_commands.md) |
| `commands/` | `crates/slapper/src/commands/` | Command dispatch via `handle_command()`; handler implementations | [cli_commands.md](cli_commands.md) |
| `config/` | `crates/slapper/src/config/` | TOML/YAML config loading, scope enforcement, profile management | [config.md](config.md) |
| `types.rs` | `crates/slapper/src/types.rs` | Canonical `Severity` enum (Critical/High/Medium/Low/Info) | - |
| `error/` | `crates/slapper/src/error/` | Core error types (`SlapperError`, `Result`) via `thiserror` | - |
| `constants.rs` | `crates/slapper/src/constants.rs` | Centralized magic numbers | - |
| `macros.rs` | `crates/slapper/src/macros.rs` | Utility macros | - |
| `logging/` | `crates/slapper/src/logging/` | Structured logging with `tracing` | - |
| `utils/` | `crates/slapper/src/utils/` | Circuit breaker, formatting, rate limiting, regex caching | - |

### Security Testing

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `scanner/` | `crates/slapper/src/scanner/` | Port scanning (TCP/SYN/UDP), service fingerprinting, endpoint discovery | [scanner.md](scanner.md) |
| `fuzzer/` | `crates/slapper/src/fuzzer/` | Security fuzzing engine with 31 payload types, grammar-based generation | [fuzzer.md](fuzzer.md) |
| `recon/` | `crates/slapper/src/recon/` | Passive/active recon: DNS, WHOIS, SSL, subdomain discovery, CVE mapping | [recon.md](recon.md) |
| `waf/` | `crates/slapper/src/waf/` | WAF detection (34 products), evasion-resistance testing, bypass | [waf.md](waf.md) |
| `auth/` | `crates/slapper/src/auth/` | Authentication testing: brute force, credential stuffing, MFA bypass | - |
| `browser/` | `crates/slapper/src/browser/` | Headless Chrome for DOM XSS and SPA crawling | - |
| `websocket/` | `crates/slapper/src/websocket/` | WebSocket security testing | - |
| `wireless/` | `crates/slapper/src/wireless/` | Wireless security testing | - |
| `hunt/` | `crates/slapper/src/hunt/` | Intelligent vulnerability hunting | - |
| `nse_tool/` | `crates/slapper/src/nse_tool/` | Optional NSE compatibility adapter | [nse_integration.md](nse_integration.md) |
| `auth_context/` | `crates/slapper/src/auth_context/` | Multi-role auth contexts with env variable interpolation | - |
| `api_schema/` | `crates/slapper/src/api_schema/` | OpenAPI v3 schema import for type-aware fuzzing | - |

### Assessment Orchestration

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `pipeline/` | `crates/slapper/src/pipeline/` | Stage-based chained assessment with pause/resume | [pipeline.md](pipeline.md) |
| `distributed/` | `crates/slapper/src/distributed/` | Worker/coordinator cluster architecture | [distributed.md](distributed.md) |
| `workflow/` | `crates/slapper/src/workflow/` | Finding management and SLA tracking | - |
| `vuln/` | `crates/slapper/src/vuln/` | Vulnerability triage and lifecycle management | - |

### AI & Agent Orchestration

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `ai/` | `crates/slapper/src/ai/` | AI/LLM integration for adaptive fuzzing and planning | [ai_agents.md](ai_agents.md) |
| `agent/` | `crates/slapper/src/agent/` | Agent orchestration with scheduled scans, memory, skills | [ai_agents.md](ai_agents.md) |
| `tool/` | `crates/slapper/src/tool/` | `SecurityTool` trait, `ToolRegistry`, MCP/REST/gRPC integration | [ai_agents.md](ai_agents.md) |

### Performance & Load

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `loadtest/` | `crates/slapper/src/loadtest/` | HTTP load testing with HDR histogram metrics | [loadtest.md](loadtest.md) |
| `stress/` | `crates/slapper/src/stress/` | Controlled SYN/UDP/HTTP/ICMP flood testing | [networking.md](networking.md) |

### Networking & Packets

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `packet/` | `crates/slapper/src/packet/` | Packet capture (libpcap), crafting (pnet), parsing | [networking.md](networking.md) |
| `proxy/` | `crates/slapper/src/proxy/` | SOCKS4/5, HTTP, HTTPS, Tor proxy pool with health checks | - |

### Data & Reporting

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `output/` | `crates/slapper/src/output/` | Report generation: JSON, HTML, CSV, SARIF, JUnit XML, PDF | [output.md](output.md) |
| `findings/` | `crates/slapper/src/findings/` | Canonical finding schema, fingerprinting, evidence redaction | - |
| `diff/` | `crates/slapper/src/diff/` | Differential scan comparison | - |
| `storage/` | `crates/slapper/src/storage/` | SQLx-based persistence for findings and history | - |

### Integration & Compliance

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `integrations/` | `crates/slapper/src/integrations/` | Jira, GitHub, GitLab connectors | - |
| `compliance/` | `crates/slapper/src/compliance/` | OWASP, PCI-DSS, HIPAA, SOC2 scanning and reporting | - |
| `container/` | `crates/slapper/src/container/` | Kubernetes and Docker security checks | - |
| `supply_chain/` | `crates/slapper/src/supply_chain/` | SBOM generation (CycloneDX, SPDX) | - |

### User Interface

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `tui/` | `crates/slapper/src/tui/` | Interactive Terminal UI (ratatui + crossterm), 28 tabs | [tui.md](tui.md) |

### Notifications & Utilities

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `notify/` | `crates/slapper/src/notify/` | Webhook notifications: Slack, Discord, Teams, email | - |

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
| `recon/` | `scanner/` | Recon feeds targets to scanner |
| `proxy/` | `stress/`, `scanner/`, `fuzzer/` | Proxy rotation for all outbound traffic |

---

## Feature Flags

### Always Compiled (default)

`auth`, `cli`, `commands`, `config`, `constants`, `distributed`, `error`, `fuzzer`, `loadtest`, `logging`, `notify`, `output`, `pipeline`, `proxy`, `recon`, `scanner`, `tui`, `types`, `utils`, `waf`

### Feature-Gated Modules

| Feature | Module(s) | Description |
|---------|-----------|-------------|
| `stress-testing` | `stress/`, `packet/` | SYN/UDP/ICMP floods, raw sockets, IP spoofing |
| `packet-inspection` | `packet/` | Live packet capture (libpcap), traceroute |
| `rest-api` | `tool/`, `agent/` | REST API + MCP for AI agents |
| `grpc-api` | `tool/` | gRPC API server |
| `nse` | `slapper-nse` (re-exported as `nse`) | Nmap Scripting Engine support |
| `nse-ssh2` | `slapper-nse` | NSE with SSH2/libssh2 support |
| `nse-sandbox` | `slapper-nse` | NSE sandbox (restricts dangerous Lua ops) |
| `ai-integration` | `ai/` | AI/LLM analysis, adaptive fuzzing, WAF bypass |
| `headless-browser` | `browser/` | DOM XSS and SPA crawling |
| `database` | `storage/` | SQLx-based persistence (Postgres) |
| `container` | `container/` | Kubernetes and Docker security checks |
| `compliance` | `compliance/` | OWASP, PCI-DSS, HIPAA, SOC2 scanning |
| `external-integrations` | `integrations/` | Jira, GitHub, GitLab connectors |
| `finding-workflow` | `workflow/` | Finding lifecycle management |
| `vuln-management` | `vuln/` | Vulnerability triage and prioritization |
| `websocket` | `websocket/` | WebSocket security testing |
| `advanced-hunting` | `hunt/` | Intelligent vulnerability hunting |
| `sbom` | `supply_chain/` | SBOM generation |
| `pdf` | - | PDF report generation |
| `wireless` | `wireless/` | Wireless security testing |
| `api-schema` | - | OpenAPI v3 schema-based fuzzing |
| **`full`** | Everything | All features combined |

---

## Workspace Crates

| Crate | Location | Description |
|-------|----------|-------------|
| `slapper` | `crates/slapper/` | Core toolkit — all security modules, CLI, TUI |
| `slapper-nse` | `crates/slapper-nse/` | Nmap Scripting Engine (NSE) via `mlua` — 164+ library modules |

---

## Key Data Types

| Type | Location | Purpose |
|------|----------|---------|
| `SlapperConfig` | `config/settings.rs` | Main configuration struct |
| `Scope` | `config/scope.rs` | Target allow/block enforcement |
| `Severity` | `types.rs` | Unified severity (Critical, High, Medium, Low, Info) |
| `SlapperError` | `error/mod.rs` | Unified error via `thiserror` |
| `PayloadType` | `fuzzer/payloads/mod.rs` | 31 payload categories |
| `SecurityTool` | `tool/traits.rs` | Trait for tool abstraction |
| `ToolRegistry` | `tool/registry.rs` | Dynamic tool registration |
| `AiClient` | `ai/client.rs` | LLM client with provider abstraction |
| `FuzzEngine` | `fuzzer/engine/mod.rs` | Core fuzzing orchestration |
| `PipelineContext` | `pipeline/context.rs` | Inter-stage data passing |
| `WafDetector` | `waf/detector/mod.rs` | 34 WAF product detection |
| `CircuitBreaker` | `utils/circuit_breaker.rs` | Fault tolerance pattern |

---

## Detailed Documentation Index

| Document | Modules Covered | Description |
|----------|-----------------|-------------|
| [ai_agents.md](ai_agents.md) | `ai/`, `agent/`, `tool/` | AI/LLM integration, agent orchestration, MCP tool exposure |
| [cli_commands.md](cli_commands.md) | `cli/`, `commands/` | CLI parsing, command dispatch, handler patterns |
| [config.md](config.md) | `config/` | Configuration system, scope enforcement, profiles |
| [distributed.md](distributed.md) | `distributed/` | Worker/coordinator cluster architecture |
| [fuzzer.md](fuzzer.md) | `fuzzer/` | Fuzzing engine, payloads, detection, grammar |
| [loadtest.md](loadtest.md) | `loadtest/` | HTTP load testing, HDR histogram metrics |
| [networking.md](networking.md) | `packet/`, `stress/` | Packet capture/crafting and stress testing |
| [output.md](output.md) | `output/` | Reporting formats, deduplication, SARIF/JUnit |
| [pipeline.md](pipeline.md) | `pipeline/` | Stage orchestration, profiles, session management |
| [nse_integration.md](nse_integration.md) | `slapper-nse/` | NSE/Lua integration |
| [recon.md](recon.md) | `recon/` | Reconnaissance modules and runner |
| [scanner.md](scanner.md) | `scanner/` | Port scanning, fingerprinting, endpoint discovery |
| [tui.md](tui.md) | `tui/` | Terminal UI, 28 tabs, components, workers |
| [waf.md](waf.md) | `waf/` | WAF detection and bypass techniques |

---

## Architectural Principles

1. **Scope enforcement is a core invariant**, not a CLI convenience.
2. **Slapper-native Rust probes are the curated core.** NSE and other compatibility layers are optional.
3. **NSE is a compatibility and knowledge layer**, not the architectural center.
4. **Low-level packet/protocol testing belongs in controlled defense-lab workflows.**
5. **Outputs should be structured and suitable for humans, CI, and agents.**
6. **Intrusive or stress behavior must be explicit and budgeted.**
7. **Profiles should compile into clear probe plans** with documented intent and risk.

---

## Undocumented Modules (Candidates for Deep Dive)

These modules lack dedicated architecture documentation files:

| Module | Purpose |
|--------|---------|
| `auth/` | Authentication testing (brute force, credential stuffing, MFA, SSH, SMTP) |
| `browser/` | Headless Chrome for DOM XSS and SPA crawling |
| `compliance/` | Compliance scanning (OWASP, PCI-DSS, HIPAA, SOC2) |
| `container/` | Kubernetes and Docker security checks |
| `findings/` | Canonical finding schema, fingerprinting, evidence redaction |
| `hunt/` | Intelligent vulnerability hunting |
| `integrations/` | Issue tracker connectors (Jira, GitHub, GitLab) |
| `notify/` | Webhook notifications (Slack, Discord, Teams, email) |
| `proxy/` | SOCKS/HTTP/Tor proxy pool with health checks |
| `storage/` | SQLx-based persistence for findings, history, configuration |
| `supply_chain/` | SBOM generation and analysis |
| `vuln/` | Vulnerability triage and lifecycle management |
| `websocket/` | WebSocket security testing |
| `workflow/` | Finding management and SLA tracking |
| `wireless/` | Wireless security testing |

---

*This overview serves as the entry point to the architecture documentation. Each linked document provides a deep dive into a specific component or domain.*
