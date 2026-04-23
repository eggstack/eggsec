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

#### 1.9: NSE Socket Library Network Restrictions (DEFERRED)

**File**: `crates/slapper-nse/src/libraries/socket.rs:244-517`

**Status**: Known limitation - Documented in `docs/NSE_SCRIPTS.md` and `slapper_skills/nse_sandbox.md`.

**Problem**: Socket library allows connections to ANY host even when `nse-sandbox` enabled.

**Note**: The `socket` library is NOT sandboxed even when `nse-sandbox` is enabled. Scripts can make arbitrary network connections. The `lfs` library IS sandboxed with path restrictions.

---

### Wave 2: Core Feature Improvements

#### 2.1: REST API WebSocket Support (NOT IMPLEMENTED)

**File**: `tool/protocol/mcp/streaming.rs`, `tool/protocol/mcp/routes.rs`

**Problem**: MCP uses SSE for streaming. WebSocket would provide lower latency but would require:
1. Adding `tokio-tungstenite` dependency
2. Implementing WebSocket handler without breaking existing SSE
3. Adding `/mcp/ws` route

**Status**: Not implemented - SSE works adequately for most use cases.

---

#### 2.2: REST API Rate Limiting Improvements (STUB)

**File**: `tool/ratelimit.rs`

**Problem**: `RateLimitConfig` has `requests_per_minute`, `concurrent_scans`, `burst_size` but NOT `per_endpoint` and `global_limit` fields as specified.

**Status**: Partial implementation - basic rate limiting works but not configurable per-endpoint.

---

#### 2.3: REST API TLS Configuration (STUB)

**File**: `tool/protocol/rest.rs`

**Problem**: `cli/misc.rs` has `tls_cert` and `tls_key` options but no centralized TLS configuration struct in `RestState`.

**Status**: Partial - CLI options exist but TLS configuration is not wired up.

---

### Wave 3: Code Quality & Polish

#### 3.6: Improve ReCon Secrets Regex Error Handling (DEFERRED)

**File**: `crates/slapper/src/recon/secrets.rs:110-302`

**Problem**: 20+ `.expect()` calls on precompiled regex patterns. These are compile-time validated patterns, so the `.expect()` is technically safe but could be improved.

**Status**: Deferred - working as designed (compile-time validated).

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
