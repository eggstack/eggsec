# NSE Integration

Slapper includes optional Nmap Scripting Engine (NSE) compatibility through the `slapper-nse` crate. The goal is broad practical compatibility for useful script categories, not perfect Nmap runtime parity.

## NSE (Nmap Scripting Engine) Compatibility (`slapper-nse`)

Slapper includes a Lua interpreter (via `mlua`) that can run a curated set of Nmap NSE scripts.

### Core Features

- **Compatibility**: Broad practical compatibility for safe discovery, version, and default-style scripts within Slapper scope and budgets.
- **Sandbox**: Optionally restricts dangerous Lua operations (e.g., file system access, network connections) for safer execution of untrusted scripts.
- **NSE Tool**: Provides a high-level API for running NSE scripts against targets discovered by Slapper.
- **Async Executor**: `async_executor.rs` manages asynchronous execution of NSE scripts with timeout and resource budget controls.

### Sandbox Configuration

```rust
pub struct SandboxConfig {
    pub enabled: bool,                    // Controlled by `sandbox` feature
    pub allowed_dir: Option<PathBuf>,     // Restrict file ops to directory (default: /tmp/slapper-nse)
    pub allowed_commands: Vec<String>,   // Whitelist for io.popen
    pub log_violations: bool,             // Log instead of block
    pub allowed_networks: Vec<IpNetwork>, // CIDR allowlist for sockets
}
```

### Sandboxed Operations

| Library | Operations | Sandbox Enforcement |
|---------|------------|---------------------|
| `io` | `open()`, `lines()`, `popen()`, `tmpfile()` | Path canonicalization, command allowlist |
| `lfs` | All file operations | Path validation against `allowed_dir` |
| `os` | `getenv()`, `setenv()` | Blocked in sandbox |
| `socket` | `connect()`, `tcp_connect()`, `sendto()` | Host validation against `allowed_networks` |

### Benefits

- **Community Scripts**: Access to thousands of community-developed security checks.
- **Lua Scripting**: Simple and familiar scripting language for custom security logic.
- **Integrated Reporting**: NSE results are integrated into Slapper's finding management and reporting system.

### NSE Libraries

169 NSE-style library modules implemented including: `stdnse`, `nmap`, `http`, `socket`, `io`, `os`, `lfs`, `dns`, `ssl`, `ssh`, `mysql`, `postgres`, `redis`, `mongodb`, `ldap`, `snmp`, `smb`, `smb2`, `vulns`, and many more. All located in `crates/slapper-nse/src/libraries/`.

### CVE Integration

The `vulns` library provides access to CVE databases:
- **NVD** (National Vulnerability Database) - `https://services.nvd.nist.gov/rest/json/cves/2.0`
- **OSV** (Open Source Vulnerabilities)
- **CISA KEV** (Known Exploited Vulnerabilities)

## Recent Bug Fixes

| Issue | Fix |
|-------|-----|
| UDP `sendto()` didn't validate sandbox | `connect_udp()` now checks host via `is_host_allowed()` |
| Duplicate `getenv` registration in `os.rs` | Removed duplicate `getenv_fn2` at line 295-302 |
| `output.rs` multiple `unwrap()` on `writeln!` calls | Changed to use `let _ = writeln!()` pattern |
| `CveCache` used `HashMap` instead of `FxHashMap` | Changed to `FxHashMap` for performance |
| `CveAggregator` used `HashSet` instead of `FxHashSet` | Changed to `FxHashSet` for performance |
| Path traversal check bypass via `..` string check | Removed simple string check; rely on `is_path_allowed()` |
| `async_executor.rs` Default impl panicked | Changed to `unwrap_or_else` panic with descriptive message |
| `lfs.rs` path traversal check bypass | Removed weak `!path.contains("..")` check; rely on canonicalization only |
| Multiple `HashMap`/`HashSet` in libraries | Changed to `FxHashMap`/`FxHashSet` for performance in 13+ library files |
| Mutex poisoning could cause panic in httpspider, pcre | Changed `.unwrap()` to `.unwrap_or_else(\|e\| e.into_inner())` |
| `rustc-hash` not in slapper-nse dependencies | Added `rustc-hash.workspace = true` to Cargo.toml |
| `CveCache` missing closing bracket in type definition | Fixed typo in struct definition |
| Async `.await` on parking_lot RwLock (sync) | Removed `.await` since parking_lot RwLock is synchronous |
| Missing `std::io::{Read, Write}` imports in libraries | Added to brute, io, nmap, openssl, ldap, and other libraries |
| Duplicate `std::io::{Read, Write}` import in ldap.rs | Removed duplicate |
| Duplicate `std::io::Write` import in nmap.rs | Removed duplicate |

---

## NSE Compatibility Policy

NSE scripts are categorized into support tiers based on risk and resource requirements:

| Tier | Category | Description |
|------|----------|-------------|
| **Tier 1** | Safe discovery/version/default | Scripts that operate within Slapper scope and budgets (e.g., `http-enum`, `ssl-cert`, `ssh-hostkey`). |
| **Tier 2** | Service-specific | Scripts requiring additional protocol libraries or credentials (e.g., `mysql-info`, `smb-enum-shares`). |
| **Tier 3** | Intrusive/brute-force/exploit-adjacent | Scripts requiring explicit opt-in (e.g., `http-brute`, `smb-vuln-*`). |
| **Unsupported** | Restricted | Scripts requiring unrestricted filesystem/process access, uncontrolled network reachability, or behavior incompatible with Slapper guardrails. |

## NSE as a Knowledge Source

NSE libraries and scripts encode mature protocol-testing concepts developed over years of community use. Beyond direct execution, NSE serves as a knowledge source:

- **Protocol patterns**: NSE scripts demonstrate correct packet construction, response parsing, and error handling for dozens of protocols.
- **Detection logic**: Scripts encode fingerprint databases, version detection heuristics, and vulnerability signatures.
- **Bypass techniques**: Scripts document evasion methods that Slapper can study and re-implement as Rust-native probes where repeatability, performance, and safety matter.

Selected NSE behaviors may be promoted into Rust-native probes over time, particularly for high-value categories where Slapper's execution model offers advantages in speed, determinism, or safety.

## Sandbox Defaults

The recommended default for NSE execution is sandboxed mode:

- **Filesystem access**: Denied unless explicitly allowed via `allowed_dir`.
- **Process execution**: `io.popen()` restricted to an explicit command allowlist.
- **Network access**: Socket operations validated against `allowed_networks` CIDR allowlist.
- **Environment access**: `os.getenv()`/`os.setenv()` blocked in sandbox mode.
- **Timeouts and budgets**: Scripts should have execution timeouts and resource budgets.
- **Capability manifests**: Script category and capability manifests determine whether a script can run under sandboxed or unrestricted execution.

Agent/tool API paths should prefer sandboxed NSE. Unrestricted execution requires explicit opt-in and should only be used in controlled defense-lab environments.
