# Configuration System

The configuration system handles loading settings from files, environment variables, and defaults, while also enforcing scanning scopes to prevent accidental testing of out-of-scope targets.

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
- `ExecutionPolicy` - Operation policy controls (scope requirements, risk levels, allowed operations)

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

**Security enforcement:**
- **Private IP blocking**: Direct IP addresses (e.g., `127.0.0.1`, `169.254.169.254`) are blocked via `TargetScope::parse()` and `parse_hostname_only()` - they now properly go through private IP checks
- **Included Targets**: IP ranges (CIDR), domains, or specific URLs
- **Excluded Targets**: Blacklisted IPs or domains that should never be touched
- **Enforcement**: Most scanning and fuzzing operations check the `Scope` before initiating a connection
- **FxHashMap**: All HashMap usages use `rustc_hash::FxHashMap` for performance:
  - `AlertChannelsConfig.channels` (`settings.rs:21`)
  - `WebhookConfigEntry.headers` (`settings.rs:38`)
  - `HttpConfig.default_headers` (`http.rs:39`)
  - `EggsecConfig.profiles` (`settings.rs:109`)
  - `WebhookConfig.headers` (`scan.rs:132`)

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
