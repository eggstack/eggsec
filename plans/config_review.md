# Config Architecture Review

**Document:** architecture/config.md
**Reviewed:** 2026-05-31
**Accuracy:** High

## Verified Claims

- **SlapperConfig struct exists**: Verified at `crates/slapper/src/config/settings.rs:92-138`
- **Sub-configs listed**: `HttpConfig` (line 95), `ScanConfig` (line 98), `OutputConfig` (line 101), `NotificationConfig` (line 104), `PathsConfig` (line 109-110, flattened), `ProxyConfigEntry` (line 122), `AiConfig` (line 125), `SearchConfig` (line 128), `AlertChannelsConfig` (line 131) — all verified in `settings.rs`
- **Scope struct and methods**: `is_target_allowed()` at `scope.rs:100`, `validate_url()` at `scope.rs:188`, `is_port_allowed()` at `scope.rs:170`, `validate()` at `scope.rs:42` — all match documentation
- **ScopeRule::new() and ScopeRule::with_cidr()**: Verified at `scope.rs:213` and `scope.rs:221`
- **Private IP blocking**: `TargetScope::parse()` at `scope.rs:273` and `parse_hostname_only()` at `scope.rs:322` both defer to `is_target_allowed()` which calls `is_private_ip()` at `scope.rs:382`
- **FxHashMap usages**: `AlertChannelsConfig.channels` at `settings.rs:22` (doc says line 21, off by 1), `WebhookConfigEntry.headers` at `settings.rs:39` (doc says line 38, off by 1), `HttpConfig.default_headers` at `http.rs:39`, `SlapperConfig.profiles` at `settings.rs:107` (doc says line 109, off by 2), `WebhookConfig.headers` at `scan.rs:132`
- **ConfigError enum**: Four variants (`Io`, `Parse`, `Serialize`, `Validation`) verified at `settings.rs:697-710`
- **Config file search order**: `find_config_file()` at `loader.rs:93-115` checks `./slapper.toml`, `./.slapper/slapper.toml`, `./config/slapper.toml`, and `~/.config/slapper/slapper.toml` via `ProjectDirs` — matches doc
- **Loader supports TOML and YAML**: Verified at `loader.rs:40-50`
- **check_config_file_permissions**: Referenced at `loader.rs:53` and `loader.rs:89` — doc claim that it warns but does not enforce is correct (function is in `types.rs:269`)
- **PROJECT_QUALIFIER in api.rs**: Verified at `api.rs:1` and `api.rs:8`
- **TUI Settings Tab preserved sections**: Doc lists `profiles`, `schedule`, `remote`, `ai`, `search`, `alert_channels` — the `SlapperConfig` struct has all these fields

## Discrepancies

- **FxHashMap line references**: Document claims `AlertChannelsConfig.channels` at `settings.rs:21` but actual is line 22; `WebhookConfigEntry.headers` at `settings.rs:38` but actual is line 39; `SlapperConfig.profiles` at `settings.rs:109` but actual is line 107. Minor line-number drift.
- **Scope::validate() max_requests_per_second range**: Doc says "must be greater than 0 if set" but actual code at `scope.rs:62-73` also checks `rate > 10000` (exceeds reasonable limit). Doc omits the upper bound check.
- **Missing sub-configs from doc**: `ReconConfig` (line 113), `RemoteConfig` (line 119), `ExecutionPolicy` (line 134), `CacheConfig`, `ScheduledScan` are present in `SlapperConfig` but not listed in the doc's "Sub-configs" section. The doc lists 9 sub-configs but the struct has ~14 fields.
- **Missing ConfigError variants**: Doc says four variants. Actual `ConfigError` at `settings.rs:697-710` has exactly four (`Io`, `Parse`, `Serialize`, `Validation`). However, there is also a separate `ScopeError` enum at `scope.rs:400-422` with six variants. The doc does not mention `ScopeError` at all.
- **SlapperConfig has additional fields not in doc**: `recon`, `schedule`, `remote`, `execution_policy`, `auto_save_interval_secs` are in the struct but not listed in the doc's Sub-configs section.

## Bugs Found

- None identified in the documentation vs. codebase comparison.

## Improvement Opportunities

- **Add ReconConfig, RemoteConfig, ExecutionPolicy to Sub-configs list**: The doc's Sub-configs section should include all 14+ fields of SlapperConfig for completeness. (priority: medium)
- **Document ScopeError enum**: The doc mentions ConfigError but not ScopeError, which is a separate error type with its own variants used by scope-related functions. (priority: medium)
- **Update FxHashMap line references**: Correct line numbers to match current code (settings.rs:22, settings.rs:39, settings.rs:107). (priority: low)
- **Document max_requests_per_second upper bound**: Add that the value must be <= 10000. (priority: low)
- **Document execution_policy field**: The `ExecutionPolicy` field exists on `SlapperConfig` (settings.rs:134) but is not mentioned in the doc. (priority: medium)

## Stale Items

- **Line number references (FxHashMap section)**: The line numbers cited for FxHashMap usages have drifted by 1-2 lines since the doc was written. Not a functional issue but reduces traceability. Recommend updating to current line numbers.
- **"Key Security Fixes (2026-05-22)" section**: The security fixes described are already merged and stable. This section could be condensed or moved to a changelog to keep the architecture doc focused on current state.
