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

- **Milestone 2 (closed)**: Library registry, rule semantics, compatibility truthfulness, structured run reports, rule evaluation reports, and per-run library-usage reporting. See [Milestone 2 Closure Note](#milestone-2-closure-note).
- **Milestone 3 (next)**: Rust-side blocking helper cancellation via capability wrappers.

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

## Next Work: Milestone 3

The Milestone 2 contract above is the boundary. Future work should:

- Treat the library registry, rule semantics report, structured reports, and compatibility corpus as closed unless regression tests reveal a defect.
- Build on `NseRunReport` and `NseRuleEvaluationReport` rather than bypass them.
- Focus Milestone 3 on Rust-side blocking-helper cancellation via capability wrappers.
- Expand corpus breadth and library behavior upgrades as separate scoped work.

The Milestone 3 plan should be written from this closure index without reopening the Milestone 2 truthfulness contracts established here.

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

`NseCapabilityEvent` integration into `NseRunReport.capability_events` — denied operations affect compatibility status (`Partial`). Pilot wrappers in `wrappers.rs` demonstrate the pattern. `ExecutorCore` stores the capability context, constructed from `with_policy()` defaults or `with_profile()` overrides. Architecture guards detect direct high-risk ops in NSE libraries (informational, will tighten as wrappers migrate).

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

A representative corpus of NSE script fixtures verifies supported, partial, approximate, unsupported, denied, and errored behavior. The corpus makes compatibility claims testable and prevents overclaiming Nmap parity.

### Location

- **Fixtures**: `crates/eggsec-nse/tests/fixtures/nse_corpus/` — minimal `.nse` and `.lua` files exercising distinct compatibility paths
- **Tests**: `crates/eggsec-nse/tests/compatibility_corpus_tests.rs` — 18 integration tests gated on `#[cfg(feature = "nse")]`

### Coverage Categories

| Category | Tests | Status | Fidelity |
|----------|-------|--------|----------|
| Supported (simple script) | simple_portrule | Compatible | Full |
| Supported (stdnse output) | stdnse_output | Compatible | Full |
| Supported (builtin module require) | builtin_module_require | Compatible | Full |
| Supported (hostrule) | simple_hostrule | Compatible | Full |
| Supported (builtin script) | builtin_script | Compatible | Full |
| Supported (inline script) | inline_script | Compatible | Full |
| Supported (module resolution) | module_resolution | Compatible | Full |
| Supported (library use report) | library_use_report | Compatible | Full |
| Supported (exact rule) | exact_rule | Compatible | Full |
| Supported (mixed diagnostics) | mixed_diagnostics | Compatible | Full |
| Supported (module not found) | module_not_found | Compatible | Full |
| Denied (agent policy) | agent_denied | Failed | Full |
| Error (file not found) | file_not_found | Failed | Full |
| Error (unsupported behavior) | unsupported_behavior | Failed | Full |
| Error (rule error) | rule_error | Failed | Full |
| Unsupported (invalid module) | invalid_module_name | Partial | Minimal |
| Approximate (approx rule) | approximate_rule | CompatibleWithWarnings | Approximate |
| Serialization | serialization_roundtrip | — | — |

### Running the Corpus

```bash
cargo test -p eggsec-nse --features nse compatibility_corpus
```

### Adding New Cases

1. Add fixture `.nse` or `.lua` to `tests/fixtures/nse_corpus/`
2. Add a `#[test]` function in `tests/compatibility_corpus_tests.rs` following the existing `make_profile()` + resolver + report pattern
3. Assert specific fields: `status`, `fidelity`, `resolved_count`, `blocked_count`, `unsupported_features`, `approximations`
4. Run the corpus test to verify
