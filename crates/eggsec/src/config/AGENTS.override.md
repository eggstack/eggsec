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

## Phase 4 Regression Tests

Policy decision tests in `policy_decision.rs` (48 tests) lock manual-mode enforcement invariants:

- `manual_override_permits_narrow_yes_for_outofscope_targetexpansion_only` - `--yes` is narrow
- `manual_override_dedicated_flags_permit_only_their_class` - Each `--allow-*` flag covers only its class
- `manual_override_traffic_interception_permits_only_web_proxy` - TrafficInterception requires web-proxy flag
- `guarded_positive_scope_miss_with_explicit_rules_denies` - ManualGuarded denies positive scope misses
- `strict_profiles_treat_require_confirmation_as_deny` - Strict profiles never honor overrides
- `manual_yes_does_not_permit_private_resolution` / `manual_yes_does_not_permit_nonbaseline_capability` - `--yes` cannot cover dedicated classes
- `explicit_exclusion_denies_in_all_profiles` - Explicit exclusions are never silently warnable
- `manual_permissive_does_not_downgrade_risk_policy_denial` / `feature_missing_denial` / `capability_denial` - Hard deny classes stay hard

See `docs/ENFORCEMENT_MODES.md` Phase 4 section for the full invariant-to-test mapping.

## Phase 8 Enforcement Matrix

`tests/enforcement_matrix.rs` (105 tests) provides systematic cross-surface coverage for the dual-mode enforcement contract. Tests cover:

- All 8 execution surfaces mapped to correct profiles
- Manual permissive: safe ops allow, scope misses require confirmation, assume_yes narrow
- Manual guarded: scope misses deny, overrides ignored
- MCP/Agent/REST/CI: strict behavior, no confirmation/warn path, no override honoring
- Risk tier matrix across all surfaces with/without policy flags
- Capability matrix: baseline allowed, non-baseline requires explicit allow, denied caps hard-deny
- Override isolation: ManualOverride::permits() per ConfirmationClass
- Scope state matrix: DefaultEmpty, explicit allow/miss/exclusion
- Dual-mode contract: permissive never hard-deny safe in-scope, strict never warn/confirm

Run: `cargo test --test enforcement_matrix -p eggsec`