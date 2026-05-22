# Plugins NSE Review - Improvement Plan

## Summary

The `plugins_nse.md` architecture document describes Slapper's NSE (Nmap Scripting Engine) integration, including:
- A Lua interpreter (via `mlua`) for running NSE scripts
- A sandbox system for restricting dangerous Lua operations
- 164 NSE-style library modules
- CVE integration via NVD, OSV, and CISA KEV
- Security checks for Python and Ruby plugins

## Verification of Key Claims

| Claim | Status | Notes |
|-------|--------|-------|
| 164 NSE-style library modules | **VERIFIED** | 169 library files exist in `libraries/` directory |
| Sandbox enforcement for `io`, `lfs`, `os`, `socket` | **VERIFIED** | Properly implemented in each library |
| CVE integration (NVD, OSV, CISA KEV) | **VERIFIED** | Implemented in `cve/` module |
| `output.rs` multiple `unwrap()` on `writeln!` | **FIXED** | Uses `let _ = writeln!()` pattern |
| `CveCache` uses `FxHashMap` | **VERIFIED** | Line 174 uses `FxHashMap` |
| `CveAggregator` uses `FxHashSet` | **VERIFIED** | Lines 260, 287 use `FxHashSet` |
| Path traversal check bypass fix | **VERIFIED** | Removed simple `..` string check |
| Duplicate `getenv` registration | **FIXED** | Only one `getenv_fn` exists |
| Mutex poisoning handling in httpspider, pcre | **VERIFIED** | Uses `unwrap_or_else(\|e\| e.into_inner())` |

## Bugs Found

### 1. **CRITICAL: Duplicate Function Definitions in `smbauth.rs`**

File: `/Users/davidbowman/projects/slapper/crates/slapper-nse/src/libraries/smbauth.rs`

The following functions are defined **twice**, causing the second definition to shadow the first:
- `compute_lm_hash` (lines 39-54 and 151-166)
- `ntlmv1_session` (lines 56-81 and 168-193)
- `ntlmv2_session` (lines 83-93 and 195-205)
- `get_ntlm_challenge` (lines 95-107 and 207-219)
- `signing_md5` (lines 109-119 and 221-231)
- `signing_hmac_md5` (lines 121-131, 233-243, and 245-256 - **THREE times**)
- `encrypt_password` (lines 133-149 and 258-276)
- `decrypt_password` (lines 278-299 and 344-364)

**Recommended Fix**: Remove duplicate definitions, keeping only the first occurrence of each function.

### 2. **WARNING: Duplicate Entries in `datafiles.rs`**

File: `/Users/davidbowman/projects/slapper/crates/slapper-nse/src/libraries/datafiles.rs`

In `get_services()`:
- `ssh` appears at lines 37 and 63
- `ntp` appears at lines 48 and 69
- `mongodb` appears at lines 46 and 77

While HashMap insertion of duplicates is not an error, this indicates copy-paste errors that could lead to maintenance issues.

**Recommended Fix**: Remove duplicate entries from the initialization code.

### 3. **Silent Error Suppression in `io.rs`**

File: `/Users/davidbowman/projects/slapper/crates/slapper-nse/src/libraries/io.rs`

Lines 140, 163, 181, 194, 211 use `unwrap_or()` on file descriptor retrieval:
```rust
let fd: i32 = file.get("fd").unwrap_or(-1);
```

This silently treats missing fd as valid (-1), which could mask bugs.

**Recommended Fix**: Return explicit error when fd is missing rather than defaulting to -1.

## Performance Issues

### 1. **`std::collections::HashMap` Still Used in Several Libraries**

The following files still use `std::collections::HashMap` instead of `FxHashMap`:

| File | Line | Type |
|------|------|------|
| `http.rs` | 8 | `HashMap<String, String>` in `parse_options` |
| `vulns.rs` | 7, 10 | `HashMap` for CVE database |
| `datafiles.rs` | 6, 9-10 | `HashMap` for protocols/services |
| `smbauth.rs` | 7, 10 | `HashMap` for hash store |
| `rpc.rs` | 7, 10, 12 | `HashMap<u32, HashMap<u32, &'static str>>` for RPC programs |
| `public_api/api.rs` | 107, 108, 381, 413, 463, 486, 532, 1106 | Multiple `HashMap` uses |
| `creds.rs` | 102, 123 | `std::collections::HashSet` (local variables, less critical) |

**Recommended Fix**: Replace with `rustc_hash::FxHashMap` or `FxHashSet` for consistency and performance.

### 2. **HashMap in Hot Paths**

The `http.rs:parse_options` function (line 143) creates a new `HashMap` on every HTTP request. This should use `FxHashMap` for better performance.

**Recommended Fix**: Change to `FxHashMap` in `parse_options`.

## Pattern Violations

### 1. **Duplicate Registration Calls in `executor_core.rs`**

File: `/Users/davidbowman/projects/slapper/crates/slapper-nse/src/executor_core.rs`

Looking at the module registration (lines 433-580), some libraries are registered with `let _ = library::register_*()` which silently ignores errors. This includes:
- `datetime::register_datetime_library(&self.lua)` at line 486
- `rand::register_rand_library(&self.lua)` at line 487

While these may not fail in practice, ignoring registration errors is inconsistent with other libraries that propagate errors.

**Recommended Fix**: Either propagate errors consistently or document why these are safe to ignore.

### 2. **Inconsistent Error Handling in `http.rs`**

Multiple places use `.unwrap_or_default()` or `.unwrap_or_else()` silently:
- Line 253-255: `block_on(resp.text()).unwrap_or_default()` - silently ignores errors
- Line 185, 227, 395: `.unwrap_or("").to_string()` on header conversion

**Recommended Fix**: Log errors explicitly rather than silently defaulting.

## Minor Issues

### 1. **Duplicate Key in `datafiles.rs` `get_services()`**

The HashMap has duplicate keys (ssh, ntp, mongodb). While inserting duplicates into a HashMap just overwrites the previous value (no error), this is likely unintended and indicates copy-paste errors during initial population.

### 2. **Comment Inconsistency**

In `smbauth.rs`, the comment at line 83-93 says "not_implemented" for `ntlmv2_session` but then the same function is defined again later with the same "not_implemented" return value.

### 3. **Missing `#[allow(dead_code)]` on `simple_hash`**

The `simple_hash` function at line 398-404 in `smbauth.rs` is only used internally but is `pub`. Consider adding documentation or marking as allow dead_code if intentional.

## Security Considerations (Verified Good)

The architecture document mentions sandbox enforcement for:
- **io library**: Path canonicalization via `is_path_allowed()`, command allowlist for `popen` - **CORRECTLY IMPLEMENTED**
- **lfs library**: Path validation against `allowed_dir` via `is_path_allowed()` - **CORRECTLY IMPLEMENTED**
- **os library**: `getenv` returns empty in sandbox, `setenv`/`unsetenv` blocked - **CORRECTLY IMPLEMENTED**
- **socket library**: Host validation via `is_host_allowed()`, UDP `sendto()` calls `connect_udp()` - **CORRECTLY IMPLEMENTED**

The sandbox enforcement appears complete and correctly implemented.

## Recommended Fix Priority

1. **HIGH**: Fix duplicate function definitions in `smbauth.rs` (bug)
2. **HIGH**: Replace `std::collections::HashMap` with `FxHashMap` in `http.rs`, `vulns.rs`, `datafiles.rs`, `smbauth.rs`, `rpc.rs`, and `public_api/api.rs`
3. **MEDIUM**: Fix duplicate entries in `datafiles.rs`
4. **MEDIUM**: Improve error handling in `io.rs` for missing fd
5. **LOW**: Clean up duplicate registration calls pattern in `executor_core.rs`
