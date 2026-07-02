# Architecture Overview

Eggsec is a high-performance, async-first security testing toolkit built in Rust. This document provides a birds-eye view of the entire system, serving as an index to detailed architecture documentation for each component.

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

Eggsec is organized as a Cargo workspace. The first-level crate boundary is:

- **`eggsec-core`**: dependency-light domain types (`Severity`, `SensitiveString`), constants, and shared primitives. Designed for fast independent compilation with a small dependency set.
- **`eggsec-tool-core`**: core data types for the tool abstraction layer (requests, responses, findings, errors). Dependency-light types shared between `eggsec` and tool protocol integrations.
- **`eggsec`**: main engine, CLI command model/dispatch, assessment modules, remaining API/agent adapters, feature-gated integrations, and the canonical `EggsecError` type.
- **`eggsec-nse`**: optional Nmap NSE compatibility runtime and libraries.
- **`eggsec-tui`**: terminal UI adapter built on `ratatui`/`crossterm`. Depends on Eggsec engine APIs but should not be required for engine-only builds.
- **`eggsec-cli`**: CLI binary entry point. Depends on both `eggsec` and `eggsec-tui`.
- **`eggsec-output`**: report formatting and output adapters (JSON, CSV, HTML, SARIF, JUnit, Markdown). Extracted from `eggsec` to reduce its dependency surface; modules with deep engine coupling (`pdf`, `report`, `report_summary`, `run_manifest`, `attack_graph`) remain in `eggsec`.
- **`eggsec-agent`**: agent coordination primitives extracted from `eggsec::tool::agents` (registry, scheduler, lifecycle, communication, delegation, aggregation). Depends on `eggsec-core` but not the main engine crate.
- **`eggsec-db-lab`**: database pentesting domain crate extracted from `eggsec::db_pentest`. Owns domain execution logic, types, and tests for Postgres/MySQL/MSSQL/MongoDB/Redis security checks. Depends on `eggsec-core` and `eggsec-output` but not the main engine crate.
- **`eggsec-web-proxy`**: web proxy and MITM interception domain crate extracted from `eggsec::proxy`. Owns proxy pool management, intercept server, TLS certificate generation, protocol handlers (WebSocket/HTTP2/gRPC), rule engine, correlation engine, and evidence bundles. Depends on `eggsec-core` and `eggsec-output` but not the main engine crate.
- **`eggsec-mobile-lab`**: mobile app security analysis domain crate extracted from `eggsec::mobile`. Owns APK/IPA static analysis (manifest, permissions, transport config, secrets, debug/backup/exported components) and Android dynamic runtime testing (ADB, Frida instrumentation, behavioral correlation, traffic capture). Depends on `eggsec-core` and `eggsec-output` but not the main engine crate.
- **`eggsec-runtime`**: frontend-neutral runtime DTOs and protocol types for daemon architecture. Dependency-light types shared between frontend adapters (CLI, TUI, REST, MCP, gRPC) and the engine core.

New modules should avoid adding heavy runtime dependencies to `eggsec-core`. Types that depend on `clap`, `reqwest`, `tokio`, `ratatui`, or other heavy crates should remain in the main `eggsec` crate or in `eggsec-tui` as appropriate.

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
| [`recon/`](../crates/eggsec/src/recon/) | DNS enumeration, WHOIS, SSL analysis, subdomain discovery, technology detection, CVE mapping, cloud asset discovery | [recon.md](recon.md) |
| [`scanner/`](../crates/eggsec/src/scanner/) | TCP/UDP port scanning, endpoint discovery, service fingerprinting, IP spoofing | [scanner.md](scanner.md) |
| [`probe.rs`](../crates/eggsec/src/probe.rs) | ICMP probing, probe intent classification, risk assessment | [probe.md](probe.md) |

### Security Testing

| Module | Purpose | Architecture Doc |
|--------|---------|------------------|
| [`fuzzer/`](../crates/eggsec/src/fuzzer/) | Security fuzzing engine with 40 payload types (SQLi, XSS, SSRF, Path Traversal, ReDoS, etc.) | [fuzzer.md](fuzzer.md) |
| [`waf/`](../crates/eggsec/src/waf/) | WAF detection (34 products), bypass techniques, evasion-resistance testing | [waf.md](waf.md) |
| [`auth/`](../crates/eggsec/src/auth/) | Authentication testing (brute force, credential stuffing, MFA bypass, lockout/rate-limit/timing; JWT/OAuth/IDOR handled in pipeline via fuzzer). TUI `AuthTab` (`Tab::Auth`) fully integrated (TabSpec, task system, policy enforcement, session save/restore). | [auth.md](auth.md) |
| [`hunt/`](../crates/eggsec/src/hunt/) | Advanced threat hunting (authorization bypass, race conditions, advanced injection) | [hunt.md](hunt.md) |
| [`browser/`](../crates/eggsec/src/browser/) | Headless browser for DOM XSS detection, SPA crawling | [browser.md](browser.md) |
| [`websocket/`](../crates/eggsec/src/websocket/) | WebSocket security testing | [websocket.md](websocket.md) |

### Performance & Stress

| Module | Purpose | Architecture Doc |
|--------|---------|------------------|
| [`loadtest/`](../crates/eggsec/src/loadtest/) | HTTP load testing with detailed latency metrics, concurrency control | [loadtest.md](loadtest.md) |
| [`stress/`](../crates/eggsec/src/stress/) | Network stress testing (SYN, UDP, HTTP, TCP, ICMP floods), IP spoofing | [stress.md](stress.md) |
| [`packet/`](../crates/eggsec/src/packet/) | Packet capture, crafting, parsing (pnet-based), traceroute | [networking.md](networking.md) |

### Orchestration & Pipeline

| Module | Purpose | Architecture Doc |
|--------|---------|------------------|
| [`pipeline/`](../crates/eggsec/src/pipeline/) | Chained security assessment profiles (18 built-in profiles) | [pipeline.md](pipeline.md) |
| [`tool/`](../crates/eggsec/src/tool/) | Unified tool registry, execution framework, MCP/OpenAI protocol integration; core DTOs in `eggsec-tool-core` | [ai_agents.md](ai_agents.md) |
| [`agent/`](../crates/eggsec/src/agent/) | Autonomous security agent with scheduling, longitudinal memory, portfolio management | [ai_agents.md](ai_agents.md) |
| [`agent/enforcement.rs`](../crates/eggsec/src/agent/enforcement.rs) | Agent scan enforcement helpers (risk/capability mapping from scan depth and type) | [ai_agents.md](ai_agents.md) |
| [`distributed/`](../crates/eggsec/src/distributed/) | Worker/coordinator cluster architecture for parallel scanning | [distributed.md](distributed.md) |

### Infrastructure & Output

| Module | Purpose | Architecture Doc |
|--------|---------|------------------|
| [`output/`](../crates/eggsec/src/output/) | Compatibility facade over `eggsec-output` plus engine-coupled report modules (PDF, report, report_summary, run_manifest, attack_graph) | [output.md](output.md) |
| [`proxy/`](../crates/eggsec/src/proxy/) | SOCKS4, SOCKS5, HTTP, HTTPS, Tor proxy pool with health checking, rotation strategies; `proxy/intercept/` submodule for MITM web proxy (feature-gated `web-proxy`) | [proxy.md](proxy.md) |
| [`config/`](../crates/eggsec/src/config/) | TOML/YAML configuration loading, scope enforcement, TUI settings | [config.md](config.md) |
| [`storage/`](../crates/eggsec/src/storage/) | SQLx-based PostgreSQL persistence for findings and scan history | [storage.md](storage.md) |
| [`workflow/`](../crates/eggsec/src/workflow/) | Finding lifecycle management (assignment, SLA tracking, status transitions) | [workflow.md](workflow.md) |

### Compliance & Risk

| Module | Purpose | Architecture Doc |
|--------|---------|------------------|
| [`compliance/`](../crates/eggsec/src/compliance/) | HIPAA, PCI, SOC2, OWASP compliance scanning and reporting | [compliance.md](compliance.md) |
| [`vuln/`](../crates/eggsec/src/vuln/) | Vulnerability triage, CVSS scoring, prioritization | [vuln.md](vuln.md) |
| [`supply_chain/`](../crates/eggsec/src/supply_chain/) | SBOM generation (CycloneDX, SPDX), typosquat detection | [supply_chain.md](supply_chain.md) |
| [`container/`](../crates/eggsec/src/container/) | Kubernetes/Docker security scanning | [container.md](container.md) |

### Specialized / Lab

| Module | Purpose | Architecture Doc |
|--------|---------|------------------|
| [`mobile/`](../crates/eggsec/src/mobile/) | Thin adapter for mobile domain crate. Re-exports `eggsec-mobile-lab` types and provides CLI bridging. Static analysis (APK/IPA) and dynamic testing (ADB/Frida) owned by `eggsec-mobile-lab`. | [mobile.md](mobile.md) |
| [`eggsec-mobile-lab`](../crates/eggsec-mobile-lab/) | Mobile app security analysis domain crate: APK/IPA static analysis + Android dynamic runtime testing (ADB, Frida, behavioral correlation, traffic capture). Defense-lab only. | [mobile.md](mobile.md) |
| [`db_pentest/`](../crates/eggsec/src/db_pentest/) | Direct database security assessment (Postgres/MySQL/MSSQL/MongoDB/Redis; Phase 1-5: checks + correlation + compliance + optional MCP). Defense-lab only. TUI tab, native pipeline stage, evidence bundles. | [database_pentest.md](database_pentest.md) |
| [`postex/`](../crates/eggsec/src/postex/) | Post-exploitation and LOTL simulation for purple teaming (MITRE ATT&CK mapped; 16 techniques across 4 categories: LOTL, persistence, lateral movement, credential access). Defense-lab only; dry-run always safe. | [postex.md](postex.md) |

### Integration & External Services

| Module | Purpose | Architecture Doc |
|--------|---------|------------------|
| [`ai/`](../crates/eggsec/src/ai/) | AI/LLM client (OpenAI, Anthropic, Azure), cache, planner, script generation, WAF bypass suggestions | [ai_agents.md](ai_agents.md) |
| [`eggsec-nse/`](../crates/eggsec-nse/) | Nmap Scripting Engine support (Lua 5.4), 166 NSE libraries | [nse_integration.md](nse_integration.md) |
| [`integrations/`](../crates/eggsec/src/integrations/) | Jira, GitHub, GitLab external connectors | [integrations.md](integrations.md) |
| [`notify/`](../crates/eggsec/src/notify/) | Webhook, Slack, Discord, Teams notifications | [notify.md](notify.md) |

### User Interfaces

| Module | Purpose | Architecture Doc |
|--------|---------|------------------|
| [`cli/`](../crates/eggsec/src/cli/) | Command-line argument parsing (clap-based), 49 commands | [cli_commands.md](cli_commands.md) |
| [`tui/`](../crates/eggsec-tui/src/) | Real-time terminal UI (ratatui-based), 33 tabs, event loop | [tui.md](tui.md) |

### Supporting Modules

| Module | Purpose | Architecture Doc |
|--------|---------|------------------|
| [`eggsec-core`](../crates/eggsec-core/) | Dependency-light shared types (Severity, SensitiveString), constants | [types.md](types.md) |
| [`eggsec-tool-core`](../crates/eggsec-tool-core/) | Protocol-neutral tool request/response/error/history DTOs | [ai_agents.md](ai_agents.md) |
| [`eggsec-output`](../crates/eggsec-output/src/) | Portable report formatting and output adapters (JSON, CSV, HTML, SARIF, JUnit, Markdown) | [output.md](output.md) |
| [`domain/`](../crates/eggsec/src/domain/) | Domain module contract — static metadata descriptors for capability domains | [domain_contract.md](domain_contract.md) |
| [`types.rs`](../crates/eggsec/src/types.rs) | Main-crate compatibility facade plus CLI-facing types such as `OutputFormat` | [types.md](types.md) |
| [`constants.rs`](../crates/eggsec/src/constants.rs) | Compatibility facade over core constants plus any engine-local constants | [constants.md](constants.md) |
| [`error/`](../crates/eggsec/src/error/) | Canonical error type with domain-specific variants | [error.md](error.md) |
| [`findings/`](../crates/eggsec/src/findings/) | Finding store, lifecycle management, fingerprinting | [findings.md](findings.md) |
| `diff/` (distributed across `output/`, `fuzzer/`, `waf/`) | Scan result diffing, baseline comparison | [diff.md](diff.md) |
| [`logging/`](../crates/eggsec/src/logging/) | Structured logging with tracing | [logging.md](logging.md) |
| [`utils/`](../crates/eggsec/src/utils/) | 23 submodules (HTTP client, rate limiting, circuit breaker, formatting) | [utils.md](utils.md) |
| [`auth_context/`](../crates/eggsec/src/auth_context/) | Auth context YAML parsing with env var interpolation | [auth_context.md](auth_context.md) |
| [`generated/`](../crates/eggsec/src/generated/) | Auto-generated protobuf code | [generated.md](generated.md) |
| [`wireless/`](../crates/eggsec/src/wireless/) | WiFi scanning (passive recon + security analysis + rogue heuristic; --repeat, --known-good, --dry-run, --detect-suspicious; WPS/hidden/transition) + active deauth/disassoc (Phase 1 complete 2026-06-12, under `wireless-advanced`; lab-only, requires `--allow-active-wireless`) | [wireless.md](wireless.md) |
| `mobile/` | Thin adapter for mobile domain crate. Re-exports `eggsec-mobile-lab` types and provides CLI bridging. Domain crate owns static APK/IPA analysis and dynamic Android testing. | [mobile.md](mobile.md) |

---

## User Interfaces

### CLI (`cli/`)

The command-line interface is built with `clap` and provides 49 commands organized into functional groups:

```
eggsec scan      # Port scanning, service fingerprinting
eggsec fuzz      # Vulnerability fuzzing
eggsec waf       # WAF detection and bypass
eggsec recon     # Reconnaissance operations
eggsec load      # Load testing
eggsec agent     # Autonomous agent control
eggsec pipeline  # Pipeline profile execution
```

- **Entry point**: `crates/eggsec/src/cli/mod.rs`
- **Handlers**: `crates/eggsec/src/commands/handlers/`
- **Documentation**: [cli_commands.md](cli_commands.md)

### TUI (`tui/`)

The terminal user interface uses `ratatui` with 33 tabs organized by function. The TUI is now a separate crate (`eggsec-tui`), extracted from the main `eggsec` crate.

| Tab Group | Tabs |
|-----------|------|
| **Dashboard** | Overview, Session, Scan Progress |
| **Recon** | Targets, DNS, Subdomains, SSL, Technologies, CVEs |
| **Scanning** | Ports, Endpoints, Services, Spoof Config |
| **Security** | Fuzzer, WAF, Auth, Hunt, Browser |
| **Infrastructure** | Proxy (Intercept), Load Test, Stress, Packets |
| **Intelligence** | Findings, Workflow, Compliance, Vulns |
| **Agent** | Portfolio, Skills, Schedule, Memory |
| **System** | Config, Scope, Logs, About |

- **Documentation**: [tui.md](tui.md)

### REST API & Agent Protocols (`tool/`)

Machine-accessible interfaces for automation and AI integration:

| Protocol | Feature Flag | Purpose |
|----------|--------------|---------|
| REST API | `rest-api` | HTTP API server for agent integration; uses `EnforcementContext` with `McpStrict` profile by default; only `Allow` permits dispatch, `Warn`/`RequireConfirmation`/`Deny` return 403 with structured `POLICY_DENIED` response; dispatch via `EnforcedDispatcher` |
| gRPC | `grpc-api` | High-performance gRPC API; uses `EnforcementContext` with `McpStrict` profile; only `Allow` permits dispatch, `Warn`/`RequireConfirmation`/`Deny` fail with `Status::permission_denied`; dispatch via `EnforcedDispatcher` |
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
| **Service Detection** | 42+ protocol fingerprints |
| **Endpoint Discovery** | 223 built-in path signatures, custom wordlist support |
| **IP Spoofing** | Raw socket spoofing (feature-gated) |
| **Timing Presets** | Paranoid, Sneaky, Polite, Normal, Aggressive, Insane |

- **Documentation**: [scanner.md](scanner.md)

### Fuzzer (`fuzzer/`)

Mutation-based security fuzzing engine with 40 payload types:

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
| **Profile Management** | 18 built-in scan profiles |

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
| **defense-lab** | Baseline diff and defense validation |
| **synvoid-local** | Localhost SYN scan testing |
| **waf-regression** | WAF detection regression testing |
| **protocol-edge** | Protocol edge case testing |
| **nse-safe** | Safe NSE script execution |
| **db-regression** | Database pentest regression (native `Stage::DbPentest` when `db-pentest` feature enabled) |
| **web-proxy** | Web proxy interception (requires `web-proxy` feature) |

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

### NSE (`eggsec-nse/`)

Nmap Scripting Engine compatibility:

| Component | Description |
|-----------|-------------|
| **Lua VM** | Lua 5.4 via mlua crate |
| **Libraries** | 166 NSE-compatible library modules |
| **CVE Integration** | NVD, OSV, CISA KEV feeds |
| **Sandbox** | Restricted Lua operation execution |

- **Documentation**: [nse_integration.md](nse_integration.md)

---

## Feature Flags

Eggsec uses Cargo feature flags to conditionally compile optional capabilities:

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
| `wireless` | `wireless/` | WiFi scanning (passive recon + security analysis + rogue heuristic; --repeat/--known-good/--dry-run/--detect-suspicious; WPS/hidden/transition). **Passive = Phase 0 (2026-06-11)**; active gated by `wireless-advanced` (see `plans/wireless-active-attacks-loadout-design-plan.md`). |
| `wireless-advanced` | `wireless/active/` | Active WiFi attacks (deauth/disassoc frame crafting + injection). Lab-only (`--allow-active-wireless`); Phase 1 complete 2026-06-12. |
| `mobile` | `mobile/` | Static analysis of Android APKs and iOS IPAs (APK/IPA manifest/config checks). Domain crate: `eggsec-mobile-lab`. |
| `mobile-dynamic` | `mobile/` | Android dynamic testing (ADB + Frida + behavioral correlation). Domain crate: `eggsec-mobile-lab`. |
| `pdf` | `output/pdf` | PDF report generation |
| `db-pentest` | `db_pentest/` | Direct database security assessment (Postgres/MySQL/MSSQL/MongoDB/Redis; Phase 1-5: checks + correlation + compliance + optional MCP via `db-pentest-mcp` marker). Defense-lab only. TUI tab + native pipeline stage. Domain crate: `eggsec-db-lab`. |
| `postex` | `postex/` | Post-exploitation and LOTL simulation for purple teaming (MITRE ATT&CK mapped; 16 techniques across 4 categories: LOTL, persistence, lateral movement, credential access). Defense-lab only; dry-run always safe; real requires `--allow-postex`. |
| `web-proxy` | `proxy/intercept/` | Standalone defense-lab MITM web proxy for HTTP/HTTPS/WebSocket/HTTP2/gRPC traffic interception. Dry-run always safe; real interception requires `--allow-web-proxy` + policy. TUI `Tab::Intercept` (interactive flow inspection, editing, HAR export, manipulation audit trail). Domain crate: `eggsec-web-proxy`. |
| `web-proxy-mcp` | `proxy/mcp.rs` | Optional MCP tool exposure for web proxy (12 tools: list flows, inspect flow, edit request/response, manage rules, session save/load, HAR export, evidence bundle). Marker feature; requires `web-proxy`. |
| `full` | All | All features combined |

See [feature_matrix.md](feature_matrix.md) for detailed feature dependencies and [docs/FEATURE_MATRIX.md](../docs/FEATURE_MATRIX.md) for the canonical feature inventory with categories, naming conventions, and build profiles.

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
| `EggsecConfig` | `config/settings.rs` | Main configuration struct |
| `Severity` | `eggsec-core::types` (re-exported by `types.rs`) | Canonical severity rating (Critical→Info) |
| `SensitiveString` | `eggsec-core::types` (re-exported by `types.rs`) | Zeroized credential wrapper |
| `OutputFormat` | `types.rs` | Report format enum (8 variants) |
| `PayloadType` | `fuzzer/payloads/mod.rs` | 40 payload categories |
| `EggsecError` | `error/mod.rs` | Canonical error type |
| `TargetScope` | `config/scope.rs` | Target scope enforcement |
| `Finding` | `findings/mod.rs` | Canonical finding structure |
| `ProbeIntent` | `probe.rs` | Probe classification intent |
| `ProbeRisk` | `probe.rs` | Probe risk level assessment |
| `DomainDescriptor` | `domain/mod.rs` | Static metadata descriptor for a capability domain |
| `DomainCategory` | `domain/mod.rs` | Classification of domain types (StandardAssessment, DefenseLab, etc.) |

### Module-Specific Types

| Module | Key Type | Purpose |
|--------|----------|---------|
| Config | `ApprovedOperation` | Proof-of-enforcement token for type-level dispatch (Phase 12) |
| Config | `EnforcementError` | Structured error from `approve()`/`approve_manual()` |
| Config | `EnforcedDispatcher` | `ToolDispatcher` wrapper requiring approval token before dispatch |
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
                │ eggsec-core │
                └──────┬───────┘
                       │
                ┌──────┴───────┐
                │   eggsec    │
                └──────┬───────┘
                       │
         ┌─────────────┼─────────────────┐
         │             │                 │
         ▼             ▼                 ▼
    ┌─────────┐  ┌──────────┐    ┌─────────────┐
    │  config │  │  scanner │    │  eggsec-nse│
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
| `tui` | `config`, `commands`, `output` (in `eggsec-tui`) |
| `ai` | `config`, `error`, `types` |
| `nse` | `scanner`, `recon` (via Lua bindings) |

---

## Cross-Cutting Concerns

### Error Handling

- **Library code**: Uses `EggsecError` via `Result<T>`
- **Command handlers**: Use `anyhow::Result` for convenience
- **Bridging**: `.map_err()` converts between types at boundaries
- See [error.md](error.md) for error variant catalog

### Configuration

- **File format**: TOML (primary), YAML (secondary)
- **Location**: `~/.config/eggsec/eggsec.toml`
- **Scope enforcement**: `TargetScope` validates targets before scanning
- **TUI settings**: Partial save with field exposure control
- **Profile management**: 18 built-in scan profiles
- **Policy evaluation**: All operations route through central `EnforcementContext::evaluate(descriptor)` (`config/policy_decision.rs`) which performs LoadedScope provenance, DenialClass downgrade (ManualPermissive only), positive capability checks for strict, and risk/feature/policy enforcement. Command handlers use `CommandContext::evaluate_and_enforce_operation()` which wraps it. REST API dispatch goes through `EnforcementContext::for_surface(ExecutionSurface::RestApi, ...)` and evaluates before every tool execution, using `McpStrict` profile by default. Legacy direct `evaluate_operation_policy` is internal for base decisions; denial paths prefer the central evaluator. **Preflight**: `preflight_operation()` wraps the same `evaluate()` path for read-only policy checks across all surfaces (CLI, TUI, REST, MCP, agent) before dispatch, returning `PreflightResult` with outcome, confirmation classes, and suggested CLI flags. See [config.md](config.md).
- See [config.md](config.md) for details

### Operation Metadata

`OperationMetadata` provides a centralized, static registry of all externally invokable operations and is the **single source of truth** for `OperationDescriptor` generation. Each entry declares the operation's ID, display name, mode, risk tier, intended uses, required features, required capabilities, target policy, and protocol exposure flags (`rest_exposable`, `mcp_exposable`, `grpc_exposable`).

The registry lives in `config::policy` and is accessible from all surfaces:
- `eggsec::config::operation_metadata(id)` — canonical ID lookup
- `eggsec::config::metadata_for_tool_id(tool_id)` — resolves aliases to canonical IDs
- `eggsec::config::all_operation_metadata()` — returns the full static array

Every `OperationDescriptor` is generated from `OperationMetadata` via `descriptor_for_target()`. This eliminates drift between REST, MCP, gRPC, TUI, and agent descriptor construction. Alias mapping (32 entries) ensures that alternate tool IDs (REST tool names, MCP tool names, registry IDs, gRPC tool IDs) all resolve to the same canonical metadata.

**Tool Registration Builder** (`tool::registration`): Derives per-protocol tool listings from `OperationMetadata` and `DomainDescriptor` `ToolIntegration`. Builder functions (`mcp_tool_registrations()`, `mcp_tool_registrations_default_visible()`, `rest_tool_registrations()`, `grpc_tool_registrations()`, `agent_tool_registrations()`) filter by exposure flags, replacing direct `registry.list()` calls. Each `ToolRegistration` carries tool ID, operation ID, exposure flags (`mcp_metadata_exposable`, `mcp_default_visible`, `rest_exposable`, `grpc_exposable`, `agent_exposable`), source (`Base`/`FeatureGated`/`Domain`), and optional MCP feature gate. Registration is computed metadata — it does not grant authorization and is not a third static registry. MCP listing uses **Model A** (profile-expanded visibility): `mcp_tool_registrations("ops-agent")` returns every `mcp_metadata_exposable` tool, not the conservative default. The conservative subset is `mcp_tool_registrations_default_visible()`. See [TOOL_REGISTRATION.md](../docs/TOOL_REGISTRATION.md).

### Normalized Audit Events (Phase 10)

`audit.rs` provides a single `EnforcementAuditEvent` model for consistent audit records across all execution surfaces. Every meaningful enforcement decision (allow, warn, deny, confirmation-required, confirmed override) produces an audit event with surface, profile, operation, target, outcome, scope provenance, and optional correlation ID.

Key functions:
- `audit_event_from_enforcement_outcome()` - builds events from enforcement decisions
- `audit_event_from_preflight()` - builds events from preflight evaluations
- `emit_audit_event()` - logs at appropriate tracing level

Manual confirmations record class and reason. Automated surfaces (REST, MCP, Agent, CI) never record accepted manual overrides.

### Type-Level Enforcement Dispatch (Phase 12)

Phase 12 moved enforcement from convention (call sites expected to evaluate first) to type-level structure. Strict programmatic surfaces cannot dispatch a tool without an `ApprovedOperation` token, enforced structurally rather than by convention.

**`ApprovedOperation`** (`config/policy_decision.rs`): A proof-of-enforcement token with private fields, produced exclusively by `EnforcementContext::approve()` (strict surfaces) or `approve_manual()` (manual surfaces). Read-only accessors: `descriptor()`, `decision()`, `surface()`, `profile()`, `audit_event_id()`. Cannot be constructed outside enforcement code.

**`EnforcementError`** (`config/policy_decision.rs`): Structured error from `approve()`/`approve_manual()` with three variants:
- `Denied { decision }` - Policy denied the operation (covers `Deny` and `Warn` on strict surfaces).
- `ConfirmationRequired { decision, required_classes }` - Manual confirmation needed.
- `ManualOverrideUnavailable { surface, decision }` - Override not supported on this surface.

**`EnforcedDispatcher`** (`tool/dispatcher.rs`): Wrapper around `ToolDispatcher` that requires an `ApprovedOperation` before dispatch via `dispatch_checked()`. Verifies the request's tool name and target match the approved descriptor, failing closed on any mismatch. Used by REST, MCP, and Agent dispatch paths.

**Approval methods:**
- `EnforcementContext::approve(surface, descriptor)` - Strict: only `Allow` outcomes produce a token. `Warn`, `RequireConfirmation`, and `Deny` all fail with `EnforcementError`.
- `EnforcementContext::approve_manual(surface, descriptor, manual_override)` - Manual permissive: supports `Warn` (approved with warning) and `RequireConfirmation` when matching `ManualOverride` flags are present. Strict/automated surfaces reject overrides.

**Surface dispatch flow (REST example):**
1. Build `OperationDescriptor` from `OperationMetadata`.
2. `let approved = enforcement.approve(ExecutionSurface::RestApi, descriptor)?;`
3. Build `ToolRequest`.
4. `dispatcher.dispatch_checked(&approved, request).await`

MCP, Agent, CI, and high-risk TUI direct-launch paths follow the same pattern. CLI/TUI manual dispatch uses `approve_manual()` to support discretion classes.

**Current adoption:** REST (`tool/protocol/rest.rs`), MCP (`tool/protocol/mcp/handlers/server.rs`), Agent (`agent/mod.rs`), gRPC (`tool/protocol/grpc.rs`), and TUI task dispatch (`eggsec-tui/src/app/mod.rs`) all use `EnforcedDispatcher` + `ApprovedOperation`.

### Logging & Tracing

- **Framework**: `tracing` with structured spans
- **Formats**: Pretty (human), JSON (machine)
- **Levels**: Error, Warn, Info, Debug, Trace
- **Sensitive data**: `SensitiveString` with redaction support
- See [logging.md](logging.md)

### Testing

| Test Suite | Command |
|------------|---------|
| Unit tests | `cargo test --lib -p eggsec` |
| TUI tests | `cargo test --lib -p eggsec-tui` |
| Integration tests | `cargo test --test scanner_tests -p eggsec` |
| Negative tests | `cargo test --test negative_tests -p eggsec` |
| Clippy | `cargo clippy --lib -p eggsec` |

- **Test count**: ~5098 (includes #[test] + #[tokio::test])
- **Visual regression**: `TestBackend` + `Terminal::new()` for TUI

---

## Defense-Lab Mode

Eggsec supports local, repeatable profiles against defensive systems for regression testing:

| Profile | Purpose |
|---------|---------|
| `DefenseLab` | Baseline diff and defense validation |
| `SynvoidLocal` | Localhost SYN scan testing |
| `WafRegression` | WAF detection regression testing |
| `ProtocolEdge` | Protocol edge case testing |
| `NseSafe` | Safe NSE script execution |
| `DbRegression` | Database pentest regression (native `Stage::DbPentest` when `db-pentest` feature enabled; falls back to defense-lab stages) |

See [defense_lab.md](defense_lab.md) for detailed documentation.

---

## See Also

### Architecture Documentation

| Category | Documents |
|----------|-----------|
| **Core** | [config.md](config.md), [types.md](types.md), [constants.md](constants.md), [error.md](error.md), [domain_contract.md](domain_contract.md), [../docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md), [../docs/ARCHITECTURE_INVARIANTS.md](../docs/ARCHITECTURE_INVARIANTS.md) |
| **Security** | [scanner.md](scanner.md), [fuzzer.md](fuzzer.md), [waf.md](waf.md), [recon.md](recon.md), [auth.md](auth.md), [hunt.md](hunt.md) |
| **Infrastructure** | [pipeline.md](pipeline.md), [distributed.md](distributed.md), [proxy.md](proxy.md), [web_proxy.md](web_proxy.md), [loadtest.md](loadtest.md) |
| **Output** | [output.md](output.md), [findings.md](findings.md), [diff.md](diff.md), [workflow.md](workflow.md) |
| **Integration** | [ai_agents.md](ai_agents.md), [nse_integration.md](nse_integration.md), [integrations.md](integrations.md), [notify.md](notify.md) |
| **UI** | [tui.md](tui.md), [cli_commands.md](cli_commands.md) |
| **Compliance** | [compliance.md](compliance.md), [vuln.md](vuln.md), [supply_chain.md](supply_chain.md), [container.md](container.md) |
| **Utilities** | [utils.md](utils.md), [logging.md](logging.md), [probe.md](probe.md) |
| **Reference** | [feature_matrix.md](feature_matrix.md), [defense_lab.md](defense_lab.md), [compile_time_baseline.md](compile_time_baseline.md), [auth_context.md](auth_context.md) |

### Implementation Plan

All implementation items are complete.

---

*Last updated: 2026-06-30*
