# Consolidated Improvement Plan

Generated: 2026-04-02
Last verified: 2026-04-01

## Overview

This plan consolidates all improvement items from 6 separate plan files into a
single prioritized roadmap. Items are organized into **10 waves** by priority
and dependency order.

**Current State:** 363 tests passing, clean compilation, 117 clippy warnings
(no errors). Many items are fixed but several remain outstanding, primarily
in subsystems that were kept but not fully functionalized (distributed worker,
proxy chaining), architectural debt (Finding type duplication, dead code), and
async correctness (blocking DNS).

| Metric | Before | After |
|--------|--------|-------|
| Critical Bugs | ~10 | 2 remaining (1.1 dead code, 1.9 scope bypass) |
| High Bugs | ~8 | 2 remaining (3.1 distributed, 3.3 proxy chain) |
| Medium Issues | ~15 | ~7 remaining |
| Known Panics (UTF-8) | 4 | 0 |
| Library Tests | 350+ | 363 |
| Clippy Errors | many | 0 (117 warnings) |

---

## Completion Summary

### Wave 1: Critical Bugs (P0) - 9 of 11 FIXED

- 1.1: **NOT FIXED** — Duplicate `g` handler still at `tui/app/runner.rs:284-286`
  (unreachable dead code; first match at line 265 always wins in Normal mode)
- 1.2: Fixed `g` key in Insert mode (InputMode guards present)
- 1.3: Fixed mouse tab selection for all 22 tabs (`Tab::all().len()` replaces hardcoded 15)
- 1.4: Fixed concurrency override (`clamp(1, 500)` replaces `min(100)`)
- 1.5: Removed `default_value = "None"` on Option fields
- 1.6: Fixed verbose forwarding in WafStressArgs
- 1.7: Fixed fuzzer baseline header capture (headers stored in `ResponseSnapshot`)
- 1.8: Fixed MCP auth Bearer stripping (`strip_prefix("Bearer ")`)
- 1.9: **NOT FIXED** — Malformed URL fallback in `config/scope.rs:183-208` still
  extracts host via naive `split(':')` when `Url::parse()` fails
- 1.10: Fixed IPv6 parsing in cluster handler
- 1.11: Fixed XSS vulnerability in pipeline reports

### Wave 2: Panic-Prone Code (P0) - 5 of 5 FIXED

- 2.1: Fixed UTF-8 byte slicing in formatting functions (`.chars().take()` replaces byte indexing)
- 2.2: Fixed UTF-8 byte slicing in fuzzer mutator
- 2.3: Fixed UTF-8 byte slicing in secret preview
- 2.4: Fixed division by zero in client pool
- 2.5: Fixed panics in stealth utilities

### Wave 3: Non-Functional Subsystems (P0) - 3 of 5 ADDRESSED

- 3.1: **NOT FUNCTIONAL** — Distributed worker has registration, heartbeat, and
  task processing but no coordinator server exists in the codebase. Cannot
  operate end-to-end.
- 3.2: Fixed LineWriter buffered data
- 3.3: **NOT FIXED** — `chain_connect()` in `proxy/socks.rs:382-421` correctly
  chains SOCKS proxies but is **never called**. `create_chained_connection` in
  `proxy/mod.rs:146-201` creates independent connections through each proxy
  instead of chaining them.
- 3.4: Fixed spoofed scanner response matching
- 3.5: Kept WAF smuggling (not removed)

### Wave 4: Security & Memory Safety (P1) - 3 of 7 FIXED, 2 DOCUMENTED

- 4.1: **NOT FULLY FIXED** — `pending_cancellations` and `completed_results`
  HashMaps have on-demand removal only. No TTL-based cleanup, no background
  reaper task. Entries grow unboundedly if clients disconnect.
- 4.2: **NOT FIXED** — No rate limiter cleanup mechanism exists anywhere in the
  codebase. The `RateLimiter` in `handlers.rs` has no eviction or TTL.
- 4.3: Fixed SSE stream heartbeat logic (15s `KeepAlive` + `tick_interval`)
- 4.4: API key in params remains (documented, accepted risk)
- 4.5: TLS MITM remains (documented, accepted risk)
- 4.6: Fixed ProxyEntry enabled default (`#[serde(default = "default_true")]`)
- 4.7: **NOT FIXED** — Blocking `to_socket_addrs()` still called in async
  contexts: `tui/workers/network.rs:176`, `recon/dns_enhanced.rs` (multiple),
  `stress/udp.rs:94`, `stress/syn.rs:138`, `stress/icmp.rs:144`,
  `scanner/icmp_probe.rs:112`

### Wave 5: TUI Fixes (P1) - 4 of 9 ADDRESSED

- 5.1: Fixed mouse event double-read (single `handle_mouse_event` call)
- 5.2: **NOT FIXED** — Export formats do not call JSON generation
- 5.3: **MOSTLY FIXED** — One `eprintln!` remains in teardown error path
  (`tui/app/runner.rs:46`); TUI runtime itself is clean
- 5.4: **NOT FIXED** — Search does not replace history
- 5.5: Silent mutex lock (documented)
- 5.6: **NOT IMPLEMENTED** — No mode indicator in status bar
- 5.7: Fixed export save uses tracing
- 5.8: **NOT FIXED** — Default TabInput behavior unchanged
- 5.9: **NOT IMPLEMENTED** — No page up/down support

### Wave 6: Code Quality & Consistency (P2) - 13 of 14 FIXED

- 6.1: Implemented `FromStr` for `Severity` (trait impl + deprecated inherent method)
- 6.2: Fixed `CircuitBreakerRegistry::get_state` (returns actual state)
- 6.3: Fixed race condition in circuit breaker
- 6.4: Removed duplicate `ToolDispatcher` (single definition in `tool/dispatcher.rs`)
- 6.5: Removed duplicate `ToolResult` (type no longer exists)
- 6.6: Fixed JUnit XML attributes
- 6.7: Removed TCP from UDP probes
- 6.8: proxy_type now uses enum
- 6.9: Fixed duplicate `PortData`
- 6.10: Fixed `update_session_from_results`
- 6.11: Fixed `fingerprint_services` concurrency
- 6.12: Fixed bypass success criteria
- 6.13: Skipped (API change)
- 6.14: `SUPPORTED_WAF_COUNT` now validated (test asserts against `get_waf_signatures().len()`)

### Wave 7: Architectural Debt (P2) - 0 of 3 NON-SKIPPED FIXED

- 7.1: **NOT FIXED** — 10 separate `Finding` structs still exist across modules:
  `output/trend.rs`, `output/markdown.rs`, `tui/tabs/plugin.rs`, `waf/types.rs`,
  `tool/response.rs`, `generated/slapper.tool.v1.rs`, `output/convert.rs`,
  `output/csv.rs`, `notify/webhook.rs`, `output/agent.rs`
- 7.2: **PARTIALLY FIXED** — `Report` trait in `output/report.rs` provides a
  unifying abstraction, but 12 separate format files remain (html, junit, sarif,
  csv, markdown, etc.)
- 7.3: Skipped (major refactor — TUI tab dispatch)
- 7.4: **NOT FIXED** — 23 `#[allow(dead_code)]` remain (12 inline + 11
  module-level across `proxy/socks.rs`, `tui/tabs/packet.rs`, `recon/ssl.rs`,
  `recon/subdomain.rs`, `proxy/http_connect.rs`, `proxy/pool.rs`,
  `stress/authorization.rs`, `stress/warning.rs`, `recon/threatintel.rs`,
  `recon/wayback.rs`, `stress/metrics.rs`, `utils/rate_limiter.rs`)
- 7.5: Skipped (major refactor — config flattening)
- 7.6: Skipped (major refactor — error consolidation)

### Wave 8: CLI & TUI UX (P3) - 5 of 10 FIXED

- 8.3: Made `--json` flags discoverable (global flag + per-subcommand)
- 8.5: Added config validation (`config::settings::validate()` called at load)
- 8.6: Added tab-specific help content (`help_popup_for_tab()` + `get_help_for_tab()`)
- 8.8: Fixed command palette shortcuts
- 8.9: Reduced status bar duplication

### Wave 9: Test Coverage (P3) - ADDRESSED VIA EXISTING TESTS

- 9.1-9.8: Existing tests cover key functionality; no dedicated new test files added

### Wave 10: Cleanup & Documentation (P3) - 3 of 5 FIXED, 1 PARTIAL

- 10.1: **PARTIALLY FIXED** — Global clippy suppressions removed from `Cargo.toml`
  and `lib.rs`, but 7 file-level `#![allow(clippy::...)]` remain in
  `output/convert.rs` and 6 payload modules (`compression.rs`, `grpc.rs`,
  `websocket.rs`, `graphql.rs`, `oauth.rs`, `idor.rs`)
- 10.2: **NOT FIXED** — `tui/tabs/packet.rs` still has `#![allow(dead_code)]`,
  `tui/workers/recon.rs` still has `#[allow(unused_variables)]`
- 10.3: Documented output patterns in AGENTS.md
- 10.4: Extracted Spinner to `recon/spinner.rs`
- 10.5: Extracted print functions to `waf/output.rs`

---

## Remaining Work

### High Priority
- 1.1: Remove dead duplicate `g` handler in `tui/app/runner.rs:284-286`
- 1.9: Reject malformed URLs in `config/scope.rs` fallback path
- 3.3: Wire `chain_connect()` into `create_chained_connection()` or remove dead code
- 4.7: Replace blocking `to_socket_addrs()` with `tokio::net::lookup_host()` in async fns

### Medium Priority
- 4.1: Add TTL-based reaper for MCP `pending_cancellations`/`completed_results`
- 4.2: Add rate limiter cleanup/eviction
- 7.1: Consolidate 10 `Finding` structs into a single canonical type
- 7.4: Remove 23 `#[allow(dead_code)]` suppressions (fix underlying unused code)

### Low Priority
- 3.1: Implement coordinator server for distributed worker (or remove subsystem)
- 5.2/5.4/5.6/5.8/5.9: TUI UX improvements (export, search, mode indicator, page nav)
- 7.2: Unify report generators behind `Report` trait
- 10.1/10.2: Clean remaining clippy and dead_code suppressions

---

## Verification Commands

```bash
# Check compilation
cargo check --lib -p slapper

# Run library tests
cargo test --lib -p slapper

# Lint
cargo clippy --lib -p slapper
```

---

## Notes

- 363 tests pass, 0 failures
- Zero clippy errors (117 warnings remain)
- UTF-8 panic fixes verified — `.chars().take()` used in all truncation paths
- MCP auth works with `Authorization: Bearer <token>` headers
- `Severity` implements `FromStr` trait (inherent `from_str` deprecated)
- `SUPPORTED_WAF_COUNT` validated against actual signature count at test time
- Mouse clicks work on all 22 tabs via `Tab::all().len()`
- Circuit breaker `get_state` returns actual circuit state
- Proxy chaining code exists (`chain_connect`) but is never invoked
- Distributed worker is structurally complete but has no coordinator to connect to

(End of file)
