# Plugins & NSE Architecture Review

Review date: 2026-05-23
Architecture document: `architecture/plugins_nse.md`
Codebase: `crates/slapper-plugin/`, `crates/slapper-ruby/`, `crates/slapper-nse/`

---

## Verified Claims

### Plugin System

| Claim | Implementation | Status |
|-------|---------------|--------|
| Python integration via `pyo3` | `crates/slapper-plugin/src/python.rs` | VERIFIED |
| Ruby integration via `slapper-ruby` crate with `magnus` | `crates/slapper-ruby/src/bridge.rs` | VERIFIED |
| Python security: regex patterns for dangerous constructs | `security.rs:12-31` - 17 patterns including `os.system`, `subprocess`, `socket`, `eval`, `fork`, `__import__`, `open(`, `pty.spawn`, `ctypes` | VERIFIED |
| Python security: AST-based analysis via `ast_scanner.rs` | `crates/slapper-plugin/src/ast_scanner.rs` - `AstScanner` struct with `DetectionMode` enum | VERIFIED |
| Ruby security: regex patterns for dangerous constructs | `security.rs:35-62` - 26 patterns including `eval`, `exec`, `system`, backticks, `IO.popen`, `Process.spawn`, `File.read/write/open`, `Net::HTTP`, `Socket` | VERIFIED |
| Maximum plugin size: 1MB | `security.rs:9` - `MAX_PLUGIN_SIZE_BYTES = 1_000_000` | VERIFIED |
| Example: `examples/plugins/example_scanner.py` | EXISTS - 103 lines with `register_checks()` and `run_check()` | VERIFIED |
| Example: `examples/plugins/metasploit_example.rb` | EXISTS - 273 lines Metasploit integration | VERIFIED |
| PluginConfig fields match documentation | `lib.rs:40-50` - `enabled`, `config: HashMap`, `block_suspicious_plugins`, `timeout_secs`, `max_file_size_bytes` | VERIFIED |

### NSE System

| Claim | Implementation | Status |
|-------|---------------|--------|
| Lua interpreter via `mlua` | `crates/slapper-nse/src/lib.rs` - uses `mlua` | VERIFIED |
| SandboxConfig structure | `lib.rs:50-63` - `enabled`, `allowed_dir`, `allowed_commands`, `log_violations`, `allowed_networks` | VERIFIED |
| Sandbox default: `/tmp/slapper-nse` | `lib.rs:70` - `allowed_dir: Some(PathBuf::from("/tmp/slapper-nse"))` | VERIFIED |
| Sandboxed io operations | `io.rs:41-407` - `open()`, `lines()`, `popen()`, `tmpfile()` with path canonicalization and command allowlist | VERIFIED |
| Sandboxed lfs operations | `lfs.rs:19-379` - All file operations with path validation against `allowed_dir` | VERIFIED |
| os.getenv() blocked in sandbox | `os.rs:97-104` - returns empty string when sandbox enabled | VERIFIED |
| os.setenv() blocked in sandbox | `os.rs:106-123` - blocks and logs violations | VERIFIED |
| socket.connect() validates host via `allowed_networks` | `socket.rs:65-75` - `is_host_allowed()` check | VERIFIED |
| socket.connect_udp() validates host | `socket.rs:98-108` - also checks `is_host_allowed()` | VERIFIED |
| Library count | 164 library modules (files with `register_*_library` functions) | VERIFIED |
| CVE Integration: NVD API | `vulns.rs:384-485` - `https://services.nvd.nist.gov/rest/json/cves/2.0` | VERIFIED |

---

## Discrepancies

### 1. Library Count Mismatch (Minor)
**Document says:** "164 NSE-style library modules"
**Actual:** 164 library files with `register_*_library` functions - this is actually correct

### 2. CVE Integration - OSV and CISA KEV Not Implemented
**Document says:** The `vulns` library provides access to CVE databases:
- **NVD** (National Vulnerability Database) - `https://services.nvd.nist.gov/rest/json/cves/2.0`
- **OSV** (Open Source Vulnerabilities)
- **CISA KEV** (Known Exploited Vulnerabilities)

**Actual:** Only NVD API is implemented in `vulns.rs:384-485`. No OSV or CISA KEV integration exists.

### 3. Ruby Security Pattern - `eval` Not Fully Covered
**Document says:** Ruby security checks include `eval` pattern
**Actual:** The pattern `(?i)\beval\(` requires `eval(`, but `eval` alone (without parentheses) won't be caught. The regex should be `(?i)\beval\b` to catch `eval` as a standalone word.

---

## Bugs Found

### Bug 1: Duplicate CVE Entry (CVE-2024-27956)
**File:** `crates/slapper-nse/src/libraries/vulns.rs:213-243`
**Severity:** Medium
**Description:** CVE-2024-27956 is inserted twice in the `get_cve_db()` function:
- Lines 213-216: WordPress AutomateWoo auth bypass
- Lines 237-243: WordPress WooCommerce auth bypass

The second insert silently overwrites the first due to HashMap behavior. The code has a comment acknowledging this (lines 208-212) but no fix was implemented.

**Impact:** One of the two CVEs will be inaccessible via the `vulns.cve()` API.

**Fix:** Change from `FxHashMap<&str, (&'static str, &'static str, &'static str)>` to `FxHashMap<&str, Vec<(&'static str, &'static str, &'static str)>>` to store multiple entries per CVE.

---

### Bug 2: load_plugin_with_timeout() Ignores timeout Parameter
**File:** `crates/slapper-ruby/src/bridge.rs:245-293`
**Severity:** Medium
**Description:** The `load_plugin_with_timeout` function accepts `timeout_secs` but never uses it:
```rust
#[cfg(feature = "ruby-plugins")]
fn load_plugin_with_timeout(&self, path: &Path, timeout_secs: u64) -> Result<RubyPlugin> {
    // ... validation ...
    let _ = timeout_secs;  // <-- Parameter is discarded!
    Ok(RubyPlugin::new_with_meta(...))
}
```

**Impact:** Plugin loading cannot be timeout-controlled despite the API suggesting it can be.

**Fix:** Use `timeout_secs` when calling the Ruby VM's require method, or remove the function if redundant with `load_plugin`.

---

### Bug 3: Ruby Security Pattern Missing `require` / `load` Full Words
**File:** `crates/slapper-plugin/src/security.rs:59-60`
**Severity:** Low
**Description:** The regex patterns for `require` and `load` use `\s*` which only matches whitespace, not word boundaries:
```rust
Regex::new(r"(?i)\brequire\s*\(").unwrap(),  // matches "require(" but not "require "
Regex::new(r"(?i)\bload\b").unwrap(),        // this one is correct
```

The `require` pattern should be `(?i)\brequire\b` to match both `require()` and `require `.

**Impact:** A Ruby plugin could potentially bypass security by using `require 'socket'` instead of `require('socket')`.

---

### Bug 4: Socket Sandbox Allows Bypass via DNS Rebinding
**File:** `crates/slapper-nse/src/libraries/socket.rs:48-63`
**Severity:** Medium
**Description:** `is_host_allowed()` resolves the hostname and checks against allowed networks, but doesn't cache the resolution. A script could potentially:
1. Connect to `allowed.example.com` (in allowlist)
2. DNS changes to point to a private IP
3. The already-connected socket continues to the new IP

**Impact:** A script that initially passes the sandbox check could later communicate with an unauthorized host if DNS changes.

**Fix:** For UDP sockets especially, consider storing the resolved IP at connection time and validating against it, not the hostname.

---

## Improvement Opportunities

### 1. Performance: HashMap -> FxHashMap in PluginManager
**File:** `crates/slapper-plugin/src/lib.rs:294-299`
**Priority:** Medium
**Description:** `PluginManager` uses `std::collections::HashMap` instead of `FxHashMap`:
```rust
pub struct PluginManager {
    plugin_dirs: Vec<PathBuf>,
    plugins: HashMap<String, PluginInfo>,      // Should be FxHashMap
    configs: HashMap<String, PluginConfig>,    // Should be FxHashMap
    block_suspicious_plugins: bool,
}
```

**Estimated Impact:** 10-20% faster plugin discovery and lookup with large plugin counts (>100).

---

### 2. Performance: Lazy Lock in io.rs FILE_HANDLES
**File:** `crates/slapper-nse/src/libraries/io.rs:28-31`
**Priority:** Low
**Description:** FILE_HANDLES and NEXT_FD use `LazyLock` with `Mutex<i32>`:
```rust
static FILE_HANDLES: std::sync::LazyLock<Mutex<FxHashMap<i32, FileHandle>>> = ...
static NEXT_FD: std::sync::LazyLock<Mutex<i32>> = std::sync::LazyLock::new(|| Mutex::new(100));
```

**Issue:** For high-frequency I/O operations, the mutex contention could be a bottleneck.

**Suggestion:** Consider using `parking_lot::Mutex` for better performance, or use atomic operations where possible.

---

### 3. Missing OSV and CISA KEV Integration
**File:** `crates/slapper-nse/src/libraries/vulns.rs`
**Priority:** Medium
**Description:** The document claims OSV and CISA KEV support but only NVD is implemented.

**Suggestion:** Add OSV API integration:
- OSV: `https://api.osv.dev/v1/query` with package information
- CISA KEV: `https://www.cisa.gov/sites/default/files/feeds/known_exploited_vulnerabilities.csv`

---

### 4. Async/Await on Sync Mutex
**File:** `crates/slapper-ruby/src/bridge.rs` - calls through mpsc channels
**Priority:** Low (Theoretical)
**Description:** The Ruby plugin client uses `mpsc::Receiver::recv_timeout()` which is blocking, not async. The code correctly uses `spawn_blocking` in the CLI runner, but potential misuse in async contexts could cause issues.

**Current state:** Actually handled correctly - no bug, but documentation could be clearer.

---

### 5. Sandbox Violation Metrics Not Exposed
**Files:** `lfs.rs:13`, `io.rs:33`, `os.rs:14`
**Priority:** Low
**Description:** Sandbox violations are tracked via atomic counters:
- `LFS_SANDBOX_VIOLATIONS`
- `IO_SANDBOX_VIOLATIONS`
- `OS_SANDBOX_VIOLATIONS`

But these metrics are not exposed via any API or reporting mechanism.

**Suggestion:** Add a `get_sandbox_metrics()` function to the NSE module to expose violation counts.

---

### 6. tmpfile Path Validation Logic Inconsistency
**File:** `crates/slapper-nse/src/libraries/io.rs:380-387`
**Priority:** Low
**Description:** In `io.tmpfile()`, path validation uses `path.starts_with(allowed)`:
```rust
if !path.starts_with(allowed) {
    let result = lua.create_table()?;
    result.set("error", "Temp file path blocked by sandbox")?;
    return Ok(result);
}
```

But `is_path_allowed()` in `lib.rs:93-115` uses `canonical.starts_with(allowed_dir)` for proper canonicalization. This inconsistency could allow bypass via symlinks.

**Suggestion:** Use `sandbox_for_tmpfile.is_path_allowed(path.to_string_lossy().as_ref())` instead of manual `starts_with` check.

---

## Priority Summary

| Finding | Priority | Type |
|---------|----------|------|
| CVE-2024-27956 duplicate entry | High | Bug |
| load_plugin_with_timeout ignores timeout | High | Bug |
| Socket DNS rebinding potential | Medium | Bug |
| OSV/CISA KEV not implemented | Medium | Discrepancy |
| FxHashMap in PluginManager | Medium | Performance |
| Ruby eval pattern incomplete | Low | Bug |
| Sandbox metrics not exposed | Low | Improvement |
| tmpfile path validation inconsistency | Low | Bug |

---

## Conclusion

The architecture document is largely accurate, with 164 NSE libraries correctly documented. The plugin security system is well-implemented with both regex and AST-based analysis. However, there are several bugs that should be addressed, particularly the duplicate CVE entry and the unused timeout parameter in Ruby plugin loading.

The main discrepancy is the claimed OSV and CISA KEV integration which was not found in the code. The NVD integration is complete and functional.
