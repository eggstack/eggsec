# Slapper Architecture Overview

Slapper is a high-performance, async-first security testing toolkit built in Rust. It provides a full assessment pipeline from reconnaissance through exploitation, with autonomous AI-driven agents, distributed cluster scanning, and a comprehensive TUI.

**Quick Facts:**
- 41 modules in `crates/slapper/src/`
- 743 source files
- 1324 base tests (1469+ with full features)
- 31 payload types for fuzzing
- 29 TUI tabs
- 34 WAF products detected
- 11 pipeline profiles
- 4 workspace crates
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
             ▼                 ▼                  ▼                ▼
      ┌─────────────┐   ┌─────────────┐   ┌─────────────┐  ┌─────────────┐
      │    waf/     │   │  loadtest/  │   │   output/    │  │ distributed/│
      │  Detection  │   │  Benchmark  │   │   Reports    │  │   Cluster   │
      │   Bypass    │   │             │   │              │  │             │
      └─────────────┘   └─────────────┘   └─────────────┘  └─────────────┘
```

---

## Module Index

Each major area links to a detailed `.md` file in this directory. Modules without links are candidates for future documentation.

### Core Infrastructure

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `cli/` | `crates/slapper/src/cli/` | Clap-based argument parsing; `Commands` enum with 35+ variants | [cli_commands.md](cli_commands.md) |
| `commands/` | `crates/slapper/src/commands/` | Command dispatch via `handle_command()`; handler implementations | [cli_commands.md](cli_commands.md) |
| `config/` | `crates/slapper/src/config/` | TOML/YAML config loading, scope enforcement, profile management | [config.md](config.md) |
| `types.rs` | `crates/slapper/src/types.rs` | Canonical `Severity` enum (Critical/High/Medium/Low/Info) | - |
| `error/` | `crates/slapper/src/error/` | Core error types (`SlapperError`, `Result`) via `thiserror` | - |
| `constants.rs` | `crates/slapper/src/constants.rs` | Centralized magic number elimination | - |
| `macros.rs` | `crates/slapper/src/macros.rs` | Utility macros | - |
| `logging/` | `crates/slapper/src/logging/` | Structured logging with `tracing` | - |

### Security Testing

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `scanner/` | `crates/slapper/src/scanner/` | Port scanning (TCP/SYN/UDP), service fingerprinting (20+ protocols), endpoint discovery, timing templates (T0-T5), ICMP/UDP probing | [scanner.md](scanner.md) |
| `fuzzer/` | `crates/slapper/src/fuzzer/` | Security fuzzing engine with 31 payload types, mutation, grammar-based generation, detection algorithms (error/boolean/time-based), API schema fuzzing | [fuzzer.md](fuzzer.md) |
| `recon/` | `crates/slapper/src/recon/` | Passive/active recon: DNS, WHOIS, SSL, subdomain discovery, CVE mapping, cloud asset discovery, CORS, JS analysis, Wayback, threat intel, email harvesting, git secrets | [recon.md](recon.md) |
| `waf/` | `crates/slapper/src/waf/` | WAF detection (34 products), header manipulation, HTTP smuggling, evasion techniques, knowledge-base-driven bypass | [waf.md](waf.md) |
| `auth/` | `crates/slapper/src/auth/` | Authentication testing: brute force, credential stuffing, MFA bypass, SSH/SMTP testing | - |
| `browser/` | `crates/slapper/src/browser/` | Headless Chrome for DOM XSS and SPA crawling (feature: `headless-browser`) | - |
| `websocket/` | `crates/slapper/src/websocket/` | WebSocket security testing (feature: `websocket`) | - |
| `wireless/` | `crates/slapper/src/wireless/` | Wireless security testing (feature: `wireless`) | - |
| `hunt/` | `crates/slapper/src/hunt/` | Intelligent vulnerability hunting (feature: `advanced-hunting`) | - |
| `nse_tool/` | `crates/slapper/src/nse_tool/` | NSE tool integration (feature: `nse` + `tool-api`) | [plugins_nse.md](plugins_nse.md) |
| `auth_context/` | `crates/slapper/src/auth_context/` | Multi-role auth contexts with env variable interpolation | [AUTH_CONTEXT.md](../docs/AUTH_CONTEXT.md) |
| `api_schema/` | `crates/slapper/src/api_schema/` | OpenAPI v3 schema import for type-aware fuzzing (feature: `api-schema`) | [API_TESTING.md](../docs/API_TESTING.md) |

### Assessment Orchestration

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `pipeline/` | `crates/slapper/src/pipeline/` | Stage-based chained assessment with pause/resume, 11 profiles (full, web, api, infrastructure, etc.), inter-stage context passing | [pipeline.md](pipeline.md) |
| `distributed/` | `crates/slapper/src/distributed/` | Worker/coordinator cluster architecture for parallel scanning | [distributed.md](distributed.md) |
| `workflow/` | `crates/slapper/src/workflow/` | Finding management and SLA tracking (feature: `finding-workflow`) | - |
| `vuln/` | `crates/slapper/src/vuln/` | Vulnerability triage and lifecycle management (feature: `vuln-management`) | - |

### AI & Autonomous Agents

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `ai/` | `crates/slapper/src/ai/` | AI/LLM integration: adaptive fuzzing, WAF bypass suggestions, payload generation, execution planning, TTL caching | [ai_agents.md](ai_agents.md) |
| `agent/` | `crates/slapper/src/agent/` | Autonomous security agent with scheduled scans, memory, alert routing, skill system (feature: `rest-api`) | [ai_agents.md](ai_agents.md) |
| `tool/` | `crates/slapper/src/tool/` | `SecurityTool` trait, `ToolRegistry`, MCP/REST/gRPC integration for AI-driven tool exposure (feature: `tool-api`) | [ai_agents.md](ai_agents.md) |

### Performance & Load

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `loadtest/` | `crates/slapper/src/loadtest/` | HTTP load testing with HDR histogram metrics, connection pooling, configurable concurrency | [loadtest.md](loadtest.md) |
| `stress/` | `crates/slapper/src/stress/` | SYN/UDP/HTTP/ICMP flood testing, proxy management, raw sockets (feature: `stress-testing`) | [networking.md](networking.md) |

### Networking & Packets

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `packet/` | `crates/slapper/src/packet/` | Packet capture (libpcap), crafting (pnet), parsing (feature: `packet-inspection` or `stress-testing`) | [networking.md](networking.md) |
| `proxy/` | `crates/slapper/src/proxy/` | SOCKS4/5, HTTP, HTTPS, Tor proxy pool with health checks and rotation | - |

### Data & Reporting

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `output/` | `crates/slapper/src/output/` | Report generation: JSON, HTML, CSV, SARIF (CI/CD), JUnit XML, PDF, Markdown, deduplication | [output.md](output.md) |
| `findings/` | `crates/slapper/src/findings/` | Canonical finding schema, fingerprinting, evidence redaction | [FINDINGS_SCHEMA.md](../docs/FINDINGS_SCHEMA.md) |
| `diff/` | `crates/slapper/src/diff/` | Differential scan comparison (new/resolved/changed/persisting findings) | [BASELINES_AND_DIFFS.md](../docs/BASELINES_AND_DIFFS.md) |
| `storage/` | `crates/slapper/src/storage/` | SQLx-based persistence for findings, history, configuration (feature: `database`) | - |

### Integration & Compliance

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `integrations/` | `crates/slapper/src/integrations/` | Jira, GitHub, GitLab connectors (feature: `external-integrations`) | - |
| `compliance/` | `crates/slapper/src/compliance/` | OWASP, PCI-DSS, HIPAA, SOC2 scanning and reporting (feature: `compliance`) | - |
| `container/` | `crates/slapper/src/container/` | Kubernetes and Docker security checks (feature: `container`) | - |
| `supply_chain/` | `crates/slapper/src/supply_chain/` | SBOM generation and analysis (feature: `sbom`) | - |

### User Interface

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `tui/` | `crates/slapper/src/tui/` | Interactive Terminal UI (ratatui + crossterm), 29 tabs, session persistence, quick-switch, overlays | [tui.md](tui.md) |

### Notifications & Utilities

| Module | Source | Description | Doc |
|--------|--------|-------------|-----|
| `notify/` | `crates/slapper/src/notify/` | Webhook notifications: Slack, Discord, Teams, email | - |
| `utils/` | `crates/slapper/src/utils/` | Circuit breaker, formatting, rate limiting, regex caching (LRU) | - |

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
| `notify/` | All modules | Alert delivery on findings |
| `storage/` | All modules | Persistent storage of results |

---

## Feature Flags

Slapper uses Cargo feature flags extensively to control compilation. Modules are conditionally compiled via `#[cfg(feature = "...")]` in `lib.rs`.

### Always Compiled (default)

These modules are always available without any feature flags:

`auth`, `cli`, `commands`, `config`, `constants`, `distributed`, `error`, `fuzzer`, `loadtest`, `logging`, `notify`, `output`, `pipeline`, `proxy`, `recon`, `scanner`, `tui`, `types`, `utils`, `waf`

### Feature-Gated Modules

| Feature | Module(s) | Description |
|---------|-----------|-------------|
| `stress-testing` | `stress/`, `packet/` | SYN/UDP/ICMP floods, raw sockets, IP spoofing |
| `packet-inspection` | `packet/` | Live packet capture (libpcap), traceroute |
| `rest-api` | `tool/`, `agent/` | REST API + MCP for AI agents (`axum`, `tower`) |
| `grpc-api` | `tool/` | gRPC API server (`tonic`, `prost`) |
| `ws-api` | - | WebSocket pub/sub API server |
| `nse` | `slapper-nse` (re-exported as `nse`) | Nmap Scripting Engine support via `mlua` |
| `nse-ssh2` | `slapper-nse` | NSE with SSH2/libssh2 support |
| `nse-sandbox` | `slapper-nse` | NSE sandbox (restricts dangerous Lua ops) |
| `ai-integration` | `ai/` | AI/LLM analysis, adaptive fuzzing, WAF bypass |
| `headless-browser` | `browser/` | DOM XSS and SPA crawling via `headless_chrome` |
| `database` | `storage/` | SQLx-based persistence (Postgres) |
| `container` | `container/` | Kubernetes and Docker security checks |
| `cloud` | - | Cloud security scanning (AWS, GCP, Azure) |
| `compliance` | `compliance/` | OWASP, PCI-DSS, HIPAA, SOC2 scanning |
| `external-integrations` | `integrations/` | Jira, GitHub, GitLab connectors |
| `finding-workflow` | `workflow/` | Finding lifecycle management and SLA tracking |
| `vuln-management` | `vuln/` | Vulnerability triage and prioritization |
| `websocket` | `websocket/` | WebSocket security testing |
| `advanced-hunting` | `hunt/` | Intelligent vulnerability hunting |
| `sbom` | `supply_chain/` | SBOM generation (`cyclonedx-bom`, `spdx`) |
| `git-secrets` | - | Git repository secrets scanning |
| `pdf` | - | PDF report generation (`printpdf`) |
| `wireless` | `wireless/` | Wireless security testing |
| `api-schema` | - | OpenAPI v3 schema-based fuzzing |
| **`full`** | Everything | All features combined |

---

## Workspace Crates

| Crate | Location | Purpose |
|-------|----------|---------|
| `slapper` | `crates/slapper/` | Core toolkit — all security modules, CLI, TUI |
| `slapper-nse` | `crates/slapper-nse/` | Full Nmap Scripting Engine (NSE) via `mlua` — 164+ NSE-style library modules |

**NSE Integration**: Full Lua VM with 164+ NSE-style library modules (stdnse, nmap, http, socket, dns, ssl, ssh, mysql, postgres, redis, mongodb, ldap, snmp, smb, vulns, etc.). See [plugins_nse.md](plugins_nse.md).

---

## Key Data Types

| Type | Location | Purpose |
|------|----------|---------|
| `SlapperConfig` | `config/settings.rs` | Main configuration struct |
| `Scope` | `config/scope.rs` | Target allow/block enforcement |
| `Severity` | `types.rs` | Unified severity (Critical, High, Medium, Low, Info) |
| `SlapperError` | `error/mod.rs` | Unified error via `thiserror` |
| `TabError` | `tui/app/tab_error.rs` | Structured error with recovery categories |
| `PayloadType` | `fuzzer/payloads/mod.rs` | 31 payload categories |
| `SecurityTool` | `tool/traits.rs` | Trait for tool abstraction |
| `ToolRegistry` | `tool/registry.rs` | Dynamic tool registration |
| `AiClient` | `ai/client.rs` | LLM client with provider abstraction |
| `AiPlanner` | `ai/planner.rs` | AI-driven execution planning |
| `SmartWafBypass` | `ai/waf_bypass.rs` | WAF bypass with knowledge base |
| `AiCache` | `ai/cache.rs` | TTL cache for AI responses |
| `FuzzEngine` | `fuzzer/engine/mod.rs` | Core fuzzing orchestration |
| `PipelineContext` | `pipeline/context.rs` | Inter-stage data passing |
| `Stage` | `pipeline/stage.rs` | Pipeline stage enum (11 profiles) |
| `WafDetector` | `waf/detector/mod.rs` | 34 WAF product detection |
| `LoadTestRunner` | `loadtest/runner.rs` | HTTP benchmarking |
| `CircuitBreaker` | `utils/circuit_breaker.rs` | Fault tolerance pattern |
| `SensitiveString` | `config/sensitive.rs` | Zeroized credential wrapper |

---

## Design Patterns

### 1. Feature-Gated Compilation

Modules are conditionally compiled using `#[cfg(feature = "...")]`. This keeps the default binary small while allowing full capabilities with `--features full`. The `lib.rs` module tree uses both `pub mod` (when enabled) and `mod` with `#[allow(dead_code)]` (when disabled) to maintain compilation without warnings.

### 2. Builder Pattern

Used extensively for constructing complex objects:

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

`ToolRegistry` manages dynamic registration for MCP/REST/gRPC exposure, allowing AI agents to discover and invoke security tools programmatically.

### 4. Scope Enforcement

`Scope` struct in `config/scope.rs` enforces target restrictions:
- `is_target_allowed(target)` — checks allow/block rules
- `validate_url(url)` — validates URL's host
- `is_port_allowed(port)` — port allowlist/blocklist
- **Private IP blocking**: Direct IPs (`127.0.0.1`, `169.254.169.254`) blocked via `TargetScope::parse()`

### 5. Performance Collections

Use `rustc_hash::FxHashMap` and `FxHashSet` instead of std collections for hot paths:
- Scanner results, fuzzing state, recon caches, WAF signatures
- All architecture documents confirm this pattern

### 6. Async-First Concurrency

Built on `tokio` for high concurrency:
- `tokio::join!` for parallel recon tasks
- `tokio::sync::Semaphore` for concurrency control
- `JoinSet` for load test workers
- `DashMap` for lock-free concurrent collection
- `tokio::time::timeout` wrappers on all I/O-bound operations

---

## Index of Detailed Documentation

| Document | Modules Covered | Description |
|----------|-----------------|-------------|
| [ai_agents.md](ai_agents.md) | `ai/`, `agent/`, `tool/` | AI/LLM integration, autonomous agents, MCP tool exposure |
| [cli_commands.md](cli_commands.md) | `cli/`, `commands/` | CLI parsing, command dispatch, handler patterns |
| [config.md](config.md) | `config/` | Configuration system, scope enforcement, profiles |
| [distributed.md](distributed.md) | `distributed/` | Worker/coordinator cluster architecture |
| [fuzzer.md](fuzzer.md) | `fuzzer/` | Fuzzing engine, payloads, detection, grammar |
| [loadtest.md](loadtest.md) | `loadtest/` | HTTP load testing, HDR histogram metrics |
| [networking.md](networking.md) | `packet/`, `stress/` | Packet capture/crafting and stress testing |
| [output.md](output.md) | `output/` | Reporting formats, deduplication, SARIF/JUnit |
| [pipeline.md](pipeline.md) | `pipeline/` | Stage orchestration, profiles, session management |
| [plugins_nse.md](plugins_nse.md) | `slapper-nse/` | NSE integration |
| [recon.md](recon.md) | `recon/` | Reconnaissance modules and runner |
| [scanner.md](scanner.md) | `scanner/` | Port scanning, fingerprinting, endpoint discovery |
| [tui.md](tui.md) | `tui/` | Terminal UI, 29 tabs, components, workers |
| [waf.md](waf.md) | `waf/` | WAF detection and bypass techniques |

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
| Workspace crates | 2 (slapper, slapper-nse) |
| Payload types | 31 (defined in `fuzzer/payloads/mod.rs`) |
| WAF products | 34 (defined in `waf/data/patterns.rs`) |
| TUI tabs | 29 |
| Pipeline profiles | 11 |
| Feature flags | 20+ |
| NSE libraries | 169 (in `crates/slapper-nse/src/libraries/`) |
| Source files | 743 |
| Tests | 1324 base, 1469+ with full features |

---

*This overview serves as the entry point to the architecture documentation. Each linked document provides a deep dive into a specific component or domain.*
