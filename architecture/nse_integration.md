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
| `ManualPermissive` | User-controlled, full trust | All builtin + files | Builtin only (filesystem modules require explicit `allowed_module_roots`) | AllowAllManual | 120s / 100M instr / 50MiB |
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

## Resolver-Owned Module Loading (Milestone 1 Closure)

All Lua `require()` filesystem loading delegates to `ScriptResolver::resolve_module()`. The resolver enforces:

1. **Module name grammar** — validated before any filesystem access
2. **Profile policy** — `allow_filesystem_modules` and `allowed_module_roots` checked
3. **Canonical root containment** — `canonical_candidate.starts_with(canonical_root)` using path-component semantics (not string prefix)
4. **Symlink escape rejection** — symlinks resolving outside approved roots are rejected
5. **Extension allowlist** — `.lua` and `.nse` only
6. **Size limits** — `max_required_module_bytes` enforced before content evaluation

### Canonical Root Containment

Every script/module filesystem load must prove:

```
canonical_candidate starts_with one canonical_allowed_root using path-component semantics
```

`canonicalize()` is required before comparison. `Path::strip_prefix()` provides component semantics — `/tmp/root_evil` does NOT match root `/tmp/root`. Symlinks that resolve outside approved roots are rejected by `validate_symlink_containment()`.

### Empty-Roots Semantics

The meaning of an empty `allowed_script_roots` / `allowed_module_roots` depends on the `allow_*` boolean and the profile that produced the policy:

| Policy area | Bool | Empty roots meaning |
|-------------|------|---------------------|
| Manual script files (`ManualPermissive`) | `allow_script_files = true` | Unrestricted manual file selection. Extension and size limits still apply. No root containment check, no symlink check — `path.canonicalize()` only. |
| Restricted script files (`ManualStrict`, `CompatibilityLab`) | `allow_script_files = true` | Misconfiguration. Files outside any configured root are rejected with `OutsideRoot`. Profiles must populate `allowed_script_roots` to permit script files. |
| Automated script files (`AgentSafe`, `CiSafe`) | `allow_script_files = false` | Denied before any root check via `BlockedByPolicy`. Roots are irrelevant. |
| Filesystem modules (`ManualPermissive`) | `allow_filesystem_modules = true` | No filesystem modules. Empty roots mean only built-in modules resolve. |
| Restricted filesystem modules (`ManualStrict`, `CompatibilityLab`) | `allow_filesystem_modules = true` | Misconfiguration. Modules outside configured roots are rejected. |
| Automated filesystem modules (`AgentSafe`, `CiSafe`) | `allow_filesystem_modules = false` | Denied before any root check. Roots are irrelevant. |

The `ManualPermissive` constructor emits a `manual-permissive profile is not agent-safe` warning so automated callers cannot accidentally use it.

### Read vs Write Authorization Helpers

`resolver.rs` exposes two distinct root-containment helpers:

- `validate_existing_path_under_roots(path, roots)` — **read-only** helper. Requires the canonical file path to resolve. Returns `IoError` for non-existent files. Used by both `resolve_script_file()` and `resolve_module()`.
- `validate_parent_under_roots(path, roots)` — **write/create** helper. Authorizes a path by canonicalizing only its parent. Currently unused by read paths. Marked `#[allow(dead_code)]` and reserved for future create/write semantics.

Read paths must never authorize non-existent script/module files. The split is documented in the helper doc comments.

### Automated Surface Profile Enforcement

`ManualPermissive` is manual-only (CLI/TUI). Automated surfaces must use explicit profiles:

| Surface | Required Profile | Enforcement |
|---------|-----------------|-------------|
| CLI handler | `ManualPermissive` | Explicit in `handle_nse()` |
| TUI dispatch | `ManualPermissive` | Explicit in `run_nse()` (currently manual-only path) |
| Agent/MCP/daemon | `AgentSafe` or `CiSafe` | Via `RunRequest` profile; not yet wired |
| CI | `CiSafe` | Explicit |

Manual-only constructors (`NseExecutor::new()`, `with_sandbox()`, `with_target()`) are documented with `# Manual-only` doc comments. Automated surfaces must use `with_policy()` or `with_profile()`.

### Cancellation Posture

Lua execution has cooperative cancellation via `NseCancellationToken` (interrupt hook fires between instructions). Core infrastructure paths (`load_script`, `setup_require`) check `is_cancelled()` before file reads. Blocking Rust-side helpers (~170 calls across 40+ library files) do NOT have individual cancellation checks — they are bounded only by the Lua interrupt hook and will be addressed in Milestone 3 via capability wrappers.

## Milestone 1 Closure Note

The NSE Milestone 1 closure is the end-state for loader/script-file/module-file policy enforcement. The following are explicitly **closed**:

- All Lua `require()` filesystem loading is resolver-owned via `ScriptResolver::resolve_module()`.
- Module name grammar is validated before any filesystem access.
- Canonical root containment and symlink escape rejection are enforced for restricted profiles.
- `ManualPermissive` script-file loading is intentionally discretionary (empty roots = unrestricted manual selection). Extension and size limits still apply.
- `AgentSafe` and `CiSafe` reject arbitrary script files and filesystem modules before any path authorization.
- Read-path authorization (`validate_existing_path_under_roots`) cannot authorize non-existent files via parent fallback. Write/create semantics, if ever added, use a separate helper (`validate_parent_under_roots`).

The following remain **deferred to Milestone 3**:

- Rust-side blocking helper cancellation. ~170 calls across 40+ library files do not have individual cancellation checks. They are bounded by the Lua interrupt hook and resource counters. Milestone 3 will introduce capability wrappers with explicit cancellation points.

The empty-roots semantic table above and the read/write helper split are the contractual surface that future maintainers must preserve. Tests in `crates/eggsec-nse/tests/script_file_policy_tests.rs` are the regression net.

## Milestone 1 Closure Index

The following files are the canonical implementation, test, and doc anchors for Milestone 1. Future maintainers should treat them as the authoritative pointers and avoid duplicating their content elsewhere.

### Canonical Implementation

- `crates/eggsec-nse/src/resolver.rs` — `ScriptResolver`, `validate_existing_path_under_roots` (read-only), `validate_parent_under_roots` (write/create, `#[allow(dead_code)]`), module-name grammar, structured diagnostics.
- `crates/eggsec-nse/src/profile.rs` — `ResolvedNseExecutionProfile` constructors (`manual_permissive`, `manual_strict`, `agent_safe`, `ci_safe`, `compatibility_lab`), `NseScriptPolicy` and `NseModulePolicy` doc tables defining empty-roots semantics.
- `crates/eggsec-nse/src/executor_core.rs` — `setup_require()` delegates Lua `require()` filesystem loading to `ScriptResolver::resolve_module()`. `default_script_policy()` / `default_module_policy()` mirror `ManualPermissive` and are reserved for manual constructors.

### Canonical Tests

- `crates/eggsec-nse/tests/script_file_policy_tests.rs` — Milestone 1 regression net: manual permissive, strict, agent/CI, module-root, symlink, and CLI resolver-path cases.
- `crates/eggsec-nse/tests/profile_guard_tests.rs` — Architecture guards: `AgentSafe`/`CiSafe` deny script files and filesystem modules, `CiSafe` has zero network ops, automated timeouts, manual-only constructor warnings.
- `crates/eggsec-nse/tests/execution_limits_tests.rs` — Wall-clock, instruction-budget, output/script/module size, resource-counter, and cooperative cancellation behavior.
- `crates/eggsec-nse/tests/profile_tests.rs` — Profile-level invariants: scoped targets, network-policy precedence, audit labels, script-policy consistency, automated vs manual limits.
- `crates/eggsec-nse/tests/sandbox_tests.rs` — `SandboxConfig` enforcement: path restrictions, command allowlist, CIDR filtering, host resolution.

### Canonical Policy Contract

- `ManualPermissive` is manual-only; automated callers must not use it (constructor emits a non-agent-safe warning).
- Empty `allowed_script_roots` under `ManualPermissive` means unrestricted manual script-file selection; extension and size limits still apply.
- Empty `allowed_module_roots` means no filesystem module loading (built-ins only).
- `AgentSafe` and `CiSafe` reject arbitrary script files and filesystem modules before any path authorization.
- Restricted profiles (`ManualStrict`, `CompatibilityLab`) require configured roots for filesystem script/module loading.
- Read-path authorization requires the canonical file to resolve; non-existent files return `IoError`. Parent-based fallback is intentionally not exposed to read paths.
- Rust-side blocking helper cancellation is deferred to Milestone 3 capability wrappers; current behavior is bounded by the Lua interrupt hook and resource counters.

### Deferred Work

- **Milestone 2 (next)**: library registry, rule semantics, compatibility truthfulness, structured run reports. Begins at library-registry/rule/report-truthfulness, not at loader-policy redesign.
- **Milestone 3**: Rust-side blocking helper cancellation via capability wrappers.

## Library Registry

The library registry (`src/resolver/registry.rs`) provides a declarative, machine-readable inventory of standard Nmap Lua library modules. It is used for policy evaluation, diagnostics, and compatibility reporting.

### Registry Structure

| Type | Description |
|------|-------------|
| `NseLibraryDescriptor` | Declarative descriptor for a single library module |
| `NseLibraryCategory` | Functional category: `Core`, `Protocol`, `Utility`, `Exploit`, `Auth` |
| `NseSandboxSideEffect` | Side effects: `None`, `FileSystemRead`, `FileSystemWrite`, `NetworkAccess`, `ProcessExecution`, `EnvAccess` |
| `NseFallbackBehavior` | Fallback: `HardFail`, `GracefulDegrade`, `Skip` |

### Static Registry

`LIBRARY_REGISTRY: &[NseLibraryDescriptor]` contains 43 entries covering Nmap's standard Lua library set (24 main + 19 auxiliary). The orchestrator `nse.lua` is intentionally excluded.

### Lookup Functions

| Function | Purpose |
|----------|---------|
| `find_library(name)` | Find a library descriptor by name |
| `all_libraries()` | Return all registered descriptors |
| `libraries_by_category(cat)` | Filter by category |
| `libraries_with_side_effects()` | Libraries with non-None side effects |
| `sandbox_policy_for_library(name)` | Effective sandbox side effects (None if clean) |
| `libraries_missing_from_nmap()` | Libraries with optional deps |
| `registry_count()` | Total registered count |

### Feature Gate

The registry compiles with the `nse` feature **off** — it contains no Lua or mlua dependencies. This allows policy code to query library metadata without requiring the full NSE runtime.

### Architecture Guard

Check 27 in `scripts/check-architecture-guards.sh` verifies:
1. Every registry entry has a corresponding Rust module in `src/libraries/`
2. Rust modules without registry entries are reported as warnings (protocol-specific implementations)

## Next Work: Milestone 2

The Milestone 1 contract above is the boundary. Future work should:

- Treat the loader and profile enforcement as closed unless regression tests reveal a defect.
- Build on `ScriptResolver` rather than bypass it.
- Move library registration toward a declarative registry with a truthfulness matrix.
- Document approximate NSE rule-matching semantics and known gaps.
- Expose profile, resolver diagnostics, limits, and compatibility status in structured run reports.

The Milestone 2 plan should be written from this closure index without reopening the loader or profile contracts established here.

## Verification Record (Milestone 1)

The intended Milestone 1 gate is:

```bash
cargo check -p eggsec-nse                          # baseline (default features); expected to require the `nse` feature for resolver paths
cargo check -p eggsec-nse --features nse          # primary NSE build
cargo test -p eggsec-nse --features nse           # all NSE unit + integration tests
cargo test -p eggsec-nse --features nse,sandbox   # sandbox profile enforcement tests
cargo check -p eggsec --features nse              # main crate wired with NSE feature
make test-nse                                     # eggsec-level NSE tests via nextest
```

Latest observed status (Milestone 1 polish-pass re-run):

| Command | Status | Notes |
|---------|--------|-------|
| `cargo check -p eggsec-nse --features nse` | PASS (0 errors, 96 warnings — pre-existing) | Library `unused` warnings are pre-existing |
| `cargo test -p eggsec-nse --features nse` | **183 passed, 1 ignored** across 7 suites | Includes `script_file_policy_tests` (14), `profile_guard_tests` (14), `profile_tests` (42), `execution_limits_tests` (21), `sandbox_tests` (17), inline `resolver` tests (~75) |
| `cargo test -p eggsec-nse --features nse,sandbox` | **183 passed, 1 ignored** | Sandbox feature does not regress the suite |
| `cargo check -p eggsec --features nse` | PASS (0 errors, 100 warnings — pre-existing) | Main crate wires NSE without errors |
| `cargo test -p eggsec --features nse --test nse_tests` | **174 passed** | Eggsec-level NSE integration tests |
| `make test-nse` | N/A locally (no `cargo-nextest` installed) | Documented equivalent: the two `cargo test` commands above |

Commands that fail or diverge in a re-run must be documented with the exact command and a follow-up task.
