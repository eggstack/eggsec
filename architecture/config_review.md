# Configuration Module Review - Improvement Plan

## Summary

The architecture document (`architecture/config.md`) describes a configuration system handling:
- Main configuration via `SlapperConfig` loaded from TOML/YAML files
- Target scope enforcement with private IP blocking
- Sub-configs: `HttpConfig`, `ScanConfig`, `OutputConfig`, `NotificationConfig`, etc.
- Config file discovery in standard locations
- Validation and error handling via `ConfigError` enum

## Verification of Key Claims

### Verified Correct

| Claim | Status | Implementation |
|-------|--------|----------------|
| `SlapperConfig` is main config | VERIFIED | `settings.rs:94-134` |
| TOML/YAML support | VERIFIED | `loader.rs:40-50` |
| Config file precedence | VERIFIED | `loader.rs:93-114` |
| `ConfigError` with 4 variants | VERIFIED | `settings.rs:620-633` |
| Private IP blocking in `parse()` | VERIFIED | `scope.rs:209-226` |
| Private IP blocking in `parse_hostname_only()` | VERIFIED | `scope.rs:269-285` |
| `PROJECT_QUALIFIER` usage | VERIFIED | `api.rs:8`, `loader.rs:8,140,146,151` |
| `AlertChannelsConfig.channels` uses `FxHashMap` | VERIFIED | `settings.rs:21` |
| `WebhookConfigEntry.headers` uses `FxHashMap` | VERIFIED | `settings.rs:38` |
| `HttpConfig.default_headers` uses `FxHashMap` | VERIFIED | `http.rs:39` |
| `WebhookConfig.headers` uses `FxHashMap` | VERIFIED | `scan.rs:132` |

### Discrepancy Found

| Claim | Expected | Actual |
|-------|----------|--------|
| `ScanConfig.profiles` uses `FxHashMap` | Document states this (config.md:40, AGENTS.override.md:35) | `ScanConfig` does not have a `profiles` field. `SlapperConfig.profiles` uses `FxHashMap` at `settings.rs:109` |

**Impact**: Documentation over-reaches slightly. The `profiles` field exists on `SlapperConfig`, not `ScanConfig`. Minor documentation error.

---

## Bugs Found

### 1. Documentation Inaccuracy: `ScanConfig.profiles` Does Not Exist

**Files**: `architecture/config.md:40`, `AGENTS.override.md:35`

**Issue**: The architecture document claims `ScanConfig.profiles` uses `FxHashMap`. However, `ScanConfig` struct (`scan.rs:16-44`) has no `profiles` field. The `profiles` field is on `SlapperConfig` (`settings.rs:109`).

**Fix**: Update documentation to reference `SlapperConfig.profiles` instead.

---

## Performance Issues

### 1. All HashMap Usages Already Use FxHashMap

**Status**: RESOLVED

The config module correctly uses `FxHashMap` for all collections:
- `settings.rs:21` - `AlertChannelsConfig.channels`
- `settings.rs:38` - `WebhookConfigEntry.headers`
- `settings.rs:109` - `SlapperConfig.profiles`
- `http.rs:39` - `HttpConfig.default_headers`
- `scan.rs:132` - `WebhookConfig.headers`

No performance issues found in HashMap/HashSet usage.

---

## Error Handling Analysis

### Properly Handled Cases

| Location | Pattern | Assessment |
|----------|---------|------------|
| `scope.rs:58-97` | `is_target_allowed()` returns `Result<bool, ScopeError>` | Correct |
| `scope.rs:117-126` | `validate_url()` returns `Result<bool, ScopeError>` | Correct |
| `scope.rs:202-260` | `TargetScope::parse()` returns `Result<Self, ScopeError>` | Correct |
| `scope.rs:262-308` | `parse_hostname_only()` returns `Result<Self, ScopeError>` | Correct |
| `scope.rs:310-337` | `resolve_host()` returns `Result<IpAddr, ScopeError>` | Correct |
| `settings.rs:541-617` | `SlapperConfig::validate()` returns `Result<(), ConfigError>` | Correct |
| `settings.rs:235-254` | `SearchConfig::validate()` returns `Result<(), ConfigError>` | Correct |
| `settings.rs:278-332` | `ProxyConfigEntry::validate()` returns `Result<(), ConfigError>` | Correct |
| `settings.rs:334-380` | `HttpConfig::validate()` returns `Result<(), ConfigError>` | Correct |
| `settings.rs:382-433` | `ScanConfig::validate()` returns `Result<(), ConfigError>` | Correct |
| `settings.rs:466-506` | `AiConfig::validate()` returns `Result<(), ConfigError>` | Correct |
| `scan.rs:150-177` | `WebhookConfig::validate()` returns `Result<(), ConfigError>` | Correct |
| `loader.rs:52` | `config.validate().map_err(...)` | Correct |
| `loader.rs:53` | `check_config_file_permissions()` logs warning only | Acceptable (advisory, not enforced) |

---

## unwrap/expect Calls Analysis

### Test Code (Acceptable)

All `unwrap()` and `expect()` calls in the config module are in test code (`#[cfg(test)]`):
- `scope.rs:408,409,419,428,437,447,451,460,470,471,472,473`
- `settings.rs:654`
- `loader.rs:207,231,233,250,252,268,270,291,293,307,309,320,327,341`

These are acceptable as they operate on known test fixtures.

### Non-Test unwrap/expect Analysis

**NONE FOUND** in the config module outside of tests. The module correctly uses `?` propagation and explicit error handling.

---

## Recommendations

### 1. Fix Documentation: `ScanConfig.profiles` Reference

**Files to update**:
- `/Users/davidbowman/projects/slapper/architecture/config.md:40` - change `ScanConfig.profiles` to `SlapperConfig.profiles`
- `/Users/davidbowman/projects/slapper/crates/slapper/src/config/AGENTS.override.md:35` - change `ScanConfig.profiles` to `SlapperConfig.profiles`

**Current**:
```markdown
- `ScanConfig.profiles` (`settings.rs:109`)
```

**Should be**:
```markdown
- `SlapperConfig.profiles` (`settings.rs:109`)
```

---

### 2. Consider Adding ScanProfile Validation

**File**: `settings.rs:572-577`

**Current**:
```rust
if let Some(ref http) = profile.http {
    http.validate()?;
}
if let Some(ref scan) = profile.scan {
    scan.validate()?;
}
```

**Observation**: `ScanProfile::validate()` does not exist. The validation only calls nested config validation but doesn't validate `ScanProfile` fields like `name`. The iteration at `settings.rs:566-577` checks if `name.is_empty()`, which is correct.

**Status**: Already handled correctly. No change needed.

---

### 3. Consider Making `check_config_file_permissions` Fail Loudly

**File**: `types.rs:269-289`, `loader.rs:53,89`

**Current**: `check_config_file_permissions()` only logs a warning for world-readable files.

**Observation**: The architecture document (line 62) states: "Config files with secrets should be `chmod 600` - `check_config_file_permissions()` warns about world/group-readable permissions but does not enforce."

This is intentional design, but if stricter security is desired, this could be made configurable.

**Recommendation**: Keep current behavior (advisory warning) but consider adding a `--strict-permissions` CLI flag to make it an error.

---

## Summary

The configuration module is well-implemented with:
- Proper error handling via `Result` types and the `?` operator
- Correct use of `FxHashMap` for performance
- Private IP blocking properly implemented
- Good validation coverage
- Minor documentation inaccuracy regarding `ScanConfig.profiles` that should be corrected

**No critical bugs found. The module follows the documented architecture correctly.**
