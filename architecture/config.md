# Configuration System

The configuration system handles loading settings from files, environment variables, and defaults, while also enforcing scanning scopes to prevent accidental testing of out-of-scope targets.

## Core Components (`src/config/`)

### `SlapperConfig` (`settings.rs`)

The main configuration struct that holds all tool settings. It is typically loaded from `slapper.toml` or `slapper.yaml`.

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

### `Scope` (`scope.rs`)

The `Scope` struct is critical for security and compliance. It defines which targets are "in-scope" and which are explicitly excluded.

**Key Methods:**
- `is_target_allowed(target)` - Returns `Result<bool, ScopeError>`, checks if target is allowed
- `validate_url(url)` - Returns `Result<bool, ScopeError>`, validates URL's host via `is_target_allowed`
- `is_port_allowed(port)` - Returns `bool`, checks port allowlist/blocklist

**Security enforcement:**
- **Private IP blocking**: Direct IP addresses (e.g., `127.0.0.1`, `169.254.169.254`) are blocked via `TargetScope::parse()` and `parse_hostname_only()` - they now properly go through private IP checks
- **Included Targets**: IP ranges (CIDR), domains, or specific URLs
- **Excluded Targets**: Blacklisted IPs or domains that should never be touched
- **Enforcement**: Most scanning and fuzzing operations check the `Scope` before initiating a connection
- **FxHashMap**: All HashMap usages use `rustc_hash::FxHashMap` for performance:
  - `AlertChannelsConfig.channels` (`settings.rs:21`)
  - `WebhookConfigEntry.headers` (`settings.rs:38`)
  - `HttpConfig.default_headers` (`http.rs:39`)
  - `SlapperConfig.profiles` (`settings.rs:109`)
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

Slapper looks for config in this order:
1. `--config` / `-c` command-line argument
2. `./slapper.toml`
3. `./.slapper/slapper.toml`
4. `./config/slapper.toml`
5. `~/.config/slapper/slapper.toml` (via `ProjectDirs`)

## Validation

`SlapperConfig::validate()` orchestrates all sub-validations. Config files with secrets should be `chmod 600` - `check_config_file_permissions()` warns about world/group-readable permissions but does not enforce.

## Error Handling

`ConfigError` enum has four variants:
- `Io` - File read/write errors
- `Parse` - TOML/YAML parsing errors
- `Serialize` - Serialization errors
- `Validation` - Validation failures (field out of range, etc.)

Use `?` propagation instead of `unwrap_or_default()` to avoid silent failures in async contexts.

## Key Security Fixes (2026-05-22)

- **Private IP bypass fixed**: Direct IP addresses now properly blocked in `TargetScope::parse()` and `parse_hostname_only()`
- **Project qualifier fixed**: `api.rs` now uses `PROJECT_QUALIFIER` consistently with other modules
- **Error propagation**: Config module uses proper error propagation rather than silent fallback to defaults
