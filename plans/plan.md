# Slapper Implementation Plan

**Status**: PENDING (2026-05-23)

New items identified from architecture review. Previous 26 items from Wave 1-3 remain COMPLETED.

---

## Wave 1 - High Priority (Can implement in parallel)

### 1. [FIX] PluginManager: Replace HashMap with FxHashMap
- **File**: `crates/slapper-plugin/src/lib.rs:296-297`
- **Issue**: `plugins: HashMap<String, PluginInfo>` and `configs: HashMap<String, PluginConfig>` use std HashMap
- **Fix**: Change to `FxHashMap` for performance
- **Verification**: `cargo clippy --lib -p slapper-plugin`

### 2. [FIX] Ruby Plugin: load_plugin_with_timeout ignores timeout parameter
- **File**: `crates/slapper-ruby/src/bridge.rs:285`
- **Issue**: `let _ = timeout_secs;` discards the timeout value without using it
- **Fix**: Use `tokio::time::timeout` around the plugin loading operation
- **Reference**: See `loader.rs:118` which does use timeout for `run_plugin`
- **Verification**: Check that plugin loading respects timeout value

### 3. [FIX] CMS Scanner: Replace unwrap_or_default with explicit error handling
- **Files**:
  - `crates/slapper/src/scanner/cms/mod.rs:248` - `resp.text().await.unwrap_or_default()`
  - `crates/slapper/src/scanner/cms/joomla.rs:35` - `resp.text().await.unwrap_or_default()`
  - `crates/slapper/src/scanner/cms/drupal.rs:35` - `resp.text().await.unwrap_or_default()`
  - `crates/slapper/src/scanner/cms/wordpress.rs:45,74,156` - Additional instances
- **Issue**: Silent failure returns empty string instead of propagating error; CMS detection may succeed but component enumeration silently fails
- **Fix**: Replace with explicit match and `tracing::debug` for network failures
- **Verification**: `cargo test --lib -p slapper -- scanner`

### 4. [FIX] AI CacheKeyBuilder: Colon separator collision
- **File**: `crates/slapper/src/ai/cache.rs:293-307`
- **Issue**: Uses `:` as separator. If `vuln_type` or `context` contains colon, cache keys collide
- **Fix**: Use a different separator (e.g., `\x00` or `|` ) or escape colons
- **Verification**: Add unit test with inputs containing colons

### 5. [FIX] NSE: CVE-2024-27956 duplicate entry documented
- **File**: `crates/slapper-nse/src/libraries/vulns.rs:208-243`
- **Issue**: CVE-2024-27956 inserted twice (AutomateWoo at line 214, WooCommerce at line 237). HashMap only stores one entry, second overwrites first
- **Fix**: Add comment explaining this is intentional limitation (two different plugins trigger same CVE)
- **Verification**: Comment exists explaining behavior

---

## Wave 2 - Medium Priority (Can implement in parallel)

### 6. [PERF] Config: Add Scope.validate() method
- **File**: `crates/slapper/src/config/scope.rs`
- **Issue**: `Scope` struct lacks a `validate()` method like other config types (`ScanConfig`, `WebhookConfig`)
- **Fix**: Consider adding `pub fn validate(&self) -> Result<(), ScopeError>` for consistency
- **Verification**: Scope implements similar validation pattern

### 7. [PERF] Config: Document allowed_ports in architecture
- **File**: `architecture/config.md`
- **Issue**: `Scope` has `allowed_ports: Option<Vec<u16>>` and `excluded_ports: Vec<u16>` but not documented
- **Fix**: Add documentation for port-based scope filtering
- **Verification**: Architecture doc reflects current Scope fields

### 8. [PERF] Scanner: Add progress bar error handling wrapper around join_all
- **File**: `crates/slapper/src/scanner/ports/mod.rs:590-593`
- **Issue**: Progress bar finished before all results processed; `join_all` panic leaves progress bar in wrong state
- **Fix**: Add error handling wrapper around `join_all` to properly finalize progress bar
- **Verification**: `cargo test --lib -p slapper -- scanner`

### 9. [PERF] Distributed: Implement TaskResult collection in coordinator
- **File**: `crates/slapper/src/distributed/remote.rs` - `RemoteListener`
- **Issue**: `TaskQueue::complete()` exists but coordinator never calls it. Results not collected
- **Fix**: Add `TaskQueue` instance to `RemoteListener` and call `complete()` when results arrive
- **Verification**: Task results appear in `completed` queue

### 10. [PERF] Distributed: send_heartbeat doesn't use cached connection
- **File**: `crates/slapper/src/distributed/remote.rs:564-594`
- **Issue**: `send_heartbeat()` calls `connect_to_coordinator()` directly without using `cached_addr` mechanism
- **Fix**: Use the same pattern as `execute()` - check `resolve_cached()` first
- **Reference**: `execute()` at `remote.rs:605-615` uses `resolve_cached()` properly
- **Verification**: Heartbeat uses cached DNS and connection

### 11. [PERF] WAF: HTTP/2 smuggling bypass not implemented
- **File**: `crates/slapper/src/waf/bypass/smuggling.rs:298-300`
- **Issue**: `supports_http2_probes()` hardcoded to return `false`. H2CUpgrade, Http2Frame techniques never execute
- **Fix**: Implement actual HTTP/2 detection and smuggling techniques, or document as limitation
- **Verification**: Test with HTTP/2-enabled server

---

## Wave 3 - Lower Priority / Technical Debt

### 12. [REFACTOR] TUI: unwrap_or_default instances
- **Files**: Multiple in `crates/slapper/src/tui/` (14 known instances)
- **Issue**: Silent failure in async operations hides errors
- **Fix**: Identify problematic instances (async operations) and replace with explicit match + tracing
- **Note**: Not all instances are problematic - focus on async operations
- **Verification**: `cargo test --lib -p slapper -- tui`

### 13. [ENHANCEMENT] CLI: Add `-o`/`--output` flag to output commands
- **File**: `crates/slapper/src/cli/misc.rs`
- **Issue**: `ConfigArgs`, `NotifyArgs`, `RemoteArgs`, `ExecArgs`, `ReportArgs` lack `-o`/`--output` flag for inconsistent UX
- **Fix**: Add output file path argument to these commands
- **Verification**: Commands accept `-o output.txt` parameter

### 14. [DOCS] Documentation: Update module counts
- **Files**: `architecture/recon.md`, `architecture/waf.md`, `architecture/overview.md`
- **Issues**:
  - Recon: 16 modules documented vs 17 actual
  - WAF: 24 WAFs documented vs 34 actual (26 explicit + 8 auto-generated)
  - NSE: 164 libraries documented vs 169 actual
- **Verification**: Doc counts match actual code

### 15. [PERF] Pipeline: run_waf() ignores config parameter
- **File**: `crates/slapper/src/pipeline/executor.rs:536`
- **Issue**: WAF doesn't receive config, missing TLS verification settings
- **Fix**: Pass config to `run_waf()` so WAF respects TLS settings
- **Verification**: TLS verification settings respected by WAF

### 16. [PERF] Fuzzer: Add progress bar to run_sequential_with_session
- **File**: `crates/slapper/src/fuzzer/engine/execution.rs:236-252`
- **Issue**: Unlike `run_sequential()`, no progress tracking
- **Fix**: Add progress bar for consistency
- **Verification**: Progress shown during session-based fuzzing

### 17. [PERF] Output: Add streaming CSV export
- **File**: `crates/slapper/src/output/csv.rs:9-78`
- **Issue**: Builds complete `String` in memory for large reports
- **Fix**: Add async streaming version using `tokio::io::BufWriter`
- **Verification**: Large CSV exports don't OOM

---

## Already VERIFIED as Fixed

| Item | Status | Evidence |
|------|--------|----------|
| Loadtest error list cap 1000 | COMPLETED | `metrics.rs:101,109` uses 1000 |
| Cloud parallelization | COMPLETED | `cloud/mod.rs:66` uses `tokio::join!` |
| Distributed queue race condition | NOT A BUG | `queue.rs:78-79` acquires both locks upfront before operations |
| Fuzzer LazyLock per-type | COMPLETED | `payloads/mod.rs:140-150` uses LazyLock correctly |
| WAF HEADER_VALUE_MAX_LEN | COMPLETED | `waf/detector/detect.rs:10` defines at module level |
| Config private IP check | COMPLETED | `scope.rs:226,280` uses `is_private_ip()` |
| Fuzzer rate < 1 | COMPLETED | `execution.rs:267` uses `rate < 1` |
| AI Agents HashMap->FxHashMap | COMPLETED | All 65+ instances use FxHashMap per grep |

---

## Verification Commands

```bash
cargo check --lib -p slapper
cargo check --lib -p slapper-plugin
cargo check --lib -p slapper-ruby
cargo check -p slapper-nse
cargo test --lib -p slapper
cargo test --test negative_tests -p slapper
cargo test --test scanner_tests -p slapper
cargo clippy --lib -p slapper
cargo clippy --lib -p slapper-plugin
```

---

## Dependencies

- Wave 1 items (1-5) are independent and can be implemented in parallel by 5 agents
- Wave 2 items (6-11) are independent and can be implemented in parallel by 6 agents
- Wave 3 items (12-17) are independent and can be implemented in parallel by 6 agents
- No cross-wave dependencies identified

---

## Security-Critical Issues (Already Mitigated)

1. **Private IP Bypass** - VERIFIED FIXED: `is_private_ip()` check in `scope.rs:226,280`
2. **CVE Duplicate Entry** - DOCUMENTED: Comment at `vulns.rs:208` explains limitation
3. **Ruby Timeout Ignored** - NEEDS FIX: Line 285 discards timeout

---

*Plan created: 2026-05-23*
*Previous plan (26 items) marked COMPLETED in `plan.md.backup-2026-05-29`*