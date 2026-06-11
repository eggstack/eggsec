# AGENTS.md

Guidelines for AI agents working on this codebase.

## Project Overview

Eggsec is a Rust-based security testing toolkit organized as a workspace with 8 crates: `eggsec-core`, `eggsec-tool-core`, `eggsec`, `eggsec-nse`, `eggsec-tui`, `eggsec-cli`, `eggsec-output`, and `eggsec-agent`. See `README.md` for features and `ARCHITECTURE.md` for design details.

## Implementation Plan

All implementation items are complete. See `plans/` directory for historical policy-related plans.

## Quick Reference

### Build & Test Commands

```bash
cargo check -p eggsec-core
cargo check -p eggsec-tool-core
cargo check --lib -p eggsec
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
| `stress/` | `crates/eggsec/src/stress/AGENTS.override.md` |
| `distributed/` | `crates/eggsec/src/distributed/AGENTS.override.md` |
| `packet/` | `crates/eggsec/src/packet/AGENTS.override.md` (uses pnet, pnet_packet for raw sockets) |
| `loadtest/` | `crates/eggsec/src/loadtest/AGENTS.override.md` |
| `pipeline/` | `crates/eggsec/src/pipeline/AGENTS.override.md` |
| `nse/` | `crates/eggsec-nse/AGENTS.override.md` (Lua VM, NSE libraries, sandbox, CVE integration) |
| `container/` | `crates/eggsec/src/container/AGENTS.override.md` |

### Architecture Index

Use these sections as the canonical reference points when updating guidance or skills:

- `architecture/overview.md` - System-wide architecture, module index, data flow
- `architecture/tui.md` - TUI event loop, key handling, overlays, tab routing, session persistence
- `architecture/config.md` - Config loading, scope enforcement, TUI settings save semantics
- `architecture/cli_commands.md` - CLI parsing, command dispatch, handler patterns
- `architecture/output.md` - Report formatting, exports, and rendering integration
- `architecture/pipeline.md` - Security assessment pipeline, 16 profiles
- `architecture/scanner.md` - Port scanning and endpoint discovery
- `architecture/fuzzer.md` - Fuzzing engine and payload generation
- `architecture/waf.md` - WAF detection and bypass
- `architecture/recon.md` - Reconnaissance module
- `architecture/distributed.md` - Distributed coordinator/worker architecture
- `architecture/compile_time_baseline.md` - Workspace crate layout and compile-time baseline
- `architecture/auth.md` - Authentication testing module (CLI `auth-test`, policy via `CredentialTesting`, local findings only; TUI `AuthTab` is CLI-only). See plans/credential-access-*.md for historical context only (superseded by adopted runtime-policy + local-findings model).

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
- `wireless` - Standalone-complete passive WiFi scanning and security analysis (summary-by-default rogue candidates; use `--detect_suspicious` for full details; real scans require Linux `iwlist` + root/CAP_NET_ADMIN)
- `pdf` - PDF report generation
- `api-schema` - OpenAPI v3 schema-based fuzzing (marker-only)
- `full` - All features combined (16 sub-features, does not include `grpc-api`, `ws-api`, or `pdf`)

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
| Tests | 3382 (3063 #[test] + 319 #[tokio::test]) |
| Clippy | ~54 warnings (pre-existing, none in ai module) |
| Source files | 786 (.rs files in crates/) |
| Payload types | 40 |
| Tabs | 29 (Tab enum variants 0-28) |
| WAF products | 34 |
| NSE libraries | 166 public modules |
| Modules | 42 |
| Output formats | 8 (Pretty, Json, Compact, Html, Csv, Sarif, Junit, Markdown) |
| Themes | 50 packaged + 3 built-in (cyber-red, dark, light) |
| CLI commands | 26 base, 42 with all features |

### Codebase Issues (Known Stub Implementations)

No remaining stub implementations.


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
- **Auth Test Policy Integration (post-2026-06-10)**: `auth-test` handler uses `evaluate_and_enforce_operation` with `CredentialTesting` risk (central `EnforcementContext`). TUI `AuthTab` exists as standalone code but is excluded from `Tab` enum (CLI-only surface). See `architecture/auth.md`, `commands/handlers/auth_test.rs`, `cli/auth.rs`, `docs/AUTH_LAB.md`. No dedicated credential-testing Cargo feature (runtime policy gate only). `auth-test` is standalone defense-lab CLI (distinct from pipeline `ScanProfile::Auth`); local `Auth*` types only (no `ScanReportData` conversion). See plans/credential-access-*.md (historical, superseded).
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

## Skills Directory

Skills are located in `.opencode/skills/`:

| Skill | Purpose |
|-------|---------|
| `eggsec-agent/` | Agent-specific workflows |
| `eggsec-ai/` | AI module workflows |
| `eggsec-architecture-review/` | Architecture document review methodology |
| `eggsec-auth/` | Authentication security testing workflows (CLI `auth-test` primary; TUI `AuthTab` standalone/CLI-only) |
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
| `eggsec-tui/` | TUI module workflows |
| `eggsec-waf/` | WAF module workflows |
| `tui-testing/` | TUI testing guidance and visual regression patterns |
| `eggsec-wave-implementation/` | Historical wave implementation reference |

Wireless-specific guidance lives in `.opencode/skills/eggsec-agent/wireless_security_testing.md` and should be used when updating the passive wireless workflow, CLI help, or lab-use guidance.

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
| `architecture/tui.md` | Terminal User Interface (TUI) module, 29 tabs, event loop, components |
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
| `architecture/wireless.md` | Standalone-complete passive WiFi scanning (summary-by-default rogue heuristic; `--detect_suspicious` expands details; `--repeat`, `--known-good`, `--dry-run`; WPS/hidden/transition) |
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
```

## Planning Notes for Future Agents

When implementing items from `plans/plan.md`:

1. **Verify before implementing**: Many items in plan.md were verified and corrected during the 2026-06-02 review session, but always verify file paths, line numbers, and whether issues still exist before implementing.

2. **Remaining items are mostly documentation fixes**: Most remaining items are documentation corrections or low-priority improvements. Security-critical items have been addressed.

3. **No remaining stubs**: All previously-stub modules (Storage, VulnAssessment) are now fully implemented.

4. **Error pattern verification**: When addressing silent error suppression issues, verify the full context - some `let _ =` patterns are followed by proper error logging, and some `.ok()` usages are actually `if let Ok` patterns which are correct.

5. **Wave plan verification**: When verifying plan claims, use subagents to check actual codebase state - plans may contain stale assertions that no longer match reality.

6. **Count verification**: Always verify statistical claims (file counts, enum variants, match arms) against actual source. Source file counts can vary by 200+ depending on whether nested crates are included.
