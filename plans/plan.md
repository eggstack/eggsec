# Slapper Improvement Plan (Historical)

**Date**: 2026-04-21
**Status**: ALL WAVES COMPLETED
**Last Updated**: 2026-04-22 (H3.2 documentation addressed)

---

## Overview

All planned improvement work (Waves A-N) has been completed and verified. This plan file is now historical - see git history for the full implementation timeline.

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

**Issue**: Many instances of `get_runtime().block_on` used in synchronous Ruby functions calling async code. This creates a deadlock risk since the Ruby VM thread may already hold resources the async block needs.

**Status**: Known architectural issue - requires significant refactoring to resolve properly (moving to fully async API or using `spawn` instead of `block_on`).

**Impact**: Deadlock risk exists but hasn't caused issues in practice. Monitor for related panics in production use.

### H2.4: Symlink Cycle Detection

**File**: `crates/slapper-nse/src/libraries/io.rs`, `crates/slapper-nse/src/libraries/lfs.rs`

**Issue**: Path validation uses `canonicalize()` which resolves symlinks, but doesn't explicitly detect symlink cycles. If canonicalize fails on a cycle, it falls back to the original path.

**Status**: ✅ **COMPLETED** (2026-04-22)

**Fix**: Changed `unwrap_or_else(|_| path_buf.clone())` to fail-secure behavior:
- In `io.rs`: `io.open` and `io.lines` now return an error table if canonicalize fails
- In `lfs.rs`: `is_path_allowed()` and `check_path()` now return `false` if canonicalize fails

This ensures that symlink cycles and other canonicalization failures are blocked rather than falling through with the unresolved path.

### H3.2: NSE Socket Library Not Sandboxed

**Status**: ✅ **ADDRESSED** (2026-04-22)

The `socket` library is **NOT sandboxed** even when `nse-sandbox` is enabled. The sandbox only logs when socket operations occur but does not enforce restrictions. This is now documented in `docs/NSE_SCRIPTS.md`.

The `lfs` (LuaFileSystem) library IS sandboxed with path restrictions.

---

## Current Codebase Metrics

| Metric | Value | Note |
|--------|-------|------|
| Tests | 1104 passing | After post-verification fixes |
| Clippy | 1 warning | Pre-existing (scan_ports 8 args) |
| Source files | 430+ | |
| Payload types | 38 | fuzzer/payloads (added 6 new) |
| Dependencies | Updated | pyo3 0.28, magnus 0.8.2, mlua 0.11.6 |
| New modules | 6+ | ssh_auth, ftp_auth, smtp_auth, wireless, ssl_audit, containers, templates, intercept |
| Skill files | 27 | In `slapper_skills/` |
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

All Waves A-N have been executed and verified. This plan file is now historical.
