# NSE Integration

Eggsec includes optional Nmap Scripting Engine (NSE) compatibility through the `eggsec-nse` crate. The goal is broad practical compatibility for useful script categories, not perfect Nmap runtime parity.

## NSE (Nmap Scripting Engine) Compatibility (`eggsec-nse`)

Eggsec includes a Lua interpreter (via `mlua`) that can run a curated set of Nmap NSE scripts.

### Core Features

- **Compatibility**: Broad practical compatibility for safe discovery, version, and default-style scripts within Eggsec scope and budgets.
- **Sandbox**: Optionally restricts dangerous Lua operations (e.g., file system access, network connections) for safer execution of untrusted scripts.
- **NSE Tool**: Provides a high-level API for running NSE scripts against targets discovered by Eggsec.
- **Async Executor**: `async_executor.rs` manages asynchronous execution of NSE scripts with timeout and resource budget controls.

### Sandbox Configuration

```rust
pub struct SandboxConfig {
    pub enabled: bool,                    // Controlled by `sandbox` feature
    pub allowed_dir: Option<PathBuf>,     // Restrict file ops to directory (default: /tmp/eggsec-nse)
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
- **Integrated Reporting**: NSE results are integrated into Eggsec's finding management and reporting system.

### NSE Libraries

164 NSE-style library modules implemented including: `stdnse`, `nmap`, `http`, `socket`, `io`, `os`, `lfs`, `dns`, `ssl`, `ssh`, `mysql`, `postgres`, `redis`, `mongodb`, `ldap`, `snmp`, `smb`, `smb2`, `vulns`, and many more. All located in `crates/eggsec-nse/src/libraries/`.

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
| `rustc-hash` not in eggsec-nse dependencies | Added `rustc-hash.workspace = true` to Cargo.toml |
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
| **Tier 1** | Safe discovery/version/default | Scripts that operate within Eggsec scope and budgets (e.g., `http-enum`, `ssl-cert`, `ssh-hostkey`). |
| **Tier 2** | Service-specific | Scripts requiring additional protocol libraries or credentials (e.g., `mysql-info`, `smb-enum-shares`). |
| **Tier 3** | Intrusive/brute-force/exploit-adjacent | Scripts requiring explicit opt-in (e.g., `http-brute`, `smb-vuln-*`). |
| **Unsupported** | Restricted | Scripts requiring unrestricted filesystem/process access, uncontrolled network reachability, or behavior incompatible with Eggsec guardrails. |

Execution profiles (`NseExecutionProfileKind`) encode these tiers as enforceable presets. `CompatibilityLab` corresponds to Tier 1 (full Nmap compatibility), while `AgentSafe` and `CiSafe` correspond to Tier 3 (sandboxed, restricted).

## NSE as a Knowledge Source

NSE libraries and scripts encode mature protocol-testing concepts developed over years of community use. Beyond direct execution, NSE serves as a knowledge source:

- **Protocol patterns**: NSE scripts demonstrate correct packet construction, response parsing, and error handling for dozens of protocols.
- **Detection logic**: Scripts encode fingerprint databases, version detection heuristics, and vulnerability signatures.
- **Bypass techniques**: Scripts document evasion methods that Eggsec can study and re-implement as Rust-native probes where repeatability, performance, and safety matter.

Selected NSE behaviors may be promoted into Rust-native probes over time, particularly for high-value categories where Eggsec's execution model offers advantages in speed, determinism, or safety.

## Sandbox Defaults

The recommended default for NSE execution is sandboxed mode:

- **Filesystem access**: Denied unless explicitly allowed via `allowed_dir`.
- **Process execution**: `io.popen()` restricted to an explicit command allowlist.
- **Network access**: Socket operations validated against `allowed_networks` CIDR allowlist.
- **Environment access**: `os.getenv()`/`os.setenv()` blocked in sandbox mode.
- **Timeouts and budgets**: Scripts should have execution timeouts and resource budgets.
- **Capability manifests**: Script category and capability manifests determine whether a script can run under sandboxed or unrestricted execution.

Agent/tool API paths should prefer sandboxed NSE. Unrestricted execution requires explicit opt-in and should only be used in controlled defense-lab environments.

## Execution Profiles

NSE execution profiles provide explicit presets that resolve into sandbox config, limits, script/module policy, network policy, and audit metadata. Each profile encodes a trust boundary assumption.

### Profile Variants

| Profile | Trust Level | Scripts | Modules | Network | Limits |
|---------|-------------|---------|---------|---------|--------|
| `ManualPermissive` | User-controlled, full trust | All builtin + files | All builtin + filesystem | AllowAllManual | 120s / 100M instr / 50MiB |
| `ManualStrict` | User-controlled, restricted | Builtin only, restricted roots | Builtin only | AllowCidrs from scope | 120s / 100M / 50MiB |
| `AgentSafe` | Autonomous agent | Builtin only | Builtin only | Derived from target/scope | 15s / 5M / 2MiB |
| `CiSafe` | CI pipeline | Builtin only | Builtin only | DenyAll | 15s / 5M / 2MiB |
| `CompatibilityLab` | Nmap compat testing | All + Nmap paths | All + Nmap paths | AllowAllManual | 120s / 100M / 50MiB |

### Resolution Flow

```
Profile Constructor → ResolvedNseExecutionProfile
    ├── SandboxConfig (enabled, allowed_dir, allowed_commands, allowed_networks)
    ├── NseExecutionLimits (wall_clock, lua_instruction_budget, resource caps)
    ├── NseScriptPolicy (builtin/files/roots/nmap_paths/size cap)
    ├── NseModulePolicy (builtin/filesystem/roots/size cap)
    ├── NseNetworkPolicy (allow/deny/CIDRs/target set)
    ├── audit_label (e.g., "nse:manual-permissive")
    └── warnings (sandbox status, scope implications)
```

### Network Policy Variants

| Policy | Behavior |
|--------|----------|
| `AllowAllManual` | No network restrictions (manual profiles only) |
| `DenyAll` | Zero network operations (CiSafe) |
| `AllowCidrs` | Only CIDRs from scope rules |
| `AllowResolvedTargetSet` | Only IPs resolved from the explicit target |

### Scope Derivation

For `AgentSafe` and `ManualStrict`, the network policy is derived from scope input:
- If `scope_cidrs` is non-empty → `AllowCidrs(scope_cidrs)`
- Else if `target_ip` is a valid IP → `AllowResolvedTargetSet(vec![target_ip])`
- Else → `DenyAll`

### Usage

```rust
use eggsec_nse::{NseExecutionProfileKind, ResolvedNseExecutionProfile, ScopeInput};

// Manual (CLI)
let profile = ResolvedNseExecutionProfile::manual_permissive();

// Agent (autonomous)
let scope = ScopeInput::new("192.168.1.1").with_scope_cidrs(&["192.168.1.0/24"]);
let profile = ResolvedNseExecutionProfile::agent_safe(&scope);

// CI (zero network)
let profile = ResolvedNseExecutionProfile::ci_safe();

// Run with profile
eggsec_nse::run_cli_with_profile(config, Some(profile)).await?;
```

## Script/Module Resolver

All script and module loading flows through `ScriptResolver` in `src/resolver.rs` to enforce security boundaries.

### Script Source Model

| Source Kind | Description | Policy |
|-------------|-------------|--------|
| `Builtin` | Shipped with eggsec-nse | Always allowed if profile permits |
| `TrustedRegistry` | Future bundled registries | Not yet implemented |
| `File` | User-provided script file | Manual-only unless profile allows |
| `InlineManual` | Tests and manual CLI | Not agent-safe by default |

### Module Name Grammar

Before any filesystem access, module names are validated:
- ASCII alphanumeric + `_`, `-`, `.`
- No leading `.`, no `..` traversal
- No path separators, shell metacharacters, or null bytes
- Max 256 characters

### Path Containment

- Canonical path resolution under approved roots
- Symlink-aware containment (symlinks resolving outside roots are rejected)
- File extension allowlist (`.lua`, `.nse` only)
- Size limits enforced before content evaluation

### Structured Diagnostics

`NseLoadDiagnostic` provides visibility into load behavior:
- `Resolved` - successful load with byte count
- `Blocked` - policy rejection
- `OutsideRoot` - path containment violation
- `SymlinkRejected` - symlink escape attempt
- `ModuleNameRejected` - grammar violation
- `OversizedRejected` - size limit exceeded
- `ModuleLoadFailed` - filesystem read error (reported, not silently skipped)
