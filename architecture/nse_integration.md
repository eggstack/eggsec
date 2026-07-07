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

166 library implementation files in `crates/eggsec-nse/src/libraries/` including: `stdnse`, `nmap`, `http`, `socket`, `io`, `os`, `lfs`, `dns`, `ssl`, `ssh`, `mysql`, `postgres`, `redis`, `mongodb`, `ldap`, `snmp`, `smb`, `smb2`, `vulns`, and many more. The library registry (`LIBRARY_REGISTRY`) contains 43 curated descriptors covering Nmap's standard Lua library set — registry metadata is the source of truth for compatibility claims, not implementation file counts.

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

Execution profiles (`NseExecutionProfileKind`) encode these tiers as enforceable presets. `CompatibilityLab` corresponds to Tier 1 (selective practical NSE compatibility), while `AgentSafe` and `CiSafe` correspond to Tier 3 (sandboxed, restricted).

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

Manual-only constructors (`NseExecutor::new()`, `with_sandbox()`, `with_target()`, `with_policy()`) are documented with `# Manual-only` doc comments. Automated surfaces must use `with_profile()` or `with_full_policy()`.

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

- **Milestone 2 (closed)**: Library registry, rule semantics, compatibility truthfulness, structured run reports, rule evaluation reports, and per-run library-usage reporting. See [Milestone 2 Closure Note](#milestone-2-closure-note).
- **Milestone 3 (closed)**: Rust-side blocking helper cancellation via capability wrappers. See [Milestone 3 Closure Note](#milestone-3-closure-note).

## Milestone 2 Closure Note

NSE Milestone 2 is closed. The following are explicitly **closed**:

- **Library registry source of truth**: `NseLibraryDescriptor` / `LIBRARY_REGISTRY` in `resolver/registry.rs` is the canonical inventory of standard Nmap Lua library modules. Compatibility claims must reference registry metadata, not implementation file counts.
- **Rule semantics report path**: `NseRuleEvaluationReport` provides structured rule-evaluation metadata (kind, status, fidelity, approximations, inputs). Rule behavior is defined by this report, not by prose descriptions.
- **Rule evaluation**: `evaluate_rule()` in `report.rs` converts Lua rule results into structured `NseRuleEvaluationReport` instances. Outcomes: evaluated+matched (bool true), evaluated+not-matched (bool false or nil), unsupported return type (non-bool with `unsupported` field), and errored (lua error). `evaluate_rule_value()` in `executor.rs` provides inline evaluation. `evaluate_rule()` is the canonical path for CLI runtime (`run_script_with_rules()`).
- **Library reports**: `NseRunReport.libraries` records per-run observed or attempted `require()` activity, along with per-run diagnostics. Each entry has a `loaded` field: `true` means the runtime observed a successful module load; `false` means a `require()` was attempted but the module failed, was blocked, was missing, had an invalid name, or was statically detected without runtime confirmation. Static `require()` detection is approximate and labeled with a warning. It is not a capability snapshot and should not be read as a static inventory of all supported NSE libraries.
- **Error path reports**: `build_failure_report()` in `lib.rs` produces a full `NseRunReport` for error paths, including library reports and error information. Empty `libraries` is valid when no runtime or static `require()` evidence is available; the report must not fabricate unobserved libraries.
- **Structured reports**: `NseRunReport` is the canonical structured output model for NSE runs. Run output truthfulness is defined by `NseRunReport` fields, not by ad-hoc log output.
- **Compatibility corpus**: A representative corpus of NSE script fixtures in `tests/fixtures/nse_corpus/` verifies supported, partial, approximate, unsupported, denied, and errored behavior. The corpus is representative and local-only by default — it does not cover all Nmap scripts.
- **Truthfulness follow-up**: `NseRunReport.libraries` was later refined to reflect per-run usage rather than a capability snapshot. That follow-up did not reopen Milestone 2.
- **Documentation/release gate**: Verification commands and compatibility claims are documented and auditable.

The following remain **deferred**:

- **Milestone 3**: Capability wrappers and Rust-side blocking-helper cancellation (~170 calls across 40+ library files).
- **Full Nmap parity gaps**: Eggsec has selective practical NSE compatibility, not full Nmap parity. Expanding coverage to full parity is not a Milestone 2 goal.
- **Expanding corpus breadth**: The current corpus is representative; broader coverage is future work.
- **Additional library behavior upgrades**: Library implementations beyond the current 166 files are not part of Milestone 2.

### Boundary for Future Work

- Loader and profile enforcement remain closed from Milestone 1 (see [Milestone 1 Closure Index](#milestone-1-closure-index)).
- Library compatibility is defined by `NseLibraryRegistry` metadata, not by implementation file counts.
- Rule behavior is defined by `NseRuleEvaluationReport` / `evaluate_rule()` / rule semantics metadata.
- `NseRunReport.libraries` records per-run required/attempted library usage; compatibility claims still come from registry metadata.
- Future milestones should build on the registry, report, and corpus foundations rather than revisiting them.

#### Milestone 3 Boundary

Milestone 3 should focus on:

- Capability wrappers for side-effecting Rust helpers
- Network/filesystem/process/time/randomness accounting
- Cancellation checks before and after blocking helper calls
- Profile-aware denial/allowance for helper operations
- Report integration for helper-side effects

Milestone 3 should not:

- Redesign loader/profile semantics
- Redo library registry truthfulness
- Assert comprehensive Nmap equivalence

## Milestone 3 Closure Note

**Status:** Complete

### Wrapped Helper Classes (fully migrated through NseCapabilityContext)

- **Filesystem**: `io.rs`, `lfs.rs`, `os.rs` — all read/write/remove/rename/create operations route through capability checks
- **Process execution**: `os.rs`, `nmap.rs` — `std::process::Command` gated by `check_process_exec()`
- **Network TCP/UDP**: `socket.rs`, `comm.rs` — connect/send/receive gated by network capability checks
- **DNS**: `dns.rs` — resolver calls gated by `check_dns()`
- **Time**: `datetime.rs` — time access gated by `check_time()`
- **Randomness**: `rand.rs` — random byte generation gated by `check_randomness()`
- **Environment**: `os.rs` — environment variable reads gated by `check_environment()`
- **Compression**: `zlib.rs` — compress/decompress gated by `check_compression()` with size limits (64 MiB input, 256 MiB output)
- **Crypto/TLS**: `openssl.rs`, `tls.rs`, `sslcert.rs` — crypto operations gated by `check_crypto()`

### Partially Wrapped

- Protocol-specific libraries (smb, ssh, ftp, http, etc.) use socket.rs/comm.rs for network I/O but may have unmigrated helper calls within their protocol logic

### Deferred (not yet migrated)

- ~~`unpwdb.rs` — password database file reads~~ **Wrapped** (Milestone 5 Phase 04)
- `brute.rs` — brute force helper operations
- `datafiles.rs` — data file reads
- Protocol-specific internal helpers beyond network I/O

### Profile Behavior Summary

| Profile | FS Read | FS Write | Process Exec | Network | DNS | Env | Random | Time | Compression |
|---------|---------|----------|--------------|---------|-----|-----|--------|------|-------------|
| ManualPermissive | Allow (warn) | Allow (warn) | Allow (warn) | Allow (warn) | Allow (warn) | Allow (warn) | Allow (warn) | Allow (warn) | Allow (warn) |
| ManualStrict | Allow (policy) | Allow (policy) | Allow (policy) | Allow (policy) | Allow (policy) | Allow (policy) | Allow (policy) | Allow (warn) | Allow (policy) |
| AgentSafe | Allow | **Deny** | **Deny** | Allow | Allow | **Deny** | Allow (warn) | Allow | Allow |
| CiSafe | **Deny** | **Deny** | **Deny** | **Deny** | **Deny** | **Deny** | **Deny** | Allow (warn) | Allow |

### How Reports Expose Helper Decisions

`NseRunReport.capability_events` contains a serializable summary of each capability check performed during script execution. Each event records:
- `kind`: operation class (e.g., "filesystem_write", "process_exec")
- `operation`: specific helper (e.g., "io.write", "os.execute")
- `target`: path/host/command (where applicable)
- `allowed`: whether the operation was permitted
- `reason`: denial or warning reason

Helper denials affect `NseRunReport.compatibility` status. If any required helper is denied, compatibility degrades accordingly.

### What Milestone 4 Should Address

- Broader compatibility corpus coverage
- Upstream NSE script subset conformance testing
- Advanced service/port context fidelity
- Richer structured evidence reports
- UX polish for CLI/TUI report display
- Migration of deferred protocol-specific helpers

### What Remains Intentionally Different

- `ManualPermissive` allows all operations with warnings (intentional for interactive use)
- `AgentSafe` and `CiSafe` deny high-risk operations by default (intentional for automated safety)
- Static `require()` detection is approximate and labeled with warnings

### Milestone 2 Final Verification

**Date:** 2026-07-05 (hardening/polish pass)

| Command | Status | Tests | Notes |
|---------|--------|-------|-------|
| `cargo check -p eggsec-nse --features nse` | PASS | — | ~96 pre-existing warnings |
| `cargo test -p eggsec-nse --features nse` | PASS | 269 | 1 ignored |
| `cargo test -p eggsec-nse --features nse,sandbox` | PASS | 269 | 1 ignored |
| `cargo check -p eggsec --features nse` | PASS | — | ~100 pre-existing warnings |
| `cargo test -p eggsec --features nse --test nse_tests` | PRE-EXISTING FAIL | — | Type mismatch at nse_tests.rs:284 (3-tuple vs 2-tuple); not introduced by this pass |
| `bash scripts/check-architecture-guards.sh` | PASS | 32 checks | Check 25 strengthened; all pass |
| `cargo fmt --all --check` | PASS | — | — |
| `cargo clippy --lib -p eggsec-nse --features nse` | PASS | — | Pre-existing warnings only |
| `cargo clippy --lib -p eggsec --features nse` | PASS | — | Pre-existing warnings only |

Architecture guard Check 25 has been strengthened to reject both empty placeholders and all-registry-loaded fabrication patterns. Library report truthfulness is protected by 6 tests covering no-require, single-require, repeated-require, missing-require, static fallback, and fabrication rejection scenarios.

## Milestone 3 Corrective Pass (profile propagation)

**Date:** 2026-07-06
**Source plan:** `plans/nse-milestone-3-corrective-pass.md`

### Bug

`run_cli_with_profile()` previously constructed executors via `NseExecutor::with_policy(...)`, which hardcodes `ManualPermissive` and `AllowAllManual` in the capability context. This meant that even when a non-manual profile (e.g., `AgentSafe`) was provided, the capability context was constructed with manual-permissive defaults — defeating the purpose of profile-aware execution.

### Fix

`run_cli_with_profile()` now uses `NseExecutor::with_profile(&resolved_profile)` so the capability context matches the resolved profile. The fix also adds new constructors that give callers explicit control over the profile kind and network policy.

### New Constructors

| Constructor | Type | Purpose |
|-------------|------|---------|
| `NseExecutor::with_profile(profile)` | Sync executor | Constructs executor from a `ResolvedNseExecutionProfile`. Capability context, limits, and policies all derive from the profile. Preferred for CLI and automated surfaces. |
| `NseExecutor::with_full_policy(profile_kind, sandbox, limits, cancellation, script_policy, module_policy, network_policy)` | Sync executor | Explicit control over every policy parameter. Use when callers need to override individual policies without constructing a full `ResolvedNseExecutionProfile`. |
| `AsyncNseExecutor::with_full_policy(profile_kind, sandbox, limits, cancellation, script_policy, module_policy, network_policy)` | Async executor | Async counterpart of `with_full_policy`. Same explicit control over all policy parameters. |
| `ExecutorCore::with_full_policy(profile_kind, sandbox, limits, cancellation, script_policy, module_policy, network_policy)` | Core | Explicit control over all policies at the core level. Used by the executor constructors above. |

### Accessor

- `NseExecutor::capability_context()` — returns `&NseCapabilityContext` for callers that need to inspect or pass the capability context (e.g., library registration).

### AgentSafe Filesystem-Read Semantics

AgentSafe filesystem reads now follow **Option A: scoped reads only**. A filesystem read is allowed only when:
1. The path is under the sandbox `allowed_dir` (i.e., `SandboxConfig.enabled` with `allowed_dir` configured), **or**
2. The path is under an explicit `allowed_script_roots` or `allowed_module_roots` entry.

Unscoped filesystem reads (reading arbitrary paths outside any configured root) are denied under AgentSafe. This tightens the previous behavior where `agent_allow_if_scoped` did not enforce path containment for reads.

### New Integration Tests

New integration tests in `crates/eggsec-nse/tests/profile_propagation_tests.rs` verify:
- `run_cli_with_profile()` constructs a capability context matching the resolved profile
- AgentSafe profile denies unscoped filesystem reads
- `with_full_policy()` constructors produce correct policy states
- `capability_context()` accessor returns the expected context

### New Architecture Guards

| Guard | Check | Description |
|-------|-------|-------------|
| Check 35 | `run_cli_with_profile uses with_profile` | Verifies `run_cli_with_profile()` calls `NseExecutor::with_profile()` (not `with_policy()`) |
| Check 36 | `automated surfaces must not use with_policy` | Detects automated entry points (run_cli_with_profile, agent/MCP/REST paths) calling `with_policy()` which hardcodes ManualPermissive |
| Check 37 | `ExecutorCore::with_policy callers info` | Informational: lists all callers of `ExecutorCore::with_policy()` for audit |

### New Profile/Report Tests

End-to-end verification tests in `crates/eggsec-nse/tests/profile_report_tests.rs` exercise the full profile→context→event→report pipeline:

| Test | Profile | Capability | Verifies |
|------|---------|------------|----------|
| `agent_safe_process_exec_denied_in_report` | AgentSafe | ProcessExec | Denied; event in report with `allowed=false`; compatibility degrades to `Partial` |
| `agent_safe_unscoped_fs_read_denied_in_report` | AgentSafe | FilesystemRead | Denied without sandbox; event in report with `allowed=false` |
| `agent_safe_scoped_fs_read_allowed_in_report` | AgentSafe | FilesystemRead | Allowed with sandbox `allowed_dir`; event in report with `allowed=true` |
| `ci_safe_network_dns_denied_in_report` | CiSafe | NetworkTcp + DnsResolution | Both denied; events in report with `allowed=false`; compatibility `Partial` |
| `manual_permissive_process_exec_warning_in_report` | ManualPermissive | ProcessExec | Allowed with warning; event in report with `allowed=true` and warning reason |

## Milestone 3 Final Verification

**Date:** 2026-07-06 (closure verification pass)

| Command | Status | Tests | Notes |
|---------|--------|-------|-------|
| `cargo check -p eggsec-nse --features nse` | PASS | — | 0 errors, 98 pre-existing warnings |
| `cargo test -p eggsec-nse --features nse` | PASS | 369 | 1 ignored |
| `cargo test -p eggsec-nse --features nse --test profile_propagation_tests` | PASS | 7 | Profile→capability regression tests |
| `cargo test -p eggsec-nse --features nse --test profile_report_tests` | PASS | 5 | New end-to-end profile/report tests |
| `cargo test -p eggsec-nse --features nse --test compatibility_corpus_tests` | PASS | 14 | Corpus compatibility verification |
| `bash scripts/check-architecture-guards.sh` | PASS | 37 checks | All pass (33b/33c/34/37 are INFO-only) |
| `cargo fmt --all --check` | PASS | — | — |
| `cargo clippy --lib -p eggsec-nse --features nse` | PASS | — | Pre-existing warnings only |
| `cargo clippy --lib -p eggsec --features nse` | PASS | — | Pre-existing warnings only |

Architecture guard Checks 35 and 36 confirm the corrective-pass fix: `run_cli_with_profile()` uses `NseExecutor::with_profile()` (not `with_policy()`), and automated surfaces do not use manual-only constructors. Informational checks 33b/33c/37 document deferred migration targets (unpwdb, brute, datafiles, protocol-specific helpers).

## Milestone 4 Closure Verification

**Date:** 2026-07-06 (closure pass)

Milestone 4 expanded the compatibility corpus (39 fixtures across 9 categories) and added runtime-execution verification on top of the resolver-only static harness. The two harnesses are now structurally separated and each has a clear scope.

| Command | Status | Tests | Notes |
|---------|--------|-------|-------|
| `cargo check -p eggsec-nse --features nse` | PASS | — | 0 errors, pre-existing warnings only |
| `cargo test -p eggsec-nse --features nse` | PASS | 432 | 1 ignored; stable across 10 consecutive runs |
| `cargo test -p eggsec-nse --features nse --test runtime_corpus_tests` | PASS | 16 | Runtime execution + manifest assertion |
| `cargo test -p eggsec-nse --features nse --test runtime_smoke_tests` | PASS | 2 | End-to-end profile→report→envelope bridge |
| `cargo test -p eggsec-nse --features nse --test compatibility_corpus_tests` | PASS | 43 | Static resolver-only harness |
| `cargo test -p eggsec-nse --features nse --test compatibility_corpus_tests -- corpus_manifest` | PASS | 25 | Manifest-driven static assertions |
| `bash scripts/check-architecture-guards.sh` | PASS | 44 checks | All pass; Check 42/43/44 added for harness separation |
| `cargo fmt --all --check` | PASS | — | — |
| `cargo clippy -p eggsec-nse --features nse --tests` | PASS | — | Pre-existing warnings only |

### Harness Separation

- **Static harness** (`compatibility_corpus_tests.rs`, `mod corpus_manifest`) verifies resolver-level behavior only. It confirms script/module resolution, blocked-at-resolver status, file/module policies. It does **not** execute scripts.
- **Runtime harness** (`runtime_corpus_tests.rs`) drives every fixture through `NseExecutor::with_profile(&ResolvedNseExecutionProfile)`, injecting a synthetic host/port context, executing the script, capturing rule/library/capability reports, and asserting manifest expectations against observed behavior.
- **Smoke tests** (`runtime_smoke_tests.rs`) exercise the full pipeline (profile → context → execution → report → `ReportEnvelope` bridge) for representative scenarios.

### Known Limitations

- Rule-level fidelity for fixtures using injected synthetic port context is `Approximate`, not `Full`. This is by design — the rule evaluator downgrades fidelity when context source is `Synthetic`. Capability-denial fixtures (e.g. `process-denied`, `fs-read-denied`, `capability-fs-deny`) declare `expected_fidelity = "approximate"` to match.

### Milestone 4 Closure Verification (2026-07-06)

Final verification pass: 369 tests pass (1 ignored), architecture guards all pass (37 checks), fmt/clippy clean. New end-to-end profile/report tests in `crates/eggsec-nse/tests/profile_report_tests.rs` verify the profile→context→event→report pipeline for AgentSafe (process exec denial, unscoped/scoped FS read), CiSafe (network/DNS denial), and ManualPermissive (process exec warning). See [Milestone 3 Final Verification](./nse_integration.md#milestone-3-final-verification).

### Milestone 5 Phase 01: Runtime Corpus Flake Isolation (2026-07-06)

**Status:** Complete

**Problem:** `runtime_corpus_tests.rs` was flaky at default test parallelism (~16 threads). Symptom: fixtures like `error-portrule` and `process-denied` occasionally reported missing capability events or empty rule reports. Stable at `--test-threads=4`.

**Root cause:** `run_fixture_runtime()` used `std::process::id()` (PID) for temp dir naming. All 16 `#[test]` functions in the binary share the same PID. When two test functions executed the same fixture concurrently (e.g., `corpus_runtime_all_fixtures_execute_and_assert` and `corpus_runtime_unsupported_fixtures` both running `error-portrule`), they wrote to the same temp file path and interacted with the same shared Lua/library statics, causing races.

**Fix:** Added a global `AtomicU32` invocation counter. Each call to `run_fixture_runtime()` obtains a unique monotonic ID, producing temp dirs like `eggsec-nse-runtime-corpus-{fixture}-{pid}-{invocation_id}`. This prevents concurrent test functions from sharing file paths or interfering with each other's Lua VM state. Also upgraded `add_port` failure logging from `debug` to `warn` for visibility.

**Verification:** 10 consecutive runs at default parallelism, all 16 tests pass every time (previously flaky on ~40% of runs).

### Milestone 5 Phase 02: Strict Runtime Assertions (2026-07-06)

**Status:** Complete

**Problem:** Runtime corpus assertions were lenient — missing libraries were logged but not failed, missing rules were silently skipped, capability denials accepted resolver-block substitutes, and no evidence assertions existed. This allowed regressions to pass silently.

**Changes across 7 workstreams:**

1. **Manifest extensions** (`manifest.toml`): Added `required: bool` to `ExpectedCapabilityEvent`, `optional_libraries`/`optional_rules`/`expected_evidence_kinds`/`optional_evidence_kinds` to `FixtureEntry`, `allow_missing_runtime_libraries`/`allow_missing_runtime_rules` to `FixtureHarness`. Updated 3 representative fixtures with `required=true` and evidence expectations.

2. **Hard library assertions** (`runtime_corpus_tests.rs:707`): Default is now hard assert. `allow_missing_runtime_libraries = true` downgrades to soft (log-only). Empty library reports (short-circuited execution) pass regardless.

3. **Hard rule assertions** (`runtime_corpus_tests.rs:737`): Hard assert when fixture declares `[[fixture.ports]]` (portrule can fire). Skip when no ports (portrule cannot fire). `allow_missing_runtime_rules = true` always skips empty rules.

4. **Hard capability event assertions**: `required = true` hard asserts denial is observed (no resolver-block/error substitute). `required = false` (default) accepts resolver block or error as substitute.

5. **Evidence assertions** (new test `corpus_runtime_strict_evidence_assertions`): `expected_evidence_kinds` hard asserts each kind is present. `optional_evidence_kinds` logged as informational.

6. **Architecture guards**: Check 45 (no self-referential expected value construction — detects `Registry::all_libraries`/`LIBRARY_REGISTRY` in runtime tests). Check 46 (no trivially satisfiable assertions — detects `assert!(true)` and self-comparing patterns). Both pass.

7. **Compatibility matrix** (`docs/NSE_COMPATIBILITY.md`): Added verification mode columns (Libs/Rules/CapEvents/Evidence) to the Script/Pattern Compatibility table. Updated test count from 16 to 17.

**Verification:** 17 runtime corpus tests pass, 2 smoke tests pass, all 46 architecture guards pass, clippy clean.

### Milestone 5 Phase 03: Local Protocol Fixtures (2026-07-06)

**Status:** Complete

**Problem:** The compatibility corpus relied entirely on mocked protocol interactions and synthetic context. No fixtures exercised real TCP/HTTP/UDP local protocol connectivity through the NSE capability wrapper pipeline. This left a gap in verifying that network wrappers (TCP connect/send/receive, UDP send/receive, HTTP GET/POST) produce correct capability events and evidence when connected to real local services.

**Changes across 7 workstreams:**

1. **Local fixture harness** (`local_fixtures.rs`): Reusable test infrastructure with `TcpEchoServer`, `HttpServer`, and `UdpEchoServer` bound to `127.0.0.1` with dynamic ports. Each server runs in a background thread with graceful shutdown. Unit tests verify roundtrip behavior.

2. **NSE fixture scripts** (5 new files in `tests/fixtures/nse_corpus/scripts/protocol/`):
   - `tcp_connect_echo.nse` — socket.tcp() connect/send/receive against local TCP echo
   - `tcp_connect_denied.nse` — same pattern, expects denial under AgentSafe/CiSafe
   - `http_get_local.nse` — `http.get()` against local HTTP server, extracts `<title>`
   - `http_post_local.nse` — `http.post()` against local HTTP server
   - `udp_echo.nse` — socket.udp() sendto/receive_from against local UDP echo

3. **Runtime tests** (`local_protocol_tests.rs`): 21 tests exercising all local protocol fixtures through `NseExecutor::with_profile()` with real listeners. Covers TCP success/denial (AgentSafe, CiSafe), HTTP GET/POST success, UDP success/denial, DNS denial (AgentSafe), TLS/sslcert local fixtures (self-signed cert generation, `get_certificate`, `parse_cert`, `get_subject`, `get_chain_certs`, `is_valid`), report JSON roundtrip, envelope bridge, evidence extraction, and profile comparison.

4. **Manifest integration** (`manifest.toml`): 7 local protocol fixtures declared with `[local_service]` metadata (server type, dynamic ports). `expected_status = "compatible_with_warnings"` for all. Runtime corpus harness skips `local_service` fixtures (7 iteration sites updated).

5. **Runtime harness skip**: `runtime_corpus_tests.rs` added `LocalService` deserialization, `local_service: Option<LocalService>` field on `FixtureEntry`, and `entry.local_service.is_none()` filter in all 7 runtime iteration sites.

6. **Architecture guard**: Check 47 verifies local protocol fixtures declare `local_service` metadata, runtime harness has skip logic, and `local_protocol_tests.rs` exists.

7. **Known limitation resolved**: HTTP library (`reqwest`) capability bypass was resolved in Milestone 6 Phase 01 — all network operations now gated via `check_network_tcp()`, denied requests never reach reqwest. AgentSafe HTTP tests upgraded to strict denial assertions (server hits == 0).

**Verification:** 452 NSE tests pass (1 ignored), all 47 architecture guards pass, clippy clean.

### Milestone 5 Phase 04: Deferred Library Migration (2026-07-06)

**Status:** Complete

**Problem:** Several NSE libraries had unguarded side-effecting operations that bypassed `NseCapabilityContext`. The `unpwdb` library performed direct `std::fs::read_to_string()` calls without capability checks, and the `http` library's network checks were incomplete (reqwest calls bypassed the capability context). The `ssl` library's registry entry was stale (marked Deferred despite being wrapped since Milestone 3 Phase 05).

**Changes across 5 workstreams:**

1. **unpwdb.rs migration**: Added `&NseCapabilityContext` parameter to `register_unpwdb_library()`. Replaced all 6 direct `std::fs::read_to_string()` calls with `wrappers::nse_fs_read_to_string()`. Each closure clones the capability context. Filesystem reads now produce capability events and are denied under AgentSafe/CiSafe profiles. Registry updated from `Deferred` to `Wrapped`.

2. **http.rs migration**: Added `&NseCapabilityContext` parameter to `register_http_library()`. Added `denied_response()` helper function. Added `wrappers::check_network_tcp()` calls before every network-performing function (get, post, put, delete, head, options, request, post_host, put_data, async_get, async_post, async_request). Fixed compilation error: replaced `error_response(lua, reqwest::Error::from(std::io::Error::new(...)))` pattern with `denied_response(lua, reason)` across all 11 instances. Registry notes updated to document the capability check advisory pattern.

3. **executor_core.rs**: Updated both `register_http_library()` and `register_unpwdb_library()` calls to pass `&self.capability_context`.

4. **Registry updates**: `ssl` (tls.rs) updated from `Deferred` to `Wrapped` (stale entry from Milestone 3 Phase 05). `unpwdb` updated from `Deferred` to `Wrapped`. `http` updated from `PartiallyWrapped` to `Wrapped` (Milestone 6: all network operations gated via `check_network_tcp()`; denied requests never reach reqwest). Tests updated: `wrapped_libraries_include_known_wrapped` now asserts `unpwdb`, `ssl`, and `http`; `partially_wrapped_libraries` no longer asserts `http`.

5. **Documentation**: `docs/NSE_COMPATIBILITY.md` updated: http/ssl/unpwdb rows reflect new statuses, deferred count reduced from 14 to 12, Milestone 5 candidates updated to remove migrated libraries.

**Verification:** 0 compilation errors, 27 registry tests pass.

### Milestone 5 Phase 05: Report UX and Runtime Performance (2026-07-06)

**Status:** Complete

Phase 05 polished user-facing report output and improved runtime corpus usability.

#### WS1: CLI Report Formatting

- Created `crates/eggsec-nse/src/format.rs` — testable `format_human_report()` returning `String`
- Created `crates/eggsec-nse/tests/format_tests.rs` — 29 snapshot-lite tests asserting headings, status labels, visual markers
- Rewrote `print_human_report()` to delegate to formatter; fixed unused `target_str` bug
- Visual improvements: UPPERCASE status labels, `[!]` prefix for denials, `[*]` prefix for warnings, `~` prefix for approximate fidelity

#### WS2: TUI/Frontend Data Contract

- Created `architecture/nse_report_display_contract.md` — structured display model mapping `NseRunReport` fields to 7 display sections (Summary, Rules, Libraries, Capability Denials, Evidence, Raw Output, Diagnostics)
- Defines `ReportEnvelope` mapping for cross-domain aggregation
- Color/semantic mapping for status and fidelity levels

#### WS3: Runtime Corpus Performance Baseline

- Added `TimingEntry` struct and thread_local timing side channel to `runtime_corpus_tests.rs`
- Added `corpus_runtime_performance_baseline` test logging per-fixture timing and top 5 slowest
- Added `LazyLock<Manifest>` for manifest caching (avoids repeated TOML parsing)
- Replaced all 10 `load_manifest()` calls with `get_manifest()`

#### WS4: Performance Improvements

- Manifest parsed once via `LazyLock` for entire test binary lifetime
- Per-fixture executor isolation preserved (unique temp dirs via `AtomicU32` counter)

#### WS5: ReportEnvelope Bridge Hardening

- Extended `tests/evidence_tests.rs` (363→693 lines) — 7 new bridge tests: compatible/partial envelopes, denial severity, rule-error/raw-output evidence, weak-evidence-not-high-severity, metadata fields
- Created `tests/bridge_tests.rs` — 4 envelope shape tests: manifest counts, finding categories, multiple evidence, no circular deps

**Verification:** All tests pass, architecture guards pass, clippy clean.

### Milestone 5 Boundary

Milestone 5 Phase 05 is complete. Future work should not reopen report formatting, data contract, or performance baseline without a regression.

Candidates for future milestones:

- TUI rendering implementation (consume `NseRunReport` or `ReportEnvelope` per `nse_report_display_contract.md`)
- Additional upstream fixtures (currently 39 fixtures; representative coverage)
- Protocol library wrappers (ssh, smb, mysql, postgres, redis, mongodb, ldap, snmp)
- `stdnse.sleep()` cancellation integration

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

## Compatibility Matrix

The compatibility matrix summarizes the registry's 43 library descriptors. The authoritative source is `LIBRARY_REGISTRY` in `crates/eggsec-nse/src/resolver/registry.rs` — the table below is a representative subset covering all categories.

### Representative Subset (15 of 43)

| Library | Category | Fallback | Side Effects | Sandbox Posture | Known Gaps | Corpus |
|---------|----------|----------|-------------|-----------------|------------|--------|
| `stdnse` | Core | HardFail | None | Clean | Full fidelity | Covered |
| `nmap` | Core | HardFail | EnvAccess, NetworkAccess | Env+net restricted | `nmap.registry` stub; scan-state not live | Covered |
| `socket` | Protocol | HardFail | NetworkAccess | CIDR-filtered | TLS not natively exposed via socket API | Partial |
| `http` | Protocol | HardFail | NetworkAccess | CIDR-filtered | HTTP/2 not supported; cookie jar simplified | Partial |
| `dns` | Protocol | HardFail | NetworkAccess | CIDR-filtered | EDNS0 options limited; DNSSEC validation stubbed | Partial |
| `ssl` | Protocol | HardFail | NetworkAccess | CIDR-filtered | Requires `openssl` dep; cipher suite enumeration stubbed | Partial |
| `ssh` | Protocol | GracefulDegrade | NetworkAccess | CIDR-filtered | Requires `libssh2`; auth methods partial | Partial |
| `smb` | Protocol | GracefulDegrade | NetworkAccess | CIDR-filtered | NTLM auth only; SMBv1 signing incomplete | Partial |
| `vulns` | Exploit | Skip | NetworkAccess | CIDR-filtered | CVE lookup via NVD/OSV APIs; offline DB not bundled | Covered |
| `creds` | Auth | Skip | FileSystemRead, NetworkAccess | FS+net restricted | Credential iteration works; file-based wordlists sandboxed | Partial |
| `io` | Core | GracefulDegrade | FileSystemRead, FileSystemWrite, ProcessExecution | Heavily sandboxed | `popen` restricted to command allowlist; `tmpfile` denied | Covered |
| `lfs` | Core | GracefulDegrade | FileSystemRead, FileSystemWrite | Restricted to `allowed_dir` | Symlink checks enforced; `attributes` partial | Covered |
| `tab` | Utility | Skip | None | Clean | Pure utility; full fidelity | Covered |
| `json` | Utility | Skip | None | Clean | Encode/decode; no streaming parser | Covered |
| `pcre` | Utility | GracefulDegrade | None | Clean | Optional `pcre` dep; falls back to Lua patterns | Covered |

### Summary by Category

| Category | Count | HardFail | GracefulDegrade | Skip |
|----------|-------|----------|-----------------|------|
| Core | 7 | 3 (`stdnse`, `nmap`, `socket`) | 3 (`io`, `os`, `lfs`) | 1 (`target`) |
| Protocol | 13 | 4 (`socket`, `http`, `dns`, `ssl`) | 9 (`ssh`, `smb`, `smb2`, `mysql`, `postgres`, `redis`, `mongodb`, `ldap`, `snmp`, `openssl`, `comm`) | 0 |
| Utility | 15 | 0 | 1 (`pcre`) | 14 (all others) |
| Exploit | 1 | 0 | 0 | 1 (`vulns`) |
| Auth | 3 | 0 | 0 | 3 (`creds`, `unpwdb`, `brute`) |

### Side-Effect Summary

| Side Effect | Libraries |
|-------------|-----------|
| None | `stdnse`, `tab`, `json`, `base64`, `base32`, `bin`, `bit`, `stringaux`, `strbuf`, `nse_string`, `nse_table`, `pcre`, `shortport`, `match_lib`, `matchs`, `datetime`, `rand`, `url`, `unicode` |
| NetworkAccess | `nmap`, `socket`, `http`, `dns`, `ssl`, `ssh`, `smb`, `smb2`, `mysql`, `postgres`, `redis`, `mongodb`, `ldap`, `snmp`, `vulns`, `brute`, `openssl`, `comm`, `target`, `creds` |
| FileSystemRead | `lfs`, `creds`, `unpwdb` |
| FileSystemWrite | `io`, `lfs` |
| ProcessExecution | `io`, `os` |
| EnvAccess | `nmap`, `os` |

## Report Examples

`NseRunReport` (defined in `crates/eggsec-nse/src/report.rs`) is the structured output of an NSE script execution. Field names are illustrative — the schema follows the Rust struct definitions and may evolve.

`NseRunReport.libraries` is a per-run record of the libraries required or attempted by that execution, together with any diagnostics from the run. It does not describe the full capability set of Eggsec NSE support. In denied or blocked runs, the field may be empty because no script execution occurred.

Each library entry has a `loaded` field: `true` means the runtime observed a successful module load; `false` means a `require()` was attempted but the module failed, was blocked by policy, was missing, had an invalid name, or was detected via static analysis without runtime confirmation. Static `require()` detection is approximate and is labeled with a warning in the `warnings` array. Registry APIs describe available capability metadata, not per-run usage.

The `rules` array reflects real rule evaluation where available. Outcomes include evaluated+matched (boolean true), evaluated+not-matched (boolean false or nil), unsupported return types (non-boolean with `unsupported` field), and errored rules (lua error). See `NseRuleEvaluationReport` for the structured metadata contract.

### Example 1: Compatible Run with Warnings

A `ManualPermissive` run of a discovery script that resolved all modules but triggered profile warnings.

```json
{
  "target": "192.168.1.10",
  "script_name": "http-enum",
  "script_source": {
    "kind": "builtin",
    "label": "http-enum",
    "size": 0
  },
  "profile": {
    "kind": "manual-permissive",
    "audit_label": "nse:manual-permissive",
    "warnings": [
      "manual-permissive profile is not agent-safe"
    ]
  },
  "sandbox": {
    "enabled": true,
    "feature_compiled": true,
    "allowed_dir": "/tmp/eggsec-nse",
    "allowed_commands_count": 0,
    "allowed_networks_count": 1
  },
  "limits": {
    "wall_clock_timeout_secs": 120.0,
    "lua_instruction_budget": 100000000,
    "max_output_bytes": 5242880,
    "max_script_bytes": 1048576,
    "max_required_module_bytes": 1048576,
    "max_network_operations": 500,
    "max_filesystem_operations": 0,
    "max_lua_memory_bytes": 52428800
  },
  "stats": {
    "elapsed_secs": 2.34,
    "output_bytes": 1024,
    "lua_instruction_count": 45230,
    "network_operations": 3,
    "network_bytes_read": 4096,
    "network_bytes_written": 1024,
    "filesystem_operations": 0,
    "filesystem_bytes_read": 0,
    "limit_violation": null
  },
  "resolver": {
    "total_diagnostics": 3,
    "resolved_count": 3,
    "blocked_count": 0,
    "rejected_count": 0,
    "diagnostics": [
      { "kind": "resolved", "source": "stdnse", "detail": "1200 bytes" },
      { "kind": "resolved", "source": "http", "detail": "8400 bytes" },
      { "kind": "resolved", "source": "shortport", "detail": "600 bytes" }
    ]
  },
  "libraries": [
    {
      "name": "stdnse",
      "category": "Core",
      "registered": true,
      "side_effects": ["None"],
      "fallback_behavior": "HardFail",
      "notes": "Core output formatting",
      "loaded": true,
      "warnings": []
    },
    {
      "name": "http",
      "category": "Protocol",
      "registered": true,
      "side_effects": ["NetworkAccess"],
      "fallback_behavior": "HardFail",
      "notes": "HTTP client library",
      "loaded": true,
      "warnings": []
    },
    {
      "name": "shortport",
      "category": "Utility",
      "registered": true,
      "side_effects": ["None"],
      "fallback_behavior": "Skip",
      "notes": "Port number normalization",
      "loaded": true,
      "warnings": []
    }
  ],
  "rules": [
    {
      "kind": "portrule",
      "evaluated": true,
      "matched": true,
      "exactness": "exact",
      "error": null,
      "summary": "Port 80 open, http service detected"
    }
  ],
  "output": {
    "has_output": true,
    "content": "80/tcp   open  http    Apache/2.4.41\n443/tcp  open  ssl/http Apache/2.4.41",
    "line_count": 2,
    "truncated": false
  },
  "compatibility": {
    "status": "compatible-with-warnings",
    "fidelity": "full",
    "unsupported_features": [],
    "approximations": []
  },
  "warnings": [
    "manual-permissive profile is not agent-safe"
  ],
  "errors": []
}
```

### Example 2: Denied Agent-Safe Arbitrary Script File

An `AgentSafe` run where the resolver rejected an arbitrary script file per profile policy. No execution occurs — the report captures the denial.

```json
{
  "target": "10.0.0.5",
  "script_name": "custom-check.nse",
  "script_source": {
    "kind": "file",
    "label": "/home/user/scripts/custom-check.nse",
    "size": 2048
  },
  "profile": {
    "kind": "agent-safe",
    "audit_label": "nse:agent-safe",
    "warnings": []
  },
  "sandbox": {
    "enabled": true,
    "feature_compiled": true,
    "allowed_dir": "/tmp/eggsec-nse",
    "allowed_commands_count": 0,
    "allowed_networks_count": 1
  },
  "limits": {
    "wall_clock_timeout_secs": 15.0,
    "lua_instruction_budget": 5000000,
    "max_output_bytes": 2097152,
    "max_script_bytes": 262144,
    "max_required_module_bytes": 262144,
    "max_network_operations": 50,
    "max_filesystem_operations": 0,
    "max_lua_memory_bytes": 2097152
  },
  "stats": {
    "elapsed_secs": 0.0,
    "output_bytes": 0,
    "lua_instruction_count": 0,
    "network_operations": 0,
    "network_bytes_read": 0,
    "network_bytes_written": 0,
    "filesystem_operations": 0,
    "filesystem_bytes_read": 0,
    "limit_violation": null
  },
  "resolver": {
    "total_diagnostics": 1,
    "resolved_count": 0,
    "blocked_count": 1,
    "rejected_count": 0,
    "diagnostics": [
      {
        "kind": "blocked",
        "source": "/home/user/scripts/custom-check.nse",
        "detail": "agent-safe profile does not permit arbitrary script files"
      }
    ]
  },
  "libraries": [],
  "rules": [],
  "output": {
    "has_output": false,
    "content": "",
    "line_count": 0,
    "truncated": false
  },
  "compatibility": {
    "status": "failed",
    "fidelity": "unknown",
    "unsupported_features": [],
    "approximations": []
  },
  "warnings": [],
  "errors": [
    "Script file denied by policy: agent-safe profile does not permit arbitrary script files"
  ]
}
```

## Next Work: Milestone 7

Milestones 1–6 are now closed. Future work should:

- Treat the library registry, rule semantics report, structured reports, compatibility corpus, capability wrappers, HTTP library capability integration, and report UX/performance as closed unless regression tests reveal a defect.
- Build on `NseRunReport` and `NseRuleEvaluationReport` rather than bypass them.
- Expand corpus breadth and library behavior upgrades as separate scoped work.
- Address Milestone 7 candidates listed in the [Milestone 6 Closure Note](#milestone-6-closure-note).

The Milestone 7 plan should be written from the closure indices established in Milestones 1–6 without reopening previously closed contracts.

### Capability Inventory (Phase 01 Complete)

A complete capability inventory and risk classification exists at [`architecture/nse_capability_inventory.md`](./nse_capability_inventory.md). The inventory classifies all side-effecting NSE helper operations by:

- **Capability class** (filesystem_read/write, process_exec, network_tcp/udp, dns_resolution, tls_crypto, compression, time_clock, randomness, environment, pure_cpu)
- **Blocking risk** (none, low, medium, high)
- **Profile policy** (manual_allowed, agent_deny, ci_deny, agent_allow_if_scoped, ci_allow_local_only)
- **Accounting needs** (filesystem_operations, network_operations, process_operations, etc.)
- **Cancellation requirements** (pre-call checks needed)
- **Report events** (event types for NseRunReport integration)

Key findings from the inventory:

1. **4 libraries** have sandbox enforcement (socket, io, os, lfs)
2. **All protocol libraries** (~100+) bypass sandbox entirely
3. **`nmap.socket_*()` and `nmap.async_socket_*()`** bypass socket sandbox
4. **`stdnse.sleep()`** blocks the thread without cancellation checks
5. **`io.read()` and `io.write()`** have TOCTOU risks
6. **`nmap.is_admin()`/`nmap.is_privileged()`** execute shell commands without sandbox

Migration priority order:
1. Process execution (`io.popen`)
2. Filesystem write/delete/rename
3. Filesystem read outside roots
4. Network TCP/UDP (all protocol libs)
5. DNS lookups
6. Compression on untrusted inputs
7. Crypto/TLS blocking
8. Time/randomness/environment reads
9. Pure CPU helpers (no migration needed)

### Capability Context and Decision Engine (Phase 02 Complete)

`NseCapabilityContext` and decision engine (`capabilities.rs`) provide centralized policy enforcement for all side-effecting helpers. Key components:

- **`NseCapabilityKind`** — 11 operation classes (FilesystemRead/Write, ProcessExec, NetworkTcp/Udp, DnsResolution, TimeClock, Randomness, Crypto, Compression, Environment)
- **`NseCapabilityRequest`** — operation request with kind, target, bytes hint, operation name
- **`NseCapabilityDecision`** — Allow, Deny{reason}, AllowWithWarning{warning}
- **`NseCapabilityEvent`** — recorded event for report integration
- **`NseCapabilityContext`** — central context with profile-specific policy checks, cancellation, resource counters

Profile-specific behavior:
- **ManualPermissive**: allows everything, warns on risky ops (process exec, FS write)
- **ManualStrict**: denies process exec, enforces path roots on FS write, enforces network CIDRs
- **AgentSafe**: denies process exec + FS write, scope-only network + DNS
- **CiSafe**: denies process exec + FS write + all network + DNS
- **CompatibilityLab**: allows with warnings, sandbox network check

`NseCapabilityEvent` integration into `NseRunReport.capability_events` — denied operations affect compatibility status (`Partial`). Pilot wrappers in `wrappers.rs` demonstrate the pattern. `ExecutorCore` stores the capability context, constructed from `with_full_policy()` (canonical), `with_profile()` (profile-derived), or `with_policy()` (manual-only compatibility). Architecture guards detect direct high-risk ops in NSE libraries (informational, will tighten as wrappers migrate).

### NSE Capability Context

`NseCapabilityContext` (defined in `capabilities.rs`) provides centralized policy enforcement for all side-effecting NSE helper operations. It evaluates each operation against the active execution profile and returns Allow, Deny, or AllowWithWarning.

**Core decision flow:**
1. Helper calls wrapper function (e.g., `check_network_tcp()`)
2. Wrapper constructs `NseCapabilityRequest` with operation details
3. `NseCapabilityContext::check_capability()` evaluates against profile policy
4. Decision returned: Allow, Deny{reason}, or AllowWithWarning{warning}
5. Event recorded in `NseCapabilityEvent` for run report

**Integration points:**
- `ExecutorCore` stores the capability context (constructed in `with_policy()` or `with_profile()`)
- `NseRunReport` includes `capability_events` and `capability_event_summary`
- `wrappers.rs` contains pilot wrapper functions demonstrating the pattern

**Migration status:**
- TimeClock, FilesystemRead, FilesystemWrite, NetworkTcp, NetworkUdp, ProcessExec, DnsResolution, Environment, Compression, Crypto, Randomness: wrapped (Phase 03–05)
- All side-effecting helper classes are now migrated

**Architecture guard:** Check 33 (FAIL) detects direct `std::process::Command` in NSE libraries (all process exec migrated); Check 33b (informational) detects direct filesystem ops in unmigrated libraries; Check 33c (informational) detects direct network calls in unmigrated libraries; Check 34 (informational) verifies capability context integration.

### Filesystem and Process Wrappers (Phase 03 Complete)

Phase 03 migrated filesystem and process operations through `NseCapabilityContext`. All side-effecting helpers in the core libraries now route through capability wrappers before performing the actual operation.

#### Migrated Libraries

| Library | Operations Migrated | Wrapper Functions Used |
|---------|--------------------|-----------------------|
| `io.rs` | `io.open()`, `io.read()`, `io.lines()`, `io.popen()`, `io.tmpfile()`, `io.write()` | `check_fs_read()`, `check_fs_write()`, `check_process_exec()`, executing wrappers (`nse_fs_read_to_string`, `nse_fs_write`, etc.) |
| `lfs.rs` | `lfs.attributes()`, `lfs.dir()`, `lfs.mkdir()`, `lfs.rmdir()`, `lfs.remove()`, `lfs.rename()`, `lfs.link()`, `lfs.touch()`, `lfs.set_mode()`, `lfs.chdir()`, `lfs.symlinkattributes()` | `check_fs_read()`, `check_fs_write()` via `NseCapabilityContext::check_capability()` |
| `os.rs` | `os.remove()`, `os.rename()` | `check_fs_write()` |
| `nmap.rs` | `nmap.is_admin()`, `nmap.is_privileged()` | `check_process_exec()` |

#### Profile-Specific Behavior

| Profile | Process Exec | Filesystem Write | Notes |
|---------|-------------|-----------------|-------|
| `ManualPermissive` | Allow with warning | Allow with warning | Warns on process exec and FS write; accounting only |
| `ManualStrict` | Allow (sandboxed popen) | Allow within roots | `get_allowed_path()` enforced; process exec via `is_command_allowed()` |
| `AgentSafe` | **Deny** | **Deny** | No process execution, no filesystem writes |
| `CiSafe` | **Deny** | **Deny** | No process execution, no filesystem writes |
| `CompatibilityLab` | Allow with warning | Allow with warning | Includes nmap paths; sandbox checks |

#### Executing Wrappers

Phase 03 introduced executing wrappers in `wrappers.rs` that combine capability checking with the actual filesystem operation. These wrappers handle cancellation checks, capability evaluation, resource counter updates, and event recording:

- `nse_fs_read_to_string()` — read file contents with FS read check
- `nse_fs_read()` — read file bytes with FS read check
- `nse_fs_write()` — write bytes with FS write check
- `nse_fs_metadata()` — stat file with FS read check
- `nse_fs_read_dir()` — list directory with FS read check
- `nse_fs_remove_file()` — delete file with FS write check
- `nse_fs_remove_dir()` — delete directory with FS write check
- `nse_fs_create_dir()` — create directory with FS write check
- `nse_fs_rename()` — rename with FS write check
- `nse_process_exec()` — execute command with process exec check

Libraries that accept `&NseCapabilityContext` in their registration function pass it to closures for use in capability checks. The context is cloned per-closure as needed.

#### Architecture Guard

Check 33 now **fails** for direct `std::process::Command` in NSE library files (outside `wrappers.rs`, `executor_core.rs`, and `tests/`), since all process execution is migrated. Check 33b remains informational for direct filesystem ops in libraries not yet fully migrated (e.g., `unpwdb`, `brute`, `datafiles`).

### Network/DNS Wrappers (Phase 04 Complete)

Phase 04 migrated network TCP/UDP and DNS resolution through `NseCapabilityContext`. Libraries performing network I/O now route through capability wrappers before performing the actual operations.

#### Migrated Libraries

| Library | Operations Migrated | Wrapper Functions Used |
|---------|--------------------|-----------------------|
| `socket.rs` | `socket.tcp_connect()`, `socket.connect()`, `socket.connect_udp()`, `socket.send()`, `socket.receive()`, `socket.sendto()`, `socket.receive_from()` | `nse_network_tcp_connect`, `nse_network_tcp_send`, `nse_network_tcp_receive`, `nse_network_udp_send`, `nse_network_udp_receive`, `check_network_udp` |
| `comm.rs` | `comm.get_banner()`, `comm.exchange()`, `comm.tryssl()` | `nse_network_tcp_connect`, `nse_network_tcp_send`, `nse_network_tcp_receive` |
| `dns.rs` | `dns.resolve()`, `dns.query()`, `dns.forward()`, `dns.ptr()` | `nse_dns_lookup` |

#### Executing Wrappers

Phase 04 introduced network/DNS executing wrappers in `wrappers.rs` that combine capability checking with the actual network operation. These wrappers handle cancellation checks, capability evaluation, resource counter updates, and event recording:

- `nse_network_tcp_connect()` — TCP connect with network TCP check
- `nse_network_tcp_send()` — TCP send with network TCP check and bytes accounting
- `nse_network_tcp_receive()` — TCP receive with network TCP check and bytes accounting
- `nse_network_udp_send()` — UDP send with network UDP check and bytes accounting
- `nse_network_udp_receive()` — UDP receive with network UDP check and bytes accounting
- `nse_dns_lookup()` — DNS resolution with DNS resolution check
- `check_network_udp()` — Check-only function for UDP operations (no executing wrapper)

#### Library Registration Changes

- `register_socket_library()` now accepts `capability_ctx: Option<NseCapabilityContext>` and passes it to closures for network operations
- `register_comm_library()` now accepts `capability_ctx: Option<NseCapabilityContext>` and passes it to closures for banner/exchange operations
- `register_dns_library()` now accepts `capability_ctx: Option<NseCapabilityContext>` and passes it to closures for DNS resolution

#### Profile-Specific Behavior

| Profile | Network TCP | Network UDP | DNS | Notes |
|---------|------------|-------------|-----|-------|
| `ManualPermissive` | Allow with warning | Allow with warning | Allow with warning | Accounting only; warns on network ops |
| `ManualStrict` | Allow within CIDRs | Allow within CIDRs | Allow within CIDRs | Scope-derived CIDR enforcement |
| `AgentSafe` | Allow if scoped | Allow if scoped | Allow if scoped | Only resolved target IPs |
| `CiSafe` | **Deny** | **Deny** | **Deny** | Zero network operations |
| `CompatibilityLab` | Allow with warning | Allow with warning | Allow | Full access for compat testing |

#### Architecture Guard

Check 33c (informational) detects direct network calls (TCP connect, UDP sendto, DNS resolution) in unmigrated library files. This check will tighten as more protocol libraries are migrated.

### Time/Randomness/Environment/Compression Wrappers (Phase 05 Complete)

Phase 05 migrated time, randomness, environment, crypto, and compression operations through `NseCapabilityContext`. Libraries performing these operations now route through capability wrappers before performing the actual operations.

#### Migrated Libraries

| Library | Operations Migrated | Wrapper Functions Used |
|---------|--------------------|-----------------------|
| `datetime.rs` | `datetime.now()`, `datetime.clock()`, `datetime.date()`, `datetime.time()` | `nse_time_now`, `check_time_clock` |
| `rand.rs` | `rand.random()`, `rand.num_range()`, `rand.random_string()`, `rand.seed()` | `nse_random_bytes`, `check_randomness` |
| `openssl.rs` | OpenSSL crypto operations, certificate handling | `check_crypto` |
| `tls.rs` | TLS connection setup, cipher suite operations | `check_crypto` |
| `sslcert.rs` | SSL certificate parsing and validation | `check_crypto` |
| `zlib.rs` | `zlib.compress()`, `zlib.decompress()` | `nse_compress`, `nse_decompress`, `check_compression` |

#### Executing Wrappers

Phase 05 introduced time/randomness/environment/compression executing wrappers in `wrappers.rs` that combine capability checking with the actual operation:

- `nse_time_now()` — wall-clock time read with time clock check
- `nse_random_bytes()` — random byte generation with randomness check
- `nse_env_var()` — environment variable read with environment check
- `nse_compress()` — compression with compression check and 64 MiB input limit
- `nse_decompress()` — decompression with compression check and 256 MiB output limit

#### Check-Only Wrappers

- `check_randomness()` — policy check for randomness operations (no executing wrapper)
- `check_environment()` — policy check for environment variable access (no executing wrapper)
- `check_crypto()` — policy check for crypto/TLS operations (no executing wrapper)
- `check_compression()` — policy check for compression operations (no executing wrapper)

#### Library Registration Changes

- `register_datetime_library()` now accepts `capability_ctx: Option<NseCapabilityContext>` and passes it to closures for time operations
- `register_rand_library()` now accepts `capability_ctx: Option<NseCapabilityContext>` and passes it to closures for randomness operations
- `register_openssl_library()` now accepts `capability_ctx: Option<NseCapabilityContext>` and passes it to closures for crypto operations
- `register_zlib_library()` now accepts `capability_ctx: Option<NseCapabilityContext>` and passes it to closures for compression operations

#### Profile-Specific Behavior

| Profile | Time | Randomness | Environment | Compression | Crypto | Notes |
|---------|------|------------|-------------|-------------|--------|-------|
| `ManualPermissive` | Allow | Allow | Allow (NSE_ENV only) | Allow | Allow | Accounting only; time nondeterminism allowed |
| `ManualStrict` | Allow | Allow | Allow (NSE_ENV only) | Allow | Allow within CIDRs | Scope-derived CIDR enforcement for crypto |
| `AgentSafe` | Allow (warn nondeterminism) | Allow (warn) | **Deny** | Allow | Allow if scoped | Environment access denied; randomness warned |
| `CiSafe` | Allow (warn nondeterminism) | **Deny** | **Deny** | Allow | **Deny** | Environment and randomness denied; time warned |
| `CompatibilityLab` | Allow | Allow | Allow | Allow | Allow | Full access for compat testing |

#### Compression Limits

- Input limit: 64 MiB (67,108,864 bytes)
- Output limit: 256 MiB (268,435,456 bytes)
- Limits enforced before compression/decompression; exceeded limits return `NseCapabilityDecision::Deny`

#### Architecture Guard

Check 33d (informational) detects direct crypto/compression operations in unmigrated library files. This check will tighten as protocol-specific libraries are migrated.

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

## Compatibility Corpus

A representative corpus of NSE script fixtures verifies supported, partial, approximate, unsupported, denied, and errored behavior. The corpus makes compatibility claims testable and prevents overclaiming Nmap parity. Milestone 4 Phase 01 expanded the corpus from 18 individual tests into a data-driven regression suite with 21 fixtures organized by category. Phase 02 added upstream-style fixtures with provenance tracking and gap classification. Phase 03 added host/port/service context fidelity fixtures.

### Location

- **Fixtures**: `crates/eggsec-nse/tests/fixtures/nse_corpus/` — minimal `.nse` and `.lua` files exercising distinct compatibility paths
- **Manifest**: `crates/eggsec-nse/tests/fixtures/nse_corpus/manifest.toml` — data-driven fixture registry with expected status, fidelity, libraries, rules, capability events, provenance, and gap classification per fixture
- **Tests**: `crates/eggsec-nse/tests/compatibility_corpus_tests.rs` — 18 legacy individual tests + 25 data-driven harness tests gated on `#[cfg(feature = "nse")]`, plus `tests/context_fidelity_tests.rs` — 8 context fidelity unit tests

### Corpus Categories

| Category | Fixtures | Description |
|----------|----------|-------------|
| discovery | 8 | Script rule types: portrule, hostrule, prerule, postrule, no-require, portrule(host,port), hostrule context, service context |
| version | 1 | Service version detection pattern |
| default | 3 | Core module usage: builtin require, stdnse output, vulns |
| protocol | 2 | HTTP title mock, DNS lookup mock |
| auth | 1 | Credential shape (brute-force pattern) |
| partial | 1 | Approximate compatibility warning |
| unsupported | 6 | Agent denied, process/fs denied, non-boolean rule, false/error portrule |
| regression | 2 | Capability fs-deny, compression bounded |
| upstream | 16 | Upstream-style patterns: shortport, sslcert, http, dns, vulns, stdnse tables, banners, etc. |

### Provenance Tracking

Every fixture declares provenance metadata:
- `provenance`: `clean-room` (original) or `upstream-derived` (pattern from Nmap)
- `upstream_reference`: description of the upstream pattern tested
- `license_note`: always "No upstream source copied" for clean-room fixtures
- `local_fixture`: `true` — all fixtures are local-only
- `public_network_required`: `false` — no fixtures require public network access

### Gap Classification

Every fixture declares a gap classification:
- `supported` — fully supported behavior in Eggsec
- `approximate` — supported with approximations or warnings
- `capability_denied` — blocked by capability context (e.g., AgentSafe process exec)
- `missing_library` — library not implemented in Eggsec (e.g., ssh2 under nse-ssh2)
- `context_gap` — behavior depends on runtime context not available in harness
- `unsupported_runtime` — Lua runtime limitation or Nmap-specific API

### Data-Driven Harness

The corpus harness (`corpus_harness` module in `compatibility_corpus_tests.rs`) loads `manifest.toml`, iterates fixtures, and asserts semantic report fields:

- **Status**: `compatible`, `compatible_with_warnings`, `partial`, `unsupported`, `failed`
- **Fidelity**: `full`, `approximate`, `minimal`
- **Resolution**: script resolved or blocked by policy
- **Libraries**: expected `require()` entries (name, loaded, registered)
- **Rules**: expected rule evaluations (kind, evaluated, matched, exactness)
- **Capability events**: expected denials/warnings (kind, allowed)
- **Provenance**: fixture provenance metadata present and valid
- **Gap classification**: gap classification is one of the defined categories

Harness tests: `loads_manifest`, `fixture_files_exist`, `manifest_parse_roundtrip`, `all_fixtures_execute`, per-category tests (including `upstream`), `capability_event_summary_fields`, `rule_report_fields`, `library_report_fields`, `diagnostics_threaded`, `capability_event_with_bytes`, `report_identity_fields`, `rejects_unknown_status`, `rejects_unknown_fidelity`, `provenance_checks`, `gap_classification_valid`, `upstream_local_only`, `fixture_count_range`.

### Adding New Cases

1. Add fixture `.nse` or `.lua` to `tests/fixtures/nse_corpus/scripts/<category>/`
2. Add entry to `manifest.toml` with `id`, `name`, `category`, `path`, `profile`, expected fields, provenance, and gap classification
3. Run the data-driven harness: `cargo test -p eggsec-nse --features nse --test compatibility_corpus_tests -- corpus_harness`
4. For protocol tests requiring services, add local mock servers in-process (TCP/UDP echo)

### Milestone 4 Phase 02: Upstream Subset Validation

Phase 02 adds deterministic validation against a curated upstream-style NSE subset. The goal is to verify that Eggsec's NSE engine handles the most common Nmap API patterns correctly, without copying upstream code.

**Selection criteria** (10-25 fixtures per plan; 16 implemented):
- Patterns from Nmap's most common NSE categories (discovery, default, safe, vuln)
- API surface coverage: shortport, sslcert, http, dns, vulns, stdnse output/args
- All fixtures are clean-room reimplementations — no upstream source copied

**What was added**:
- 16 new upstream-style fixtures in `scripts/upstream/` covering: shortport patterns, sslcert, HTTP GET/POST, DNS reverse lookup, stdnse args/output tables, hostname hostrule, graceful degradation, vulns, brute categories, nmap.fetch_file, structured output, banner parsing
- Provenance metadata on all 37 fixtures (clean-room/upstream-derived, reference, license)
- Gap classification on all 37 fixtures (supported/approximate/capability_denied/missing_library/context_gap/unsupported_runtime)
- 4 new validation tests: provenance checks, gap classification validation, upstream local-only constraints, fixture count range (10-25)
- Regression guard (Check 38) in `scripts/check-architecture-guards.sh` verifying all fixtures are local-only

### Milestone 4 Phase 03: Host, Port, and Service Context Fidelity

Phase 03 introduces structured context types for host, port, and service data, replacing raw Lua table construction with typed builders. The goal is to ensure `hostrule(host)`, `portrule(host, port)`, and `action(host, port)` receive correctly-shaped Lua tables matching Nmap's API contract, with provenance tracking for context sources.

**Context types** (in `context.rs`):
- `NseContextSource` — enum: `Scan`, `Fixture`, `Synthetic`, `Unknown` — provenance for context data
- `NseHostContext` — structured host data: `ip`, `hostname`, `target_label`, `source` + `to_table()`, `from_host_info()`, `synthetic()`
- `NsePortContext` — structured port data: `port`, `protocol`, `state`, `service: Option<NseServiceContext>`, `source` + `to_table()`, `from_port_info()`, `minimal()`
- `NseServiceContext` — service metadata: `name`, `product`, `version`, `tunnel`, `confidence` + `to_table()` + `Default`

**Lua table construction** (in `executor.rs`):
- `hostrule` receives a structured host table from `NseHostContext::to_table()` (not raw `nmap` global)
- `portrule` receives `(host_table, port_table)` matching Nmap's `portrule(host, port)` signature
- `action()` after hostrule: `action(host_table)`; after portrule: `action(host_table, port_table)`
- `evaluate_rule_with_context()` annotates reports with context source provenance

**Rule report fidelity fields** (in `report.rs`):
- `host_context_source: Option<String>` — provenance of host context (e.g., "synthetic", "scan")
- `port_context_source: Option<String>` — provenance of port context
- `service_context_available: Option<bool>` — whether service sub-table was present
- `fidelity_reason: Option<String>` — why exactness was downgraded (e.g., "synthetic host context")
- `evaluate_rule_with_context()` — constructs reports with context annotations; synthetic contexts downgrade exactness to "approximate"

**What was added**:
- 4 new context types in `context.rs` with Lua table builders and provenance tracking
- 3 new corpus fixtures: `portrule_host_port.nse`, `hostrule_host_context.nse`, `portrule_service_context.nse`
- 8 new unit tests in `tests/context_fidelity_tests.rs`
- Manifest entries with `gap_classification = "approximate"` and `expected_fidelity = "approximate"`
- 402 tests pass (1 ignored), 0 compilation errors

### Milestone 4 Phase 04: Structured Evidence Reports

Phase 04 introduces structured evidence extraction from NSE run reports, bridging NSE execution results into the normalized `ReportEnvelope` used by cross-domain reporting.

**Evidence model** (in `report.rs`):
- `NseEvidenceKind` — 8-variant enum: `ServiceFingerprint`, `VersionInfo`, `CertificateInfo`, `VulnerabilitySignal`, `Misconfiguration`, `CapabilityDenial`, `CompatibilityWarning`, `ScriptOutput`
- `NseEvidenceItem` — structured evidence record: `id`, `kind`, `title`, `summary`, `target`, `port`, `service`, `confidence` (0.0–1.0), `source`, `raw_excerpt`, `references`, `tags`
- `NseRunReport.evidence: Vec<NseEvidenceItem>` — evidence field on the canonical run report
- `extract_evidence()` — conservative extraction function operating on `NseRunReport` fields: capability denials → `CapabilityDenial`, unsupported features → `CompatibilityWarning`, approximate rules → `CompatibilityWarning`, rule errors → `Misconfiguration`, script output with service/version signals → `ServiceFingerprint`/`VersionInfo`

**Report envelope bridge** (in `bridge.rs`):
- `to_report_envelope()` — converts `NseRunReport` → `ReportEnvelope` following the db-pentest bridge pattern
- Maps `NseEvidenceKind` → `eggsec_output::envelope::EvidenceKind` and `eggsec_core::Severity`
- Attaches `ToolMetadata { tool_name: "eggsec-nse", ... }`
- Calls `envelope.refresh_evidence_manifest()` before return

**What was added**:
- `NseEvidenceKind` enum, `NseEvidenceItem` struct, `extract_evidence()` in `report.rs`
- `evidence` field on `NseRunReport` with `with_evidence()` builder
- `extract_evidence()` wired into `run_cli_with_profile()` JSON path
- `bridge.rs` module with `to_report_envelope()` and mapping functions
- `eggsec-core` and `eggsec-output` dependencies in `Cargo.toml`
- 12 evidence tests in `tests/evidence_tests.rs` — all pass

### Milestone 4 Closure Note

Milestone 4 is complete. The following summarizes all Phase 01–05 deliverables:

**Corpus expansion (Phase 01)**: 40 total fixtures across 5 categories (12 core, 9 partial/unsupported/regression, 16 upstream-style, 3 context fidelity). All local-only, with provenance tracking and gap classification.

**Upstream subset validation (Phase 02)**: 16 upstream-style fixtures covering shortport, HTTP, DNS, stdnse output, vulns, brute, and banner parsing patterns. No upstream Nmap code is copied.

**Context fidelity (Phase 03)**: 3 context fidelity fixtures validating portrule/hostrule host, port, and service context injection. Synthetic context with `eggsec_context_source` provenance tracking.

**Structured evidence reports (Phase 04)**: `NseEvidenceKind` (8 variants), `NseEvidenceItem`, `extract_evidence()` for conservative extraction from capability events, compatibility, rules, and output. `bridge.rs` maps NSE evidence to `ReportEnvelope`. 12 evidence tests pass.

**CLI UX (Phase 05)**: `print_human_report()` produces human-readable output showing profile, compatibility, rule evaluation, library use, capability denials, evidence, errors, warnings, and raw output (truncated at 20 lines).

**Compatibility matrix**: `docs/NSE_COMPATIBILITY.md` — 43 library entries, 40 corpus fixtures, profile compatibility, known gaps, and Milestone 5 candidates.

**Architecture guards**: 3 new checks (39–41) for evidence extraction consistency, bridge module existence, and compatibility matrix presence.

**Test results**: 414 tests pass (1 ignored), 41 architecture guards (38 existing + 3 new), `cargo fmt` clean.

**Deferred to Milestone 5**:
- Protocol library wrappers (ssl, ssh, smb, mysql, postgres, redis, mongodb, ldap, snmp)
- Authentication library wrappers (creds, unpwdb, brute)
- `stdnse.sleep()` cancellation integration
- Structured Lua output table parsing
- TUI-first compatibility debugging workflow
- Performance/caching for large corpus runs
- Total: 414 tests (0 failures, 1 ignored)

---

## Milestone 5 Final Verification (2026-07-06)

16-command verification matrix executed after all Phase 01–05 work:

| # | Command | Result | Notes |
|---|---------|--------|-------|
| 1 | `cargo check -p eggsec-nse --features nse` | PASS | |
| 2 | `runtime_corpus_tests --test-threads=1` | PASS | 18 tests, 0.57s sequential |
| 3 | `runtime_corpus_tests --test-threads=4` | PASS | 18 tests, 0.17s parallel |
| 4 | `runtime_corpus_tests` (default) | PASS | 18 tests, 0.14s |
| 5 | `runtime_smoke_tests` | PASS | 2 tests |
| 6 | `compatibility_corpus_tests` | PASS | 43 tests |
| 7 | `evidence_tests` | PASS | 19 tests |
| 8 | `context_fidelity_tests` | PASS | 8 tests |
| 9 | `cargo test -p eggsec-nse --features nse` (all) | PASS | 493 passed, 1 ignored |
| 10 | `cargo test -p eggsec --features nse --test nse_tests` | PASS | 174 passed |
| 11 | `cargo check -p eggsec --features nse` | PASS | |
| 12 | `cargo test -p eggsec --features nse --test feature_matrix` | PASS | 352 passed |
| 13 | Architecture guards | PASS | 47 checks |
| 14 | `cargo fmt --all --check` | PASS | |
| 15 | `cargo clippy --lib -p eggsec-nse --features nse` | PASS | 99 pre-existing warnings, no new |
| 16 | `cargo clippy --lib -p eggsec --features nse` | PASS | Pre-existing warnings only |

### Bug Fixes in Phase 06

**`test_nse_prerule_postrule`** (nse_tests.rs): prerule/postrule functions must return booleans (`true`) for `evaluate_rule()` to set `evaluated: true`. String returns produce "unsupported" reports that are silently dropped by the executor guard. Fixed to use `stdnse.register_prerule(func)` / `stdnse.register_postrule(func)` with boolean returns.

**`local_http_get_agent_safe_documentation`** (local_protocol_tests.rs): Updated assertion to match actual AgentSafe output (`"HTTP GET failed"` not `"HTTP request failed"`). HTTP library now fully capability-gated — resolved in Milestone 6 Phase 01.

### Final Counts

- **eggsec-nse lib tests**: 493 passed, 1 ignored
- **eggsec nse_tests**: 174 passed
- **eggsec feature_matrix + enforcement_matrix**: 352 passed
- **Architecture guards**: 47 checks (all pass)
- **Total across all binaries**: 1,019+ tests pass

### Milestone 5 Closure Summary

| Phase | Content | Status |
|-------|---------|--------|
| 01 | CLI report formatting (`format.rs`) + 29 snapshot tests | Closed |
| 02 | Strict runtime assertions + manifest caching (`LazyLock`) | Closed |
| 03 | Local protocol fixtures (TCP/HTTP/UDP) + 16 runtime tests | Closed |
| 04 | Deferred library migration (unpwdb→Wrapped, http→Wrapped) | Closed |
| 05 | ReportEnvelope bridge + 19 evidence tests + 4 envelope shape tests | Closed |
| 06 | Release closure: verification matrix, bug fixes, documentation | Closed |

**Milestone 5 is closed.** All 16 verification commands pass. Remaining deferred items (protocol library wrappers, `stdnse.sleep()` cancellation, TUI-first debugging) are candidates for Milestone 6.

### Milestone 6 Phase 01: HTTP Capability Bypass and Runtime Strictness (2026-07-06)

**Status:** Complete

Phase 01 closed the HTTP capability bypass gap and tightened runtime test strictness.

#### Changes

1. **http.rs promoted to Wrapped**: All 12 network-performing HTTP functions already called `check_network_tcp()` before reqwest. This phase formalized the status: denied requests return `denied_response()` without reaching reqwest. Registry entry updated from `PartiallyWrapped` to `Wrapped`.

2. **Atomic hit counters**: `HttpServer` in `local_fixtures.rs` gained `Arc<AtomicUsize>` hit counters. Every accepted connection increments the counter. `hits()` accessor enables test assertions proving denied requests don't reach the server.

3. **Strict AgentSafe HTTP assertions**: `local_http_get_agent_safe_documentation` replaced permissive "may succeed or fail" with three strict checks: script completes, capability events contain `network_tcp` denial, server hits == 0. New `local_http_get_ci_safe_denied` test with identical strict assertions under CiSafe profile.

4. **Tightened runtime library assertions**: `corpus_runtime_observed_libraries_match_expected()` replaced lenient `report.libraries.is_empty() || found` with hard `found` assertion. `allow_missing_runtime_libraries` soft path preserved for fixtures with legitimate runtime gaps (e.g., builtin modules not tracked by `require()`).

5. **Architecture guards 48-50**: Check 48 verifies `http.rs` has ≥5 `check_network_tcp` calls. Check 49 ensures no lenient permissive text for AgentSafe HTTP tests. Check 50 guards against reintroduction of lenient `is_empty() || found` patterns.

6. **Documentation updates**: Registry, NSE_COMPATIBILITY.md, nse_integration.md, AGENTS.md, AGENTS.override.md, and SKILL.md all updated to reflect `http` as `Wrapped`.

**Verification:** 494 NSE tests pass (1 ignored), 50 architecture guards pass.

### Milestone 6 Phase 02: HTTP Method Coverage and Guard Hardening (2026-07-07)

**Status:** Complete

Phase 02 extended the HTTP capability-bypass proof across all HTTP network methods and replaced coarse guards with method/path-specific guardrails.

#### Changes

1. **HttpServer method/path tracking**: `HttpServer` in `local_fixtures.rs` gained `last_method` and `last_path` tracking via `Arc<Mutex<Option<String>>>`. New accessors `last_method()` and `last_path()` enable precise test assertions.

2. **Local fixture scripts for all HTTP methods**: Added `http_put_local.nse`, `http_delete_local.nse`, `http_head_local.nse`, `http_options_local.nse`, and `http_request_local.nse` under `scripts/protocol/`. Each exercises the corresponding HTTP library method against a local server.

3. **ManualPermissive success tests for all methods**: PUT, DELETE, HEAD, OPTIONS, and generic `request` now have ManualPermissive success tests proving real local server contact with hit count and method assertions.

4. **AgentSafe/CiSafe zero-hit denial tests**: Every covered HTTP method (GET, POST, PUT, DELETE, HEAD, OPTIONS, request) has at least one automated-profile denial test asserting `network_tcp` denial events and zero server hits.

5. **Centralized HTTP policy check**: `maybe_denied_response()` helper in `http.rs` consolidates the repeated `check_network_tcp` + `denied_response` pattern. All synchronous HTTP methods use the same denial helper.

6. **Path-specific architecture guards**: Checks 48b-48d verify HTTP method operation strings exist in `http.rs`, local denied tests assert strict zero hits, and no permissive language appears in automated HTTP denial tests.

7. **Manifest and documentation updates**: New fixtures registered in `manifest.toml` with `[local_service]` metadata. NSE_COMPATIBILITY.md, registry, AGENTS.override.md, SKILL.md, and AGENTS.md updated.

**Verification:** All HTTP methods (GET/POST/PUT/DELETE/HEAD/OPTIONS/request) have ManualPermissive success tests and automated-profile zero-hit denial tests. Architecture guards 48-50 plus 48b-48d pass.

### Milestone 6 Phase 03 (2026-07-07, CiSafe symmetry, send-path guards, method/path assertions)

Phase 03 completes the HTTP enforcement polish by closing test symmetry gaps, adding runtime structural guards, and strengthening success test assertions.

#### Changes

1. **CiSafe HTTP denial symmetry**: Added 5 new CiSafe denial tests (PUT, DELETE, HEAD, OPTIONS, request) to match the existing AgentSafe coverage. All 7 HTTP methods now have both AgentSafe and CiSafe denial tests. Total HTTP tests: 21 (7 ManualPermissive + 7 AgentSafe + 7 CiSafe).

2. **Send-path architecture guards**: Check 51 verifies every `.send()` call in `http.rs` is preceded by a capability check within 15 lines. Check 51b verifies all 3 async HTTP functions (`async_get`, `async_post`, `async_request`) use `check_network_tcp` directly. Guard script now has 52 checks (1-50, 51, 51b).

3. **Method/path assertions in success tests**: All 7 ManualPermissive HTTP success tests now assert `server.last_method()` and `server.last_path()` in addition to hit count and status. This proves requests reach the correct HTTP method handler on the test server.

4. **Async HTTP documentation**: Async HTTP functions are explicitly documented as synchronous Lua-callable functions using `block_on()`, not true async. Architecture guard Check 52 enforces this naming invariant.

5. **Documentation consistency**: NSE_COMPATIBILITY.md header updated from Milestone 4 to Milestone 6. SKILL.md stale "Known limitation: HTTP library (reqwest) bypasses NseCapabilityContext" removed (resolved in Phase 04/06). Deferred items updated to remove reqwest bypass.

**Verification:** 511 NSE tests pass (1 ignored), 52 architecture guards pass. All HTTP methods covered by all 3 test profiles.

### Milestone 6 Candidates

- Protocol library wrappers (smb, ssh, ftp, mysql, postgres, redis, mongodb, ldap, snmp)
- `stdnse.sleep()` cancellation integration
- Structured Lua output table parsing
- TUI-first compatibility debugging workflow
- Performance/caching for large corpus runs
