# Slapper Architecture Overview

Slapper is a Rust-native security assessment and defense-validation engine designed for scoped, repeatable security testing of live systems.

**Quick Facts:**
- 39 modules in `crates/slapper/src/`
- 741 source files
- 1324 base tests (1469+ with full features)
- 30 payload types for fuzzing
- 28 TUI tabs
- 34 WAF products detected
- 40+ service fingerprinting protocols
- 169 NSE library modules
- 16 scan profiles

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
                ┌────────────────┴────────────────┐
                ▼                                 ▼
┌──────────────────────────┐        ┌──────────────────────────┐
│    handle_command()      │        │         TUI              │
│    (CLI dispatch)        │        │   (interactive mode)     │
└──────────┬───────────────┘        └──────────┬───────────────┘
           │                                   │
  ┌────────┴────────┐                          │
  ▼                 ▼                          ▼
┌─────────┐   ┌──────────┐            ┌──────────────┐
│ Module  │   │  Tool    │            │  Tab Router  │
│ Engines │   │ Registry │            │  (28+ tabs)  │
└────┬────┘   └────┬─────┘            └──────┬───────┘
     │             │                          │
     ▼             ▼                          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                     Security Testing Modules                                │
│  scanner/ │ fuzzer/ │ recon/ │ waf/ │ loadtest/ │ stress/ │ auth/          │
└─────────────────────────────┬───────────────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                          Output Layer                                       │
│    output/ (Pretty, JSON, Compact, HTML, CSV, SARIF, JUnit, Markdown)      │
│    findings/ │ diff/ │ storage/ │ notify/                                   │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Module Index

Each module links to a detailed `.md` file in this directory for deep-dive documentation.

### Entry Point

| Module | Source | Description | Deep Dive |
|--------|--------|-------------|-----------|
| `main.rs` | `crates/slapper/src/main.rs` | Binary entry, CLI parsing, config loading, command dispatch | [cli_commands.md](cli_commands.md) |

---

### Core Infrastructure

These modules are always compiled and form the foundation of the toolkit.

| Module | Source | Description | Deep Dive |
|--------|--------|-------------|-----------|
| `cli/` | `crates/slapper/src/cli/` | Clap-based argument parsing; `Commands` enum with 37 variants; 16 scan profiles | [cli_commands.md](cli_commands.md) |
| `commands/` | `crates/slapper/src/commands/` | Central dispatch via `handle_command()` exhaustive match; 20+ handler modules | [cli_commands.md](cli_commands.md) |
| `config/` | `crates/slapper/src/config/` | TOML/YAML config loading, `SlapperConfig` struct, scope enforcement with CIDR support, execution policy | [config.md](config.md) |
| `types.rs` | `crates/slapper/src/types.rs` | Canonical `Severity` enum (Critical/High/Medium/Low/Info) with CVSS conversion; `SensitiveString` zeroized wrapper; `OutputFormat` enum (8 formats) | - |
| `error/` | `crates/slapper/src/error/` | `SlapperError` enum (20+ variants) via `thiserror`; `Result<T>` type alias; `From` impls for reqwest, toml, serde_json, url, tokio | - |
| `constants.rs` | `crates/slapper/src/constants.rs` | Centralized defaults: HTTP timeouts, concurrency limits, scan ranges, WAF thresholds, UI symbols | - |
| `macros.rs` | `crates/slapper/src/macros.rs` | Utility macros: `run_if_enabled!`, `stage_task!`, `recon_stage!`, `print_if_some!`, `option_as_result!` | - |
| `logging/` | `crates/slapper/src/logging/` | Structured logging via `tracing`; `LogFormat` and `LogLevel` enums | - |
| `utils/` | `crates/slapper/src/utils/` | 23 sub-modules: circuit breaker, client pool, rate limiter, HTTP client factory, scope checking, progress bars, input validation, service detection, stealth mode | - |
| `probe.rs` | `crates/slapper/src/probe.rs` | Shared probe vocabulary: `ProbeIntent` (10 categories), `ProbeRisk` (6 risk levels), `ProbeMetadata` | - |

---

### Scanning & Discovery

Port scanning, service fingerprinting, and endpoint enumeration.

| Module | Source | Description | Deep Dive |
|--------|--------|-------------|-----------|
| `scanner/` | `crates/slapper/src/scanner/` | TCP connect/SYN scanning with async connections; 40+ protocol fingerprinting (SSH, SMTP, FTP, MySQL, Redis, Elasticsearch, Kafka, etc.); wordlist-based endpoint discovery (223 built-in paths); ICMP probing; UDP fingerprinting; Nmap-style timing templates (T0-T5); IP spoofing support | [scanner.md](scanner.md) |

---

### Security Testing

The core security assessment modules.

| Module | Source | Description | Deep Dive |
|--------|--------|-------------|-----------|
| `fuzzer/` | `crates/slapper/src/fuzzer/` | Fuzzing engine with 30 payload types (SQLi, XSS, SSRF, SSTI, IDOR, GraphQL, JWT, OAuth, gRPC, WebSocket, etc.); mutation-based and grammar-based fuzzing; adaptive rate limiting; session handling; response diffing; baseline capture; ReDoS detection; WAF fingerprinting; request chaining; auto-calibration; OpenAPI schema ingestion | [fuzzer.md](fuzzer.md) |
| `waf/` | `crates/slapper/src/waf/` | WAF detection for 34 products (Cloudflare, Akamai, AWS, Azure, GCP, Imperva, ModSecurity, etc.); scoring system (header/cookie/body/IP matching); evasion-resistance testing with 15 bypass techniques; HTTP smuggling; WAF-specific bypass profiles | [waf.md](waf.md) |
| `recon/` | `crates/slapper/src/recon/` | 32-file passive/active recon suite: DNS records, subdomain enumeration, WHOIS, ASN lookup, geolocation, reverse DNS, SSL/TLS analysis, technology detection, CVE mapping, content discovery, JS analysis, Wayback Machine, CORS misconfiguration, API schema discovery, subdomain takeover detection, email security (SPF/DKIM/DMARC), threat intelligence, git secrets, cloud asset discovery (AWS/Azure/GCP), container detection | [recon.md](recon.md) |
| `auth/` | `crates/slapper/src/auth/` | Authentication security testing: brute force, credential stuffing, lockout detection, MFA bypass, rate limit testing, session analysis, timing attacks; protocol-specific testers for SSH, SMTP, FTP | - |
| `browser/` | `crates/slapper/src/browser/` | Headless Chrome integration for DOM XSS detection (source/sink tracing), SPA route discovery, client-side security checks | - |
| `websocket/` | `crates/slapper/src/websocket/` | WebSocket security testing: connection validation, frame fuzzing, injection testing, origin validation | - |
| `wireless/` | `crates/slapper/src/wireless/` | Wireless security testing via `iwlist` scanning, authentication testing | - |
| `hunt/` | `crates/slapper/src/hunt/` | Intelligent vulnerability hunting: attack chain detection, business logic flaw detection, race condition testing, authorization bypass, session security | - |
| `nse_tool/` | `crates/slapper/src/nse_tool.rs` | Optional NSE compatibility adapter bridging `slapper-nse` with the tool abstraction layer | [nse_integration.md](nse_integration.md) |
| `auth_context/` | `crates/slapper/src/auth_context/` | Multi-role auth contexts with environment variable interpolation for credential management | - |
| `api_schema/` | `crates/slapper/src/api_schema/` | OpenAPI v3 schema ingestion for type-aware fuzzing; `parse_openapi()` and `generate_fuzz_targets()` | - |

---

### Assessment Orchestration

Pipeline, distributed scanning, and workflow management.

| Module | Source | Description | Deep Dive |
|--------|--------|-------------|-----------|
| `pipeline/` | `crates/slapper/src/pipeline/` | Stage-based chained assessment with 7 stages (PortScan, Fingerprint, EndpointScan, Fuzz, LoadTest, Waf, Recon); 16 scan profiles; pause/resume via session files; shared `PipelineContext` for inter-stage data passing; `PipelineTool` implements `SecurityTool` for API exposure | [pipeline.md](pipeline.md) |
| `distributed/` | `crates/slapper/src/distributed/` | Worker/coordinator cluster architecture; task queue with lifecycle management; line-based JSON over TCP; PSK authentication; TLS encryption; worker self-registration and resource monitoring | [distributed.md](distributed.md) |
| `workflow/` | `crates/slapper/src/workflow/` | Finding lifecycle management: status workflow transitions, assignment, comments, SLA tracking | - |
| `vuln/` | `crates/slapper/src/vuln/` | Vulnerability management: CVSS 3.1 scoring, exploitability assessment, asset criticality, risk prioritization, triage, remediation guidance | - |

---

### AI & Agent Orchestration

LLM integration, autonomous agents, and tool abstraction.

| Module | Source | Description | Deep Dive |
|--------|--------|-------------|-----------|
| `ai/` | `crates/slapper/src/ai/` | AI/LLM client with provider abstraction (OpenAI, Azure, Anthropic, OpenAI-compatible); TTL cache (`AiCache`, `CacheKeyBuilder`); adaptive fuzzing via `AdaptiveScanEngine`; AI-powered WAF bypass suggestions (`SmartWafBypass`); script generation; payload generation; circuit breaker for fault tolerance | [ai_agents.md](ai_agents.md) |
| `agent/` | `crates/slapper/src/agent/` | Autonomous security agent: target portfolio management, longitudinal memory, alert routing (webhook, email, Slack, PagerDuty), constraint enforcement (operational constraints, do-not-do lists), config watching/reloading, skill registry/loader | [ai_agents.md](ai_agents.md) |
| `tool/` | `crates/slapper/src/tool/` | `SecurityTool` trait with execute/validate/capabilities; `ToolRegistry` for dynamic registration; `ToolDispatcher` for routing; 8 concrete implementations (ReconTool, ScannerTool, FuzzerTool, LoadTestTool, WafTool, PipelineTool, SearchTool, OastTool); protocol adapters: MCP (JSON-RPC 2.0), REST (Axum), gRPC (Tonic), OpenAI-compatible, OpenResponses; `ChainPlanner` for multi-stage assessment planning; rate limiting; execution history; session management | [ai_agents.md](ai_agents.md) |

---

### Performance & Load

HTTP benchmarking and network stress testing.

| Module | Source | Description | Deep Dive |
|--------|--------|-------------|-----------|
| `loadtest/` | `crates/slapper/src/loadtest/` | HTTP load testing with `JoinSet` worker model; semaphore-based rate limiting; HDR histogram metrics (min/max/mean/p50/p90/p95/p99); status code tracking; error categorization; response body handling for connection pool health | [loadtest.md](loadtest.md) |
| `stress/` | `crates/slapper/src/stress/` | Controlled stress testing: SYN flood, UDP flood, HTTP flood, TCP flood, ICMP flood; IP spoofing support; authorization system with confirmation dialogs; metrics collection; safety warnings | [networking.md](networking.md) |

---

### Networking & Packets

Low-level network operations and proxy management.

| Module | Source | Description | Deep Dive |
|--------|--------|-------------|-----------|
| `packet/` | `crates/slapper/src/packet/` | Packet capture via `pnet` with BPF filters; packet crafting (`PacketBuilder` for TCP/UDP/ICMP with custom flags/payloads); protocol parsing (Ethernet, IP, TCP, UDP, ICMP, DNS, TLS, HTTP); hexdump output; traceroute implementation; validation utilities | [networking.md](networking.md) |
| `proxy/` | `crates/slapper/src/proxy/` | Proxy pool management: SOCKS4/SOCKS5, HTTP, HTTPS, Tor; health checking with configurable test URLs; rotation strategies (sequential, random); HTTP CONNECT tunneling; `ProxyManager` + `ProxyPool` + `ProxyRotator` | - |

---

### Data & Reporting

Report generation, findings management, and persistence.

| Module | Source | Description | Deep Dive |
|--------|--------|-------------|-----------|
| `output/` | `crates/slapper/src/output/` | 8 output formats: Pretty, JSON, Compact, HTML, CSV, SARIF, JUnit XML, Markdown; finding deduplication (Strict/Fuzzy/Disabled); trend analysis across scans; baseline comparison for regression; diff engine for scan comparison; run manifest tracking; report templates (executive, technical, developer, compliance); XXE safety and CSV formula injection protection | [output.md](output.md) |
| `findings/` | `crates/slapper/src/findings/` | Canonical `Finding` schema with `Confidence` levels, `EvidenceKind` types, `AffectedAsset`, `FindingLocation`, `FindingType`; lifecycle management; finding storage | - |
| `diff/` | `crates/slapper/src/diff/` | `diff_findings()` for comparing scan results; `DiffResult` with `FindingChange` entries; `DiffSummary` for aggregation | - |
| `storage/` | `crates/slapper/src/storage/` | SQLx-based persistence (PostgreSQL) for findings, scan history, and configuration; predefined queries | - |

---

### Integration & Compliance

External system connectors and compliance frameworks.

| Module | Source | Description | Deep Dive |
|--------|--------|-------------|-----------|
| `integrations/` | `crates/slapper/src/integrations/` | Issue tracker connectors: Jira, GitHub Issues, GitLab Issues; `IssueTracker` trait for common interface | - |
| `compliance/` | `crates/slapper/src/compliance/` | Compliance scanning and reporting: OWASP Top 10, PCI DSS, HIPAA, SOC 2; framework-specific report generation | - |
| `container/` | `crates/slapper/src/container/` | Container security: Docker image analysis, Kubernetes security checks, container escape detection, CIS benchmark validation | - |
| `supply_chain/` | `crates/slapper/src/supply_chain/` | SBOM generation (CycloneDX, SPDX); vulnerability scanning against SBOMs; typosquatting detection | - |
| `notify/` | `crates/slapper/src/notify/` | Notification system: webhook support for Slack, Discord, Teams; email notifications; `NotifyManager` + `NotifyConfig` | - |

---

### User Interface

Interactive terminal-based UI.

| Module | Source | Description | Deep Dive |
|--------|--------|-------------|-----------|
| `tui/` | `crates/slapper/src/tui/` | Ratatui + crossterm-based TUI with 28 tabs (recon, scan, fingerprint, fuzz, waf, load, proxy, packet, GraphQL, OAuth, cluster, stress, report, NSE, settings, history, dashboard, etc.); 12 reusable components (InputField, Selector, Checkbox, RadioGroup, ProgressGauge, ScrollableText, Popup, etc.); 8 background workers (network, scanner, fuzzer, recon, API, security, task runner); theme manager (dark/light); session persistence (auto-save at 30s); global search; command palette; keyboard-driven navigation (hjkl, Ctrl+combinations) | [tui.md](tui.md) |

---

### Defense Lab

Controlled testing and regression validation.

| Module | Source | Description | Deep Dive |
|--------|--------|-------------|-----------|
| *(cross-cutting)* | `probe.rs`, `output/run_manifest.rs`, `pipeline/` | Defense-lab mode for local, controlled testing against defensive systems; probe categories (TCP/IP stack, malformed packets, TLS/client fingerprints, HTTP ambiguity, WAF payload classification, bot-like patterns, rate-limit/tarpit, load-bearing); safety model (target scope, explicit scope, rate/concurrency budgets); `RunManifest` for structured regression analysis; five defense-lab profiles | [defense_lab.md](defense_lab.md) |

---

## Module Interconnections

### Data Flow

```
Target Input
     │
     ▼
┌─────────┐    ┌─────────┐    ┌──────────┐    ┌─────────┐
│  Recon  │───▶│ Scanner │───▶│ Endpoints│───▶│  Fuzzer │
│ (intel) │    │ (ports) │    │ (paths)  │    │  (bugs) │
└─────────┘    └─────────┘    └──────────┘    └────┬────┘
     │               │              │               │
     ▼               ▼              ▼               ▼
┌─────────┐    ┌─────────┐    ┌──────────┐    ┌─────────┐
│   WAF   │◀───│ Pipeline│───▶│ LoadTest │    │   AI    │
│(bypass) │    │(orchest)│    │  (perf)  │    │(adapt)  │
└─────────┘    └────┬────┘    └──────────┘    └─────────┘
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
| `commands/handlers/` | `pipeline/` | Pipeline execution entry |
| `pipeline/` | `scanner/`, `fuzzer/`, `recon/`, `waf/`, `loadtest/` | Stage orchestration |
| `scanner/` | `waf/` | WAF detection during port/service discovery |
| `fuzzer/` | `waf/` | Bypass detection during fuzzing campaigns |
| `fuzzer/` | `ai/` | Adaptive payload generation via LLM |
| `agent/` | `tool/` | Autonomous scanning via tool abstraction |
| `tool/` | All security modules | MCP/REST/gRPC API exposure |
| `output/` | All modules | Report generation from any findings source |
| `recon/` | `scanner/` | Recon feeds discovered targets to scanner |
| `proxy/` | `stress/`, `scanner/`, `fuzzer/` | Proxy rotation for outbound traffic |
| `findings/` | `output/`, `storage/`, `workflow/` | Canonical finding schema consumed downstream |
| `config/` | All modules | Configuration and scope enforcement |

---

## Feature Flags

### Always Compiled (default)

`auth`, `cli`, `commands`, `config`, `constants`, `distributed`, `error`, `fuzzer`, `findings`, `loadtest`, `logging`, `notify`, `output`, `pipeline`, `proxy`, `recon`, `scanner`, `tui`, `types`, `utils`, `waf`

### Feature-Gated Modules

| Feature | Module(s) | Description |
|---------|-----------|-------------|
| `tool-api` | `tool/` | Tool abstraction layer (always enabled internally for API features) |
| `stress-testing` | `stress/`, `packet/` | SYN/UDP/ICMP floods, raw sockets, IP spoofing |
| `packet-inspection` | `packet/` | Live packet capture (libpcap), traceroute |
| `rest-api` | `tool/`, `agent/` | REST API server + MCP for AI agent integration |
| `ws-api` | `tool/` | WebSocket API server support |
| `grpc-api` | `tool/` | gRPC API server for external tool integration |
| `nse` | `slapper-nse` (re-exported as `nse`) | Nmap Scripting Engine support (169 libraries) |
| `nse-ssh2` | `slapper-nse` | NSE with SSH2/libssh2-backed connections |
| `nse-sandbox` | `slapper-nse` | NSE sandbox (restricts dangerous Lua operations) |
| `ai-integration` | `ai/`, `agent/skills.rs` | AI/LLM analysis, adaptive fuzzing, WAF bypass suggestions, script generation |
| `headless-browser` | `browser/` | DOM XSS and SPA crawling via headless Chrome |
| `database` | `storage/` | SQLx-based persistence (PostgreSQL) |
| `container` | `container/` | Kubernetes and Docker security checks |
| `cloud` | `recon/cloud/` | Cloud security scanning (AWS, GCP, Azure) |
| `compliance` | `compliance/` | OWASP, PCI-DSS, HIPAA, SOC2 scanning and reporting |
| `external-integrations` | `integrations/` | Jira, GitHub, GitLab issue tracker connectors |
| `finding-workflow` | `workflow/` | Finding lifecycle management |
| `vuln-management` | `vuln/` | Vulnerability triage and prioritization |
| `websocket` | `websocket/` | WebSocket security testing with real connections |
| `advanced-hunting` | `hunt/` | Intelligent vulnerability hunting and attack chain detection |
| `sbom` | `supply_chain/` | SBOM generation (CycloneDX, SPDX) |
| `git-secrets` | `recon/git_secrets.rs` | Git repository secret scanning |
| `pdf` | `output/pdf.rs` | PDF report generation |
| `wireless` | `wireless/` | Wireless security testing |
| `api-schema` | `api_schema/` | OpenAPI v3 schema-based fuzz target generation |
| `insecure-tls` | - | Disables TLS cert verification (testing only) |
| **`full`** | Everything | All features combined |

See [feature_matrix.md](feature_matrix.md) for detailed feature dependencies and build commands.

---

## Workspace Crates

| Crate | Location | Description | Deep Dive |
|-------|----------|-------------|-----------|
| `slapper` | `crates/slapper/` | Core toolkit: all security modules, CLI, TUI, output, config | *(this document)* |
| `slapper-nse` | `crates/slapper-nse/` | Nmap Scripting Engine via `mlua` (Lua 5.4): 169 library modules, sandbox configuration, CVE integration, SSH2 support, async executor | [nse_integration.md](nse_integration.md) |

---

## Key Data Types

| Type | Location | Purpose |
|------|----------|---------|
| `SlapperConfig` | `config/settings.rs` | Main configuration struct (HTTP, scan, output, notification, paths, proxy, AI, cache, alert channels) |
| `Scope` | `config/scope.rs` | Target allow/block enforcement with CIDR and glob/regex patterns |
| `Severity` | `types.rs` | Unified severity (Critical, High, Medium, Low, Info) with CVSS conversion |
| `SlapperError` | `error/mod.rs` | Unified error via `thiserror` with 20+ variants |
| `PayloadType` | `fuzzer/payloads/mod.rs` | 30 payload categories (SQLi, XSS, SSRF, SSTI, IDOR, etc.) |
| `SecurityTool` | `tool/traits.rs` | Trait for tool abstraction (execute, validate, capabilities) |
| `ToolRegistry` | `tool/registry.rs` | Dynamic tool registration and lookup |
| `AiClient` | `ai/client.rs` | LLM client with provider abstraction and circuit breaker |
| `FuzzEngine` | `fuzzer/engine/mod.rs` | Core fuzzing orchestration with state management |
| `PipelineContext` | `pipeline/context.rs` | Inter-stage data passing (target, ports, services, endpoints) |
| `WafDetector` | `waf/detector/mod.rs` | 34 WAF product detection with scoring system |
| `CircuitBreaker` | `utils/circuit_breaker.rs` | Fault tolerance pattern for external calls |
| `ProbeIntent` | `probe.rs` | Shared probe vocabulary (10 intent categories) |
| `ProbeRisk` | `probe.rs` | Shared risk classification (6 risk levels) |
| `Finding` | `findings/mod.rs` | Canonical finding schema with evidence, confidence, affected assets |
| `McpProfile` | `tool/protocol/mcp/profile.rs` | MCP agent profiles (OpsAgent, CodingAgent) |
| `TargetPolicy` | `tool/protocol/mcp/policy.rs` | MCP target scope enforcement policy |

---

## Detailed Documentation Index

| Document | Modules Covered | Description |
|----------|-----------------|-------------|
| [ai_agents.md](ai_agents.md) | `ai/`, `agent/`, `tool/` | AI/LLM integration, agent orchestration, MCP tool exposure, protocol adapters |
| [cli_commands.md](cli_commands.md) | `cli/`, `commands/` | CLI parsing, command dispatch, handler patterns, 37 command variants |
| [config.md](config.md) | `config/` | Configuration system, scope enforcement, profiles, TUI settings semantics |
| [defense_lab.md](defense_lab.md) | `probe.rs`, `pipeline/`, `output/` | Defense-lab mode, probe vocabulary, regression validation, safety model |
| [distributed.md](distributed.md) | `distributed/` | Worker/coordinator cluster architecture, task queue, PSK auth, TLS |
| [feature_matrix.md](feature_matrix.md) | *(cross-cutting)* | Feature flag reference, dependencies, build commands, stability levels |
| [fuzzer.md](fuzzer.md) | `fuzzer/` | Fuzzing engine, 30 payload types, detection algorithms, grammar, advanced fuzzers |
| [loadtest.md](loadtest.md) | `loadtest/` | HTTP load testing, HDR histogram metrics, worker model |
| [networking.md](networking.md) | `packet/`, `stress/` | Packet capture/crafting/parsing, stress testing (SYN/UDP/ICMP/HTTP) |
| [nse_integration.md](nse_integration.md) | `slapper-nse/` | NSE/Lua integration, 169 libraries, sandbox, CVE integration |
| [output.md](output.md) | `output/` | Report formats, deduplication, trend analysis, diff engine, run manifest |
| [pipeline.md](pipeline.md) | `pipeline/` | Stage orchestration, 16 profiles, session management, Tool integration |
| [recon.md](recon.md) | `recon/` | 32-file reconnaissance suite, parallel execution, performance optimizations |
| [scanner.md](scanner.md) | `scanner/` | Port scanning, 40+ protocol fingerprinting, endpoint discovery, timing templates |
| [tui.md](tui.md) | `tui/` | Terminal UI, 28 tabs, 12 components, 8 workers, themes, sessions |
| [waf.md](waf.md) | `waf/` | 34 WAF detection, 15 bypass techniques, scoring system, WAF-specific profiles |

---

## CLI Commands Reference

### Scan Operations
| Command | Description |
|---------|-------------|
| `scan-ports` | TCP port scanning with async connections |
| `scan-endpoints` | Discover sensitive HTTP endpoints (wordlist-based) |
| `fingerprint` | AMAP-style service fingerprinting (40+ protocols) |
| `scan` | Chained security assessment pipeline (16 profiles) |
| `resume` | Resume a previous scan from session file |

### Assessment Operations
| Command | Description |
|---------|-------------|
| `fuzz` | Security fuzzing with 30 payload types |
| `waf` | WAF detection and evasion-resistance evaluation |
| `waf-stress` | Comprehensive WAF stress testing |
| `graphql` | GraphQL endpoint security validation |
| `oauth` | OAuth/OIDC endpoint security validation |
| `auth-test` | Authentication control validation |
| `recon` | Comprehensive reconnaissance |

### Infrastructure
| Command | Description |
|---------|-------------|
| `load` | HTTP load test against target URL |
| `report` | Convert and generate security scan reports |
| `cluster` | Manage distributed scanning cluster |
| `remote` | Start remote listener for distributed commands |
| `exec` | Execute commands on remote systems |
| `serve` | Start REST API server (feature-gated) |
| `mcp-serve` | Start MCP server for AI assistant integration (feature-gated) |
| `codegg-mcp` | Start MCP server for coding agent integration (feature-gated) |
| `agent` | Run security agent for scheduled assessments (feature-gated) |
| `ai-analyze` | Post-scan AI analysis of findings (feature-gated) |
| `grpc` | Start gRPC server (feature-gated) |

### Planning & CI
| Command | Description |
|---------|-------------|
| `plan` | Preview execution plan without running |
| `ci` | Run security checks in CI/CD mode |
| `config` | Validate configuration files |
| `doctor` | Check system and runtime dependencies |
| `sbom` | Generate SBOM and check supply chain security (feature-gated) |

### Feature-Gated Operations
| Command | Feature | Description |
|---------|---------|-------------|
| `packet` | `packet-inspection` | Packet inspection and analysis |
| `nse` | `nse` | Run Nmap NSE-compatible scripts |
| `stress` | `stress-testing` | SYN/UDP/HTTP/TCP/ICMP stress testing |
| `proxy` | `stress-testing` | Manage proxy pool and rotation |
| `icmp` | `stress-testing` | ICMP echo probes |
| `traceroute` | `stress-testing` | Network path tracing |
| `vuln` | `vuln-management` | Vulnerability management tools |
| `storage` | `database` | Database storage and query operations |

---

## Scan Profiles

| Profile | Stages | Description |
|---------|--------|-------------|
| `quick` | PortScan, Fingerprint | Fast target assessment |
| `endpoint` | Quick + EndpointScan | Add endpoint discovery |
| `web` | Endpoint + Fuzz | Web-focused with fuzzing |
| `waf` | Web + Waf | Add WAF detection |
| `full` | All stages | Comprehensive assessment |
| `api` | Fuzz, GraphQL, OAuth, JWT | API-focused testing |
| `recon` | Recon, Fingerprint | Intelligence-led assessment |
| `stealth` | Web stages | Randomized timing, evasive |
| `deep` | Web + mutation Fuzz | Thorough with mutation fuzzing |
| `vuln` | CVE-prioritized Fuzz | CVE-driven vulnerability focus |
| `auth` | JWT, OAuth, IDOR | Authentication-focused |
| `defense-lab` | Baseline, Diff | Local defense validation |
| `synvoid-local` | SYN scan | Localhost SYN scan testing |
| `waf-regression` | Waf | WAF detection regression |
| `protocol-edge` | Protocol tests | Protocol edge cases |
| `nse-safe` | NSE scripts | Safe NSE script execution |

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

## Codebase Health

| Metric | Value |
|--------|-------|
| Tests | 1324 base, 1469+ with full features |
| Clippy warnings | ~33 (pre-existing, none in ai module) |
| Source files | 741 |
| Payload types | 30 |
| TUI tabs | 28 (+ conditional feature tabs) |
| WAF products | 34 |
| NSE libraries | 169 |
| Modules | 39 |
| Protocols fingerprinted | 40+ |
| CLI commands | 37 |
| Output formats | 8 |
| Feature flags | 28 |
| Scan profiles | 16 |
| Fuzzer modes | 3 |
| Proxy types | 5 |
| Recon sub-modules | 32 |

---

*This overview serves as the entry point to the architecture documentation. Each linked document provides a deep dive into a specific component or domain. Start here for a birds-eye view, then follow the Deep Dive links for detailed analysis.*
