# Slapper Implementation Plan

**Status**: ALL ITEMS ADDRESSED (2026-05-29) - 20/24 completed, 4 deferred then pruned after review

## Overview

This plan consolidates findings from 14 architecture review documents. All implementation items have been completed and verified. Remaining items are either:
- **Verified as already correct** (not bugs)
- **Pruned** (deferred items that were reviewed and determined unnecessary)
- **Future considerations** (lower priority items for later planning)

---

## Completed Implementation Items (20 items)

All 20 implementation items from Waves 1-3 were completed and verified:

| Wave | Items | Status |
|------|-------|--------|
| Wave 1 | 1-6 | Completed |
| Wave 2 | 7-14 | Completed |
| Wave 3 | 15, 17-21, 24 | Completed |

See git history for commit references. Key implementations:
- PluginManager: FxHashMap (`crates/slapper-plugin/src/lib.rs:296-297`)
- Ruby timeout: Properly enforced at RubyPluginClient level (`bridge.rs:87`)
- CMS Scanner: Explicit error handling (`scanner/cms/*.rs`)
- AI CacheKeyBuilder: Null byte separator (`ai/cache.rs`)
- AI Agents: FxHashMap throughout
- NSE CVE-2024-27956: Vec-based storage (`slapper-nse/src/libraries/vulns.rs`)
- WAF HTTP/2: Documented limitation (`waf/bypass/smuggling.rs:298-300`)
- Config Scope.validate(): Implemented (`config/scope.rs`)
- Scanner progress bar: catch_unwind wrapper (`scanner/ports/mod.rs:590-593`)
- Distributed TaskResult collection: Implemented (`distributed/remote.rs`)
- Distributed heartbeat: Cached DNS (`distributed/remote.rs:600-610`)
- Pipeline checkpoint_error: Implemented (`pipeline/executor.rs:224-231`)
- Pipeline run_fingerprint: Uses EXTENDED_SCAN_PORTS (`executor.rs:325-326`)
- Pipeline run_waf: Uses self.common for TLS (`executor.rs:538`)
- TUI TabDispatcher caching: Reduced 4 calls to 1 (`tui/app/mod.rs:371-382`)
- CLI -o/--output flag: Added to all output commands (`cli/misc.rs`)
- Documentation module counts: Updated (`architecture/recon.md`)
- Fuzzer progress bar: Added to run_sequential_with_session (`fuzzer/engine/execution.rs:236-252`)
- Output streaming CSV: Async BufWriter (`output/csv.rs:34-69`)
- TUI SessionManager theme restore: Implemented (`tui/session.rs:147`)
- Recon secrets module: Integrated into pipeline (`recon/mod.rs:364, runner.rs:443-461`)

---

## Verified as Already Correct (DO NOT IMPLEMENT)

These items were flagged but verified to be already correct in the codebase:

| Item | Evidence |
|------|----------|
| Loadtest error list cap 1000 | `metrics.rs:101,109` uses 1000 |
| Cloud parallelization | `cloud/mod.rs:66` uses `tokio::join!` |
| Distributed queue race condition | `queue.rs:78-79` acquires both locks upfront |
| Fuzzer LazyLock per-type | `payloads/mod.rs:140-150` uses LazyLock correctly |
| WAF HEADER_VALUE_MAX_LEN | `waf/detector/detect.rs:10` defines at module level |
| Config private IP check | `scope.rs:226,280` uses `is_private_ip()` |
| Fuzzer rate < 1 | `execution.rs:267` uses `rate < 1` |
| NSE library count 164 | Correct - documentation was accurate |
| NSE OSV/CISA KEV integration | Already implemented in `slapper-nse/src/cve/` |
| NSE DNS rebinding bypass | Architecture validates IPs at connection time |

---

## Deferred Items (Pruned After Review)

| Item | Recommendation | Reason |
|------|---------------|--------|
| **16. TUI unwrap_or_default** | PRUNED | Low value, high refactoring risk, already async-safe per plan |
| **22. NSE DNS rebinding** | PRUNED | Architecture already validates IPs; no clear exploit scenario |
| **23. OSV/CISA KEV integration** | PRUNED | Already fully implemented in `slapper-nse/src/cve/` |

---

## Future Considerations

Lower priority items identified during architecture reviews:

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

*Plan consolidated: 2026-05-23*
*Final update: 2026-05-29 - All items addressed, completed items pruned*
*Source reviews: ai_agents, cli_commands, config, distributed, fuzzer, loadtest, networking, output, overview, pipeline, recon, scanner, tui, waf, plugins_nse*