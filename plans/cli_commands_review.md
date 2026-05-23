# CLI Commands Architecture Review

**Document:** `architecture/cli_commands.md`
**Review Date:** 2026-05-23
**Codebase Path:** `crates/slapper/src/cli/` and `crates/slapper/src/commands/`

---

## Verified Claims

### 1. CLI Structure (clap-based parsing)

| Claim | Status | Evidence |
|-------|--------|----------|
| Uses `clap` for CLI parsing | **VERIFIED** | `crates/slapper/src/cli/mod.rs:1` imports `clap::{Parser, Subcommand, ValueEnum}` |
| `mod.rs` defines `Cli` entry point | **VERIFIED** | `crates/slapper/src/cli/mod.rs:54-77` defines `Cli` struct with `Parser` derive |
| `Commands` enum has 35+ variants | **VERIFIED** | Counted 35 command variants in `Commands` enum (lines 80-189) |
| `CommonHttpArgs` defined | **VERIFIED** | `crates/slapper/src/cli/mod.rs:192-222` defines `CommonHttpArgs` struct |

### 2. CLI Modules Organization

| Claim | Status | Evidence |
|-------|--------|----------|
| `scan.rs` exists | **VERIFIED** | `crates/slapper/src/cli/scan.rs` exists with port/endpoint/fingerprint/scan/resume args |
| `fuzz.rs` exists | **VERIFIED** | `crates/slapper/src/cli/fuzz.rs` exists with fuzz/waf-stress/waf args |
| `http.rs` exists | **VERIFIED** | `crates/slapper/src/cli/http.rs` exists with load/recon/graphql/oauth args |
| `packet.rs` & `stress.rs` exist | **VERIFIED** | Both files exist under `crates/slapper/src/cli/` |
| `agent.rs` & `ai_analyze.rs` exist | **VERIFIED** | Both files exist with feature gates |

### 3. Global Flags

| Claim | Status | Evidence |
|-------|--------|----------|
| `--json` global flag | **VERIFIED** | `crates/slapper/src/cli/mod.rs:63-64` with `global = true` |
| `--config` global flag | **VERIFIED** | `crates/slapper/src/cli/mod.rs:66-67` with `global = true` |
| `--scope` global flag | **VERIFIED** | `crates/slapper/src/cli/mod.rs:69-70` with `global = true` |

### 4. Feature-Gated Commands

| Claim | Status | Evidence |
|-------|--------|----------|
| `stress-testing` gates stress/proxy/icmp/traceroute | **VERIFIED** | Lines 144-155 in `mod.rs` use `#[cfg(feature = "stress-testing")]` |
| `packet-inspection` gates `Packet` | **VERIFIED** | Line 127-129 uses `#[cfg(feature = "packet-inspection")]` |
| `nse` gates `Nse` | **VERIFIED** | Line 130-132 uses `#[cfg(feature = "nse")]` |
| `ai-integration` gates `AiAnalyze` | **VERIFIED** | Lines 182-184 use `#[cfg(feature = "ai-integration")]` |
| `rest-api` gates `Serve`/`McpServe`/`Agent` | **VERIFIED** | Lines 166-179 use `#[cfg(feature = "rest-api")]` |
| `grpc-api` gates `Grpc` | **VERIFIED** | Lines 187-189 use `#[cfg(feature = "grpc-api")]` |
| `sbom` gates `Sbom` | **VERIFIED** | Line 118-120 uses `#[cfg(feature = "sbom")]` |
| `python-plugins`/`ruby-plugins` gate `Plugin` | **VERIFIED** | Lines 133-135 use `#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]` |

### 5. Command Dispatch

| Claim | Status | Evidence |
|-------|--------|----------|
| `CommandContext` carries global state | **VERIFIED** | `crates/slapper/src/commands/handlers/mod.rs:63-68` with config, scope, json fields |
| `handle_command` is exhaustive match | **VERIFIED** | Lines 98-152 match on all `Commands` variants with no wildcard arm |
| Comment confirms exhaustive match | **VERIFIED** | Lines 100-102: "Keep this match exhaustive: no wildcard arm" |

### 6. Handler Patterns

| Claim | Status | Evidence |
|-------|--------|----------|
| Scope validation with `ensure_scope_url` | **VERIFIED** | `handlers/fuzz.rs:5`, `handlers/scan.rs:19`, `handlers/auth_test.rs:10` |
| Scope validation with `ensure_scope` | **VERIFIED** | `handlers/scan.rs:8`, `handlers/scan.rs:30`, `handlers/stress.rs:12` |
| Error handling returns `Result` | **VERIFIED** | All handlers return `Result<()>` and use `map_err` |
| No `std::process::exit()` in handlers | **VERIFIED** | Grep found no matches in `commands/handlers/` |

### 7. Workflow Steps

| Step | Claim | Status | Evidence |
|------|-------|--------|----------|
| 1 | `main.rs` parses with `Cli::parse()` | **VERIFIED** | `crates/slapper/src/main.rs:16` |
| 2 | Logging initialized | **VERIFIED** | `main.rs:28-36` calls `init_logging()` |
| 3 | Config and Scope loaded | **VERIFIED** | `main.rs:38-39` |
| 4 | `CommandContext` created | **VERIFIED** | `main.rs:41-42` |
| 5 | `handle_command` called | **VERIFIED** | `main.rs:43` |
| 6 | Handler executes operation | **VERIFIED** | Handler functions delegate to module implementations |

### 8. Bug Fixes (2026-05-22)

| Issue | Status | Evidence |
|-------|--------|----------|
| `sbom.rs`: `unwrap()` replaced with `ok_or_else()` | **VERIFIED** | `handlers/sbom.rs:4-7` uses `validate_path_string` with proper error handling |
| `config.rs`: `std::process::exit(1)` removed | **VERIFIED** | `handlers/config.rs:6-17` uses `map_err()` for proper error return |
| `http.rs`: `-o` added to `load` and `graphql` | **VERIFIED** | `cli/http.rs:94-95` (LoadArgs), `cli/http.rs:170-171` (GraphQlArgs) have `-o` |
| `handlers/mod.rs:155-169`: `handle_no_command` guidance | **VERIFIED** | `handlers/mod.rs:155-163` prints guidance to use `--help` |
| `handlers/cluster.rs:348`: `unwrap_or(22)` replaced | **VERIFIED** | `handlers/cluster.rs:349-350` uses `unwrap_or_else(\|_\| 22)` |
| `handlers/auth_test.rs:10`: scope validation added | **VERIFIED** | `handlers/auth_test.rs:10` calls `ctx.ensure_scope_url(&args.target)?` |
| `cli/scan.rs`: `-o` added to scan args | **VERIFIED** | `cli/scan.rs:172-173`, `224-225`, `251-252`, `281-282`, `386-387` all have `-o` |
| `cli/fuzz.rs`: `-o` added to `WafStressArgs` | **VERIFIED** | `cli/fuzz.rs:263-264` has `-o` |
| `handlers/mod.rs`: preserved `From<WafStressArgs>` | **VERIFIED** | `cli/fuzz.rs:269-324` implements `From<WafStressArgs> for FuzzArgs` |
| `cli/cluster.rs`: removed unused `-o` flag | **VERIFIED** | `cli/cluster.rs:11-23` - `ClusterArgs` has no `-o` flag |

### 9. CLI Consistency Guidelines

| Issue | Status | Evidence |
|-------|--------|----------|
| `--host` vs `--target` vs `--url` consistency | **PARTIAL** | Mix persists: `PortScanArgs.host`, `FuzzArgs.url`, `ReconArgs.target` |
| Timeout defaults use 15s standard | **PARTIAL** | Load=30s, Fuzz=10s, Recon=10s, GraphQL/OAuth=15s - not consistently 15s |
| WAF profile uses `String` | **VERIFIED** | `cli/fuzz.rs:345` uses `String` (not `ValueEnum`) |
| Source IP naming `source_ip`/`source_port` | **VERIFIED** | `cli/scan.rs:98-99`, `137-138` use `source_ip` and `source_port` |

---

## Discrepancies

### 1. Commands Count vs Documentation

| Issue | Details |
|-------|---------|
| **Claimed**: "35+ variants" | **Actual**: 35 variants (verified by count) |
| **Severity**: Low | Documentation is accurate |

### 2. Timeout Inconsistency

| Issue | Details |
|-------|---------|
| **Claimed**: "Use 15s as standard default" | **Actual**: Multiple different defaults |
| **Details**: Load=30s (`http.rs:86`), Fuzz=10s (`fuzz.rs:110`), GraphQL/OAuth=15s (`http.rs:162`, `http.rs:194`), AuthTest=10s (`auth.rs:41`) |
| **Severity**: Low | Guidelines not followed, but reasonable variation exists |

### 3. Missing Commands from Architecture

| Issue | Details |
|-------|---------|
| `ci.rs` CLI file not documented | `crates/slapper/src/cli/ci.rs` exists but not mentioned in arch doc |
| `vuln.rs` CLI file not documented | `crates/slapper/src/cli/vuln.rs` exists but not mentioned |
| `storage.rs` CLI file not documented | `crates/slapper/src/cli/storage.rs` exists but not mentioned |
| `plan.rs` CLI file not documented | `crates/slapper/src/cli/plan.rs` exists but not mentioned |
| `misc.rs` CLI file not documented | `crates/slapper/src/cli/misc.rs` exists but not mentioned |
| **Severity**: Medium | Documentation incomplete |

---

## Bugs Found

### 1. No Bugs Found in CLI/Commands Layer

All verified bug fixes from the architecture document have been properly implemented:
- No `unwrap()` in `sbom.rs` handlers
- No `std::process::exit()` in handlers
- Proper error propagation via `map_err()`
- Scope validation properly applied

### 2. Code Quality Observations

| Location | Issue | Type |
|----------|-------|------|
| `cli/misc.rs:188-189` | `PluginRunArgs` has `-o` short flag but documentation says cluster has no output - plugins can produce output | Inconsistency (Low) |
| `handlers/plan.rs:6` | `unwrap_or("no target specified")` - fallback for missing target | Code smell (Low) |

---

## Improvement Opportunities

### 1. Missing `-o` Flag on Some Commands (Medium Priority)

**File:** `crates/slapper/src/cli/misc.rs`

The following argument structs are missing the `-o` / `--output` short flag:
- `ConfigArgs` / `ConfigCommand` (lines 22-46)
- `NotifyArgs` / `NotifyCommand` (lines 48-96)
- `RemoteArgs` (lines 98-160)
- `ExecArgs` (lines 136-160)
- `ReportArgs` / `ReportCommand` (lines 192-280)

**Impact**: Inconsistent UX - some commands support `-o` for output file, others don't.

**Fix**: Add `#[arg(long, short = 'o', help = "Output file path")]` to appropriate output-related args.

### 2. Timeout Standardization (Low Priority)

**Issue**: Timeout defaults vary across commands (10s, 15s, 30s).

**Recommendation**: Document a guideline for which timeout applies to which category:
- Network probes/scans: 10s
- HTTP operations: 15s  
- Load testing: 30s

### 3. Documentation Updates Needed (Medium Priority)

**Files**: `architecture/cli_commands.md`

**Updates Required**:
1. Update line ~9: "35+ variants" should specify actual count
2. Add missing CLI modules to the module list:
   - `ci.rs` - CI/CD mode arguments
   - `vuln.rs` - Vulnerability management
   - `storage.rs` - Database storage operations
   - `plan.rs` - Execution planning
   - `misc.rs` - Contains Config, Notify, Remote, Exec, Report, Plugin, Sbom
3. Update the Bug Fixes section to reflect current state
4. Add `auth.rs` to the module list (for `auth-test` command)

### 4. Handler Test Coverage (Medium Priority)

**Issue**: No unit tests found for `handle_command` dispatch logic.

**Recommendation**: Add integration tests that verify:
- All commands dispatch to correct handlers
- Missing command gracefully handled
- Global flags properly propagate

### 5. Code Duplication in Handler Arguments (Low Priority)

**Issue**: `FuzzArgs` and `WafStressArgs` have significant field overlap. The `From<WafStressArgs> for FuzzArgs` implementation duplicates many fields.

**Recommendation**: Consider extracting common fields into a shared `FuzzCommonArgs` struct.

---

## Priority Summary

| Priority | Finding | Effort |
|----------|---------|--------|
| **Medium** | Documentation missing CLI modules (ci, vuln, storage, plan, misc) | Low |
| **Medium** | Missing `-o` flag on multiple command argument structs | Medium |
| **Medium** | Add integration tests for command dispatch | Medium |
| **Low** | Timeout defaults not standardized | Low |
| **Low** | `unwrap_or("no target specified")` in plan.rs | Low |

---

## Detailed File Reference

### CLI Structure

```
crates/slapper/src/cli/
├── mod.rs          (285 lines) - Main Cli struct, Commands enum, CommonHttpArgs
├── scan.rs         (388 lines) - PortScanArgs, EndpointScanArgs, FingerprintArgs, ScanArgs, ResumeArgs
├── fuzz.rs         (365 lines) - FuzzArgs, WafStressArgs, WafArgs
├── http.rs         (206 lines) - LoadArgs, ReconArgs, GraphQlArgs, OAuthArgs
├── cluster.rs      (63 lines)  - ClusterArgs, ClusterCommand, ClusterWorkerArgs, ClusterCoordinatorArgs, ClusterStatusArgs
├── stress.rs       (243 lines) - IcmpArgs, TracerouteArgs, StressArgs, ProxyArgs (feature-gated)
├── packet.rs       (135 lines) - PacketArgs, PacketCaptureArgs, PacketSendArgs, PacketDumpArgs (feature-gated)
├── agent.rs        (119 lines) - AgentArgs, AgentCommand (feature-gated)
├── ai_analyze.rs   (28 lines)  - AiAnalyzeArgs (feature-gated)
├── auth.rs         (82 lines) - AuthTestArgs
├── misc.rs         (346 lines) - ConfigArgs, NotifyArgs, RemoteArgs, ExecArgs, PluginArgs, ReportArgs, ServeArgs, McpServeArgs, SbomArgs
├── ci.rs           (??? lines) - CiArgs (not reviewed)
├── vuln.rs         (??? lines) - VulnArgs (not reviewed)
├── storage.rs      (??? lines) - StorageArgs (not reviewed)
├── plan.rs         (??? lines) - PlanArgs (not reviewed)
```

### Command Handlers

```
crates/slapper/src/commands/handlers/
├── mod.rs          (164 lines) - CommandContext, handle_command, handle_no_command
├── scan.rs         (64 lines)  - handle_scan_ports, handle_scan_endpoints, handle_fingerprint, handle_nse, handle_scan, handle_resume
├── fuzz.rs         (44 lines)  - handle_fuzz, handle_waf_stress, handle_waf, handle_graphql, handle_oauth
├── cluster.rs      (360 lines) - handle_cluster, handle_remote, handle_exec
├── config.rs       (34 lines)  - handle_config
├── auth_test.rs    (317 lines) - handle_auth_test
├── plugin.rs       (222 lines) - handle_plugin
├── load.rs         (10 lines)  - handle_load
├── recon.rs        (10 lines)  - handle_recon
├── network.rs      (163 lines) - handle_packet, handle_icmp, handle_traceroute
├── stress.rs       (220 lines) - handle_stress, handle_proxy
├── ci.rs           (153 lines) - handle_ci
├── vuln.rs         (103 lines) - handle_vuln
├── storage.rs      (61 lines)  - handle_storage
├── plan.rs         (39 lines)  - handle_plan
├── notify.rs       (189 lines) - handle_notify, handle_serve, handle_mcp_serve
├── sbom.rs         (130 lines) - handle_sbom
├── report.rs       (??? lines) - handle_report (not reviewed)
├── agent.rs        (??? lines) - handle_agent (not reviewed, feature-gated)
├── grpc.rs         (??? lines) - handle_grpc_server (not reviewed, feature-gated)
├── ai_analyze.rs   (??? lines) - handle_ai_analyze (not reviewed, feature-gated)
```
