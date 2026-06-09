# Architecture Overview

Slapper is a high-performance, async-first security testing toolkit built in Rust. This document provides a birds-eye view of the entire system, serving as an index to detailed architecture documentation for each component.

## Table of Contents

- [Crate Layout](#crate-layout)
- [System Architecture](#system-architecture)
- [Module Index](#module-index) (Deep dive links for each component)
- [User Interfaces](#user-interfaces)
- [Core Security Modules](#core-security-modules)
- [Infrastructure Modules](#infrastructure-modules)
- [Integration Modules](#integration-modules)
- [Supporting Modules](#supporting-modules)
- [Feature Flags](#feature-flags)
- [Data Flow](#data-flow)
- [Key Types](#key-types)
- [Module Dependency Map](#module-dependency-map)
- [Cross-Cutting Concerns](#cross-cutting-concerns)

---

## Crate Layout

Slapper is organized as a Cargo workspace. The first-level crate boundary is:

- **`slapper-core`**: dependency-light domain types (`Severity`, `SensitiveString`), constants, and shared primitives. Designed for fast independent compilation with a small dependency set.
- **`slapper-tool-core`**: core data types for the tool abstraction layer (requests, responses, findings, errors). Dependency-light types shared between `slapper` and tool protocol integrations.
- **`slapper`**: main engine, CLI command model/dispatch, assessment modules, remaining API/agent adapters, feature-gated integrations, and the canonical `SlapperError` type.
- **`slapper-nse`**: optional Nmap NSE compatibility runtime and libraries.
- **`slapper-tui`**: terminal UI adapter built on `ratatui`/`crossterm`. Depends on Slapper engine APIs but should not be required for engine-only builds.
- **`slapper-cli`**: CLI binary entry point. Depends on both `slapper` and `slapper-tui`.
- **`slapper-output`**: report formatting and output adapters (JSON, CSV, HTML, SARIF, JUnit, Markdown). Extracted from `slapper` to reduce its dependency surface; modules with deep engine coupling (`pdf`, `report`, `report_summary`, `run_manifest`, `attack_graph`) remain in `slapper`.

New modules should avoid adding heavy runtime dependencies to `slapper-core`. Types that depend on `clap`, `reqwest`, `tokio`, `ratatui`, or other heavy crates should remain in the main `slapper` crate or in `slapper-tui` as appropriate.

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         User Interfaces                              │
│  ┌─────────┐  ┌──────────┐  ┌─────────┐  ┌──────────────────────┐ │
│  │   CLI   │  │   TUI    │  │  REST   │  │  MCP/OpenAI Agents   │ │
│  │ (clap)  │  │(ratatui) │  │  API    │  │  (Tool Protocol)     │ │
│  └────┬────┘  └────┬─────┘  └────┬────┘  └──────────┬───────────┘ │
│       │            │             │                   │             │
├───────┴────────────┴─────────────┴───────────────────┴─────────────┤
│                       Command Dispatch Layer                         │
│                     (commands/handlers/)                              │
├─────────────────────────────────────────────────────────────────────┤
│                       Core Security Modules                          │
│  ┌─────────┐ ┌────────┐ ┌──────┐ ┌─────────┐ ┌─────────────────┐  │
│  │ Scanner │ │ Fuzzer │ │ WAF  │ │  Recon  │ │   Load Test     │  │
│  └─────────┘ └────────┘ └──────┘ └─────────┘ └─────────────────┘  │
│  ┌─────────┐ ┌────────┐ ┌──────┐ ┌─────────┐ ┌─────────────────┐  │
│  │  Auth   │ │ Proxy  │ │Stress│ │ Packet  │ │   Pipeline      │  │
│  └─────────┘ └────────┘ └──────┘ └─────────┘ └─────────────────┘  │
├─────────────────────────────────────────────────────────────────────┤
│                       Infrastructure Layer                           │
│  ┌──────────┐ ┌───────────┐ ┌──────────┐ ┌──────────────────────┐ │
│  │  Config  │ │ Distributed│ │  Output  │ │  Storage/Workflow    │ │
│  └──────────┘ └───────────┘ └──────────┘ └──────────────────────┘ │
├─────────────────────────────────────────────────────────────────────┤
│                       Integration Layer                              │
│  ┌─────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────────────┐  │
│  │   AI    │ │   NSE    │ │ Browser  │ │  External Services   │  │
│  │(LLM/Gen)│ │(Lua NSE) │ │(Headless)│ │ (Jira/GitHub/GitLab) │  │
│  └─────────┘ └──────────┘ └──────────┘ └──────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Module Index

Use this index to navigate to detailed architecture documentation for each component.

### Reconnaissance & Discovery

| Module | Purpose | Architecture Doc |
|--------|---------|------------------|
| [`recon/`](../crates/slapper/src/recon/) | DNS enumeration, WHOIS, SSL analysis, subdomain discovery, technology detection, CVE mapping, cloud asset discovery | [recon.md](recon.md) |
| [`scanner/`](../crates/slapper/src/scanner/) | TCP/UDP port scanning, endpoint discovery, service fingerprinting, IP spoofing | [scanner.md](scanner.md) |
| [`probe.rs`](../crates/slapper/src/probe.rs) | ICMP probing, probe intent classification, risk assessment | [probe.md](probe.md) |

### Security Testing

| Module | Purpose | Architecture Doc |
|--------|---------|------------------|
| [`fuzzer/`](../crates/slapper/src/fuzzer/) | Security fuzzing engine with 30 payload types (SQLi, XSS, SSRF, Path Traversal, ReDoS, etc.) | [fuzzer.md](fuzzer.md) |
| [`waf/`](../crates/slapper/src/waf/) | WAF detection (34 products), bypass techniques, evasion-resistance testing | [waf.md](waf.md) |
| [`auth/`](../crates/slapper/src/auth/) | Authentication testing (brute force, credential stuffing, MFA bypass, JWT analysis, OAuth/OIDC) | [auth.md](auth.md) |
| [`hunt/`](../crates/slapper/src/hunt/) | Advanced threat hunting (authorization bypass, race conditions, advanced injection) | [hunt.md](hunt.md) |
| [`browser/`](../crates/slapper/src/browser/) | Headless browser for DOM XSS detection, SPA crawling | [browser.md](browser.md) |
| [`websocket/`](../crates/slapper/src/websocket/) | WebSocket security testing | [websocket.md](websocket.md) |

### Performance & Stress

| Module | Purpose | Architecture Doc |
|--------|---------|------------------|
| [`loadtest/`](../crates/slapper/src/loadtest/) | HTTP load testing with detailed latency metrics, concurrency control | [loadtest.md](loadtest.md) |
| [`stress/`](../crates/slapper/src/stress/) | Network stress testing (SYN, UDP, HTTP, TCP, ICMP floods), IP spoofing | [stress.md](stress.md) |
| [`packet/`](../crates/slapper/src/packet/) | Packet capture, crafting, parsing (pnet-based), traceroute | [networking.md](networking.md) |

### Orchestration & Pipeline

| Module | Purpose | Architecture Doc |
|--------|---------|------------------|
| [`pipeline/`](../crates/slapper/src/pipeline/) | Chained security assessment profiles (11 built-in profiles) | [pipeline.md](pipeline.md) |
| [`tool/`](../crates/slapper/src/tool/) | Unified tool registry, execution framework, MCP/OpenAI protocol integration; core DTOs in `slapper-tool-core` | [ai_agents.md](ai_agents.md) |
| [`agent/`](../crates/slapper/src/agent/) | Autonomous security agent with scheduling, longitudinal memory, portfolio management | [ai_agents.md](ai_agents.md) |
| [`distributed/`](../crates/slapper/src/distributed/) | Worker/coordinator cluster architecture for parallel scanning | [distributed.md](distributed.md) |

### Infrastructure & Output

| Module | Purpose | Architecture Doc |
|--------|---------|------------------|
| [`output/`](../crates/slapper/src/output/) | Compatibility facade over `slapper-output` plus engine-coupled report modules (PDF, report, report_summary, run_manifest, attack_graph) | [output.md](output.md) |
| [`proxy/`](../crates/slapper/src/proxy/) | SOCKS4, SOCKS5, HTTP, HTTPS, Tor proxy pool with health checking, rotation strategies | [proxy.md](proxy.md) |
| [`config/`](../crates/slapper/src/config/) | TOML/YAML configuration loading, scope enforcement, TUI settings | [config.md](config.md) |
| [`storage/`](../crates/slapper/src/storage/) | SQLx-based PostgreSQL persistence for findings and scan history | [storage.md](storage.md) |
| [`workflow/`](../crates/slapper/src/workflow/) | Finding lifecycle management (assignment, SLA tracking, status transitions) | [workflow.md](workflow.md) |

### Compliance & Risk

| Module | Purpose | Architecture Doc |
|--------|---------|------------------|
| [`compliance/`](../crates/slapper/src/compliance/) | HIPAA, PCI, SOC2, OWASP compliance scanning and reporting | [compliance.md](compliance.md) |
| [`vuln/`](../crates/slapper/src/vuln/) | Vulnerability triage, CVSS scoring, prioritization | [vuln.md](vuln.md) |
| [`supply_chain/`](../crates/slapper/src/supply_chain/) | SBOM generation (CycloneDX, SPDX), typosquat detection | [supply_chain.md](supply_chain.md) |
| [`container/`](../crates/slapper/src/container/) | Kubernetes/Docker security scanning | [container.md](container.md) |

### Integration & External Services

| Module | Purpose | Architecture Doc |
|--------|---------|------------------|
| [`ai/`](../crates/slapper/src/ai/) | AI/LLM client (OpenAI, Anthropic, Azure), cache, planner, script generation, WAF bypass suggestions | [ai_agents.md](ai_agents.md) |
| [`slapper-nse/`](../crates/slapper-nse/) | Nmap Scripting Engine support (Lua 5.4), 169 NSE libraries | [nse_integration.md](nse_integration.md) |
| [`integrations/`](../crates/slapper/src/integrations/) | Jira, GitHub, GitLab external connectors | [integrations.md](integrations.md) |
| [`notify/`](../crates/slapper/src/notify/) | Webhook, Slack, Discord, Teams notifications | [notify.md](notify.md) |

### User Interfaces

| Module | Purpose | Architecture Doc |
|--------|---------|------------------|
| [`cli/`](../crates/slapper/src/cli/) | Command-line argument parsing (clap-based), 37+ commands | [cli_commands.md](cli_commands.md) |
| [`tui/`](../crates/slapper-tui/src/) | Real-time terminal UI (ratatui-based), 28+ tabs, event loop | [tui.md](tui.md) |

### Supporting Modules

| Module | Purpose | Architecture Doc |
|--------|---------|------------------|
| [`slapper-core`](../crates/slapper-core/) | Dependency-light shared types (Severity, SensitiveString), constants | [types.md](types.md) |
| [`slapper-tool-core`](../crates/slapper-tool-core/) | Protocol-neutral tool request/response/error/history DTOs | [ai_agents.md](ai_agents.md) |
| [`slapper-output`](../crates/slapper-output/src/) | Portable report formatting and output adapters (JSON, CSV, HTML, SARIF, JUnit, Markdown) | [output.md](output.md) |
| [`types.rs`](../crates/slapper/src/types.rs) | Main-crate compatibility facade plus CLI-facing types such as `OutputFormat` | [types.md](types.md) |
| [`constants.rs`](../crates/slapper/src/constants.rs) | Compatibility facade over core constants plus any engine-local constants | [constants.md](constants.md) |
| [`error/`](../crates/slapper/src/error/) | Canonical error type with domain-specific variants | [error.md](error.md) |
| [`findings/`](../crates/slapper/src/findings/) | Finding store, lifecycle management, fingerprinting | [findings.md](findings.md) |
| [`diff/`](../crates/slapper/src/diff/) | Scan result diffing, baseline comparison | [diff.md](diff.md) |
| [`logging/`](../crates/slapper/src/logging/) | Structured logging with tracing | [logging.md](logging.md) |
| [`utils/`](../crates/slapper/src/utils/) | 23 submodules (HTTP client, rate limiting, circuit breaker, formatting) | [utils.md](utils.md) |
| [`auth_context/`](../crates/slapper/src/auth_context/) | Auth context YAML parsing with env var interpolation | [auth_context.md](auth_context.md) |
| [`generated/`](../crates/slapper/src/generated/) | Auto-generated protobuf code | [generated.md](generated.md) |
| [`wireless/`](../crates/slapper/src/wireless/) | WiFi scanning, authentication testing | [wireless.md](wireless.md) |

---

## User Interfaces

### CLI (`cli/`)

The command-line interface is built with `clap` and provides 37+ commands organized into functional groups:

```
slapper scan      # Port scanning, service fingerprinting
slapper fuzz      # Vulnerability fuzzing
slapper waf       # WAF detection and bypass
slapper recon     # Reconnaissance operations
slapper load      # Load testing
slapper agent     # Autonomous agent control
slapper pipeline  # Pipeline profile execution
```

- **Entry point**: `crates/slapper/src/cli/mod.rs`
- **Handlers**: `crates/slapper/src/commands/handlers/`
- **Documentation**: [cli_commands.md](cli_commands.md)

### TUI (`tui/`)

The terminal user interface uses `ratatui` with 28+ tabs organized by function. The TUI is now a separate crate (`slapper-tui`), extracted from the main `slapper` crate.

| Tab Group | Tabs |
|-----------|------|
| **Dashboard** | Overview, Session, Scan Progress |
| **Recon** | Targets, DNS, Subdomains, SSL, Technologies, CVEs |
| **Scanning** | Ports, Endpoints, Services, Spoof Config |
| **Security** | Fuzzer, WAF, Auth, Hunt, Browser |
| **Infrastructure** | Proxy, Load Test, Stress, Packets |
| **Intelligence** | Findings, Workflow, Compliance, Vulns |
| **Agent** | Portfolio, Skills, Schedule, Memory |
| **System** | Config, Scope, Logs, About |

- **Documentation**: [tui.md](tui.md)

### REST API & Agent Protocols (`tool/`)

Machine-accessible interfaces for automation and AI integration:

| Protocol | Feature Flag | Purpose |
|----------|--------------|---------|
| REST API | `rest-api` | HTTP API server for agent integration |
| gRPC | `grpc-api` | High-performance gRPC API |
| WebSocket | `ws-api` | Pub/sub event streaming |
| MCP | Built-in | Model Context Protocol for AI agents |
| OpenAI | Built-in | OpenAI tool protocol compatibility |

- **Documentation**: [ai_agents.md](ai_agents.md)

---

## Core Security Modules

### Scanner (`scanner/`)

High-performance port scanning with configurable timing:

| Capability | Description |
|------------|-------------|
| **TCP Scanning** | SYN, CONNECT, FIN, NULL, XMAS, ACK scans |
| **UDP Scanning** | UDP probe-based scanning |
| **Service Detection** | 20+ protocol fingerprints |
| **Endpoint Discovery** | 261 built-in path signatures |
| **IP Spoofing** | Raw socket spoofing (feature-gated) |
| **Timing Presets** | Paranoid, Sneaky, Polite, Normal, Aggressive, Insane |

- **Documentation**: [scanner.md](scanner.md)

### Fuzzer (`fuzzer/`)

Mutation-based security fuzzing engine with 30 payload types:

| Category | Payload Types |
|----------|---------------|
| **Injection** | SQLi (5 variants), XSS (6 variants), SSRF, SSTI, Command Injection |
| **Traversal** | Path Traversal, Path Normalization |
| **Protocol** | HTTP smuggling, Header Injection, ReDoS |
| **API** | GraphQL introspection/injection, JWT manipulation |
| **Discovery** | Directory brute force, Virtual host detection |

- **Documentation**: [fuzzer.md](fuzzer.md)

### WAF (`waf/`)

WAF detection and bypass testing:

| Capability | Description |
|------------|-------------|
| **Detection** | 34 WAF products identified |
| **Bypass Techniques** | Header manipulation, encoding, protocol violations |
| **Evasion Testing** | Multi-encoded payloads, case normalization |
| **Smart Bypass** | AI-powered bypass suggestion (with `ai-integration`) |

- **Documentation**: [waf.md](waf.md)

### Recon (`recon/`)

Passive and active reconnaissance:

| Capability | Description |
|------------|-------------|
| **DNS** | Enumeration, zone transfer, DNS-over-HTTPS |
| **WHOIS** | Domain registration data |
| **SSL/TLS** | Certificate analysis, heartbleed, criminality checks |
| **Subdomains** | Brute force, certificate transparency, DNS aggregation |
| **Technologies** | HTTP fingerprinting, framework detection |
| **CVE Mapping** | Technology-to-CVE correlation |
| **Cloud Assets** | AWS, GCP, Azure asset discovery |
| **CORS Analysis** | Cross-origin policy testing |

- **Documentation**: [recon.md](recon.md)

---

## Infrastructure Modules

### Config (`config/`)

Configuration management with scope enforcement:

| Component | Description |
|-----------|-------------|
| **TOML/YAML Loading** | Hierarchical config from files and env vars |
| **Scope Enforcement** | Target restrictions via pattern/CIDR rules |
| **TUI Settings** | Partial save with field exposure control |
| **Profile Management** | 11 built-in scan profiles |

- **Documentation**: [config.md](config.md)

### Pipeline (`pipeline/`)

Chained security assessment profiles:

| Profile | Stages |
|---------|--------|
| **quick** | Port scan → Service fingerprint |
| **endpoint** | Quick → Directory discovery |
| **web** | Endpoint → Vulnerability fuzzing |
| **waf** | Endpoint → WAF detection → Bypass |
| **full** | All stages → Load testing |
| **api** | GraphQL → JWT → OAuth testing |
| **recon** | Intelligence-led → Tech detection → CVE mapping |
| **stealth** | Evasion mode with randomized delays |
| **deep** | Mutation fuzzing enabled |
| **vuln** | CVE-prioritized based on detected tech |
| **auth** | JWT, OAuth, IDOR focused |

- **Documentation**: [pipeline.md](pipeline.md)

### Output (`output/`)

Multi-format report generation:

| Format | Use Case |
|--------|----------|
| JSON | Machine parsing, automation |
| HTML | Human-readable reports |
| CSV | Spreadsheet analysis |
| SARIF | CI/CD security scanning (GitHub, GitLab) |
| JUnit XML | Test integration (CI pipelines) |
| Markdown | Documentation, GitHub issues |
| PDF | Formal reports (feature-gated) |

- **Documentation**: [output.md](output.md)

### Distributed (`distributed/`)

Worker/coordinator cluster for parallel scanning:

| Component | Role |
|-----------|------|
| **Coordinator** | Task distribution, result aggregation |
| **Worker** | Parallel scan execution |
| **Task Router** | Work stealing, load balancing |
| **Result Merger** | Findings deduplication |

- **Documentation**: [distributed.md](distributed.md)

---

## Integration Modules

### AI (`ai/`)

LLM integration for intelligent security testing:

| Capability | Description |
|------------|-------------|
| **Multi-Provider** | OpenAI, Anthropic, Azure OpenAI |
| **Response Caching** | TTL cache to reduce API calls |
| **WAF Bypass** | AI-powered bypass technique suggestion |
| **Script Generation** | Dynamic payload generation |
| **Planner** | AI-driven execution planning |

- **Documentation**: [ai_agents.md](ai_agents.md)

### NSE (`slapper-nse/`)

Nmap Scripting Engine compatibility:

| Component | Description |
|-----------|-------------|
| **Lua VM** | Lua 5.4 via mlua crate |
| **Libraries** | 169 NSE-compatible library wrappers |
| **CVE Integration** | NVD, OSV, CISA KEV feeds |
| **Sandbox** | Restricted Lua operation execution |

- **Documentation**: [nse_integration.md](nse_integration.md)

---

## Feature Flags

Slapper uses Cargo feature flags to conditionally compile optional capabilities:

| Flag | Modules | Description |
|------|---------|-------------|
| `stress-testing` | `stress/`, `packet/` | Raw sockets, IP spoofing, DoS tools |
| `packet-inspection` | `packet/` | Live packet capture, traceroute |
| `rest-api` | `tool/protocol/rest` | HTTP REST API server |
| `grpc-api` | `tool/protocol/grpc` | gRPC API server |
| `ws-api` | `tool/protocol/ws` | WebSocket pub/sub |
| `nse` | `slapper-nse` | Nmap NSE script support |
| `nse-ssh2` | NSE SSH2 libs | Full SSH2/libssh2 support |
| `nse-sandbox` | NSE sandbox | Restrict dangerous Lua operations |
| `ai-integration` | `ai/` | AI planner, script generation |
| `websocket` | `websocket/` | WebSocket security testing |
| `headless-browser` | `browser/` | DOM XSS and SPA crawling |
| `database` | `storage/` | SQLx-based persistence |
| `container` | `container/` | Kubernetes/Docker scanning |
| `sbom` | `supply_chain/` | SBOM generation |
| `advanced-hunting` | `hunt/` | Advanced threat hunting |
| `compliance` | `compliance/` | Compliance scanning |
| `external-integrations` | `integrations/` | Jira, GitHub, GitLab |
| `finding-workflow` | `workflow/` | Finding lifecycle management |
| `vuln-management` | `vuln/` | Vulnerability triage |
| `wireless` | `wireless/` | WiFi scanning, auth testing |
| `pdf` | `output/pdf` | PDF report generation |
| `full` | All | All features combined |

See [feature_matrix.md](feature_matrix.md) for detailed feature dependencies.

---

## Data Flow

```
                     ┌─────────────┐
                     │   Target    │
                     │  (URL/IP)   │
                     └──────┬──────┘
                            │
            ┌───────────────┼───────────────┐
            │               │               │
            ▼               ▼               ▼
     ┌──────────┐    ┌──────────┐    ┌──────────┐
     │  Recon   │    │ Scanner  │    │  Probe   │
     │(DNS,SSL) │    │(Ports)   │    │ (ICMP)   │
     └────┬─────┘    └────┬─────┘    └────┬─────┘
          │               │               │
          └───────────────┼───────────────┘
                          │
                          ▼
               ┌─────────────────────┐
               │  Service Detection  │
               │  (Fingerprinting)   │
               └──────────┬──────────┘
                          │
          ┌───────────────┼───────────────┐
          │               │               │
          ▼               ▼               ▼
    ┌──────────┐    ┌──────────┐    ┌──────────┐
    │   WAF    │    │  Fuzz    │    │  Auth    │
    │ Detection│    │ Engine   │    │  Tests   │
    └────┬─────┘    └────┬─────┘    └────┬─────┘
         │               │               │
         └───────────────┼───────────────┘
                         │
                         ▼
              ┌─────────────────────┐
              │   Findings Store    │
              │ (Dedup, Triage)     │
              └──────────┬──────────┘
                         │
           ┌─────────────┼─────────────┐
           │             │             │
           ▼             ▼             ▼
     ┌──────────┐  ┌──────────┐  ┌──────────┐
     │  Output  │  │ Workflow │  │  Alert   │
     │(Reports) │  │(Lifecycle)│  │(Webhook) │
     └──────────┘  └──────────┘  └──────────┘
```

---

## Key Types

### Core Types

| Type | Location | Purpose |
|------|----------|---------|
| `SlapperConfig` | `config/settings.rs` | Main configuration struct |
| `Severity` | `slapper-core::types` (re-exported by `types.rs`) | Canonical severity rating (Critical→Info) |
| `SensitiveString` | `slapper-core::types` (re-exported by `types.rs`) | Zeroized credential wrapper |
| `OutputFormat` | `types.rs` | Report format enum (8 variants) |
| `PayloadType` | `fuzzer/payloads/mod.rs` | 30 payload categories |
| `SlapperError` | `error/mod.rs` | Canonical error type |
| `TargetScope` | `config/scope.rs` | Target scope enforcement |
| `Finding` | `findings/mod.rs` | Canonical finding structure |
| `ProbeIntent` | `probe.rs` | Probe classification intent |
| `ProbeRisk` | `probe.rs` | Probe risk level assessment |

### Module-Specific Types

| Module | Key Type | Purpose |
|--------|----------|---------|
| Scanner | `FingerprintResults` | Service identification results |
| Scanner | `TimingPreset` | Scan speed configuration |
| Scanner | `SpoofConfig` | IP spoofing configuration |
| Fuzzer | `FuzzEngine` | Main fuzzing orchestrator |
| Fuzzer | `FuzzResult` | Individual test result |
| WAF | `WafDetector` | WAF identification engine |
| WAF | `BypassEngine` | Bypass technique execution |
| WAF | `SmartWafBypass` | AI-powered WAF bypass |
| Recon | `FullReconResult` | Complete recon results |
| Pipeline | `Pipeline` | Stage orchestrator |
| Pipeline | `Stage` | Individual scan stage |
| Pipeline | `PipelineContext` | Shared stage state |
| Tool | `ToolRegistry` | Central tool registry |
| Tool | `SecurityTool` | Tool trait definition |
| Tool | `McpProfile` | Agent profile (Ops/Coding) |
| Tool | `McpProfilePolicy` | Per-profile tool restrictions |
| AI | `AiClient` | LLM client |
| AI | `AiPlanner` | AI-driven execution planning |
| AI | `AiCache` | TTL response cache |
| Loadtest | `LoadTestRunner` | Load test orchestrator |
| Loadtest | `LoadTestResults` | Performance metrics |

---

## Module Dependency Map

### High-Level Dependencies

```
                ┌──────────────┐
                │ slapper-core │
                └──────┬───────┘
                       │
                ┌──────┴───────┐
                │   slapper    │
                └──────┬───────┘
                       │
         ┌─────────────┼─────────────────┐
         │             │                 │
         ▼             ▼                 ▼
    ┌─────────┐  ┌──────────┐    ┌─────────────┐
    │  config │  │  scanner │    │  slapper-nse│
    └────┬────┘  └────┬─────┘    └─────────────┘
         │            │
         └────────────┼─────────────────┘
                      │
                      ▼
               ┌─────────────┐
               │    tool     │
               │ (registry)  │
               └──────┬──────┘
                      │
         ┌────────────┼────────────────┐
         │            │                │
         ▼            ▼                ▼
    ┌─────────┐ ┌──────────┐    ┌─────────┐
    │   waf   │ │ pipeline │    │  agent  │
    └─────────┘ └──────────┘    └─────────┘
```

### Module Group Dependencies

| Module | Depends On |
|--------|------------|
| `scanner` | `config`, `error`, `types`, `proxy` (optional) |
| `fuzzer` | `config`, `error`, `types`, `waf` (optional) |
| `waf` | `config`, `error`, `types`, `fuzzer` (payloads) |
| `recon` | `config`, `error`, `types` |
| `auth` | `config`, `error`, `types`, `scanner` |
| `loadtest` | `config`, `error`, `types` |
| `pipeline` | `scanner`, `fuzzer`, `waf`, `recon`, `loadtest` |
| `tool` | All security modules (via `ToolRegistry`) |
| `agent` | `tool`, `config`, `output`, `ai` (optional) |
| `distributed` | `tool`, `config` |
| `output` | `types`, `findings` |
| `tui` | `config`, `commands`, `output` (in `slapper-tui`) |
| `ai` | `config`, `error`, `types` |
| `nse` | `scanner`, `recon` (via Lua bindings) |

---

## Cross-Cutting Concerns

### Error Handling

- **Library code**: Uses `SlapperError` via `Result<T>`
- **Command handlers**: Use `anyhow::Result` for convenience
- **Bridging**: `.map_err()` converts between types at boundaries
- See [error.md](error.md) for error variant catalog

### Configuration

- **File format**: TOML (primary), YAML (secondary)
- **Location**: `~/.config/slapper/slapper.toml`
- **Scope enforcement**: `TargetScope` validates targets before scanning
- **TUI settings**: Partial save with field exposure control
- See [config.md](config.md) for details

### Logging & Tracing

- **Framework**: `tracing` with structured spans
- **Formats**: Pretty (human), JSON (machine)
- **Levels**: Error, Warn, Info, Debug, Trace
- **Sensitive data**: `SensitiveString` with redaction support
- See [logging.md](logging.md)

### Testing

| Test Suite | Command |
|------------|---------|
| Unit tests | `cargo test --lib -p slapper` |
| TUI tests | `cargo test --lib -p slapper-tui` |
| Integration tests | `cargo test --test scanner_tests -p slapper` |
| Negative tests | `cargo test --test negative_tests -p slapper` |
| Clippy | `cargo clippy --lib -p slapper` |

- **Test count**: 1324 base, 1469+ with full features
- **Visual regression**: `TestBackend` + `Terminal::new()` for TUI

---

## Defense-Lab Mode

Slapper supports local, repeatable profiles against defensive systems for regression testing:

| Profile | Purpose |
|---------|---------|
| `DefenseLab` | Baseline diff and defense validation |
| `SynvoidLocal` | Localhost SYN scan testing |
| `WafRegression` | WAF detection regression testing |
| `ProtocolEdge` | Protocol edge case testing |
| `NseSafe` | Safe NSE script execution |

See [defense_lab.md](defense_lab.md) for detailed documentation.

---

## See Also

### Architecture Documentation

| Category | Documents |
|----------|-----------|
| **Core** | [config.md](config.md), [types.md](types.md), [constants.md](constants.md), [error.md](error.md) |
| **Security** | [scanner.md](scanner.md), [fuzzer.md](fuzzer.md), [waf.md](waf.md), [recon.md](recon.md), [auth.md](auth.md), [hunt.md](hunt.md) |
| **Infrastructure** | [pipeline.md](pipeline.md), [distributed.md](distributed.md), [proxy.md](proxy.md), [loadtest.md](loadtest.md) |
| **Output** | [output.md](output.md), [findings.md](findings.md), [diff.md](diff.md), [workflow.md](workflow.md) |
| **Integration** | [ai_agents.md](ai_agents.md), [nse_integration.md](nse_integration.md), [integrations.md](integrations.md), [notify.md](notify.md) |
| **UI** | [tui.md](tui.md), [cli_commands.md](cli_commands.md) |
| **Compliance** | [compliance.md](compliance.md), [vuln.md](vuln.md), [supply_chain.md](supply_chain.md), [container.md](container.md) |
| **Utilities** | [utils.md](utils.md), [logging.md](logging.md), [probe.md](probe.md) |
| **Reference** | [feature_matrix.md](feature_matrix.md), [defense_lab.md](defense_lab.md), [review_plan.md](review_plan.md), [compile_time_baseline.md](compile_time_baseline.md) |

### Implementation Plan

See [plans/plan.md](../plans/plan.md) for implementation history and completed waves.

---

*Last updated: 2026-06-08*