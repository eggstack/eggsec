---
name: eggsec-config
description: "Configuration module workflows - use when working with EggsecConfig, scope enforcement, execution profiles, EnforcementContext, SensitiveString, or config loading/validation."
---

# Eggsec Config Skill

Configuration module workflows and patterns for managing Eggsec settings.

## Key Types and Patterns

### EggsecConfig (`settings.rs`)
Main configuration struct loaded via `config::load_config()`.

**Sub-configs:**
- `HttpConfig` - HTTP client settings (timeout, retries, proxy, TLS)
- `ScanConfig` - Scanning settings (concurrency, timing, stealth)
- `OutputConfig` - Output formatting
- `NotificationConfig` - Webhook/email notifications
- `PathsConfig` - Directory paths (flattened via `#[serde(flatten)]`)
- `ProxyConfigEntry` - Proxy list entries
- `AiConfig` - AI provider settings
- `SearchConfig` - SearXNG search integration
- `AlertChannelsConfig` - Alert routing

### Scope (`scope.rs`)
Target restrictions for security compliance.

**Key security fix**: Direct IP addresses now properly blocked via private IP checks in `TargetScope::parse()` and `parse_hostname_only()`. Previously, passing an IP like `127.0.0.1` bypassed DNS resolution and the private IP block.

### Config Loading (`loader.rs`)
```rust
let config = load_config(None)?;  // Auto-discovers config file
let config = load_config(Some("/path/to/config.toml"))?;  // Explicit path
```

**File discovery order:**
1. Explicit path from CLI (`-c` flag)
2. `./eggsec.toml`
3. `./.eggsec/eggsec.toml`
4. `./config/eggsec.toml`
5. `~/.config/eggsec/eggsec.toml` (via `ProjectDirs`)

### SensitiveString (`types.rs`)
Zeroized credential wrapper for API keys, passwords, PSK.
```rust
pub struct SensitiveString(String);
```
Used in `ApiKeyConfig`, `ProxyConfigEntry.password`, `RemoteConfig.psk`, `WebhookConfigEntry.secret`.

**Warning**: Serializes in plaintext - config files with secrets need strict permissions (`chmod 600`).

## Common Tasks

### Adding a New Configuration Option
1. Add field to `EggsecConfig` or relevant sub-config struct
2. Add `#[serde(default)]` for the field
3. Add `validate()` method if validation is needed
4. Add tests following patterns in `config/loader.rs`

### Config Validation
`EggsecConfig::validate()` orchestrates sub-validations:
```rust
impl EggsecConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.http.validate()?;
        self.scan.validate()?;
        // ...
    }
}
```

**AlertChannelsConfig validation (2026-05-28)**: `EggsecConfig::validate()` now validates all alert channel types:
- **Webhook**: URL must start with http:// or https://
- **Email**: smtp_host, smtp_port, from, to cannot be empty
- **Slack**: webhook_url must start with http:// or https://
- **PagerDuty**: routing_key cannot be empty

### Execution Profiles
```rust
use eggsec::config::EnforcementContext;

let enforcement = EnforcementContext::mcp_strict(policy, loaded_scope);
let outcome = enforcement.evaluate(&descriptor);
match outcome {
    EnforcementOutcome::Allow(decision) => { /* proceed */ }
    EnforcementOutcome::Warn(decision) => { /* log warnings, proceed (CLI/TUI); deny (REST/MCP/Agent/CI) */ }
    EnforcementOutcome::RequireConfirmation(decision) => { /* manual-only (ManualPermissive); convert to proceed only if matching ManualOverride present (narrow --yes for out-of-scope/target-expansion only; dedicated --allow-private-resolution / --allow-cross-host-redirect etc. for others); else treat as Deny. Strict profiles/MCP/agent/REST never honor overrides. */ }
    EnforcementOutcome::Deny(decision) => { /* deny */ }
}
```

`EnforcementContext::evaluate(descriptor)` is the central boundary (provenance via LoadedScope, DenialClass downgrade for ManualPermissive only on safe scope misses, positive capability checks for strict, per-scan agent re-eval). Agent execution defensively rebuilds `AgentStrict` in the handler (defense-in-depth); `Agent::new()` validates profile at runtime. `evaluate_and_enforce_operation` (CLI handlers) matches on `RequireConfirmation` only for `ManualPermissive` + matching `ManualOverride` (narrow --yes semantics for out-of-scope/target-expansion; dedicated private/redirect flags for their classes; stable kebab audit via as_str/confirmation_class_strings). All other profiles treat it as `Deny`; strict profiles/MCP/agent/REST never honor overrides. REST specifically: only `Allow` permits dispatch; `Warn`, `RequireConfirmation`, and `Deny` all return HTTP 403 with structured `POLICY_DENIED` response. Preferred MCP production constructor: `McpServer::with_enforcement`. Legacy direct `evaluate_enforcement` / `evaluate_operation_policy` deprecated for denial paths; prefer with-enforcement path. Profiles: `ManualPermissive` (default CLI), `ManualGuarded` (--strict-scope), `CiStrict` (CI), `McpStrict` (MCP/REST), `AgentStrict` (agent).

### ExecutionSurface

`ExecutionSurface` derives `ExecutionProfile` from caller origin. Use `EnforcementContext::for_surface()` instead of profile-specific constructors at entry points:

```rust
use eggsec::config::{ExecutionSurface, EnforcementContext};

let surface = ExecutionSurface::CliManual; // or TuiManual, McpServer, etc.
let enforcement = EnforcementContext::for_surface(surface, policy, loaded_scope);
```

Surfaces that `honors_manual_override()` are only `CliManual` and `TuiManual`. Automated surfaces (`is_agent_controlled()`) require explicit scope manifests for networked operations.

### OperationMetadata

`OperationMetadata` lives in `config::policy` and is the canonical source of truth for `OperationDescriptor` generation across all surfaces (REST, MCP, TUI, agent). It is a static registry of 29 operations with 32 alias mappings. Use `metadata_for_tool_id(tool_id)` to resolve any tool ID (including aliases like "scan" → "scan-ports") to its canonical `OperationMetadata`, then call `descriptor_for_target()` to generate an `OperationDescriptor` with surface-specific overrides applied.

### Capability Declarations
```rust
use eggsec::config::Capability;
// Tools declare required_capabilities in OperationDescriptor
// Policies declare allowed_capabilities / denied_capabilities in ExecutionPolicy
```

### Scope Enforcement
```rust
let scope = load_scope(None)?;
if !scope.is_target_allowed("example.com")? {
    return Err("Target out of scope");
}
```

## Error Handling

`ConfigError` enum:
- `Io` - File read/write errors
- `Parse` - TOML/YAML parsing errors
- `Serialize` - Serialization errors
- `Validation` - Validation failures

**Warning**: Avoid `unwrap_or_default()` on async operations; use explicit match with tracing instead.

## Testing

### Running Config Tests
```bash
cargo test --lib -p eggsec config::        # All config tests
cargo test --lib -p eggsec config::loader  # Loader tests only
cargo test --lib -p eggsec config::scope   # Scope tests only
cargo test --lib -p eggsec config::policy_decision  # Enforcement policy tests
```

### Test Patterns
See inline tests in:
- `config/loader.rs` - Config loading, file discovery, TOML/YAML parsing
- `config/scope.rs` - Scope rule matching, CIDR, wildcard patterns
- `config/settings.rs` - Validation, defaults
- `config/policy_decision.rs` - 48 enforcement policy tests (Phase 4 regression coverage for manual discretion invariants)

## Security Notes

### Private IP Blocking
`Scope::is_target_allowed()` blocks:
- Loopback (`127.0.0.0/8`, `::1`)
- Private (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`)
- Link-local (`169.254.0.0/16`, `fe80::/10`)
- ULA (`fc00::/7`)

This prevents SSRF against cloud metadata endpoints (e.g., `169.254.169.254`).

### Config File Permissions
`check_config_file_permissions()` warns but does NOT enforce. Config files with secrets should be `chmod 600`.

## Performance

**FxHashMap**: All HashMap usages in the config module use `rustc_hash::FxHashMap` instead of `std::collections::HashMap` for performance. This applies to:
- `AlertChannelsConfig.channels` (`settings.rs:21`)
- `WebhookConfigEntry.headers` (`settings.rs:38`)
- `HttpConfig.default_headers` (`http.rs:39`)
- `ScanConfig.profiles` (`settings.rs:109`)
- `WebhookConfig.headers` (`scan.rs:132`)

## TUI Settings Tab

The TUI Settings tab (`tui/tabs/settings/main.rs`) exposes only a subset of config fields, but saving now merges those changes into the loaded config instead of rebuilding from defaults. Non-exposed sections are preserved, including `profiles`, `schedule`, `remote`, `ai`, `search`, `alert_channels`, and other untouched fields.

For full config management, use CLI commands or edit config files directly when you need to inspect or change the hidden sections explicitly.

## Related Documentation

- `crates/eggsec/src/config/AGENTS.override.md` - Detailed config patterns
- `architecture/config.md` - Architecture documentation
