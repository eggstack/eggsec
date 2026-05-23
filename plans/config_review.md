# Configuration System Architecture Review

## Verified Claims

### 1. Core Components Structure
**Claim**: SlapperConfig contains sub-configs: HttpConfig, ScanConfig, OutputConfig, NotificationConfig, PathsConfig, ProxyConfigEntry, AiConfig, SearchConfig, AlertChannelsConfig.

**Verification**: All present in `settings.rs:94-134`. Correctly implemented with serde flatten for PathsConfig (`settings.rs:112`).

### 2. Scope Security - Private IP Blocking
**Claim**: Direct IP addresses (e.g., `127.0.0.1`, `169.254.169.254`) are blocked via `TargetScope::parse()` and `parse_hostname_only()`.

**Verification**: Fully implemented in `scope.rs:202-260` and `scope.rs:262-308`. Both methods check:
- `ip.is_loopback()` at lines 210 and 270
- `is_private_ip(&ip)` at lines 216 and 276
- Link-local (169.254.x.x) is included in `is_private_ip()` at `scope.rs:347`

### 3. FxHashMap Usage
**Claim**: All HashMap usages use `rustc_hash::FxHashMap`.

**Verification**: All 5 documented locations use FxHashMap:
- `AlertChannelsConfig.channels` (`settings.rs:21`)
- `WebhookConfigEntry.headers` (`settings.rs:38`)
- `HttpConfig.default_headers` (`http.rs:39`)
- `SlapperConfig.profiles` (`settings.rs:109`)
- `WebhookConfig.headers` (`scan.rs:132`)

### 4. Configuration File Search Order
**Claim**: Slapper looks for config in order: `--config` arg, `./slapper.toml`, `./.slapper/slapper.toml`, `./config/slapper.toml`, `~/.config/slapper/slapper.toml`.

**Verification**: Implemented in `loader.rs:93-115` (`find_config_file`). The order matches:
1. `base.join(DEFAULT_CONFIG_NAME)` - ./slapper.toml
2. `base.join(".slapper").join(DEFAULT_CONFIG_NAME)` - ./.slapper/slapper.toml
3. `base.join("config").join(DEFAULT_CONFIG_NAME)` - ./config/slapper.toml
4. Then `config_dir()` - ~/.config/slapper/slapper.toml

Note: `--config` argument handled separately in `load_config()` at `loader.rs:15-18`.

### 5. ConfigError Variants
**Claim**: ConfigError has 4 variants: Io, Parse, Serialize, Validation.

**Verification**: Confirmed in `settings.rs:685-698`. All 4 variants present with proper thiserror derive.

### 6. TOML/YAML Support
**Claim**: Loader supports TOML (primary) and YAML (`.yaml`/`.yml`).

**Verification**: Implemented in `loader.rs:40-50` using `serde_yaml_neo` for YAML files.

---

## Discrepancies

### 1. `validate_url` Returns Inverted Result
**Doc Claim**: `validate_url(url)` returns `Result<bool, ScopeError>`, validates URL's host via `is_target_allowed`.

**Issue**: `validate_url` at `scope.rs:117-126` calls `is_target_allowed(host)` but returns its result directly. If `is_target_allowed` returns `Ok(true)`, the URL is valid. However, the method name `validate_url` suggests it validates and returns error on invalid, but it actually returns `Ok(false)` for out-of-scope targets - not an error.

**Code** (`scope.rs:117-126`):
```rust
pub fn validate_url(&self, url: &str) -> Result<bool, ScopeError> {
    let parsed = Url::parse(url)...;
    self.is_target_allowed(host)  // Returns Ok(false) for out-of-scope, not Err
}
```

This is a semantic inconsistency - `validate_*` methods typically return `Result<()>` or `Err` on invalid, not `Ok(false)`.

### 2. Scope Rule Matching - Prefix Wildcard Edge Case
**Doc Claim**: Excluded targets can block IPs or domains "that should never be touched".

**Issue**: In `scope.rs:183-186`, wildcard suffix matching:
```rust
if self.pattern.starts_with("*.") {
    let suffix = &self.pattern[1..];
    return target.host.ends_with(suffix) || target.host == self.pattern[2..];
}
```

For pattern `*.example.com`:
- `sub.example.com` matches (correct)
- `example.com` matches via `target.host == self.pattern[2..]` (correct)

But for pattern `*` (wildcard-only), line 171 returns true:
```rust
if self.pattern == "*" { return true; }
```

This means an excluded target with pattern `*` would block ALL targets. This is likely intended but undocumented.

### 3. `resolve_host` Failure is Non-Fatal
**Doc Claim**: Scope defines which targets are "in-scope" and which are "explicitly excluded".

**Code**: In `TargetScope::parse()` at `scope.rs:251-257`:
```rust
let ip = match Self::resolve_host(&host) {
    Ok(ip) => Some(ip),
    Err(e) => {
        tracing::debug!("DNS resolution failed for '{}': {}", host, e);
        None
    }
};
```

When DNS resolution fails, the target is still allowed if it matches a hostname pattern, with `ip: None`. This means scope checking with CIDR rules won't work for targets that fail DNS resolution. This is a significant limitation not documented.

### 4. Config File Permissions Check is Advisory Only
**Doc Claim**: "Config files with secrets should be `chmod 600` - `check_config_file_permissions()` warns about world/group-readable permissions but does not enforce."

**Verification**: Implemented correctly - `check_config_file_permissions` just logs a warning (see `types.rs:269`). However, this is a design decision, not a discrepancy. The documentation could be clearer that this is intentional security guidance rather than enforcement.

---

## Bugs Found

### 1. Private IP Check Includes Link-Local but Documentation Lists Only 3 Ranges
**Severity**: Medium

**Location**: `scope.rs:340-356`

**Issue**: The `is_private_ip` function checks:
- 10.0.0.0/8
- 172.16.0.0/12 (doc says 172.16-31, code uses 15-31 - correct per RFC)
- 192.168.0.0/16
- 169.254.0.0/16 (link-local, NOT documented)
- 127.0.0.0/8 (loopback)

The documentation at `config.md:32` only mentions `169.254.169.254` as an example but doesn't explicitly state this range is blocked. The `is_private_ip` function at line 347 explicitly includes link-local (`169.254.x.x`).

This is actually correct implementation but undocumented. The 169.254 range is important for AWS metadata endpoint blocking.

### 2. Missing Validation for `port_timeout_secs` in TargetScope
**Severity**: Low

**Location**: `scope.rs` (not present)

**Issue**: While `ScanConfig` has `port_timeout_secs` validation (`scan.rs:37`), there's no corresponding port timeout validation in `Scope`. The `Scope::is_port_allowed` method at `scope.rs:99-109` only checks allow/block lists, not timeouts.

---

## Improvement Opportunities

### 1. Add TTL/Cache Validation to Config
**Priority**: Medium

**Location**: `settings.rs` - AlertChannelsConfig

**Suggestion**: Alert channel configurations don't have validation for retry counts or timeout values. Consider adding validation similar to `HttpConfig::validate()`.

### 2. Improve Error Messages in Scope Matching
**Priority**: Low

**Location**: `scope.rs`

**Current**: When `is_target_allowed` returns `false`, it just logs a warning and returns `Ok(false)`. For debugging scope issues, it would be helpful to indicate *why* a target was rejected (e.g., matched an exclude rule vs. not matching any include rule).

### 3. Add Profile Merge Validation
**Priority**: Medium

**Location**: `settings.rs:566-578`

**Current**: Profile validation only checks name non-emptiness and sub-config validations. It doesn't detect conflicts between profile settings.

**Suggestion**: Add warnings for potentially conflicting settings when merging profiles.

### 4. Consider Adding Schema Validation
**Priority**: Low

**Current**: Config loading relies on serde deserialization with default values for missing fields.

**Suggestion**: Consider adding JSON Schema validation for config files to catch configuration errors early with clear error messages.

### 5. DNS Resolution Failure Should Fail Closed for CIDR Rules
**Priority**: Medium

**Location**: `scope.rs:58-97`

**Issue**: When a target fails DNS resolution and `has_ip_based_rules()` is true, the target uses `parse_hostname_only` which doesn't resolve DNS. This means CIDR-based rules won't match hostname targets that fail DNS.

**Suggestion**: When DNS resolution fails for a hostname target and CIDR rules exist, return an error rather than silently allowing the target.

### 6. Document `is_private_ip` RFC Ranges
**Priority**: Low

**Location**: `scope.rs:340-356`

**Suggestion**: Add comments explaining the RFC 1918 private address ranges and link-local addresses being blocked.

---

## Priority Summary

| Category | Item | Priority |
|----------|------|----------|
| **Discrepancy** | `validate_url` returns Ok(false) instead of Err for invalid scope | Low |
| **Discrepancy** | DNS failure non-fatal with CIDR rules | Medium |
| **Improvement** | DNS resolution failure should fail closed for CIDR rules | Medium |
| **Improvement** | Profile merge conflict detection | Medium |
| **Improvement** | AlertChannelsConfig validation | Medium |
| **Improvement** | Scope rejection reason reporting | Low |
| **Bug** | Link-local (169.254) blocking undocumented | Low |