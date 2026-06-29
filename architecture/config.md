# Configuration System

The configuration system handles loading settings from files, environment variables, and defaults, while also enforcing scanning scopes to prevent accidental testing of out-of-scope targets.

> See [../../docs/ENFORCEMENT_MODES.md](../../docs/ENFORCEMENT_MODES.md) for the canonical dual-mode enforcement contract.

## Core Components (`src/config/`)

### `EggsecConfig` (`settings.rs`)

The main configuration struct that holds all tool settings. It is typically loaded from `eggsec.toml` or `eggsec.yaml`.

**Sub-configs:**
- `HttpConfig` - HTTP client settings (timeout, retries, proxy, TLS, `retry_delay_ms`)
- `ScanConfig` - Scanning settings (concurrency, timing, stealth, `port_timeout_secs`)
- `OutputConfig` - Output formatting
- `NotificationConfig` - Webhook/email notifications
- `PathsConfig` - Directory paths (flattened via `#[serde(flatten)]`)
- `ProxyConfigEntry` - Proxy list entries
- `AiConfig` - AI provider settings
- `SearchConfig` - SearXNG search integration
- `AlertChannelsConfig` - Alert routing
- `ReconConfig` - Reconnaissance settings (`dns_concurrency`, `apis` for API configuration)
- `RemoteConfig` - Remote worker settings (`psk`, `default_port`, `allowed_workers`)
- `ExecutionPolicy` - Operation policy controls (scope requirements, risk levels, allowed operations); includes `allow_exploit_adjacent` field for near-exploitation testing

### `Scope` (`scope.rs`)

The `Scope` struct is critical for security and compliance. It defines which targets are "in-scope" and which are explicitly excluded.

**Key Methods:**
- `is_target_allowed(target)` - Returns `Result<bool, ScopeError>`, checks if target is allowed
- `validate_url(url)` - Returns `Result<bool, ScopeError>`, validates URL's host via `is_target_allowed`
- `is_port_allowed(port)` - Returns `bool`, checks port allowlist/blocklist
- `validate()` - Validates scope configuration: `allowed_targets` must not be empty when `require_explicit_scope` is true; duplicate ports in `allowed_ports` are rejected; `max_requests_per_second` must be greater than 0 if set

**ScopeRule Construction:**
- `ScopeRule::new(pattern)` - Creates a scope rule from a glob/regex pattern string
- `ScopeRule::with_cidr(cidr)` - Creates a scope rule from CIDR notation (e.g., `10.0.0.0/8`). Parses via `IpNetwork::from_str()` and stores the CIDR for IP-range matching

### `LoadedScope` and `ScopeSource` (`scope.rs`)

`LoadedScope` wraps a `Scope` with provenance metadata indicating where the scope was loaded from:

```rust
pub enum ScopeSource {
    DefaultEmpty,       // No scope provided by user
    ConfigFile,         // Loaded from eggsec.toml
    CliScopeFile,       // Loaded from --scope CLI flag
    GeneratedPreset,    // Generated from a profile or preset
}
```

**Key methods on `LoadedScope`:**
- `is_explicit_manifest()` - Returns `true` if the scope was provided via `--scope` or config file (not default empty)
- `source()` - Returns the `ScopeSource`
- `scope()` - Returns a reference to the underlying `Scope`

**Security enforcement:**
- Strict profiles (`CiStrict`, `McpStrict`, `AgentStrict`) require `is_explicit_manifest() == true` for networked operations
- `DefaultEmpty` scope blocks all networked operations in strict profiles
- Private IP blocking: Direct IP addresses (e.g., `127.0.0.1`, `169.254.169.254`) are blocked via `TargetScope::parse()` and `parse_hostname_only()` - they now properly go through private IP checks
- Included Targets: IP ranges (CIDR), domains, or specific URLs
- Excluded Targets: Blacklisted IPs or domains that should never be touched
- Enforcement: Most scanning and fuzzing operations check the `Scope` before initiating a connection

**Scope loading:**
- `load_scope_with_source()` loads a scope from a file and tags it with the appropriate `ScopeSource`
- When `--scope` is provided, the result has `ScopeSource::CliScopeFile`
- When loaded from config, the result has `ScopeSource::ConfigFile`
- When no scope is provided, the result has `ScopeSource::DefaultEmpty`
- **FxHashMap**: All HashMap usages use `rustc_hash::FxHashMap` for performance:
  - `AlertChannelsConfig.channels` (`settings.rs:21`)
  - `WebhookConfigEntry.headers` (`settings.rs:38`)
  - `HttpConfig.default_headers` (`http.rs:39`)
  - `EggsecConfig.profiles` (`settings.rs:109`)
  - `WebhookConfig.headers` (`scan.rs:132`)

### `ExecutionProfile` and `EnforcementOutcome` (`policy.rs`, `policy_decision.rs`)

- `ExecutionProfile` - Caller trust boundary: `ManualPermissive`, `ManualGuarded`, `CiStrict`, `McpStrict`, `AgentStrict`
- `EnforcementOutcome` - Profile-aware result: `Allow(PolicyDecision)`, `Warn(PolicyDecision)`, `RequireConfirmation(PolicyDecision)`, `Deny(PolicyDecision)`
- `evaluate_enforcement()` - Wraps `evaluate_operation_policy()` with profile-specific behavior
- `Capability` - Operation capability declarations for tool metadata
- `DiscoveredTargetStatus` - Discovery promotion model for agent/MCP modes
- `ManualOverride` and `ConfirmationClass` (policy_decision.rs) - Manual discretion overrides (see below)

### Manual discretion mode (plan 2026-06-10)

Under `ManualPermissive` (default CLI/TUI), `evaluate_enforcement` returns `EnforcementOutcome::RequireConfirmation(PolicyDecision)` (instead of hard `Deny`) for operator-discretion cases: explicit allowlist miss with positive scope rules (`ConfirmationClass::OutOfScope`), explicit exclusion (`ExplicitExclusion`), high-risk operations (`HighRisk`), non-baseline capability (`NonBaselineCapability`), private resolution (`PrivateResolution`), cross-host redirect (`CrossHostRedirect`), or target expansion (`TargetExpansion`).

`ManualGuarded`, `CiStrict`, `McpStrict`, and `AgentStrict` treat `RequireConfirmation` as `Deny` (no proceed path).

`CommandContext::evaluate_and_enforce_operation` (in commands/handlers/mod.rs) matches on `RequireConfirmation` only for `ManualPermissive`: if the `CommandContext`'s `manual_override: ManualOverride` has flags permitting the required classes (e.g. `allow_out_of_scope`, `allow_explicit_exclusion`, `allow_high_risk`, `allow_nonbaseline_capability`, `assume_yes`, `allow_private_resolution`, `allow_cross_host_redirect`), it proceeds and records the override (`manual_override_used`, `manual_override_reason`, `manual_override_classes` on the decision for audit, using stable kebab strings from `ConfirmationClass::as_str()`). `--yes` / `assume_yes` is narrow (only `out-of-scope`/`target-expansion`); dedicated `--allow-private-resolution` / `--allow-cross-host-redirect` etc. are required for their classes. Without matching flags it bails with a precise message listing the required override flag(s). Automated profiles never reach a proceed path for `RequireConfirmation`. `confirmation_class_strings` dedups classes for audit/JSON/warnings while preserving first-seen order.

`ManualOverride` (with `permits(class: ConfirmationClass)`) and `ConfirmationClass` live in `policy_decision.rs`; they are not part of MCP/agent schemas or automated paths. Override flags are CLI-only (global, manual-only; ignored/rejected under `--strict-scope` or strict profiles). Strict profiles/MCP/agent never honor overrides.

This preserves hard denials for missing features, invalid targets, and all automated enforcement. See `docs/plans/2026-06-10-manual-discretion-mode-plan.md`.

**Phase 4 regression coverage**: 96 tests across `config::policy_decision::tests` (48) and `commands::handlers::tests` (48) lock all manual-mode invariants. **Phase 8 enforcement matrix**: 134 tests in `tests/enforcement_matrix.rs` provide systematic cross-surface coverage for the dual-mode contract. See `docs/ENFORCEMENT_MODES.md` Phase 4 and Phase 8 sections for the full invariant-to-test mapping.

### `EnforcementContext` (`policy_decision.rs`)

`EnforcementContext` bundles `ExecutionProfile`, `ExecutionPolicy`, and `LoadedScope` into a single struct for shared enforcement across all execution paths. This eliminates the need to pass profile/policy/scope separately through the call stack. `EnforcementContext::evaluate(descriptor)` is the mandatory central boundary: it performs LoadedScope provenance checks (strict profiles deny `DefaultEmpty` for `requires_explicit_scope` target-bearing ops), applies `DenialClass` downgrade logic (ManualPermissive only for safe ScopeMissing/TargetOutOfScope when no positive rules declared and no exclusions/feature/risk/capability/hazard denials), performs positive-capability allow checks for strict profiles, and runs full risk/feature/policy enforcement. Per-scan re-evaluation occurs for agents in `execute_scan_with_depth`.

> For MCP and autonomous-agent execution, `EnforcementContext::evaluate()` is the mandatory pre-dispatch gate. Scope provenance must come from `LoadedScope`; raw `Scope` is not sufficient for automated execution.

**Baseline capabilities** for strict automated profiles: `PassiveFingerprint`, `ActiveProbe`, `Crawl`, `WafDetect`. Non-baseline capabilities require explicit `allowed_capabilities`.

### ExecutionSurface

`ExecutionSurface` (defined in `config/policy.rs`) describes where an operation originates and derives the correct `ExecutionProfile`. Entry points should select an `ExecutionSurface` variant rather than hand-picking profiles.

| Surface | Profile | Manual Override |
|---------|---------|-----------------|
| `CliManual` | `ManualPermissive` | Yes |
| `TuiManual` | `ManualPermissive` | Yes |
| `CliManualStrict` | `ManualGuarded` | No |
| `TuiManualStrict` | `ManualGuarded` | No |
| `McpServer` | `McpStrict` | No |
| `SecurityAgent` | `AgentStrict` | No |
| `Ci` | `CiStrict` | No |
| `RestApi` | `McpStrict` (placeholder) | No |

Use `EnforcementContext::for_surface(surface, policy, loaded_scope)` for centralized construction.

**Preferred constructors:** `EnforcementContext::manual_permissive`, `manual_guarded`, `ci_strict`, `mcp_strict`, `agent_strict` (no `cli(...)` helper; callers construct the appropriate profile).

**Construction per execution path:**
- CLI commands: `EnforcementContext::manual_permissive(...)` (default) or `manual_guarded(...)` (when `--strict-scope` is used)
- MCP server: Forces `McpStrict` profile; preferred production constructor is `McpServer::with_enforcement(registry, api_key, profile, enforcement)` (passes pre-built `EnforcementContext`)
- Agent: Forces `AgentStrict` profile; `EnforcementContext::agent_strict` is passed to `AgentConfig`. Handler defensively rebuilds `AgentStrict` from policy/scope (defense-in-depth). `Agent::new()` validates profile at runtime.
- CI mode: Uses `CiStrict` profile when detected

**Key methods:**
- `evaluate(descriptor)` - Central evaluator; returns `EnforcementOutcome` (Allow/Warn/RequireConfirmation/Deny) wrapping `PolicyDecision`. Handles provenance, DenialClass downgrades, and capability checks internally. (RequireConfirmation is produced only for ManualPermissive discretion cases per 2026-06-10 plan with narrow `--yes` + dedicated `--allow-*` semantics; automated profiles treat it as denial.)
- `requires_explicit_manifest_for(descriptor)` / `require_explicit_scope_for_networked()` - Provenance helpers used by `evaluate`.
- `profile()` - Returns the `ExecutionProfile`
- `scope()` - Returns the `LoadedScope`

**Security enforcement:**
- MCP tools/call handler evaluates `self.enforcement.evaluate()` BEFORE dispatch to any tool; `EnforcementContext` is the sole policy/scope authority. Legacy helpers (`policy_decision_for_mcp_call`, `denial_from_violation`, `with_scope`, `with_scope_and_profile`) have been removed.
- Agent refuses to run without an explicit scope manifest; handler defensively rebuilds `AgentStrict` enforcement (defense-in-depth); `Agent::new()` rejects non-`AgentStrict` profiles; per-scan `enforcement.evaluate` immediately before dispatch.
- Strict profiles require `is_explicit_manifest() == true` for networked operations (enforced centrally inside `evaluate`).

### `Loader` (`loader.rs`)

Handles the mechanics of finding and parsing configuration files.

- Supports TOML (primary) and YAML (`.yaml`/`.yml`) formats
- Merges file-based config with command-line overrides
- Provides default values for all settings

## TUI Settings Tab

The TUI settings editor in `tui/tabs/settings/main.rs` applies exposed fields on top of an existing config instead of rebuilding from defaults. Non-exposed sections are preserved, including:
- `profiles`
- `schedule`
- `remote`
- `ai`
- `search`
- `alert_channels`
- Other fields not shown in the UI

The editor is still a quick-settings surface, but saving it is no longer destructive for untouched config sections.

## Configuration Files

Eggsec looks for config in this order:
1. `--config` / `-c` command-line argument
2. `./eggsec.toml`
3. `./.eggsec/eggsec.toml`
4. `./config/eggsec.toml`
5. `~/.config/eggsec/eggsec.toml` (via `ProjectDirs`)

## Validation

`EggsecConfig::validate()` orchestrates all sub-validations. Config files with secrets should be `chmod 600` - `check_config_file_permissions()` warns about world/group-readable permissions but does not enforce.

## Error Handling

`ConfigError` enum has four variants:
- `Io` - File read/write errors
- `Parse` - TOML/YAML parsing errors
- `Serialize` - Serialization errors
- `Validation` - Validation failures (field out of range, etc.)

Use `?` propagation instead of `unwrap_or_default()` to avoid silent failures in async contexts.

### ScopeError Enum

`ScopeError` enum at `scope.rs:400-422` has 7 variants:
- `Validation(String)` - General validation error
- `FileRead(String, String)` - Failed to read scope file
- `Parse(String, String)` - Failed to parse scope file
- `InvalidUrl(String, String)` - Invalid URL
- `InvalidCidr(String, String)` - Invalid CIDR notation
- `InvalidTarget(String)` - Invalid target
- `DnsResolution(String, String)` - DNS resolution failed

## Key Security Fixes (2026-05-22)

- **Private IP bypass fixed**: Direct IP addresses now properly blocked in `TargetScope::parse()` and `parse_hostname_only()`
- **Project qualifier fixed**: `api.rs` now uses `PROJECT_QUALIFIER` consistently with other modules
- **Error propagation**: Config module uses proper error propagation rather than silent fallback to defaults
