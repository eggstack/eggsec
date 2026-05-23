---
name: slapper-nse
description: "NSE (Nmap Scripting Engine) support for Slapper - Lua VM, libraries, sandbox, CVE integration"
triggers:
  - nse
  - lua
  - mlua
  - nmap scripting engine
  - nse libraries
metadata:
  category: security
  tools: [nse, scanner, recon]
  scope: slapper-nse
---

## Overview

The `slapper-nse` crate (`crates/slapper-nse/`) provides Nmap Scripting Engine support via a Lua 5.4 interpreter using `mlua`. It allows running standard NSE scripts within Slapper.

## Key Components

| Component | File | Purpose |
|-----------|------|---------|
| `NseExecutor` | `src/executor.rs` | Sync Lua VM wrapper with NSE rule execution |
| `AsyncNseExecutor` | `src/async_executor.rs` | Async wrapper with tokio runtime |
| `ExecutorCore` | `src/executor_core.rs` | Shared Lua VM, globals, library registration |
| `SandboxConfig` | `src/lib.rs:50-76` | Sandbox restrictions for scripts |
| `ScanContext` | `src/context.rs:141-149` | Host info, ports, output during execution |

## Features

```
nse = ["mlua", "mlua-luau-scheduler", "openssl", "des"]
nse-ssh2 = ["nse", "dep:ssh2"]
sandbox = []  # Enables SandboxConfig enforcement
```

## Libraries (80+)

Located in `src/libraries/`:
- **socket.rs** (703 lines) - TCP/UDP/SCTP sockets with sandbox enforcement
- **io.rs** (391 lines) - File I/O with path sandboxing
- **lfs.rs** (379 lines) - LuaFileSystem with path restrictions
- **os.rs** (316 lines) - OS operations (getenv, setenv, date, exit, etc.)
- **http.rs** (803 lines) - HTTP client (blocking + async)
- **vulns.rs** (571 lines) - CVE database with major vulnerabilities
- **dns.rs**, **ssl.rs**, **ssh.rs**, **mysql.rs**, **redis.rs**, **mongodb.rs**, **ldap.rs**, **snmp.rs**, **smb.rs**, etc.

### Library Registration

Libraries are registered via `register_*_library()` functions. See `executor_core.rs:272-450` for the full list of modules registered as NSE globals.

## Sandbox Enforcement

| Library | Sandbox Enforcement |
|---------|---------------------|
| `io` | `is_path_allowed()` validates paths against `allowed_dir` |
| `lfs` | Path checks + violation counter (`LFS_SANDBOX_VIOLATIONS`) |
| `os` | `getenv/setenv` blocked, file ops path-checked |
| `socket` | `is_host_allowed()` validates hosts against `allowed_networks` CIDR |

### SandboxConfig

```rust
pub struct SandboxConfig {
    pub enabled: bool,                    // Controlled by `sandbox` feature
    pub allowed_dir: Option<PathBuf>,     // Default: /tmp/slapper-nse
    pub allowed_commands: Vec<String>,   // Empty = block all popen
    pub log_violations: bool,             // Default: true
    pub allowed_networks: Vec<IpNetwork>, // CIDR allowlist
}
```

### Metrics

```rust
pub struct SandboxMetrics {
    pub io_handles: usize,       // Active file handles
    pub io_violations: usize,    // io library violations
    pub lfs_violations: usize,   // lfs library violations
    pub os_violations: usize,    // os library violations
}
```

## CVE Integration

Located in `src/cve/`:
- **mod.rs** - `CveClient` trait, `CveAggregator`, `CveCache` with TTL
- **nvd.rs** - NVD API client (6 req/min without API key)
- **osv.rs** - OSV API client
- **cisa_kev.rs** - CISA Known Exploited Vulnerabilities

## Output Formats

`src/output.rs` provides:
- `generate_xml()` - nmap XML format
- `generate_grepable()` - nmap -oG format
- `generate_normal()` - nmap human-readable format

## Bug Fixes Logged in AGENTS.md

| Issue | Fix |
|-------|-----|
| UDP `sendto()` didn't validate sandbox | `connect_udp()` now checks host via `is_host_allowed()` |
| Duplicate `getenv` registration in `os.rs` | Removed duplicate `getenv_fn2` |
| `output.rs` multiple `unwrap()` on `writeln!` calls | Changed to use `let _ = writeln!()` pattern |
| `CveCache` used `HashMap` instead of `FxHashMap` | Changed to `FxHashMap` for performance |
| `CveAggregator` used `HashSet` instead of `FxHashSet` | Changed to `FxHashSet` for performance |
| Path traversal check bypass via `..` string check | Removed simple string check; rely on `is_path_allowed()` canonicalization |
| `async_executor.rs` Default impl panicked | Changed to propagate error via `unwrap_or_else` panic |
| `lfs.rs` path traversal check bypass | Removed weak `!path.contains("..")` check; rely on canonicalization only |
| Multiple libraries using `HashMap`/`HashSet` | Changed to `FxHashMap`/`FxHashSet` for performance in 13+ libraries |
| Mutex poisoning could cause panic | Changed `.unwrap()` to `.unwrap_or_else(\|e\| e.into_inner())` in httpspider, pcre |
| Missing `std::io::{Read, Write}` imports | Added to brute, io, nmap, openssl, ldap, and other libraries |
| `rustc-hash` not in slapper-nse dependencies | Added `rustc-hash.workspace = true` to Cargo.toml |
| `CveCache` missing closing bracket | Fixed typo in struct definition |
| `CveCache` using async `.await` on parking_lot RwLock | Removed `.await`; parking_lot RwLock is sync |
| `public_api/api.rs` 8 std::HashMap instances | Replaced with FxHashMap for performance |
| `libraries/http.rs:143` HashMap in parse_options | Changed to FxHashMap |
| `libraries/datafiles.rs:31-33` HashMap in get_services | Changed to FxHashMap |
| `libraries/creds.rs:102,123` HashSet usage | Changed to FxHashSet |

## Common Patterns

### Creating an Executor

```rust
use slapper_nse::{NseExecutor, SandboxConfig};

let executor = NseExecutor::with_target("example.com")?;
executor.set_script_args("user=admin")?;
let result = executor.run_script(script_content)?;
```

### Running with Sandbox

```rust
let sandbox = SandboxConfig::enabled();
let executor = NseExecutor::with_sandbox(sandbox)?;
```

### Accessing Metrics

```rust
let metrics = executor.get_sandbox_metrics();
println!("IO violations: {}", metrics.io_violations);
```

## Error Handling

Use explicit error handling instead of `unwrap_or_default()`:
```rust
let result = match executor.run_script(script) {
    Ok(output) => output,
    Err(e) => {
        tracing::warn!("Script failed: {}", e);
        return Err(e);
    }
};
```

## Testing

```bash
cargo test -p slapper-nse
cargo check --lib -p slapper-nse --features nse
```