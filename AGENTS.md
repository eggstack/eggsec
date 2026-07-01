# AGENTS.md

Guidelines for AI agents working on this codebase.

## Project Overview

Eggsec is a Rust-based security testing toolkit organized as a workspace with 11 crates: `eggsec-core`, `eggsec-tool-core`, `eggsec`, `eggsec-nse`, `eggsec-tui`, `eggsec-cli`, `eggsec-output`, `eggsec-agent`, `eggsec-db-lab`, `eggsec-web-proxy`, and `eggsec-mobile-lab`. See `README.md` for features and `architecture/overview.md` for design details.

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
# db-pentest (domain crate)
cargo check -p eggsec-db-lab
cargo test -p eggsec-db-lab
cargo clippy -p eggsec-db-lab

# db-pentest (main crate with adapter)
cargo check -p eggsec --features db-pentest
cargo test --lib -p eggsec --features db-pentest
cargo clippy --lib -p eggsec --features db-pentest

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
make check-no-default     # validate no-default-features workspace build
make check-feature-profiles # representative feature profile checks
make build         # release build
```

> **Note**: CI uses `cargo-tarpaulin` for coverage, while the Makefile uses `cargo llvm-cov`. Both measure the same thing but with different tools.

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
- `docs/FEATURE_MATRIX.md` - Feature inventory, classification, naming conventions, build profiles, and cross-reference

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

**Marker-only features (no deps, just build gating):**

`tool-api`, `insecure-tls`, `rest-api` (strict enforcement via `EnforcementContext` + `McpStrict` by default; includes `POST /api/v1/tools/{tool_id}/preflight` endpoint), `grpc-api`, `ws-api`, `nse-ssh2`, `nse-sandbox`, `ai-integration`, `websocket`, `headless-browser`, `database`, `container`, `sbom`, `advanced-hunting`, `compliance`, `external-integrations`, `finding-workflow`, `vuln-management`, `cloud`, `git-secrets`, `web-proxy-mcp`, `c2-mcp`, `transparent-proxy`, `dynamic-plugins`, `pdf`, `api-schema`, `db-pentest-mongodb`, `db-pentest-redis`, `db-pentest-mcp`

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
- `OperationMetadata` - Canonical operation metadata, single source of truth for `OperationDescriptor` generation across REST, MCP, TUI, and agent surfaces. Defined in `config::policy`, re-exported from `config` and `tool::metadata`. Static registry with 29 operations + 32 aliases.
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
| Tests | ~4876 (includes #[test] + #[tokio::test]) |
| Clippy | ~54 warnings (pre-existing, none in ai module) |
| Source files | 878 (.rs files in crates/) |
| Tabs | 33 (Tab enum variants 0-32) |
| Pipeline profiles | 18 |
| Output formats | 8 |
| Themes | 50 packaged + 3 built-in |
| CLI commands | 27 base, 46 total with all features |

### Security Notes

- **Scope Enforcement**: Private IP checks are deferred to scope rule evaluation in `is_target_allowed()` (`config/scope.rs`). Scope rules like `allow 10.0.0.0/8` correctly match private IPs before the fallback private-IP block.
- **MCP Coding Agent**: Default deny posture; stress/load/packet tools are hidden from coding-agent profile
- **Manual Overrides**: `--yes` is narrow (only `out-of-scope`/`target-expansion`); dedicated `--allow-*` flags required for others. Strict profiles/MCP/agent/REST never honor overrides.
- **REST Strict Enforcement**: REST API uses `EnforcementContext` with `McpStrict` profile. Only `EnforcementOutcome::Allow` permits dispatch; `Warn`, `RequireConfirmation`, and `Deny` all return HTTP 403 with structured `POLICY_DENIED` response. `RestState` carries `EnforcementContext` instead of `Option<Scope>`. Metadata `rest_exposable` flags are enforced before policy evaluation.
- **gRPC Strict Enforcement**: gRPC API uses `EnforcementContext` with `McpStrict` profile. Only `EnforcementOutcome::Allow` produces an `ApprovedOperation` token; `Warn`, `RequireConfirmation`, and `Deny` all fail with `Status::permission_denied`. Dispatch goes through `EnforcedDispatcher::dispatch_checked()`. Metadata `grpc_exposable` flags are enforced before policy evaluation.

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
- **TUI pending_approved**: TUI caches `ApprovedOperation` in `pending_approved` field for reuse between pre-dispatch gate and `evaluate_policy_and_dispatch()`.
- **Domain descriptors always present**: Domain descriptors are always present regardless of feature state; check `required_feature` before use.
- **Feature metadata validation**: `tests/feature_matrix.rs` validates that feature strings in OperationMetadata and DomainDescriptor match actual Cargo features. `KNOWN_EGGSEC_FEATURES` must be updated when features are added.

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

1. **Plan lifecycle**: Implementation plans in `plans/` are executed and deleted after completion. Focus on the current codebase state rather than plan files.
2. **Verify before implementing**: Always verify file paths, line numbers, and whether issues still exist before implementing.
3. **Error pattern verification**: Some `let _ =` patterns are followed by proper error logging via `tracing::warn!`. Verify the full context before claiming silent suppression.
4. **Wave plan verification**: Plans may contain stale assertions. Use subagents to check actual codebase state.
5. **Orphan directories**: `crates/eggstack-tui/` and `crates/slapper/` are orphan directories not in the workspace. Do not reference or depend on them.
