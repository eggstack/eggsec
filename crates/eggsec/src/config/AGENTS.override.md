# Config Module Override

Specialized guidance for the configuration module.

## EggsecConfig

`config::load_config()` returns the main configuration.

## PathsConfig

Directory paths are flattened into `EggsecConfig`.

## Scope Enforcement Pattern

When checking if a target is allowed, use `scope.is_target_allowed()` which returns `Result<bool, ScopeError>`:
```rust
match scope.is_target_allowed(target)? {
    true => proceed_with_scan(target),
    false => return Err("Target not in scope"),
}
```

## Key Security Fixes (2026-05-22)

1. **Private IP bypass fixed**: Direct IP addresses (e.g., `127.0.0.1`) now properly blocked in `TargetScope::parse()` and `parse_hostname_only()`. Previously they bypassed DNS resolution and private IP blocking.

2. **Project qualifier fixed**: `api.rs` now uses `PROJECT_QUALIFIER` consistently with other modules.

## Performance (2026-05-22)

All HashMap usages use `rustc_hash::FxHashMap` for performance:
- `AlertChannelsConfig.channels`
- `WebhookConfigEntry.headers`
- `HttpConfig.default_headers`
- `EggsecConfig.profiles`

## Validation

`EggsecConfig::validate()` orchestrates sub-validations. Always call it after loading config:
```rust
let config = load_config(None)?;
config.validate()?; // Returns ConfigError::Validation on failure
```

`Scope::validate()` added (2026-05-29) to check:
1. At least one allowed target exists when `require_explicit_scope` is true
2. No duplicate ports in `allowed_ports`
3. `max_requests_per_second` is between 1 and 10000 if specified

## Error Handling

- `ConfigError::Io` - File read/write errors
- `ConfigError::Parse` - TOML/YAML parsing errors  
- `ConfigError::Serialize` - Serialization errors
- `ConfigError::Validation` - Validation failures (field out of range, etc.)

Use `?` propagation instead of `unwrap_or_default()` to avoid silent failures.