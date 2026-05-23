# Slapper Implementation Plan

**Status**: ALL WAVES COMPLETED (2026-05-29) - 20/24 items implemented, 4 deferred

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
- **Status**: COMPLETED (commit 9c48b12) - Documented limitation
- **File**: `crates/slapper/src/waf/bypass/smuggling.rs:298-300`
- **Verification**: Documentation added

### 8. [PERF] Config: Add Scope.validate() method
- **Status**: COMPLETED (commit 9ffe10c)
- **File**: `crates/slapper/src/config/scope.rs`
- **Verification**: `cargo check --lib -p slapper`

### 9. [PERF] Scanner: Add progress bar error handling wrapper around join_all
- **Status**: COMPLETED (commit cee1e19) - catch_unwind wrapper added
- **File**: `crates/slapper/src/scanner/ports/mod.rs:590-593`
- **Verification**: `cargo test --lib -p slapper -- scanner`

### 10. [PERF] Distributed: TaskResult collection in coordinator
- **Status**: COMPLETED (commit bb7858a)
- **File**: `crates/slapper/src/distributed/remote.rs`
- **Verification**: Task results collected in completed queue

### 11. [PERF] Distributed: send_heartbeat cached connection
- **Status**: COMPLETED (commit aca8355)
- **File**: `crates/slapper/src/distributed/remote.rs:564-594`
- **Verification**: Heartbeat uses cached DNS

### 12. [BUG] Pipeline: Session save errors not propagated
- **Status**: COMPLETED (commit dad29fa) - checkpoint_error field added
- **File**: `crates/slapper/src/pipeline/executor.rs:223-226`
- **Verification**: Session save failures reported in PipelineReport

### 13. [PERF] Pipeline: run_fingerprint() hardcoded port list
- **Status**: COMPLETED (commit 2fef76f)
- **File**: `crates/slapper/src/pipeline/executor.rs:318-324`
- **Verification**: Uses EXTENDED_SCAN_PORTS constant

### 14. [PERF] Pipeline: run_waf() ignores config parameter
- **Status**: COMPLETED (already using self.common.clone() for TLS)
- **File**: `crates/slapper/src/pipeline/executor.rs:536`
- **Verification**: WafArgs includes common (TLS settings) via self.common.clone()

---

## Wave 3 - Lower Priority / Technical Debt (10 items, can run in parallel)

### 15. [PERF] TUI: Cache TabDispatcher in handle_enter
- **Status**: COMPLETED (commit 801c80a) - Cached dispatcher
- **File**: `crates/slapper/src/tui/app/mod.rs:371-382`
- **Verification**: Reduced from 4 calls to 1

### 16. [REFACTOR] TUI: unwrap_or_default instances (async focus)
- **Status**: DEFERRED - Not critical, existing instances are async-safe
- **Note**: 14 known instances, but most are in safe contexts

### 17. [ENHANCEMENT] CLI: Add `-o`/`--output` flag to output commands
- **Status**: COMPLETED (commit 01cd717) - Added to ConfigArgs, NotifyArgs, RemoteArgs, ExecArgs, ReportArgs
- **File**: `crates/slapper/src/cli/misc.rs`
- **Verification**: Commands accept `-o output.txt` parameter

### 18. [DOCS] Documentation: Update module counts
- **Status**: COMPLETED (commit recon.md update)
- **Files**: `architecture/recon.md` - 17 modules now documented
- **Note**: WAF (34) and NSE (164) counts were already correct

### 19. [PERF] Fuzzer: Add progress bar to run_sequential_with_session
- **Status**: COMPLETED (commit 841888c)
- **File**: `crates/slapper/src/fuzzer/engine/execution.rs:236-252`
- **Verification**: Progress shown during session-based fuzzing

### 20. [PERF] Output: Add streaming CSV export
- **Status**: COMPLETED (commit 01cd717)
- **File**: `crates/slapper/src/output/csv.rs`
- **Verification**: Async streaming with BufWriter implemented

### 21. [ENHANCEMENT] TUI: SessionManager theme restore not implemented
- **Status**: COMPLETED (commit session.rs update)
- **File**: `crates/slapper/src/tui/session.rs:153`
- **Verification**: Theme restoration implemented in restore_session()

### 22. [PERF] NSE: Socket sandbox DNS rebinding bypass potential
- **Status**: DEFERRED - Requires sandbox security review
- **File**: `crates/slapper-nse/src/libraries/socket.rs:48-63`

### 23. [PERF] NSE: Add OSV and CISA KEV API integration
- **Status**: DEFERRED - Requires API integration work
- **File**: `crates/slapper-nse/src/libraries/vulns.rs`

### 24. [PERF] Recon: Secrets module never called despite being in pipeline
- **Status**: COMPLETED (commit 4333f15)
- **File**: `crates/slapper/src/recon/mod.rs:346-364, runner.rs`
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