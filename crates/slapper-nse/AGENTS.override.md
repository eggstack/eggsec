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

## NSE Libraries HashMap Usage

The following NSE library files use `std::collections::HashMap`/`HashSet` instead of `FxHashMap`/`FxHashSet`:

| File | Issue | Type |
|------|-------|------|
| `src/libraries/http.rs:8,143-144` | Performance | `HashMap<String, String>` in `parse_options` |
| `src/libraries/vulns.rs:7,10-11` | Performance | `HashMap<&'static str, ...>` CVE database |
| `src/libraries/datafiles.rs:6,9-10` | Performance | `HashMap<&'static str, ...>` protocols/services |
| `src/libraries/smbauth.rs:7,10` | Performance | `HashMap<String, ...>` hash store |
| `src/libraries/rpc.rs:7,10,12` | Performance | `HashMap<u32, HashMap<u32, &str>>` nested |
| `src/libraries/public_api/api.rs` | Performance | Multiple `HashMap` for CVE database, HTTP headers |
| `src/libraries/creds.rs:102,123` | Performance | `HashSet` local variables |

**Fix**: Replace with `rustc_hash::FxHashMap` or `FxHashSet` for consistency and performance. See `plans/plan.md` Wave 3.

## Key Patterns

- **NSE duplicate functions**: Check for duplicate function definitions (especially in `smbauth.rs`)
- **Sandbox enforcement**: UDP sendto is sandboxed via `connect_udp()` host check
- **Mutex poisoning**: Use `.unwrap_or_else(|e| e.into_inner())` for graceful handling
- **Async on sync RwLock**: parking_lot RwLock is synchronous - don't use `.await`

## Dependencies

- `mlua` for Lua VM
- `rb-sys` / `magnus` for Ruby (feature-gated)
- `pyo3` for Python (feature-gated)