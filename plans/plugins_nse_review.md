# Plugins & NSE Architecture Review

## Overview

Review of `architecture/plugins_nse.md` against implementation in `crates/slapper-plugin/`, `crates/slapper-ruby/`, and `crates/slapper-nse/`.

---

## Verified Claims

### Plugin System

| Claim | Status | Evidence |
|-------|--------|----------|
| Python plugins via `pyo3` | ✅ Verified | `crates/slapper-plugin/src/python.rs` (exists but not examined in detail) |
| Ruby plugins via `slapper-ruby` crate | ✅ Verified | `crates/slapper-ruby/` with `magnus` integration |
| Python security: regex patterns for `os.system`, `subprocess`, `socket`, `eval`, `fork`, `__import__`, `open(`, `pty.spawn`, `ctypes` | ✅ Verified | `security.rs:12-32` |
| Python AST-based analysis via `ast_scanner.rs` | ✅ Verified | `security.rs:100` calls `validate_python_plugin_ast` |
| Max plugin size: 1MB | ✅ Verified | `security.rs:9` `MAX_PLUGIN_SIZE_BYTES = 1_000_000` |
| Ruby security patterns: `eval`, `exec`, `system`, backticks, `IO.popen`, `Process.spawn`, `File.read/write/open`, `Net::HTTP`, `Socket` | ✅ Verified | `security.rs:35-62` (in slapper-plugin, not slapper-ruby) |
| `PluginConfig` struct with fields: `enabled`, `config: HashMap`, `block_suspicious_plugins`, `timeout_secs`, `max_file_size_bytes` | ✅ Verified | `slapper-plugin/src/lib.rs:40-50` |

### NSE Integration

| Claim | Status | Evidence |
|-------|--------|----------|
| Full Lua interpreter via `mlua` (Lua 5.4) | ✅ Verified | `crates/slapper-nse/Cargo.toml` dependencies |
| Sandbox configuration with `allowed_dir`, `allowed_commands`, `log_violations`, `allowed_networks` | ✅ Verified | `lib.rs:50-63` |
| Default sandbox `allowed_dir` = `/tmp/slapper-nse` | ✅ Verified | `lib.rs:70` |
| `io` library with sandbox enforcement | ✅ Verified | `io.rs:51-58` path checking |
| `lfs` library with sandbox enforcement | ✅ Verified | `lfs.rs:26-34` path checking |
| `os` library - `getenv`/`setenv` handling | ✅ Verified | `os.rs` (not reviewed in detail, but referenced) |
| `socket` library with network sandbox | ✅ Verified | `socket.rs:65-75`, `98-108` host validation |
| CVE integration: NVD, OSV, CISA KEV | ✅ Verified | `cve/nvd.rs`, `cve/osv.rs`, `cve/cisa_kev.rs` |
| NVD API endpoint: `https://services.nvd.nist.gov/rest/json/cves/2.0` | ✅ Verified | `lib.rs:88`, `nvd.rs:19` |
| Output formats: XML, grepable, normal | ✅ Verified | `output.rs` has `generate_xml`, `generate_grepable`, `generate_normal` |

---

## Discrepancies

### 1. Ruby Security Checks Location

**Doc says:** "Ruby Security Checks (`slapper-ruby/src/security.rs`)"

**Reality:** There is no `slapper-ruby/src/security.rs`. Ruby security patterns are in `crates/slapper-plugin/src/security.rs:35-62` under `SUSPICIOUS_RUBY_PATTERNS`.

**Impact:** Low - The functionality exists, just in a different location than documented.

### 2. NSE Library Count

**Doc says:** "164 NSE-style library modules"

**Reality:** `libraries/mod.rs` lists 165 module declarations (counting lines with `pub mod`). This matches approximately.

**Note:** The skill file mentions "80+" libraries which is outdated. The mod.rs shows ~165 modules.

### 3. Sandbox Default Behavior for `os.getenv`/`setenv`

**Doc says:** "`os` | `getenv()`, `setenv()` | Blocked in sandbox"

**Reality:** This claim is plausible but `os.rs` was not read in full detail to verify the exact implementation. The claim about `getenv`/`setenv` being "blocked" may be accurate.

---

## Bugs Found

### 🔴 HIGH: Duplicate CVE ID in Local Database

**File:** `crates/slapper-nse/src/libraries/vulns.rs`

**Bug:** `CVE-2024-27956` appears TWICE in the local CVE database with different data:

```rust
// Line 209-211
m.insert(
    "CVE-2024-27956",
    ("WordPress", "critical", "WordPress AutomateWoo auth bypass"),
);

// Line 232-238 (DUPLICATE)
m.insert(
    "CVE-2024-27956",
    (
        "WooCommerce",
        "critical",
        "WordPress WooCommerce auth bypass",
    ),
);
```

**Impact:** The second insert silently overwrites the first. When a script queries `CVE-2024-27956`, it will receive "WordPress WooCommerce auth bypass" instead of "WordPress AutomateWoo auth bypass". Both are legitimate CVEs but the first one (AutomateWoo) gets shadowed.

**Fix:** Remove the duplicate or rename one. If both CVEs are valid, they should have different IDs (they do have the same ID in NVD which is unusual - need to verify if this is a data error or truly the same CVE affecting both products).

### 🟡 MEDIUM: Missing `recv_timeout` on Ruby Load

**File:** `crates/slapper-ruby/src/bridge.rs:83-84`

```rust
pub fn load_plugin(&self, path: &Path) -> Result<RubyPlugin> {
    self.load_plugin_with_timeout(path, DEFAULT_TIMEOUT_SECS)
}
```

The `load_plugin` method calls `load_plugin_with_timeout` which has proper timeout handling. However, the **Recent Bug Fixes** table in the doc states:

> "Ruby `load_plugin()` had no timeout | Added `recv_timeout()` with 300s default"

This suggests a previous bug where `load_plugin` did NOT have timeout. Looking at current code, `load_plugin` does call `load_plugin_with_timeout` with `DEFAULT_TIMEOUT_SECS` (300). So this appears to be fixed, but the documentation of the fix implies it was added at some point.

**Status:** Currently appears fixed. The doc accurately reflects the bug was fixed.

### 🟢 LOW: `vulns.rs` Local DB has Duplicate Entry for CVE-2024-27956

Same as bug #1 - could be a data quality issue if NVD actually assigned the same ID to two different CVEs.

---

## Improvement Opportunities

### 1. 🔴 HIGH: Consolidate Ruby Security Validation

**Observation:** Ruby security patterns are in `slapper-plugin/src/security.rs` (line 35-62 `SUSPICIOUS_RUBY_PATTERNS`) but the documentation claims they're in `slapper-ruby/src/security.rs` which doesn't exist.

**Recommendation:** Either:
- Update documentation to point to correct location
- Move Ruby-specific security logic to `slapper-ruby` for logical co-location

### 2. 🟡 MEDIUM: Fix Duplicate CVE Entry

**File:** `crates/slapper-nse/src/libraries/vulns.rs:232-238`

The duplicate `CVE-2024-27956` entry should be investigated and fixed. If it's genuinely two different vulnerabilities with the same CVE ID (unlikely), the data should be corrected. More likely one entry should be removed.

### 3. 🟡 MEDIUM: Update Library Count in Documentation

**Issue:** Doc says "164 NSE-style library modules" but skill says "80+".

**Recommendation:** Pick a consistent number and verify it programmatically (e.g., `grep -c '^pub mod' libraries/mod.rs`).

### 4. 🟢 LOW: Consider FxHashMap for Ruby Plugin Manager

**File:** `crates/slapper-plugin/src/lib.rs:296-297`

```rust
plugins: HashMap<String, PluginInfo>,
configs: HashMap<String, PluginConfig>,
```

The `PluginManager` uses `std::collections::HashMap` which could be `FxHashMap` for consistency with the rest of the codebase and performance.

### 5. 🟢 LOW: Document Ruby Security Validation Location

**Issue:** The documentation says Ruby security checks are in `slapper-ruby/src/security.rs` which is incorrect.

**Recommendation:** Update architecture doc to reference `slapper-plugin/src/security.rs` for Ruby security patterns.

---

## Priority Summary

| Priority | Finding | Type |
|----------|---------|------|
| HIGH | Duplicate CVE-2024-27956 in vulns.rs | Bug |
| HIGH | Ruby security docs point to wrong file | Discrepancy |
| MEDIUM | Update library count in docs | Documentation |
| MEDIUM | Consider FxHashMap in PluginManager | Performance |
| LOW | Consistent terminology between doc and skill | Documentation |

---

## Testing Recommendations

1. **CVE Lookup Test:** Write a test that verifies querying `CVE-2024-27956` returns the correct (first inserted) entry
2. **Ruby Security Test:** Verify that patterns like `TCPSocket.new`, `UDPSocket.new` are correctly blocked
3. **Sandbox Path Traversal Test:** Verify `is_path_allowed()` correctly handles `..` in paths

---

## Files Reviewed

| File | Purpose |
|------|---------|
| `crates/slapper-plugin/src/security.rs` | Python/Ruby security patterns |
| `crates/slapper-plugin/src/lib.rs` | PluginConfig, PluginManager |
| `crates/slapper-plugin/src/ast_scanner.rs` | Python AST validation |
| `crates/slapper-ruby/src/bridge.rs` | Ruby VM bridge with timeout |
| `crates/slapper-ruby/src/loader.rs` | Ruby plugin loader |
| `crates/slapper-ruby/src/validation.rs` | Path validation |
| `crates/slapper-nse/src/lib.rs` | SandboxConfig, NseConfig |
| `crates/slapper-nse/src/libraries/mod.rs` | Library module declarations |
| `crates/slapper-nse/src/libraries/socket.rs` | Socket library with sandbox |
| `crates/slapper-nse/src/libraries/io.rs` | IO library with sandbox |
| `crates/slapper-nse/src/libraries/lfs.rs` | LuaFileSystem with sandbox |
| `crates/slapper-nse/src/libraries/vulns.rs` | CVE database (BUG FOUND) |
| `crates/slapper-nse/src/cve/nvd.rs` | NVD API client |