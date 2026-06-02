# NSE Module AGENTS Override

## Module Overview

The NSE (Nmap Scripting Engine) module (`crates/slapper-nse/`) provides Lua VM integration, NSE libraries, sandbox enforcement, and CVE integration.

## Recent Bug Fixes (2026-05-28)

| Component | Issue | Fix |
|-----------|-------|-----|
| `slapper-nse/src/libraries/smbauth.rs` | 8 functions defined twice (shadowing issue) | Removed duplicate definitions, keep first occurrence |
| `slapper-nse/src/libraries/smbauth.rs` | `signing_hmac_md5` defined 3 times | Kept first (lines 121-131), removed others |
| `slapper-nse/src/libraries/datafiles.rs` | `ssh`, `ntp`, `mongodb` entries duplicated | Removed duplicate entries |
| `slapper-nse/src/libraries/io.rs:140,163,181,194,211` | `file.get("fd").unwrap_or(-1)` masks missing fd | Return explicit error when fd missing |
| `src/libraries/http.rs:143-144` | Performance | Replaced `HashMap` with `FxHashMap` in `parse_options` |
| `src/libraries/datafiles.rs:31-33` | Performance | Replaced `HashMap` with `FxHashMap` in `get_services()` |
| `src/libraries/creds.rs:102,123` | Performance | Replaced `HashSet` with `FxHashSet` for local `seen` variables |
| `src/public_api/api.rs:107-108,381,413,463,486,532` | Performance | Replaced all `HashMap` with `FxHashMap` for CVE database, HTTP headers |

## NSE Libraries HashMap Usage

All NSE library files now use `rustc_hash::FxHashMap`/`FxHashSet` for consistency and performance.

## Key Patterns

- **NSE duplicate functions**: Check for duplicate function definitions (especially in `smbauth.rs`)
- **Sandbox enforcement**: UDP sendto is sandboxed via `connect_udp()` host check
- **Mutex poisoning**: Use `.unwrap_or_else(|e| e.into_inner())` for graceful handling
- **Async on sync RwLock**: parking_lot RwLock is synchronous - don't use `.await`

## Known Issues (Pending Fix)

1. **Missing Sandbox Integration Tests**: No visible test coverage for NSE sandbox enforcement (network and filesystem restrictions). The sandbox has `is_path_allowed()`, `is_host_allowed()`, `is_command_allowed()` methods in `slapper-nse/src/lib.rs:93-159`, but there are no `#[test]` functions testing these restrictions. Add integration tests for sandbox enforcement, particularly around network and filesystem restrictions.

2. **TOCTOU Vulnerability in lfs Path Traversal**: `is_path_allowed()` could be bypassed via symlinks or race conditions between check and use.

3. **DNS Rebinding Attack Vector**: `is_host_allowed()` DNS resolution could be vulnerable to DNS rebinding if `allowed_networks` changes between check and connect.

4. **LazyLock Initialization Contention**: `WAF_SIGNATURES` LazyLock in the main slapper crate may have thread contention during first access in multi-threaded context.

## Dependencies

- `mlua` for Lua VM
- `rb-sys` / `magnus` for Ruby (feature-gated)
- `pyo3` for Python (feature-gated)