# Architecture Overview

Eggsec is a Rust-native, scope-enforced security assessment and defense-validation engine with multiple frontends (CLI, TUI, REST, MCP, gRPC, Agent), centralized policy enforcement, and domain execution crates. This document provides a birds-eye view of the entire system and serves as an index to detailed architecture documentation for each component.

## Quick Navigation

Jump to a topic by role:

| I want to understand... | Start here |
|--------------------------|------------|
| **How the system is laid out** (crates, deps, layers) | [Workspace Crates](#workspace-crates) and [System Architecture](#system-architecture) |
| **A specific module** (scanner, fuzzer, recon, etc.) | [Module Index](#module-index) — each entry links to a deep-dive doc |
| **How policy/enforcement works** | [Enforcement Model](#enforcement-model) |
| **How data flows through the system** | [Data Flow](#data-flow) |
| **What features are available and what they gate** | [Feature Flags](#feature-flags) |
| **The core types I'll encounter** | [Key Types](#key-types) |
| **How crates depend on each other** | [Dependency Map](#dependency-map) |
| **Error handling, config, logging patterns** | [Cross-Cutting Concerns](#cross-cutting-concerns) |
| **How the daemon/runtime bridge works** | [daemon.md](daemon.md), [runtime.md](runtime.md), [runtime_bridge.md](runtime_bridge.md) |
| **Python bindings** | [python_api.md](python_api.md) |

## Table of Contents

- [Quick Navigation](#quick-navigation)
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

Eggsec is organized as a Cargo workspace with 16 crates. The first-level crate boundary separates dependency-light leaf crates from the composition root and frontends.

### Release validation boundary

Release validation is owned by `scripts/release-check.sh` and
`scripts/release-package-graph.py`. The default path runs Cargo's workspace
packager (`cargo package --workspace --no-verify --target-dir <isolated-target>`)
with private packages explicitly excluded, then checks an exact JSONL archive
inventory and parses every extracted manifest with standalone
`cargo metadata --no-deps --offline`. Registry-sensitive
`cargo publish --dry-run` checks are optional locally but must be performed in
dependency layers immediately before manual publication. Neither validation
nor hosted CI publishes a package.

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
| `eggsec-daemon-protocol` | Daemon IPC protocol | Yes | Session/task DTOs, client registry. No persistence/TLS deps. |
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

This is the complete index of all modules and components. Each entry links to its detailed architecture document. Modules are grouped by functional area; the **Source** column shows where the code lives, and the **Architecture Doc** links to the deep-dive.

### Reconnaissance & Discovery

The discovery layer gathers intelligence about a target before active testing. These modules are typically the first stage in any assessment pipeline.

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Recon | `crates/eggsec/src/recon/` | 30+ sub-modules: DNS enumeration, WHOIS, SSL/TLS analysis, subdomain discovery, technology detection, CVE mapping, cloud asset discovery, email/security analysis, secrets detection | [recon.md](recon.md) |
| Scanner | `crates/eggsec/src/scanner/` | TCP/UDP port scanning (connect + SYN), endpoint discovery (223 built-in paths), service fingerprinting with confidence scoring, IP spoofing, Nmap-style T0-T5 timing presets | [scanner.md](scanner.md) |
| Probe | `crates/eggsec/src/probe.rs` | ICMP host discovery, probe intent classification (`ProbeIntent`), risk assessment (`ProbeRisk`) shared across scan profiles | [probe.md](probe.md) |
| Wireless | `crates/eggsec/src/wireless/` | WiFi passive recon + security analysis + rogue AP heuristic; active deauth/disassoc under `wireless-advanced` (lab-only) | [wireless.md](wireless.md) |

### Security Testing

Active vulnerability discovery modules. Each sends crafted input to targets and analyzes responses for signs of vulnerability.

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Fuzzer | `crates/eggsec/src/fuzzer/` | Security fuzzing engine with 42+ payload types (SQLi, XSS, SSRF, SSTI, IDOR, OAuth, JWT, GraphQL, gRPC, etc.), Aho-Corasick leak detection, timing analysis, response diffing, grammar fuzzer | [fuzzer.md](fuzzer.md) |
| WAF | `crates/eggsec/src/waf/` | WAF detection (34 products) via fingerprinting, bypass technique library, evasion-resistance regression testing | [waf.md](waf.md) |
| Auth | `crates/eggsec/src/auth/` | Authentication testing: brute force, credential stuffing, MFA bypass, lockout detection, rate-limit analysis, timing attacks | [auth.md](auth.md) |
| Hunt | `crates/eggsec/src/hunt/` | Advanced threat hunting: authorization bypass, race conditions, advanced injection patterns (feature-gated: `advanced-hunting`) | [hunt.md](hunt.md) |
| Browser | `crates/eggsec/src/browser/` | Headless browser for DOM XSS detection, SPA route discovery, client-side security checks (feature-gated: `headless-browser`) | [browser.md](browser.md) |
| WebSocket | `crates/eggsec/src/websocket/` | WebSocket security testing: protocol upgrade, message fuzzing, cross-site WebSocket hijacking (feature-gated: `websocket`) | [websocket.md](websocket.md) |
| Evasion | `crates/eggsec/src/evasion/` | Evasion technique detection for defense validation, MITRE ATT&CK mapped (feature-gated: `evasion`) | [evasion.md](evasion.md) |

### Performance & Stress

Modules for testing system resilience, throughput, and behavior under load.

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Load Test | `crates/eggsec/src/loadtest/` | HTTP load testing with hdrhistogram latency percentiles, concurrency control, rate limiting, warm-up phases | [loadtest.md](loadtest.md) |
| Stress | `crates/eggsec/src/stress/` | Network stress testing: SYN/UDP/HTTP/TCP/ICMP floods, IP spoofing, raw sockets (feature-gated: `stress-testing`) | [stress.md](stress.md) |
| Packet | `crates/eggsec/src/packet/` | Packet capture, crafting, parsing (pnet-based), hexdump, traceroute (feature-gated: `packet-inspection` or `stress-testing`) | [networking.md](networking.md) |

### Orchestration & Pipeline

Modules that coordinate, schedule, and chain other modules into complete assessments.

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Pipeline | `crates/eggsec/src/pipeline/` | Chained security assessment profiles: 18 built-in profiles (Quick, Endpoint, Web, WAF, Full, API, Recon, Stealth, Deep, etc.) | [pipeline.md](pipeline.md) |
| Tool Registry | `crates/eggsec/src/tool/` | Unified tool registry, execution framework, MCP/REST/gRPC/agent protocol integration (feature-gated: `tool-api`/`rest-api`/`grpc-api`) | [ai_agents.md](ai_agents.md) |
| Agent | `crates/eggsec/src/agent/` | Autonomous security agent: event-driven scheduling, longitudinal memory, portfolio management, alert routing (feature-gated: `rest-api`) | [ai_agents.md](ai_agents.md) |
| Distributed | `crates/eggsec/src/distributed/` | Worker/coordinator cluster architecture: PSK-authenticated, TLS, task distribution, result aggregation | [distributed.md](distributed.md) |

### Infrastructure & Output

Configuration, persistence, reporting, and supporting infrastructure.

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Config | `crates/eggsec/src/config/` | TOML/YAML configuration loading, scope enforcement (`TargetScope`), policy model (`OperationMetadata`, `EnforcementContext`), execution profiles, feature registry | [config.md](config.md) |
| Output | `crates/eggsec-output/src/` + `crates/eggsec/src/output/` | Multi-format report generation: JSON, CSV, HTML, SARIF, JUnit, Markdown, PDF; envelope wrapping, deduplication, trend analysis, baseline comparison | [output.md](output.md) |
| Proxy | `crates/eggsec/src/proxy/` | SOCKS4/5, HTTP, HTTPS, Tor proxy pool with health checking, rotation, and failover | [proxy.md](proxy.md) |
| Web Proxy | `crates/eggsec-web-proxy/` | MITM web proxy domain: HTTP/HTTPS/WebSocket/HTTP2/gRPC interception, TLS cert generation, protocol handlers (feature-gated: `web-proxy`) | [web_proxy.md](web_proxy.md) |
| Storage | `crates/eggsec/src/storage/` | SQLx-based PostgreSQL persistence for findings and scan history (feature-gated: `database`) | [storage.md](storage.md) |
| Workflow | `crates/eggsec/src/workflow/` | Finding lifecycle management: assignment, SLA tracking, status transitions (feature-gated: `finding-workflow`) | [workflow.md](workflow.md) |
| Findings | `crates/eggsec/src/findings/` | Canonical `Finding` schema, finding store, lifecycle management, fingerprinting for deduplication | [findings.md](findings.md) |
| Diff | (distributed) | Scan result diffing, baseline comparison, regression detection | [diff.md](diff.md) |
| Domain Contract | `crates/eggsec/src/domain/` | `DomainDescriptor` static metadata: declares what each capability domain can do without performing authorization | [domain_contract.md](domain_contract.md) |

### Daemon & Runtime

The daemon architecture enables persistent sessions, background task execution, and multi-client connectivity.

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Daemon | `crates/eggsec-daemon/` | Long-running daemon host: session persistence (SQLite via rusqlite), client registry, Unix socket IPC, optional HTTP/SSE transport | [daemon.md](daemon.md) |
| Runtime | `crates/eggsec-runtime/` | Frontend-neutral async runtime: `Runtime` orchestrator, `RuntimeTaskExecutor` trait, task lifecycle, session management, event broadcasting. Dependency-light (serde/tokio/tracing only) | [runtime.md](runtime.md) |
| Runtime Bridge | `crates/eggsec/src/runtime_bridge/` | Converts `eggsec-runtime` DTOs (`RuntimeSurface`, `RunRequest`, `TaskKind`) to engine enforcement types; preflight policy preview, approval binding, dispatch validation | [runtime_bridge.md](runtime_bridge.md) |
| UI Model | `crates/eggsec-ui-model/` | Frontend-neutral view DTOs: `SessionView`, `TaskView`, `ResultEnvelopeView`, `DashboardSummaryView`; renderer registry | [ui_model.md](ui_model.md) |

### User Interfaces

Frontend entry points for human interaction.

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| CLI | `crates/eggsec/src/cli/` + `crates/eggsec-cli/` | Command-line interface: clap-based argument parsing, 52 commands, handler dispatch, logging setup, daemon CLI integration | [cli_commands.md](cli_commands.md) |
| TUI | `crates/eggsec-tui/src/` | Real-time terminal UI: ratatui/crossterm, 33 tabs, event loop, 50 LZMA-compressed themes, search, session management, visual regression testing | [tui.md](tui.md) |

### Compliance & Risk

Modules for regulatory compliance, vulnerability management, and supply chain security.

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Compliance | `crates/eggsec/src/compliance/` | HIPAA, PCI DSS, SOC2, OWASP Top 10 compliance scanning and reporting (feature-gated: `compliance`) | [compliance.md](compliance.md) |
| Vuln Management | `crates/eggsec/src/vuln/` | Vulnerability triage, CVSS scoring, prioritization (feature-gated: `vuln-management`) | [vuln.md](vuln.md) |
| Supply Chain | `crates/eggsec/src/supply_chain/` | SBOM generation (CycloneDX, SPDX), typosquat detection, dependency vulnerability checking (feature-gated: `sbom`) | [supply_chain.md](supply_chain.md) |
| Container | `crates/eggsec/src/container/` | Kubernetes/Docker security scanning, CIS benchmark checks (feature-gated: `container`) | [container.md](container.md) |

### Specialized / Lab

Defense-lab and specialized testing domains. These operate in controlled environments with explicit authorization.

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Mobile | `crates/eggsec-mobile-lab/` | APK/IPA static analysis (manifest, permissions, code signing, secrets) + Android dynamic testing via ADB/Frida with behavioral correlation (feature-gated: `mobile`/`mobile-dynamic`) | [mobile.md](mobile.md) |
| DB Pentest | `crates/eggsec-db-lab/` + `crates/eggsec/src/db_pentest/` | Database security assessment: Postgres/MySQL/MSSQL/MongoDB/Redis checks, compliance mapping, baseline comparison (feature-gated: `db-pentest`) | [database_pentest.md](database_pentest.md) |
| Post-Exploitation | `crates/eggsec/src/postex/` | LOTL (Living-off-the-Land) simulation for purple teaming, MITRE ATT&CK mapped, 16 techniques (feature-gated: `postex`) | [postex.md](postex.md) |
| C2 | `crates/eggsec/src/c2/` | C2 framework simulation for defense validation (feature-gated: `c2`, requires `postex` + `evasion`) | [c2.md](c2.md) |

### Integration & External Services

Modules that connect to external platforms and AI services.

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| AI/LLM | `crates/eggsec/src/ai/` | AI/LLM client (OpenAI, Anthropic, Azure), response cache, adaptive planner, script generation, smart WAF bypass (feature-gated: `ai-integration`) | [ai_agents.md](ai_agents.md) |
| NSE | `crates/eggsec-nse/` | Nmap Scripting Engine compatibility: Lua 5.4 VM, 166 library implementations, sandbox, script resolver, profile system (feature-gated: `nse`) | [nse_integration.md](nse_integration.md) |
| Integrations | `crates/eggsec/src/integrations/` | Jira, GitHub, GitLab external connectors for ticket creation and issue tracking (feature-gated: `external-integrations`) | [integrations.md](integrations.md) |
| Notifications | `crates/eggsec/src/notify/` | Webhook, Slack, Discord, Microsoft Teams notification delivery | [notify.md](notify.md) |

### Supporting Modules

Shared types, utilities, and cross-cutting infrastructure used by all other modules.

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Core Types | `crates/eggsec-core/` | Dependency-light shared primitives: `Severity` (5 levels), `SensitiveString` (zeroized), constants. Zero workspace deps — the leaf crate | [types.md](types.md) |
| Tool Core | `crates/eggsec-tool-core/` | Protocol-neutral DTOs: `ToolRequest`, `ToolResponse`, `ToolError`, history types. No engine deps | [ai_agents.md](ai_agents.md) |
| Error | `crates/eggsec/src/error/` | `EggsecError` canonical error type with 18 domain-specific variants, `From` impls for reqwest/toml/json/url | [error.md](error.md) |
| Logging | `crates/eggsec/src/logging/` | Structured logging with `tracing`, subscriber/appender setup (feature-gated: `logging-subscriber`) | [logging.md](logging.md) |
| Utils | `crates/eggsec/src/utils/` | 22 utility sub-modules: HTTP client, caching, circuit breaker, rate limiting, formatting, stealth, rate-limited clients | [utils.md](utils.md) |
| Auth Context | `crates/eggsec/src/auth_context/` | Auth context YAML parsing with environment variable interpolation for credential management | [auth_context.md](auth_context.md) |
| Constants | `crates/eggsec/src/constants.rs` | Compatibility facade over `eggsec-core` constants + engine-local constants (WAF count validation, timing defaults) | [constants.md](constants.md) |
| Generated | `crates/eggsec/src/generated/` | Auto-generated protobuf/gRPC code (checked-in, regenerated via `build.rs`) | [generated.md](generated.md) |
| Python | `crates/eggsec-python/` | Python bindings via PyO3/maturin: 22 stable-core operations, sync/async clients, feature-gated provisional domains | [python_api.md](python_api.md) |

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
    │       ├── eggsec-daemon-protocol (IPC DTOs, client registry)
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

*Last updated: 2026-08-15 — Enhanced module index with capability summaries, quick-nav, and Python deep-dive link*
