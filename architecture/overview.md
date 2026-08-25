# Architecture Overview

Eggsec is a Rust-native, scope-enforced security assessment and defense-validation engine with multiple frontends (CLI, TUI, REST, MCP, gRPC, Agent), centralized policy enforcement, and domain execution crates. This document is the bird's-eye view of the system and the index into the per-component deep-dive documents that live alongside it in `architecture/`.

Every number in this document was verified against source on 2026-08-25. Where a claim depends on a count (variants, commands, tabs), the verifying location is cited.

## Quick Navigation

| I want to understand... | Start here |
|--------------------------|------------|
| **The crates and how they layer** | [Workspace Crates](#workspace-crates) |
| **How an operation flows end-to-end** | [How an Operation Flows](#how-an-operation-flows) |
| **A specific module** (scanner, fuzzer, recon, ...) | [Module Index](#module-index) — each row links to a deep-dive doc |
| **Policy/enforcement mechanics** | [Enforcement Model](#enforcement-model), [config.md](config.md) |
| **Task dispatch and the runtime bridge** | [dispatch.md](dispatch.md), [runtime_bridge.md](runtime_bridge.md) |
| **What features gate what** | [Feature Flags](#feature-flags), [feature_matrix.md](feature_matrix.md) |
| **Core types** | [Key Types](#key-types) |
| **Crate dependency rules** | [Dependency Map](#dependency-map) |
| **Full deep-dive doc catalog** | [Deep-Dive Index](#deep-dive-index) |

## Table of Contents

- [Quick Navigation](#quick-navigation)
- [Workspace Crates](#workspace-crates)
- [System Architecture](#system-architecture)
- [How an Operation Flows](#how-an-operation-flows)
- [Module Index](#module-index)
- [Enforcement Model](#enforcement-model)
- [Data Flow](#data-flow)
- [Feature Flags](#feature-flags)
- [Key Types](#key-types)
- [Dependency Map](#dependency-map)
- [Cross-Cutting Concerns](#cross-cutting-concerns)
- [Deep-Dive Index](#deep-dive-index)
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
| `eggsec-core` | Shared primitives | Yes | `Severity` (5 levels), `SensitiveString` (zeroize + constant-time eq), constants. Zero internal deps. |
| `eggsec-tool-core` | Protocol-neutral DTOs | Yes | `ToolRequest`, `ToolResponse`, `ToolError`, history/rate-limit types. Depends only on `eggsec-core`. |
| `eggsec-output` | Report formatting | Yes | JSON/CSV/HTML/SARIF/JUnit/Markdown, envelope, dedup, trends, diff, scheduling. No engine/runtime deps. PDF lives in the engine crate, not here. |
| `eggsec-agent` | Agent coordination | Yes | Registry, scheduler, lifecycle, delegation, aggregation. Internal deps: `eggsec-core` only. |
| `eggsec-runtime` | Frontend-neutral runtime | Yes | `Runtime`, `RuntimeTaskExecutor`, task lifecycle; zero workspace deps (serde/tokio/tracing only). |
| `eggsec-ui-model` | Frontend view DTOs | Yes | View models + renderer registry (23 entries). Depends only on `eggsec-runtime`. |
| `eggsec` | Main engine (lib) | No | Composition root: all security modules, policy enforcement, dispatch, runtime bridge. |
| `eggsec-nse` | NSE compatibility | Yes | Lua 5.4 VM (mlua), 166 library implementations / 44 registered descriptors, sandbox, ScriptResolver. Optional. |
| `eggsec-db-lab` | DB pentest domain | Yes | Postgres/MySQL/MSSQL/MongoDB/Redis checks, each driver behind its own feature. |
| `eggsec-web-proxy` | Web proxy domain | Yes | MITM intercept (HTTP/HTTPS/WS/H2/gRPC), TLS cert generation, proxy pool. Highest test density in the workspace. |
| `eggsec-mobile-lab` | Mobile analysis domain | Yes | APK/IPA static analysis + Android dynamic testing (`mobile-dynamic`). |
| `eggsec-daemon` | Persistent daemon host | Yes | Unix socket server, session lifecycle, SQLite (rusqlite 0.31 bundled); optional `http-api` SSE transport, optional `full-executor`. |
| `eggsec-daemon-protocol` | Daemon IPC protocol | Yes | `ClientCommand` (14), `ServerMessage` (11), `ErrorCode` (11), RBAC client registry. Depends only on `eggsec-runtime`. |
| `eggsec-tui` | Terminal UI | No | 33 tabs (21 base + 12 feature-gated), ratatui/crossterm, 50 LZMA-packaged themes. |
| `eggsec-cli` | CLI binary | Yes | Thin binary shell over the engine's `cli` feature; optional `tui` and `daemon-client`. |
| `eggsec-python` | Python bindings | Yes | PyO3/maturin. 22 stable-core operations, each with sync + async paths (asserted by test). |

**Dependency direction**: Leaf crates have no internal workspace dependencies (except where noted above). The main `eggsec` crate is the composition root. Only `eggsec-cli`, `eggsec-tui`, and `eggsec-python` sit above it.

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
│            (commands/handlers/ · dispatch/ · tool/)                  │
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

## How an Operation Flows

Every externally invokable operation takes the same path, regardless of frontend:

1. **Parse** — the frontend turns user input into a request (CLI args → `Commands` enum variant; REST/MCP/gRPC → `ToolRequest`; TUI/daemon → `RunRequest`).
2. **Describe** — the request resolves to an `OperationDescriptor` derived from `OperationMetadata` (the single source of truth; 31 canonical operations + 42 aliases).
3. **Evaluate** — `EnforcementContext::evaluate()` checks scope provenance (`LoadedScope` for automated surfaces), risk tier vs. profile allowlist, capabilities, and features. Outcome: `Allow` / `Warn` / `RequireConfirmation` / `Deny`.
4. **Approve** — evaluation yields an `ApprovedOperation` token. Strict surfaces dispatch only through `EnforcedDispatcher::dispatch_checked()`, which re-validates tool+target binding against the token and fails closed.
5. **Execute** — either a command handler (`crates/eggsec/src/commands/handlers/`, 32 handler modules behind `handle_command()`) or the domain executor layer (`crates/eggsec/src/dispatch/executors/`) calls the engine function.
6. **Report** — results flow back as typed values or `TaskResult`s, are wrapped by the output envelope, and can be persisted (findings store, storage backend) or exported (JSON/SARIF/JUnit/HTML/CSV/Markdown/PDF).

Key structural point verified in review: the engine modules themselves (scanner, recon, fuzzer, ...) are **policy-free executors**. They take plain config structs (`PortScanConfig`, `ReconRequest`, ...) and never touch `EnforcementContext`. All authorization happens upstream in steps 3–4. See [dispatch.md](dispatch.md) and [config.md](config.md).

---

## Module Index

This is the complete index of all modules and components. Each entry links to its detailed architecture document. The **Source** column shows where the code lives; the **Architecture Doc** links to the deep-dive.

### Reconnaissance & Discovery

The discovery layer gathers intelligence about a target before active testing. These modules are typically the first stage in any assessment pipeline.

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Recon | `crates/eggsec/src/recon/` | ~30 source files: 20 declared modules + 7 detached utilities. Full-recon pipeline runs 17 modules (`FULL_RECON_PIPELINE_MODULES`): reverse DNS, geoIP, threat intel, SSL/TLS analysis, WHOIS, subdomain enumeration, DNS records, tech detection, JS analysis, wayback, cloud assets, content discovery, CORS, email discovery, subdomain takeover, CVE mapping (NVD), secrets detection (31 secret types). Entry: `run_full_recon_from_request()` (`recon/runner.rs:536`) | [recon.md](recon.md) |
| Scanner | `crates/eggsec/src/scanner/` | TCP port scanning (+ raw spoofed SYN/NULL/FIN/Xmas via `SpoofConfig`), endpoint discovery with **347 built-in paths** (`DEFAULT_ENDPOINTS`, `endpoints.rs:95`), service fingerprinting with confidence scoring (**47 probes**, CPE + possible-CVE output), UDP fingerprinting, Nmap-style T0–T5 timing presets, CMS scanning (WordPress/Drupal/Joomla), Nuclei-compatible template engine with signing/marketplace | [scanner.md](scanner.md) |
| Probe | `crates/eggsec/src/probe.rs` | Shared probe-risk vocabulary: `ProbeIntent` (7 variants), `ProbeRisk` (6 tiers, converts to `OperationRisk`); used by scanner/NSE/WAF/loadtest risk budgeting | [probe.md](probe.md) |
| Wireless | `crates/eggsec/src/wireless/` | Passive WiFi recon via `iwlist` parsing, rogue-AP/evil-twin heuristics, temporal scan diffing; active deauth/disassoc frame crafting + raw injection under `wireless-advanced` (lab-only, root + monitor mode) | [wireless.md](wireless.md) |

### Security Testing

Active vulnerability discovery modules. Each sends crafted input to targets and analyzes responses for signs of vulnerability.

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Fuzzer | `crates/eggsec/src/fuzzer/` | Security fuzzing engine, **40 `PayloadType` variants** (`fuzzer/payloads/mod.rs:49`), Aho-Corasick leak detection, timing analysis, response diffing, grammar/stateful fuzzers, chained requests, per-target profiles (Apache/PHP/nginx), calibration. Always compiled | [fuzzer.md](fuzzer.md) |
| WAF | `crates/eggsec/src/waf/` | Detection of **34 WAF signatures/products** (`waf/data/patterns.rs`, includes one generic catch-all), block-page comparison, bypass library (evasion/headers/smuggling/profiles), regression reporting. Always compiled; shares types bidirectionally with fuzzer | [waf.md](waf.md) |
| Auth | `crates/eggsec/src/auth/` | Brute force, credential stuffing, lockout detection, MFA bypass, rate-limit analysis, session tests, timing attacks, password policy; multi-protocol (FTP/SSH/SMTP) under `nse-ssh2` | [auth.md](auth.md) |
| Hunt | `crates/eggsec/src/hunt/` | Authorization bypass, business logic, race conditions, attack chains, session issues (`run_hunt()`); feature-gated: `advanced-hunting` | [hunt.md](hunt.md) |
| Browser | `crates/eggsec/src/browser/` | Headless browser: DOM XSS, SPA route discovery, client-side security checks, corpus; real impl behind `headless-browser`, error stub otherwise | [browser.md](browser.md) |
| WebSocket | `crates/eggsec/src/websocket/` | Connection handling, message fuzzing, injection, origin checks; live tests behind `websocket` feature (report types always available) | [websocket.md](websocket.md) |
| Evasion | `crates/eggsec/src/evasion/` | Evasion technique **detection** for defense validation: 16 techniques across 6 categories, each MITRE ATT&CK mapped (test-enforced); feature-gated: `evasion` | [evasion.md](evasion.md) |
| API Schema | `crates/eggsec/src/api_schema/` | Standalone OpenAPI 3.0 (JSON/YAML) parser → fuzz-target generation, independent type hierarchy from `fuzzer/api_schema/`; feature-gated: `api-schema` | [api_schema.md](api_schema.md) |

### Performance & Stress

Modules for testing system resilience, throughput, and behavior under load.

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Load Test | `crates/eggsec/src/loadtest/` | HTTP load testing with hdrhistogram p50/p90/p95/p99, concurrency control, rate limiting, warm-up | [loadtest.md](loadtest.md) |
| Stress | `crates/eggsec/src/stress/` | Network stress testing: SYN, UDP, HTTP, ICMP floods implemented; **TCP flood is declared but not implemented** (`stress/mod.rs:140` returns an error). Raw sockets + IP spoofing; feature-gated: `stress-testing` (authorization/metrics/warning always compiled) | [stress.md](stress.md) |
| Packet | `crates/eggsec/src/packet/` | Packet capture, crafting, parsing (pnet-based), hexdump, traceroute; feature-gated: `packet-inspection` (CLI surface needs `cli` too) | [networking.md](networking.md) |

### Orchestration & Pipeline

Modules that coordinate, schedule, and chain other modules into complete assessments.

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Pipeline | `crates/eggsec/src/pipeline/` | Chained assessments: **18 `ScanProfile` variants** (`types.rs:123`: Quick … WebProxy), stage context/session/report/executor split | [pipeline.md](pipeline.md) |
| Dispatch | `crates/eggsec/src/dispatch/` | Frontend-neutral task execution: `dispatch_task()`/`dispatch_inner()` route `TaskKind` (29 variants) → engine workers, returning typed `TaskResult`s over channels; registry-backed executor adapters (Scanner/Recon/Waf/Fuzz/Network always; NSE/DB-pentest gated) | [dispatch.md](dispatch.md) |
| Tool Registry | `crates/eggsec/src/tool/` | `ToolRegistry` (FxHashMap + RwLock) registering 11 base tools (+ gated proxy/db-pentest/c2 tools), protocol servers (REST/MCP/gRPC/agent/AI routes/OpenAI-compatible), and `EnforcedDispatcher` with fail-closed binding validation; protocols behind `rest-api`/`grpc-api` | [ai_agents.md](ai_agents.md), [cli_commands.md](cli_commands.md) |
| Agent | `crates/eggsec/src/agent/` | Autonomous security agent: event-driven scheduling, longitudinal memory, portfolio management, alert routing (engine crate side; coordination primitives live in `eggsec-agent`) | [ai_agents.md](ai_agents.md) |
| Distributed | `crates/eggsec/src/distributed/` | Worker/coordinator cluster (`RemoteListener`/`RemoteClient`): PSK auth with constant-time compare, TLS, heartbeats, task queue, result aggregation | [distributed.md](distributed.md) |

### Infrastructure & Output

Configuration, persistence, reporting, and supporting infrastructure.

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Config | `crates/eggsec/src/config/` | TOML/YAML config loading, scope model (`Scope`/`LoadedScope`/`TargetScope`/`AddressClass`), policy model (`OperationMetadata`, `EnforcementContext`), budgets, presets, feature registry | [config.md](config.md) |
| Output | `crates/eggsec-output/src/` + `crates/eggsec/src/output/` | Report generation: JSON, CSV, HTML, SARIF, JUnit, Markdown from `eggsec-output`; PDF via printpdf lives in the engine crate behind the `pdf` feature. Envelope wrapping, dedup, trend/baseline/diff analysis | [output.md](output.md) |
| Proxy | `crates/eggsec/src/proxy/` | Facade over `eggsec-web-proxy`: pool/rotator/health-check for SOCKS4/SOCKS5/HTTP/HTTPS/Tor (`ProxyType`, 5 variants); stubs when `web-proxy` disabled | [proxy.md](proxy.md) |
| Web Proxy | `crates/eggsec-web-proxy/` | MITM web proxy domain: HTTP/HTTPS/WebSocket/HTTP2/gRPC interception, on-the-fly TLS cert generation (`CertGenerator`), evidence bundles, RBAC rules; feature-gated: `web-proxy` | [web_proxy.md](web_proxy.md) |
| Storage | `crates/eggsec/src/storage/` | SQLx PostgreSQL persistence for findings/scan history (`PgPool`); feature-gated: `database` | [storage.md](storage.md) |
| Workflow | `crates/eggsec/src/workflow/` | Finding lifecycle: assignment, comments, SLA tracking, status transitions; feature-gated: `finding-workflow` | [workflow.md](workflow.md) |
| Findings | `crates/eggsec/src/findings/` | Canonical `Finding` schema (17 fields), `FindingStore` (JSONL persistence), lifecycle states, fingerprints for dedup; `FindingType` (9), `Confidence` (5), `EvidenceKind` (13) | [findings.md](findings.md) |
| Diff | (in `eggsec-output` + engine) | Scan result diffing, baseline comparison, regression detection | [diff.md](diff.md) |
| Domain Contract | `crates/eggsec/src/domain/` | Static `DomainDescriptor` metadata (3 domains today: `db-pentest`, `mobile-static`, `mobile-dynamic`, all DefenseLab category); declares capability without authorizing it; `required_feature` gates availability | [domain_contract.md](domain_contract.md) |

### Daemon & Runtime

The daemon architecture enables persistent sessions, background task execution, and multi-client connectivity.

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Daemon | `crates/eggsec-daemon/` | Long-running host: session persistence (SQLite via rusqlite, bundled), client registry, Unix socket IPC, optional HTTP/SSE transport (`http-api`), optional full engine executor (`full-executor`) | [daemon.md](daemon.md) |
| Daemon Protocol | `crates/eggsec-daemon-protocol/` | Wire DTOs + RBAC: `ClientCommand` (14), `ServerMessage` (11), `ErrorCode` (11), `ClientRegistry` with roles/permissions; no persistence/TLS deps | [daemon.md](daemon.md) |
| Runtime | `crates/eggsec-runtime/` | Frontend-neutral async runtime: `Runtime` orchestrator, `RuntimeTaskExecutor` trait, task lifecycle, sessions, event broadcasting; zero workspace deps | [runtime.md](runtime.md) |
| Runtime Bridge | `crates/eggsec/src/runtime_bridge/` | Converts `RuntimeSurface`→`ExecutionSurface` (1:1 except `Unknown`→error), `RunRequest`/`TaskKind`→`OperationDescriptor`; `preflight_run_request()` preview, `approve_run_request()` issues `ApprovedOperation`; manual overrides honored only on ManualPermissive surfaces | [runtime_bridge.md](runtime_bridge.md) |
| UI Model | `crates/eggsec-ui-model/` | View DTOs (`SessionView`, `TaskView`, `ResultEnvelopeView`, `DashboardSummaryView`, event/artifact/permission/policy-prompt views) + renderer registry (23 entries) keyed by task kind | [ui_model.md](ui_model.md) |

### User Interfaces

Frontend entry points for human interaction.

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| CLI | `crates/eggsec/src/cli/` + `crates/eggsec/src/commands/handlers/` + `crates/eggsec-cli/` | **52 clap subcommands** (27 unconditional, 25 feature-gated; `cli/mod.rs`). Argument types only in `cli/`; handlers (32 modules) in `commands/handlers/`; `crates/eggsec-cli/` is the thin binary shell (surface resolution, logging, optional TUI/daemon client) | [cli_commands.md](cli_commands.md) |
| TUI | `crates/eggsec-tui/src/` | Real-time terminal UI: ratatui/crossterm, **33 tabs (21 base + 12 feature-gated)** (`tabs/mod.rs:142`), tab-spec table, event loop with daemon attach mode, **50 LZMA-packaged themes**, search, overlays, `TestBackend` visual regression suite | [tui.md](tui.md) |

### Compliance & Risk

Modules for regulatory compliance, vulnerability management, and supply chain security.

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Compliance | `crates/eggsec/src/compliance/` | Framework scans + reports for OWASP, PCI DSS, HIPAA, SOC2 (one module each); feature-gated: `compliance` | [compliance.md](compliance.md) |
| Vuln Management | `crates/eggsec/src/vuln/` | CVSS scoring, exploitability, asset criticality, triage, prioritization, remediation; feature-gated: `vuln-management` | [vuln.md](vuln.md) |
| Supply Chain | `crates/eggsec/src/supply_chain/` | SBOM generation (CycloneDX + SPDX exporters), typosquat detection, dependency scanning; feature-gated: `sbom` | [supply_chain.md](supply_chain.md) |
| Container | `crates/eggsec/src/container/` | Docker image analysis, Kubernetes API scanning, CIS benchmark checks, escape-path detection; feature-gated: `container` | [container.md](container.md) |

### Specialized / Lab

Defense-lab and specialized testing domains. These operate in controlled environments with explicit authorization.

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Mobile | `crates/eggsec-mobile-lab/` | APK/IPA static analysis (manifest, permissions, signing, secrets) always compiled; ADB/Frida dynamic testing + traffic capture behind `mobile-dynamic` | [mobile.md](mobile.md) |
| DB Pentest | `crates/eggsec-db-lab/` + `crates/eggsec/src/db_pentest/` | Postgres/MySQL/MSSQL/MongoDB/Redis assessment, compliance mapping, baselines/correlation, evidence bundles; drivers individually feature-gated; feature-gated: `db-pentest` | [database_pentest.md](database_pentest.md) |
| Post-Exploitation | `crates/eggsec/src/postex/` | Purple-team simulation: **16 techniques across 4 categories** (LOTL, Persistence, Lateral Movement, Credential Access — 4 each, `postex/mod.rs:236`), MITRE mapped, dry-run safe; feature-gated: `postex` | [postex.md](postex.md) |
| C2 | `crates/eggsec/src/c2/` | C2 framework simulation (agents/beacons/campaigns/tasking/opsec; campaign profiles incl. APT29, Carbanak); standalone defense-lab surface, deliberately not wired to MCP/TUI/pipeline; feature `c2 = ["postex", "evasion"]` | [c2.md](c2.md) |

### Integration & External Services

Modules that connect to external platforms and AI services.

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| AI/LLM | `crates/eggsec/src/ai/` | Multi-provider client (OpenAI, Anthropic, Azure, OpenAI-compatible), response cache, adaptive planner + script generation behind `ai-integration`; WAF-bypass suggestions, payload suggestion | [ai_agents.md](ai_agents.md) |
| NSE | `crates/eggsec-nse/` | Nmap Scripting Engine compatibility: Lua 5.4 VM (mlua), **166 library implementation files** exposing the NSE stdlib, **44 registered library descriptors**, `ScriptResolver`, sandbox (`SandboxConfig`), execution profiles, CVE integration; feature-gated: `nse` | [nse_integration.md](nse_integration.md), [nse_capability_inventory.md](nse_capability_inventory.md) |
| Integrations | `crates/eggsec/src/integrations/` | Jira, GitHub, GitLab connectors behind a common `IssueTracker` trait; feature-gated: `external-integrations` | [integrations.md](integrations.md) |
| Notifications | `crates/eggsec/src/notify/` | Webhook delivery plus Slack/Discord/MS Teams channels via `NotifyManager`; always compiled | [notify.md](notify.md) |

### Supporting Modules

Shared types, utilities, and cross-cutting infrastructure used by all other modules.

| Module | Source | Purpose | Architecture Doc |
|--------|--------|---------|------------------|
| Core Types | `crates/eggsec-core/` | `Severity` (Critical/High/Medium/Low/Info), `SensitiveString` (zeroized on drop, constant-time compare), shared constants; zero workspace deps | [types.md](types.md) |
| Tool Core | `crates/eggsec-tool-core/` | Protocol-neutral DTOs: `ToolRequest`, `ToolResponse`, `ToolError`, finding/history/rate-limit types, cancellation tokens | [ai_agents.md](ai_agents.md) |
| Error | `crates/eggsec/src/error/` | `EggsecError` with **23 variants** spanning config/target/network/http/parse/policy/proxy domains, ergonomic `From` impls | [error.md](error.md) |
| Logging | `crates/eggsec/src/logging/` | tracing init: Pretty/Json/Compact formats (`LogFormat`); subscriber/appender setup also ships as the portable `logging-subscriber` feature for process hosts | [logging.md](logging.md) |
| Utils | `crates/eggsec/src/utils/` | **20 utility sub-modules**: HTTP client, caching, circuit breaker, client pool, rate limiter, redaction, stealth, service detection, target/validation helpers, formatting (`strip_controls`), privilege (gated) | [utils.md](utils.md) |
| Auth Context | `crates/eggsec/src/auth_context/` | Auth-context YAML parsing with env-var interpolation; applies credentials to requests | [auth_context.md](auth_context.md) |
| Constants | `crates/eggsec/src/constants.rs` | Facade over `eggsec-core` constants + compile-time validation (`SUPPORTED_WAF_COUNT` = 34 asserted at compile time) | [constants.md](constants.md) |
| Audit | `crates/eggsec/src/audit.rs` | `EnforcementAuditEvent` (15 fields) normalized audit record for every enforcement/preflight decision; `AuditOutcome` (5 variants) | [audit.md](audit.md) |
| Generated | `crates/eggsec/src/generated/` | Checked-in protobuf/gRPC code, regenerated via `build.rs` (protoc needed only for descriptor set) | [generated.md](generated.md) |
| Python | `crates/eggsec-python/` | PyO3 bindings: `_core` module, `Engine`/`AsyncEngine`, 22 stable-core operations (exhaustiveness test-enforced), feature-gated provisional/experimental domains | [python_api.md](python_api.md) |

---

## Enforcement Model

All side-effecting operations pass through a centralized enforcement gate. The model has three layers:

### Surfaces

`ExecutionSurface` identifies the caller origin — 9 variants (`config/policy.rs:357`):

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

Supporting vocabularies (same file): `ExecutionProfile` (5 variants), `OperationRisk` (15 tiers, `Passive` → `AgentAutonomous`), `Capability` (19), `IntendedUse` (8), `DenialClass` (8), `ConfirmationClass` (8).

### Evaluation

`EnforcementContext::evaluate()` (`config/policy_decision.rs:561`) is the mandatory pre-dispatch gate:

1. **Scope provenance**: automated surfaces require `LoadedScope` from an explicit manifest (`DefaultEmpty` scope + networked op ⇒ `Deny(ScopeMissing)`)
2. **Risk assessment**: operation risk tier against profile allowlists
3. **Capability checks**: positive capability allowlist for strict profiles
4. **Override handling**: ManualPermissive honors operator discretion; strict surfaces reject `ManualOverride` outright (enforced again in the runtime bridge)
5. **Outcome**: `Allow`, `Warn`, `RequireConfirmation`, or `Deny` — each carrying a `PolicyDecision`

### Type-Level Dispatch

Strict surfaces use `EnforcedDispatcher::dispatch_checked()` (`tool/dispatcher.rs`), which requires an `ApprovedOperation` token produced only by `EnforcementContext::approve()`/`approve_manual()`. Token binding is re-validated against the request (tool name ↔ canonical operation, target normalization, typed-vs-parameter agreement) and **fails closed** on any mismatch.

```
OperationDescriptor → EnforcementContext::evaluate() → ApprovedOperation → EnforcedDispatcher::dispatch_checked()
```

See [config.md](config.md) for the full enforcement model, [dispatch.md](dispatch.md) for the execution layer, and [../docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md) for the complete authorization flow.

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

Eggsec uses Cargo feature flags to conditionally compile optional capabilities. Verified gates live in `crates/eggsec/src/lib.rs`.

| Flag | Modules | Description |
|------|---------|-------------|
| `stress-testing` | `stress/` flood engines, scanner ICMP probe | SYN/UDP/HTTP/ICMP floods, raw sockets, IP spoofing |
| `packet-inspection` | `packet/` | Live packet capture, crafting, traceroute |
| `rest-api` | `tool/protocol/*` (REST/MCP/ws/agent/AI routes) | HTTP REST + MCP API servers |
| `grpc-api` | `tool/protocol/grpc` | gRPC API server |
| `ws-api` | `tool/protocol/ws` | WebSocket pub/sub |
| `nse` | `eggsec-nse`, engine `nse_tool` | Nmap NSE script support (Lua VM) |
| `nse-ssh2` | NSE SSH2 libs, `auth/multi_protocol` | SSH2/libssh2 support |
| `nse-sandbox` | NSE sandbox | Restrict dangerous Lua operations |
| `ai-integration` | `ai/planner`, `ai/script_gen` | AI planner, script generation |
| `websocket` | `websocket/` live tests | WebSocket security testing |
| `headless-browser` | `browser/` real impl | DOM XSS and SPA crawling |
| `database` | `storage/` SQLx ops | PostgreSQL persistence |
| `container` | `container/` | Kubernetes/Docker scanning |
| `sbom` | `supply_chain/sbom` | SBOM generation |
| `advanced-hunting` | `hunt/` | Advanced threat hunting |
| `compliance` | `compliance/` | Compliance scanning (OWASP/PCI/HIPAA/SOC2) |
| `external-integrations` | `integrations/` | Jira, GitHub, GitLab |
| `finding-workflow` | `workflow/` | Finding lifecycle management |
| `vuln-management` | `vuln/` | Vulnerability triage |
| `wireless` | `wireless/` | WiFi passive recon + security analysis |
| `wireless-advanced` | `wireless/active/` | Deauth/disassoc injection. Lab-only |
| `mobile` | `eggsec-mobile-lab` static | APK/IPA static analysis |
| `mobile-dynamic` | `eggsec-mobile-lab` dynamic | Android dynamic testing (ADB + Frida) |
| `db-pentest` | `db_pentest/`, `eggsec-db-lab` | Database security assessment. Defense-lab only |
| `postex` | `postex/` | Post-exploitation simulation. Defense-lab only |
| `c2` | `c2/` | C2 simulation (`= ["postex", "evasion"]`) |
| `web-proxy` | `proxy/` facade, `eggsec-web-proxy` deps | MITM web proxy. Defense-lab only |
| `pdf` | engine `output/pdf` (printpdf) | PDF report generation |
| `email-notifications` | rest-api email transport | SMTP email via lettre |
| `logging-subscriber` | process-host crates | tracing subscriber/appender setup |
| `config-watch` | config hot-reload | File watching (notify + debouncer) |
| `full` | All | All features combined |

Marker features (gate code without adding dependencies) include `tool-api`, `api-schema`, `git-secrets`, `cloud`, and the `*-mcp` exposure markers. See [feature_matrix.md](feature_matrix.md) for dependency edges and [../docs/FEATURE_MATRIX.md](../docs/FEATURE_MATRIX.md) for the canonical inventory.

---

## Key Types

### Core Types

| Type | Location | Purpose |
|------|----------|---------|
| `Severity` | `eggsec-core::types` | Canonical severity rating (Critical/High/Medium/Low/Info) |
| `SensitiveString` | `eggsec-core::types` | Zeroized credential wrapper, constant-time compare |
| `EggsecConfig` | `config/settings.rs` | Main configuration struct |
| `OutputFormat` | `types.rs` | Report format enum |
| `PayloadType` | `fuzzer/payloads/mod.rs:49` | Exactly 40 payload categories |
| `ScanProfile` | `types.rs:123` | 18 pipeline profile variants |
| `EggsecError` | `error/mod.rs:44` | Canonical error type, 23 variants |
| `Finding` | `findings/mod.rs` | Canonical finding structure (17 fields) |
| `DomainDescriptor` | `domain/mod.rs` | Static metadata descriptor for capability domains |
| `TaskKind` | `eggsec-runtime/src/request.rs:53` | 29 frontend-neutral task variants |

### Enforcement Types

| Type | Location | Purpose |
|------|----------|---------|
| `ExecutionSurface` | `config/policy.rs:357` | Caller origin (9 variants) |
| `ExecutionProfile` | `config/policy.rs:461` | Trust boundary (5 variants) |
| `OperationRisk` | `config/policy.rs:9` | Risk tier (15 levels) |
| `OperationMetadata` | `config/policy.rs` | Static registry of all operations (31 canonical + 42 aliases) — single source of truth |
| `OperationDescriptor` | `config/policy.rs` | Unit of policy evaluation |
| `EnforcementContext` | `config/policy_decision.rs` | Central policy evaluation gate |
| `ApprovedOperation` | `config/policy_decision.rs:331` | Proof-of-enforcement token |
| `EnforcedDispatcher` | `tool/dispatcher.rs` | Type-level dispatch gate with binding validation |
| `LoadedScope` | `config/scope.rs:217` | Scope + provenance (`ScopeSource`: DefaultEmpty/ConfigFile/CliScopeFile/GeneratedPreset) |

### Runtime/Daemon Types

| Type | Location | Purpose |
|------|----------|---------|
| `Runtime` | `eggsec-runtime` | Task submit/cancel/snapshot/subscribe |
| `RuntimeTaskExecutor` | `eggsec-runtime` | Frontend-supplied execution logic trait |
| `RunRequest` | `eggsec-runtime` | Task execution request |
| `RuntimeSurface` | `eggsec-runtime` | Frontend-neutral surface identity |
| `ApprovedRunRequest` | `runtime_bridge/bundle.rs` | Coupled approval + request |
| `DaemonHost` | `eggsec-daemon` | Runtime bridge with client registry |
| `ClientRegistry` | `eggsec-daemon-protocol` | Connected-client tracking + RBAC |
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
    │       ├── eggsec-ui-model        (frontend-neutral view DTOs)
    │       ├── eggsec-daemon-protocol (IPC DTOs, RBAC client registry)
    │       └── eggsec-daemon          (persistent sessions, Unix socket IPC)
    │
    ├── eggsec-db-lab        (database pentest domain)
    ├── eggsec-web-proxy     (MITM proxy domain)
    ├── eggsec-mobile-lab    (mobile analysis domain)
    ├── eggsec-nse           (Nmap NSE/Lua VM)
    │
    └── eggsec               (ALL above — main engine, lib only)
            ↑
            ├── eggsec-tui    (engine + runtime + daemon-protocol + ui-model)
            ├── eggsec-cli    (engine + runtime + ui-model; optional: tui + daemon)
            └── eggsec-python (engine + core + tool-core; optional domain crates)
```

### Dependency Guardrails

Enforced by `scripts/check-architecture-guards.sh`:

- `eggsec-core` has no workspace crate dependencies (leaf crate)
- `eggsec-runtime` has no TUI, transport, persistence, or engine dependencies
- `eggsec-output` has no engine or runtime dependencies
- `eggsec-daemon` has no non-optional TUI/engine dependencies; transport deps only behind `http-api`
- Domain crates (`db-lab`, `web-proxy`, `mobile-lab`, `nse`) depend only on core + output
- Only frontends (`eggsec-cli`, `eggsec-tui`, `eggsec-python`) depend on the engine

### Intra-Engine Dependencies

Within the `eggsec` crate:

| Module | Depends On |
|--------|------------|
| `scanner` | `config`, `error`, `types`, `proxy` (optional) |
| `fuzzer` | `config`, `error`, `waf` (shared types) |
| `waf` | `config`, `error`, `fuzzer` (WafConfig) — deliberate two-way type sharing, both always compiled |
| `recon` | `config`, `error`, `types` |
| `auth` | `config`, `error`, `types`, `scanner` |
| `loadtest` | `config`, `error`, `types` |
| `pipeline` | `scanner`, `fuzzer`, `waf`, `recon`, `loadtest` |
| `dispatch` | all security modules (per-domain workers) |
| `tool` | all security modules (via `ToolRegistry`) |
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
- **Scope enforcement**: `TargetScope` validates resolved addresses before scanning; classification via `classify_address()` (`AddressClass`, 7 classes)
- **Policy evaluation**: all operations route through `EnforcementContext::evaluate()`
- See [config.md](config.md)

### Operation Metadata

`OperationMetadata` is the single source of truth for all externally invokable operations: **31 canonical operations + 42 aliases** (`config/policy.rs:1494`, alias table at `:2027`). Every `OperationDescriptor` derives from metadata; alias mapping ensures REST, MCP, gRPC, TUI, and agent tool IDs resolve to the same canonical entry.

### Audit Trail

`EnforcementAuditEvent` provides normalized audit records for every enforcement decision across all surfaces, including manual-override and scope-provenance detail. See [audit.md](audit.md).

### Logging & Tracing

- **Framework**: `tracing` with structured spans
- **Formats**: Pretty, Json, Compact
- **Sensitive data**: `SensitiveString` with redaction support
- See [logging.md](logging.md)

### Testing

| Test Suite | Command |
|------------|---------|
| Unit tests | `cargo test --lib -p eggsec` |
| Architecture guards | `bash scripts/check-architecture-guards.sh` |
| Full CI | `make check` |

- Several thousand `#[test]`/`#[tokio::test]` attributes across the workspace; densest suites are `eggsec-web-proxy`, `eggsec-nse`, `eggsec-output`, and `eggsec-daemon-protocol` (serialization round-trips)
- Visual regression: `TestBackend` + `Terminal::new()` buffer assertions throughout `eggsec-tui`

---

## Deep-Dive Index

Complete catalog of component deep-dives in this directory:

| Category | Documents |
|----------|-----------|
| **Core & Config** | [config.md](config.md), [types.md](types.md), [constants.md](constants.md), [error.md](error.md), [domain_contract.md](domain_contract.md), [feature_matrix.md](feature_matrix.md) |
| **Enforcement & Dispatch** | [dispatch.md](dispatch.md), [runtime_bridge.md](runtime_bridge.md), [audit.md](audit.md), [auth_context.md](auth_context.md) |
| **Discovery** | [recon.md](recon.md), [scanner.md](scanner.md), [probe.md](probe.md), [networking.md](networking.md), [wireless.md](wireless.md) |
| **Security Testing** | [fuzzer.md](fuzzer.md), [api_schema.md](api_schema.md), [waf.md](waf.md), [auth.md](auth.md), [hunt.md](hunt.md), [browser.md](browser.md), [websocket.md](websocket.md), [evasion.md](evasion.md) |
| **Load & Stress** | [loadtest.md](loadtest.md), [stress.md](stress.md) |
| **Orchestration** | [pipeline.md](pipeline.md), [distributed.md](distributed.md), [ai_agents.md](ai_agents.md) |
| **Infrastructure** | [proxy.md](proxy.md), [web_proxy.md](web_proxy.md), [storage.md](storage.md), [workflow.md](workflow.md), [findings.md](findings.md), [diff.md](diff.md), [notify.md](notify.md), [integrations.md](integrations.md) |
| **Daemon & Runtime** | [daemon.md](daemon.md), [runtime.md](runtime.md), [ui_model.md](ui_model.md) |
| **Interfaces** | [cli_commands.md](cli_commands.md), [tui.md](tui.md), [python_api.md](python_api.md) |
| **Compliance & Risk** | [compliance.md](compliance.md), [vuln.md](vuln.md), [supply_chain.md](supply_chain.md), [container.md](container.md) |
| **Defense Lab** | [defense_lab.md](defense_lab.md), [database_pentest.md](database_pentest.md), [mobile.md](mobile.md), [postex.md](postex.md), [c2.md](c2.md) |
| **Integration** | [nse_integration.md](nse_integration.md), [nse_capability_inventory.md](nse_capability_inventory.md), [nse_report_display_contract.md](nse_report_display_contract.md) |
| **Utilities & Support** | [utils.md](utils.md), [logging.md](logging.md), [generated.md](generated.md) |
| **Process & Reference** | [compile_time_baseline.md](compile_time_baseline.md), [api_extraction_boundary.md](api_extraction_boundary.md), [report_envelope.md](report_envelope.md), [supply_chain.md](supply_chain.md), [workflow.md](workflow.md) |

Process/reference docs not tied to a single component: [review_plan.md](review_plan.md), [audit.md](audit.md).

---

## See Also

Workspace-level canonical docs:

- [../AGENTS.md](../AGENTS.md) — agent-facing guidelines; its Module Index table maps each module to the deep-dive listed here plus its override file and loadable skill
- [../docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md) — workspace ownership, enforcement model, execution flows
- [../docs/ARCHITECTURE_INVARIANTS.md](../docs/ARCHITECTURE_INVARIANTS.md) — normative invariants
- [../docs/COMMAND_REGISTRY.md](../docs/COMMAND_REGISTRY.md) — command inventory
- [../docs/TOOL_REGISTRATION.md](../docs/TOOL_REGISTRATION.md) — tool registration for MCP/REST/gRPC/agent
- [../docs/CI_ARCHITECTURE_GUARDS.md](../docs/CI_ARCHITECTURE_GUARDS.md) — guard inventory

---

*Last updated: 2026-08-25 — Full verification pass against source. Corrections: EggsecError 23 variants (was 18), PayloadType exactly 40 (was "42+"), utils 20 sub-modules (was 22), TaskKind 29 (was "27+"), stress TCP flood documented as unimplemented, postex techniques described as 16 across 4 categories, NSE 166 implementations vs 44 registered descriptors distinguished, PDF attributed to engine crate not eggsec-output, ScanProfile path corrected to src/types.rs. Added How-an-Operation-Flows section, dispatch.md and api_schema.md deep dives. Same-day addendum: See Also now links AGENTS.md Module Index (override + skill mapping); scanner endpoint count re-verified at 347.*
