# Plugins & NSE Architecture Review

## Summary

The NSE module implementation in `crates/slapper-nse/src/` has several performance issues where `std::collections::HashMap` and `std::collections::HashSet` are used instead of the more efficient `FxHashMap` and `FxHashSet` from rustc-hash. Most of these were documented as fixed in `architecture/plugins_nse.md`, but some issues remain.

## What's Implemented Correctly

### Core NSE Features
- Full Lua interpreter via `mlua` for running NSE scripts
- Sandbox configuration with `allowed_dir`, `allowed_commands`, `log_violations`, `allowed_networks`
- Sandboxed operations for `io`, `lfs`, `os`, `socket` libraries
- 164 NSE-style library modules implemented in `libraries/` directory
- CVE integration via `vulns` library with NVD, OSV, CISA KEV support

### Library Implementation (using FxHashMap correctly)
- `vulns.rs` - CVE_DB uses `FxHashMap`
- `rpc.rs` - RPC_PROGRAMS uses `FxHashMap<u32, FxHashMap<u32, &'static str>>`
- `smbauth.rs` - HASH_STORE uses `FxHashMap<String, (String, String)>`
- `httpspider.rs` - CRAWLERS uses `FxHashMap`, visited uses `FxHashSet`
- `pcre.rs` - COMPILED_REGEX uses `FxHashMap`
- `context.rs` - `ScanContext` uses `FxHashMap` throughout
- `executor.rs` - `parse_all_script_categories()` returns `FxHashMap`
- `executor_core.rs` - registry and cache use `FxHashMap`
- All other library files correctly use `FxHashMap`/`FxHashSet`

### Bug Fixes Verified from Documentation
- `output.rs` - Multiple `unwrap()` on `writeln!` calls changed to `let _ = writeln!()` pattern
- `lfs.rs` - Path traversal check bypass fixed by relying on canonicalization only
- `async_executor.rs:108` - Default impl uses `unwrap_or_else` with descriptive panic message
- `smbauth.rs` - No duplicate `getenv` registration (removed previously)
- `CveCache` - Uses `FxHashMap` (verified in vulns.rs)
- `CveAggregator` - Uses `FxHashSet` (not found, may be deprecated)

## Issues Found

### 1. `public_api/api.rs` uses std::collections::HashMap (HIGH - Performance)

**File**: `crates/slapper-nse/src/public_api/api.rs`

**Lines 107-108**: `get_cve_database()` returns `std::collections::HashMap` instead of `FxHashMap`:
```rust
fn get_cve_database(
) -> std::collections::HashMap<&'static str, (&'static str, &'static str, &'static str)> {
    let mut m = std::collections::HashMap::new();
```

**Lines 381, 486**: `NseHttpResponse.headers` and `NseHttpRequest.headers` use `std::collections::HashMap`:
```rust
pub headers: std::collections::HashMap<String, String>,
```

**Lines 413, 463, 532**: Local variables in functions use `std::collections::HashMap`:
```rust
let mut headers = std::collections::HashMap::new();
```

**Recommended fix**: Replace all `std::collections::HashMap` with `FxHashMap` in `api.rs`.

### 2. `libraries/http.rs` uses std::collections::HashMap (MEDIUM - Performance)

**File**: `crates/slapper-nse/src/libraries/http.rs`

**Line 143**: `parse_options()` function uses `HashMap`:
```rust
fn parse_options(opts: Option<&Table>) -> (HashMap<String, String>, Duration) {
    let mut headers = HashMap::new();
```

**Recommended fix**: Import `FxHashMap` and use it in `parse_options()`.

### 3. `libraries/datafiles.rs` uses std::collections::HashMap (MEDIUM - Performance)

**File**: `crates/slapper-nse/src/libraries/datafiles.rs`

**Line 31-33**: `get_services()` returns `HashMap` instead of `FxHashMap`:
```rust
fn get_services() -> &'static HashMap<&'static str, (u16, &'static str)> {
    SERVICES.get_or_init(|| {
        let mut m = HashMap::new();
```

Note: `PROTOCOLS` correctly uses `FxHashMap`, but `SERVICES` does not.

**Recommended fix**: Change `get_services()` to return `FxHashMap` and use `FxHashMap::default()`.

### 4. `libraries/creds.rs` uses std::collections::HashSet (MEDIUM - Performance)

**File**: `crates/slapper-nse/src/libraries/creds.rs`

**Lines 102, 123**: Local `seen` variables use `std::collections::HashSet`:
```rust
let mut seen = std::collections::HashSet::new();
```

**Recommended fix**: Change to `FxHashSet` with `FxHashSet::default()`.

## Verification

All other NSE library files correctly use `FxHashMap`/`FxHashSet` from rustc-hash as expected.

## Files Reviewed

| File | Status | Notes |
|------|--------|-------|
| `context.rs` | ✓ Correct | Uses FxHashMap throughout |
| `executor.rs` | ✓ Correct | Returns FxHashMap |
| `executor_core.rs` | ✓ Correct | Uses FxHashMap for registry |
| `async_executor.rs` | ✓ Correct | Default impl uses unwrap_or_else |
| `output.rs` | ✓ Correct | Uses `let _ = writeln!()` pattern |
| `libraries/vulns.rs` | ✓ Correct | CVE_DB uses FxHashMap |
| `libraries/rpc.rs` | ✓ Correct | Uses FxHashMap |
| `libraries/smbauth.rs` | ✓ Correct | Uses FxHashMap |
| `libraries/httpspider.rs` | ✓ Correct | Uses FxHashMap and FxHashSet |
| `libraries/pcre.rs` | ✓ Correct | Uses FxHashMap |
| `public_api/api.rs` | ⚠ Issues | Uses std HashMap (4 locations) |
| `libraries/http.rs` | ⚠ Issues | Uses std HashMap (line 143) |
| `libraries/datafiles.rs` | ⚠ Issues | get_services uses std HashMap |
| `libraries/creds.rs` | ⚠ Issues | Uses std HashSet (lines 102, 123) |

## Recommended Fixes Summary

| File | Line | Issue | Priority |
|------|------|-------|----------|
| public_api/api.rs | 107-108 | HashMap → FxHashMap | HIGH |
| public_api/api.rs | 381, 486 | HashMap → FxHashMap in struct | HIGH |
| public_api/api.rs | 413, 463, 532 | Local HashMap → FxHashMap | HIGH |
| libraries/http.rs | 143 | HashMap → FxHashMap | MEDIUM |
| libraries/datafiles.rs | 31-33 | HashMap → FxHashMap | MEDIUM |
| libraries/creds.rs | 102, 123 | HashSet → FxHashSet | MEDIUM |