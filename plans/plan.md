# Slapper Improvement Plan

**Date**: 2026-04-23
**Status**: PARTIAL - Some items completed, remaining items are deferred or stubs
**Last Updated**: 2026-04-23

---

## Overview

This plan consolidates all planned improvement work for Slapper. Most items have been implemented. This document contains only the remaining incomplete or deferred items.

### Wave Summary

| Wave | Focus | Items | Completed | Remaining |
|------|-------|-------|-----------|-----------|
| 1 | Critical Security & API Fixes | 15 | 14 | 1 deferred |
| 2 | Core Feature Improvements | 22 | 19 | 3 stubs |
| 3 | Code Quality & Polish | 18 | 17 | 1 deferred |
| 4 | TUI Enhancements | 17 | 17 | 0 |
| 5 | Performance Optimizations | 15 | 12 | 3 stubs |
| 6 | Advanced Capabilities | 22 | 20 | 2 deferred |

---

## Remaining Items

### Wave 1: Critical Security & API Fixes

#### 1.9: NSE Socket Library Network Restrictions (PARTIALLY IMPLEMENTED)

**File**: `crates/slapper-nse/src/libraries/socket.rs:244-631`

**Status**: Conditional restrictions available via `allowed_networks` configuration.

**Behavior**:
- `allowed_networks` NOT configured → socket operations proceed (with warning log)
- `allowed_networks` configured → connections validated against CIDR blocklist

**Documentation**: Updated in `docs/NSE_SCRIPTS.md` and `slapper_skills/nse_sandbox.md` to accurately reflect this capability.

---

### Wave 2: Core Feature Improvements

#### 2.1: REST API WebSocket Support (NOT IMPLEMENTED)

**File**: `crates/slapper/src/tool/protocol/mcp/streaming.rs`, `crates/slapper/src/tool/protocol/mcp/routes.rs`

**Status**: VERIFIED - SSE implemented, WebSocket not implemented.

SSE streaming works correctly for all use cases. WebSocket would provide lower latency but adds complexity. This is an enhancement for future consideration.

---

#### 2.2: REST API Rate Limiting Improvements (NOT IMPLEMENTED)

**File**: `crates/slapper/src/tool/ratelimit.rs:12-16`

**Status**: VERIFIED - Basic rate limiting works but per-endpoint configuration not implemented.

`RateLimitConfig` has `requests_per_minute`, `concurrent_scans`, `burst_size`. Missing fields: `per_endpoint` (HashMap<String, RateLimitConfig>), `global_limit` (u32). Enhancement for future consideration.

---

#### 2.3: REST API TLS Configuration (NOT IMPLEMENTED)

**File**: `crates/slapper/src/tool/protocol/rest.rs:17-22`

**Status**: VERIFIED - TLS not wired up in RestState.

CLI options `tls_cert` and `tls_key` exist but `RestState` doesn't have TLS configuration fields. Enhancement for future consideration.

---

### Wave 3: Code Quality & Polish

#### 3.6: Improve ReCon Secrets Regex Error Handling (DEFERRED)

**File**: `crates/slapper/src/recon/secrets.rs:103-310`

**Status**: VERIFIED - Working as designed.

Single `.expect()` at line 309 wrapping entire `PATTERNS` initialization. All 28+ individual regex patterns use `.map_err()` for descriptive error messages. Patterns are hardcoded literals, so the `.expect()` is technically safe. Deferred - no improvement needed.

---

### Wave 5: Performance Optimizations

#### 5.3: Timing Analyzer Lock Contention (STUB)

**File**: `crates/slapper/src/fuzzer/engine/core.rs:92`

**Problem**: `TimingAnalyzer` uses `AtomicU64` counters (`total_requests`, `total_response_time`, etc.) but `Mutex` still needed for `samples: Vec<f64>` and `baseline_ms: Option<f64>`.

**Status**: Partial - atomic counters used where possible, mutex still needed for compound types.

---

#### 5.7: Arc<RwLock<HashMap>> → DashMap (NOT CONVERTED)

**Files**:
- `crates/slapper/src/agent/alerts/routing.rs:19`
- `crates/slapper/src/agent/alerts/mod.rs:46`
- `crates/slapper/src/utils/circuit_breaker.rs:126`
- `crates/slapper/src/tool/protocol/mcp/handlers.rs:27-28`

**Problem**: These modules still use `Arc<RwLock<HashMap>>` instead of `DashMap`.

**Status**: Not converted - conversion would require careful testing to ensure lock-free semantics are correct.

---

#### 5.11: std::thread::sleep in Async Context (STUB)

**File**: `crates/slapper/src/recon/mod.rs:153,260`

**Problem**: `std::thread::sleep` is used inside `std::thread::spawn` blocks (separate OS threads), NOT directly in async context.

**Status**: Technically correct - separate thread for spinner avoids blocking async runtime. Could be optimized but not harmful.

---

### Wave 6: Advanced Capabilities

#### 6.17: UDP IP Spoofing (DEFERRED)

**File**: `stress/udp.rs`

**Problem**: UDP flood uses standard `tokio::net::UdpSocket` without `IP_HDRINCL` for raw socket IP spoofing.

**Status**: Deferred - requires platform-specific raw socket support.

---

#### 6.22: Formula Injection Multibyte Check (DEFERRED)

**File**: `crates/slapper/src/output/escape.rs:17-27`

**Problem**: Formula injection check uses `starts_with` at character level but only handles ASCII. Fullwidth variants (U+FF1D) of formula characters could potentially bypass.

**Status**: Deferred - check exists but may not handle all Unicode edge cases.

---

## Known Limitations

### rt.block_on Deadlock Risk (Ruby API)

**File**: `crates/slapper-ruby/src/api.rs`

35 instances of `get_runtime().block_on` in synchronous Ruby functions calling async code. Requires significant refactoring.

---

## For Future Agents

When starting new improvement work:

1. Run `cargo test --lib -p slapper` to verify baseline
2. Run `cargo clippy --lib -p slapper` to check warnings
3. Add new items to this consolidated plan.md (don't create new plan files)
4. Update AGENTS.md with any new patterns discovered
5. Always verify plan items against actual codebase before assuming they still apply
6. Use `rg` to confirm file paths, line numbers, and patterns exist
7. Check test counts: `cargo test --lib -p slapper -- --list 2>/dev/null | wc -l`

---

## Historical Context

Original plan files consolidated into this document:
- plan.md — Original consolidated plan
- plan2.md — Code Quality Issues
- plan3.md — Security Issues
- plan4.md — Performance Issues
- plan5.md — CLI Interface
- plan6.md — TUI Improvements
- plan7.md — Agentic Capabilities

All items from Waves 1-6 were addressed during the 2026-04-23 implementation session. Remaining items are documented above as DEFERRED (known limitations) or STUB (partial implementation).
