# AGENTS.md

Guidelines for AI agents working on this codebase.

## Project Overview

Eggsec is a Rust-based security testing toolkit organized as a workspace with 8 crates: `eggsec-core`, `eggsec-tool-core`, `eggsec`, `eggsec-nse`, `eggsec-tui`, `eggsec-cli`, `eggsec-output`, and `eggsec-agent`. See `README.md` for features and `architecture/overview.md` for design details.

## Implementation Plan

All implementation items are complete.

## Quick Reference

### Build & Test Commands

```bash
cargo check -p eggsec-core
cargo check -p eggsec-tool-core
cargo check --lib -p eggsec
cargo check -p eggsec --features mobile
cargo test --lib -p eggsec --features mobile
cargo check -p eggsec --features mobile-dynamic
cargo test --lib -p eggsec --features mobile-dynamic
# (smoke via ./scripts/test-mobile-dynamic.sh after polish)
cargo check -p eggsec-tui
cargo check -p eggsec-cli
cargo check -p eggsec-nse
cargo check -p eggsec-output
cargo test -p eggsec-core
cargo test -p eggsec-tool-core
cargo test -p eggsec-output
cargo test --lib -p eggsec
cargo test --test negative_tests -p eggsec
cargo test --test scanner_tests -p eggsec
cargo clippy --lib -p eggsec
cargo build --release -p eggsec-cli
# db-pentest feature (Phase 1-6 complete; baseline/regression + MCP deepening + compliance)
cargo check -p eggsec --features db-pentest
cargo test --lib -p eggsec --features db-pentest
cargo clippy --lib -p eggsec --features db-pentest
# Baseline capture + regression comparison:
# eggsec db pentest postgres://lab@127.0.0.1:5432/labdb --dry-run --capture-baseline --baseline-label "v1" --json -o baseline.json
# eggsec db pentest postgres://lab@127.0.0.1:5432/labdb --dry-run --baseline baseline.json --json -o result.json
```

### Module Override Files

For specialized guidance on specific modules, see `AGENTS.override.md` in each module directory:

| Module | Override File |
|--------|---------------|
| `agent/` | `crates/eggsec/src/agent/AGENTS.override.md` |
| `agent/enforcement.rs` | `crates/eggsec/src/agent/enforcement.rs` (scan-depth risk/capability mapping, per-scan descriptor construction) |
| `ai/` | `crates/eggsec/src/ai/AGENTS.override.md` |
| `fuzzer/` | `crates/eggsec/src/fuzzer/AGENTS.override.md` |
| `scanner/` | `crates/eggsec/src/scanner/AGENTS.override.md` |
| `tui/` | `crates/eggsec-tui/src/AGENTS.override.md` |
| `waf/` | `crates/eggsec/src/waf/AGENTS.override.md` |
| `recon/` | `crates/eggsec/src/recon/AGENTS.override.md` |
| `tool/` | `crates/eggsec/src/tool/AGENTS.override.md` |
| `config/` | `crates/eggsec/src/config/AGENTS.override.md` |
| `output/` | `crates/eggsec/src/output/AGENTS.override.md` (core modules remain; report formatting moved to `eggsec-output`) |
| `proxy/` | `crates/eggsec/src/proxy/AGENTS.override.md` |
| `proxy/intercept/` | `crates/eggsec/src/proxy/AGENTS.override.md` (web proxy types, bridge, TUI, manipulation audit trail, Phase 4 pipeline/MCP/evidence) |
| `stress/` | `crates/eggsec/src/stress/AGENTS.override.md` |
| `distributed/` | `crates/eggsec/src/distributed/AGENTS.override.md` |
| `packet/` | `crates/eggsec/src/packet/AGENTS.override.md` (uses pnet, pnet_packet for raw sockets) |
| `loadtest/` | `crates/eggsec/src/loadtest/AGENTS.override.md` |
| `mobile/` | `crates/eggsec/src/mobile/AGENTS.override.md` (static analysis patterns, pure-Rust parsers) |
| `pipeline/` | `crates/eggsec/src/pipeline/AGENTS.override.md` |
| `nse/` | `crates/eggsec-nse/AGENTS.override.md` (Lua VM, NSE libraries, sandbox, CVE integration) |
| `container/` | `crates/eggsec/src/container/AGENTS.override.md` |
| `db_pentest/` | `crates/eggsec/src/db_pentest/AGENTS.override.md` (Phase 1 foundation + postgres/mysql + manifest + bridge; Phase 2 executed on main: real tiberius behind marker, docker lab + --real smoke, combined web+db example artifact with db-* + sqli-*, qcount cleanup, full parity + docs; Phase 3: TUI tab + pipeline DbRegression + advanced gated checks + correlation/evidence stubs; Phase 4: full real advanced execution, correlation engine with scoring, native Stage::DbPentest, evidence bundle v2; Phase 5: MongoDB/Redis engines, cross-DB correlation, compliance mapping, MCP opt-in; Phase 6 (complete 2026-06-14): baseline capture + regression comparison, MCP deepening (baseline ops, parameterized calls), extended compliance (NIST/ISO27001); cleanup + polish: shared URL builders, standardized error handling, improved redaction, types-only MCP module) |
| `wireless/` | `crates/eggsec/src/wireless/AGENTS.override.md` |
| `evasion/` | `crates/eggsec/src/evasion/AGENTS.override.md` |

### Architecture Index

Use these sections as the canonical reference points when updating guidance or skills:

- `architecture/overview.md` - System-wide architecture, module index, data flow
- `architecture/tui.md` - TUI event loop, key handling, overlays, tab routing, session persistence
- `architecture/config.md` - Config loading, scope enforcement, TUI settings save semantics
- `architecture/cli_commands.md` - CLI parsing, command dispatch, handler patterns
- `architecture/output.md` - Report formatting, exports, and rendering integration
- `architecture/pipeline.md` - Security assessment pipeline, 18 profiles
- `architecture/scanner.md` - Port scanning and endpoint discovery
- `architecture/fuzzer.md` - Fuzzing engine and payload generation
- `architecture/waf.md` - WAF detection and bypass
- `architecture/recon.md` - Reconnaissance module
- `architecture/distributed.md` - Distributed coordinator/worker architecture
- `architecture/compile_time_baseline.md` - Workspace crate layout and compile-time baseline
- `architecture/mobile.md` - Mobile app static + dynamic analysis (APK/IPA; static pure-Rust parsers; dynamic Phase 1-4a delivered under `mobile-dynamic` feature; standalone defense-lab, MCP-absent; `to_scan_report_data` bridge)
- `architecture/auth.md` - Authentication testing module (CLI `auth-test`, policy via `CredentialTesting`, local findings only; TUI `AuthTab` fully integrated as `Tab::Auth`). See `architecture/auth.md` for current design.
- `architecture/c2.md` - C2 (Command & Control) framework (beaconing, tasking, campaign orchestration, OPSEC scoring; MITRE ATT&CK profiles; depends on postex + evasion; standalone defense-lab; `to_scan_report_data` bridge; auto-bridged in report convert; TUI tab `Tab::C2`; MCP exposure via `c2-mcp` marker)

### Feature Flags

- `tool-api` - Tool abstraction layer (always enabled internally)
- `insecure-tls` - TLS bypass for testing only
- `rest-api` / `grpc-api` - API server integration
- `ws-api` - WebSocket pub/sub
- `nse` - Nmap NSE script support
- `nse-ssh2` - NSE with SSH2/libssh2 support
- `nse-sandbox` - Restrict dangerous Lua operations
- `ai-integration` - AI planner, script generation, autonomous agent skills
- `websocket` - WebSocket security testing
- `headless-browser` - DOM XSS and SPA crawling
- `database` - SQLx-based persistence
- `container` - Kubernetes/Docker scanning
- `sbom` - SBOM generation (CycloneDX, SPDX)
- `stress-testing` - Raw sockets, IP spoofing
- `packet-inspection` - Packet capture
- `advanced-hunting` - Advanced threat hunting
- `compliance` - Compliance scanning (OWASP, PCI, HIPAA, SOC2)
- `external-integrations` - Jira, GitHub, GitLab connectors
- `finding-workflow` - Finding lifecycle management
- `vuln-management` - Vulnerability triage and CVSS scoring
- `cloud` - AWS/GCP/Azure asset discovery
- `git-secrets` - Git secrets scanning
- `wireless` - Standalone-complete passive WiFi scanning and security analysis (summary-by-default rogue candidates; use `--detect-suspicious` for full details; real scans require Linux `iwlist` + root/CAP_NET_ADMIN). TUI tab present under feature with full passive + active integration (Phase 0 passive + Phase 1 active complete); MCP/agent tool exposure intentionally absent (standalone defense-lab surface). Deauth (Phase 1) available under `wireless-advanced`.
- `wireless-advanced` - Wireless active attack primitives (deauth, disassoc) for lab-only defense validation. Phase 1: targeted/broadcast deauth frame crafting and injection. Pure-Rust 802.11 + radiotap + Linux AF_PACKET/SOCK_RAW. Policy gated (`OperationRisk::Intrusive` + `wireless-advanced` feature). Same standalone pattern (no MCP/agent exposure). Requires `wireless` feature.
- `mobile` - Mobile app static analysis (APK/IPA; lab/defense framing; static + dynamic design in `plans/dynamic-mobile-testing-loadout-design-plan.md`).
- `mobile-dynamic` - Mobile dynamic testing (Phase 1: Android ADB core + runtime log analysis; Phase 2 2026-06-12: proxy Level-1 device config + traffic summary + runtime permission testing + correlation; Phase 3a 2026-06-12: Frida foundation + basic_method_trace under single mobile-dynamic per phase3-frida-expansion-plan Key Decision (no separate mobile-frida sub-feature); runtime --allow-frida + Intrusive policy for real ops; dry-run safe; standalone defense-lab, MCP-absent; `mobile-dynamic = ["mobile"]`). Auto-bridge in `report convert` via `to_scan_report_data_dynamic` (now includes mobile-dynamic-android-frida-*). Phase 1 complete 2026-06-12 per `plans/mobile-dynamic-phase1-implementation-handoff-plan.md` (executed). Phase 1 polish complete 2026-06-12 per `plans/mobile-dynamic-post-phase1-polish-and-phase2-planning.md` (executed). Phase 2 per `plans/mobile-dynamic-phase2-implementation-handoff-plan.md` + final polish + close-out (executed); `correlate_findings` + `static_correlation` delivered. Phase 2 officially closed 2026-06-12 per `plans/mobile-dynamic-phase2-closeout-and-phase3-kickoff-plan.md` (executed); decision: keep all dynamic under mobile-dynamic (no mobile-dynamic-advanced sub-feature). Phase 3a (Frida) delivered 2026-06-12 per `plans/mobile-dynamic-phase3-frida-expansion-plan.md` (executed). Phase 4a (Core Correlation Engine + Evidence Foundation) delivered 2026-06-12 (executed per plan); non-breaking extension; CorrelationEngine + correlate_reports + enriched CorrelatedFinding (score/correlation_type/enrichment) + CorrelationResult (correlations + timeline + summary) + scoring inside correlate_findings; builds on 3c baseline/regression/bundles; 6 new unit tests; all ~85+ mobile-dynamic tests green; dry-run safe/hardware-free; no new deps; serde roundtrips; standalone defense-lab (MCP/agent/TUI/pipeline absent); Phase 4b TUI reviewed + deferred per standalone (zero mobile in tui/*.rs; Tab enum 30 variants no Mobile; no task wiring); reporting polish delivered 2026-06-12 (human output regression/correlation hints in format_dynamic_report + build_dynamic_recommendations; 1 new test; all under single mobile-dynamic; dry safe; MCP/agent/TUI/pipeline absent).
- `evasion` - Standalone defense-lab evasion technique detection (MITRE ATT&CK mapped; 16 techniques across 6 categories; dry-run always safe; requires `--allow-evasion-testing` for real runs)
- `postex` - Standalone defense-lab post-exploitation and LOTL simulation (MITRE ATT&CK mapped; 16 techniques across 4 categories: LOTL, persistence, lateral movement, credential access; dry-run always safe; requires `--allow-postex` for real runs; reversible actions in lab mode)
- `c2` - Standalone defense-lab C2 (Command & Control) framework (depends on postex + evasion; beaconing, tasking, campaign orchestration, OPSEC scoring; MITRE ATT&CK profiles: APT29, Carbanak; dry-run always safe; requires `--allow-c2` for real runs; `to_scan_report_data` bridge; auto-bridged in report convert; TUI tab `Tab::C2`; MCP exposure via `c2-mcp` marker)
- `db-pentest` - Standalone defense-lab direct Postgres/MySQL/MSSQL/MongoDB/Redis security assessment (Phase 1 foundation + Phase 2 real MSSQL tiberius + Phase 3 TUI tab + pipeline ScanProfile::DbRegression + advanced gated checks + correlation/evidence stubs + Phase 4 full real advanced execution, correlation engine with scoring/typed results, native Stage::DbPentest pipeline stage, evidence bundle v2 + Phase 5 MongoDB/Redis engines, cross-DB correlation, compliance mapping, optional MCP exposure via `db-pentest-mcp` marker + Phase 6 baseline capture + regression comparison (`DbBaseline`/`DbRegressionResult`), MCP deepening (baseline operations, parameterized calls), extended compliance (NIST SP 800-53, ISO 27001), cloud DB guidance notes; cleanup + polish: shared URL builders in `utils.rs`, standardized error handling, improved redaction, types-only MCP module; requires `--allow-db-pentest` for non-dry runs; dry-run always safe; local `DbPentestReport`/`DbFinding` + optional `to_scan_report_data_db` bridge; auto-bridged in report convert; TUI tab `Tab::DbPentest` under feature). See `plans/database-pentesting-phase1-foundation-handoff-plan.md` (executed), `plans/database-pentesting-phase3-advanced-and-integration-handoff-plan.md` (executed), and `plans/database-pentesting-phase5-engines-mcp-and-correlation-handoff-plan.md` (executed).
- `web-proxy` - Standalone defense-lab interactive web proxy for HTTP/HTTPS/WebSocket/HTTP2/gRPC traffic interception. Phase 1: MITM server + CA + CLI + dry-run + policy + bridge. Phase 2: Interactive TUI tab (`Tab::Intercept`) with live flow inspection, header/body editing, forward/drop/replay actions, intercept rules, session save/load, HAR export, and full manipulation audit trail. Phase 3: WebSocket/HTTP/2/gRPC protocol support (real `tokio-tungstenite` and `h2` backends), enhanced rule engine with complex conditions (AND/OR/NOT, regex, body size, protocol-specific), persistence (JSON), new actions (InjectResponse/Delay/Tag), cross-loadout correlation hooks (`CorrelationContext`), TUI protocol detail panes and rule management toggle, extended bridge findings. Phase 4: pipeline profile (`ScanProfile::WebProxy`, `Stage::WebProxy`), MCP proxy surface (12 tools via `web-proxy-mcp` marker feature), evidence bundle v2 (export/import, multi-loadout correlation), performance optimizations (`FlowBuffer` LRU-evicting buffer, `ProxyMetrics` runtime telemetry), real WebSocket/HTTP2 protocol support. Requires `--allow-web-proxy` + policy for real interception. `web-proxy = []` marker feature; `web-proxy-mcp` optional MCP exposure marker.
- `web-proxy-mcp` - Optional MCP tool exposure for web proxy (12 tools: list flows, inspect flow, edit request/response, manage rules, session save/load, HAR export, evidence bundle). Marker feature; requires `web-proxy`.
- `c2-mcp` - Optional MCP tool exposure for C2 campaign simulation (1 tool: c2). Marker feature; requires `c2`. Always forces dry-run for safety.
- `transparent-proxy` - Transparent proxy mode (Linux iptables/nftables REDIRECT). Marker feature; requires `web-proxy`.
- `dynamic-plugins` - Dynamic plugin loading from shared libraries (.so/.dylib). Marker feature; requires `web-proxy`. SECURITY WARNING: Only load plugins from trusted sources!
- `pdf` - PDF report generation
- `api-schema` - OpenAPI v3 schema-based fuzzing (marker-only)
- `full` - All features combined (22 sub-features, does not include `grpc-api`, `ws-api`, or `pdf`)

### Key Types

- `EggsecConfig` - Main configuration (`config::load_config()`)
- `Severity` - Unified severity (defined in `eggsec-core::types`, re-exported by `types.rs`)
- `SensitiveString` - Zeroized credential wrapper (defined in `eggsec-core::types`, re-exported by `types.rs`)
- `TabError` - Structured error type with categories (Network, Auth, Config, Resource, Target, Internal, Unknown) in `eggsec-tui` (`tui/app/tab_error.rs`)
- `ThemeLoadState` - Grouped theme-load runtime state (`rx`, `handle`, deferred restore, user-change flag) in `eggsec-tui` (`tui/app/state.rs`)
- `FuzzEngine` / `FuzzResult` - Fuzzing engine
- `PayloadType` - Enum of 40 payload categories
- `AiClient` / `Provider` - AI LLM client and provider enum
- `AiCache` / `CacheKeyBuilder` - TTL cache for AI responses
- `SmartWafBypass` - WAF bypass with knowledge base
- `AiPlanner` - AI-driven execution planning (requires `ai-integration`)
- `McpProfile` - MCP agent profile (`OpsAgent`, `CodingAgent`) in `tool/protocol/mcp/profile.rs`
- `McpProfilePolicy` - 18-field policy struct enforcing tool visibility and call restrictions per profile in `tool/protocol/mcp/policy.rs`
- `LoadedScope` - Scope with provenance metadata (`DefaultEmpty`, `ConfigFile`, `CliScopeFile`, `GeneratedPreset`) in `config/scope.rs`
- `ScopeSource` - Enum tracking where a scope manifest was loaded from in `config/scope.rs`
- `EnforcementContext` - Bundles `ExecutionProfile`, `ExecutionPolicy`, and `LoadedScope` for shared enforcement in `config/policy_decision.rs`
- `DenialClass` - Classification of denial reasons (`ScopeMissing`, `TargetOutOfScope`, `ExplicitExclusion`, etc.) in `config/policy.rs`
- `EnforcementOutcome` - Profile-aware result from `EnforcementContext::evaluate()`: `Allow`/`Warn`/`RequireConfirmation`/`Deny` (wrapping `PolicyDecision`)
- `ManualOverride` - CLI-only flags for satisfying `RequireConfirmation` (e.g. `allow_out_of_scope`, `assume_yes`, `allow_private_resolution`, `allow_cross_host_redirect`); audited on `PolicyDecision`. `--yes` is narrow (only `out-of-scope`/`target-expansion`); dedicated flags required for others. Strict profiles/MCP/agent never honor overrides.
- `ConfirmationClass` - Categories triggering `RequireConfirmation` under ManualPermissive (OutOfScope, ExplicitExclusion, HighRisk, NonBaselineCapability, PrivateResolution, CrossHostRedirect, TargetExpansion) in `config/policy_decision.rs`. Stable kebab-case via `as_str()`: "out-of-scope", "explicit-exclusion", "high-risk", "nonbaseline-capability", "private-resolution", "cross-host-redirect", "target-expansion". Dedup helper: `confirmation_class_strings`.
- `TargetPolicy` - Target scope enforcement policy in `tool/protocol/mcp/policy.rs`
- `CodingAgentFindingReport` - Typed output schema for coding-agent findings in `tool/protocol/mcp/coding_agent_output.rs`
- `ProbeIntent` / `ProbeRisk` - Probe classification in `probe.rs` (`ProbeRisk::ExploitAdjacent` maps to `OperationRisk::ExploitAdjacent`)
- `OperationDescriptor` - Bundles operation metadata for the shared policy evaluator in `config/policy.rs`
- `PolicySummary` - Report-ready policy summary struct in `eggsec-output` crate (operation mode, max risk, decisions, denial/warning counts)
- `StoredFinding` - Unified finding type in `findings::lifecycle`, re-exported by `storage::models` for database persistence
- `Wordlist` - Validated endpoint wordlist parsing with normalization (`scanner/wordlist.rs`)
- `OperationRisk::CredentialTesting`, `Capability::CredentialTesting`, `allow_credential_testing` in `ExecutionPolicy` (default false; high-risk tier for auth-test credential control validation)
- `MobilePlatform` / `MobileFinding` / `MobileScanReport` - Mobile static analysis types (`mobile/mod.rs`; public under `mobile` feature; `MobileScanReport` provides `to_scan_report_data` bridge to unified reports). Dynamic loadout design in `plans/dynamic-mobile-testing-loadout-design-plan.md`.
- `DynamicMobileReport` / `DynamicMobileFinding` / `LabManifest` / `run_dynamic_cli` - Dynamic mobile types + entrypoint (under `mobile-dynamic` feature; `mobile-dynamic = ["mobile"]`; Phase 1: Android ADB core + runtime log analysis; Phase 2 closed 2026-06-12: + `traffic_summary`/`permission_state` + proxy/permission actions + correlation; Phase 3a 2026-06-12: + `frida_instrumentation` + frida-* findings under single mobile-dynamic (no sub-feature); Phase 3b/3c 2026-06-12: richer FridaInstrumentation (structured_results/correlation_notes/regression_notes), multiple builtins + library: via run_frida_spec, multi-script, advanced static↔dynamic↔Frida correlation, MobileBaseline + capture_baseline/compare_to_baseline + behavioral regression, export_evidence_bundle; standalone defense-lab, MCP-absent; `to_scan_report_data_dynamic` bridge to `ScanReportData` with `mobile-dynamic-*` + `mobile-dynamic-android-frida-*` + behavioral-regression categories; auto-bridged in `report convert`). Phase 1 complete 2026-06-12 per `plans/mobile-dynamic-phase1-implementation-handoff-plan.md` (executed). Phase 1 polish complete 2026-06-12 per `plans/mobile-dynamic-post-phase1-polish-and-phase2-planning.md` (executed). Phase 2 closed 2026-06-12 per combined closeout+kickoff plan (all under mobile-dynamic). Phase 3a/3b/3c delivered per `plans/mobile-dynamic-phase3-frida-expansion-plan.md` (executed; Key Decision followed; 3c: library + multi + regression + bundles). Phase 4a (Core Correlation Engine + Evidence Foundation) delivered 2026-06-12 (executed per plan; non-breaking); + CorrelationEngine + correlate_reports + enriched CorrelatedFinding (score/correlation_type/enrichment) + CorrelationResult (correlations + timeline + summary) + scoring inside correlate_findings; builds on 3c; 6 new unit tests; ~85+ tests green; dry safe; no new deps; serde roundtrips; standalone defense-lab (MCP/agent/TUI/pipeline absent); 4b/4c deferred. Phase 4b: TUI deferred; polish in human formatter + rec builder.
- `TrafficSummary` / `parse_traffic_capture` - Phase 2 (closed) traffic summary type + parser (public under `mobile-dynamic`; summary-only from mitmproxy-style capture; feeds `traffic_summary` in report + bridge info findings).
- `MobileBaseline` / `capture_baseline` / `compare_to_baseline` / `export_evidence_bundle` - Phase 3c (delivered 2026-06-12) behavioral regression baseline + compare + pure flate2 evidence bundle export (public under `mobile-dynamic`; --baseline / --evidence-bundle; regression notes + optional behavioral-regression findings; bundles include report + traffic + frida structured).
- `CorrelatedFinding` (enriched) / `CorrelationType` / `CorrelationResult` / `CorrelationEngine` + `correlate_reports` - Phase 4a (Core Correlation Engine + Evidence Foundation; delivered 2026-06-12 executed per plan; non-breaking) under `mobile-dynamic`. `CorrelatedFinding` now carries optional conservative 0-100 `score`, `correlation_type: Option<CorrelationType>` (Direct/Indirect/Behavioral/CrossLayer), `enrichment: Option<String>`. `CorrelationResult` bundles correlations + timeline (timestamps + actions + Frida start + regression notes) + `CorrelationSummary` (total + avg_confidence). `CorrelationEngine` (min_score configurable) + `correlate_reports` convenience for full static+dynamic+Frida reports. Scoring lives inside `correlate_findings` (and engine path). Builds on 3c baseline/regression/bundles. 6 new unit tests; all ~85+ mobile-dynamic tests green. Dry-run safe/hardware-free; no new deps; serde roundtrips. Standalone defense-lab (MCP/agent/TUI/pipeline absent). Phase 4b TUI deferred + polish delivered 2026-06-12 (human); Phase 4c explored + partial 2026-06-12 (added frida-native-load correlation rule + richer regression + workflow helper; see plan). Locations: `crates/eggsec/src/mobile/dynamic.rs` (~216 CorrelatedFinding, ~247 CorrelationType, ~263 CorrelationSummary, ~270 CorrelationResult, ~281 CorrelationEngine, ~343 correlate_reports, ~1276 updated correlate_findings + scoring + build_timeline, ~1460 4c native-load rule, ~295 workflow). Phase 4b: TUI deferred; polish in human formatter + rec builder.
- `ActiveWirelessAttackResult` / `ActiveWirelessFinding` - Active wireless attack result and finding types (`wireless/active/mod.rs`; public under `wireless-advanced` feature; `to_active_scan_report_data()` bridges to `ScanReportData` with `wireless-active-*` categories)
- `DbPentestReport` / `DbFinding` / `LabDbManifest` / `DbTarget` / `CheckType` / `to_scan_report_data_db` / `DbCorrelationNote` - Database pentesting types (Phase 1: Postgres + MySQL checks + manifest + bridge; Phase 2: MSSQL tiberius; Phase 3: TUI tab + pipeline + advanced checks + correlation stub + evidence bundle; Phase 4: full real advanced execution + correlation engine + native pipeline stage + evidence bundle v2; Phase 5: MongoDB/Redis engines, cross-DB correlation, compliance mapping, optional MCP exposure via `db-pentest-mcp` marker; Phase 6: baseline capture + regression comparison (`DbBaseline`/`DbRegressionResult`/`SeverityChange`), MCP deepening (baseline ops, parameterized calls), extended compliance (NIST SP 800-53, ISO 27001); cleanup + polish: shared URL builders in `utils.rs`, standardized error handling, improved redaction, types-only MCP module; public under `db-pentest` feature; `DbPentestReport` provides `to_scan_report_data_db` bridge and `correlation: Option<DbCorrelationResult>` + `compliance: Option<ComplianceResult>` + `baseline_label: Option<String>` + `regression_summary: Option<DbRegressionResult>` fields to unified reports). See `plans/database-pentesting-phase1-foundation-handoff-plan.md` (executed), `plans/database-pentesting-phase2-mssql-and-polish-handoff-plan.md` (executed), `plans/database-pentesting-phase3-advanced-and-integration-handoff-plan.md` (executed), and `plans/database-pentesting-phase5-engines-mcp-and-correlation-handoff-plan.md` (executed).
- `DbBaseline` / `DbRegressionResult` / `SeverityChange` - Baseline and regression types (`db_pentest/baseline.rs`; public under `db-pentest` feature). `capture_baseline()` snapshots a report; `compare_to_baseline()` detects regressions (new findings, severity increases) and improvements. Baseline stored as JSON. Phase 6 delivered.
- `DbCorrelationEngine` / `DbCorrelatedFinding` / `DbCorrelationType` / `DbCorrelationResult` / `DbCorrelationSummary` - Database correlation engine types (`db_pentest/correlation.rs` and `db_pentest/types.rs`; public under `db-pentest` feature). `DbCorrelationEngine` uses rule-based matching (23 rules incl. cross-DB behavioral) with configurable `min_score` to correlate db findings against web SQLi signals and across multiple DB engine types. `correlate_cross_db()` finds behavioral patterns spanning heterogeneous DB reports. `DbCorrelatedFinding` carries optional score (0-100), correlation_type (Direct/Indirect/Behavioral/CrossLayer), and enrichment text. `DbCorrelationResult` bundles correlations + timeline + summary. Phase 4 delivered; Phase 5 extended with cross-DB rules.
- `ComplianceResult` / `ComplianceHit` / `ComplianceMapping` / `ComplianceSummary` / `map_findings_to_compliance` - Lightweight compliance mapping types (`db_pentest/compliance.rs`; public under `db-pentest` feature). Maps high-signal db findings to PCI-DSS, CIS, HIPAA, SOC2 control families. `ComplianceResult` is automatically produced for all reports (dry-run and real). Phase 5 delivered.
- `EvasionScanner` / `EvasionReport` / `EvasionDetection` / `EvasionTechnique` - Evasion detection types (`evasion/mod.rs`; public under `evasion` feature; `to_scan_report_data` bridge to unified reports)
- `PostexScanner` / `PostexReport` / `PostexDetection` / `PostexTechnique` - Post-exploitation simulation types (`postex/mod.rs`; public under `postex` feature; `PostexReport` provides `to_scan_report_data` bridge to unified reports)
- `C2Scanner` / `C2Report` / `C2Campaign` / `BeaconResult` / `TaskResult` / `OpsecAssessment` - C2 framework types (`c2/mod.rs`; public under `c2` feature; `C2Report` provides `to_scan_report_data` bridge to unified reports; depends on postex + evasion features; MCP exposure via `c2-mcp` marker feature)
- `PostexProfile` - Profile enum controlling technique coverage (Minimal/Standard/Aggressive) in `postex/mod.rs`
- `PostexCategory` - Category enum (Lotl/Persistence/LateralMovement/CredentialAccess) in `postex/mod.rs`
- `LotlCommand` - LOTL command types (PowerShell, WMIC, certutil, etc.) in `postex/lotl.rs`
- `PersistenceType` - Persistence mechanism types in `postex/persistence.rs`
- `LateralTechnique` - Lateral movement technique types in `postex/lateral.rs`
- `CredentialTechnique` - Credential access technique types in `postex/credential.rs`
- `WebProxySessionReport` / `ProxyFlow` / `BudgetUsage` - Interactive web proxy types (`proxy/intercept/types.rs`; public under `web-proxy` feature; `WebProxySessionReport` provides `to_scan_report_data_proxy` bridge to unified reports)
- `ManipulationRecord` - Immutable record of a request/response manipulation (field, before, after, reason, timestamp) in `proxy/intercept/types.rs`
- `InterceptSession` - Saveable session with flows, manipulations, and flow actions (JSON save/load + HAR export) in `proxy/intercept/types.rs`
- `FlowAction` - Enum of per-flow actions (Forward/Drop/Replay/Paused) in `proxy/intercept/types.rs`
- `EnhancedRule` / `EnhancedRuleSet` - Enhanced rule engine with complex conditions, persistence, and rule management (`proxy/intercept/rules.rs`; public under `web-proxy` feature)
- `RuleCondition` - Complex condition type with AND/OR/NOT combinators, protocol-specific matching (`proxy/intercept/rules.rs`)
- `RuleContext` - Context for rule evaluation (`proxy/intercept/rules.rs`)
- `WebSocketSession` / `WebSocketMessage` / `WebSocketOpcode` - WebSocket interception types (`proxy/intercept/protocols.rs`; public under `web-proxy` feature)
- `Http2Session` / `Http2Stream` / `Http2StreamState` - HTTP/2 stream tracking (`proxy/intercept/protocols.rs`; public under `web-proxy` feature)
- `GrpcSession` / `GrpcCall` / `GrpcMethodType` - gRPC call interception (`proxy/intercept/protocols.rs`; public under `web-proxy` feature)
- `ProtocolDetection` - Protocol detection result with confidence (`proxy/intercept/protocols.rs`)
- `ProxyProtocol` - Protocol enum (Http1, Http2, WebSocket, Grpc) (`proxy/intercept/protocols.rs`)
- `CorrelationContext` / `CorrelationReference` / `CorrelationSource` / `CorrelationHook` - Cross-loadout correlation (`proxy/intercept/correlation.rs`; public under `web-proxy` feature)
- `InjectResponseConfig` - Inject-response rule action configuration (`proxy/intercept/rules.rs`)
- `RuleId` - Rule identifier newtype (`proxy/intercept/rules.rs`)
- `EvidenceBundle` / `BundleManifest` - Evidence bundle export/import types for multi-loadout correlation (`proxy/intercept/types.rs`; public under `web-proxy` feature)
- `FlowBuffer` - LRU-evicting flow buffer with configurable capacity for performance (`proxy/intercept/buffer.rs`; public under `web-proxy` feature)
- `ProxyMetrics` - Runtime performance telemetry (latency histograms, throughput counters, error rates) for the proxy server (`proxy/intercept/metrics.rs`; public under `web-proxy` feature)
- `WebProxyToolSchema` / `WebProxyToolCall` - MCP proxy tool schema and call types for the 12-tool proxy MCP surface (`proxy/intercept/mcp.rs`; public under `web-proxy-mcp` feature)

### Important Patterns

- **Severity Enum**: Single canonical definition in `types.rs`. Re-export, don't recreate.
- **TabError Enum**: Structured error handling for tabs with `is_recoverable()` method for auto-recovery logic
- **Tool Abstraction**: `tool/traits.rs` has `SecurityTool` trait, `tool/registry.rs` has `ToolRegistry`
- **Regex Caching**: Use `lru = "0.18"` with cache size 100 (NonZeroUsize)
- **Circuit Breaker**: `utils/circuit_breaker.rs` - `CircuitBreaker` with configurable thresholds
- **Truncation**: `utils/formatting.rs` - `strip_controls` (recommended) and `preserve_all`
- **Visual Regression Testing**: Use `TestBackend` + `Terminal::new()` with `terminal.backend().buffer()` to verify rendered content
- **AI Cache Keys**: Always use `CacheKeyBuilder` for cache keys in AI module to avoid collisions
- **Hash Collections**: Use `rustc_hash::FxHashMap` and `rustc_hash::FxHashSet` instead of std collections for performance
- **Error Handling**: Avoid `unwrap_or_default()` on async operations; use explicit match with tracing instead
- **Shared Policy Evaluator**: Use `EnforcementContext::evaluate()` (central) in `config/policy_decision.rs` (wraps `evaluate_enforcement` + provenance/DenialClass/positive-capability checks) instead of building policy checks inline or calling `evaluate_operation_policy` directly for denial paths.
- **EnforcementContext**: Use `EnforcementContext` struct (preferred constructors: `cli`, `mcp_strict`, `agent_strict`, `ci_strict`); central `evaluate(descriptor)` handles LoadedScope provenance, DenialClass downgrade (ManualPermissive only for safe ScopeMissing/TargetOutOfScope when no positive rules), positive capability allow for strict, per-scan agent re-eval. CLI builds `ManualPermissive`/`ManualGuarded`/`CiStrict`, MCP forces `McpStrict`, agent forces `AgentStrict`.
- **MCP/agent invariant**: For MCP and autonomous-agent execution, `EnforcementContext::evaluate()` is the mandatory pre-dispatch gate. Scope provenance must come from `LoadedScope`; raw `Scope` is not sufficient for automated execution. Baseline strict-automated capabilities (`PassiveFingerprint`, `ActiveProbe`, `Crawl`, `WafDetect`) do not require explicit `allowed_capabilities`; all others do.
- **Scope Provenance**: Use `LoadedScope` (with `ScopeSource`) to distinguish "no scope provided" (`DefaultEmpty`) from "explicit empty scope". Strict profiles require `is_explicit_manifest() == true` for networked operations; enforced inside `EnforcementContext::evaluate`.
- **MCP Enforcement**: `handle_tools_call()` evaluates `self.enforcement.evaluate()` BEFORE dispatch. Production constructor: `McpServer::with_enforcement`. Legacy constructors (`with_scope`, `with_scope_and_profile`) and deprecated helpers (`policy_decision_for_mcp_call`, `denial_from_violation`) have been removed.
- **Agent Enforcement**: `handle_agent()` now requires explicit scope manifest and passes `EnforcementContext` to `AgentConfig`; per-scan `enforcement.evaluate` immediately before dispatch in `execute_scan_with_depth`.
- **Capability Mapping**: `required_capabilities_for_tool_call()` in `tool/protocol/mcp/policy.rs` maps tool IDs to required capabilities for enforcement (populated in descriptors for strict positive checks).
- **CommandContext Policy Wrapper**: Use `CommandContext::evaluate_and_enforce_operation()` for command handlers — it wraps `self.enforcement.evaluate()` with profile-aware scope enforcement and structured denial output.
- **Execution Profiles**: Use `ExecutionProfile` enum for caller trust boundary. `ManualPermissive` for CLI, `McpStrict` for MCP, `AgentStrict` for agents.
- **Enforcement Outcomes**: `EnforcementContext::evaluate()` returns `EnforcementOutcome` (Allow/Warn/RequireConfirmation/Deny) wrapping `PolicyDecision`; `evaluate_enforcement` is internal. ManualPermissive produces `RequireConfirmation` for operator-discretion cases (explicit positive-scope out-of-scope, explicit exclusion, high-risk, non-baseline capability, private-resolution, cross-host-redirect, target-expansion); automated profiles (McpStrict/AgentStrict/CiStrict) and ManualGuarded treat `RequireConfirmation` as Deny. Manual overrides are CLI-only and audited on `PolicyDecision`. `--yes` narrow (only `out-of-scope`/`target-expansion`); dedicated `--allow-private-resolution` / `--allow-cross-host-redirect` etc. for others; strict profiles/MCP/agent never honor overrides. Stable kebab strings from `ConfirmationClass::as_str()` used in audit/JSON/errors; `confirmation_class_strings` dedups while preserving order.
- **Capability Declarations**: Tools declare `required_capabilities` in `OperationDescriptor`. Policies control via `allowed_capabilities` / `denied_capabilities`.
- **Discovery Promotion**: `DiscoveredTargetStatus` controls whether discovered targets can be scanned. Only `ApprovedInScope` allows scanning.
- **MCP Profile Policy**: Use `McpProfilePolicy` struct in `tool/protocol/mcp/policy.rs` to enforce tool visibility and call restrictions per profile (overlays shared enforcement).
- **MCP Policy Helpers**: `classify_tool_risk()` and `infer_tool_category()` in MCP policy infer tool metadata from tool IDs; `policy_decision_for_mcp_call_with_enforcement` (via `EnforcementContext`) builds a `PolicyDecision` for MCP tool invocations.
- **IPv6 Hostname Parsing**: `extract_hostname()` in `tool/protocol/mcp/policy.rs` counts colons to distinguish bare IPv6 (>=2 colons, returned as-is) from host:port (1 colon, port stripped if valid u16). Never strip port from bare IPv6 addresses.
- **Feature Availability Checks**: Use `is_feature_enabled()` in `config/policy_decision.rs` for compile-time feature availability checks in policy decisions
- **eggsec-output Re-exports**: The `eggsec-output` crate re-exports key types (`Severity`, `AgentFinding`, `ScanReportData`, `DiffSummary`, `TrendAnalyzer`, etc.) at its crate root. Use `eggsec_output::Severity` rather than reaching into `eggsec_output::agent::Severity` directly.

### Codebase Health

| Metric | Value |
|--------|-------|
| Tests | ~4144 (includes #[test] + #[tokio::test]) |
| Clippy | ~54 warnings (pre-existing, none in ai module) |
| Source files | 865 (.rs files in crates/) |
| Payload types | 40 |
| Tabs | 33 (Tab enum variants 0-32) |
| WAF products | 34 |
| NSE libraries | 166 public modules |
| Modules | 45 (top-level directories in `crates/eggsec/src/`) |
| Output formats | 8 (Pretty, Json, Compact, Html, Csv, Sarif, Junit, Markdown) |
| Themes | 50 packaged + 3 built-in (cyber-red, dark, light) |
| CLI commands | 26 base, 45 total with all features |
| Pipeline profiles | 18 (Quick, Endpoint, Web, Waf, Full, Api, Recon, Stealth, Deep, Vuln, Auth, DefenseLab, SynvoidLocal, WafRegression, ProtocolEdge, NseSafe, DbRegression, WebProxy) |

### Codebase Issues (Known Stub Implementations)

No remaining stub implementations.

- **Web Proxy Phase 2 (Interactive TUI)**: Complete - Tab::Intercept TUI tab with flow list, detail panes, edit modal, forward/drop/replay, rules display, session management, HAR export, and ManipulationRecord audit trail.


### Security Notes

- **Scope Enforcement**: Private IP checks are deferred to scope rule evaluation in `is_target_allowed()` (`config/scope.rs:146-159`). Scope rules like `allow 10.0.0.0/8` correctly match private IPs before the fallback private-IP block. When no scope rules exist, private IPs are blocked unconditionally.
- **TUI Settings Tab**: The settings editor applies exposed fields on top of an existing config and preserves non-exposed sections such as `profiles`, `schedule`, `remote`, `ai`, `search`, and `alert_channels`. See `architecture/config.md` for the current save semantics.
- **MCP Coding Agent**: Default deny posture; stress/load/packet tools are hidden from coding-agent profile
- **Docker Shell Injection**: FIXED - `container/docker.rs:inspect_image()` now validates image names before passing to shell (2026-06-02)
- **Silent Error Suppression**: FIXED - All listed issues now properly log errors instead of silent suppression (2026-06-02):
  - `notify/mod.rs` - now logs with `tracing::warn!`
  - `loadtest/runner.rs` - now handles semaphore acquire errors gracefully
  - `packet/capture.rs` - now logs pcap write failures
  - `kubernetes.rs` - now logs network errors
- **NSE TOCTOU Vulnerability**: FIXED - lfs and os libraries now use `get_allowed_path()` to avoid race conditions (2026-06-02)
- **NSE DNS Rebinding Attack**: MITIGATED - `is_host_allowed()` limitation documented; `resolve_host()` returns bound IPs (2026-06-02)
- **NSE Sandbox Enforcement**: FIXED - 17 integration tests added for path/command/network restrictions (2026-06-02)
- **Browser ClientIssueType**: FIXED - now detects all 8 variants (was only 3) (2026-06-02)
- **FindingStore Deduplication**: FIXED - now deduplicates by fingerprint before appending (2026-06-02)
- **Remote Listener Policy**: `remote start` now uses `evaluate_and_enforce_operation` with `HazardousLab` mode and `RemoteExecution` risk (2026-06-10)
- **Handler Policy Adoption Complete**: All 27 target-bearing CLI handlers now use `evaluate_and_enforce_operation` with `OperationDescriptor`-based policy checks. 18 regression tests cover all risk tiers. See `docs/internal/POLICY_HANDLER_AUDIT.md` and `docs/internal/POLICY_VALIDATION_RESULTS.md` (2026-06-10)
- **Auth Test Policy Integration (post-2026-06-10)**: `auth-test` handler uses `evaluate_and_enforce_operation` with `CredentialTesting` risk (central `EnforcementContext`). TUI `AuthTab` is now fully integrated into the TUI as `Tab::Auth` (TabSpec, task system, policy enforcement, session save/restore). See `architecture/auth.md`, `commands/handlers/auth_test.rs`, `cli/auth.rs`, `docs/AUTH_LAB.md`. No dedicated credential-testing Cargo feature (runtime policy gate only). `auth-test` is standalone defense-lab CLI (distinct from pipeline `ScanProfile::Auth`); local `Auth*` types only (no `ScanReportData` conversion).
- **Mobile Static Analysis**: Standalone defense-lab CLI (`eggsec mobile <path.{apk,ipa}>`) under `mobile` feature (gated command/module, not in TUI or pipeline profiles). Handler uses `evaluate_and_enforce_operation` with `SafeActive` risk + `required_features: ["mobile"]` (local file target, no scope). Pure-Rust ZIP/AXML/plist parsers only. Produces local `Mobile*` findings + `to_scan_report_data` bridge (like wireless). Phase 1 closed 2026-06-11. Dynamic Phase 1 polish (smoke test script `scripts/test-mobile-dynamic.sh`, `--list-devices` convenience, troubleshooting, docs) complete 2026-06-12 per `plans/mobile-dynamic-post-phase1-polish-and-phase2-planning.md` (executed). Phase 2 (proxy Level-1 + permissions + correlation) complete 2026-06-12 per `plans/mobile-dynamic-phase2-implementation-handoff-plan.md` + final polish + close-out (executed); Phase 2 officially closed 2026-06-12 per `plans/mobile-dynamic-phase2-closeout-and-phase3-kickoff-plan.md` (executed; all under mobile-dynamic, no sub-feature split). Phase 3a/3b/3c (Frida foundation + builtins + library + multi-script + advanced correlation + behavioral regression + evidence bundles) delivered 2026-06-12 under single mobile-dynamic per `plans/mobile-dynamic-phase3-frida-expansion-plan.md` (executed; Key Decision: no mobile-frida sub-feature; --frida-script (repeatable, supports "builtin:NAME"/"library:NAME") / --allow-frida + Intrusive policy gate + richer frida_instrumentation (incl. regression_notes) + bridge categories incl. mobile-dynamic-android-frida-* + behavioral-regression; frida.rs real + library + run_frida_spec with CLI fallback; baseline/regression + bundles). Phase 4a (Core Correlation Engine + Evidence Foundation) delivered 2026-06-12 (executed per plan; non-breaking) under single mobile-dynamic: CorrelationEngine + correlate_reports + enriched CorrelatedFinding (score/correlation_type/enrichment) + CorrelationResult (correlations + timeline + summary) + scoring inside correlate_findings; builds on 3c baseline/regression/bundles; 6 new unit tests; all ~85+ mobile-dynamic tests green; dry-run safe/hardware-free; no new deps; serde roundtrips; standalone defense-lab (MCP/agent/TUI/pipeline absent); Phase 4b TUI deferred + reporting polish delivered (human only) per standalone policy. See `commands/handlers/mobile.rs`, `mobile/mod.rs`, `src/mobile/AGENTS.override.md`.
- **Standalone Defense-Lab Surfaces (wireless, mobile, auth-test, db-pentest, web-proxy, evasion, postex)**: Consolidated pattern post-integration-work-plan (2026-06-11). `auth-test`: fully integrated as `Tab::Auth` in the TUI (TabSpec, task system, policy enforcement, session save/restore); local findings only (`Auth*` types, no `ScanReportData` bridge, no pipeline). `wireless` + `mobile`: local types direct (CLI/TUI/human/JSON) + optional `to_scan_report_data` bridge (wired to eggsec-output; native --json auto-bridged in `report convert` handler when feature present). **db-pentest** (`eggsec db pentest ...`, feature `db-pentest`): TUI tab `Tab::DbPentest` (Phase 3) + pipeline `ScanProfile::DbRegression` (Phase 3). Local `DbPentestReport`/`DbFinding` + optional `to_scan_report_data_db` bridge (auto-bridged in `report convert`). Advanced gated checks behind `--allow-db-pentest-advanced` (dry-run always safe). Phase 5 adds MongoDB/Redis engines, cross-DB correlation, compliance mapping, optional MCP exposure via `db-pentest-mcp` marker. Phase 6 adds baseline capture + regression comparison (`--baseline`, `--capture-baseline`, `--baseline-label`), MCP deepening (baseline operations, parameterized calls), extended compliance (NIST/ISO27001). Cleanup + polish complete: shared URL builders in `utils.rs`, standardized error handling, improved redaction, types-only MCP module. See `architecture/database_pentest.md`. **Active wireless reporting bridge (2026-06-12)**: `to_active_scan_report_data()` in `wireless/active/mod.rs` bridges `ActiveWirelessAttackResult` → `ScanReportData` with `wireless-active-*` categories; auto-bridged in `report convert` for `wireless-advanced` feature. **TUI active integration (2026-06-12)**: Wireless tab supports both passive scanning (`wireless` feature) and active attacks — deauth/disassoc (`wireless-advanced` feature). Active mode toggled via `a` key; input fields for BSSID, Client MAC, Frame Count, Rate Limit; dry-run via `d` (default on). Policy confirmation overlay triggers for active attacks (`OperationRisk::Intrusive`). db-pentest now has `ScanProfile::DbRegression` (Phase 3) and `Tab::DbPentest` TUI tab (Phase 3).   **web-proxy** (`eggsec proxy intercept ...`, feature `web-proxy`): standalone defense-lab interactive web proxy for HTTP/HTTPS traffic interception. **Phase 1** complete: MITM server, CA, CLI, dry-run, policy, bridge (`WebProxySessionReport`/`ProxyFlow`/`BudgetUsage` + `to_scan_report_data_proxy`). **Phase 2** complete: Interactive TUI tab `Tab::Intercept` with live flow list, header/body detail panes, manipulation audit trail (`ManipulationRecord`), session save/load (JSON), HAR export, intercept rules display, forward/drop/replay/pause actions. **Phase 3 (2026-06-12)**: Advanced protocols (WebSocket, HTTP/2, gRPC detection and types), enhanced rule engine (`EnhancedRule`/`EnhancedRuleSet` with complex AND/OR/NOT conditions, persistence, new actions), cross-loadout correlation hooks (`CorrelationContext`), TUI protocol detail panes and rule management toggle, extended bridge findings (`proxy-websocket-session`, `proxy-http2-session`, `proxy-grpc-session`, `proxy-correlation-summary`). **Phase 4**: Pipeline profile (`ScanProfile::WebProxy`, `Stage::WebProxy`), MCP proxy surface (12 tools via `web-proxy-mcp` marker feature), evidence bundle v2 (export/import, multi-loadout correlation), performance optimizations (`FlowBuffer` LRU-evicting buffer, `ProxyMetrics` runtime telemetry), real WebSocket/HTTP2 protocol support (`tokio-tungstenite`, `h2`). `web-proxy = []` marker feature; `web-proxy-mcp` optional MCP exposure marker. Other standalone surfaces remain aspirational for pipeline integration (see `architecture/{wireless,mobile,auth,cli_commands,defense_lab,output}.md`, docs/WIRELESS.md + docs/MOBILE.md "Integration with Reporting Pipeline" sections, CAPABILITIES.md Lab Defense). Lightweight opt-in reporting unification only. Auto-bridge lives in `commands/handlers/report.rs`. **Passive wireless = Phase 0 (complete 2026-06-11)**. **Active wireless = Phase 1 (CLI + TUI integration complete 2026-06-12)** — deauth/disassoc commands, reporting bridge, TUI active attacks integration. Design in `plans/wireless-active-attacks-loadout-design-plan.md` (gated by `wireless-advanced`; same standalone + MCP-absent pattern). **Dynamic mobile loadout (design phase complete 2026-06-12 per `plans/dynamic-mobile-testing-loadout-design-plan.md`; Phase 1 complete 2026-06-12; Phase 2 closed 2026-06-12 per combined closeout+kickoff plan (all under mobile-dynamic, no sub-feature split); Phase 3 (Frida) kickoff vision documented). (line truncated to 2000 chars)s/dynamic-mobile-testing-loadout-design-plan.md`; Phase 1 implementation complete 2026-06-12 per `plans/mobile-dynamic-phase1-implementation-handoff-plan.md` (executed); Phase 1 polish (smoke script `scripts/test-mobile-dynamic.sh`, `--list-devices` convenience, troubleshooting, docs + success criteria) complete 2026-06-12 per `plans/mobile-dynamic-post-phase1-polish-and-phase2-planning.md` (executed) — Android ADB core + runtime log analysis; Phase 2a (proxy Level-1 device config + traffic summary + runtime permission testing) complete 2026-06-12 per `plans/mobile-dynamic-phase2-implementation-handoff-plan.md` (executed; Level 1: --proxy/--traffic-capture + grant/revoke/list; `traffic_summary` + `permission_state` in DynamicMobileReport; bridge categories mobile-dynamic-android-traffic-summary etc.; still standalone defense-lab, MCP/agent absent, same pattern as wireless-active + static-mobile + auth-test; no TUI/pipeline/MCP; all under mobile-dynamic) (final polish executed 2026-06-12 per plans/mobile-dynamic-phase2-final-polish-handoff-plan.md (executed); `correlate_findings` + `static_correlation` delivered). `DynamicMobileReport`/`DynamicMobileFinding` + `to_scan_report_data_dynamic` bridge (categories `mobile-dynamic-*`); auto-bridged in `report convert`; no TUI/pipeline/MCP in this round).** Mobile static = Phase 1 complete 2026-06-11. Phase 2 closed 2026-06-12 per combined closeout+kickoff plan (all under mobile-dynamic; Phase 3 Frida vision kickoff documented). Phase 4b: TUI absent/deferred; reporting polish in native human output only. `db-pentest` (Phase 1 complete 2026-06-12): local `DbPentestReport`/`DbFinding`/`LabDbManifest`/`DbTarget`/`CheckType` types + optional `to_scan_report_data_db` bridge (auto-bridged in `report convert` when feature present); no TUI/pipeline/MCP in Phase 1; policy via `DbPentest` risk (real) or `SafeActive` (dry-run) + `DefenseLab` mode + `--allow-db-pentest` (audited narrow override); dry-run always produces complete report; see `plans/non-web-database-pentesting-loadout-design-plan.md` and `plans/database-pentesting-phase1-foundation-handoff-plan.md` (executed). **postex** (`eggsec postex ...`, feature `postex`): standalone defense-lab post-exploitation and LOTL simulation for purple teaming. Local `PostexReport`/`PostexFinding` types + optional `to_scan_report_data` bridge (auto-bridged in `report convert`). MITRE ATT&CK mapped techniques across 4 categories. Dry-run always safe; real mode requires `--allow-postex` + scope. Reversible actions in lab mode.
  A short shared "Output Models" explanation (pipeline full `ScanReportData` vs. wireless/mobile/db-pentest optional bridge vs. `auth-test` local-only) lives in `docs/USAGE.md` (Report Management → Convert Reports). This is the canonical cross-reference; keep language consistent across README, CAPABILITIES, per-module docs, AGENTS, architecture/*.md, and skills. See also docs/WIRELESS.md + docs/MOBILE.md ("Integration with Reporting Pipeline") and architecture/{wireless,mobile,auth,output,cli_commands,defense_lab}.md.
- **TUI Policy Alignment (2026-06-11)**: TUI is now aligned (uses the same central `EnforcementContext::evaluate()` evaluator and `ConfirmationClass` kebab strings for `RequireConfirmation` via `PendingPolicyConfirmation` + `PolicyConfirm` overlay; `PendingAction` remains separate).
- **TUI Architecture & Usability Pass (2026-06-11)**: 10-phase refactor completed (plan: docs/plans/tui-architecture-usability-pass.md). Key artifacts:
  - Phase 1: `UiAction` + decode/apply split (`app/action.rs`; `KeyHandler` now returns actions; `App::apply_action` is the mutation point; decode tests added).
  - Phase 2: `OverlayController` extracted (`app/overlay.rs`); one routing `decode` that asks `topmost_overlay()` and dispatches per-overlay input rules to UiActions; exact 7-level precedence preserved (PolicyConfirm highest); no leaks; tests direct.
  - Phase 3: `TabSpec` registry + `TabCategory`/`TabRiskGroup` (`tabs/spec.rs`); single source for title/stable/cli/desc/category/risk/feature/breadcrumb; `Tab` methods delegate; `all()`/`from_stable_id`/`visible`/`numeric`/`quickswitch`/`palette`/`session` unchanged; feature gating and roundtrips identical.
  - Phase 4: Operation descriptor construction moved toward tabs (`TabSpec.operation` + `direct_launch`, `TabInput::primary_target` default + impls on ~19 tabs, `risk_from_group`; `build_current_operation_descriptor` / `current_tab_target` / `is_direct_launch_tab` reduced to thin delegation; enforcement remains central; direct-launch retro gate and policy confirm unchanged).
  - Phase 5: Manual-mode visibility (status bar preflight via `EnforcementContext::evaluate` + `LoadedScope` provenance + spec risk + "will: run/warn/confirm/deny"; "Mode: manual-permissive", scope source labels, risk badges; advisory only; concise on narrow; policy popup behavior identical).
  - Phase 6: Global task strip (TaskState + started_at; status shows tab/name/state/elapsed/hints even after nav away; help text + compact; pause/resume visible; quit-block not surprising; jump in palette).
  - Phase 7: Palette action-complete (all keybound globals + required list: run-current, stop/pause/resume/jump, quick-switch, help-current, search/global, theme, cycle/export, copy-cli, settings, reload-scope stub, save-settings contextual, clear/delete-history contextual; disabled-with-reason for no-task/no-settings/no-history; tests for global + tab + unavailable).
  - Phase 8: Copy CLI equivalent (`copy_cli_equivalent` using `cli_command()` + primary_target + safe options + `--format` + explicit `--scope`; `shell_escape`; palette "copy-cli" wired; graceful clipboard fail + notif; no broad bypass flags; tests for recon/scan-ports/intrusive/non-exec).
  - Phase 9: Small-terminal degradation (TabWindow breadcrumb on narrow, too-small (<40x10) clear fallback "Terminal too small" (input/quit still work), popups clamped, policy confirm preserved, status/task/preflight drop low-pri first on <60w; 60x20 usable, 80x24 good; layout tests added).
  - Phase 10: Semantic styling tokens (palette.rs 10 new roles: safe/danger/muted/active_task/paused_task/scope_match/miss/policy_required/denied; builtin 3 themes + loader updated; style.rs helpers + `style_for_risk`/`policy_outcome`/`task_state`; adopted in preflight/status/task/policy paths; existing themes + cyber-red fallback + non-blocking load unchanged).
  All phases compile/test green independently. TUI crate 301 tests at end. Docs (this file, README, AGENTS.override.md TUI, architecture/tui.md) updated. See plan for acceptance criteria and validation commands.
- **Manual Discretion Semantics**: `ManualPermissive` (default CLI/TUI) produces `EnforcementOutcome::RequireConfirmation` for operator-discretion cases; `CommandContext::evaluate_and_enforce_operation` converts to proceed only with matching `ManualOverride` flags (audited on the decision; `--yes` narrow for `out-of-scope`/`target-expansion`, dedicated `--allow-private-resolution`/`--allow-cross-host-redirect` etc. for others; precise required-flag error messages). Strict/automated profiles + `ManualGuarded` treat `RequireConfirmation` as hard `Deny` (no proceed path, no overrides honored). Tests in `enforcement_tests.rs` lock narrow `--yes` semantics, dedicated flags, stable `as_str()` kebab strings, and `confirmation_class_strings` dedup behavior.
- **MCP Strict Enforcement**: MCP server requires `EnforcementContext` for all construction. `McpServer` stores no raw `Scope` or separate `ExecutionPolicy` field. `EnforcementContext` is the sole policy/scope authority (2026-06-10). Production MCP startup uses `McpServer::with_enforcement`.
- **Agent Strict Enforcement**: `handle_agent()` now requires explicit scope manifest and refuses to run without it. `EnforcementContext::agent_strict` is passed to `AgentConfig` (2026-06-10); per-scan re-eval in `execute_scan_with_depth`.
- **Scope Provenance Tracking**: `LoadedScope` tracks whether scope came from CLI (`--scope`), config file, or default empty. Strict profiles require `is_explicit_manifest()` for networked operations (2026-06-10); central check inside `EnforcementContext::evaluate`.
- **MCP Profile Tightening**: Both `OpsAgent` and `CodingAgent` MCP profiles now have `require_explicit_scope: true`. Ops-agent retains broader tool visibility but requires explicit scope for networked operations (2026-06-10)
- **Capability Enforcement**: `required_capabilities_for_tool_call()` maps tool IDs to capability requirements. Denied capabilities are enforced across all profiles (2026-06-10)

### Key Patterns (Lessons Learned)

- **TUI bounds checking**: Always use `.get(i)` pattern instead of direct `chunks[i]` indexing
- **TUI is_running() guards**: All input/navigation handlers must check `!self.is_running()` before processing
- **TUI reset() methods**: Must reset all state (selectors, checkboxes, fields, focus areas)
- **TUI edge detection**: `is_at_left_edge()`/`is_at_right_edge()` need `is_empty()` guards
- **Silent error suppression**: Never use `let _ =` or `filter_map(|e| e.ok())` - always log with tracing
- **Timeout wrappers**: All spawned tokio tasks should have timeout wrappers (30-300s depending on operation)
- **FxHashMap migration**: Replace `std::collections::HashMap` with `rustc_hash::FxHashMap` in performance-critical paths
- **Distributed results**: Workers must send `CommandMessage::Result` back to coordinator via channel
- **Verification before claims**: Always verify line numbers, file paths, and whether issues still exist before including in plans
- **File path conventions**: Use `commands/handlers/` not `cli/handlers/` - the latter directory does not exist
- **Dead code detection**: Check if `#![allow(dead_code)]` is at file top - many items flagged in reviews may already be resolved
- **Rate limiter patterns**: Use `tokio::time::sleep()` not spin loops; check if rate limiter is actually used (some are dead code)
- **Bounds check patterns**: Check for existing `if let Some(idx)` or `if len() > N` guards before claiming missing bounds checks
- **Wave plan verification**: When verifying plan claims, use subagents to check actual codebase state - plans may contain stale assertions that no longer match reality
- **Count verification**: Always verify statistical claims (file counts, enum variants, match arms) against actual source. Source file counts can vary by 200+ depending on whether nested crates are included
- **TUI stale detection**: TUI styling fixes may already be applied in a previous pass - always verify before re-implementing. Check actual `.rs` files, not just plan descriptions
- **PayloadType location**: `PayloadType` enum is in `fuzzer/payloads/mod.rs`, not `types.rs`. `types.rs` contains `OutputFormat`, `Severity`, etc.
- **Verification before claims**: Always verify line numbers, file paths, and current implementation state before asserting (e.g. `auth/multi_protocol/` + `ProtocolAuthTester` under `nse-ssh2` feature + declaration in `auth/mod.rs` is implemented and gated, not dead/unreachable).
- **Proxy features exist**: `Tor` ProxyType and `Weighted`/`LowestLatency` rotation strategies already exist in code — verify before claiming they're missing.
- **Feature matrix math**: When verifying feature counts, sum the sub-counts to check for arithmetic errors (e.g., 18+12=30≠28). Correct counts: 16 features-with-deps + 12 marker-only = 28.
- **`.ok()` vs `if let Ok`**: Not all `.ok()` calls are bugs - `if let Ok` is proper error handling that doesn't log, while `.ok()` silently converts `Result` to `Option`. Verify which pattern is used before claiming an issue.
- **`let _ =` pattern verification**: Some `let _ =` usages properly log errors via `tracing::warn!` in subsequent lines - verify the full context before claiming silent suppression.
- **Ownership vs mutation**: `push()` takes ownership, doesn't mutate the pushed item - don't claim TOCTOU issues without verifying whether data is actually modified.
- **JSONL format verification**: Code may correctly use JSONL format (line-delimited JSON) even when documentation claims otherwise. The findings store uses JSONL correctly.
- **AiClient Clone**: Uses `#[derive(Clone)]` at `client.rs:54`, not manual implementation. Don't claim manual implementation without verifying.
- **Method call patterns**: A method being "called unconditionally" isn't a bug if the method internally handles `None` values appropriately.
- **Packaged themes**: Run `python3 scripts/package_themes.py` after modifying `themes/*.toml` to regenerate `crates/eggsec-tui/src/theme/packaged.rs`. The script is deterministic.
- **Theme system**: 50 Halloy-format themes are packaged into the binary via LZMA compression. Packaged theme names are canonicalized to stable IDs, selector labels are display-friendly, and the `cyber-red` fallback theme is always available in-code, independent of file system access.
- **Theme loader**: `theme/loader.rs` parses Halloy `.toml` themes into Eggsec `Theme` structs. Missing fields use defaults from built-in themes.
- **Theme install**: Packaged themes are installed idempotently to the user's config dir (`~/.config/eggsec/themes` on Linux). Existing files are never overwritten.
- **Theme background loading**: Theme loading runs in a background thread (`std::thread::spawn`) with results sent via `std::sync::mpsc`. The receiver, join handle, and deferred restore live in `ThemeLoadState`. `App::update()` polls the channel and joins the loader handle once the final report arrives. `App::spawn_theme_loader()` starts the thread. `new_for_testing()` skips the loader.

### Session Fixes (2026-06-11)

- **Theme cycling**: `Ctrl+T` now cycles ALL themes alphabetically via `list_theme_ids_owned()` (not just built-in trio)
- **Theme default**: `Theme::default()` returns `cyber-red` (was `dark_theme`, disagreed with `ThemeManager::default`)
- **Theme logging**: `set_theme()` logs at debug level when a theme is not found
- **Theme notifications**: Theme install failures surfaced via notification system (no longer silent)
- **Theme fallback**: `set_items_with_extra` on Selector adds missing theme to dropdown without replacing with index 0
- **Content_len cap**: `archive.rs` caps content_len at 1 MiB to prevent pathological allocation
- **Session cleanup**: `.json.tmp` orphans cleaned up on both save paths
- **Session corruption**: `load_latest_session` quarantines corrupt files (`.json.bad`) and tries next
- **Session auto-save**: `auto_save_if_due` skips during active tasks
- **Session fallback path**: `SessionConfig` fallback uses `$HOME/.eggsec/sessions` (was bare `~/.eggsec/sessions`)
- **Session interval**: `auto_save_interval` clamped to min 1 second
- **Session snapshots**: `load_latest_session` filters out `quick_save.json` from snapshot candidates

### Session Fixes (2026-06-17)

- **Theme luminance bug fixed**: `luminance()` in `theme/loader.rs` now correctly handles 3-char hex shorthand (#FFF → #FFFFFF) instead of returning neutral 0.5
- **Dead style methods removed**: Removed `style_for_tab`, `style_for_mode`, `style_for_status` from `theme/style.rs` (never called anywhere)
- **Dead manager methods removed**: Removed deprecated `register_theme_if_absent` and dead `set_current_by_name` from `theme/manager.rs` + 4 associated tests
- **Theme toggle result handling**: `ThemeManager::toggle()` now logs debug on `set_theme` failure instead of discarding via `let _ =`
- **Worker error handling**: All 15 `let _ =` on channel sends across `workers/security.rs`, `c2_worker.rs`, `intercept_worker.rs`, `db_pentest.rs` now use `if let Err(e) = ... { tracing::warn!(...) }`
- **Dead shim methods removed**: Removed 20 dead transition shim methods from `app/key_handler.rs` (~180 lines) and 3 dead shim methods from `app/overlay.rs` (~25 lines)
- **Dead settings methods removed**: Removed `sync_with_theme` and `sync_theme_selector` from `tabs/settings/main.rs` (never called)
- **Session error handling**: Quarantine rename and orphan cleanup in `session.rs` now log errors instead of silent `let _ =`
- **Dead theme macro removed**: Removed unused `theme!()` macro from `theme/legacy.rs` (only `tc!()` was used)
- **Dead style calls removed**: Removed `style_for_risk()` and `scope_match()`/`scope_miss()` calls that assigned to `let _` in `ui/shell.rs` (results discarded)
- **Hardcoded colors fixed**: Replaced 15 hardcoded `Color::Red/Gray/Yellow/DarkGray/Cyan` in `tabs/wireless.rs` and 2 in `tabs/intercept.rs` with `tc!()` theme tokens for proper theme support
- **handle_enter() Results guard**: Fixed `tabs/graphql.rs` and `tabs/oauth.rs` to not trigger `start()` from Results focus area; added `Results` guard to `tabs/db_pentest.rs`
- **page_up/page_down guard**: Added `is_running()` guard to `tabs/cluster.rs` page navigation
- **Session cleanup perf**: Changed `sessions.remove(0)` O(n) to `swap_remove(0)` O(1) in `session.rs`
- **Dead code cleanup**: Removed empty `if is_advanced {}` block, `let _ = d` PolicyDecision discard, stale `#[allow(unused_variables)]`, and redundant `#[cfg(feature)]` pairs in `app/export.rs`

### Session Fixes (2026-06-17) - Deep Audit

- **UTF-8 panic fix**: `ui/shell.rs:201,341` used byte-offset slicing which panics on multi-byte characters. Changed to character-aware truncation
- **handle_enter() scan-from-input**: `graphql.rs`, `oauth.rs`, `cluster.rs` started scans when Enter pressed in input fields (blurred without returning). Added `return;` after blur. `wireless.rs` added Results focus area guard
- **Theme loader luminance()**: Named colors like "black" were misclassified as Light mode. Extended to handle named colors
- **Theme loader has_any_color**: Check omitted `buttons` section — added guard
- **db_pentest handle_left/right**: Missing `is_running()` guard — added
- **proxy page_up/page_down**: Ignored `page_size` param, hardcoded 20. Fixed to use parameter
- **graphql/oauth page_up/page_down**: Missing overrides — PageUp/PageDown were non-functional. Added delegates
- **auth handle_escape**: Transitions to Results instead of Target. Fixed
- **runner.rs config error**: `.ok()` silently swallowed parse errors. Changed to `match` with `tracing::warn!`
- **workers/auth.rs**: Dead `if let Some(ref cred_file)` block removed
- **help_config.rs**: Stale Ctrl+T description "Cycle built-in theme" → "Cycle theme"
- **Dead code cleanup**: Removed stale `#[allow(dead_code)]` on `InputField.label`, replaced blanket `#[allow(dead_code)]` on Popup impl with per-method annotations

### Session Fixes (2026-06-18) - TUI Audit

- **graphql.rs handle_enter() fallthrough**: Options arm toggled checkbox then fell through to `self.start()`, silently starting a scan. Added `return;` after toggle
- **oauth.rs handle_enter() fallthrough**: Same pattern — Options arm fell through to `self.start()`. Added `return;` after toggle
- **intercept.rs truncate_str() UTF-8 panic**: Used byte-offset slicing `&s[..max_len]` which panics on multi-byte characters. Changed to character-aware truncation via `.chars().take()`
- **settings/main.rs Session max_focus_index**: Returned `1` but `session_inputs` has only 1 field (index 0). Changed to `0`
- **theme/loader.rs luminance() named colors**: `lightblue`/`lightred`/`darkgreen` etc. shared luminance values with base colors, misclassifying Light/Dark mode. Fixed to use distinct values (light* → higher, dark* → lower)
- **popup.rs scroll cast truncation**: `scroll_offset as u16` silently truncated values > 65535. Added `.min(u16::MAX as usize)` clamp
- **popup.rs button width u16 overflow**: Button width sum could overflow u16. Changed to `saturating_add`
- **session.rs swap_remove(0) order**: `cleanup_old_sessions` used `swap_remove(0)` which broke sorted order, deleting wrong sessions. Changed to `remove(0)`
- **db_pentest.rs allow_db_pentest hardcoded**: Worker unconditionally passed `allow_db_pentest: true`. Changed to pass `dry_run` value to respect lib safety gate
- **selector.rs height overflow**: Dropdown height calculation could overflow on extreme item counts. Added `.min(u16::MAX as usize - 2)` clamp
- **help_scroll_offset usize::MAX**: `HelpScrollBottom` set offset to `usize::MAX` which could cause unexpected behavior. Changed to `u16::MAX as usize`
- **ThemeInstallReport Clone data loss**: Lossy `Clone` impl silently dropped `loaded_themes` Vec. Removed impl (never cloned; consumed via channels)

### Session Fixes (2026-06-18) - TUI Audit

- **db_pentest worker missing timeout**: `run_db_pentest_cli()` called without `tokio::time::timeout` — hung database connections blocked TUI permanently. Wrapped in 60s timeout with three-arm match pattern
- **session.rs load_quick() quarantine**: Corrupt `quick_save.json` propagated error directly — hard failure, no recovery, session lost. Added quarantine logic matching `load_latest_session` pattern (rename to `.json.bad`, log warning, return `Ok(None)`)
- **intercept.rs page_up/page_down page_size**: Methods accepted `_page_size` parameter but hardcoded `20`. Changed to use the parameter
- **intercept.rs edit_modal reset**: `reset()` didn't clear `edit_modal` — stale modal state persisted after tab reset. Added `close_edit_modal()` call
- **packet.rs page_up/page_down**: Missing from `impl TabInput` — PageUp/PageDown keys were no-ops. Added both methods delegating to `results_view`
- **5 tabs missing handle_copy()**: `load.rs`, `report.rs`, `auth.rs`, `c2.rs`, `db_pentest.rs` silently ignored Ctrl+C. Added `handle_copy()` implementations
- **runner.rs redundant .map()**: Removed no-op `.map(|ls| ls)` identity transform
- **command.rs silent let _ =**: `set_current_tab_if_available` failure discarded. Changed to log on failure
- **workers/security.rs silent HTTP error**: Compliance preflight request error silently discarded. Added `tracing::debug!`
- **session.rs metadata error swallowing**: Double `.ok()` in tmp cleanup silently swallowed metadata errors. Changed to explicit `match` with logging

### Session Fixes (2026-06-18) - TUI Bug Fixes

- **Settings Theme reload path**: Normal-mode `r` in Settings > Theme now correctly triggers `UiAction::ReloadThemes` instead of `ResetCurrent`. The `ReloadThemes` action directly spawns the theme loader with `ManualReload` reason. Insert-mode `r` path via `pending_theme_reload` still works for backward compatibility
- **Theme action hints section-aware**: `settings_hints()` in `action_hints.rs` now takes `&app` and returns different hints based on `current_section` and `theme_selector.is_open()`
- **Theme source attribution fixed**: `load_themes_from_dir()` now accepts a `packaged_ids: &FxHashSet<String>` parameter and sets `ThemeSource::Packaged` vs `ThemeSource::Custom` based on file stem membership. `ThemeInstallReport.loaded_themes` changed from `Vec<Result<Theme, ThemeLoadError>>` to `Vec<LoadedThemeRecord>`
- **Invalid themes tracked**: `ThemeManager::register_theme_invalid()` method added. `handle_theme_install_report()` calls it for error records
- **Contrast warnings per-theme**: `SettingsTab` gained `theme_contrast_cache: FxHashMap<String, Vec<String>>` field (per-theme contrast warnings). `update_theme_metadata()` now computes and stores per-theme contrast warnings. Render shows actual warnings from cache
- **Theme load reason**: `ThemeLoadReason` enum added to `ThemeLoadState`. `spawn_theme_loader_with_reason()` shows notifications for manual reload
- **Task hints detection**: `get_action_hints()` uses `app.has_active_task()` instead of `app.task_state.handle.is_some()`
- **Numeric tab jump off-by-one fixed**: `key_handler.rs` numeric decode now maps `'1'..='9'` to `digit - 1` for `Tab::from_visible_index()`, and `'0'` to index 9. 5 new tests
- **Warning cleanup**: Removed unused `ThemeSource` import (`theme_runtime.rs`), unused `AppState` test import (`action_hints.rs`), unused `all_specs` re-export (`tabs/mod.rs`). Added `#[allow(dead_code)]` to test-only `TabSpec` fields (`supports_run`, `supports_export`, `supports_help`, `has_settings`) and methods (`can_start_task`, `shows_in_export`). TUI crate now has zero warnings in both lib and test builds
- **Settings layout split**: `render.rs` now splits `inner` into `body`, optional `status`, and `footer` rows before rendering any section content. FormBuilder renders into `body` only. Status message uses severity-aware styling (error/warning/success). 3 new layout tests (80x24, status collision, 60x20 small terminal)
- **Theme preview uses selected theme colors**: SettingsTab gained `theme_contrast_cache: FxHashMap<String, Vec<String>>` (per-theme contrast warnings) and `resolved_theme_colors: Option<ThemeColors>` (for preview). Preview render uses resolved colors instead of `tc!()` thread-local. `handle_theme_install_report` computes contrast for all loaded themes and resolves the selected theme's colors. `update_settings_theme_selector` also resolves colors. 1 new per-theme contrast test
- **Theme metadata enrichment**: Theme render now shows status label (Loaded/adjusted/invalid/missing), loaded/invalid/fallback-adjusted counts, and per-theme contrast warnings from cache. Contrast validation expanded to 9 semantic pairs (text, selected_text, text_dim, warning, error, success, mode_normal, mode_insert, focus_input vs background)
- **Reload notification enhanced**: Manual reload notification now includes loaded, invalid, and error counts with severity-aware styling

### Active Wireless Reporting Bridge (2026-06-12)

- **Bridge function**: `to_active_scan_report_data()` in `wireless/active/mod.rs` bridges `ActiveWirelessAttackResult` → `ScanReportData` with `wireless-active-*` finding categories (deauth, disassoc, etc.).
- **Auto-bridge**: Wired in `report.rs` for `eggsec report convert` — native JSON from active wireless commands auto-bridges when `wireless-advanced` feature is present.
- **TUI active integration**: Wireless tab now supports active attacks (deauth/disassoc) with `wireless-advanced` feature; active mode via `a` key, dry-run via `d`, live non-dry-run operations require the policy confirmation overlay while dry-run stays `SafeActive`.
- **Tests added**: Unit tests for `to_active_scan_report_data()` bridge covering BSSID and non-BSSID cases.

### TUI Wireless Active Execution Completion (2026-06-12)

- **Execution path wired**: `WirelessTab::handle_enter()` now launches the active attack when in `ActiveConfig` focus and `active_attack_config()` is valid (previously only blurred inputs and returned). New `start_active_attack()` helper transitions the tab to `AppState::Running`, clearing `active_results` and `results_view`/`error`.
- **Tab Spec integration**: The wireless TabSpec is `direct_launch: true`, so `App::handle_enter()` retroactively evaluates the policy descriptor (promoted to `OperationRisk::SafeActive` for dry-run or `OperationRisk::Intrusive` for live attacks under `OperationMode::DefenseLab`) and routes through `EnforcementContext::evaluate()` + `request_policy_confirmation()` exactly like Auth/Stress/Packet.
- **Task system**: `TaskConfig::WirelessActive` + `TaskResult::WirelessActive` (worker `run_wireless_active_task`) were already in place; results flow back through `state_update.rs` to `set_active_results()`. Dry-run is the default and proceeds without the confirmation overlay; live attacks surface `RequireConfirmation` through the existing enforcement path.
- **UX polish**: Passive results view no longer states "Active attacks are available via CLI only" — replaced with a TUI-side tip describing the `a`/`d` keys and Enter behavior. Help popup (popup.rs) reflects active mode in its Enter binding description.
- **Tests added**: 12 new unit tests under `tabs/wireless::tests` (under `wireless-advanced` feature): `active_attack_config` (inactive / no interface / values / omitted optional MACs), `set_active_results` rendering and state transition, `toggle_active_mode` clears inputs, `toggle_dry_run` flips, `start_active_attack` (valid -> Running, invalid -> Idle), `handle_enter` (valid active config -> Running, invalid -> blur). Two additional descriptor tests in `app/mod.rs` (`test_wireless_active_descriptor_uses_safeactive_for_dry_run` + `test_wireless_active_descriptor_uses_intrusive_for_live_attack`) lock the `SafeActive` / `Intrusive` risk gating. Total: 14 tests across the active flow.
- **Plan completed**: `plans/wireless-active-tui-execution-completion-plan.md` (focused plan for TUI execution path) is now closed.
- **Stale plan closed**: `plans/wireless-active-tui-execution-closure-plan.md` (drafted 2026-06-12) was a duplicate of this work — all five "remaining gaps" it called out were already resolved by the commits cited above. The plan now carries a resolution note with file:line references to the shipped implementation.
- **Final wiring-and-polish plan resolved (2026-06-12)**: `plans/wireless-active-tui-final-wiring-and-polish-plan.md` closed after this verification pass; lingering references to removed `wireless_active_handler.rs`/`dispatcher_wiring_example.rs` and `TaskBuilder::new(task_config)` patterns cleaned from historical plans; one E2E-style test addition context and cross-doc accuracy confirmed (README, architecture/*, docs/WIRELESS.md, skills).

## Skills Directory

Skills are located in `.opencode/skills/`:

| Skill | Purpose |
|-------|---------|
| `eggsec-agent/` | Agent-specific workflows |
| `eggsec-ai/` | AI module workflows |
| `eggsec-architecture-review/` | Architecture document review methodology |
| `eggsec-auth/` | Authentication security testing workflows (CLI `auth-test` primary; TUI `AuthTab` fully integrated as `Tab::Auth`) |
| `eggsec-browser/` | Headless browser security testing |
| `eggsec-cli/` | CLI parsing, command dispatch, handler patterns |
| `eggsec-config/` | Config module workflows |
| `eggsec-distributed/` | Distributed module workflows |
| `eggsec-fuzzer/` | Fuzzer module workflows |
| `eggsec-hunt/` | Vulnerability hunting workflows |
| `eggsec-loadtest/` | Loadtest module workflows |
| `eggsec-nse/` | NSE/Lua module workflows |
| `eggsec-output/` | Output module workflows |
| `eggsec-packet/` | Packet capture/crafting/parsing workflows |
| `eggsec-pipeline/` | Pipeline module workflows |
| `eggsec-proxy/` | Proxy module workflows |
| `eggsec-recon/` | Reconnaissance module workflows |
| `eggsec-scanner/` | Scanner module workflows |
| `eggsec-security/` | Security testing skill workflows |
| `eggsec-stress/` | Stress module workflows |
| `eggsec-tool/` | Tool module workflows |
| `eggsec-tui/` | TUI module workflows (includes `tui_testing.md` for visual regression patterns) |
| `eggsec-waf/` | WAF module workflows |
| `eggsec-wave-implementation/` | Historical wave implementation reference (all waves completed 2026-06-02) |

Wireless-specific guidance lives in `.opencode/skills/eggsec-agent/wireless_security_testing.md` and should be used when updating the wireless workflow, CLI help, or lab-use guidance.

Use the `skill` tool to load relevant skills when tackling tasks in their domain.

## Architecture Documentation

Detailed architecture documentation is in the `architecture/` directory:

| File | Module |
|------|--------|
| `architecture/overview.md` | System-wide architecture, module index, data flow |
| `architecture/cli_commands.md` | CLI parsing, command dispatch, handler patterns |
| `architecture/ai_agents.md` | AI/LLM integration and autonomous agents |
| `architecture/config.md` | Configuration system, scope enforcement |
| `architecture/scanner.md` | Port scanning and endpoint discovery |
| `architecture/fuzzer.md` | Fuzzing engine and payload generation |
| `architecture/waf.md` | WAF detection and bypass |
| `architecture/recon.md` | Reconnaissance module |
| `architecture/pipeline.md` | Security assessment pipeline |
| `architecture/distributed.md` | Distributed coordinator/worker architecture |
| `architecture/loadtest.md` | HTTP load testing and benchmarking |
| `architecture/networking.md` | Networking & packets module |
| `architecture/output.md` | Output & reporting module |
| `architecture/nse_integration.md` | NSE integration |
| `architecture/tui.md` | Terminal User Interface (TUI) module, 33 tabs, event loop, components |
| `architecture/compile_time_baseline.md` | Workspace crate layout and compile-time baseline |
| `architecture/defense_lab.md` | Defense-lab mode and regression validation |
| `architecture/stress.md` | Stress testing module (raw sockets, IP spoofing) |
| `architecture/utils.md` | Utility functions (23 submodules) |
| `architecture/types.md` | Core types (Severity, SensitiveString, OutputFormat) |
| `architecture/constants.md` | Centralized constants |
| `architecture/probe.md` | Probe classification (ProbeIntent, ProbeRisk) |
| `architecture/auth_context.md` | Auth context YAML parsing |
| `architecture/logging.md` | Logging configuration |
| `architecture/api_extraction_boundary.md` | API/agent extraction boundary analysis |
| `architecture/generated.md` | Auto-generated protobuf code |
| `architecture/auth.md` | Authentication testing module |
| `architecture/browser.md` | Headless browser security testing |
| `architecture/compliance.md` | Compliance scanning |
| `architecture/container.md` | Kubernetes/Docker scanning |
| `architecture/diff.md` | Scan result diffing |
| `architecture/error.md` | Error type catalog |
| `architecture/feature_matrix.md` | Feature flags reference |
| `architecture/findings.md` | Finding store and lifecycle |
| `architecture/hunt.md` | Advanced threat hunting |
| `architecture/integrations.md` | External service integrations |
| `architecture/notify.md` | Notification system |
| `architecture/proxy.md` | Proxy pool management |
| `architecture/storage.md` | Database persistence |
| `architecture/supply_chain.md` | SBOM generation |
| `architecture/vuln.md` | Vulnerability triage |
| `architecture/websocket.md` | WebSocket security testing |
| `architecture/wireless.md` | Standalone-complete passive WiFi scanning + active attacks (summary-by-default rogue heuristic; `--detect-suspicious` expands details; `--repeat`, `--known-good`, `--dry-run`; WPS/hidden/transition; deauth/disassoc under `wireless-advanced`). TUI tab with full passive + active integration; MCP/agent exposure intentionally absent (standalone defense-lab). **Phase 0 passive + Phase 1 active = complete (2026-06-12)**. See also `docs/USAGE.md` Output Models. |
| `architecture/evasion.md` | Evasion technique detection (MITRE ATT&CK mapped; standalone defense-lab) |
| `architecture/postex.md` | Post-exploitation and LOTL simulation (MITRE ATT&CK mapped; standalone defense-lab) |
| `architecture/web_proxy.md` | Interactive MITM web proxy (HTTP/HTTPS/WebSocket/HTTP2/gRPC; Phases 1-5 complete) |
| `architecture/database_pentest.md` | Database pentesting (Postgres/MySQL/MSSQL/MongoDB/Redis; Phases 1-6 complete) |
| `architecture/workflow.md` | Finding lifecycle management |

## Verification Commands

```bash
cargo check --lib -p eggsec
cargo check -p eggsec-tui
cargo check -p eggsec-cli
cargo check -p eggsec-nse
cargo check -p eggsec-tool-core
cargo check -p eggsec-output
cargo test --lib -p eggsec
cargo test --test negative_tests -p eggsec
cargo test --test scanner_tests -p eggsec
cargo clippy --lib -p eggsec

# Wireless feature (parsing/analysis tests require no hardware; TUI tab under feature)
cargo check -p eggsec --features wireless
cargo check -p eggsec-tui --features wireless
cargo test --lib -p eggsec --features wireless
cargo clippy --lib -p eggsec --features wireless

# Wireless-advanced (Phase 1 deauth; requires wireless feature)
cargo check -p eggsec --features wireless-advanced
cargo test --lib -p eggsec --features wireless-advanced
cargo clippy --lib -p eggsec --features wireless-advanced

# Mobile-dynamic (no hardware required for unit tests)
cargo check -p eggsec --features mobile-dynamic
cargo test --lib -p eggsec --features mobile-dynamic
cargo clippy --lib -p eggsec --features mobile-dynamic

# db-pentest (Phase 1-6 complete; baseline/regression + MCP deepening + compliance)
cargo check -p eggsec --features db-pentest
cargo test --lib -p eggsec --features db-pentest
cargo clippy --lib -p eggsec --features db-pentest

# web-proxy (dry-run; no hardware required)
cargo check -p eggsec --features web-proxy
cargo test --lib -p eggsec --features web-proxy
cargo clippy --lib -p eggsec --features web-proxy

# web-proxy-mcp (MCP proxy surface)
cargo check -p eggsec --features web-proxy-mcp
cargo test --lib -p eggsec --features web-proxy-mcp
cargo clippy --lib -p eggsec --features web-proxy-mcp

# Evasion detection (standalone defense-lab)
cargo check -p eggsec --features evasion
cargo test --lib -p eggsec --features evasion
cargo clippy --lib -p eggsec --features evasion

# Post-exploitation simulation (standalone defense-lab)
cargo check -p eggsec --features postex
cargo test --lib -p eggsec --features postex
cargo clippy --lib -p eggsec --features postex

# C2 framework (standalone defense-lab; depends on postex + evasion)
cargo check -p eggsec --features c2
cargo test --lib -p eggsec --features c2
cargo clippy --lib -p eggsec --features c2

# C2 MCP tool exposure
cargo check -p eggsec --features c2-mcp
cargo test --lib -p eggsec --features c2-mcp
cargo clippy --lib -p eggsec --features c2-mcp
```

### Session Fixes (2026-06-18) - TUI Bugs Plan

- **Settings validation** (`tabs/settings/main.rs`): Added `validate()` method returning `Result<(), Vec<String>>` for all numeric fields and report format. `save_config()` now validates before writing; invalid values produce error status message instead of silent fallback. 14 new unit tests.
- **Theme named-color unification** (`theme/loader.rs`): New shared `named_color()` function with all 27 named colors. `parse_hex_color()` and `luminance()` now share one table. 5 new tests.
- **Theme contrast validation** (`theme/contrast.rs`): New module with `relative_luminance()`, `contrast_ratio()`, `check_contrast()`. Loaded themes validate text/background and selected_text/selected contrast (min 4.5:1). Low contrast triggers fallback to base theme with warning (non-fatal). 7 new tests.
- **Selector dropdown clamping** (`components/selector.rs`): `dropdown_info()` now takes `viewport_height: u16` parameter. Dropdowns clamp to viewport and flip above anchor when no room below. 6 new tests, 10 call sites updated across 6 files.
- **Overlay leakage tests** (`app/overlay.rs`): 14 new tests verifying Ctrl+C bubbles, unknown keys produce Noop, overlay precedence.
- **Enter/Escape regression tests** (6 tab files): 18 new tests across recon, load, scan_ports, fingerprint, stress, packet tabs. Verify Enter in focused input blurs without starting, Options toggle without starting, Results no-op.
- **Explicit theme render path** (`components/selector.rs`, `components/input.rs`): `render_with_theme()` methods for Selector, Checkbox, InputField. Existing `render()` delegates to theme-based version. 3 new tests.
- **AGENTS.override.md**: Fixed stale "Ctrl+T cycles built-in trio" → "Ctrl+T cycles all registered themes alphabetically".

### Session Fixes (2026-06-18) - TUI Bug Fixes Plan

- **Settings Theme dead selected/applied state fixed**: `tabs/settings/render.rs` now renders "Selected/Applied" labels using `SettingsTab.applied_theme_id` (set from `ThemeManager::current_id()`). Dead `applied_name`/`show_applied` locals removed. `ThemeManager` gained `current_id: String` field and `current_id()` accessor. 2 new tests.
- **Settings Theme Preview stale while browsing fixed**: Added `needs_theme_preview_refresh` flag to `SettingsTab`, set when theme selector moves (Up/Down) or opens/cancels. `App::maybe_refresh_theme_preview()` checks the flag after dispatch and refreshes `resolved_theme_colors`. Preview now tracks highlighted theme in real time.
- **Low-Contrast Theme FallbackAdjusted status fixed**: `halloy_to_theme()` now returns `ThemeLoadOutcome` with `pre_adjustment_warnings` captured before color mutation. `install.rs` uses these pre-adjustment warnings instead of recomputing on already-adjusted theme. `ThemeInstallReport` gained `adjusted: usize` field. Manual reload notification now includes adjusted count.
- **InputGroup stale focus hardening**: Added `valid_focused_index()` (mutable, clears stale) and `valid_focused_index_ref()` (read-only) helpers. All `InputGroup` methods now use these instead of raw `self.fields[idx]` access. 6 new tests covering stale insert, blur, move_left, get_focused_value, focus_next recovery, focus_prev recovery.
- **Word-backward 'b' binding matches help**: Added lowercase `b` → `MoveWordBackward` binding alongside shifted `B`. Help text and key handling now agree. 2 new decode tests.
- **Help overlay hints fixed**: Changed from `h/l:pane` (non-existent) to `j/k:scroll g/G:top/end` (implemented actions). Updated `action_hints.rs`, `ui/shell.rs` status bar, and `help_config.rs`. 1 new test.

### Session Fixes (2026-06-18) - TUI Bug Fixes (continued)

- **Numeric tab jump off-by-one fixed**: `key_handler.rs` numeric decode now maps `'1'..='9'` to `digit - 1` for `Tab::from_visible_index()`, and `'0'` to index 9. 5 new tests.
- **Warning cleanup**: Removed unused `ThemeSource` import (`theme_runtime.rs`), unused `AppState` test import (`action_hints.rs`), unused `all_specs` re-export (`tabs/mod.rs`). Added `#[allow(dead_code)]` to test-only `TabSpec` fields (`supports_run`, `supports_export`, `supports_help`, `has_settings`) and methods (`can_start_task`, `shows_in_export`). TUI crate now has zero warnings in both lib and test builds.
- **Settings layout split**: `render.rs` now splits `inner` into `body`, optional `status`, and `footer` rows before rendering any section content. FormBuilder renders into `body` only. Status message uses severity-aware styling (error/warning/success). 3 new layout tests (80x24, status collision, 60x20 small terminal).
- **Theme preview uses selected theme colors**: SettingsTab gained `theme_contrast_cache: FxHashMap<String, Vec<String>>` (per-theme contrast warnings) and `resolved_theme_colors: Option<ThemeColors>` (for preview). Preview render uses resolved colors instead of `tc!()` thread-local. `handle_theme_install_report` computes contrast for all loaded themes and resolves the selected theme's colors. `update_settings_theme_selector` also resolves colors. 1 new per-theme contrast test.
- **Theme metadata enrichment**: Theme render now shows status label (Loaded/adjusted/invalid/missing), loaded/invalid/fallback-adjusted counts, and per-theme contrast warnings from cache. Contrast validation expanded to 9 semantic pairs (text, selected_text, text_dim, warning, error, success, mode_normal, mode_insert, focus_input vs background).
- **Reload notification enhanced**: Manual reload notification now includes loaded, invalid, and error counts with severity-aware styling.

## Planning Notes for Future Agents

When implementing items:

1. **Plan lifecycle**: Implementation plans in `plans/` are executed and then cleaned up (deleted) after completion. Docs and AGENTS.md may still reference plans that no longer exist on disk — this is expected. The plans served their purpose during execution. Focus on the current codebase state rather than plan files.

2. **Verify before implementing**: Always verify file paths, line numbers, and whether issues still exist before implementing.

2. **Error pattern verification**: When addressing silent error suppression issues, verify the full context - some `let _ =` patterns are followed by proper error logging, and some `.ok()` usages are actually `if let Ok` patterns which are correct.

3. **Wave plan verification**: When verifying plan claims, use subagents to check actual codebase state - plans may contain stale assertions that no longer match reality.

4. **Count verification**: Always verify statistical claims (file counts, enum variants, match arms) against actual source. Source file counts can vary by 200+ depending on whether nested crates are included.
