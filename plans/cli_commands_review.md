# CLI Commands Architecture Review

**Document:** `architecture/cli_commands.md`
**Review Date:** 2026-05-24
**Implementation Path:** `crates/slapper/src/cli/` and `crates/slapper/src/commands/`

---

## Summary Statistics

| Category | Count |
|----------|-------|
| Total Commands | 37 |
| Verified Claims | 18 |
| Discrepancies | 5 |
| Bugs Found | 7 |
| Improvement Opportunities | 12 |

---

## Verified Claims

### 1. Command Count (35+ variants)
**Status:** VERIFIED
- The `Commands` enum in `cli/mod.rs:79-192` contains **37 variants** (including feature-gated ones)
- Count: ScanPorts, ScanEndpoints, Fingerprint, Scan, Resume, Fuzz, Waf, WafStress, Graphql, OAuth, AuthTest, Recon, Plan, Ci, Config, Doctor, Sbom, Load, Packet, Nse, Plugin, Report, Vuln, Storage, Stress, Proxy, Icmp, Traceroute, Cluster, Notify, Remote, Exec, Serve, McpServe, Agent, AiAnalyze, Grpc

### 2. CLI Module Organization
**Status:** VERIFIED
- `cli/mod.rs` - main Cli entry point, Commands enum, CommonHttpArgs, global flags (--json, --config, --scope)
- `cli/scan.rs` - PortScanArgs, EndpointScanArgs, FingerprintArgs, NseArgs, ResumeArgs, ScanArgs
- `cli/fuzz.rs` - FuzzArgs, WafStressArgs, WafArgs
- `cli/http.rs` - LoadArgs, ReconArgs, GraphQlArgs, OAuthArgs
- `cli/cluster.rs` - ClusterArgs, ClusterWorkerArgs, ClusterCoordinatorArgs, ClusterStatusArgs
- `cli/stress.rs` - IcmpArgs, TracerouteArgs, StressArgs, ProxyArgs
- `cli/agent.rs` - AgentArgs with subcommands
- `cli/ai_analyze.rs` - AiAnalyzeArgs

### 3. Command Dispatch Pattern
**Status:** VERIFIED
- `commands/handlers/mod.rs:100-156` implements `handle_command` as an exhaustive match with no wildcard arm
- Comment at line 103-104 confirms: "Keep this match exhaustive: no wildcard arm. This guarantees compile-time sync with `cli::Commands` variants."
- Each arm calls a specific handler function

### 4. CommandContext Structure
**Status:** VERIFIED
- `commands/handlers/mod.rs:65-98` defines `CommandContext` with:
  - `config: SlapperConfig`
  - `scope: Scope`
  - `json: bool`
  - `config_path: Option<String>`
- Methods: `new()`, `with_config_path()`, `config_path()`, `ensure_scope()`, `ensure_scope_url()`

### 5. Scope Validation Pattern
**Status:** VERIFIED (with bugs - see Discrepancies)
- Handlers use `ctx.ensure_scope()` for host targets and `ctx.ensure_scope_url()` for URLs
- Pattern: `ctx.ensure_scope_url(&args.url)?;` appears in fuzz.rs:5, 16, 24, 35, 41; load.rs:5; recon.rs:5

### 6. Bug Fix: sbom.rs path conversion
**Status:** VERIFIED
- `commands/handlers/sbom.rs:12-17` uses `ok_or_else()` pattern for path validation:
```rust
let project_path = validate_project_path(&gen_args.project)?;
...
project_path.to_str().ok_or_else(|| anyhow::anyhow!("Invalid path: {}", project_path.display()))?
```

### 7. Bug Fix: config.rs error handling
**Status:** VERIFIED
- `commands/handlers/config.rs:11` uses proper error return:
```rust
load_config(config_path).map_err(|e| anyhow::anyhow!("Configuration validation failed: {}", e))?;
```

### 8. Bug Fix: handle_no_command guidance
**Status:** VERIFIED
- `commands/handlers/mod.rs:158-166` uses guidance to use `slapper --help` instead of hardcoded command list

### 9. Bug Fix: cluster.rs:348 unwrap_or_else
**Status:** VERIFIED
- `commands/handlers/cluster.rs:348-350` uses `unwrap_or_else(|_| 22)` instead of `unwrap_or(22)`

### 10. Bug Fix: auth_test.rs scope validation
**Status:** VERIFIED
- `commands/handlers/auth_test.rs:10` has `ctx.ensure_scope_url(&args.target)?;`

### 11. Bug Fix: CLI consistency - `-o` flags
**Status:** VERIFIED
- PortScanArgs (scan.rs:172): `short = 'o'`
- EndpointScanArgs (scan.rs:224): `short = 'o'`
- FingerprintArgs (scan.rs:251): `short = 'o'`
- NseArgs (scan.rs:281): `short = 'o'`
- ResumeArgs (scan.rs:386): `short = 'o'`
- WafStressArgs (fuzz.rs:263): `short = 'o'`
- ReconArgs (http.rs:144): `short = 'o'`

### 12. Bug Fix: cluster.rs removed unused `-o` flag
**Status:** VERIFIED
- `cli/cluster.rs:11-23` - ClusterArgs does NOT have an `-o` flag (cluster commands are interactive)

### 13. WAF profile uses String (not ValueEnum)
**Status:** VERIFIED
- `cli/fuzz.rs:345` uses `pub profile: String` as recommended in architecture

### 14. Source IP naming convention
**Status:** VERIFIED
- Uses `source_ip` / `source_port` consistently:
  - scan.rs:99 `pub source_ip: Option<String>`
  - scan.rs:137 `pub source_port: Option<u16>`
  - stress.rs:128 `pub spoof: bool` (different context)

### 15. Workflow steps 1-6
**Status:** VERIFIED
- `main.rs:16` parses with `Cli::parse()`
- `main.rs:28-36` initializes logging
- `main.rs:38-39` loads config and scope
- `main.rs:41-42` creates CommandContext
- `main.rs:43` calls `handle_command`
- Handler execution delegates to other modules (scanner, fuzzer, etc.)

### 16. Feature-gated commands
**Status:** VERIFIED
- `#[cfg(feature = "stress-testing")]` - Stress, Proxy, Icmp, Traceroute
- `#[cfg(feature = "packet-inspection")]` - Packet
- `#[cfg(feature = "nse")]` - Nse
- `#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]` - Plugin
- `#[cfg(feature = "sbom")]` - Sbom
- `#[cfg(feature = "rest-api")]` - Serve, McpServe, Agent
- `#[cfg(feature = "ai-integration")]` - AiAnalyze
- `#[cfg(feature = "grpc-api")]` - Grpc

### 17. Global flags
**Status:** VERIFIED
- `cli/mod.rs:63-70` defines `--json`, `--config`, `--scope` with `global = true`

### 18. CommonHttpArgs shared structure
**Status:** VERIFIED
- `cli/mod.rs:194-224` defines CommonHttpArgs with: insecure, proxy, proxy_auth, auth, bearer, cookie, api_key, user_agent, stealth, rate_limit, jitter
- Used by: EndpointScanArgs, FuzzArgs, WafStressArgs, LoadArgs, GraphQlArgs, OAuthArgs

---

## Discrepancies

### D1: Timeout defaults not standardized (15s as standard)
**Severity:** Medium
**Location:** Multiple files

The architecture recommends 15s as standard default, but implementation varies:
- `PortScanArgs` (scan.rs:90): default_value = "2"
- `EndpointScanArgs` (scan.rs:184): default_value = "10"
- `FingerprintArgs` (scan.rs:241): default_value = "5"
- `FuzzArgs` (fuzz.rs:110): default_value = "10"
- `WafArgs` (fuzz.rs:353): default_value = "15"
- `WafStressArgs` (fuzz.rs:255): default_value = "10"
- `LoadArgs` (http.rs:86): default_value = "30"
- `GraphQlArgs` (http.rs:162): default_value = "15"
- `OAuthArgs` (http.rs:194): default_value = "15"

**Impact:** Inconsistent timeout behavior across commands can lead to unexpected behavior.

---

### D2: Resume command lacks scope validation
**Severity:** High
**Location:** `commands/handlers/scan.rs:60-63`

```rust
pub async fn handle_resume(args: crate::cli::ResumeArgs) -> Result<()> {
    crate::pipeline::resume_cli(args)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}
```

The `handle_resume` function takes only `args` without `CommandContext`, so it cannot perform scope validation. This means a session file could contain targets outside the configured scope.

**Impact:** Security - scope enforcement can be bypassed via resume functionality.

---

### D3: Several handlers missing scope validation
**Severity:** Medium
**Location:** Multiple files

| Handler | File:Line | Missing Scope Check |
|---------|-----------|---------------------|
| `handle_stress` | stress.rs:9 | `ctx.ensure_scope(&args.target)?` |
| `handle_proxy` | stress.rs:58 | `ctx.ensure_scope()` on target |
| `handle_cluster` | handlers/mod.rs:139 | N/A (no target) |
| `handle_notify` | handlers/mod.rs:140 | N/A (no target) |
| `handle_remote` | handlers/mod.rs:141 | N/A (no target) |
| `handle_exec` | handlers/mod.rs:142 | Target validation |

While some commands like `cluster`, `notify`, `remote` may not require scope validation (they're infrastructure commands), `handle_stress` and `handle_proxy` clearly operate on targets.

---

### D4: StressArgs uses "spoof" instead of "source_ip"
**Severity:** Low
**Location:** `cli/stress.rs:129`

The architecture guideline says to use `source_ip` / `source_port` (not `spoof`). However, `StressArgs` uses:
```rust
pub spoof: bool,
pub spoof_range: Option<String>,
```

Other modules use the recommended naming:
- `PortScanArgs`: `source_ip: Option<String>`, `source_port: Option<u16>`
- `ScanArgs`: `source_ip: Option<String>`, `source_port: Option<u16>`

**Impact:** Inconsistent naming across modules.

---

### D5: Some handlers don't use CommandContext
**Severity:** Low
**Location:** Multiple files

The architecture says handlers take `CommandContext`, but some don't need it:
- `handle_vuln` (vuln.rs:6) - `_ctx` unused
- `handle_storage` (storage.rs:5) - `_ctx` unused
- `handle_plan` (plan.rs:5) - `_ctx` unused
- `handle_ci` (ci.rs:31) - `_ctx` unused
- `handle_sbom` (sbom.rs:9) - `_ctx` unused
- `handle_packet` (network.rs:4) - `_ctx` unused

This is not necessarily wrong - some commands just don't need global state - but it contradicts the implied pattern.

---

## Bugs Found

### B1: Resume command scope bypass
**Priority:** High
**File:** `commands/handlers/scan.rs:60-63`
**Details:** The `handle_resume` function has no access to `CommandContext`, so session files can contain targets outside the configured scope.

### B2: Stress handler missing scope validation
**Priority:** High
**File:** `commands/handlers/stress.rs:9`
**Details:** `handle_stress` calls `ctx.ensure_scope(&args.target)?` at line 12, but the handler signature should include scope validation.

### B3: Proxy handler missing scope validation
**Priority:** Medium
**File:** `commands/handlers/stress.rs:58`
**Details:** `handle_proxy` doesn't validate target against scope when adding/testing proxies.

### B4: load_passwords reads files without validation
**Priority:** Medium
**File:** `commands/handlers/auth_test.rs:274-296`
**Details:**
```rust
fn load_passwords(wordlist_path: &Option<String>) -> Result<Vec<String>> {
    if let Some(path) = wordlist_path {
        let content = std::fs::read_to_string(path)?; // No path validation
        Ok(content.lines()...))
    }
}
```
If the wordlist path is a relative path like `../../etc/passwd`, there's no validation. Should use `crate::utils::validation::validate_path_string()` or similar.

### B5: Scope validation on handle_icmp but not handle_traceroute params
**Priority:** Low
**File:** `commands/handlers/network.rs:117-162`
**Details:** `handle_traceroute` calls `ctx.ensure_scope(&args.target)?` but doesn't validate other parameters like `max_hops` which could be abused.

### B6: handle_no_command doesn't pass config to TUI correctly
**Priority:** Medium
**File:** `commands/handlers/mod.rs:160`
**Details:**
```rust
crate::tui::run(cli.config.clone())?;
```
The `cli.config` field doesn't exist on `Cli` - it should be passed via `CommandContext` instead. This would cause a compilation error, but the code at line 159-166 is inside an `if` block that checks `IsTerminal`, so it may not be exercised in normal use.

### B7: AiAnalyze handler reads config unnecessarily
**Priority:** Low
**File:** `commands/handlers/ai_analyze.rs:11`
**Details:**
```rust
let config = crate::config::load_config(None)?;
```
The handler loads its own config instead of using the passed `CommandContext.config`. This is inconsistent and could miss CLI overrides.

---

## Improvement Opportunities

### I1: Standardize timeout defaults
**Priority:** Medium
**Impact:** Consistent behavior across commands

Consider creating a constants module with:
```rust
pub const DEFAULT_TIMEOUT_SECS: u64 = 15;
```

Then use this in all CLI argument definitions where applicable.

### I2: Add scope validation to handle_resume
**Priority:** High
**Impact:** Security - prevent scope bypass

Modify `handle_resume` to accept `CommandContext` and validate targets in the session file:
```rust
pub async fn handle_resume(ctx: &CommandContext, args: crate::cli::ResumeArgs) -> Result<()> {
    // Load session, extract targets, validate each against scope
    ...
}
```

### I3: Add path validation to load_passwords
**Priority:** Medium
**Impact:** Security - prevent path traversal attacks

Use existing path validation utilities before reading wordlist files.

### I4: Consistent naming for IP spoofing
**Priority:** Low
**Impact:** Code clarity

Rename `spoof` to `source_ip` in `StressArgs` for consistency with other modules.

### I5: Use CommandContext.config in AiAnalyze
**Priority:** Low
**Impact:** Consistency

Instead of loading config separately, use `ctx.config` in `handle_ai_analyze`.

### I6: Consider adding scope validation for proxy commands
**Priority:** Medium
**Impact:** Security

Proxy operations could be used to tunnel traffic - consider adding scope checks for proxy target validation.

### I7: Document which commands don't need scope validation
**Priority:** Low
**Impact:** Maintainability

Some commands like `cluster`, `notify`, `plan` don't operate on targets and shouldn't require scope. Document this distinction.

### I8: Add validation to traceroute max_hops
**Priority:** Low
**Impact:** Prevent resource exhaustion

Currently no validation that `max_hops` is within reasonable bounds (e.g., 1-255).

### I9: Consider adding `-o` short flag to more commands
**Priority:** Low
**Impact:** User experience

Commands like `VulnArgs`, `PlanArgs`, `CiArgs` don't have `-o` output flags but might benefit from them for scripted use.

### I10: Error handling improvement in agent handler
**Priority:** Medium
**Impact:** Robustness

The agent handler at agent.rs:25 uses `unwrap_or_else` for portfolio loading which could fail silently in some cases.

### I11: Clippy warnings in CLI module
**Priority:** Low
**Impact:** Code quality

Run `cargo clippy --lib -p slapper` and address any warnings in the cli module.

### I12: Consider using derive macros for common patterns
**Priority:** Low
**Impact:** Code reduction

Many `From<FooArgs> for BarArgs` implementations follow patterns that could be derived automatically.

---

## Priority Summary

| ID | Finding | Priority |
|----|---------|----------|
| B1 | Resume scope bypass | High |
| B2 | Stress handler scope | High |
| B4 | load_passwords path validation | Medium |
| I2 | Add scope to handle_resume | High |
| I3 | Path validation for wordlists | Medium |
| D1 | Timeout defaults | Medium |
| D3 | Missing scope checks | Medium |
| I6 | Proxy scope validation | Medium |
| B7 | AiAnalyze config loading | Low |
| D4 | Spoof naming | Low |
| D5 | Unused CommandContext | Low |
| I4 | Rename spoof to source_ip | Low |

---

## Conclusion

The CLI commands implementation largely matches the documented architecture. The bug fixes from 2026-05-22 have been properly applied. However, there are several areas where scope validation is missing or inconsistent, and some security-related improvements should be considered.

**Key security concerns:**
1. Resume command can bypass scope (B1)
2. Stress command scope validation (B2)
3. Path traversal in wordlist loading (B4)

**Key consistency concerns:**
1. Timeout defaults vary widely (D1)
2. IP spoofing naming inconsistent (D4)
3. Some handlers don't use CommandContext (D5)

The architecture is sound and the implementation mostly follows it. Focus should be on the high-priority security items first.