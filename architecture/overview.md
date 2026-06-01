# Architecture Overview

Slapper is a high-performance, async-first security testing toolkit built in Rust. This document provides a birds-eye view of the entire system, linking to detailed architecture docs for each component.

## Table of Contents

- [System Architecture](#system-architecture)
- [Core Layers](#core-layers)
- [Module Groups](#module-groups)
- [Feature Flags](#feature-flags)
- [Data Flow](#data-flow)
- [Key Types](#key-types)
- [Module Dependency Map](#module-dependency-map)

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        User Interfaces                              │
│  ┌─────────┐  ┌──────────┐  ┌─────────┐  ┌──────────────────────┐ │
│  │   CLI   │  │   TUI    │  │  REST   │  │  MCP/OpenAI Agents   │ │
│  │ (clap)  │  │(ratatui) │  │  API    │  │  (Tool Protocol)     │ │
│  └────┬────┘  └────┬─────┘  └────┬────┘  └──────────┬───────────┘ │
│       │            │             │                   │             │
├───────┴────────────┴─────────────┴───────────────────┴─────────────┤
│                      Command Dispatch Layer                         │
│                    (commands/handlers/)                              │
├─────────────────────────────────────────────────────────────────────┤
│                      Core Security Modules                          │
│  ┌─────────┐ ┌────────┐ ┌──────┐ ┌─────────┐ ┌─────────────────┐  │
│  │ Scanner │ │ Fuzzer │ │ WAF  │ │  Recon  │ │   Load Test     │  │
│  └─────────┘ └────────┘ └──────┘ └─────────┘ └─────────────────┘  │
│  ┌─────────┐ ┌────────┐ ┌──────┐ ┌─────────┐ ┌─────────────────┐  │
│  │  Auth   │ │ Proxy  │ │Stress│ │ Packet  │ │   Pipeline      │  │
│  └─────────┘ └────────┘ └──────┘ └─────────┘ └─────────────────┘  │
├─────────────────────────────────────────────────────────────────────┤
│                      Infrastructure Layer                           │
│  ┌──────────┐ ┌───────────┐ ┌──────────┐ ┌──────────────────────┐ │
│  │  Config  │ │ Distributed│ │  Output  │ │  Storage/Workflow    │ │
│  └──────────┘ └───────────┘ └──────────┘ └──────────────────────┘ │
├─────────────────────────────────────────────────────────────────────┤
│                      Integration Layer                              │
│  ┌─────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────────────┐  │
│  │   AI    │ │   NSE    │ │ Browser  │ │  External Services   │  │
│  │(LLM/Gen)│ │(Lua NSE) │ │(Headless)│ │ (Jira/GitHub/GitLab) │  │
│  └─────────┘ └──────────┘ └──────────┘ └──────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Core Layers

### 1. Interface Layer
| Interface | Module | Documentation |
|-----------|--------|---------------|
| CLI (clap-based) | `cli/` | [cli_commands.md](cli_commands.md) |
| TUI (ratatui) | `tui/` | [tui.md](tui.md) |
| REST API | `tool/protocol/rest` | [ai_agents.md](ai_agents.md) |
| MCP/OpenAI | `tool/protocol/mcp`, `tool/protocol/openai` | [ai_agents.md](ai_agents.md) |

### 2. Command Dispatch
| Module | Purpose | Documentation |
|--------|---------|---------------|
| `commands/` | Command routing and handler orchestration | [cli_commands.md](cli_commands.md) |
| `commands/handlers/` | Individual command implementations | [cli_commands.md](cli_commands.md) |

### 3. Core Security Modules
See [Module Groups](#module-groups) below.

### 4. Infrastructure Layer
| Module | Purpose | Documentation |
|--------|---------|---------------|
| `config/` | Configuration loading, scope enforcement | [config.md](config.md) |
| `distributed/` | Worker/coordinator cluster architecture | [distributed.md](distributed.md) |
| `output/` | Report generation and format conversion | [output.md](output.md) |
| `storage/` | Database persistence (SQLx) | [storage.md](storage.md) |
| `workflow/` | Finding lifecycle management | [workflow.md](workflow.md) |

### 5. Integration Layer
| Module | Purpose | Documentation |
|--------|---------|---------------|
| `ai/` | LLM integration, script generation, WAF bypass | [ai_agents.md](ai_agents.md) |
| `nse` (crate) | Nmap NSE script execution (Lua VM) | [nse_integration.md](nse_integration.md) |
| `browser/` | Headless browser for DOM XSS/SPA | [browser.md](browser.md) |
| `integrations/` | Jira, GitHub, GitLab connectors | [integrations.md](integrations.md) |

---

## Module Groups

### Reconnaissance & Discovery
| Module | Purpose | Docs |
|--------|---------|------|
| `recon/` | DNS, WHOIS, SSL, tech detection, subdomain enum, CVE mapping | [recon.md](recon.md) |
| `scanner/` | TCP port scanning, endpoint discovery, service fingerprinting | [scanner.md](scanner.md) |
| `probe.rs` | ICMP probing and target classification | [scanner.md](scanner.md) |

### Security Testing
| Module | Purpose | Docs |
|--------|---------|------|
| `fuzzer/` | Security fuzzing engine (30 payload types) | [fuzzer.md](fuzzer.md) |
| `waf/` | WAF detection (34 products) and bypass techniques | [waf.md](waf.md) |
| `auth/` | Authentication testing (brute force, credential stuffing, MFA) | [auth.md](auth.md) |
| `hunt/` | Advanced threat hunting (authz bypass, race conditions) | [hunt.md](hunt.md) |
| `browser/` | DOM XSS and SPA crawling | [browser.md](browser.md) |
| `websocket/` | WebSocket security testing | [websocket.md](websocket.md) |

### Performance & Stress
| Module | Purpose | Docs |
|--------|---------|------|
| `loadtest/` | HTTP load testing with latency metrics | [loadtest.md](loadtest.md) |
| `stress/` | Network stress testing (SYN/UDP/HTTP/ICMP floods) | [networking.md](networking.md) |
| `packet/` | Packet capture, crafting, traceroute | [networking.md](networking.md) |

### Orchestration & Pipeline
| Module | Purpose | Docs |
|--------|---------|------|
| `pipeline/` | Chained security assessment profiles | [pipeline.md](pipeline.md) |
| `tool/` | Unified tool registry and execution framework | [ai_agents.md](ai_agents.md) |
| `agent/` | Autonomous security agent with scheduling | [ai_agents.md](ai_agents.md) |

### Infrastructure & Output
| Module | Purpose | Docs |
|--------|---------|------|
| `output/` | 8 report formats (JSON, HTML, CSV, SARIF, JUnit, etc.) | [output.md](output.md) |
| `distributed/` | Worker/coordinator cluster for parallel scanning | [distributed.md](distributed.md) |
| `proxy/` | SOCKS/HTTP/Tor proxy pool management | [proxy.md](proxy.md) |
| `config/` | TOML/YAML configuration with scope enforcement | [config.md](config.md) |

### Compliance & Risk
| Module | Purpose | Docs |
|--------|---------|------|
| `compliance/` | HIPAA, PCI, SOC2, OWASP compliance scanning | [compliance.md](compliance.md) |
| `vuln/` | Vulnerability triage, CVSS scoring, prioritization | [vuln.md](vuln.md) |
| `supply_chain/` | SBOM generation, typosquat detection | [supply_chain.md](supply_chain.md) |
| `container/` | Kubernetes/Docker security scanning | [container.md](container.md) |

### Integration & Storage
| Module | Purpose | Docs |
|--------|---------|------|
| `ai/` | AI/LLM client, cache, planner, script generation | [ai_agents.md](ai_agents.md) |
| `nse` (crate) | Lua VM with 169 NSE libraries | [nse_integration.md](nse_integration.md) |
| `storage/` | SQLx-based findings and history persistence | [storage.md](storage.md) |
| `workflow/` | Finding lifecycle (assignment, SLA, status) | [workflow.md](workflow.md) |
| `integrations/` | Jira, GitHub, GitLab external integrations | [integrations.md](integrations.md) |

### Support Modules
| Module | Purpose | Docs |
|--------|---------|------|
| `types.rs` | Shared types (Severity, SensitiveString, OutputFormat) | This document |
| `error/` | SlapperError canonical error type | [error.md](error.md) |
| `findings/` | Finding store and lifecycle management | [findings.md](findings.md) |
| `diff/` | Scan result diffing | [diff.md](diff.md) |
| `notify/` | Webhook notifications | [notify.md](notify.md) |
| `logging/` | Structured logging with tracing | This document |
| `constants.rs` | Shared constants | This document |
| `macros.rs` | Utility macros | This document |

---

## Feature Flags

Slapper uses Cargo feature flags to conditionally compile optional capabilities:

### Default Features
Core scanning, fuzzing, WAF detection, and load testing are always available.

### Optional Feature Flags

| Flag | Modules Enabled | Description |
|------|-----------------|-------------|
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
| `cloud` | Cloud scanning | AWS, GCP, Azure |
| `git-secrets` | Git scanning | Repository secret detection |
| `wireless` | `wireless/` | WiFi scanning, auth testing |
| `pdf` | `output/pdf` | PDF report generation |
| `api-schema` | `api_schema/` | API schema support |
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
| `Severity` | `types.rs` | Canonical severity rating (Critical→Info) |
| `SensitiveString` | `types.rs` | Zeroized credential wrapper |
| `OutputFormat` | `types.rs` | Report format enum (8 variants) |
| `SlapperError` | `error/mod.rs` | Canonical error type |
| `TargetScope` | `config/scope.rs` | Target scope enforcement |

### Scanner Types
| Type | Location | Purpose |
|------|----------|---------|
| `ScanResults` | `waf/types.rs:188` | Port scan results |
| `FingerprintResults` | `scanner/fingerprint.rs:83` | Service identification |
| `SpoofConfig` | `scanner/spoof.rs` | IP spoofing configuration |
| `TimingPreset` | `scanner/timing.rs` | Scan speed presets |

### Fuzzer Types
| Type | Location | Purpose |
|------|----------|---------|
| `FuzzEngine` | `fuzzer/engine/` | Main fuzzing orchestrator |
| `PayloadType` | `fuzzer/payloads/mod.rs` | 30 payload categories |
| `FuzzResult` | `fuzzer/engine/types.rs:10` | Individual test result |

### WAF Types
| Type | Location | Purpose |
|------|----------|---------|
| `WafDetector` | `waf/detector/` | WAF identification |
| `BypassEngine` | `waf/bypass/` | Bypass technique execution |
| `WafProfile` | `waf/bypass/profiles.rs:9` | WAF-specific configurations |

### Tool/Agent Types
| Type | Location | Purpose |
|------|----------|---------|
| `ToolRegistry` | `tool/registry.rs` | Central tool registry |
| `SecurityTool` | `tool/traits.rs` | Tool trait definition |
| `McpProfile` | `tool/protocol/mcp/profile.rs` | Agent profile (Ops/Coding) |
| `McpProfilePolicy` | `tool/protocol/mcp/policy.rs` | Per-profile tool restrictions |
| `AiClient` | `ai/client.rs` | LLM client |
| `AiPlanner` | `ai/planner.rs` | AI-driven execution planning |

### Pipeline Types
| Type | Location | Purpose |
|------|----------|---------|
| `Pipeline` | `pipeline/executor.rs:38` | Stage orchestrator |
| `Stage` | `pipeline/stage.rs` | Individual scan stage |
| `PipelineContext` | `pipeline/context.rs` | Shared stage state |

---

## Module Dependency Map

### High-Level Dependencies

```
                    ┌─────────────┐
                    │   config    │
                    └──────┬──────┘
                           │
         ┌─────────────────┼─────────────────┐
         │                 │                 │
         ▼                 ▼                 ▼
    ┌─────────┐      ┌──────────┐      ┌─────────┐
    │ scanner │      │  recon   │      │ fuzzer  │
    └────┬────┘      └────┬─────┘      └────┬────┘
         │                │                 │
         └────────────────┼─────────────────┘
                          │
                          ▼
                   ┌─────────────┐
                   │    tool     │
                   │ (registry)  │
                   └──────┬──────┘
                          │
         ┌────────────────┼────────────────┐
         │                │                │
         ▼                ▼                ▼
    ┌─────────┐     ┌──────────┐     ┌─────────┐
    │   waf   │     │ pipeline │     │ agent   │
    └─────────┘     └──────────┘     └─────────┘
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
| `tui` | `config`, `commands`, `output` |
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

### Testing
- **Unit tests**: `cargo test --lib -p slapper`
- **Integration tests**: `cargo test --test scanner_tests -p slapper`
- **Negative tests**: `cargo test --test negative_tests -p slapper`
- **Visual regression**: `TestBackend` + `Terminal::new()` for TUI
- **Test count**: 1324 base, 1469+ with full features

### Code Quality
- **Lints**: `cargo clippy --lib -p slapper`
- **Formatting**: `cargo fmt`
- **Pre-commit**: Clippy warnings ~33 (pre-existing, none in ai module)
- **Hash collections**: `rustc_hash::FxHashMap` for performance paths

---

## See Also

- [feature_matrix.md](feature_matrix.md) - Detailed feature flag dependencies
- [defense_lab.md](defense_lab.md) - Defense-lab mode and regression validation
- [review_plan.md](review_plan.md) - Architecture review methodology

---

*Last updated: 2026-05-31*
