# AGENTS.md

Guidelines for AI agents working on this codebase.

## Quick Verification

Before claiming code is correct, run these in order:

```bash
cargo fmt --all --check          # format
cargo clippy --lib -p eggsec     # lint (pre-existing warnings OK)
cargo test --lib -p eggsec       # unit tests
cargo test -p eggsec --test feature_matrix   # feature metadata
cargo test -p eggsec --test enforcement_matrix
bash scripts/check-architecture-guards.sh    # requires ripgrep
```

Or use the Makefile (requires `cargo-nextest`): `make check-architecture-ci`

Feature-gated crates need explicit features: `cargo check -p eggsec --features mobile`, `cargo check -p eggsec --features db-pentest`, etc.

## Project Overview

Eggsec is a Rust-based security testing toolkit organized as a workspace with 13 crates: `eggsec-core`, `eggsec-tool-core`, `eggsec`, `eggsec-nse`, `eggsec-tui`, `eggsec-cli`, `eggsec-output`, `eggsec-agent`, `eggsec-db-lab`, `eggsec-web-proxy`, `eggsec-mobile-lab`, `eggsec-runtime`, and `eggsec-ui-model`. The `eggsec-runtime` crate provides frontend-neutral task lifecycle management (`Runtime`, `RuntimeConfig`, `RuntimeTaskExecutor` trait) used by TUI, CLI, REST, MCP, and agent surfaces. See `README.md` for features and `architecture/overview.md` for design details.

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
cargo check -p eggsec-tui
cargo check -p eggsec-cli
cargo check -p eggsec-nse
cargo check -p eggsec-output
cargo check -p eggsec-db-lab
cargo check -p eggsec-mobile-lab
cargo test -p eggsec-core
cargo test -p eggsec-tool-core
cargo test -p eggsec-output
cargo test -p eggsec-db-lab
cargo test -p eggsec-mobile-lab
cargo check -p eggsec-runtime
cargo test -p eggsec-runtime
cargo check -p eggsec-ui-model
cargo test -p eggsec-ui-model
cargo check -p eggsec-daemon
cargo test -p eggsec-daemon                    # daemon tests including persistence layer (145 tests)
cargo check -p eggsec-daemon --features http-api
cargo test -p eggsec-daemon --features http-api
cargo check -p eggsec-web-proxy
cargo test -p eggsec-web-proxy
cargo test --lib -p eggsec
cargo test --test negative_tests -p eggsec
cargo test --test scanner_tests -p eggsec
cargo test --test enforcement_matrix -p eggsec
cargo test -p eggsec --test feature_matrix
cargo check --workspace --no-default-features
cargo check -p eggsec --features rest-api
cargo check -p eggsec --features db-pentest
cargo check -p eggsec --features mobile
cargo check -p eggsec --features web-proxy
cargo check -p eggsec --features wireless
cargo test -p eggsec --features rest-api --test enforcement_matrix
cargo clippy --lib -p eggsec
cargo build --release -p eggsec-cli
```

#### Feature-Specific Build & Test

```bash
# CLI headless (no TUI, no daemon client)
cargo check -p eggsec-cli --no-default-features
cargo test -p eggsec-cli --no-default-features

# CLI daemon client
cargo check -p eggsec-cli --no-default-features --features daemon-client
cargo test -p eggsec-cli --no-default-features --features daemon-client

# db-pentest (domain crate)
cargo check -p eggsec-db-lab
cargo test -p eggsec-db-lab
cargo clippy -p eggsec-db-lab

# db-pentest (main crate with adapter)
cargo check -p eggsec --features db-pentest
cargo test --lib -p eggsec --features db-pentest
cargo clippy --lib -p eggsec --features db-pentest

# NSE (domain crate)
cargo check -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse
cargo test -p eggsec-tui --features nse -- nse_report_view
cargo test -p eggsec-nse --test execution_limits_tests
cargo test -p eggsec-nse --lib -- registry      # library registry
cargo test -p eggsec-nse --test profile_tests
cargo test -p eggsec-nse --test profile_guard_tests
cargo test -p eggsec-nse --test script_file_policy_tests
cargo test -p eggsec-nse --test sandbox_tests
cargo test -p eggsec-nse --test compatibility_corpus_tests
cargo test -p eggsec-nse --test compatibility_corpus_tests -- corpus_harness  # data-driven harness only
cargo test -p eggsec-nse --test rule_evaluation_tests
cargo test -p eggsec-nse --test profile_propagation_tests
cargo clippy -p eggsec-nse --features nse

# Wireless
cargo check -p eggsec --features wireless
cargo test --lib -p eggsec --features wireless
cargo clippy --lib -p eggsec --features wireless

# wireless-advanced (deauth; requires wireless feature)
cargo check -p eggsec --features wireless-advanced
cargo test --lib -p eggsec --features wireless-advanced
cargo clippy --lib -p eggsec --features wireless-advanced

# mobile-dynamic
cargo check -p eggsec --features mobile-dynamic
cargo test --lib -p eggsec --features mobile-dynamic
cargo clippy --lib -p eggsec --features mobile-dynamic

# mobile-lab (domain crate)
cargo check -p eggsec-mobile-lab
cargo test -p eggsec-mobile-lab
cargo clippy -p eggsec-mobile-lab

# mobile-dynamic (domain crate with feature)
cargo check -p eggsec-mobile-lab --features mobile-dynamic
cargo test -p eggsec-mobile-lab --features mobile-dynamic
cargo clippy -p eggsec-mobile-lab --features mobile-dynamic

# web-proxy (domain crate)
cargo check -p eggsec-web-proxy
cargo test -p eggsec-web-proxy
cargo clippy -p eggsec-web-proxy

# web-proxy (main crate with adapter)
cargo check -p eggsec --features web-proxy
cargo test --lib -p eggsec --features web-proxy
cargo clippy --lib -p eggsec --features web-proxy

# web-proxy-mcp
cargo check -p eggsec --features web-proxy-mcp
cargo test --lib -p eggsec --features web-proxy-mcp
cargo clippy --lib -p eggsec --features web-proxy-mcp

# Evasion, postex, c2
cargo check -p eggsec --features evasion
cargo test --lib -p eggsec --features evasion
cargo check -p eggsec --features postex
cargo test --lib -p eggsec --features postex
cargo check -p eggsec --features c2
cargo test --lib -p eggsec --features c2
cargo check -p eggsec --features c2-mcp
cargo test --lib -p eggsec --features c2-mcp

# Command registry
cargo test -p eggsec --test command_registry

# Architecture guards (CI required)
bash scripts/check-architecture-guards.sh
```

#### Make Targets

Requires `cargo-nextest` (`cargo install cargo-nextest`). Uses `cargo-nextest` instead of `cargo test`.

```bash
make test          # unit tests only (default, fast)
make test-ci       # full suite, no retries (CI-style)
make test-integration  # integration tests (wiremock, may need network)
make test-nse      # NSE tests (requires nse feature)
make test-slow     # run ignored tests
make clippy        # lint (-D warnings)
make fmt           # format check
make test-coverage # llvm-cov with rest-api,nse features
make test-feature-matrix  # feature metadata validation (feature_matrix + metadata_consistency tests)
make test-architecture-guards  # static grep checks for invariant regressions
make check-architecture-ci    # full architecture guard CI reproduction
make check-no-default     # validate no-default-features workspace build
make check-feature-profiles # representative feature profile checks
make build         # release build
```

> **Note**: CI uses `cargo-tarpaulin` for coverage, while the Makefile uses `cargo llvm-cov`. Both measure the same thing but with different tools.

### New CLI Commands

```bash
eggsec daemon history [--json]                  # List persisted sessions
eggsec daemon show <session-id> [--json]        # Show persisted snapshot details
```

### Module Override Files

For specialized guidance on specific modules, see `AGENTS.override.md` in each module directory:

| Module | Override File |
|--------|---------------|
| `agent/` | `crates/eggsec/src/agent/AGENTS.override.md` |
| `ai/` | `crates/eggsec/src/ai/AGENTS.override.md` |
| `fuzzer/` | `crates/eggsec/src/fuzzer/AGENTS.override.md` |
| `scanner/` | `crates/eggsec/src/scanner/AGENTS.override.md` |
| `tui/` | `crates/eggsec-tui/src/AGENTS.override.md` |
| `waf/` | `crates/eggsec/src/waf/AGENTS.override.md` |
| `recon/` | `crates/eggsec/src/recon/AGENTS.override.md` |
| `tool/` | `crates/eggsec/src/tool/AGENTS.override.md` |
| `config/` | `crates/eggsec/src/config/AGENTS.override.md` |
| `output/` | `crates/eggsec/src/output/AGENTS.override.md` |
| `proxy/` | `crates/eggsec/src/proxy/AGENTS.override.md` |
| `stress/` | `crates/eggsec/src/stress/AGENTS.override.md` |
| `distributed/` | `crates/eggsec/src/distributed/AGENTS.override.md` |
| `packet/` | `crates/eggsec/src/packet/AGENTS.override.md` |
| `loadtest/` | `crates/eggsec/src/loadtest/AGENTS.override.md` |
| `mobile/` | `crates/eggsec/src/mobile/AGENTS.override.md` |
| `pipeline/` | `crates/eggsec/src/pipeline/AGENTS.override.md` |
| `nse/` | `crates/eggsec-nse/AGENTS.override.md` |
| `container/` | `crates/eggsec/src/container/AGENTS.override.md` |
| `db_pentest/` | `crates/eggsec/src/db_pentest/AGENTS.override.md` |
| `wireless/` | `crates/eggsec/src/wireless/AGENTS.override.md` |
| `evasion/` | `crates/eggsec/src/evasion/AGENTS.override.md` |
| `c2/` | `crates/eggsec/src/c2/AGENTS.override.md` |
| `postex/` | `crates/eggsec/src/postex/AGENTS.override.md` |

### Architecture Index

Canonical reference points when updating guidance or skills:

- `docs/ARCHITECTURE.md` - Workspace crate ownership, enforcement model, execution flows, side-effecting path inventory, transitional APIs
- `docs/ARCHITECTURE_INVARIANTS.md` - 30 normative invariants for enforcement, execution, and structure
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
- `architecture/mobile.md` - Mobile app static + dynamic analysis
- `architecture/auth.md` - Authentication testing module
- `architecture/c2.md` - C2 framework
- `architecture/audit.md` - Normalized audit events for enforcement decisions
- `architecture/domain_contract.md` - Domain module contract (Phase 3): static metadata descriptors for capability domains
- `architecture/report_envelope.md` - Normalized report/evidence envelope for cross-domain report unification
- `architecture/database_pentest.md` - Database pentest module (Postgres/MySQL/MSSQL/MongoDB/Redis)
- `architecture/defense_lab.md` - Defense lab mode and standalone surfaces
- `architecture/web_proxy.md` - Web proxy intercept and traffic analysis
- `architecture/wireless.md` - WiFi recon and wireless security testing
- `architecture/evasion.md` - Evasion technique detection
- `architecture/postex.md` - Post-exploitation simulation
- `architecture/hunt.md` - Vulnerability hunting workflows
- `architecture/browser.md` - Headless browser security testing
- `architecture/nse_integration.md` - NSE/Lua script integration
- `architecture/nse_report_display_contract.md` - NSE report display model for TUI/frontends
- `architecture/nse_capability_inventory.md` - NSE helper capability inventory, risk classification, and migration priority ranking
- `docs/FEATURE_MATRIX.md` - Feature inventory, classification, naming conventions, build profiles, and cross-reference
- `docs/EXTENSIBILITY.md` - Contributor extensibility guide: operations, domains, commands, tool exposure, TUI actions, reports, features, tests
- `docs/extending/operations.md` - Adding new OperationMetadata
- `docs/extending/domains.md` - Adding new domain crates and DomainDescriptors
- `docs/extending/commands.md` - Adding new CLI commands and the command registry
- `docs/extending/tool-exposure.md` - Exposing tools through MCP, REST, gRPC, and agent surfaces
- `docs/extending/tui-actions.md` - Adding TUI tabs and actions
- `docs/extending/report-evidence.md` - Report/evidence output using eggsec-output
- `docs/extending/features.md` - Adding Cargo features and the feature matrix
- `docs/extending/testing.md` - Test matrix and pre-handoff checklist
- `docs/extending/templates.md` - Short templates for each extension type

### Feature Flags

**Feature-gated modules (require system deps or root for real scans):**

| Feature | Module | System Dep | Notes |
|---------|--------|------------|-------|
| `wireless` | WiFi recon | `wireless-tools` (iwlist) | Passive scans; root/CAP_NET_ADMIN for real; TUI tab present |
| `wireless-advanced` | WiFi active | (needs wireless) | deauth/disassoc; `--allow-active-wireless`; policy gated `Intrusive` |
| `mobile` | APK/IPA static | none | Pure-Rust parsers; local file only; domain crate: `eggsec-mobile-lab` |
| `mobile-dynamic` | Mobile dynamic | ADB + device | Phase 1-4a complete; `--allow-dynamic-mobile` for real; policy gated `Intrusive`; domain crate: `eggsec-mobile-lab` |
| `db-pentest` | DB security | none (drivers) | Postgres/MySQL/MSSQL/MongoDB/Redis; `--allow-db-pentest` for real |
| `web-proxy` | MITM proxy | none | `--allow-web-proxy` + policy for real interception |
| `evasion` | Evasion detection | none | `--allow-evasion-testing` for real |
| `postex` | Post-ex simulation | none | `--allow-postex` + scope for real |
| `c2` | C2 simulation | none | `--allow-c2`; depends on postex + evasion |
| `stress-testing` | Flood testing | none | Raw sockets, IP spoofing |
| `packet-inspection` | Packet capture | `libpcap-dev` | |
| `nse` | NSE scripts | `libssl-dev` | |
| `http-api` | Daemon HTTP transport | none | `axum`-based loopback HTTP server; `McpStrict` profile; loopback-only default bind |

**Marker-only features (no deps, just build gating):**

`tool-api`, `insecure-tls`, `rest-api` (strict enforcement via `EnforcementContext` + `McpStrict` by default; includes `POST /api/v1/tools/{tool_id}/preflight` endpoint), `grpc-api`, `ws-api`, `nse-ssh2`, `nse-sandbox`, `ai-integration`, `websocket`, `headless-browser`, `database`, `container`, `sbom`, `advanced-hunting`, `compliance`, `external-integrations`, `finding-workflow`, `vuln-management`, `cloud`, `git-secrets`, `web-proxy-mcp`, `c2-mcp`, `transparent-proxy`, `dynamic-plugins`, `pdf`, `api-schema`, `db-pentest-mongodb`, `db-pentest-redis`, `db-pentest-mcp`

**CLI-level features** (on `eggsec-cli` crate):

| Feature | Module | Notes |
|---------|--------|-------|
| `tui` | Terminal UI | Default; `dep:eggsec-tui` |
| `daemon-client` | Daemon client | `dep:eggsec-daemon` + `dep:tokio-util` + `eggsec/daemon-client` |
| `headless` | Marker | No TUI, no daemon client; CI/scripting builds |

**Aggregate features:**

`full` — developer/lab aggregate that enables all non-default features including advanced/lab-only capabilities (`wireless-advanced`, `evasion`, `postex`, `c2`, `mobile-dynamic`). Not a conservative default or production profile.

> **Note**: The `eggsec-output::envelope` module (normalized report/evidence types) is always available — no feature gate required.

> **Feature Matrix**: See `docs/FEATURE_MATRIX.md` for the complete feature inventory with categories, naming conventions, build profiles, and metadata cross-references.

### Key Types

- `Severity` - Canonical definition in `eggsec-core::types`, re-exported by `types.rs`. Don't recreate.
- `SensitiveString` - Zeroized credential wrapper (defined in `eggsec-core::types`)
- `EggsecConfig` - Main configuration (`config::load_config()`)
- `EnforcementContext` - Central policy evaluator (`config/policy_decision.rs`); constructors: `cli`, `mcp_strict`, `agent_strict`, `ci_strict`
- `LoadedScope` - Scope with provenance (`DefaultEmpty`, `ConfigFile`, `CliScopeFile`, `GeneratedPreset`) in `config/scope.rs`
- `ExecutionProfile` - Trust boundary enum: `ManualPermissive`, `ManualGuarded`, `McpStrict`, `AgentStrict`, `CiStrict`
- `ExecutionSurface` - Caller-origin enum that derives `ExecutionProfile`; single source of truth for surface-to-profile mapping
- `OperationMetadata` - Canonical operation metadata, single source of truth for `OperationDescriptor` generation across REST, MCP, TUI, and agent surfaces. Defined in `config::policy`, re-exported from `config` and `tool::metadata`. Static registry with 32 operations + 33 aliases.
- `TargetPolicyKind` - Target policy requirement enum for operation metadata (`NoTarget`, `OptionalTarget`, `TargetRequired`, `ExplicitScopeRequired`, `PrivateOrLocalRequired`).
- `ConfirmationClass` - Kebab-case strings for policy confirmations; use `as_str()` for stable IDs
- `TabError` - Structured error type with `is_recoverable()` in `eggsec-tui`
- `TuiEnforcementState` - TUI-local enforcement posture model in `eggsec-tui::app::enforcement`
- `TuiPreflightResult` - Advisory preflight evaluation result for display in status bar
- `EnforcementFacade` - Extracted enforcement evaluation/approval logic in `eggsec-tui::app::enforcement_facade`; owns `TuiEnforcementState` + cached approval token
- `TuiActionSpec` - Metadata-backed TUI action descriptor pointing to canonical `OperationMetadata` in `eggsec-tui::app::action_spec`
- `PreflightResult` - Shared preflight evaluation result across CLI/TUI/REST/MCP/agent (`config::policy_decision`)
- `PreflightOutcomeKind` - Simplified outcome enum for preflight results (`config::policy_decision`)
- `EnforcementAuditEvent` - Normalized audit record for enforcement decisions (`audit.rs`)
- `AuditOutcome` - Simplified audit outcome enum: Allow/Warn/Confirmed/Deny/ConfirmationRequired
- `AuditSummary` - Audit event summary with outcome/surface counts for report generation (`eggsec-output::audit_summary`)
- `ScopeAudit` - Scope provenance summary for audit events
- `PayloadType` - Enum of 40 payload categories; lives in `fuzzer/payloads/mod.rs`, NOT `types.rs`
- `McpProfile` / `McpProfilePolicy` - MCP agent profiles and per-profile tool visibility in `tool/protocol/mcp/`
- `ApprovedOperation` - Proof-of-enforcement token with private fields; produced exclusively by `EnforcementContext::approve()` or `approve_manual()`. Read-only accessors: `descriptor()`, `decision()`, `surface()`, `profile()`, `audit_event_id()`.
- `EnforcementError` - Structured error from `approve()`/`approve_manual()`: `Denied`, `ConfirmationRequired`, `ManualOverrideUnavailable`.
- `EnforcedDispatcher` - Wrapper around `ToolDispatcher` requiring `ApprovedOperation` before dispatch via `dispatch_checked()`.
- `CommandPermission` - Per-command authorization level enum for daemon RBAC (`Public`, `DeclaredClient`, `Observer`, `Controller`, `Owner`, `Approver`). Single source of truth in `eggsec-daemon/src/client_registry.rs`.
- `DaemonStore` - Trait for daemon persistence (trait + SQLite implementation). Defined in `eggsec-daemon::store`.
- `SqliteStore` - SQLite-backed implementation of `DaemonStore` with WAL mode.
- `NoopStore` - Test stub implementing `DaemonStore`.
- `TransportKind` - Daemon transport type enum (`UnixSocket`, `LoopbackHttp`). Defined in `eggsec-daemon::protocol`.
- `DaemonRequestContext` - Per-request context carrying `client_id`, `session_id`, `request_id`, and `transport` kind. Defined in `eggsec-daemon::protocol`.
- `TransportCapability` - Declares a daemon transport's `kind` and `default_bind` address. Defined in `eggsec-daemon::protocol`.
- `DaemonCapabilities` - Capabilities descriptor carrying `transports: Vec<TransportCapability>`. Returned in `ServerMessage::Capabilities` (`crates/eggsec-daemon/src/protocol.rs`). Defined in `eggsec-daemon::protocol`.
- `DAEMON_PROTOCOL_VERSION` - Protocol version constant (`u32 = 1`) for daemon IPC compatibility negotiation. Defined in `eggsec-daemon::protocol`. Clients should check this value before sending commands.
- `HttpConfig` - HTTP transport configuration (`bind`, `mcp_strict_by_default`, `cors_origin`). Defined in `eggsec-daemon::http`. Default: loopback `127.0.0.1:0`.
- `DomainDescriptor` - Static metadata descriptor for a capability domain (`domain/mod.rs`); declares operations, CLI/TUI/MCP/report integrations, feature gates, dry-run/evidence support. Pilot: `db-pentest`.
- `DomainCategory` - Classification enum for domains: `StandardAssessment`, `DefenseLab`, `HazardousLab`, `FrontendAdapter`, `OutputAdapter`.
- `CapabilityMatrixRow` - Generated row from `DomainDescriptor` + `OperationMetadata` for the capability matrix (`domain/mod.rs`). Produced by `generate_capability_matrix()`. Fields: `tool_integration: bool`, `mcp_exposed_by_default: bool`, `required_mcp_feature: Option<&'static str>`, `rest_exposable: bool`, `agent_exposable: bool`.
- `Capability` - Enum of domain capability categories used in `DomainDescriptor` operations (e.g. `MobileDynamicAnalysis`). Defined in `config::policy`.
- `DryRunSupport` - Enum for dry-run support level: `AlwaysAvailable`, `FeatureGated(&str)`, `NotSupported`.
- `EvidenceSupport` - Enum for evidence bundle support level: `AlwaysAvailable`, `FeatureGated(&str)`, `NotSupported`.
- `BaselineSupport` - Enum for baseline/regression support level: `AlwaysAvailable`, `FeatureGated(&str)`, `NotSupported`.
- `CommandRegistration` - Static metadata for registered commands (`commands/registry.rs`); declares command ID, operation ID, category, feature gate, visibility flags (`cli_visible`, `tui_visible`, `programmatic_visible`, `cli_interactive_only`), `registry_backed`, and `dispatch_mode`. The `cli_interactive_only` flag marks CLI-helper/config/report-style commands that should not be TUI-visible or programmatically exposed — it does **not** apply to all human-interactive surfaces. Registry is metadata and routing, not authorization.
- `CommandCategory` - Classification enum for command registry entries: `SideEffectingNetwork`, `LocalFileDomain`, `PassiveAnalytical`, `ConfigOutputHelper`, `FrontendServer`, `LegacySpecial`.
- `CommandDispatchMode` - Dispatch classification enum: `RegistryBacked`, `LegacyWrapped`, `CatalogOnly`, `ServerLifecycle`, `HelperOnly`. Describes how a command's execution path relates to the registry.
- `ToolRegistration` - Canonical tool registration metadata, single source of truth for tool listing across MCP, REST, gRPC, and agent surfaces. Defined in `tool::registration`. Carries `mcp_metadata_exposable` (OperationMetadata-level) and `mcp_default_visible` (conservative default listing). The MCP surface uses Model A profile-expanded visibility: `mcp_tool_registrations("ops-agent")` returns all `mcp_metadata_exposable` tools (not the conservative default). The conservative subset is `mcp_tool_registrations_default_visible()`. See `docs/TOOL_REGISTRATION.md`.
- `ToolRegistrationSource` - Origin enum for tool registrations: `Base`, `FeatureGated(&str)`, `Domain(&str)`.
- `ReportEnvelope` - Normalized report container (`eggsec-output::envelope`); preserves report identity, findings, evidence, policy, and baseline summaries
- `FindingRecord` - Normalized finding record within a ReportEnvelope; includes evidence items, references, and category
- `EvidenceItem` - Single evidence entry with kind, source, redaction state, and optional data reference
- `EvidenceManifest` - Manifest of all evidence items in a report; tracks total/redacted counts and provenance
- `BaselineSummary` - Standardized baseline comparison result; added/resolved/unchanged counts with severity deltas
- `ToolMetadata` - Tool/version metadata for report envelopes
- `EvidenceKind` - Category of evidence data (HttpRequest, DatabaseFinding, MobileManifest, TrafficCapture, etc.)
- `EvidenceSource` - Provenance of evidence (tool, module, run_id)
- `RedactionState` - Sensitivity classification: None, FullyRedacted, PartiallyRedacted, Summarized
- `RedactionPolicy` - Manifest-level redaction strategy: None, RedactAll, RedactSensitive, SummarizeAll, DomainSpecific
- `NseExecutionProfileKind` - NSE execution profile enum: `ManualPermissive`, `ManualStrict`, `AgentSafe`, `CiSafe`, `CompatibilityLab`. Defined in `eggsec-nse::profile`. Encodes trust boundary assumption for NSE script execution.
- `ResolvedNseExecutionProfile` - Resolved NSE profile with all policies: `kind`, `sandbox`, `limits`, `script_policy`, `module_policy`, `network_policy`, `audit_label`, `warnings`. Constructors: `manual_permissive`, `manual_strict`, `agent_safe`, `ci_safe`, `compatibility_lab`.
- `NseScriptPolicy` - NSE script access rules: `allow_builtin_scripts`, `allow_script_files`, `allowed_script_roots`, `allow_conventional_nmap_paths`, `max_script_bytes`.
- `NseModulePolicy` - NSE module access rules: `allow_builtin_modules`, `allow_filesystem_modules`, `allowed_module_roots`, `max_module_bytes`.
- `NseNetworkPolicy` - NSE network access policy: `AllowAllManual`, `DenyAll`, `AllowCidrs`, `AllowResolvedTargetSet`.
- `NseScriptSource` - Explicit script source kind for resolver (Builtin, TrustedRegistry, File, InlineManual) in `eggsec-nse::resolver`
- `NseModuleName` - Validated module name type with strict grammar enforcement in `eggsec-nse::resolver`
- `NseLibraryDescriptor` - Declarative descriptor for NSE library modules (`resolver::registry`); fields: `name`, `category`, `sandbox_side_effects`, `optional_deps`, `fallback_behavior`, `notes`
- `NseLibraryCategory` - Functional category enum: `Core`, `Protocol`, `Utility`, `Exploit`, `Auth` (`resolver::registry`)
- `NseSandboxSideEffect` - Sandbox side effect enum: `None`, `FileSystemRead`, `FileSystemWrite`, `NetworkAccess`, `ProcessExecution`, `EnvAccess` (`resolver::registry`)
- `NseFallbackBehavior` - Fallback behavior enum: `HardFail`, `GracefulDegrade`, `Skip` (`resolver::registry`)
- `ResolvedNseScript` - Resolved script with content and metadata in `eggsec-nse::resolver`
- `ResolvedNseModule` - Resolved module with content and metadata in `eggsec-nse::resolver`
- `NseLoadError` - Structured load error type for script/module resolution in `eggsec-nse::resolver`
- `NseLoadDiagnostic` - Load behavior diagnostic for visibility in `eggsec-nse::resolver`
- `ScriptResolver` - Hardened script/module resolver enforcing policies, path containment, and size limits in `eggsec-nse::resolver`
- `ScopeInput` - Scope input for network policy derivation in NSE profiles: `target_ip`, `resolved_ips`, `scope_cidrs`.
- `NseRunReport` - Structured run output model for NSE execution (`eggsec-nse::report`); run output truthfulness is defined by this type, and `NseRunReport.libraries` records per-run required/attempted library usage rather than a capability snapshot.
- `NseRuleEvaluationReport` - Rule evaluation metadata (`eggsec-nse::report`); rule behavior is defined by this type (kind, status, fidelity, approximations, inputs).
- `nse_report_view::render_report()` - View model converting `NseRunReport` to styled TUI lines (`eggsec-tui::tabs::nse_report_view`)

> **NSE Milestone 1 (loader/profile) is closed.** The canonical implementation, tests, and policy contract are listed in the [Milestone 1 Closure Index](./architecture/nse_integration.md#milestone-1-closure-index). Future work should not reopen loader/profile policy unless a regression is found.
> **NSE Milestone 2 (registry/report/corpus) is closed.** Library compatibility is defined by `NseLibraryRegistry` metadata (43 descriptors). `NseRunReport.libraries` records per-run required/attempted library usage, not a capability snapshot, and the later truthfulness follow-up refined that reporting without reopening Milestone 2. Rule behavior is defined by `NseRuleEvaluationReport`. Rule evaluation produces structured reports via `evaluate_rule()`. Error paths emit full reports by `build_failure_report()`. Run output truthfulness is defined by `NseRunReport`. The compatibility corpus is representative and local-only. See the [Milestone 2 Closure Note](./architecture/nse_integration.md#milestone-2-closure-note).
> **NSE Milestone 3 (capability wrappers) Phase 01 complete.** A complete capability inventory and risk classification exists at `architecture/nse_capability_inventory.md`. The inventory classifies all side-effecting NSE helper operations by capability class, blocking risk, profile policy, accounting needs, cancellation requirements, and report events. Key findings: 4 libraries sandboxed (socket, io, os, lfs), all protocol libraries (~100+) bypass sandbox, `nmap.socket_*()` bypasses socket sandbox, `stdnse.sleep()` blocks without cancellation checks. Migration priority: process execution → filesystem write → filesystem read → network TCP/UDP → DNS → compression → crypto/TLS → time/randomness → pure CPU.
> **NSE Milestone 3 Phase 02 complete.** `NseCapabilityContext` and decision engine (`capabilities.rs`) provide centralized policy enforcement for all side-effecting helpers. `NseCapabilityKind` covers 11 operation classes. Profile-specific checks: ManualPermissive allows all with warnings, ManualStrict enforces path/network policy, AgentSafe denies process exec + FS write, CiSafe denies all side effects. `NseCapabilityEvent` integration into `NseRunReport.capability_events` — denied operations affect compatibility status. Pilot wrappers in `wrappers.rs` demonstrate the pattern. `ExecutorCore` stores the capability context, constructed from `with_policy()` defaults or `with_profile()` overrides. Architecture guards detect direct high-risk ops in NSE libraries (informational, will tighten as wrappers migrate).
> **NSE Milestone 3 Phase 03 complete.** Filesystem and process wrappers are now fully migrated through `NseCapabilityContext`. Libraries `io.rs`, `lfs.rs`, `os.rs`, and `nmap.rs` route all side-effecting operations through capability checks. Executing wrappers (`nse_fs_read_to_string`, `nse_fs_write`, `nse_fs_remove_file`, `nse_fs_create_dir`, `nse_fs_rename`, `nse_process_exec`, etc.) combine capability checking with the actual operation. `AgentSafe` and `CiSafe` deny process execution and filesystem writes by default. `ManualPermissive` allows with warnings. Architecture guard Check 33 now fails for direct `std::process::Command` in NSE libraries. Library registration functions accept `&NseCapabilityContext`. Network TCP/UDP, compression, and crypto remain pending.

> **NSE Milestone 3 Phase 04 complete.** Network TCP/UDP and DNS wrappers migrated through `NseCapabilityContext`. Executing wrappers added: `nse_network_tcp_connect`, `nse_network_tcp_send`, `nse_network_tcp_receive`, `nse_network_udp_send`, `nse_network_udp_receive`, `nse_dns_lookup`, plus check-only `check_network_udp`. Libraries `socket.rs`, `comm.rs`, and `dns.rs` now accept `&NseCapabilityContext` in their registration functions and route network/DNS operations through capability wrappers before performing the actual operations. Architecture guard Check 33c (informational) detects direct network calls in unmigrated libraries. All 318 tests pass. Compression, crypto/TLS, and protocol-specific libraries (smb, ssh, ftp, http, etc.) remain unmigrated.
> **NSE Milestone 3 Phase 05 complete.** Time, randomness, environment, crypto, and compression helpers are now routed through `NseCapabilityContext`. Executing wrappers added: `nse_time_now`, `nse_random_bytes`, `nse_env_var`, `nse_compress`, `nse_decompress`. Check-only wrappers added: `check_randomness`, `check_environment`, `check_crypto`, `check_compression`. Profile-specific policies: AgentSafe denies environment access, warns on randomness; CiSafe denies environment and randomness, warns on time nondeterminism. Compression enforces 64 MiB input and 256 MiB output limits. Libraries migrated: `datetime.rs`, `rand.rs`, `openssl.rs`, `tls.rs`, `sslcert.rs`, `zlib.rs` now accept `&NseCapabilityContext`. All 200+ tests pass.

> **NSE Milestone 3 (capability wrappers) is closed.** All side-effecting helper classes (filesystem, process, network, DNS, time, randomness, environment, compression, crypto) are routed through `NseCapabilityContext`. Protocol-specific libraries beyond network I/O remain deferred. Capability events are visible in `NseRunReport.capability_events`. Architecture guards (Check 33/33b/33c) prevent new direct bypasses. See the [Milestone 3 Closure Note](./architecture/nse_integration.md#milestone-3-closure-note).
>
> **Milestone 3 Corrective Pass (profile propagation).** `run_cli_with_profile()` now uses `NseExecutor::with_profile(&resolved_profile)` instead of `NseExecutor::with_policy(...)`, which previously hardcoded `ManualPermissive` in the capability context. New constructors: `NseExecutor::with_full_policy(...)`, `AsyncNseExecutor::with_full_policy(...)`, `ExecutorCore::with_full_policy(...)` for explicit policy control. `NseExecutor::capability_context()` accessor added. AgentSafe filesystem reads are now scoped-only (path must be under sandbox `allowed_dir` or explicit root). New architecture guards: Check 35 (run_cli_with_profile uses with_profile), Check 36 (automated surfaces must not use with_policy), Check 37 (ExecutorCore::with_policy callers info). New integration tests in `crates/eggsec-nse/tests/profile_propagation_tests.rs`. See [Milestone 3 Corrective Pass](./architecture/nse_integration.md#milestone-3-corrective-pass-profile-propagation) and `plans/nse-milestone-3-corrective-pass.md`.
> **Milestone 3 Closure Verification (2026-07-06).** Final verification pass: 369 tests pass (1 ignored), architecture guards all pass (37 checks), fmt/clippy clean. New end-to-end profile/report tests in `crates/eggsec-nse/tests/profile_report_tests.rs` verify the profile→context→event→report pipeline for AgentSafe (process exec denial, unscoped/scoped FS read), CiSafe (network/DNS denial), and ManualPermissive (process exec warning). See [Milestone 3 Final Verification](./architecture/nse_integration.md#milestone-3-final-verification).
> **NSE Milestone 4 (compatibility corpus, fidelity, runtime harness) is closed.** The compatibility corpus expanded to 39 fixtures across 9 categories (discovery/version/default/protocol/auth/partial/unsupported/regression/upstream). A dedicated runtime test binary (`runtime_corpus_tests.rs`) drives every fixture through `NseExecutor::with_profile()` with synthetic host/port context and asserts manifest expectations against observed rule/library/capability reports. Smoke tests (`runtime_smoke_tests.rs`) verify the full pipeline (profile → context → execution → report → `ReportEnvelope` bridge). The static harness (`compatibility_corpus_tests.rs` `mod corpus_manifest`) remains resolver-only by design; new architecture guards Check 42/43/44 enforce that separation. Architecture guards now cover 46 checks (all pass). See [Milestone 4 Closure Verification](./architecture/nse_integration.md#milestone-4-closure-verification).
> **NSE Milestone 5 Phase 02 (strict runtime assertions) is closed.** Runtime corpus assertions upgraded from lenient (log-only) to strict (hard assert). `expected_libraries` now hard-asserts by default; `allow_missing_runtime_libraries` downgrades to soft. `expected_rules` hard-asserts when fixture declares ports (portrule can fire); skips when no ports. `expected_capability_events` with `required=true` hard-asserts denial (no resolver-block substitute). New `expected_evidence_kinds`/`optional_evidence_kinds` assertions. Architecture guards Check 45 (no self-referential expected values) and Check 46 (no trivially satisfiable assertions). 17 runtime corpus tests, all 46 architecture guards pass. See [Milestone 5 Phase 02](./architecture/nse_integration.md#milestone-5-phase-02-strict-runtime-assertions-2026-07-06).
> **NSE Milestone 5 Phase 03 (local protocol fixtures) is closed.** Local TCP/HTTP/UDP fixture harness (`local_fixtures.rs`), 5 new `.nse` scripts, 16 runtime tests (`local_protocol_tests.rs`). Manifest `local_service` metadata + runtime harness skip for all 7 iteration sites. Architecture guard Check 47. 452 NSE tests pass (1 ignored), all 47 architecture guards pass. See [Milestone 5 Phase 03](./architecture/nse_integration.md#milestone-5-phase-03-local-protocol-fixtures-2026-07-06).
> **NSE Milestone 5 Phase 04 (deferred library migration) is closed.** `unpwdb.rs` migrated from Deferred to Wrapped (FS reads through `nse_fs_read_to_string`). `http.rs` migrated to Wrapped (all network operations gated via `check_network_tcp()`; denied requests never reach reqwest). `ssl` registry entry corrected to Wrapped (stale since Milestone 3 Phase 05). Registry tests updated. 182 lib tests, 43 corpus tests, 47 architecture guards pass. See [Milestone 5 Phase 04](./architecture/nse_integration.md#milestone-5-phase-04-deferred-library-migration-2026-07-06).
> **NSE Milestone 5 Phase 05 (report UX and performance) is closed.** CLI report formatting extracted to testable `format.rs` with 29 snapshot tests. TUI/frontend data contract documented in `architecture/nse_report_display_contract.md`. Runtime corpus performance baseline with timing instrumentation and manifest caching (`LazyLock`). ReportEnvelope bridge hardened with 11 evidence tests and 4 envelope shape tests. 18 runtime corpus tests, 19 evidence tests, 4 bridge tests, 29 format tests pass. See [Milestone 5 Phase 05](./architecture/nse_integration.md#milestone-5-phase-05-report-ux-and-runtime-performance-2026-07-06).
> **NSE Milestone 5 Phase 06 (release closure) is closed.** 16-command verification matrix passes: 493 eggsec-nse tests (1 ignored), 174 eggsec nse_tests, 352 feature/enforcement matrix tests, 47 architecture guards. Bug fixes: `test_nse_prerule_postrule` (boolean return + `stdnse.register_prerule`), `local_http_get_agent_safe_documentation` (assertion update for AgentSafe output). Documentation updates: `architecture/nse_integration.md`, `docs/NSE_COMPATIBILITY.md`, `AGENTS.override.md`, skills. Remaining deferred: protocol library wrappers, `stdnse.sleep()` cancellation — candidates for Milestone 6. See [Milestone 5 Final Verification](./architecture/nse_integration.md#milestone-5-final-verification-2026-07-06).
> **NSE Milestone 6 Phase 01 (HTTP capability bypass and runtime strictness) is closed.** HTTP library `http.rs` promoted from `PartiallyWrapped` to `Wrapped` — all network operations gated via `check_network_tcp()`; denied requests never reach reqwest. Local HTTP fixtures gained atomic hit counters proving denied requests don't reach server. AgentSafe HTTP tests upgraded from permissive to strict denial assertions (server hits == 0, capability events contain network_tcp denial). CiSafe HTTP test added. Runtime library assertions tightened from lenient (`is_empty() || found`) to hard failures. Architecture guards 48-50 added (HTTP check_network_tcp count, no permissive AgentSafe text, no lenient library assertions). 494 NSE tests pass, 50 architecture guards pass. Registry, NSE_COMPATIBILITY.md, nse_integration.md, skills updated. See [Milestone 6 Phase 01](./architecture/nse_integration.md).
> **NSE Milestone 6 Phase 02 (HTTP method coverage and guard hardening) is closed.** All HTTP methods (GET/POST/PUT/DELETE/HEAD/OPTIONS/request) now have ManualPermissive success tests and AgentSafe/CiSafe zero-hit denial tests. `maybe_denied_response()` helper centralizes HTTP policy checks. HttpServer tracks method/path. Architecture guards 48b-48d added (HTTP method operation strings, strict zero-hit assertions, no permissive denial language). All tests pass, architecture guards pass. See [Milestone 6 Phase 02](./architecture/nse_integration.md).
> **NSE Milestone 6 is closed (2026-07-06).** HTTP method coverage complete — all HTTP methods (GET/POST/PUT/DELETE/HEAD/OPTIONS/request) have local fixture scripts with zero-hit denial tests for AgentSafe and CiSafe. HTTP library (`reqwest`) fully migrated to Wrapped status. Architecture guards 48-50 enforce strict assertions. 511 tests pass, 52 architecture guards pass. Remaining deferred: protocol library wrappers, `stdnse.sleep()` cancellation — candidates for Milestone 7. See [Milestone 6 Closure](./architecture/nse_integration.md#milestone-6-closure-verification-2026-07-06).
> **NSE Milestone 6 Phase 03 (TLS/sslcert local fixtures) is closed (2026-07-06).** `TlsEchoServer` in `local_fixtures.rs` generates self-signed X.509 certs at startup via openssl, creates `native_tls::Identity` via PKCS12, binds `127.0.0.1:0` (ephemeral), accepts TLS connections with per-connection thread spawning. 5 sslcert `.nse` fixture scripts exercise `get_certificate`, `parse_cert`, `get_subject`, `get_chain_certs`, and `is_valid`. All 5 manifest entries with `local_service.type = "tls_echo"`. 5 NSE integration tests + 1 unit test (40 total in local_protocol_tests). Architecture guard Check 53 added. `sslcert.rs` fixed to return actual PEM-encoded certificates. 518 eggsec-nse tests pass, 53 architecture guards pass. See [Milestone 6 Phase 03](./architecture/nse_integration.md#milestone-6-phase-03).
> **NSE Expansion Phase 06 (sslcert guard symmetry) is closed (2026-07-07).** CiSafe `get_chain_certs` zero-hit denial test added. ManualPermissive TLS hit assertions added to 4 success tests. Per-connect architecture guard (Check 56) replaces aggregate count check with awk-based proximity verification. 522 eggsec-nse tests pass, 56 architecture guards pass. See [Expansion Phase 06](./architecture/nse_integration.md#nse-expansion-phase-06-2026-07-07-sslcert-guard-symmetry).
- `SessionId` - Opaque session identifier (`eggsec-runtime::ids`)
- `TaskId` - Opaque task identifier (`eggsec-runtime::ids`)
- `ClientId` - Opaque client identifier (`eggsec-runtime::ids`)
- `RunRequest` - Frontend-neutral task request (`eggsec-runtime::request`)
- `TaskKind` - Frontend-neutral task kind enum (`eggsec-runtime::request`)
- `RuntimeSurface` - Frontend-neutral execution surface mirror (`eggsec-runtime::request`)
- `RuntimeEvent` - Runtime lifecycle event (`eggsec-runtime::event`)
- `TaskStatus` - Task lifecycle status enum (`eggsec-runtime::event`)
- `TaskProgress` - Task progress info (`eggsec-runtime::event`)
- `TaskOutcome` - Generic task outcome (`eggsec-runtime::event`)
- `SessionSnapshot` - Runtime session state snapshot (`eggsec-runtime::session`)
- `RuntimeCapabilities` - Runtime capability descriptor (`eggsec-runtime::capabilities`)
- `Runtime` - Async task lifecycle manager (`eggsec-runtime::runtime`); owns task submit/cancel/snapshot/subscribe; single-active-task policy (new task cancels existing)
- `RuntimeConfig` - Runtime configuration (`eggsec-runtime::runtime`); `default_task_timeout`, `max_active_tasks_per_session`, `event_channel_capacity`
- `SessionOptions` - Session creation options (`eggsec-runtime::runtime`)
- `RuntimeTaskExecutor` - Trait allowing frontends to supply task execution logic (`eggsec-runtime::runtime`)
- `TaskDispatcher` - Frontend-neutral task dispatch trait (`eggsec-runtime::dispatcher`); maps `RunRequest` to `TaskOutcome`; dependency-free, implementations live in frontend crates
- `TuiTaskDispatcher` - TUI implementation of `TaskDispatcher` (`eggsec-tui::app::task_dispatcher`); holds `Arc<ArcSwap<TuiDispatcherContext>>` and calls `eggsec::dispatch::dispatch_inner()` directly; returns `TaskOutcome::Result(TaskResultEnvelope)` for lifecycle tracking while typed results flow through typed `mpsc` channels
- `TaskResultEnvelope` - Protocol-neutral result wrapper (`eggsec-runtime::event`); carries `kind`, `summary`, JSON `payload`, and `artifacts: Vec<ArtifactRef>`. Produced by `TuiTaskDispatcher::dispatch()` via `task_result_to_envelope()`.
- `ArtifactRef` - Artifact reference (`eggsec-runtime::event`); carries `id`, `kind`, `path`, `mime_type`, `summary` for output artifacts produced by a task.
- `TuiExecutor` - TUI implementation of `RuntimeTaskExecutor` (`eggsec-tui::app::task_runtime`); wraps `TuiTaskDispatcher`, loads per-task channels via `ArcSwap<TuiDispatcherContext>`
- `TuiDispatcherContext` - Per-task channel context (`eggsec-tui::app::task_runtime`); holds `mpsc::Sender<(u64,u64)>` and `mpsc::Sender<TaskResult>` for a single task submission
- `RuntimeEventSink` - Event sink for runtime lifecycle events (`eggsec-runtime::runtime`)
- `RuntimeEventReceiver` - Event receiver for runtime lifecycle events (`eggsec-runtime::runtime`)
- `SessionSummaryView` - Frontend-neutral session summary DTO (`eggsec-ui-model::session_view`); `From<&SessionSummary>` conversion
- `SessionView` - Full session view with task lists (`eggsec-ui-model::session_view`); `From<&SessionSnapshot>` conversion
- `TaskView` - Task view with status/kind labels (`eggsec-ui-model::task_view`); `From<&TaskSnapshot>` conversion
- `TaskProgressView` - Progress with percentage (`eggsec-ui-model::task_view`); `From<&TaskProgress>` conversion
- `ResultEnvelopeView` - Normalized result envelope with renderer lookup (`eggsec-ui-model::result_view`); `From<&TaskResultEnvelope>` conversion
- `OutcomeView` - Task outcome view (`eggsec-ui-model::result_view`); `From<&TaskOutcome>` conversion
- `ArtifactView` - Artifact reference view (`eggsec-ui-model::artifact_view`); `From<&ArtifactRef>` conversion
- `EventView` - Runtime event view (`eggsec-ui-model::event_view`); `From<&RuntimeEvent>` conversion for all 11 variants
- `DashboardSummaryView` - Aggregated session statistics (`eggsec-ui-model::dashboard_view`); `from_summaries(&[SessionSummary])` constructor
- `ClientRoleView` - Permission role view (`eggsec-ui-model::permission_view`); static constructors for owner/controller/observer/approver
- `PolicyPromptView` - Policy prompt view (`eggsec-ui-model::policy_view`); `From<&PolicyPrompt>` conversion
- `ResultRendererDescriptor` - Metadata for rendering a result kind (`eggsec-ui-model::renderer_registry`); fields: `kind`, `title`, `summary_fields`, `artifact_kinds`, `supports_rich_tui`, `supports_json_detail`
- `renderer_for_kind()` - Lookup function for `ResultRendererDescriptor` by kind string (`eggsec-ui-model::renderer_registry`)

### Important Patterns

- **Severity Enum**: Single canonical definition in `eggsec-core::types`. Re-export, don't recreate.
- **Tool Abstraction**: `tool/traits.rs` has `SecurityTool` trait, `tool/registry.rs` has `ToolRegistry`
- **Regex Caching**: Use `lru = "0.18"` with cache size 100 (NonZeroUsize)
- **Circuit Breaker**: `utils/circuit_breaker.rs` - `CircuitBreaker` with configurable thresholds
- **Truncation**: `utils/formatting.rs` - `strip_controls` (recommended) and `preserve_all`
- **Visual Regression Testing**: Use `TestBackend` + `Terminal::new()` with `terminal.backend().buffer()` to verify rendered content
- **AI Cache Keys**: Always use `CacheKeyBuilder` for cache keys in AI module to avoid collisions
- **Hash Collections**: Use `rustc_hash::FxHashMap` and `rustc_hash::FxHashSet` instead of std collections for performance
- **Error Handling**: Avoid `unwrap_or_default()` on async operations; use explicit match with tracing instead
- **ExecutionSurface**: Introduces caller-origin semantics; `ExecutionProfile` describes enforcement behavior, `ExecutionSurface` describes where it comes from. Use `EnforcementContext::for_surface()` for centralized construction.
- **Operation Metadata**: `OperationMetadata` in `config::policy` is the single source of truth for `OperationDescriptor` generation. All surfaces (REST, MCP, TUI, agent) use `metadata_for_tool_id()` or `operation_metadata()` to look up canonical operation definitions. Alias mapping resolves alternate tool IDs (e.g., "scan" → "scan-ports", "fuzz" → "fuzz") to canonical metadata. Descriptors are generated via `metadata.descriptor_for_target()`. Surface-specific overrides (e.g., REST always sets `requires_explicit_scope = true`, MCP uses profile policy) are applied after metadata lookup.
- **Domain Contract**: `DomainDescriptor` in `domain/mod.rs` groups operations under a domain umbrella with CLI/TUI/tool/report integrations. `generate_capability_matrix()` produces `CapabilityMatrixRow` entries from domain metadata. `docs/CAPABILITY_MATRIX.md` is the canonical human-readable matrix. Tests in `tests/metadata_consistency.rs` validate cross-references between `DomainDescriptor` and `OperationMetadata`.
- **Shared Policy Evaluator**: Use `EnforcementContext::evaluate()` (central) in `config/policy_decision.rs` instead of building policy checks inline
- **Shared Preflight**: `preflight_operation()` in `config::policy_decision` is the single entry point for all surfaces. CLI, TUI, REST, MCP, and agent all use it. It evaluates the same `EnforcementContext::evaluate()` path as dispatch without executing the tool. CLI has a standalone `preflight` command. REST has `POST /api/v1/tools/{tool_id}/preflight`. MCP has `eggsec_preflight` tool. Agent logs preflight results before dispatch.
- **Normalized Audit Events**: `audit.rs` provides `EnforcementAuditEvent` for consistent audit records across all surfaces (CLI, TUI, REST, MCP, Agent, gRPC). `audit_event_from_enforcement_outcome()` builds events from enforcement decisions. `emit_audit_event()` logs at appropriate tracing levels (info for allow/warn/confirmed, warn for deny/confirmation-required). Manual confirmations record class and reason. Automated surfaces never record accepted manual overrides. Scope provenance included.
- **TUI Enforcement Posture**: TUI uses `EnforcementFacade` (wrapping `TuiEnforcementState`) to manage enforcement context, loaded scope, and cached approval tokens. Default is `ManualPermissive` (TuiManual). Toggle to `ManualGuarded` (TuiManualStrict) via Ctrl+G. Guarded mode denies scope ambiguity. Preflight evaluation is advisory and displayed in status bar.
- **MCP/Agent/REST/gRPC Invariant**: For MCP, agent, REST, and gRPC execution, `EnforcementContext::evaluate()` is the mandatory pre-dispatch gate. Scope must come from `LoadedScope`. REST now carries `EnforcementContext` (via `EnforcementContext::for_surface(ExecutionSurface::RestApi, ...)` in `handle_serve()`) and dispatches through `enforcement.evaluate()` before tool execution. REST is strict by default (`McpStrict` profile). gRPC carries `EnforcementContext` in `GrpcService` and dispatches through `enforcement.approve(ExecutionSurface::GrpcApi, ...)` → `EnforcedDispatcher::dispatch_checked()`. Agent execution defensively rebuilds `AgentStrict` in the handler and validates it at runtime (`Agent::new()` rejects non-`AgentStrict` profiles). If `enforced_dispatcher` is present but `ApprovedOperation` is missing at dispatch time, agent returns a hard invariant error (no raw dispatch fallback). See `docs/ENFORCEMENT_MODES.md` for the canonical dual-mode enforcement contract.
- **eggsec-output Re-exports**: Use `eggsec_output::Severity` rather than reaching into `eggsec_output::agent::Severity`
- **Type-Level Enforcement**: Strict programmatic surfaces (REST, MCP, Agent, gRPC) require an `ApprovedOperation` token before dispatch. `EnforcedDispatcher::dispatch_checked()` verifies the request matches the approved descriptor (tool name and target). Manual surfaces (CLI, TUI) use `approve_manual()` which supports `Warn` outcomes and manual override.
- **EnforcementError Mapping**: Each surface maps `EnforcementError` to its native error type (REST → HTTP 403, MCP → error `-32025`, Agent → `anyhow::bail!`, gRPC → `Status::permission_denied`).
- **CI has no dispatch path**: The CI handler is a passive quality gate that processes pre-existing findings from stdin; it does not dispatch tools.
- **Domain Module Contract**: `DomainDescriptor` in `domain/mod.rs` is the static metadata contract for capability domains. Domains declare operations, feature gates, CLI/TUI/MCP/report integrations, and dry-run/evidence support. Descriptors are `const`-constructible, authorization-neutral, and never perform network I/O. `all_domain_descriptors()` returns all known domains regardless of feature state; check `required_feature` before use. Pilot domain: `db-pentest`. Use `all_domain_descriptors()` for the registry, `domain_descriptor_by_id()` for lookup.
- **Command Registry**: `commands/registry.rs` has `CommandRegistration` and `REGISTERED_COMMANDS` static array. `CommandContext::describe_from_registry()` builds `OperationDescriptor` from registry metadata. Pilot commands (recon, scan-ports, scan-endpoints, fingerprint) use registry-based descriptor generation; legacy commands remain on inline construction. `suggest_command()` provides edit-distance suggestions for unknown commands. See `docs/COMMAND_REGISTRY.md`.
- **Tool Registration Builder**: `tool::registration` provides `all_tool_registrations()`, `mcp_tool_registrations()`, `rest_tool_registrations()`, `grpc_tool_registrations()`, `agent_tool_registrations()`. These derive from `OperationMetadata` and `DomainDescriptor` `ToolIntegration`. Protocol listing functions now filter through registration metadata. See `docs/TOOL_REGISTRATION.md`.
- **Normalized Report Envelope**: `ReportEnvelope` in `eggsec-output::envelope` is the protocol-neutral report contract. Domain crates convert their domain-specific types into `ReportEnvelope` via `to_report_envelope()` functions. The envelope preserves report identity, finding records, evidence manifests, policy summaries, and baseline summaries. Domain bridges (mobile-static, db-pentest) produce envelopes alongside existing `to_scan_report_data()` bridges. See `docs/REPORT_EVIDENCE_MODEL.md`.
- **Evidence Redaction Model**: `RedactionState` in `eggsec-output::envelope` classifies evidence sensitivity. `RedactionPolicy` on `EvidenceManifest` declares the manifest-level redaction strategy. `EvidenceManifest.redacted_items` tracks redacted count. Domains classify evidence as `None`, `FullyRedacted`, `PartiallyRedacted`, or `Summarized` based on content sensitivity.
- **Domain Descriptor Report Metadata**: `ReportIntegration` in `domain/mod.rs` includes `normalized_report_supported: bool` flag indicating whether a domain has implemented the `to_report_envelope()` bridge. Currently `true` for db-pentest and mobile-static.

### Codebase Health

| Metric | Value |
|--------|-------|
| Tests | ~5098 (includes #[test] + #[tokio::test]) |
| Clippy | ~8 warnings (pre-existing) |
| Source files | 908 (.rs files in crates/) |
| Tabs | 33 (Tab enum variants 0-32) |
| Pipeline profiles | 18 |
| Output formats | 8 |
| Themes | 50 packaged + 3 built-in |
| CLI commands | 33 base, 52 total with all features |

### Security Notes

- **Scope Enforcement**: Private IP checks are deferred to scope rule evaluation in `is_target_allowed()` (`config/scope.rs`). Scope rules like `allow 10.0.0.0/8` correctly match private IPs before the fallback private-IP block.
- **MCP Coding Agent**: Default deny posture; stress/load/packet tools are hidden from coding-agent profile
- **Manual Overrides**: `--yes` is narrow (only `out-of-scope`/`target-expansion`); dedicated `--allow-*` flags required for others. Strict profiles/MCP/agent/REST never honor overrides.
- **REST Strict Enforcement**: REST API uses `EnforcementContext` with `McpStrict` profile. Only `EnforcementOutcome::Allow` permits dispatch; `Warn`, `RequireConfirmation`, and `Deny` all return HTTP 403 with structured `POLICY_DENIED` response. `RestState` carries `EnforcementContext` instead of `Option<Scope>`. Metadata `rest_exposable` flags are enforced before policy evaluation.
- **gRPC Strict Enforcement**: gRPC API uses `EnforcementContext` with `McpStrict` profile. Only `EnforcementOutcome::Allow` produces an `ApprovedOperation` token; `Warn`, `RequireConfirmation`, and `Deny` all fail with `Status::permission_denied`. Dispatch goes through `EnforcedDispatcher::dispatch_checked()`. Metadata `grpc_exposable` flags are enforced before policy evaluation.
- **Daemon Authorization**: `eggsec-daemon` uses `CommandPermission` enum (not stringly-typed names) for per-command RBAC. Every `ClientCommand` variant maps to a permission level via `command_permission()`. Session access stores `RuntimeSurface` and `owner_client_kind` at creation time. Strict-surface sessions (McpServer, RestApi, GrpcApi, SecurityAgent, Ci) restrict policy approval to the session Owner only. `ApprovePolicy` returns `ErrorCode::Unsupported` (not wired yet) instead of silently succeeding.
- **Daemon HTTP Transport**: The `http-api` feature enables an `axum`-based loopback HTTP transport. Default bind is `127.0.0.1:0` (loopback-only, ephemeral port). Uses `McpStrict` profile by default (`mcp_strict_by_default: true`). Each HTTP request carries a `DaemonRequestContext` with `transport: TransportKind::LoopbackHttp`. The transport is feature-gated — daemon compiles without it (Unix socket only).

### Key Patterns (Lessons Learned)

- **TUI bounds checking**: Always use `.get(i)` pattern instead of direct `chunks[i]` indexing
- **TUI is_running() guards**: All input/navigation handlers must check `!self.is_running()` before processing
- **TUI reset() methods**: Must reset all state (selectors, checkboxes, fields, focus areas)
- **Silent error suppression**: Never use `let _ =` or `filter_map(|e| e.ok())` - always log with tracing
- **Timeout wrappers**: All spawned tokio tasks should have timeout wrappers (30-300s depending on operation)
- **FxHashMap migration**: Replace `std::collections::HashMap` with `rustc_hash::FxHashMap` in performance-critical paths
- **Verification before claims**: Always verify line numbers, file paths, and whether issues still exist before including in plans
- **File path conventions**: Use `commands/handlers/` not `cli/handlers/` - the latter directory does not exist
- **Dead code detection**: Check if `#![allow(dead_code)]` is at file top - many items flagged in reviews may already be resolved
- **PayloadType location**: `PayloadType` enum is in `fuzzer/payloads/mod.rs`, not `types.rs`
- **`.ok()` vs `if let Ok`**: Not all `.ok()` calls are bugs - `if let Ok` is proper error handling. Verify the context.
- **Count verification**: Always verify statistical claims (file counts, enum variants) against actual source
- **Packaged themes**: Run `python3 scripts/package_themes.py` after modifying `themes/*.toml` to regenerate `crates/eggsec-tui/src/theme/packaged.rs`
- **Theme system**: 50 Halloy-format themes packaged via LZMA. `cyber-red` fallback always available in-code. `Theme::default()` returns `cyber-red`.
- **Theme loader**: `theme/loader.rs` parses Halloy `.toml` themes. Background thread loading via `std::thread::spawn` + `std::sync::mpsc`.
- **TUI enforcement toggle**: `TuiEnforcementState::toggle_posture()` switches between TuiManual and TuiManualStrict. TuiManualStrict does NOT honor manual overrides (unlike TuiManual).
- **TUI pending_approved**: TUI caches `ApprovedOperation` in `pending_approved` field of `EnforcementFacade` for reuse between pre-dispatch gate and `evaluate_policy_and_dispatch()`.
- **REST EnforcementContext**: `RestState` now carries `EnforcementContext` instead of `Option<Scope>`. `handle_serve()` constructs `EnforcementContext::for_surface(ExecutionSurface::RestApi, ...)`. All REST dispatch goes through `enforcement.evaluate()` before tool execution. REST is strict by default (`McpStrict` profile). Only `Allow` permits dispatch; `Warn`/`RequireConfirmation`/`Deny` all return HTTP 403. Metadata `rest_exposable` is enforced. See `docs/ENFORCEMENT_MODES.md`.
- **EnforcedDispatcher**: REST, MCP, and gRPC store `EnforcedDispatcher` (not raw `ToolDispatcher`) to structurally prevent bypass.
- **Domain descriptors always present**: Domain descriptors are always present regardless of feature state; check `required_feature` before use.
- **Feature metadata validation**: `tests/feature_matrix.rs` validates that feature strings in OperationMetadata and DomainDescriptor match actual Cargo features. `KNOWN_EGGSEC_FEATURES` must be updated when features are added.
- **CI architecture guards**: `scripts/check-architecture-guards.sh` runs static grep checks for stale terminology, MCP exposure split, raw dispatch prevention, plan retention, docs currency, and runtime boundary invariants (no TUI workers dir, no TUI deps in runtime, no transport deps in runtime, no unimplemented transports, no canonical TaskConfig/TaskResult in TUI). Requires ripgrep (`rg`). Required for every PR. `make check-architecture-ci` reproduces the full architecture guard CI job locally.
- **Feature-profile CI**: CI runs `cargo check` for 9 representative feature profiles on every PR. Platform-sensitive profiles (mobile-dynamic) may fail due to missing system deps.
- **MCP Model A assertion**: OpsAgent listing is strictly broader than conservative default (`ops_ids.len() > default_ids.len()`). The test comment and assertion must both reflect strict broadness.
- **TUI Runtime Phase 3 Dispatch**: `eggsec-runtime` defines a `TaskDispatcher` trait (dependency-free) in `dispatcher.rs`. `eggsec` crate owns the canonical dispatch logic in `eggsec::dispatch` module with `dispatch_task()` and `dispatch_inner()` public functions, plus `TaskResult` and all worker functions. Workers return `TaskResult` directly (no channel sends). `dispatch_inner()` takes `(RunRequest, progress_tx)` and returns `anyhow::Result<TaskResult>`. `eggsec-tui` implements `TuiTaskDispatcher` which calls `eggsec::dispatch::dispatch_inner()` directly, converts the result to `TaskResultEnvelope` via `task_result_to_envelope()`, sends typed `TaskResult` through `result_tx` for TUI rendering, and returns `TaskOutcome::Result(envelope)` for lifecycle tracking. `TuiExecutor` implements `RuntimeTaskExecutor`, loading per-task channel senders via `ArcSwap<TuiDispatcherContext>`. `TaskBuilder` trait now produces `RunRequest` instead of `TaskConfig`. The `eggsec-tui/src/workers/` directory has been removed.
- **Runtime dependency boundary**: `eggsec-runtime` is dependency-light (serde, tokio, thiserror, uuid, tracing). The `eggsec` engine crate depends on `eggsec-runtime` for `RunRequest`/`TaskKind`/`TaskOutcome`/`TaskResultEnvelope` types — this direction is intentional. `eggsec-runtime` must never depend on `eggsec` (no reverse dependency), and must never gain TUI (`ratatui`/`crossterm`) or transport (`axum`/`tonic`/`tokio-tungstenite`) dependencies. Enforced by architecture guards in `scripts/check-architecture-guards.sh`.
- **Daemon CommandPermission**: `CommandPermission` enum in `eggsec-daemon/src/client_registry.rs` is the single source of truth for per-command authorization levels. `command_permission()` maps every `ClientCommand` variant. Adding a new command variant without updating `command_permission()` triggers a `#[non_exhaustive]` compile error. `SessionAccess` stores `surface: RuntimeSurface` and `owner_client_kind: ClientKind` — do not derive these from other fields.
- **Daemon Persistence**: SQLite-backed session snapshots stored at lifecycle points (create, submit, cancel, close). Recovery on daemon startup via `recover_persisted_state()`. All persistence operations are fire-and-forget (spawned async, best-effort).
- **Daemon Transport Abstraction**: `DaemonRequestContext` carries `transport: TransportKind` on every inbound command. The daemon host constructs the context per-transport (`UnixSocket` in `server.rs`, `LoopbackHttp` in `http.rs`). `DaemonCapabilities` is returned in `ServerMessage::Capabilities` and declares which transports are available. `DAEMON_PROTOCOL_VERSION` (currently `1`) is included in `ServerMessage::Health` for client-side compatibility checks. Adding a new transport means implementing a listener, constructing `DaemonRequestContext` with the correct `TransportKind`, and declaring a `TransportCapability`.

## Skills Directory

Skills are located in `.opencode/skills/`:

| Skill | Purpose |
|-------|---------|
| `eggsec-agent/` | Agent-specific workflows |
| `eggsec-ai/` | AI module workflows |
| `eggsec-architecture-review/` | Architecture document review methodology |
| `eggsec-auth/` | Authentication security testing workflows |
| `eggsec-browser/` | Headless browser security testing |
| `eggsec-cli/` | CLI parsing, command dispatch, handler patterns |
| `eggsec-config/` | Config module workflows |
| `eggsec-distributed/` | Distributed module workflows |
| `eggsec-evasion/` | Evasion technique detection workflows |
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
| `eggsec-tui/` | TUI module workflows (includes `tui_testing.md` for visual regression) |
| `eggsec-waf/` | WAF module workflows |

Use the `skill` tool to load relevant skills when tackling tasks in their domain.

## Planning Notes for Future Agents

1. **Plan lifecycle**: Implementation plans in `plans/` are normally executed and deleted after completion. However, plan files are part of the handoff/audit trail and should be **retained** (with a `Status: Executed` header) for milestones where reviewers benefit from tracing overview → phase plans → corrective passes — specifically NSE milestones and other multi-phase correctness efforts. Do not delete phase plan files ad hoc; either retain them with a `Status` header or move them into a documented `plans/archive/` convention. Focus reasoning on the current codebase state rather than plan files, but use plan files for historical/audit context.
2. **Verify before implementing**: Always verify file paths, line numbers, and whether issues still exist before implementing.
3. **Error pattern verification**: Some `let _ =` patterns are followed by proper error logging via `tracing::warn!`. Verify the full context before claiming silent suppression.
4. **Wave plan verification**: Plans may contain stale assertions. Use subagents to check actual codebase state.
5. **Orphan directories**: `crates/eggstack-tui/` and `crates/slapper/` are orphan directories not in the workspace. Do not reference or depend on them.
