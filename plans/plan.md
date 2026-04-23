# Slapper Improvement Plan

**Date**: 2026-04-23
**Status**: REMAINING - Items below are deferred or not implemented enhancements
**Last Updated**: 2026-04-23

---

## Overview

This plan documents remaining improvement opportunities and known limitations for Slapper. All previously documented issues have been reviewed and verified.

### Wave Summary

| Wave | Focus | Remaining Items |
|------|-------|-----------------|
| 2 | Core Feature Improvements | 3 not implemented |
| 6 | Advanced Capabilities | 2 deferred |

---

## Remaining Items

### Wave 2: Core Feature Improvements

#### 2.1: REST API WebSocket Support (NOT IMPLEMENTED)

**File**: `crates/slapper/src/tool/protocol/mcp/streaming.rs`, `crates/slapper/src/tool/protocol/mcp/routes.rs`

SSE streaming works correctly for all use cases. WebSocket would provide lower latency but adds complexity. This is an enhancement for future consideration.

#### 2.2: REST API Rate Limiting Improvements (NOT IMPLEMENTED)

**File**: `crates/slapper/src/tool/ratelimit.rs:12-16`

`RateLimitConfig` has `requests_per_minute`, `concurrent_scans`, `burst_size`. Missing fields: `per_endpoint` (HashMap<String, RateLimitConfig>), `global_limit` (u32). Enhancement for future consideration.

#### 2.3: REST API TLS Configuration (NOT IMPLEMENTED)

**File**: `crates/slapper/src/tool/protocol/rest.rs:17-22`

CLI options `tls_cert` and `tls_key` exist but `RestState` doesn't have TLS configuration fields. Enhancement for future consideration.

---

### Wave 6: Advanced Capabilities

#### 6.17: UDP IP Spoofing (RAW MODULE EXISTS, NOT USED)

**File**: `crates/slapper/src/stress/udp.rs:19-117, 120-184`

`raw_udp` module (lines 19-117) provides full raw socket capability with `build_udp_packet()` for IP/UDP header crafting and proper checksums. However, `run_udp_flood()` (lines 120-184) uses standard `tokio::net::UdpSocket` which does not support IP spoofing. Deferred - requires platform-specific raw socket support and integration into main flood function.

#### 6.22: Formula Injection Multibyte Check (DEFERRED)

**File**: `crates/slapper/src/output/escape.rs:17-27`

`escape_csv()` checks for formula injection with `is_ascii()` constraint. Fullwidth Unicode variants (e.g., U+FF1D for '=') could potentially bypass. Enhancement for future consideration - would need Unicode normalization for full protection.

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