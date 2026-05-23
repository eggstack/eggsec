# Config Module Architecture Review

## Summary

The configuration module (`crates/slapper/src/config/`) largely matches the documented architecture in `architecture/config.md`. Most implementations are correct, but there are some discrepancies and potential improvements.

## Verified Correct

| Claim | Implementation | Status |
|-------|----------------|--------|
| `SlapperConfig` in `settings.rs` | All sub-configs present | ✅ |
| `Scope` with `is_target_allowed`, `validate_url`, `is_port_allowed` | All in `scope.rs` | ✅ |
| Private IP blocking via `TargetScope::parse()` | Implemented with `is_private_ip()` | ✅ |
| FxHashMap: `AlertChannelsConfig.channels` | `settings.rs:21` uses `FxHashMap` | ✅ |
| FxHashMap: `WebhookConfigEntry.headers` | `settings.rs:38` uses `FxHashMap` | ✅ |
| FxHashMap: `HttpConfig.default_headers` | `http.rs:39` uses `FxHashMap` | ✅ |
| FxHashMap: `SlapperConfig.profiles` | `settings.rs:109` uses `FxHashMap` | ✅ |
| FxHashMap: `WebhookConfig.headers` | `scan.rs:132` uses `FxHashMap` | ✅ |
| `ConfigError` enum with Io/Parse/Serialize/Validation | `settings.rs:620-633` | ✅ |
| TOML/YAML support via `serde_yaml_neo` | `loader.rs:45-50` | ✅ |
| Proper error propagation via `?` | All async operations | ✅ |
| `PROJECT_QUALIFIER` usage in `api.rs` | `api.rs:1` uses correct constant | ✅ |

## Bugs Found

| Priority | Issue | Location |
|----------|-------|----------|
| P2 | Missing command-line override merging | `loader.rs:14-56` - `load_config()` only reads file, no CLI override support |
| P3 | Config search path incomplete | `loader.rs:93-115` - Only searches 4 paths; `--config` / `-c` argument not handled |
| P3 | `unwrap_or` instead of `expect` on split operations | `scope.rs:245, 301` - `split(':').next().unwrap_or(target)` never returns None |

## Recommended Fixes

### 1. Add CLI Override Support

The architecture states config supports "Merges file-based config with command-line overrides" but `load_config()` does not implement this.

```rust
// loader.rs - add a with_overrides method or merge CLI args
pub fn load_config(config_path: Option<&str>, cli_overrides: Option<CliOverrides>) -> Result<SlapperConfig>
```

### 2. Document the Config Search Order Discrepancy

The documented search order is:
1. `--config` / `-c` command-line argument
2. `./slapper.toml`
3. `./.slapper/slapper.toml`
4. `./config/slapper.toml`
5. `~/.config/slapper/slapper.toml`

The actual implementation (loader.rs:93-115) searches:
1. `base.join("slapper.toml")` 
2. `base.join(".slapper/slapper.toml")`
3. `base.join("config/slapper.toml")`
4. `~/.config/slapper/slapper.toml`

The `--config` / `-c` argument handling appears to be missing from `load_config()`.

## Discrepancies

| Item | Documented | Actual |
|------|-----------|--------|
| Config merging | "Merges file-based config with command-line overrides" | Only reads from file |
| CLI argument handling | `--config` / `-c` is first search path | Not implemented in `load_config()` |
| Default config path | `~/.config/slapper/slapper.toml` via `ProjectDirs` | Correctly implemented |

## Notes

- The private IP blocking in `TargetScope::parse()` and `parse_hostname_only()` correctly implements the documented security enforcement (lines 209-226 and 269-286 in `scope.rs`)
- `is_private_ip()` function correctly handles IPv4 and IPv6 private ranges
- Error handling is properly implemented with `Result` types and `?` propagation