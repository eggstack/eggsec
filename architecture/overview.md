# Architecture Overview

Eggsec is a Rust-native, scope-enforced security assessment and defense-validation engine with multiple frontends (CLI, TUI, REST, MCP, gRPC, Agent), centralized policy enforcement, and domain execution crates. This document provides a birds-eye view of the entire system and serves as an index to detailed architecture documentation for each component.

## Table of Contents

- [Workspace Crates](#workspace-crates)
- [System Architecture](#system-architecture)
- [Module Index](#module-index)
- [Enforcement Model](#enforcement-model)
- [Data Flow](#data-flow)
- [Feature Flags](#feature-flags)
- [Key Types](#key-types)
- [Dependency Map](#dependency-map)
- [Cross-Cutting Concerns](#cross-cutting-concerns)
- [See Also](#see-also)

---

## Workspace Crates

Eggsec is organized as a Cargo workspace with 15 crates. The first-level crate boundary separates dependency-light leaf crates from the composition root and frontends.

| Crate | Role | Dependency-Light | Notes |
|-------|------|:---:|-------|
| `eggsec-core` | Shared primitives | Yes | `Severity`, `SensitiveString`, constants. Zero internal deps. |
| `eggsec-tool-core` | Protocol-neutral DTOs | Yes | `ToolRequest`, `ToolResponse`, `ToolError`, history types. |
| `eggsec-output` | Report formatting | Yes | JSON/CSV/HTML/SARIF/JUnit/Markdown. No engine/runtime deps. |
| `eggsec-agent` | Agent coordination | Yes | Registry, scheduler, lifecycle. Depends only on `eggsec-core`. |
| `eggsec-runtime` | Frontend-neutral runtime | Yes | `Runtime`, `RuntimeTaskExecutor`, task lifecycle. No TUI/transport deps. |
| `eggsec-ui-model` | Frontend view DTOs | Yes | View model types for TUI/daemon rendering. Depends only on `eggsec-runtime`. |
| `eggsec` | Main engine (lib) | No | Composition root. All security modules, policy enforcement, dispatch. |
| `eggsec-nse` | NSE compatibility | Yes | Lua VM, 166 NSE libraries. Optional. |
| `eggsec-db-lab` | DB pentest domain | Yes | Postgres/MySQL/MSSQL/MongoDB/Redis checks. |
| `eggsec-web-proxy` | Web proxy domain | Yes | MITM intercept, TLS, protocol handlers. |
| `eggsec-mobile-lab` | Mobile analysis domain | Yes | APK/IPA static analysis + Android dynamic testing. |
| `eggsec-daemon` | Persistent daemon host | Yes | Unix socket server, session lifecycle, SQLite persistence. |
| `eggsec-tui` | Terminal UI | No | 33 tabs, ratatui/crossterm. Depends on engine + runtime + daemon. |
| `eggsec-cli` | CLI binary | Yes | Thin wrapper: `eggsec` + `eggsec-tui` (optional) + `eggsec-daemon` (optional). |
| `eggsec-python` | Python bindings | Yes | PyO3/maturin. 22 stable-core operations. |

**Dependency direction**: Leaf crates have no internal workspace dependencies. The main `eggsec` crate is the composition root. `eggsec-cli` and `eggsec-tui` are the only frontends that depend on `eggsec`.

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         User Interfaces                              │
│  ┌─────────┐  ┌──────────┐  ┌─────────┐  ┌──────────────────────┐ │
│  │   CLI   │  │   TUI    │  │  REST   │  │  MCP / gRPC / Agent  │ │
│  │ (clap)  │  │(ratatui) │  │  API    │  │  (Tool Protocol)     │ │
│  └────┬────┘  └────┬─────┘  └────┬────┘  └──────────┬───────────┘ │
│       │            │             │                   │             │
├───────┴────────────┴─────────────┴───────────────────┴─────────────┤
│                    EnforcementContext::evaluate()                    │
│                 (mandatory pre-dispatch gate)                        │
├─────────────────────────────────────────────────────────────────────┤
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
│  │  Config  │ │ Distributed│ │  Output  │ │  Storage / Workflow  │ │
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

This is the complete index of all modules and components. Each entry links to its detailed architecture document.

### Reconnaissance & Discovery

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Recon | `crates/eggsec/src/recon/` | DNS enumeration, WHOIS, SSL analysis, subdomain discovery, technology detection, CVE mapping, cloud asset discovery | [recon.md](recon.md) |
| Scanner | `crates/eggsec/src/scanner/` | TCP/UDP port scanning, endpoint discovery, service fingerprinting, IP spoofing, timing presets | [scanner.md](scanner.md) |
| Probe | `crates/eggsec/src/probe.rs` | ICMP probing, probe intent classification, risk assessment | [probe.md](probe.md) |
| Wireless | `crates/eggsec/src/wireless/` | WiFi passive recon + security analysis + rogue heuristic; active deauth/disassoc under `wireless-advanced` | [wireless.md](wireless.md) |

### Security Testing

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Fuzzer | `crates/eggsec/src/fuzzer/` | Security fuzzing engine with 42+ payload types (SQLi, XSS, SSRF, SSTI, etc.) | [fuzzer.md](fuzzer.md) |
| WAF | `crates/eggsec/src/waf/` | WAF detection (34 products), bypass techniques, evasion-resistance testing | [waf.md](waf.md) |
| Auth | `crates/eggsec/src/auth/` | Authentication testing (brute force, credential stuffing, MFA bypass, lockout/rate-limit) | [auth.md](auth.md) |
| Hunt | `crates/eggsec/src/hunt/` | Advanced threat hunting (authorization bypass, race conditions, advanced injection) | [hunt.md](hunt.md) |
| Browser | `crates/eggsec/src/browser/` | Headless browser for DOM XSS detection, SPA crawling | [browser.md](browser.md) |
| WebSocket | `crates/eggsec/src/websocket/` | WebSocket security testing | [websocket.md](websocket.md) |
| Evasion | `crates/eggsec/src/evasion/` | Evasion technique detection for defense validation | [evasion.md](evasion.md) |

### Performance & Stress

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Load Test | `crates/eggsec/src/loadtest/` | HTTP load testing with hdrhistogram metrics, concurrency control | [loadtest.md](loadtest.md) |
| Stress | `crates/eggsec/src/stress/` | Network stress testing (SYN, UDP, HTTP, TCP, ICMP floods), IP spoofing | [stress.md](stress.md) |
| Packet | `crates/eggsec/src/packet/` | Packet capture, crafting, parsing (pnet-based), traceroute | [networking.md](networking.md) |

### Orchestration & Pipeline

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Pipeline | `crates/eggsec/src/pipeline/` | Chained security assessment profiles (18 built-in profiles) | [pipeline.md](pipeline.md) |
| Tool Registry | `crates/eggsec/src/tool/` | Unified tool registry, execution framework, MCP/REST/gRPC protocol integration | [ai_agents.md](ai_agents.md) |
| Agent | `crates/eggsec/src/agent/` | Autonomous security agent with scheduling, longitudinal memory, portfolio management | [ai_agents.md](ai_agents.md) |
| Distributed | `crates/eggsec/src/distributed/` | Worker/coordinator cluster architecture for parallel scanning | [distributed.md](distributed.md) |

### Infrastructure & Output

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Config | `crates/eggsec/src/config/` | TOML/YAML configuration loading, scope enforcement, policy model, execution profiles | [config.md](config.md) |
| Output | `crates/eggsec-output/src/` + `crates/eggsec/src/output/` | Multi-format report generation (JSON, CSV, HTML, SARIF, JUnit, Markdown, PDF) | [output.md](output.md) |
| Proxy | `crates/eggsec/src/proxy/` | SOCKS4/5, HTTP, HTTPS, Tor proxy pool with health checking and rotation | [proxy.md](proxy.md) |
| Web Proxy | `crates/eggsec-web-proxy/` | MITM web proxy: HTTP/HTTPS/WebSocket/HTTP2/gRPC interception, TLS cert generation | [web_proxy.md](web_proxy.md) |
| Storage | `crates/eggsec/src/storage/` | SQLx-based PostgreSQL persistence for findings and scan history | [storage.md](storage.md) |
| Workflow | `crates/eggsec/src/workflow/` | Finding lifecycle management (assignment, SLA tracking, status transitions) | [workflow.md](workflow.md) |
| Findings | `crates/eggsec/src/findings/` | Finding types, store, lifecycle management, fingerprinting | [findings.md](findings.md) |
| Diff | (distributed) | Scan result diffing, baseline comparison | [diff.md](diff.md) |
| Domain Contract | `crates/eggsec/src/domain/` | Static metadata descriptors for capability domains | [domain_contract.md](domain_contract.md) |

### Daemon & Runtime

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Daemon | `crates/eggsec-daemon/` | Long-running daemon host: session persistence (SQLite), client registry, Unix socket/HTTP transport | [daemon.md](daemon.md) |
| Runtime | `crates/eggsec-runtime/` | Frontend-neutral async runtime: task lifecycle, session management, event broadcasting | [runtime.md](runtime.md) |
| Runtime Bridge | `crates/eggsec/src/runtime_bridge/` | Converts `eggsec-runtime` DTOs to engine enforcement types; preflight/approval/dispatch | [runtime_bridge.md](runtime_bridge.md) |
| UI Model | `crates/eggsec-ui-model/` | Frontend-neutral view DTOs and renderer registry | [ui_model.md](ui_model.md) |

### User Interfaces

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| CLI | `crates/eggsec/src/cli/` + `crates/eggsec-cli/` | Command-line argument parsing (clap), 52 commands, handler dispatch | [cli_commands.md](cli_commands.md) |
| TUI | `crates/eggsec-tui/src/` | Real-time terminal UI (ratatui), 33 tabs, event loop, 50 themes | [tui.md](tui.md) |

### Compliance & Risk

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Compliance | `crates/eggsec/src/compliance/` | HIPAA, PCI, SOC2, OWASP compliance scanning and reporting | [compliance.md](compliance.md) |
| Vuln Management | `crates/eggsec/src/vuln/` | Vulnerability triage, CVSS scoring, prioritization | [vuln.md](vuln.md) |
| Supply Chain | `crates/eggsec/src/supply_chain/` | SBOM generation (CycloneDX, SPDX), typosquat detection | [supply_chain.md](supply_chain.md) |
| Container | `crates/eggsec/src/container/` | Kubernetes/Docker security scanning | [container.md](container.md) |

### Specialized / Lab

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Mobile | `crates/eggsec-mobile-lab/` | APK/IPA static analysis + Android dynamic testing (ADB, Frida, behavioral correlation) | [mobile.md](mobile.md) |
| DB Pentest | `crates/eggsec-db-lab/` + `crates/eggsec/src/db_pentest/` | Database security assessment (Postgres/MySQL/MSSQL/MongoDB/Redis) | [database_pentest.md](database_pentest.md) |
| Post-Exploitation | `crates/eggsec/src/postex/` | LOTL simulation for purple teaming (MITRE ATT&CK mapped, 16 techniques) | [postex.md](postex.md) |
| C2 | `crates/eggsec/src/c2/` | C2 simulation for defense validation | [c2.md](c2.md) |

### Integration & External Services

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| AI/LLM | `crates/eggsec/src/ai/` | AI/LLM client (OpenAI, Anthropic, Azure), cache, planner, script generation | [ai_agents.md](ai_agents.md) |
| NSE | `crates/eggsec-nse/` | Nmap Scripting Engine support (Lua 5.4), 166 library implementations | [nse_integration.md](nse_integration.md) |
| Integrations | `crates/eggsec/src/integrations/` | Jira, GitHub, GitLab external connectors | [integrations.md](integrations.md) |
| Notifications | `crates/eggsec/src/notify/` | Webhook, Slack, Discord, Teams notifications | [notify.md](notify.md) |

### Supporting Modules

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Core Types | `crates/eggsec-core/` | Dependency-light shared types (`Severity`, `SensitiveString`), constants | [types.md](types.md) |
| Tool Core | `crates/eggsec-tool-core/` | Protocol-neutral tool request/response/error/history DTOs | [ai_agents.md](ai_agents.md) |
| Error | `crates/eggsec/src/error/` | Canonical error type with domain-specific variants | [error.md](error.md) |
| Logging | `crates/eggsec/src/logging/` | Structured logging with tracing | [logging.md](logging.md) |
| Utils | `crates/eggsec/src/utils/` | HTTP client, rate limiting, circuit breaker, formatting (23 submodules) | [utils.md](utils.md) |
| Auth Context | `crates/eggsec/src/auth_context/` | Auth context YAML parsing with env var interpolation | [auth_context.md](auth_context.md) |
| Constants | `crates/eggsec/src/constants.rs` | Compatibility facade over core constants + engine-local constants | [constants.md](constants.md) |
| Generated | `crates/eggsec/src/generated/` | Auto-generated protobuf code | [generated.md](generated.md) |
| Python | `crates/eggsec-python/` | Python bindings via PyO3/maturin. 22 stable-core operations. | [docs/python/](../docs/python/) |

---

## Enforcement Model

All side-effecting operations pass through a centralized enforcement gate. The model has three layers:

### Surfaces

`ExecutionSurface` identifies the caller origin. 9 variants:

| Surface | Profile | Overrides | Automated |
|---------|---------|:---------:|:---------:|
| `CliManual` | `ManualPermissive` | Yes | No |
| `TuiManual` | `ManualPermissive` | Yes | No |
| `CliManualStrict` | `ManualGuarded` | No | No |
| `TuiManualStrict` | `ManualGuarded` | No | No |
| `McpServer` | `McpStrict` | No | Yes |
| `RestApi` | `McpStrict` | No | Yes |
| `GrpcApi` | `McpStrict` | No | Yes |
| `SecurityAgent` | `AgentStrict` | No | Yes |
| `Ci` | `CiStrict` | No | Yes |

### Evaluation

`EnforcementContext::evaluate()` is the mandatory pre-dispatch gate:

1. **Scope provenance**: Checks `LoadedScope` (not raw `Scope`) for automated surfaces
2. **Risk assessment**: Maps operation risk tier against profile allowlists
3. **Capability checks**: Positive capability allowlist for strict profiles
4. **Override handling**: ManualPermissive allows operator discretion; strict surfaces never honor overrides
5. **Outcome**: `Allow`, `Warn`, `RequireConfirmation`, or `Deny`

### Type-Level Dispatch

Strict surfaces use `EnforcedDispatcher::dispatch_checked()` requiring an `ApprovedOperation` token. This is structural, not conventional — you cannot dispatch without the token.

```
OperationDescriptor → EnforcementContext::evaluate() → ApprovedOperation → EnforcedDispatcher::dispatch_checked()
```

See [config.md](config.md) for the full enforcement model. See [../docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md) for the complete authorization flow.

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

## Feature Flags

Eggsec uses Cargo feature flags to conditionally compile optional capabilities.

| Flag | Modules | Description |
|------|---------|-------------|
| `stress-testing` | `stress/`, `packet/` | Raw sockets, IP spoofing, DoS tools |
| `packet-inspection` | `packet/` | Live packet capture, traceroute |
| `rest-api` | `tool/protocol/rest` | HTTP REST API server |
| `grpc-api` | `tool/protocol/grpc` | gRPC API server |
| `ws-api` | `tool/protocol/ws` | WebSocket pub/sub |
| `nse` | `eggsec-nse` | Nmap NSE script support |
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
| `wireless` | `wireless/` | WiFi passive recon + security analysis |
| `wireless-advanced` | `wireless/active/` | Active WiFi attacks (deauth/disassoc). Lab-only. |
| `mobile` | `mobile/` | APK/IPA static analysis |
| `mobile-dynamic` | `mobile/` | Android dynamic testing (ADB + Frida) |
| `db-pentest` | `db_pentest/` | Database security assessment. Defense-lab only. |
| `postex` | `postex/` | Post-exploitation simulation. Defense-lab only. |
| `c2` | `c2/` | C2 simulation |
| `web-proxy` | `proxy/intercept/` | MITM web proxy. Defense-lab only. |
| `web-proxy-mcp` | `proxy/mcp.rs` | MCP tool exposure for web proxy |
| `pdf` | `output/pdf` | PDF report generation |
| `full` | All | All features combined |

See [feature_matrix.md](feature_matrix.md) for detailed feature dependencies and [../docs/FEATURE_MATRIX.md](../docs/FEATURE_MATRIX.md) for the canonical feature inventory.

---

## Key Types

### Core Types

| Type | Location | Purpose |
|------|----------|---------|
| `Severity` | `eggsec-core::types` | Canonical severity rating (Critical → Info) |
| `SensitiveString` | `eggsec-core::types` | Zeroized credential wrapper |
| `EggsecConfig` | `config/settings.rs` | Main configuration struct |
| `OutputFormat` | `types.rs` | Report format enum (8 variants) |
| `PayloadType` | `fuzzer/payloads/mod.rs` | 42+ payload categories |
| `EggsecError` | `error/mod.rs` | Canonical error type |
| `Finding` | `findings/mod.rs` | Canonical finding structure |
| `DomainDescriptor` | `domain/mod.rs` | Static metadata descriptor for capability domains |

### Enforcement Types

| Type | Location | Purpose |
|------|----------|---------|
| `ExecutionSurface` | `config/policy.rs` | Caller origin (9 variants) |
| `ExecutionProfile` | `config/policy.rs` | Trust boundary (5 variants) |
| `OperationRisk` | `config/policy.rs` | Risk tier (15 levels) |
| `OperationMetadata` | `config/policy.rs` | Static registry of all operations (single source of truth) |
| `OperationDescriptor` | `config/policy.rs` | Unit of policy evaluation |
| `EnforcementContext` | `config/policy_decision.rs` | Central policy evaluation gate |
| `ApprovedOperation` | `config/policy_decision.rs` | Proof-of-enforcement token |
| `EnforcedDispatcher` | `tool/dispatcher.rs` | Type-level dispatch gate |
| `LoadedScope` | `config/scope.rs` | Scope with provenance |

### Runtime/Daemon Types

| Type | Location | Purpose |
|------|----------|---------|
| `Runtime` | `eggsec-runtime` | Task submit/cancel/snapshot/subscribe |
| `RuntimeTaskExecutor` | `eggsec-runtime` | Frontend-supplied execution logic trait |
| `RunRequest` | `eggsec-runtime` | Task execution request |
| `TaskKind` | `eggsec-runtime` | 27+ task type variants |
| `RuntimeSurface` | `eggsec-runtime` | Frontend-neutral surface identity |
| `ApprovedRunRequest` | `runtime_bridge/bundle.rs` | Coupled approval + request |
| `DaemonHost` | `eggsec-daemon` | Runtime bridge with client registry |
| `ClientRegistry` | `eggsec-daemon` | Connected client tracking |
| `DaemonStore` | `eggsec-daemon` | Persistence trait (SQLite backend) |

---

## Dependency Map

### Crate-Level

```
eggsec-core (leaf — no workspace deps)
    ↑
    ├── eggsec-tool-core     (ToolRequest/Response/Finding/Error DTOs)
    ├── eggsec-output        (report formats, envelope, dedup, trends)
    ├── eggsec-agent         (agent registry, scheduler, lifecycle)
    │
    ├── eggsec-runtime       (no workspace deps — only serde/tokio/tracing)
    │       ↑
    │       ├── eggsec-ui-model  (frontend-neutral view DTOs)
    │       └── eggsec-daemon    (persistent sessions, Unix socket IPC)
    │
    ├── eggsec-db-lab        (database pentest domain)
    ├── eggsec-web-proxy     (MITM proxy domain)
    ├── eggsec-mobile-lab    (mobile analysis domain)
    ├── eggsec-nse           (Nmap NSE/Lua VM)
    │
    └── eggsec               (ALL above — main engine, lib only)
            ↑
            ├── eggsec-tui   (engine + runtime + daemon + ui-model — 33 tabs)
            ├── eggsec-cli   (engine + runtime + ui-model; optional: tui + daemon)
            └── eggsec-python (core + pyo3 — Python bindings)
```

### Dependency Guardrails

Enforced by `scripts/check-architecture-guards.sh`:

- `eggsec-core` has no workspace crate dependencies (leaf crate)
- `eggsec-runtime` has no TUI, transport, or engine dependencies
- `eggsec-output` has no engine or runtime dependencies
- `eggsec-daemon` has no TUI or engine dependencies
- Domain crates (`db-lab`, `web-proxy`, `mobile-lab`, `nse`) depend only on core + output
- The engine (`eggsec`) depends on all other crates but no crate depends on `eggsec` except the frontends

### Intra-Engine Dependencies

Within the `eggsec` crate:

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
| `runtime_bridge` | `config`, `dispatch` (enforcement types) |

---

## Cross-Cutting Concerns

### Error Handling

- **Library code**: `EggsecError` via `Result<T>`
- **Command handlers**: `anyhow::Result` for convenience
- **Bridging**: `.map_err()` converts between types at boundaries
- See [error.md](error.md)

### Configuration

- **Format**: TOML (primary), YAML (secondary)
- **Location**: `~/.config/eggsec/eggsec.toml`
- **Scope enforcement**: `TargetScope` validates targets before scanning
- **Policy evaluation**: All operations route through `EnforcementContext::evaluate()`
- See [config.md](config.md)

### Operation Metadata

`OperationMetadata` is the single source of truth for all externally invokable operations. 32 operations + 33 aliases. Every `OperationDescriptor` is generated from metadata via `descriptor_for_target()`. Alias mapping ensures REST, MCP, gRPC, TUI, and agent tool IDs all resolve to the same canonical metadata.

### Audit Trail

`EnforcementAuditEvent` provides normalized audit records for every enforcement decision across all surfaces. See [audit.md](audit.md).

### Logging & Tracing

- **Framework**: `tracing` with structured spans
- **Sensitive data**: `SensitiveString` with redaction support
- See [logging.md](logging.md)

### Testing

| Test Suite | Command |
|------------|---------|
| Unit tests | `cargo test --lib -p eggsec` |
| Architecture guards | `bash scripts/check-architecture-guards.sh` |
| Full CI | `make check` |

- ~5098 tests (including `#[test]` + `#[tokio::test]`)
- Visual regression: `TestBackend` + `Terminal::new()` for TUI

---

## See Also

### Architecture Documentation

| Category | Documents |
|----------|-----------|
| **Core** | [config.md](config.md), [types.md](types.md), [constants.md](constants.md), [error.md](error.md), [domain_contract.md](domain_contract.md), [../docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md), [../docs/ARCHITECTURE_INVARIANTS.md](../docs/ARCHITECTURE_INVARIANTS.md) |
| **Security** | [scanner.md](scanner.md), [fuzzer.md](fuzzer.md), [waf.md](waf.md), [recon.md](recon.md), [auth.md](auth.md), [hunt.md](hunt.md), [evasion.md](evasion.md), [browser.md](browser.md), [websocket.md](websocket.md) |
| **Infrastructure** | [pipeline.md](pipeline.md), [distributed.md](distributed.md), [proxy.md](proxy.md), [web_proxy.md](web_proxy.md), [loadtest.md](loadtest.md), [storage.md](storage.md), [workflow.md](workflow.md) |
| **Daemon & Runtime** | [daemon.md](daemon.md), [runtime.md](runtime.md), [runtime_bridge.md](runtime_bridge.md), [ui_model.md](ui_model.md) |
| **Output** | [output.md](output.md), [findings.md](findings.md), [diff.md](diff.md) |
| **Integration** | [ai_agents.md](ai_agents.md), [nse_integration.md](nse_integration.md), [integrations.md](integrations.md), [notify.md](notify.md) |
| **UI** | [tui.md](tui.md), [cli_commands.md](cli_commands.md) |
| **Compliance** | [compliance.md](compliance.md), [vuln.md](vuln.md), [supply_chain.md](supply_chain.md), [container.md](container.md) |
| **Specialized** | [mobile.md](mobile.md), [database_pentest.md](database_pentest.md), [postex.md](postex.md), [c2.md](c2.md), [wireless.md](wireless.md) |
| **Utilities** | [utils.md](utils.md), [logging.md](logging.md), [probe.md](probe.md), [networking.md](networking.md), [auth_context.md](auth_context.md) |
| **Reference** | [feature_matrix.md](feature_matrix.md), [defense_lab.md](defense_lab.md), [compile_time_baseline.md](compile_time_baseline.md), [audit.md](audit.md), [generated.md](generated.md) |

---

*Last updated: 2026-07-28 — architecture overview rewrite with runtime_bridge module doc*
