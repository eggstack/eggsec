# Slapper Implementation Plan

**Status**: Wave 1 COMPLETED (2026-05-29), Wave 2 PENDING

## Overview

This plan consolidates findings from 14 architecture review documents. Items marked "COMPLETED" were verified in code. Items marked "PENDING" need implementation. Items can be executed in parallel within each wave.

---

## Wave 1 - High Priority (6 items, can run in parallel)

### 1. [PERF] PluginManager: Replace HashMap with FxHashMap
- **Status**: COMPLETED (commit 7827441)
- **File**: `crates/slapper-plugin/src/lib.rs:296-297`
- **Verification**: `cargo clippy --lib -p slapper-plugin`

### 2. [BUG] Ruby Plugin: load_plugin_with_timeout ignores timeout parameter
- **Status**: COMPLETED (commit 0305fb3) - Clarified with `_timeout` prefix
- **File**: `crates/slapper-ruby/src/bridge.rs:285`
- **Verification**: Timeout IS used via `rx.recv_timeout()` - just clarified variable naming

### 3. [BUG] CMS Scanner: Replace unwrap_or_default with explicit error handling
- **Status**: COMPLETED (commit b917251)
- **Files**: `cms/mod.rs:248`, `cms/joomla.rs:35`, `cms/drupal.rs:35`, `cms/wordpress.rs:156`
- **Verification**: `cargo test --lib -p slapper -- scanner`

### 4. [BUG] AI CacheKeyBuilder: Colon separator collision
- **Status**: COMPLETED (commit 480ea69) - Changed to `\x00` separator
- **File**: `crates/slapper/src/ai/cache.rs:293-307`
- **Verification**: Unit test added for collision prevention

### 5. [REFACTOR] AI Agents: Replace remaining HashMap with FxHashMap
- **Status**: COMPLETED (already FxHashMap in codebase)
- **Files**: Verified `alerts/mod.rs`, `constraints/checker.rs`, `portfolio.rs` all use FxHashMap
- **Verification**: `cargo clippy --lib -p slapper`

### 6. [BUG] NSE: CVE-2024-27956 needs structural fix, not just documentation
- **Status**: COMPLETED (commit 1f2ced1) - Vec-based storage implemented
- **File**: `crates/slapper-nse/src/libraries/vulns.rs`
- **Verification**: `cargo check -p slapper-nse`

---

## Wave 2 - Medium Priority (8 items, can run in parallel)

### 7. [PERF] WAF HTTP/2 smuggling bypass not implemented
- **File**: `crates/slapper/src/waf/bypass/smuggling.rs:298-300`
- **Issue**: `supports_http2_probes()` hardcoded to return `false`. H2CUpgrade, Http2Frame techniques never execute
- **Fix**: Implement actual HTTP/2 detection and smuggling techniques, OR document as limitation
- **Verification**: Test with HTTP/2-enabled server

### 8. [PERF] Config: Add Scope.validate() method
- **File**: `crates/slapper/src/config/scope.rs`
- **Issue**: `Scope` struct lacks a `validate()` method like other config types (`ScanConfig`, `WebhookConfig`)
- **Fix**: Add `pub fn validate(&self) -> Result<(), ScopeError>` for consistency
- **Verification**: Scope implements similar validation pattern

### 9. [PERF] Scanner: Add progress bar error handling wrapper around join_all
- **File**: `crates/slapper/src/scanner/ports/mod.rs:590-593`
- **Issue**: Progress bar finished before all results processed; `join_all` panic leaves progress bar in wrong state
- **Fix**: Add error handling wrapper around `join_all` to properly finalize progress bar
- **Verification**: `cargo test --lib -p slapper -- scanner`

### 10. [PERF] Distributed: TaskResult collection in coordinator
- **File**: `crates/slapper/src/distributed/remote.rs` - `RemoteListener`
- **Issue**: `TaskQueue::complete()` exists but coordinator never calls it. Results not collected
- **Fix**: Add `TaskQueue` instance to `RemoteListener` and call `complete()` when results arrive
- **Verification**: Task results appear in `completed` queue

### 11. [PERF] Distributed: send_heartbeat cached connection
- **File**: `crates/slapper/src/distributed/remote.rs:564-594`
- **Issue**: `send_heartbeat()` calls `connect_to_coordinator()` directly without using `cached_addr` mechanism
- **Fix**: Use the same pattern as `execute()` - check `resolve_cached()` first
- **Reference**: `execute()` at `remote.rs:605-615` uses `resolve_cached()` properly
- **Verification**: Heartbeat uses cached DNS and connection

### 12. [BUG] Pipeline: Session save errors not propagated
- **File**: `crates/slapper/src/pipeline/executor.rs:223-226`
- **Issue**: Session save failure only logs warning, doesn't return error to caller
- **Fix**: Change from `tracing::warn!()` to `tracing::error!()` with return value
- **Verification**: Session save failures are reported, not silently ignored

### 13. [PERF] Pipeline: run_fingerprint() hardcoded port list
- **File**: `crates/slapper/src/pipeline/executor.rs:318-324`
- **Issue**: Uses hardcoded port list instead of `EXTENDED_SCAN_PORTS` constant
- **Fix**: Use `EXTENDED_SCAN_PORTS` constant for consistency with other scanner code
- **Verification**: Port constants centralized in one location

### 14. [PERF] Pipeline: run_waf() ignores config parameter
- **File**: `crates/slapper/src/pipeline/executor.rs:536`
- **Issue**: WAF doesn't receive config, missing TLS verification settings
- **Fix**: Pass config to `run_waf()` so WAF respects TLS settings
- **Verification**: TLS verification settings respected by WAF

---

## Wave 3 - Lower Priority / Technical Debt (10 items, can run in parallel)

### 15. [PERF] TUI: Cache TabDispatcher in handle_enter
- **File**: `crates/slapper/src/tui/app/mod.rs:371-382`
- **Issue**: `dispatcher_mut()` called 4 times per Enter keypress
- **Fix**: Extract to local variable to reduce redundant calls
- **Priority**: HIGH per TUI review

### 16. [REFACTOR] TUI: unwrap_or_default instances (async focus)
- **Files**: Multiple in `crates/slapper/src/tui/` (14 known instances)
- **Issue**: Silent failure in async operations hides errors
- **Fix**: Identify problematic instances (async operations) and replace with explicit match + tracing
- **Note**: Not all instances are problematic - focus on async operations
- **Verification**: `cargo test --lib -p slapper -- tui`

### 17. [ENHANCEMENT] CLI: Add `-o`/`--output` flag to output commands
- **File**: `crates/slapper/src/cli/misc.rs`
- **Issue**: `ConfigArgs`, `NotifyArgs`, `RemoteArgs`, `ExecArgs`, `ReportArgs` lack `-o`/`--output` flag for inconsistent UX
- **Fix**: Add output file path argument to these commands
- **Verification**: Commands accept `-o output.txt` parameter

### 18. [DOCS] Documentation: Update module counts
- **Files**: `architecture/recon.md`, `architecture/waf.md`, `architecture/overview.md`
- **Issues**:
  - Recon: 16 modules documented vs 17 actual (missing "secrets")
  - WAF: 23 WAFs documented vs 34 actual (26 explicit + 8 auto-generated)
  - NSE: 164 libraries documented vs 169 actual (NOTE: review says 164 is correct count)
- **Verification**: Doc counts match actual code
- **Note**: NSE library count (164) is actually CORRECT per review - don't change

### 19. [PERF] Fuzzer: Add progress bar to run_sequential_with_session
- **File**: `crates/slapper/src/fuzzer/engine/execution.rs:236-252`
- **Issue**: Unlike `run_sequential()`, no progress tracking
- **Fix**: Add progress bar for consistency
- **Verification**: Progress shown during session-based fuzzing

### 20. [PERF] Output: Add streaming CSV export
- **File**: `crates/slapper/src/output/csv.rs:9-78`
- **Issue**: Builds complete `String` in memory for large reports
- **Fix**: Add async streaming version using `tokio::io::BufWriter`
- **Verification**: Large CSV exports don't OOM

### 21. [ENHANCEMENT] TUI: SessionManager theme restore not implemented
- **File**: `crates/slapper/src/tui/session.rs:153`
- **Issue**: `theme_name` captured but not restored on load
- **Fix**: Implement theme restoration on session load
- **Verification**: Theme persists across sessions

### 22. [PERF] NSE: Socket sandbox DNS rebinding bypass potential
- **File**: `crates/slapper-nse/src/libraries/socket.rs:48-63`
- **Issue**: Sandbox may not properly validate DNS rebinding attacks
- **Fix**: Add DNS rebinding protection if not present
- **Verification**: Security test for DNS rebinding

### 23. [PERF] NSE: Add OSV and CISA KEV API integration
- **File**: `crates/slapper-nse/src/libraries/vulns.rs`
- **Issue**: Only NVD implemented, OSV and CISA KEV not integrated
- **Fix**: Add additional CVE source APIs
- **Verification**: More comprehensive CVE coverage

### 24. [PERF] Recon: Secrets module never called despite being in pipeline
- **File**: `crates/slapper/src/recon/mod.rs:346-364, runner.rs`
- **Issue**: `FULL_RECON_PIPELINE_MODULES` includes "secrets" but it's never invoked
- **Fix**: Ensure secrets module is actually called in the pipeline
- **Verification**: Secrets enumeration runs as part of recon

---

## Already VERIFIED as Fixed (DO NOT IMPLEMENT)

| Item | Status | Evidence |
|------|--------|----------|
| Loadtest error list cap 1000 | COMPLETED | `metrics.rs:101,109` uses 1000 |
| Cloud parallelization | COMPLETED | `cloud/mod.rs:66` uses `tokio::join!` |
| Distributed queue race condition | NOT A BUG | `queue.rs:78-79` acquires both locks upfront |
| Fuzzer LazyLock per-type | COMPLETED | `payloads/mod.rs:140-150` uses LazyLock correctly |
| WAF HEADER_VALUE_MAX_LEN | COMPLETED | `waf/detector/detect.rs:10` defines at module level |
| Config private IP check | COMPLETED | `scope.rs:226,280` uses `is_private_ip()` |
| Fuzzer rate < 1 | COMPLETED | `execution.rs:267` uses `rate < 1` |
| NSE library count 164 | CORRECT | No change needed, documentation was accurate |

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
cargo clippy --lib -p slapper-ruby
```

---

## Dependencies & Parallelization

- **Wave 1 (Items 1-6)**: Independent, can assign to 6 parallel agents
- **Wave 2 (Items 7-14)**: Independent, can assign to 8 parallel agents
- **Wave 3 (Items 15-24)**: Independent, can assign to 10 parallel agents
- **No cross-wave dependencies identified**
- **Verification step**: Run verification commands after each wave

---

## Security-Critical Issues

1. **Private IP Bypass** - VERIFIED FIXED: `is_private_ip()` check in `scope.rs:226,280`
2. **CVE-2024-27956** - NEEDS FIX: HashMap overwrite - implement Vec-based storage or document limitation
3. **Ruby Timeout** - VERIFIED: `recv_timeout` IS used, may be false positive - verify before changing

---

## Additional Items from Reviews (Lower Priority - Consider for Future)

These items were identified but are lower priority. Consider for future planning sessions:

| Item | Priority | Source |
|------|----------|--------|
| TUI: Command palette FxHashMap | Low | TUI review |
| TUI: Overlay precedence tests | Low | TUI review |
| Output: TrendAnalyzer unbounded history | Medium | Output review |
| Output: PDF 30-finding limit | Low | Output review |
| Output: ScanSession bincode serialization | Medium | Output review |
| Recon: TLS cert expiration warning | Medium | Recon review |
| Recon: CVE engine blocking HTTP | Medium | Recon review |
| Fuzzer: JWT unwrap_or_default | Medium | Fuzzer review |
| Fuzzer: GrammarFuzzer RNG not serializable | Medium | Fuzzer review |
| Networking: IPv4 options bounds check | High | Networking review |
| Networking: DNS name parsing heap alloc | High | Networking review |
| PluginManager: Lazy lock contention | Low | Plugins/NSE review |
| NSE: Sandbox violation metrics | Low | Plugins/NSE review |
| Pipeline: concurrent_stages CLI flag | Medium | Pipeline review |

---

*Plan consolidated: 2026-05-23*
*Source reviews: ai_agents, cli_commands, config, distributed, fuzzer, loadtest, networking, output, overview, pipeline, recon, scanner, tui, waf, plugins_nse*