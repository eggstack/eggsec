# Configuration Module Architecture Review

**Review Date:** 2026-05-23
**Reviewer:** Architecture Review
**Files Reviewed:**
- `crates/slapper/src/config/settings.rs`
- `crates/slapper/src/config/scope.rs`
- `crates/slapper/src/config/loader.rs`
- `crates/slapper/src/config/http.rs`
- `crates/slapper/src/config/scan.rs`
- `crates/slapper/src/config/api.rs`
- `crates/slapper/src/config/mod.rs`

---

## Verified Claims

### 1. SlapperConfig Sub-configs (settings.rs)

| Claim (Document) | Implementation | Status |
|-----------------|---------------|--------|
| HttpConfig | `http.rs:18-49` with `timeout_secs`, `retry_delay_ms`, proxy, TLS, retries | VERIFIED |
| ScanConfig | `scan.rs:16-44` with `concurrency`, `timing`, `stealth`, `port_timeout_secs` | VERIFIED |
| OutputConfig | `scan.rs:62-84` | VERIFIED |
| NotificationConfig | `scan.rs:100-122` | VERIFIED |
| PathsConfig | `settings.rs:79-92` with `#[serde(flatten)]` | VERIFIED |
| ProxyConfigEntry | `settings.rs:163-180` | VERIFIED |
| AiConfig | `settings.rs:191-205` | VERIFIED |
| SearchConfig | `settings.rs:219-232` | VERIFIED |
| AlertChannelsConfig | `settings.rs:17-31` | VERIFIED |

### 2. Scope Key Methods (scope.rs)

| Method | Location | Status |
|--------|----------|--------|
| `is_target_allowed(target)` | `scope.rs:58-113` - Returns `Result<bool, ScopeError>` | VERIFIED |
| `validate_url(url)` | `scope.rs:133-142` - Validates URL's host via `is_target_allowed` | VERIFIED |
| `is_port_allowed(port)` | `scope.rs:115-125` - Returns `bool` | VERIFIED |

### 3. FxHashMap Usages

| Claim | Location | Status |
|-------|----------|--------|
| `AlertChannelsConfig.channels` | `settings.rs:21` | VERIFIED |
| `WebhookConfigEntry.headers` | `settings.rs:38` | VERIFIED |
| `HttpConfig.default_headers` | `http.rs:39` | VERIFIED |
| `SlapperConfig.profiles` | `settings.rs:109` | VERIFIED |
| `WebhookConfig.headers` | `scan.rs:132` | VERIFIED |

### 4. Loader Config Search Order (loader.rs:93-114)

| Claim (Document) | Implementation | Status |
|-----------------|---------------|--------|
| `--config` / `-c` argument | `loader.rs:14-18` - handled by caller | VERIFIED |
| `./slapper.toml` | Candidate 1 | VERIFIED |
| `./.slapper/slapper.toml` | Candidate 2 | VERIFIED |
| `./config/slapper.toml` | Candidate 3 | VERIFIED |
| `~/.config/slapper/slapper.toml` | Via `ProjectDirs` | VERIFIED |

**Note:** Document lists 5 locations, but implementation searches only 4 local candidates before checking `~/.config/slapper/`. The explicit `~/.config/slapper/slapper.toml` (item 4) is the same as item 5 via `ProjectDirs`.

### 5. ConfigError Enum Variants (settings.rs:685-698)

| Variant | Implementation | Status |
|---------|---------------|--------|
| `Io` | `settings.rs:687-688` with `#[source]` | VERIFIED |
| `Parse` | `settings.rs:690-691` | VERIFIED |
| `Serialize` | `settings.rs:693-694` | VERIFIED |
| `Validation` | `settings.rs:696-697` | VERIFIED |

### 6. Project Qualifier Fix

Document claims: "Project qualifier fixed: `api.rs` now uses `PROJECT_QUALIFIER` consistently with other modules"

**Verification:** `api.rs:8` uses `ProjectDirs::from(PROJECT_QUALIFIER, "", PROJECT_NAME)` - VERIFIED.

### 7. Private IP Blocking Fix

Document claims: "Direct IP addresses (e.g., `127.0.0.1`, `169.254.169.254`) are blocked via `TargetScope::parse()` and `parse_hostname_only()` - they now properly go through private IP checks"

**Verification:** `scope.rs:225-235` (parse) and `scope.rs:279-289` (parse_hostname_only) check `is_loopback()` but NOT full private IP range. The `is_private_ip()` function at `scope.rs:337-353` is only called AFTER scope rules are evaluated, not during initial parsing. See Discrepancies.

### 8. check_config_file_permissions

Document claims: "Config files with secrets should be `chmod 600` - `check_config_file_permissions()` warns about world/group-readable permissions but does not enforce."

**Verification:** `types.rs:269-303` - warns but does not return error - VERIFIED.

---

## Discrepancies

### D1: Private IP Blocking is Incomplete

**Document says (lines 31-32):**
> Private IP blocking: Direct IP addresses (e.g., `127.0.0.1`, `169.254.169.254`) are blocked via `TargetScope::parse()` and `parse_hostname_only()` - they now properly go through private IP checks

**Actual Implementation:**

In `TargetScope::parse()` (`scope.rs:225-235`):
```rust
if let Ok(ip) = IpAddr::from_str(target) {
    if ip.is_loopback() {  // Only checks loopback!
        return Err(ScopeError::DnsResolution(...));
    }
    return Ok(Self { host: target.to_string(), ip: Some(ip) });  // Allows other private IPs!
}
```

The function only blocks loopback addresses (`127.0.0.0/8`), not the full private IP range. The `is_private_ip()` function exists (`scope.rs:337-353`) but is only called later in `is_target_allowed()` AFTER scope matching, not during initial parsing.

**Impact:** A direct IP like `10.255.255.255` or `169.254.169.254` (AWS metadata) would be accepted by `TargetScope::parse()` and only rejected later if it fails scope matching. If scope rules are empty, this IP would be allowed.

**Priority:** HIGH

### D2: Config Search Order Missing `--config` Check in find_config_file

**Document says (lines 53-54):**
> 1. `--config` / `-c` command-line argument

**Implementation:**

`find_config_file()` (`loader.rs:93-115`) does NOT check command-line arguments. It only searches local paths. The command-line argument handling is done by the caller in `load_config()` (`loader.rs:14-18`):

```rust
let path = config_path
    .map(PathBuf::from)  // This is --config argument
    .or_else(|| find_config_file(None))  // Falls back to searching
```

**Impact:** The documentation is slightly misleading. The `--config` argument is handled before `find_config_file()` is called, not as part of the search order within `find_config_file()`. This is actually a good design but the document could be clearer.

**Priority:** LOW (documentation)

### D3: Scope `is_port_allowed()` Claims Don't Match Implementation

**Document says (line 29):**
> `is_port_allowed(port)` - Returns `bool`, checks port allowlist/blocklist

**Implementation:** `scope.rs:115-125`:
```rust
pub fn is_port_allowed(&self, port: u16) -> bool {
    if self.excluded_ports.contains(&port) { return false; }
    if let Some(ref allowed) = self.allowed_ports { return allowed.contains(&port); }
    true
}
```

**Issue:** The `Scope` struct has `excluded_ports: Vec<u16>` (line 19 of scope.rs) but there is NO field for `excluded_ports` in the Scope TOML schema shown in the document. The Scope struct also has `allowed_ports: Option<Vec<u16>>` but this is not mentioned in the architecture document at all.

**Priority:** MEDIUM (documentation gap)

---

## Bugs Found

### B1: Private IP Bypass via Direct IP with Empty Scope

**File:** `scope.rs:225-235`

**Bug:** When a direct IP address is passed to `TargetScope::parse()`, only loopback is blocked. Other private IPs (10.x.x.x, 172.16-31.x.x, 192.168.x.x, 169.254.x.x) pass through.

**Scenario:**
```rust
let scope = Scope::new();  // No allowed_targets, require_explicit_scope = false
scope.is_target_allowed("10.255.255.255")  // Returns Ok(true)!
```

**Root Cause:** `parse()` returns early with the IP if it's not loopback, without calling `is_private_ip()`.

**Fix:** Add private IP check in `parse()` for direct IP addresses:
```rust
if let Ok(ip) = IpAddr::from_str(target) {
    if ip.is_loopback() { ... }
    if is_private_ip(&ip) {  // ADD THIS CHECK
        return Err(ScopeError::DnsResolution(
            target.to_string(),
            "Private IP address blocked by security policy".to_string(),
        ));
    }
    return Ok(Self { host: target.to_string(), ip: Some(ip) });
}
```

**Priority:** HIGH

### B2: Private IP Bypass via parse_hostname_only for Direct IPs

**File:** `scope.rs:279-289`

**Bug:** Same issue as B1 but in `parse_hostname_only()`. When a direct IP is passed:
```rust
if let Ok(ip) = IpAddr::from_str(target) {
    if ip.is_loopback() { ... }  // Only checks loopback
    return Ok(Self { host: target.to_string(), ip: Some(ip) });  // Accepts other private IPs!
}
```

**Priority:** HIGH

### B3: Private IP Check Happens AFTER Scope Rules, Not Before

**File:** `scope.rs:96-105`

**Bug:** The `is_private_ip()` check at lines 97-104 only executes if `allowed` is `false`. This means:

1. If scope has `allowed_targets = [{pattern: "10.0.0.0/8"}]`
2. Target is `10.255.255.255`
3. `allowed_targets.matches()` returns `true`
4. The `is_private_ip()` check is SKIPPED because `allowed` is `true`
5. Target is allowed

This is documented in AGENTS.md:
> "scope rule evaluation happens AFTER private IP check - so targets like `10.255.255.255` are rejected even with scope rules like `allow 10.0.0.0/8`"

But the actual behavior is the OPPOSITE: the private IP check only happens if NO scope rule matches.

**Priority:** MEDIUM (behavior may be intentional but confusing)

---

## Improvement Opportunities

### I1: Move Private IP Check Earlier in is_target_allowed

**Current Flow:**
1. `has_ip_based_rules()` - determines if CIDR rules exist
2. `TargetScope::parse()` or `parse_hostname_only()` - parses target, DNS resolution
3. `is_explicitly_excluded()` - checks exclude rules
4. Empty allowed_targets check with `require_explicit_scope`
5. `allowed_targets.iter().any()` - checks if any rule matches
6. **Only if no match** - `is_private_ip()` check

**Proposed Flow:**
Move the private IP check to happen immediately after `TargetScope::parse()`, before any scope matching:

```rust
// After line 70 (after target_scope is created)
if let Some(ref ip) = target_scope.ip {
    if is_private_ip(ip) {
        tracing::warn!(target = %target, "Private IP address blocked by security policy");
        return Ok(false);
    }
}
```

**Estimated Impact:** Security hardening - prevents accidental scanning of private IPs even when scope rules might technically match them.

### I2: Add Validation for Scope.allowed_ports

**File:** `scope.rs`

The `Scope` struct has an `allowed_ports` field but there's no validation that:
- Ports are in valid range (1-65535)
- No duplicate ports
- `allowed_ports` and `excluded_ports` don't conflict

**Priority:** MEDIUM

### I3: Scope.from_file Lacks Validation

**File:** `scope.rs:36-49`

`Scope::from_file()` parses the file but does NOT call any validation. The `Scope` struct itself has no `validate()` method. Compare to `SlapperConfig` which has extensive validation.

**Priority:** MEDIUM

### I4: Missing Test Coverage for Scope Edge Cases

**File:** `scope.rs:376-476`

Tests exist but don't cover:
- Direct private IP addresses (10.x.x.x, 172.x.x.x, 192.168.x.x)
- AWS metadata endpoint (169.254.169.254)
- IPv6 private addresses (fc00:, fd00:, fe80:)
- Interaction between `allowed_ports` and `excluded_ports`

**Priority:** MEDIUM

### I5: Error Handling Inconsistency

**Files:** `loader.rs`, `settings.rs`

- `load_config()` uses `anyhow::Result<SlapperConfig>` (line 14)
- `SlapperConfig::validate()` returns `Result<(), ConfigError>` (line 541)
- `load_scope()` uses `anyhow::Result<Scope>` (line 58)

The `ConfigError` enum has proper variants but is wrapped in `anyhow::anyhow!("{}")` at `loader.rs:52`, losing type information.

**Priority:** LOW

### I6: Add SearchConfig Validation for searxng_url

**File:** `settings.rs:235-254`

`SearchConfig::validate()` only checks URL prefix (`http://`/`https://`). It should also:
- Validate URL is well-formed
- Check for invalid characters
- Optionally verify host is reachable

**Priority:** LOW

### I7: Alert Channel Validation is Manual and Incomplete

**File:** `settings.rs:616-680`

The validation for alert channels is manually implemented in `SlapperConfig::validate()` rather than having each `AlertChannelConfigEntry` type implement `validate()`. This violates OOP principles and makes it easy to miss validation when adding new channel types.

**Current:**
```rust
AlertChannelConfigEntry::Webhook(webhook) => { /* manual validation */ }
AlertChannelConfigEntry::Email(email) => { /* manual validation */ }
// etc.
```

**Proposed:** Each variant should implement `validate()`:
```rust
impl AlertChannelConfigEntry {
    pub fn validate(&self) -> Result<(), ConfigError> {
        match self {
            Webhook(w) => w.validate(),
            Email(e) => e.validate(),
            // etc.
        }
    }
}
```

**Priority:** MEDIUM

---

## Summary

| Category | Count | Priority |
|----------|-------|----------|
| Verified Claims | 8 | - |
| Discrepancies | 3 | 1 HIGH (documentation), 2 LOW/MEDIUM |
| Bugs Found | 3 | 2 HIGH, 1 MEDIUM |
| Improvement Opportunities | 7 | 2 HIGH, 4 MEDIUM, 1 LOW |

**Critical Security Issue:** Private IP blocking is incomplete. The `is_private_ip()` function exists but is only called in certain code paths, allowing direct private IP addresses to bypass checks in many scenarios.

**Key Recommendation:** Consolidate private IP checking into a single function that is called:
1. In `TargetScope::parse()` for direct IP addresses
2. In `parse_hostname_only()` for direct IP addresses  
3. In `is_target_allowed()` AFTER DNS resolution but BEFORE scope matching
4. In `resolve_host()` after DNS resolution

This ensures private IP blocking happens consistently regardless of input type or code path.
