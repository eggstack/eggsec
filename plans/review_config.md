# Config Module Architecture Review

**Document:** architecture/config.md
**Reviewed:** 2026-05-31
**Accuracy:** High
**Lines Reviewed:** 110

## Verified Claims

- **SlapperConfig struct** (`settings.rs:93`): Main configuration struct with `#[derive(Debug, Clone, Serialize, Deserialize, Default)]` at `crates/slapper/src/config/settings.rs:93`
- **HttpConfig sub-config**: Re-exported at `settings.rs:2` via `pub use super::http::HttpConfig`
- **ScanConfig sub-config**: Re-exported at `settings.rs:3` via `pub use super::scan::ScanConfig`
- **OutputConfig sub-config**: Re-exported at `settings.rs:3` via `pub use super::scan::OutputConfig`
- **NotificationConfig sub-config**: Re-exported at `settings.rs:3` via `pub use super::scan::NotificationConfig`
- **PathsConfig with `#[serde(flatten)]`**: Verified at `settings.rs:109-110`
- **ProxyConfigEntry**: Defined at `settings.rs:172-188`
- **AiConfig**: Defined at `settings.rs:199-213`
- **SearchConfig**: Defined at `settings.rs:227-240`
- **AlertChannelsConfig**: Defined at `settings.rs:19-23`
- **ReconConfig**: Defined at `settings.rs:190-197`
- **RemoteConfig**: Defined at `settings.rs:144-154`
- **ExecutionPolicy**: Re-exported from `policy.rs` at `mod.rs:46`
- **Scope struct** (`scope.rs`): Verified at `crates/slapper/src/config/scope.rs:7-29`
- **Scope::is_target_allowed()**: Returns `Result<bool, ScopeError>` at `scope.rs:100`
- **Scope::validate_url()**: Returns `Result<bool, ScopeError>` at `scope.rs:188`
- **Scope::is_port_allowed()**: Returns `bool` at `scope.rs:170`
- **Scope::validate()**: Orchestrates scope validation at `scope.rs:42-76`
- **ScopeRule::new(pattern)**: Creates rule from string at `scope.rs:213-219`
- **ScopeRule::with_cidr(cidr)**: Creates rule from CIDR at `scope.rs:221-230`
- **Private IP blocking**: Implemented via `is_private_ip()` at `scope.rs:382-398` and called from `is_target_allowed()` at `scope.rs:131-143`
- **FxHashMap in AlertChannelsConfig.channels**: Verified at `settings.rs:22`
- **FxHashMap in WebhookConfigEntry.headers**: Verified at `settings.rs:39`
- **FxHashMap in HttpConfig.default_headers**: Verified at `http.rs:39`
- **FxHashMap in SlapperConfig.profiles**: Verified at `settings.rs:107`
- **FxHashMap in WebhookConfig.headers**: Verified at `scan.rs:132`
- **Loader supports TOML and YAML**: Verified at `loader.rs:40-50`
- **Config file search order**: All 5 paths verified at `loader.rs:93-115` (matches doc order exactly)
- **ConfigError enum has 4 variants**: Io, Parse, Serialize, Validation at `settings.rs:697-710`
- **ScopeError enum has 7 variants**: All 7 variants verified at `scope.rs:400-422`
- **ScopeError line range `scope.rs:400-422`**: Exact match verified
- **check_config_file_permissions warns but does not enforce**: Confirmed at `loader.rs:53,89` calling `types::check_config_file_permissions`
- **TUI settings tab preserves non-exposed sections**: Verified in `tui/tabs/settings/main.rs:419-464` -- the `to_config()` method clones existing config before applying UI field changes
- **HttpConfig sub-config includes retry_delay_ms**: Verified at `http.rs:27`
- **ScanConfig sub-config includes port_timeout_secs**: Verified at `scan.rs:37`

## Discrepancies

- **Line number for SlapperConfig.profiles**: Documented as `settings.rs:109`, actual is `settings.rs:107` (minor off-by-2, likely due to added field)
- **Scope::validate() max_requests_per_second upper bound**: Document says "must be greater than 0 if set" but omits the upper bound check of 10000 (`scope.rs:68-70`). The doc should mention both bounds.

## Bugs Found

- No bugs found in the architecture document.

## Improvement Opportunities

- **[Item]: Document max_requests_per_second upper bound (10000)**: The `Scope::validate()` description at line 33 omits the `rate > 10000` check. This is a security-relevant limit. (priority: medium)
- **[Item]: Document ScopeError derive attribute**: The ScopeError uses `#[derive(Debug, thiserror::Error)]` (not just `thiserror::Error`). Adding this detail helps developers understand the error display behavior. (priority: low)
- **[Item]: Document `Scope::from_file()` method**: The scope module has a `from_file()` method at `scope.rs:78-91` that supports TOML and YAML loading, which is not mentioned in the architecture doc. (priority: low)
- **[Item]: Document `TargetScope` struct**: The `TargetScope` struct at `scope.rs:266-270` is a key internal type used by scope matching but is not described in the architecture doc. (priority: low)
- **[Item]: Document `is_private_ip()` function scope**: The private IP check at `scope.rs:382-398` covers IPv4 (10.x, 172.16-31.x, 192.168.x, 169.254.x, 127.x) and IPv6 (loopback, ULA fc00::/7, link-local fe80::/10). This detail is useful for security review. (priority: low)

## Stale Items

- **[Key Security Fixes (2026-05-22)] section**: The "Private IP bypass fixed" and "Project qualifier fixed" items at lines 106-109 reference specific dates. These are now historical and could be moved to a changelog or marked as "verified fixed" to indicate they are confirmed resolved. (priority: low)
