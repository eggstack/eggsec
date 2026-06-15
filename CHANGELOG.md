# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- C2 (Command & Control) framework under `c2` feature (`postex` + `evasion` dependencies)
  - `C2Scanner` with dry-run and defense-lab simulation
  - `C2Campaign` with APT29, Carbanak/FIN7, and generic profiles
  - Beacon protocol simulation (HTTP/S, DNS, TCP)
  - Task queue with MITRE ATT&CK technique mapping
  - Agent lifecycle (register, check-in, task dispatch, self-destruct)
  - Postex integration: LOTL, lateral, credential, persistence techniques mapped to C2 tasks
  - Attack graph generation with critical path analysis
  - Campaign timeline with sequential phase events
  - OPSEC scoring and findings
  - `to_scan_report_data()` bridge (auto-detected in `report convert`)
  - Policy: `C2Operation` risk tier, `allow_c2` flag, `--allow-c2` CLI gate
  - Real C2 simulation mode (HTTP/S beacons, TCP port scans, HTTP task delivery)
  - 57 unit tests; all green
- C2 TUI tab (`Tab::C2`) under `c2` feature
  - Target and campaign profile inputs
  - Results display with beacons, tasks, OPSEC, attack graph, timeline
  - Full keyboard navigation, focus management, error handling
  - Worker integration with progress tracking
  - 310 TUI tests; all green
- C2 MCP tool exposure under `c2-mcp` marker feature
  - `C2Tool` implementing `SecurityTool` trait (id: "c2")
  - Always forces dry-run for safety
  - OpsAgent visible, CodingAgent hidden
  - `C2Simulation` capability added to policy enum
  - Risk mapping: `c2` → `C2Operation`
  - Capability mapping: `c2` → `C2Simulation`
- `architecture/c2.md` documentation

### Fixed

- Fixed broken import paths in test files (`recon_tests.rs`)
- Added missing feature flag guards for NSE integration tests
- Removed unused imports in stress module exports
- Fixed invalid `SynFlooder` import reference in TUI workers
- Added MIT license file
- Added `#![allow(dead_code)]` to stress metrics module

### Removed

- Python plugin runtime and all Python plugin support (`python-plugins` feature)
- Ruby plugin runtime and all Ruby plugin support (`ruby-plugins` feature)
- Metasploit RPC integration (`eggsec-ruby` crate)
- `eggsec-plugin` crate (Python plugin manager, AST scanner, security validation)
- `eggsec-ruby` crate (Ruby plugin bridge, loader, MSF client)
- `eggsec plugin list` and `eggsec plugin run` CLI commands
- TUI plugin tab for Python/Ruby plugin discovery
- Plugin-related configuration fields (`plugins_dir`)
- Plugin development documentation (`PLUGIN_DEVELOPMENT.md`, `PLUGINS.md`)

NSE support remains available as an optional Nmap NSE compatibility layer via the `nse`, `nse-sandbox`, and `nse-ssh2` features.

### Added

#### Web Proxy / Traffic Interception
- Interactive MITM web proxy (`web-proxy` feature) for HTTP/HTTPS traffic interception in authorized lab environments
- Dynamic TLS certificate generation via `rcgen` with per-host caching
- CLI command `proxy-intercept` with full policy integration (`OperationRisk::TrafficInterception`, `--allow-web-proxy` gate)
- Dry-run mode: complete `WebProxySessionReport` with synthetic flows, zero network activity
- Budget enforcement (flows, bytes per flow, duration, concurrent connections)
- Intercept rules with host/path pattern matching, priority, and YAML parsing
- Reporting bridge: `to_scan_report_data_proxy()` converts to `ScanReportData` (auto-bridged in `report convert`)
- Interactive TUI tab `Tab::Intercept` with live flow inspection, header/body editing, forward/drop/replay/pause actions
- Session save/load (JSON) with full manipulation history and flow actions
- HAR 1.2 export for browser DevTools import
- Manipulation audit trail (`ManipulationRecord`) for every request/response edit
- WebSocket interception via `tokio-tungstenite` with full message capture
- HTTP/2 stream tracking via `h2` with multiplexed stream state
- gRPC call interception with method type detection (unary, streaming)
- Enhanced rule engine: AND/OR/NOT conditions, regex, body size, protocol-specific matching
- Rule actions: Allow, Block, Intercept, Monitor, Modify, InjectResponse, Delay, Tag
- Rule persistence: JSON file save/load
- Cross-loadout correlation hooks (jwt-to-db, auth, mobile)
- Pipeline profile: `ScanProfile::WebProxy` / `Stage::WebProxy`
- MCP proxy surface: 12 tools via `web-proxy-mcp` marker feature
- Evidence bundle v2: `EvidenceBundle` / `BundleManifest` with gzip compression and multi-loadout correlation
- Performance: `FlowBuffer` (capacity-capped) and `ProxyMetrics` (telemetry snapshot)
- Standalone defense-lab pattern (same as wireless, mobile, auth-test, db-pentest)

#### Security
- Auth control validation (`eggsec auth-test`): Stabilized under runtime policy gate only (`OperationRisk::CredentialTesting` + `allow_credential_testing` in `ExecutionPolicy`, default false; central `EnforcementContext::evaluate()`). Local `AuthTestReport`/`AuthFinding` only (no canonical conversion or pipeline profile integration). Distinct from `ScanProfile::Auth` (JWT/OAuth/IDOR). TUI `AuthTab` is CLI-only (excluded from `Tab` enum). No dedicated Cargo feature. See `docs/AUTH_LAB.md` and `architecture/auth.md`. All tests green.
- Mobile dynamic testing close-out (Phase 1 + Phase 2a + final polish + close-out polish): Android ADB core + runtime log analysis + Phase 2a proxy Level-1 device config + traffic summary + runtime permission testing all complete 2026-06-12 under `mobile-dynamic` feature (`mobile-dynamic = ["mobile"]`; standalone defense-lab CLI, MCP-absent, `to_scan_report_data_dynamic` bridge with `mobile-dynamic-android-*` categories; auto-bridged in `report convert`; no TUI/pipeline/MCP in this round). Includes a `correlate_findings` helper that populates `DynamicMobileFinding.static_correlation` for high-value static ↔ dynamic overlaps (cleartext traffic ↔ static `usesCleartextTraffic`/network-config; runtime-perm ↔ static declared dangerous perms). Final polish (F2 correlation + F3 parser robustness + F5 report surface + F6 docs) per `plans/mobile-dynamic-phase2-final-polish-handoff-plan.md` (executed). Close-out polish (final code hygiene: `format_dynamic_report` "Phase 2 extensions present" header renamed to "Runtime extensions"; "P1 skeleton"/"stub" doc comments updated to reflect current state; CLI about-string + struct docs updated for Phase 2a; smoke-test script header refreshed; feature gating decision documented to keep all dynamic functionality under `mobile-dynamic` with no `mobile-dynamic-advanced` sub-feature) per `plans/mobile-dynamic-phase2-close-out-polish-plan.md` (executed). Design in `plans/dynamic-mobile-testing-loadout-design-plan.md`; Phase 1 per `plans/mobile-dynamic-phase1-implementation-handoff-plan.md` (executed); Phase 2a per `plans/mobile-dynamic-phase2-implementation-handoff-plan.md` (executed). All tests + smoke + clippy green. See `docs/MOBILE.md`, `architecture/mobile.md`, `crates/eggsec/src/mobile/AGENTS.override.md`, and `scripts/test-mobile-dynamic.sh`.

#### Configuration System
- Configuration file support (TOML/YAML) at `~/.config/eggsec/eggsec.toml`
- Environment variable support with `EGGSEC_` prefix
- Scope file support for target authorization (`scope.toml`)
- 16 scan profiles (quick, endpoint, web, waf, full, api, recon, stealth, deep, vuln, auth, defense-lab, synvoid-local, waf-regression, protocol-edge, nse-safe)
- Custom scan profile definitions

#### Output Formats
- SARIF output format for GitHub Code Scanning integration
- JUnit XML output for CI/CD integration
- HTML report generation
- CSV export format

#### Notifications
- Webhook notifications for scan events
- Slack webhook integration
- Discord webhook integration
- Configurable severity thresholds for notifications

#### Logging & Observability
- Structured logging with `tracing`
- JSON log format support
- Configurable log levels (trace, debug, info, warn, error)
- Request/response logging with timing

#### Security
- Scope-based target authorization
- CIDR-based allow/block lists
- Port exclusion rules
- Secret handling with `secrecy` crate
- Rate limiting with configurable limits

#### Infrastructure
- Dockerfile for containerized deployment
- docker-compose.yml with optional services
- Multi-stage Docker build for smaller images

### Changed

- Improved error handling across all modules
- Better error messages with context
- Removed `.unwrap()` and `.expect()` in favor of proper error propagation
- Enhanced TUI with better error display

### Fixed

- Various race conditions in concurrent operations
- Memory leaks in long-running scans
- Proper cleanup of resources on interruption

## [0.1.0] - 2024-01-15

### Added

- Load testing module with concurrent request support
- Port scanner with service detection
- Endpoint discovery scanner
- Service fingerprinting (20+ protocols)
- WAF detection (30+ WAFs)
- WAF bypass techniques
  - Header manipulation
  - HTTP smuggling
  - Evasion techniques (homoglyphs, zero-width, encoding)
- Security fuzzing
  - SQL injection payloads
  - XSS payloads
  - Path traversal
  - SSRF
  - Open redirect
  - ReDoS
  - Header expansion
  - Compression bombs
- Pipeline mode for chained assessments
- Session persistence and resume capability
- Interactive TUI
- JSON output
- Progress bars with indicatif

### Security

- Initial security controls for responsible testing
- TLS certificate verification (configurable)

[Unreleased]: https://github.com/eggstack/eggsec/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/eggstack/eggsec/releases/tag/v0.1.0
