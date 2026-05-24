# Configuration Module Architecture Review

## Overview

This review compares the architecture document `architecture/config.md` against the actual implementation in `crates/slapper/src/config/`. The review identifies verified claims, discrepancies, bugs, and improvement opportunities.

---

## Summary Statistics

| Metric | Count |
|--------|-------|
| Verified Claims | 24 |
| Discrepancies | 2 |
| Bugs Found | 0 |
| Improvement Opportunities | 1 |

---

## Verified Claims

### Core Configuration Structure

1. **`SlapperConfig` in `settings.rs`** - Main configuration struct holding all tool settings
   - Verified: Lines 95-134 define `SlapperConfig` with all documented sub-configs

2. **Sub-configs present** - `HttpConfig` (`http.rs:18-49`), `ScanConfig` (`scan.rs:16-44`), `OutputConfig` (`scan.rs:62-98`), `NotificationConfig` (`scan.rs:100-122`), `PathsConfig` (`settings.rs:79-92`), `ProxyConfigEntry` (`settings.rs:163-180`), `AiConfig` (`settings.rs:191-205`), `SearchConfig` (`settings.rs:219-232`), `AlertChannelsConfig` (`settings.rs:17-22`)

3. **`Scope` in `scope.rs`** - Target scope rules for security compliance
   - Verified: Lines 7-29 define `Scope` struct with `allowed_targets`, `excluded_targets`, `allowed_ports`, `excluded_ports`, `max_requests_per_second`, `require_explicit_scope`

### Scope Security Enforcement

4. **Private IP blocking** - Direct IP addresses blocked via `TargetScope::parse()` and `parse_hostname_only()`
   - Verified: `scope.rs:262-268` blocks private IPs in `parse()`, `scope.rs:316-322` blocks in `parse_hostname_only()`
   - `is_private_ip()` function at `scope.rs:374-390` checks:
     - IPv4: 10.x.x.x, 172.16-31.x.x, 192.168.x.x, 169.254.x.x, 127.x.x.x
     - IPv6: loopback, fc00::/7 (ULA), fe80::/10 (link-local)

5. **Scope enforcement methods** - `is_target_allowed()`, `validate_url()`, `is_port_allowed()`
   - Verified: `scope.rs:95-150`, `scope.rs:170-179`, `scope.rs:152-162`

6. **Smart optimization for non-CIDR scopes** - `Scope::is_target_allowed()` uses `parse_hostname_only()` when no IP-based rules exist (`scope.rs:95-107`)
   - Avoids unnecessary DNS resolution when only domain patterns are used

### FxHashMap Usage (Performance)

7. **All HashMap usages are `FxHashMap`**:
   - `AlertChannelsConfig.channels` (`settings.rs:21`)
   - `WebhookConfigEntry.headers` (`settings.rs:38`)
   - `HttpConfig.default_headers` (`http.rs:39`)
   - `SlapperConfig.profiles` (`settings.rs:109`)
   - `WebhookConfig.headers` (`scan.rs:132`)

### Configuration File Loading

8. **Config file search order** (`loader.rs:93-115`):
   - `--config` / `-c` argument (passed directly)
   - `./slapper.toml`
   - `./.slapper/slapper.toml`
   - `./config/slapper.toml`
   - `~/.config/slapper/slapper.toml` (via ProjectDirs)

9. **TOML and YAML support** (`loader.rs:40-50`)
   - TOML primary format
   - YAML detected by `.yaml`/`.yml` extension
   - Uses `serde_yaml_neo` for YAML parsing

### Error Handling

10. **`ConfigError` enum with four variants** (`settings.rs:685-698`):
    - `Io` - File read/write errors
    - `Parse` - TOML/YAML parsing errors
    - `Serialize` - Serialization errors
    - `Validation` - Validation failures

11. **`ScopeError` enum with seven variants** (`scope.rs:392-414`):
    - `Validation`, `FileRead`, `Parse`, `InvalidUrl`, `InvalidCidr`, `InvalidTarget`, `DnsResolution`

12. **Proper error propagation using `?`** - No `unwrap_or_default()` in production code
    - Verified: `scope.rs` uses `map_err` and `ok_or_else` throughout
    - `loader.rs` uses `map_err` for error conversion

### Validation

13. **`SlapperConfig::validate()`** orchestrates all sub-validations (`settings.rs:541-682`)
    - HTTP, scan, AI, search, proxy, profiles, paths all validated
    - Alert channels validated for each type (Webhook, Email, Slack, PagerDuty)

14. **`Scope::validate()`** checks (`scope.rs:36-71`):
    - At least one allowed target when `require_explicit_scope` is true
    - No duplicate ports in `allowed_ports`
    - `max_requests_per_second` between 1 and 10000 if specified

15. **`check_config_file_permissions()`** warns about world/group-readable permissions (`types.rs:269-303`)
    - Called after loading config (`loader.rs:53`) and scope (`loader.rs:89`)
    - Does NOT enforce - only warns

### Constants Usage

16. **All magic numbers from `constants.rs`**:
    - `DEFAULT_TIMEOUT_SECS = 30` (`http.rs:7`)
    - `DEFAULT_MAX_REDIRECTS = 10` (`http.rs:15`)
    - `DEFAULT_CONCURRENCY = 10` (`scan.rs:9`, `http.rs:10`)
    - `DEFAULT_REMOTE_PORT = 7890` (`settings.rs:66-67`)

### Project Qualifier

17. **`PROJECT_QUALIFIER` used consistently** (`constants.rs:7`)
    - `api.rs:8` uses `PROJECT_QUALIFIER` (documented as "fixed")
    - `settings.rs:534` uses `PROJECT_QUALIFIER`
    - `loader.rs:140,146,151` uses `PROJECT_QUALIFIER`

### ScopeRule Matching

18. **CIDR, wildcard, and exact pattern matching** (`scope.rs:214-245`)
    - CIDR from explicit `cidr` field
    - CIDR from pattern containing `/`
    - Wildcard `*` matches all
    - `*.example.com` suffix matching
    - Exact host matching

### URL Validation

19. **`validate_url()` parses URL and checks host** (`scope.rs:170-179`)
    - Uses `url::Url::parse()`
    - Extracts `host_str()` and passes to `is_target_allowed()`

### Config Save/Load

20. **`SlapperConfig::load()` and `save()` methods** (`settings.rs:518-530`)
    - Load uses `toml::from_str`
    - Save uses `toml::to_string_pretty`

### Default Configuration

21. **`get_default_config()` in `mod.rs:58-90`** generates default TOML template

### Scope::from_file()

22. **`Scope::from_file()` auto-detects format** (`scope.rs:73-86`)
    - `.yaml` or `.yml` extension uses `serde_yaml_neo`
    - Otherwise uses `toml::from_str`

### ProxyConfigEntry Validation

23. **Comprehensive proxy validation** (`settings.rs:278-332`)
    - Address non-empty
    - Port non-zero
    - Username requires password
    - `local_addr` must be valid IP
    - `weight` and `priority` must be non-zero if specified

### Alert Channel Validation

24. **`AlertChannelConfigEntry` enum with Webhook/Email/Slack/PagerDuty** (`settings.rs:24-31`)
    - Each type validated in `SlapperConfig::validate()` (`settings.rs:616-680`)

---

## Discrepancies

### 1. `Scope::validate()` not documented

**Severity**: Low (Documentation)

**Location**: `scope.rs:36-71`

**Details**: The architecture document does not mention `Scope::validate()` method, which was added (per `AGENTS.override.md:45-48`) to validate:
- At least one allowed target when `require_explicit_scope` is true
- No duplicate ports in `allowed_ports`
- `max_requests_per_second` between 1 and 10000 if specified

**Recommendation**: Update `architecture/config.md` to document the `Scope::validate()` method and its validation rules.

### 2. `ScopeRule::with_cidr()` not documented

**Severity**: Low (Documentation)

**Location**: `scope.rs:203-212`

**Details**: The architecture document does not mention `ScopeRule::with_cidr()` constructor method that creates a scope rule from a CIDR string.

**Recommendation**: Update `architecture/config.md` to document `ScopeRule::with_cidr()`.

---

## Bugs Found

**None found.** All claims verified, no actual bugs in the implementation.

---

## Improvement Opportunities

### 1. Document `has_ip_based_rules()` optimization

**Priority**: Low

**Estimated Impact**: Better code understanding

**Details**: The `Scope::is_target_allowed()` method (lines 95-107) has smart logic: it uses `TargetScope::parse()` (which resolves DNS) only when CIDR rules are present, otherwise it uses `parse_hostname_only()` as an optimization. This behavior is not documented.

**Recommendation**: Add a doc comment explaining the optimization behavior.

---

## Verification Commands

```bash
# Check config module compiles
cargo check --lib -p slapper

# Run config tests
cargo test --lib -p slapper -- config

# Run clippy on config module
cargo clippy --lib -p slapper -- -A clippy::all -W clippy::unused_self

# Run scope-specific tests
cargo test --lib -p slapper -- scope
```

---

## Conclusion

The configuration module is well-architected and matches the documentation closely:

1. **All claims verified** - The architecture document accurately describes the implementation
2. **Two undocumented features** - `Scope::validate()` and `ScopeRule::with_cidr()` exist but aren't documented
3. **No bugs found** - The implementation is correct
4. **One improvement opportunity** - Document the smart optimization in `is_target_allowed()`

The module follows good practices: proper error propagation, FxHashMap for performance, comprehensive validation, and consistent use of constants.