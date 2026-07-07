---
name: eggsec-nse
description: "NSE (Nmap Scripting Engine) support for Eggsec - Lua VM, libraries, sandbox, CVE integration"
triggers:
  - nse
  - lua
  - mlua
  - nmap scripting engine
  - nse libraries
metadata:
  category: security
  tools: [nse, scanner, recon]
  scope: eggsec-nse
---

## Overview

The `eggsec-nse` crate (`crates/eggsec-nse/`) provides Nmap Scripting Engine support via a Lua 5.4 interpreter using `mlua`. It allows running standard NSE scripts within Eggsec.

> **Milestone 1 (loader/profile) is closed.** Canonical implementation, tests, policy contract, and deferred work are listed in the [Milestone 1 Closure Index](../../architecture/nse_integration.md#milestone-1-closure-index). Future work should treat that index as the authoritative pointer and not reopen loader/profile policy unless a regression is found.

> **Milestone 3 (capability wrappers) Phase 01 complete.** A complete capability inventory and risk classification exists at `architecture/nse_capability_inventory.md`. The inventory classifies all side-effecting NSE helper operations by capability class, blocking risk, profile policy, accounting needs, cancellation requirements, and report events. Key findings: 4 libraries sandboxed (socket, io, os, lfs), all protocol libraries (~100+) bypass sandbox, `nmap.socket_*()` bypasses socket sandbox, `stdnse.sleep()` blocks without cancellation checks. Migration priority: process execution → filesystem write → filesystem read → network TCP/UDP → DNS → compression → crypto/TLS → time/randomness → pure CPU.

> **Milestone 3 Phase 02 complete.** `NseCapabilityContext` and decision engine (`capabilities.rs`) provide centralized policy enforcement. `NseCapabilityKind` covers 11 operation classes. Profile-specific checks: ManualPermissive allows all with warnings, ManualStrict enforces path/network policy, AgentSafe denies process exec + FS write, CiSafe denies all side effects. `NseCapabilityEvent` integration into `NseRunReport.capability_events`. Pilot wrappers in `wrappers.rs` demonstrate the pattern. `ExecutorCore` stores the capability context. Architecture guards detect direct high-risk ops in NSE libraries (informational). **New side-effecting helpers must route through `NseCapabilityContext` via wrapper functions in `wrappers.rs`.** Direct use of high-risk ops (process exec, filesystem write, network TCP/UDP) in library code is detectable by Check 33 (informational) and will be tightened in future phases.

> **Milestone 3 Phase 03 complete.** Filesystem and process wrappers are now fully migrated through `NseCapabilityContext`. Libraries `io.rs`, `lfs.rs`, `os.rs`, and `nmap.rs` route all side-effecting operations through capability checks. Executing wrappers (`nse_fs_read_to_string`, `nse_fs_write`, `nse_fs_remove_file`, `nse_fs_create_dir`, `nse_fs_rename`, `nse_process_exec`, etc.) combine capability checking with the actual operation, handling cancellation, resource counters, and event recording. `AgentSafe` and `CiSafe` deny process execution and filesystem writes by default. `ManualPermissive` allows with warnings. Architecture guard Check 33 now fails for direct `std::process::Command` in NSE libraries (outside wrappers.rs/executor_core.rs/tests). Library registration functions now take `&NseCapabilityContext` parameter. Network TCP/UDP, compression, and crypto remain pending for future phases.

> **Milestone 3 Phase 04 complete.** Network TCP/UDP and DNS wrappers migrated through `NseCapabilityContext`. Executing wrappers added: `nse_network_tcp_connect`, `nse_network_tcp_send`, `nse_network_tcp_receive`, `nse_network_udp_send`, `nse_network_udp_receive`, `nse_dns_lookup`, plus check-only `check_network_udp`. Libraries `socket.rs`, `comm.rs`, and `dns.rs` now accept `&NseCapabilityContext` in their registration functions and route network/DNS operations through capability wrappers before performing the actual operations. Architecture guard Check 33c (informational) detects direct network calls in unmigrated libraries. All 318 tests pass. Compression, crypto/TLS, and protocol-specific libraries (smb, ssh, ftp, http, etc.) remain unmigrated.

> **Milestone 3 Phase 05 complete.** Time, randomness, environment, crypto, and compression helpers are now routed through `NseCapabilityContext`. Executing wrappers added: `nse_time_now`, `nse_random_bytes`, `nse_env_var`, `nse_compress`, `nse_decompress`. Check-only wrappers added: `check_randomness`, `check_environment`, `check_crypto`, `check_compression`. Profile-specific policies: AgentSafe denies environment access, warns on randomness; CiSafe denies environment and randomness, warns on time nondeterminism. Compression enforces 64 MiB input and 256 MiB output limits. Libraries migrated: `datetime.rs`, `rand.rs`, `openssl.rs`, `tls.rs`, `sslcert.rs`, `zlib.rs` now accept `&NseCapabilityContext`. All 200+ tests pass.

> **NSE Milestone 3 (capability wrappers) is closed.** All side-effecting helper classes (filesystem, process, network, DNS, time, randomness, environment, compression, crypto) are routed through `NseCapabilityContext`. Protocol-specific libraries beyond network I/O remain deferred. Capability events are visible in `NseRunReport.capability_events`. Architecture guards prevent new direct bypasses. See the [Milestone 3 Closure Note](../../architecture/nse_integration.md#milestone-3-closure-note) for the canonical summary.
> **Milestone 3 Closure Verification (2026-07-06).** Final verification: 369 tests, 37 architecture guard checks, fmt/clippy clean. New end-to-end profile/report tests in `tests/profile_report_tests.rs`. See [Milestone 3 Final Verification](../../architecture/nse_integration.md#milestone-3-final-verification).

> **NSE Milestone 4 Phase 01 (corpus expansion) complete.** Compatibility corpus expanded from 18 individual tests into 21 fixtures organized by 8 categories with a data-driven harness (`manifest.toml`). Harness tests assert semantic report fields: status, fidelity, resolution, libraries, rules, capability events. 389 tests pass (1 ignored), architecture guards pass. See [Milestone 4 Phase 01](../../architecture/nse_integration.md#compatibility-corpus).

> **NSE Milestone 4 Phase 03 (context fidelity) complete.** Structured context types (`NseHostContext`, `NsePortContext`, `NseServiceContext`, `NseContextSource`) provide typed Lua table construction with provenance tracking. `hostrule(host)` receives structured host table (not raw nmap global). `portrule(host, port)` receives (host, port) pair matching Nmap signature. `NseRuleEvaluationReport` extended with context fidelity fields. 402 tests pass, 8 new context fidelity tests. See [Milestone 4 Phase 03](../../architecture/nse_integration.md#milestone-4-phase-03-host-port-and-service-context-fidelity).

> **Milestone 4 complete.** Structured evidence reports (`NseEvidenceKind`, `NseEvidenceItem`, `extract_evidence()`), bridge to `ReportEnvelope` (`bridge.rs`), CLI human-readable output (`print_human_report()`), upstream-style corpus (16 fixtures), context fidelity fixtures (3), compatibility matrix (`docs/NSE_COMPATIBILITY.md`), 43 library registry entries, 40 corpus fixtures total.

> **Milestone 4 closure pass (2026-07-06, runtime harness).** The compatibility corpus is now verified by two structurally separated harnesses. The static harness (`compatibility_corpus_tests.rs` `mod corpus_manifest`) is resolver-only; the runtime harness (`runtime_corpus_tests.rs`) drives every fixture through `NseExecutor::with_profile()` with synthetic host/port context and asserts manifest expectations against observed rule/library/capability reports. Smoke tests (`runtime_smoke_tests.rs`) exercise the full pipeline (profile → context → execution → report → `ReportEnvelope` bridge). Architecture guards 42/43/44 enforce the separation. 432 tests pass (1 ignored), 44 architecture guard checks pass. See [Milestone 4 Closure Verification](../../architecture/nse_integration.md#milestone-4-closure-verification).
>
> **Milestone 5 Phase 01 (2026-07-06, flake isolation).** The `runtime_corpus_tests` parallel-execution flake has been fixed. Root cause: `run_fixture_runtime()` used PID-only temp dir naming, causing concurrent test functions to share file paths. Fix: global `AtomicU32` invocation counter ensures each call gets a unique temp dir (`{fixture}-{pid}-{invocation_id}`). Stable at default parallelism — no `--test-threads` workaround needed. See [Milestone 5 Phase 01](../../architecture/nse_integration.md#milestone-5-phase-01-runtime-corpus-flake-isolation-2026-07-06).

> **Milestone 5 Phase 03 (2026-07-06, local protocol fixtures).** Local TCP/HTTP/UDP fixture harness with real listeners, 5 new `.nse` scripts, 16 runtime tests in `local_protocol_tests.rs`. Manifest `local_service` metadata enables runtime harness skip. Architecture guard Check 47. Known limitation: HTTP library (reqwest) bypasses `NseCapabilityContext`. 452 NSE tests pass, 47 architecture guards pass.

## Key Components

| Component | File | Purpose |
|-----------|------|---------|
| `NseExecutor` | `src/executor.rs` | Sync Lua VM wrapper with NSE rule execution |
| `AsyncNseExecutor` | `src/async_executor.rs` | Async wrapper with tokio runtime |
| `ExecutorCore` | `src/executor_core.rs` | Shared Lua VM, globals, library registration |
| `ScriptResolver` | `src/resolver.rs` | Policy-enforcing script/module resolver with diagnostics |
| `SandboxConfig` | `src/lib.rs:50-76` | Sandbox restrictions for scripts |
| `ScanContext` | `src/context.rs:141-149` | Host info, ports, output during execution |
| `NseHostContext` | `src/context.rs` | Structured host data with provenance for rule evaluation |
| `NsePortContext` | `src/context.rs` | Structured port data with service context for rule evaluation |
| `NseServiceContext` | `src/context.rs` | Service metadata (name, product, version, tunnel, confidence) |
| `NseContextSource` | `src/context.rs` | Provenance enum: Scan, Fixture, Synthetic, Unknown |
| `NseExecutionLimits` | `src/limits.rs` | Bounded execution: wall-clock, instruction count, output size, script size, resource usage |
| `NseCancellationToken` | `src/limits.rs` | Cooperative cancellation via `Arc<AtomicBool>` |
| `NseResourceCounters` | `src/limits.rs` | Atomic counters for network/filesystem operations |
| `NseExecutionStats` | `src/limits.rs` | Execution stats snapshot (elapsed, instructions, bytes, violation) |
| `evaluate_rule()` | `src/report.rs` | Converts Lua rule results to structured `NseRuleEvaluationReport` |
| runtime `require()` tracking | `src/executor_core.rs` / `src/lib.rs` | Populates per-run library usage entries in `NseRunReport.libraries` |
| `build_failure_report()` | `src/lib.rs` | Builds full `NseRunReport` for error paths with library data |

## Rule Evaluation Reports

`evaluate_rule()` in `report.rs` converts Lua rule return values into structured `NseRuleEvaluationReport` instances. The CLI runtime path (`run_script_with_rules()`) calls `evaluate_rule()` for each rule result.

| Outcome | `evaluated` | `matched` | `exactness` | `unsupported` | Description |
|---------|-------------|-----------|-------------|---------------|-------------|
| Boolean true | true | true | `"exact"` | None | Rule matched |
| Boolean false | true | false | `"exact"` | None | Rule did not match |
| Nil | true | false | `"exact"` | None | Rule returned nil |
| Non-boolean | false | false | `"unsupported"` | Some | Return type not supported by NSE semantics |
| Lua error | false | false | `"exact"` | None | `error` field populated with error message |

Runtime `require()` tracking in `executor_core.rs`, surfaced through `run_cli_with_profile()` and `NseExecutor::build_report()`, populates `NseRunReport.libraries` with per-run observed or attempted `require()` activity, including error paths. Each entry has a `loaded` field: `true` means the runtime observed a successful module load; `false` means a `require()` was attempted but the module failed, was blocked, was missing, had an invalid name, or was statically detected without runtime confirmation. Static `require()` detection is approximate and labeled with a warning. The field records per-run usage and diagnostics; it is not a capability snapshot. `build_failure_report()` produces a full `NseRunReport` for error paths with library data and error information.

The `unsupported` field on `NseRuleEvaluationReport` is `Option<String>` and is `#[serde(skip_serializing_if = "Option::is_none")]` — it only appears in serialized output when present (non-boolean return types).

## Execution Profiles

NSE execution profiles provide explicit presets for sandbox config, limits, script/module policy, network policy, and audit metadata.

### Available Profiles

| Profile | Use Case | Scripts | Network | Limits |
|---------|----------|---------|---------|--------|
| `ManualPermissive` | CLI (manual-only) | All builtin + files | AllowAllManual | 120s / 100M / 50MiB |
| `ManualStrict` | CLI restricted | Builtin only, restricted roots | AllowCidrs | 120s / 100M / 50MiB |
| `AgentSafe` | Autonomous agents | Builtin only | From target/scope | 15s / 5M / 2MiB |
| `CiSafe` | CI pipelines | Builtin only | DenyAll | 15s / 5M / 2MiB |
| `CompatibilityLab` | Nmap compat | All + Nmap paths | AllowAllManual | 120s / 100M / 50MiB |

### Key Types

```rust
use eggsec_nse::{
    NseExecutionProfileKind,    // enum: ManualPermissive, ManualStrict, AgentSafe, CiSafe, CompatibilityLab
    ResolvedNseExecutionProfile, // Resolved profile with all policies
    ScopeInput,                 // Target + scope CIDRs for network policy derivation
    NseScriptPolicy,            // Script access rules
    NseModulePolicy,            // Module access rules
    NseNetworkPolicy,           // Network access rules
    NseRunReport,               // Structured run output (Milestone 2; per-run library usage)
    NseRuleEvaluationReport,    // Rule evaluation metadata (Milestone 2)
    NseLibraryDescriptor,       // Library registry descriptor (Milestone 2)
    NseHostContext,             // Structured host data with provenance (Milestone 4 Phase 03)
    NsePortContext,             // Structured port data with service context (Milestone 4 Phase 03)
    NseServiceContext,          // Service metadata (Milestone 4 Phase 03)
    NseContextSource,           // Context provenance enum (Milestone 4 Phase 03)
};
```

### Creating Profiles

```rust
// CLI default (full access)
let profile = ResolvedNseExecutionProfile::manual_permissive();

// Agent (restricted, network from scope)
let scope = ScopeInput::new("192.168.1.1").with_scope_cidrs(&["192.168.1.0/24"]);
let profile = ResolvedNseExecutionProfile::agent_safe(&scope);

// CI (zero network)
let profile = ResolvedNseExecutionProfile::ci_safe();
```

### Running with Profile

```rust
// Profile-aware execution — capability context matches the resolved profile
eggsec_nse::run_cli_with_profile(config, Some(profile)).await?;

// Fallback to manual_permissive if None
eggsec_nse::run_cli_with_profile(config, None).await?;
```

`run_cli_with_profile()` constructs the executor via `NseExecutor::with_profile(&resolved_profile)` so the capability context matches the resolved profile. `NseExecutor::with_profile()` is the preferred constructor for CLI and automated surfaces.

### CLI Handler Integration

The CLI handler (`handle_nse` in `crates/eggsec/src/commands/handlers/scan.rs`) constructs a `ManualPermissive` profile and passes it to `run_cli_with_profile`. Profile warnings (sandbox disabled, scope implications) are logged at startup.

## Features

```
nse = ["mlua", "mlua-luau-scheduler", "openssl", "des"]
nse-ssh2 = ["nse", "dep:ssh2"]
nse-sandbox = []  # Enables SandboxConfig enforcement
```

## Libraries (166 implementations, 43 registry descriptors)

Located in `src/libraries/`:
- **socket.rs** (703 lines) - TCP/UDP/SCTP sockets with sandbox enforcement
- **io.rs** (391 lines) - File I/O with path sandboxing
- **lfs.rs** (379 lines) - LuaFileSystem with path restrictions
- **os.rs** (316 lines) - OS operations (getenv, setenv, date, exit, etc.)
- **http.rs** (803 lines) - HTTP client (blocking + async)
- **vulns.rs** (571 lines) - CVE database with major vulnerabilities
- **dns.rs**, **ssl.rs**, **ssh.rs**, **mysql.rs**, **redis.rs**, **mongodb.rs**, **ldap.rs**, **snmp.rs**, **smb.rs**, etc.

### Library Registration

Libraries are registered via `register_*_library()` functions. All side-effecting libraries (`io`, `lfs`, `os`, `nmap`, `socket`, `comm`, `dns`) accept `&NseCapabilityContext` for capability-gated operations. See `executor_core.rs:272-450` for the full list of modules registered as NSE globals.

## Sandbox Enforcement

| Library | Sandbox Enforcement |
|---------|---------------------|
| `io` | `is_path_allowed()` validates paths; `check_fs_read()`/`check_fs_write()`/`check_process_exec()` via capability context |
| `lfs` | Path checks + capability context `check_fs_read()`/`check_fs_write()` |
| `os` | `getenv/setenv` blocked; `check_fs_write()` via capability context for file ops |
| `socket` | `is_host_allowed()` validates hosts; `nse_network_tcp_connect`/`nse_network_udp_send`/`nse_dns_lookup` via capability context |
| `comm` | `nse_network_tcp_connect`/`nse_network_tcp_send`/`nse_network_tcp_receive` via capability context |
| `dns` | `nse_dns_lookup` via capability context |

### SandboxConfig

```rust
pub struct SandboxConfig {
    pub enabled: bool,                    // Controlled by `sandbox` feature
    pub allowed_dir: Option<PathBuf>,     // Default: /tmp/eggsec-nse
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
| `rustc-hash` not in eggsec-nse dependencies | Added `rustc-hash.workspace = true` to Cargo.toml |
| `CveCache` missing closing bracket | Fixed typo in struct definition |
| `CveCache` using async `.await` on parking_lot RwLock | Removed `.await`; parking_lot RwLock is sync |
| `public_api/api.rs` 8 std::HashMap instances | Replaced with FxHashMap for performance |
| `libraries/http.rs:143` HashMap in parse_options | Changed to FxHashMap |
| `libraries/datafiles.rs:31-33` HashMap in get_services | Changed to FxHashMap |
| `libraries/creds.rs:102,123` HashSet usage | Changed to FxHashSet |

## Execution Limits

`NseExecutionLimits` bounds script execution across multiple dimensions:

| Limit | Default | Automated | Purpose |
|-------|---------|-----------|---------|
| `wall_clock_timeout` | 30s | 15s | Max wall-clock time |
| `lua_instruction_budget` | 10M | 5M | Max Lua instructions (interrupt hook) |
| `max_output_bytes` | 10 MiB | 2 MiB | Max total output |
| `max_script_bytes` | 5 MiB | 1 MiB | Max script source size |
| `max_required_module_bytes` | 2 MiB | 512 KiB | Max required module size |
| `max_network_operations` | None | 100 | Max network ops (socket) |
| `max_filesystem_operations` | None | 50 | Max FS ops (io/lfs) |

### Profiles

```rust
NseExecutionLimits::manual_defaults()    // CLI/interactive: 120s timeout, 100M instructions
NseExecutionLimits::automated_defaults() // MCP/agent/REST: 15s timeout, 5M instructions
NseExecutionLimits::unlimited()          // No limits (use with caution)
```

## Script/Module Resolver

`ScriptResolver` in `src/resolver.rs` enforces hardened script and module loading:

| Component | Purpose |
|-----------|---------|
| `NseScriptSource` | Explicit script source kind (Builtin, TrustedRegistry, File, InlineManual) |
| `NseModuleName` | Validated module name (ASCII alphanumeric + `_`, `-`, `.`) |
| `ScriptResolver` | Policy-enforcing resolver with diagnostics |
| `NseLoadError` | Structured load error (NotFound, BlockedByPolicy, OutsideRoot, SymlinkEscape, InvalidExtension, Oversized, InvalidModuleName, IoError, EvalError) |
| `NseLoadDiagnostic` | Load behavior diagnostics for visibility |

### Module Name Grammar

Validated before any filesystem access:
- ASCII letters, digits, `_`, `-`, `.`
- Must not start with `.`
- Must not contain `..`
- Must not contain `/`, `\`, `:`, `~`, null bytes, glob chars, whitespace
- Max length: 256

### Path Containment

- Canonical paths validated under approved roots
- Symlink escape rejected
- File extension allowlist: `.lua`, `.nse`

### Usage

```rust
use eggsec_nse::{ScriptResolver, NseScriptSource, validate_nse_module_name};

let mut resolver = ScriptResolver::new(
    profile.script_policy,
    profile.module_policy,
    profile.limits,
);

// Resolve a script file
let script = resolver.resolve_script(NseScriptSource::File {
    path: PathBuf::from("/tmp/test.lua"),
})?;

// Validate a module name
let name = validate_nse_module_name("stdnse")?;

// Resolve a filesystem module
let module = resolver.resolve_module("stdnse")?;
```

### Cancellation

```rust
let cancellation = NseCancellationToken::new();
cancellation.cancel();  // Request cancellation
cancellation.is_cancelled();  // Check
```

### Creating with Limits

```rust
use eggsec_nse::{NseExecutor, NseExecutionLimits, NseCancellationToken};
use eggsec_nse::{default_script_policy, default_module_policy};

let limits = NseExecutionLimits::automated_defaults();
let cancellation = NseCancellationToken::new();
let executor = NseExecutor::with_policy(
    SandboxConfig::default(),
    limits,
    cancellation,
    default_script_policy(),
    default_module_policy(),
)?;
let result = executor.run_script_with_limits(script)?;
let stats = executor.execution_stats();
```

> **Manual-only constructors**: `NseExecutor::new()`, `with_sandbox()`, and `with_target()` use permissive defaults. Automated surfaces must use `with_profile()` or `with_full_policy(...)`.
>
> **Profile-aware constructors**:
> - `NseExecutor::with_profile(profile)` — preferred for CLI and automated surfaces; capability context derives from the resolved profile.
> - `NseExecutor::with_full_policy(profile_kind, sandbox, limits, cancellation, script_policy, module_policy, network_policy)` — explicit control over every policy parameter.
> - `AsyncNseExecutor::with_full_policy(...)` — async counterpart.
> - `ExecutorCore::with_full_policy(...)` — core-level explicit control.
>
> **AgentSafe filesystem reads**: Scoped-only. A filesystem read is allowed only when the path is under the sandbox `allowed_dir` (with sandbox enabled) or an explicit root entry. Unscoped reads are denied.

### Resolver-Owned Module Loading

Lua `require()` filesystem loading delegates to `ScriptResolver::resolve_module()`. The resolver enforces module name grammar, profile policy, canonical root containment, symlink escape rejection, extension allowlist, and size limits. All script/module loading flows through `ScriptResolver` — no direct `std::fs::read_to_string()` in execution paths.

### Empty-Roots Semantics

The meaning of empty `allowed_script_roots` / `allowed_module_roots` depends on the `allow_*` boolean and the profile that produced the policy. Doc comments on `NseScriptPolicy` / `NseModulePolicy` enumerate the full table; the short form:

| Profile kind | Script files | Filesystem modules |
|--------------|--------------|--------------------|
| `ManualPermissive` | Empty roots = unrestricted manual file selection. Extension + size limits still apply. | Empty roots = no filesystem modules (built-ins only). |
| `ManualStrict` / `CompatibilityLab` | Empty roots = misconfiguration. Files outside any configured root are rejected. | Empty roots = misconfiguration. Modules outside configured roots are rejected. |
| `AgentSafe` / `CiSafe` | `allow_script_files = false` — denied before any root check. | `allow_filesystem_modules = false` — denied before any root check. |

`ManualPermissive` emits a `manual-permissive profile is not agent-safe` warning so automated callers cannot accidentally use it.

### Read vs Write Authorization

`resolver.rs` exposes two distinct root-containment helpers:

- `validate_existing_path_under_roots(path, roots)` — **read-only** helper. Requires the canonical file path to resolve. Returns `IoError` for non-existent files. Used by `resolve_script_file()` and `resolve_module()`.
- `validate_parent_under_roots(path, roots)` — reserved for future create/write semantics. Currently `#[allow(dead_code)]` and intentionally not used by read paths.

Read paths must never authorize non-existent script/module files via parent fallback.

## Common Patterns

### Creating an Executor

```rust
use eggsec_nse::{NseExecutor, SandboxConfig};

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

## Compatibility Corpus

A representative corpus of NSE script fixtures verifies supported, partial, approximate, unsupported, denied, and errored behavior. The corpus is representative and local-only by default — it does not cover all Nmap scripts. The corpus makes compatibility claims testable and prevents overclaiming Nmap parity.

- **Fixtures**: `tests/fixtures/nse_corpus/` — 44 `.nse` and `.lua` files organized by category (discovery, version, default, protocol, auth, partial, unsupported, regression, upstream)
- **Manifest**: `tests/fixtures/nse_corpus/manifest.toml` — data-driven fixture registry with expected status, fidelity, libraries, rules, capability events, provenance, and gap classification
- **Tests**: `tests/compatibility_corpus_tests.rs` — 18 legacy individual tests + 25 data-driven harness tests

```bash
# Data-driven harness (all fixtures from manifest)
cargo test -p eggsec-nse --features nse --test compatibility_corpus_tests -- corpus_harness

# Legacy individual tests
cargo test -p eggsec-nse --features nse compatibility_corpus

# Local protocol fixtures (TCP/HTTP/UDP with real listeners)
cargo test -p eggsec-nse --features nse --test local_protocol_tests
```

Harness tests assert semantic report fields: `status`, `fidelity`, resolved/blocked state, `libraries`, `rules`, `capability_events`, provenance metadata, and gap classification. Adding a new fixture requires only a `.nse`/`.lua` file and a `manifest.toml` entry with provenance and gap classification.

## Testing

```bash
cargo test -p eggsec-nse
cargo check --lib -p eggsec-nse --features nse
cargo test -p eggsec-nse --features nse --test script_file_policy_tests
cargo test -p eggsec-nse --features nse --test profile_guard_tests
cargo test -p eggsec-nse --features nse --test profile_tests
cargo test -p eggsec-nse --features nse --test execution_limits_tests
cargo test -p eggsec-nse --features nse --test sandbox_tests
cargo test -p eggsec-nse --features nse --test compatibility_corpus_tests
cargo test -p eggsec-nse --features nse --test rule_evaluation_tests
cargo test -p eggsec-nse --features nse --test profile_propagation_tests
cargo test -p eggsec-nse --features nse --test profile_report_tests
```
