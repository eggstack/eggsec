# Eggsec - Rust Security Assessment Engine

Eggsec is a Rust-native, scope-enforced security assessment and defense-validation engine for authorized testing, local lab validation, WAF regression, CI security checks, and agent-readable security workflows.

## What Eggsec is

Eggsec is a command-line security assessment tool designed for security professionals, developers, and defensive teams who need to:

- **Discover attack surfaces** - Reconnaissance, subdomain enumeration, technology detection
- **Assess web application security** - Find vulnerabilities like SQL injection, XSS, SSRF, and more
- **Test infrastructure** - Scan ports, fingerprint services, discover endpoints
- **Evaluate defenses** - Test WAF detection and evasion-resistance
- **Load test** - Measure application performance under controlled load
- **Repeat assessments** - Pipeline scans with customizable profiles for regression workflows

### Why Eggsec?

| Capability | Description |
|------------|-------------|
| **Scoped Repeatable Testing** | Run the same assessment profiles repeatedly for regression validation |
| **Rust-Native Primitives** | High-performance async I/O, no external runtime dependencies |
| **Structured Outputs** | JSON, SARIF, JUnit, HTML, CSV for humans, CI, and agents |
| **WAF and Defense Validation** | Detection of 34 WAF products with evasion-resistance testing |
| **Local Lab/Regression Workflows** | Repeatable profiles against local test environments |
| **Optional NSE Compatibility** | Curated Nmap NSE script support as an optional layer |

### Core Capabilities

Eggsec covers reconnaissance, web security fuzzing, API testing, WAF validation, load/stress testing, auth control validation, proxy management, traffic interception, distributed scanning, mobile static/dynamic analysis, database pentesting, evasion detection, post-exploitation simulation, and C2 defense validation.

For the full capability matrix with risk tiers, feature gates, surface exposure, and scope requirements, see [`docs/CAPABILITY_MATRIX.md`](docs/CAPABILITY_MATRIX.md).

## Architecture

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the workspace crate ownership table, enforcement model, frontend execution flows, side-effecting execution path inventory, and transitional API register. See [`docs/COMMAND_REGISTRY.md`](docs/COMMAND_REGISTRY.md) for the command registry inventory and dispatch architecture. See [`docs/ARCHITECTURE_INVARIANTS.md`](docs/ARCHITECTURE_INVARIANTS.md) for the 30 normative invariants that all code must preserve.

## What Eggsec is not

Eggsec is not an exploitation framework, botnet component, credential attack platform, or tool for unscoped internet scanning. The `auth-test` command exists for defense validation of authentication controls (lockout policies, MFA enforcement, rate limiting, etc.) under strict scope/policy gating — it is not a credential attack platform (see architecture/auth.md for adopted model details: runtime policy gate, local findings only, standalone CLI distinct from pipeline `ScanProfile::Auth`). The `mobile` command performs static-only analysis of user-supplied .apk/.ipa files (manifest, permissions, transport config, secrets, debug/backup/exported components) for authorized lab/defense use only (feature-gated behind `mobile`; no execution, no device interaction, no network traffic to the app). Dynamic analysis (Android ADB + runtime log analysis + Frida instrumentation + behavioral correlation) is available under `mobile-dynamic` feature (`mobile-dynamic = ["mobile"]`); standalone defense-lab, MCP-absent; bridge via `to_scan_report_data_dynamic`. See docs/MOBILE.md for full details. A lightweight `correlate_findings` helper (and `static_correlation` on findings) links high-value static signals (cleartext, dangerous permissions) to dynamic observations. Same standalone pattern as wireless active. The `db-pentest` command (requires `--features db-pentest`) performs direct Postgres/MySQL checks for authorized lab/defense use only (standalone defense-lab; dry-run always safe; real runs require `--allow-db-pentest`; local `DbPentestReport`/`DbFinding` + optional `to_scan_report_data_db` bridge via report convert; Phase 1 complete 2026-06-12 per `plans/database-pentesting-phase1-foundation-handoff-plan.md` (executed); see `plans/non-web-database-pentesting-loadout-design-plan.md`). Some modules can generate aggressive traffic or security-test payloads, so advanced capabilities are feature-gated and intended for systems you own, operate, or have explicit authorization to test.

## Why Low-Level Features Exist

Eggsec includes stress testing, raw packet inspection, proxy management, and distributed scanning capabilities. These tools exist to validate the resilience of systems you own or are explicitly authorized to test — such as Synvoid, a distributed WAF platform.

These capabilities are framed as **defense-lab** and **hazardous-lab** workflows with:
- Mandatory scope files restricting targets to localhost or private lab networks
- Finite execution budgets (duration, request count, packet count)
- Policy decision records for auditability
- Clear CLI help text indicating the operating mode and required features

They are **not** generic offensive automation.

## Safety Model

Eggsec enforces a defense-in-depth safety model built around scope control, configuration defaults, and feature gating.

**Scope files** restrict every scan to explicitly authorized targets. Define allowed domains, CIDR ranges, and exclusions in a TOML file. When `require_explicit_scope = true`, any target not in the allowed list is rejected before a single packet is sent.

```toml
# scope.toml
require_explicit_scope = true

[[allowed_targets]]
pattern = "*.lab.internal"
description = "Lab environment"

[[allowed_targets]]
cidr = "10.0.0.0/8"
description = "Internal network"

[[excluded_targets]]
pattern = "admin.lab.internal"
description = "Admin panel - excluded"
```

**Configuration defaults** keep aggressive capabilities disabled until you opt in. Rate limits, concurrency caps, and timeouts are configurable per profile. Dry-run planning (`eggsec plan`) previews what a scan will do without sending traffic.

**Feature gating** ensures intrusive modules (stress testing, raw packet crafting, headless browser, NSE, database storage, container scanning, and more) require explicit build flags and cannot be invoked accidentally.

**Execution profiles** separate manual CLI/TUI operator-directed discretion from hard enforcement in strict and automated modes:
- **Manual CLI/TUI (default)**: `ManualPermissive` — operator-directed: warnings for safe scope ambiguity/missing scope; `RequireConfirmation` (with confirm/override) for discretion cases (explicit allowlist miss with positive rules, exclusions, high-risk, non-baseline capabilities, private resolution, cross-host redirect, target expansion). `--yes` is narrow (only `out-of-scope`/`target-expansion`); dedicated `--allow-private-resolution` / `--allow-cross-host-redirect` etc. are required for their classes. Override flags honored and audited only here; strict profiles/MCP/agent never honor overrides.
- **Manual strict**: `--strict-scope` uses `ManualGuarded` — hard enforcement (no discretion path).
- **MCP server**: always `McpStrict` (via `EnforcementContext`); explicit scope manifest (`LoadedScope::is_explicit_manifest()`) required for networked operations; warnings and `RequireConfirmation` treated as denials. Manual override flags ignored.
- **Agent**: `AgentStrict`; explicit scope manifest required; per-scan enforcement re-evaluated immediately before dispatch (in addition to startup gating); override flags ignored.
- **CI**: `CiStrict` — hard enforcement; override flags ignored.

`EnforcementContext::evaluate()` is the mandatory central boundary for all paths (CLI, TUI, MCP, agent, CI): performs LoadedScope provenance checks (strict/automated profiles require explicit manifest for networked ops), applies DenialClass downgrade (ManualPermissive only for safe ScopeMissing/TargetOutOfScope when no positive rules), positive capability allow for strict profiles, `RequireConfirmation` for manual discretion cases, and full risk/feature/policy enforcement. `DenialClass` drives `ManualPermissive` downgrade logic for safe scope-selection misses only (never for explicit exclusions, feature/risk/capability/hazard denials, or when positive scope rules were declared). Strict profiles, higher-risk operations, and automated paths never downgrade; `RequireConfirmation` is treated as hard `Deny` outside ManualPermissive. MCP production uses `McpServer::with_enforcement`; legacy `policy_decision_for_mcp_call` / direct `evaluate_operation_policy` deprecated for denial paths.

> For MCP and autonomous-agent execution, `EnforcementContext::evaluate()` is the mandatory pre-dispatch gate. Scope provenance must come from `LoadedScope`; raw `Scope` is not sufficient for automated execution. Baseline strict-automated capabilities are `PassiveFingerprint`, `ActiveProbe`, `Crawl`, `WafDetect`; non-baseline require explicit `allowed_capabilities`. Manual override flags have no effect for MCP, agent, CI, or strict manual profiles.

MCP and autonomous agent paths are always strict and cannot be downgraded or overridden by model-supplied flags.

**Normalized audit events** produce consistent records for every enforcement decision across all surfaces (CLI, TUI, REST, MCP, Agent). Each `EnforcementAuditEvent` captures the execution surface, profile, operation, target, outcome, confirmation classes, scope provenance, and correlation ID. Manual confirmations record class and reason; automated surfaces never record accepted manual overrides. See [docs/ENFORCEMENT_MODES.md](docs/ENFORCEMENT_MODES.md#audit-trail) for the full audit trail specification.

```bash
# Manual permissive (default: operator-directed; warn + confirm/override for discretion)
eggsec scan example.com --profile quick

# Manual permissive with override (audited; --yes narrow, use dedicated for private/redirect)
eggsec waf-stress https://lab.example --allow-high-risk --manual-override-reason "authorized Synvoid regression"
eggsec scan 10.0.0.5 --allow-private-resolution --manual-override-reason "lab private target"

# Manual strict (hard enforcement)
eggsec scan example.com --profile quick --scope scope.toml --strict-scope

# MCP strict (hard enforcement; override flags ignored)
eggsec codegg-mcp --stdio --scope scope.toml

# Agent strict (hard enforcement; override flags ignored)
eggsec agent run --scope scope.toml --portfolio portfolio.json
```

See [docs/SAFETY.md](docs/SAFETY.md) for full details on authorization, risk tiers, and scope rule evaluation. See [docs/ENFORCEMENT_MODES.md](docs/ENFORCEMENT_MODES.md) for the canonical dual-mode enforcement contract defining manual vs. automated posture semantics.

## Quick Start

### Workspace Layout

Eggsec is organized as a Cargo workspace with these crates:

| Crate | Purpose |
|-------|---------|
| `eggsec-core` | Dependency-light types, constants, shared primitives |
| `eggsec-tool-core` | Core data types for the tool abstraction layer (requests, responses, findings, errors) |
| `eggsec` | Assessment engine library (no binary) |
| `eggsec-nse` | Optional Nmap NSE compatibility runtime |
| `eggsec-tui` | Terminal UI adapter (`ratatui`/`crossterm`) with packaged themes, tab workflows, task runtime, and interactive enforcement preflight. |
| `eggsec-cli` | CLI binary entry point |
| `eggsec-output` | Report formatting and output adapters (JSON, CSV, HTML, SARIF, JUnit, Markdown) |
| `eggsec-agent` | Agent coordination primitives (registry, scheduler, lifecycle, communication) |
| `eggsec-db-lab` | Database pentesting domain crate (Postgres/MySQL/MSSQL/MongoDB/Redis security checks) |
| `eggsec-web-proxy` | Web proxy and MITM interception domain crate (proxy pool, intercept server, TLS, protocol handlers) |
| `eggsec-mobile-lab` | Mobile app security analysis domain crate (APK/IPA static + Android dynamic runtime testing) |
| `eggsec-daemon` | Long-running daemon host for persistent sessions (`Runtime`), transport abstraction (Unix socket default; HTTP/SSE feature-gated via `http-api`), client library, multi-client registry (`ClientKind`/`ClientRole`), session access control, and role-based permission checks |
| `eggsec-runtime` | Frontend-neutral runtime with task lifecycle management (`Runtime`, `RuntimeConfig`, `RuntimeTaskExecutor` trait) |

### Prerequisites

```bash
# Ubuntu/Debian
sudo apt-get install libpcap-dev libssl-dev wireless-tools

# Fedora/RHEL
sudo dnf install libpcap-devel openssl-devel wireless-tools
```

### Build and Run

```bash
# Clone and build
git clone https://github.com/eggstack/eggsec.git
cd eggsec
cargo build --release -p eggsec-cli

# Generate a config file
./target/release/eggsec --generate-config > eggsec.toml

# Validate your config
./target/release/eggsec config validate --config eggsec.toml

# Plan a scan (dry-run, no traffic sent)
./target/release/eggsec plan --scope examples/scope-localhost.toml --target http://127.0.0.1:8080

# Run a scoped scan against localhost
./target/release/eggsec scan 127.0.0.1 --profile quick --scope examples/scope-localhost.toml --json
```

### Installing to PATH

```bash
# Install to ~/.cargo/bin/eggsec
cargo install --path crates/eggsec-cli

# With all features
cargo install --path crates/eggsec-cli --features full

# Verify
eggsec --version
```

## Pipeline Profiles

Eggsec includes 18 built-in profiles that chain multiple security tests together. Choose the profile that matches your assessment goals.

| Profile | Use Case |
|---------|----------|
| **quick** | Fast port scan and service fingerprinting |
| **endpoint** | Quick + directory/endpoint discovery |
| **web** | Endpoint + web vulnerability fuzzing |
| **waf** | Endpoint + WAF detection and bypass |
| **full** | All stages including load testing |
| **api** | GraphQL, JWT, OAuth focused |
| **recon** | Intelligence-led with tech detection and CVE mapping |
| **stealth** | Evasion mode with randomized delays and header rotation |
| **deep** | Mutation fuzzing enabled for thorough testing |
| **vuln** | CVE-prioritized based on detected technologies |
| **auth** | JWT, OAuth, IDOR focused (pipeline: PortScan+Fingerprint+EndpointScan+Fuzz; distinct from `auth-test` credential/brute/MFA control validation) |
| **defense-lab** | Local lab regression testing |
| **synvoid-local** | Local SYN scan testing |
| **waf-regression** | WAF regression testing |
| **protocol-edge** | Protocol edge case testing (requires `packet-inspection`) |
| **nse-safe** | Safe NSE script execution (requires `nse`) |
| **db-regression** | Database security regression (requires `db-pentest`) |
| **web-proxy** | Web proxy intercept pipeline (requires `web-proxy`) |

Defense-lab profiles require private/localhost targets and enforce conservative budgets. Use `eggsec policy-explain` to inspect what a profile would do before running it.

```bash
# Quick scan - port scan + fingerprinting
./eggsec scan example.com --profile quick

# Web assessment - endpoint discovery + vulnerability fuzzing
./eggsec scan example.com --profile web

# Full assessment - all stages including load testing
./eggsec scan example.com --profile full

# API-focused - GraphQL/JWT/OAuth testing
./eggsec scan example.com --profile api
```

## Core Workflows

- **Scoped web assessment** - Port scanning, service fingerprinting, endpoint discovery, and vulnerability fuzzing against authorized targets
- **WAF/defense validation in lab** - Detect 34 WAF products, test evasion resistance, run regression suites against local WAF instances
- **CI regression checks** - Structured output (SARIF, JUnit, JSON) for integration into GitHub Actions, GitLab CI, and other pipelines
- **Agent/MCP integration** - Autonomous security agent with skills, portfolio management, and structured findings for AI-driven workflows
- **Optional NSE compatibility** - Curated Nmap NSE script support as an optional build layer

## Quick Command Reference

```bash
# Load testing
./eggsec load https://example.com -n 1000 -c 50

# Port scanning
./eggsec scan-ports example.com -p 1-1000 -c 100

# Endpoint discovery
./eggsec scan-endpoints https://example.com

# Vulnerability fuzzing
./eggsec fuzz https://example.com/api -t sqli,xss

# GraphQL security testing
./eggsec graphql https://api.example.com/graphql

# WAF detection and bypass testing
./eggsec waf https://example.com --bypass

# Reconnaissance
./eggsec recon example.com

# Wireless with deauth (Phase 1 active attacks; requires --features wireless-advanced + root/CAP_NET_ADMIN)
./eggsec wireless wlan0 deauth --bssid AA:BB:CC:DD:EE:FF --count 10 --allow-active-wireless
./eggsec wireless wlan0 deauth --bssid AA:BB:CC:DD:EE:FF --client FF:EE:DD:CC:BB:AA --broadcast --dry-run
# (Passive wireless under --features wireless; Phase 1 deauth under --features wireless-advanced; see docs/WIRELESS.md.)
# TUI: launch eggsec-tui (also --features wireless-advanced); navigate to the Wireless tab; press 'a' to enter
# Active mode, fill in BSSID / Client / Frame Count / Rate Limit, then press Enter to launch the dry-run attack immediately.
# Press 'd' to switch to live mode, which keeps the existing confirmation prompt.

# Mobile static analysis (APK/IPA; requires --features mobile; lab binaries only)
./eggsec mobile app.apk
./eggsec mobile MyApp.ipa --json -o mobile.json

# Mobile dynamic (Phase 2 closed + Phase 3/4a: proxy/permissions/correlation + Frida + CorrelationEngine; lab-only; requires --features mobile-dynamic + --allow-dynamic-mobile for real)
./eggsec mobile dynamic test.apk --device emulator-5554 --dry-run --json
./eggsec mobile dynamic test.apk --device emulator-5554 --proxy 127.0.0.1:8080 --traffic-capture /tmp/mitm.log --grant-permission android.permission.CAMERA --allow-dynamic-mobile --json

# Database pentesting (lab; requires --features db-pentest; Phase 6 complete 2026-06-14)
./eggsec db pentest --host 127.0.0.1 --port 5432 --user lab --db postgres --checks all --dry-run --json
./eggsec db pentest --host 127.0.0.1 --port 3306 --user lab --db mysql --checks all --allow-db-pentest --json
./eggsec db pentest mongodb://admin:pass@127.0.0.1:27017/labdb --dry-run --json  # requires db-pentest-mongodb
./eggsec db pentest redis://127.0.0.1:6379 --dry-run --json  # requires db-pentest-redis
./eggsec db pentest postgres://labuser@127.0.0.1:5432/labdb --dry-run --capture-baseline --baseline-label "v1" --json
./eggsec db pentest postgres://labuser@127.0.0.1:5432/labdb --dry-run --baseline /tmp/v1-baseline.json --json
./eggsec db pentest --host 127.0.0.1 --port 5432 --user lab --db postgres --checks all --allow-db-pentest --allow-db-pentest-advanced --evidence-bundle --json

# Evasion detection (defense-lab; requires --features evasion)
./eggsec evasion --target /path/to/binary --type file --dry-run --json
./eggsec evasion --target /path/to/binary --type process --pid 1234 --json
./eggsec evasion --type network --dry-run --json

# Post-exploitation simulation (defense-lab; requires --features postex)
./eggsec postex --target 10.0.0.5 --dry-run --json
./eggsec postex --target 10.0.0.5 --profile minimal --dry-run
./eggsec postex --category lotl --dry-run --json

# C2 simulation (defense-lab; requires --features c2)
./eggsec c2 --target 10.0.0.5 --dry-run --json
./eggsec c2 --target 10.0.0.5 --campaign apt29 --dry-run
./eggsec c2 --target 10.0.0.5 --campaign carbanak --dry-run -o c2-report.json

# Web proxy / traffic interception (defense-lab; requires --features web-proxy)
./eggsec proxy-intercept --dry-run --json
# TUI: launch eggsec-tui, navigate to Intercept tab, configure and press Enter for interactive flow inspection

# Web proxy pipeline scan (Phase 4; requires --features web-proxy)
./eggsec scan 127.0.0.1 --profile web-proxy --scope scope.toml

# Preview enforcement decision for an operation (dry-run policy check)
./eggsec preflight scan-ports --target 192.168.1.1
./eggsec preflight fuzz --target https://example.com/api --json

# Resume a previous scan
./eggsec resume session.json
```

Human-readable wireless output summarizes rogue candidates by default; add `--detect-suspicious` when you want the full findings list.

Run `eggsec --help` or `eggsec <command> --help` for the full command reference with all options.

### Lab Defense Commands

| Command | Mode | Description |
|---------|------|-------------|
| `eggsec policy-explain` | - | Explain policy decisions for a target/profile |
| `eggsec scope-explain` | - | Explain scope matching for a target |
| `eggsec preflight <operation>` | - | Preview enforcement decision for an operation without executing (shows scope, risk, confirmation requirements, suggested CLI flags) |
| `eggsec scan --profile defense-lab` | defense-lab | Comprehensive local defense validation |
| `eggsec scan --profile waf-regression` | defense-lab | WAF payload regression |
| `eggsec scan --profile synvoid-local` | defense-lab | Synvoid-specific local validation |
| `eggsec scan --profile protocol-edge` | defense-lab | Malformed protocol edge testing |
| `eggsec auth-test <target>` | defense-lab | High-risk credential control validation (brute-force, stuffing, lockout, MFA, rate-limit, timing; policy-gated via `CredentialTesting` risk + `allow_credential_testing`). Standalone defense-lab CLI (intentionally separate from pipeline); local `AuthTestReport`/`AuthFinding` only (direct emit; no `ScanReportData`, no SARIF/JUnit/etc conversion or bridge). Distinct from `ScanProfile::Auth` (JWT/OAuth/IDOR fuzzing via pipeline stages). See `docs/AUTH_LAB.md` + architecture/auth.md. |
| `eggsec proxy-intercept` | defense-lab | Interactive web proxy for HTTP/HTTPS traffic interception with TUI (flow inspection, editing, HAR export, pipeline profile `ScanProfile::WebProxy`, MCP tools via `web-proxy-mcp`, evidence bundles) |
| `eggsec wireless <iface>` | defense-lab (passive) | Standalone-complete passive WiFi recon (iwlist): Open/WEP/WPA/WPA2/WPA3/Enterprise + WPS/hidden/transition/weak-signal detection, vuln findings, rogue/Evil-Twin heuristic (passive; security-diff elevates to Medium). Supports `--repeat` (diffs + temporal summary), `--known-good` allowlist (suppresses rogue for lab baselines), `--dry-run` (plan/CI, valid JSON), `--detect-suspicious` (full rogue details; summarized by default in human output). Requires `--features wireless` + root/CAP_NET_ADMIN + wireless-tools/iwlist. Native `--json` auto-bridges to `ScanReportData` for `eggsec report convert` (SARIF/JUnit/etc). Optional explicit `to_scan_report_data` bridge. Bridged findings use `wireless-*` categories (e.g. wireless-rogue, wireless-security). MCP/agent tool exposure intentionally absent (standalone defense-lab design decision; not a SecurityTool). Deauth subcommand: `eggsec wireless <iface> deauth --bssid MAC [--client MAC] [--broadcast] [--count N] [--allow-active-wireless]`. Pure-Rust 802.11 frame crafting + radiotap + Linux raw socket injection (AF_PACKET/SOCK_RAW). Policy gated: `OperationRisk::Intrusive` + `wireless-advanced` feature. Lab-only authorized use; `--dry-run` supported. See docs/WIRELESS.md and architecture/wireless.md. |
| `eggsec mobile <path.{apk,ipa}>` (or `eggsec mobile static ...`) | defense-lab (static) | Standalone static analysis of Android APKs and iOS IPAs (manifest, permissions, transport config, secrets, debug/backup/exported components, signing/provisioning). Pure-Rust offline on user-supplied lab binaries only. Feature-gated `mobile` (default/legacy static path). Policy via SafeActive + required_features:["mobile"]; local MobileScanReport/MobileFinding + optional to_scan_report_data bridge. Native `--json` auto-bridges for `eggsec report convert`. See docs/MOBILE.md (Integration section) and architecture/mobile.md. `eggsec mobile dynamic ...` requires `--features mobile-dynamic` (Android ADB + runtime log analysis + Frida instrumentation + behavioral correlation; standalone defense-lab, MCP-absent; `to_scan_report_data_dynamic` bridge with `mobile-dynamic-*` + behavioral-regression + frida-* categories; auto-bridged in `report convert`; no TUI/pipeline/MCP). See docs/MOBILE.md for full details. |
| `eggsec evasion` | defense-lab | Detect common defense evasion techniques (16 built-in techniques across 6 categories: syscall, hook bypass, obfuscation, injection, anti-analysis, traffic obfuscation) mapped to MITRE ATT&CK IDs with confidence scores. Standalone defense-lab module (dry-run always safe; real runs require explicit authorization). Local `EvasionReport`/`EvasionDetection` + optional `to_scan_report_data` bridge via report convert. Feature-gated `evasion`. No MCP/agent/TUI/pipeline integration. |
| `eggsec postex` | defense-lab | Simulate post-exploitation techniques for purple teaming (16 techniques across 4 categories: LOTL, persistence, lateral movement, credential access) mapped to MITRE ATT&CK IDs. Standalone defense-lab module (dry-run always safe; real runs require `--allow-postex` + scope; reversible actions in lab mode). Local `PostexReport`/`PostexFinding` + optional `to_scan_report_data` bridge via report convert. Feature-gated `postex`. No MCP/agent/TUI/pipeline integration. |
| `eggsec c2` | defense-lab | Simulate C2 operations for purple teaming (beaconing, tasking, campaign orchestration, OPSEC scoring; MITRE ATT&CK profiles: APT29, Carbanak). Standalone defense-lab module (dry-run always safe; real runs require `--allow-c2`; depends on postex + evasion features). Local `C2Report`/`C2Campaign` + optional `to_scan_report_data` bridge via report convert. Feature-gated `c2`. No MCP/agent/TUI/pipeline integration. |

## Daemon Persistence

The `eggsec-daemon` crate provides durable session state backed by SQLite. Session snapshots are persisted at lifecycle points (create, submit, cancel, close) and recovered automatically on daemon restart.

### Daemon Transport

The daemon supports pluggable transport layers for client connectivity:

| Transport | Feature Flag | Default | Description |
|-----------|-------------|---------|-------------|
| Unix socket | Built-in | Yes (`/tmp/eggsec-daemon.sock`) | JSON-line protocol over Unix domain socket; primary IPC transport |
| HTTP/SSE | `http-api` | No (`127.0.0.1:9876`) | HTTP REST + Server-Sent Events via `axum`; 12 routes mapping to `ClientCommand`; loopback-only bind by default; requires explicit `http-api` feature |

WebSocket and gRPC transports were evaluated but deferred — they are not implemented in Phase 12.

The daemon advertises its available transports to clients via `DaemonCapabilities` (returned in `ServerMessage::Capabilities`). Clients send requests through `DaemonRequestContext` which carries the client ID, peer address, and transport kind. The daemon includes a `DAEMON_PROTOCOL_VERSION` (currently `1`) in its welcome message for client-side compatibility checks.

**HTTP transport details:**
- Binds to loopback only (`127.0.0.1`) by default; public bind (`0.0.0.0`) requires explicit config and emits a warning
- Uses `McpStrict` enforcement profile by default — noninteractive, no manual overrides
- 12 HTTP routes map to `ClientCommand` variants (create session, submit task, cancel, list sessions, etc.)
- SSE endpoint provides real-time session event streaming

### Configuration

| Field | Default | Description |
|-------|---------|-------------|
| `enable_persistence` | `true` | Persist session snapshots and audit events to SQLite |
| `data_dir` | `~/.local/share/eggsec/daemon/` | Directory for the `eggsec-daemon.sqlite` database file |

### Features

- **Session snapshots** — `SessionSnapshot` stored as JSON with timestamps in `session_snapshots` table
- **Session recovery** — On startup, `recover_persisted_state()` hydrates all persisted sessions; running/queued tasks are dropped (not auto-resumed) and recorded as `Cancelled` with `last_error: "interrupted by daemon restart"`. Only completed task records are preserved across restarts.
- **Audit event logging** — Security actions (create-session, submit-task, cancel, etc.) recorded with action, surface, outcome, client/session IDs, and timestamp
- **Artifact indexing** — Task artifacts (`ArtifactRef`) persisted within session snapshots, tracked by session association with kind, path, and MIME type
- **Schema migration** — SQLite schema versioned via `schema_meta` table (current: `2`); WAL mode enabled for concurrent reads; newer-than-current stored versions are explicitly refused to avoid silent corruption on downgrade.

### CLI Commands

```bash
# Start daemon with persistence (default)
eggsec daemon start

# List all persisted sessions
eggsec daemon history
eggsec daemon history --json

# Inspect a specific session's persisted snapshot
eggsec daemon show <session-id>
eggsec daemon show <session-id> --json

# Check daemon health
eggsec daemon status

# Stop daemon
eggsec daemon stop
```

### Local Smoke Test

`scripts/smoke-daemon-local.sh` is the canonical local-only lifecycle test for the
daemon. It runs against an ephemeral socket and a temporary data directory, with
no public network exposure. It validates daemon start, health, client
declaration, session create/list/snapshot, observer-deny + owner-allow
permission posture, persisted history/show, event stream subscription, and
graceful SIGTERM shutdown. Run with:

```bash
bash scripts/smoke-daemon-local.sh                 # defaults
bash scripts/smoke-daemon-local.sh /custom/path    # custom socket path
```

### Database Schema

| Table | Columns | Purpose |
|-------|---------|---------|
| `session_snapshots` | `session_id` (PK), `snapshot_json`, `created_at_secs` | Session state snapshots |
| `audit_events` | `audit_id` (PK), `action`, `surface`, `outcome`, `client_id`, `session_id`, `created_at_secs` | Security audit log |
| `schema_meta` | `key` (PK), `value` | Schema version tracking |

## Build Features

| Feature | Description | Status |
|---------|-------------|--------|
| `stress-testing` | SYN/UDP/ICMP floods, proxy management, IP spoofing | Lab-only |
| `packet-inspection` | Live packet capture, traceroute | Experimental |
| `nse` | Nmap NSE script compatibility | Experimental |
| `nse-ssh2` | NSE with SSH2/libssh2 support | Experimental |
| `nse-sandbox` | Restrict dangerous Lua operations | Experimental |
| `api-schema` | OpenAPI v3 schema-based fuzzing | Stable |
| `sbom` | SBOM generation (CycloneDX, SPDX) | Stable |
| `rest-api` | REST API server for agent integration | Experimental |
| `grpc-api` | gRPC API server | Experimental |
| `ws-api` | WebSocket pub/sub | Experimental |
| `ai-integration` | AI planner, script generation, autonomous agent | Experimental |
| `websocket` | WebSocket security testing | Stable |
| `headless-browser` | DOM XSS and SPA crawling | Stable |
| `web-proxy` | MITM proxy for capturing and inspecting HTTP/HTTPS traffic in authorized lab environments (Phase 2: interactive TUI with flow inspection, editing, HAR export, manipulation audit trail; Phase 4: pipeline profile, MCP tools via `web-proxy-mcp`, evidence bundle v2, performance optimizations, real WebSocket/HTTP2 backends) | Stable |
| `web-proxy-mcp` | Optional MCP tool exposure for web proxy (12 tools: list flows, inspect flow, edit request/response, manage rules, session save/load, HAR export, evidence bundle). Requires `web-proxy`. | Stable |
| `database` | SQLx-based PostgreSQL persistence | Stable |
| `container` | Kubernetes/Docker security scanning | Stable |
| `mobile` | Mobile app static analysis (APK/IPA manifest & config checks for authorized lab/defense use only; static-only Phase 1) | Stable |
| `mobile-dynamic` | Mobile dynamic testing (Android ADB + runtime log analysis + Frida instrumentation + behavioral correlation; standalone defense-lab, MCP-absent; `mobile-dynamic = ["mobile"]`; auto-bridge via `to_scan_report_data_dynamic`; see docs/MOBILE.md) | Stable |
| `cloud` | AWS/GCP/Azure asset discovery | Marker (planned) |
| `git-secrets` | Git secrets scanning | Marker (planned) |
| `wireless` | WiFi scanning (standalone-complete passive recon + security analysis; summary-by-default rogue heuristic; --repeat, --known-good, --dry-run, --detect-suspicious). TUI tab under feature; MCP/agent tool exposure intentionally absent (standalone defense-lab). **Passive = Phase 0 (complete 2026-06-11).** |
| `wireless-advanced` | Wireless active attack primitives (deauth, disassoc) for lab-only defense validation. Phase 1: targeted/broadcast deauth frame crafting and injection via `eggsec wireless <iface> deauth --bssid MAC [--client MAC] [--broadcast] [--count N]`. Pure-Rust 802.11 + radiotap + Linux AF_PACKET/SOCK_RAW. Policy gated (`OperationRisk::Intrusive` + `wireless-advanced` feature). Same standalone defense-lab pattern (no MCP/agent exposure). Requires `wireless` feature. Also powers the TUI Wireless tab active mode (dry-run default; live attacks prompt for policy confirmation). | Stable |
| `evasion` | Evasion technique detection (MITRE ATT&CK mapped; 16 techniques across 6 categories: syscall, hook bypass, obfuscation, injection, anti-analysis, traffic obfuscation). Standalone defense-lab module; dry-run always safe; real runs require explicit authorization. Local `EvasionReport`/`EvasionDetection` + optional `to_scan_report_data` bridge. No MCP/agent/TUI/pipeline integration. | Stable |
| `postex` | Post-exploitation and LOTL simulation for purple teaming (MITRE ATT&CK mapped; 16 techniques across 4 categories: LOTL, persistence, lateral movement, credential access). Standalone defense-lab module; dry-run always safe; real runs require `--allow-postex` + scope; reversible actions in lab mode. Local `PostexReport`/`PostexFinding` + optional `to_scan_report_data` bridge. No MCP/agent/TUI/pipeline integration. | Stable |
| `c2` | C2 (Command & Control) framework for defense-lab purple teaming (depends on postex + evasion; beaconing, tasking, campaign orchestration, OPSEC scoring; MITRE ATT&CK profiles: APT29, Carbanak). Standalone defense-lab module; dry-run always safe; real runs require `--allow-c2`. Local `C2Report`/`C2Campaign` + optional `to_scan_report_data` bridge. No MCP/agent/TUI/pipeline integration. | Stable |
| `c2-mcp` | MCP tool exposure for C2 module. Requires `c2`. | Marker (planned) |
| `db-pentest-mssql-tiberius` | MSSQL driver support for db-pentest (Tiberius). Requires `db-pentest`. | Stable |
| `db-pentest-mongodb` | MongoDB driver support for db-pentest. Requires `db-pentest`. | Stable |
| `db-pentest-redis` | Redis driver support for db-pentest. Requires `db-pentest`. | Stable |
| `db-pentest-mcp` | MCP tool exposure for db-pentest module. Requires `db-pentest`. | Marker (planned) |
| `transparent-proxy` | Transparent proxy mode for web-proxy | Marker (planned) |
| `dynamic-plugins` | Dynamic plugin loading system | Marker (planned) |
| `pdf` | PDF report generation | Marker (planned) |
| `advanced-hunting` | Advanced threat hunting | Marker (planned) |
| `compliance` | Compliance scanning (OWASP, PCI, HIPAA, SOC2) | Marker (planned) |
| `external-integrations` | Jira, GitHub, GitLab connectors | Marker (planned) |
| `finding-workflow` | Finding lifecycle management | Marker (planned) |
| `vuln-management` | Vulnerability triage and CVSS scoring | Marker (planned) |
| `full` | Most non-default features combined (excludes `grpc-api`, `ws-api`, `pdf`, `nse-ssh2`, `nse-sandbox`, `db-pentest-mssql-tiberius`, `db-pentest-mongodb`, `db-pentest-redis`, `db-pentest-mcp`, `c2-mcp`, `web-proxy-mcp`, `transparent-proxy`, `dynamic-plugins`, `api-schema`, `git-secrets`, `cloud`, `insecure-tls`, `tool-api`) | - |

**CLI-level features** (on `eggsec-cli` crate):

| Feature | Description | Default |
|---------|-------------|---------|
| `tui` | Terminal UI adapter (`eggsec-tui`) | Yes |
| `daemon-client` | Daemon client CLI commands (`eggsec-daemon` client library) | No |
| `headless` | Marker for headless/CI builds (no TUI, no daemon client) | No |

### Build Examples

```bash
# Default build - load testing, scanning, fuzzing, WAF testing
cargo build --release -p eggsec-cli

# With stress testing (controlled flood testing, proxy pool)
cargo build --release -p eggsec-cli --features stress-testing

# With packet inspection (live capture)
cargo build --release -p eggsec-cli --features packet-inspection

# With NSE support
cargo build --release -p eggsec-cli --features nse

# With mobile static analysis (APK/IPA manifest/config checks for authorized lab/defense use only; static-only)
cargo build --release -p eggsec-cli --features mobile

# With wireless passive recon (TUI tab; requires wireless-tools at runtime for real scans)
cargo build --release -p eggsec-cli --features wireless

# With wireless active attacks (Phase 1 deauth; requires wireless feature)
cargo build --release -p eggsec-cli --features wireless-advanced

# With evasion detection (defense-lab evasion technique analysis)
cargo build --release -p eggsec-cli --features evasion

# With post-exploitation simulation (defense-lab postex/LOTL simulation)
cargo build --release -p eggsec-cli --features postex

# With C2 framework (defense-lab C2 simulation; depends on postex + evasion)
cargo build --release -p eggsec-cli --features c2

# With web proxy MCP tools (requires web-proxy)
cargo build --release -p eggsec-cli --features web-proxy-mcp

# Full build - all features (includes mobile, wireless, container, etc.)
cargo build --release -p eggsec-cli --features full

# Headless build - no TUI, no daemon client (CI/scripting)
cargo build --release -p eggsec-cli --no-default-features

# Daemon client build - CLI commands without TUI
cargo build --release -p eggsec-cli --no-default-features --features daemon-client

# Daemon with HTTP/SSE transport (feature-gated; loopback-only bind by default)
cargo build --release -p eggsec-daemon --features http-api
```

## System Dependencies

| Feature | Required Packages | Install (Ubuntu/Debian) |
|---------|-------------------|--------------------------|
| `packet-inspection` | `libpcap-dev` | `sudo apt-get install libpcap-dev` |
| `wireless` | `wireless-tools` | `sudo apt-get install wireless-tools` (provides `iwlist` scanner). Tests (parsing/analysis, no hardware): `cargo test -p eggsec --features wireless`. See docs/WIRELESS.md and architecture/wireless.md. |
| `nse` | `libssl-dev` | `sudo apt-get install libssl-dev` |

## Output Formats

| Format | Use Case |
|--------|----------|
| Pretty | Human-readable terminal output (default) |
| JSON | Machine parsing, automation |
| Compact | Condensed terminal output |
| HTML | Human-readable reports |
| CSV | Spreadsheet analysis |
| SARIF | CI/CD security scanning (GitHub, GitLab) |
| JUnit XML | Test integration (CI pipelines) |
| Markdown | Documentation, GitHub issues |

## Defense-Lab Mode

Eggsec can run local, repeatable profiles against defensive systems for regression testing.

- **Repeatable adversarial traffic** - Run the same probe suite multiple times to measure changes in WAF or protocol behavior
- **Structured observations and baseline diffs** - Compare current results against a saved baseline to identify regressions or improvements
- **WAF regression testing** - Validate that WAF rules continue to catch known evasion patterns after updates

```bash
# Run a defense-lab profile against a local instance
./eggsec scan localhost:8080 --profile defense-lab --json -o baseline.json

# Run WAF regression testing
./eggsec scan localhost:8080 --profile waf-regression --json
```

## Relationship to Nmap/NSE

Eggsec borrows proven scanning concepts from Nmap but is not a drop-in replacement.

- **NSE is an optional compatibility layer.** Build with `--features nse` to enable curated Nmap NSE script support.
- **No full Nmap parity.** Eggsec does not aim to replicate all Nmap behavior. The goal is selective practical NSE compatibility for useful script categories. Library compatibility is defined by `NseLibraryRegistry` metadata (43 descriptors), not by implementation file counts.
- **NSE is a protocol-testing knowledge source.** Selected behaviors may be promoted into Rust-native probes over time for repeatability, performance, and safety.
- **Execution profiles enforce trust boundaries.** `NseExecutionProfileKind` presets (`ManualPermissive`, `AgentSafe`, `CiSafe`, etc.) resolve into sandbox config, limits, script/module/network policy, and audit metadata. CLI uses `ManualPermissive` by default; agents and CI use restrictive profiles.
- **Loader policy is closed at Milestone 1.** All script/module filesystem loading flows through `ScriptResolver` with canonical root containment, symlink escape rejection, extension allowlist, and size limits. `ManualPermissive` script-file loading with empty roots is intentionally permissive (manual CLI/TUI discretion); filesystem modules under `ManualPermissive` with empty roots resolve to built-ins only. Restricted profiles enforce roots strictly; automated profiles (`AgentSafe`, `CiSafe`) deny script files and filesystem modules before any path authorization. Read-path authorization cannot authorize non-existent script/module files. Rust-side blocking helper cancellation remains a Milestone 3 follow-up. See `architecture/nse_integration.md` for the empty-roots semantic table and the [Milestone 1 Closure Index](./architecture/nse_integration.md#milestone-1-closure-index).
- **Milestone 2 is closed.** Run output truthfulness is defined by `NseRunReport`; `NseRunReport.libraries` records per-run required/attempted library usage, not a capability snapshot. Rule behavior is defined by `NseRuleEvaluationReport`. Rule evaluation produces structured reports via `evaluate_rule()`. Error paths emit full reports by `build_failure_report()`. The compatibility corpus is representative and local-only by default. See the [Milestone 2 Closure Note](./architecture/nse_integration.md#milestone-2-closure-note).

## Agent and Orchestration

Eggsec includes a security agent for continuous monitoring and scheduled assessments. The agent maintains longitudinal memory of scan results, routes alerts to configured channels, and uses AI-powered skills for intelligent security testing.

The agent always requires an explicit scope manifest and uses `AgentStrict` execution profile. Networked operations are rejected without a valid scope file.

```bash
# Build with agent support
cargo build --release --features rest-api

# Run the agent with explicit scope
./eggsec agent run --scope scope.toml --portfolio /path/to/portfolio.json
```

See [docs/AGENT.md](docs/AGENT.md) for full documentation.

## Docker Usage

```bash
# Start test environment with vulnerable targets
docker-compose --profile testing up -d dvwa

# Run scans against containerized target
docker-compose --profile testing run --rm eggsec fuzz http://dvwa.target.local/login -t xss
```

See [docker-compose.yml](docker-compose.yml) for Docker configuration.

## Documentation

- [Safety and Scope Enforcement](docs/SAFETY.md) - Authorization, risk tiers, scope rules
- [Enforcement Modes](docs/ENFORCEMENT_MODES.md) - Dual-mode enforcement contract, manual vs. automated postures
- [Capability Matrix](docs/CAPABILITY_MATRIX.md) - Canonical operation/risk/feature/exposure matrix (derived from metadata)
- [Metadata Ownership](docs/METADATA_OWNERSHIP.md) - Metadata model, update workflow, and validation pipeline
- [Feature Matrix](docs/FEATURE_MATRIX.md) - Complete feature inventory, classification, naming conventions, and build profiles
- [Canonical Findings Schema](docs/FINDINGS_SCHEMA.md) - Finding structure, fingerprinting, redaction
- [Auth Context Configuration](docs/AUTH_CONTEXT.md) - Multi-role testing, env interpolation
- [Baselines and Differential Scans](docs/BASELINES_AND_DIFFS.md) - Comparing scan results over time
- [API Testing with OpenAPI Schemas](docs/API_TESTING.md) - Schema import, fuzz target generation
- [Agent Documentation](docs/AGENT.md) - Autonomous agent setup and usage
- [Capabilities](docs/CAPABILITIES.md) - Feature matrix and capabilities overview
- [Web Proxy Guide](docs/WEB_PROXY.md) - Interactive MITM proxy, HTTPS interception, TUI, rules
- [Web Proxy Playbook](docs/web-proxy-playbook.md) - Common attack/defense patterns and lab scenarios
- [Database Pentesting](docs/DATABASE_PENTEST.md) - Direct DB security assessment (Postgres/MySQL/MSSQL/MongoDB/Redis)
- [Wireless Testing](docs/WIRELESS.md) - WiFi scanning and active attacks (passive recon + deauth/disassoc)
- [Mobile Analysis](docs/MOBILE.md) - APK/IPA static and dynamic analysis (ADB, Frida, correlation)
- [Auth Testing Lab](docs/AUTH_LAB.md) - Authentication control validation (brute-force, lockout, MFA)
- [Tool Registration](docs/TOOL_REGISTRATION.md) - Registration inventory, protocol listing, enforcement paths
- [Usage Guide](docs/USAGE.md) - Comprehensive usage reference, output models, command examples
- [Extending Eggsec](docs/EXTENSIBILITY.md) - Contributor guide for adding operations, domains, commands, tools, TUI actions, reports, and features

## Security Considerations

- **Always ensure you have explicit permission** to test targets
- Use the scope file to restrict testing to authorized systems
- Use rate limiting to avoid overwhelming targets: `--rate-limit 10`
- Consider stealth mode for evasive testing: `--stealth`

## Troubleshooting

**Permission denied when running packet capture**
Packet capture requires root/sudo privileges. Run with `sudo eggsec packet capture -i eth0`.

**Panic: "command X alias X is duplicated"**
Update to the latest version from the repository.

**Target rejected by scope file**
Ensure your target matches an `allowed_targets` pattern or CIDR range in your scope TOML file. Use `eggsec plan` to preview what targets will be accepted.

**Build fails with missing system packages**
Install the required system dependencies for your platform. See the System Dependencies section above.

**High memory usage during large scans**
Reduce concurrency with `--concurrency 10` or use a more targeted port range with `-p`.

**`cargo install` fails with "found a virtual manifest"**
Run `cargo install --path crates/eggsec-cli` instead of bare `cargo install`. The workspace root is a virtual manifest; the binary crate is in `crates/eggsec-cli`.

## Responsible Use

Eggsec is designed for authorized security testing only. Use it against systems you own, operate, or have explicit written authorization to test. Always define scope files, use rate limits, and prefer local lab environments for development and regression testing.

## License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution guidelines. For adding new operations, domains, commands, tools, TUI actions, report outputs, or features, start with the [Extensibility Guide](docs/EXTENSIBILITY.md) -- it covers the metadata-first extension model, required tests, and pre-handoff checklist.
