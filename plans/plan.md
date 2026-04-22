# Slapper Improvement Plan (Historical)

**Date**: 2026-04-21
**Status**: COMPLETED (deferred items remain)
**Last Updated**: 2026-04-22 (H2.4 symlink cycle detection fixed)

---

## Overview

All planned improvement work (Waves A-N) has been completed and verified. Two items remain as known limitations documented below.

### Implementation Summary

| Wave | Track | Status |
|------|-------|--------|
| A-G | Previous Waves | ✅ COMPLETED |
| H | Security Foundations | ✅ COMPLETED |
| I | Code Quality | ✅ COMPLETED |
| J | Performance | ✅ COMPLETED |
| K | Plugin System | ✅ COMPLETED |
| L | AI Agent Testing | ✅ COMPLETED |
| M | Pentesting Tools | ✅ COMPLETED |
| N | TUI & Attack Patterns | ✅ COMPLETED |

**Total Tests**: 1104 passing
**Clippy Warnings**: 1 (pre-existing, scan_ports 8 args)

---

## Known Limitations

The following items have known limitations that are documented but not fully resolved:

### K2.3: rt.block_on Deadlock Risk (Ruby API)

**File**: `crates/slapper-ruby/src/api.rs`

**Issue**: 35 instances of `get_runtime().block_on` used in synchronous Ruby functions calling async code. This creates a deadlock risk since the Ruby VM thread may already hold resources the async block needs.

**Status**: Known architectural issue - requires significant refactoring to resolve properly (moving to fully async API or using `spawn` instead of `block_on`).

**Impact**: Deadlock risk exists but hasn't caused issues in practice. Monitor for related panics in production use.

### H3.2: NSE Socket Library Not Sandboxed

**Status**: ✅ **DOCUMENTED** (2026-04-22)

The `socket` library is **NOT sandboxed** even when `nse-sandbox` is enabled. Scripts can still make arbitrary network connections. This is documented in `docs/NSE_SCRIPTS.md` and `slapper_skills/nse_sandbox.md`.

The `lfs` library IS sandboxed with path restrictions.

---

## Current Codebase Metrics

| Metric | Value | Note |
|--------|-------|------|
| Tests | 1104 passing | After post-verification fixes |
| Clippy | 1 warning | Pre-existing (scan_ports 8 args) |
| Source files | 430+ | |
| Payload types | 38 | fuzzer/payloads |
| Dependencies | Updated | pyo3 0.28, magnus 0.8.2, mlua 0.11.6 |
| New modules | 6+ | ssh_auth, ftp_auth, smtp_auth, wireless, ssl_audit, containers, templates, intercept |
| Skill files | 28 | In `slapper_skills/` |
| ADRs | 5 | In `docs/adr/` |

---

## For Future Agents

When starting new improvement work:

1. Run `cargo test --lib -p slapper` to verify baseline
2. Run `cargo clippy --lib -p slapper` to check warnings
3. Create a new plan file for new work (don't modify this one)
4. Update AGENTS.md with any new patterns discovered
5. Always verify plan items against actual codebase before assuming they still apply
6. Use `rg` to confirm file paths, line numbers, and patterns exist

---

## Historical Context

This plan consolidates all planned improvement work for Slapper. Original plan files are preserved in git history.

All Waves A-N have been executed and verified.
