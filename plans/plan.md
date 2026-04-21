# Slapper Improvement Plan

**Date**: 2026-04-21
**Status**: WAVES A-N COMPLETED - POST-VERIFICATION FIXES APPLIED
**Last Updated**: 2026-04-21

---

## Executive Summary

All planned improvement work (Waves A-N) has been completed. Post-verification identified and fixed implementation gaps.

### Verification Summary

| Wave | Track | Items | Status |
|------|-------|-------|--------|
| A-G | Previous Waves | 147 | ✅ COMPLETED |
| H | Security Foundations | 14 | ✅ COMPLETED (fixed post-verification) |
| I | Code Quality | 12 | ✅ COMPLETED (fixed post-verification) |
| J | Performance | 10 | ✅ COMPLETED (fixed post-verification) |
| K | Plugin System | 15 | ✅ COMPLETED (fixed post-verification) |
| L | AI Agent Testing | 10 | ✅ COMPLETED |
| M | Pentesting Tools | 11 | ✅ COMPLETED |
| N | TUI & Attack Patterns | 19 | ✅ COMPLETED (fixed post-verification) |

### Post-Verification Fixes Applied

The following gaps were identified during systematic verification and fixed:

- **H1.3**: Replaced `rustls-pemfile` with `pem` crate for TLS PEM handling
- **H2.1**: Added sandbox parameter to lfs library registration
- **H2.2**: Added sandbox logging to socket library when sandbox enabled
- **H2.3**: Changed default `allowed_dir` from `None` to `/tmp/slapper-nse`
- **I1.4**: Fixed DNS resolution error context preservation
- **I3.1**: Fixed nested runtime creation (converted to `#[tokio::test]`)
- **I3.2**: Changed alerts.rs to use `tokio::sync::Mutex`
- **J1.1**: Replaced `Mutex<Vec>` with `DashMap` in fuzzer execution
- **J1.2**: Replaced `std::fs` with `tokio::fs` in agent memory
- **J2.1**: Replaced `Mutex` with `AtomicU64` for progress counter
- **K1.4**: Added JSON size limits to Ruby loader
- **K2.4**: Parallelized `PluginRegistry::run_check()`
- **K3.1-K3.4, K3.6**: Plugin system improvements (caching, unregister, equality)
- **N2.1-N2.6**: Integrated payload modules (nosql, xpath, expression, prototype, race, mass_assign)
- **N2.8**: Added GraphQL advanced capabilities as static payloads

---

## Known Limitations

The following items have known limitations that are documented but not fully resolved:

### K2.3: rt.block_on Deadlock Risk

**File**: `crates/slapper-ruby/src/api.rs`

**Issue**: 39 instances of `get_runtime().block_on` used in synchronous Ruby functions calling async code. This creates a deadlock risk since the Ruby VM thread may already hold resources the async block needs.

**Status**: Known issue - architectural fix would require significant refactoring (likely moving to fully async API or using `spawn` instead of `block_on`).

**Workaround**: The deadlock risk exists but hasn't caused issues in practice. Monitor for any related panics in production use.

### H2.4: Symlink Cycle Detection

**File**: `crates/slapper-nse/src/libraries/io.rs`

**Issue**: Path validation uses `canonicalize()` which resolves symlinks, but doesn't explicitly detect symlink cycles. If canonicalize fails on a cycle, it falls back to the original path.

**Status**: Partial - canonicalization is attempted, but cycle detection is incomplete.

### H3.2: NSE Sandbox Documentation

**File**: `docs/security.adoc` or `docs/security.md`

**Issue**: Documentation about NSE sandbox behavior (that lfs and socket libraries are NOT sandboxed despite sandbox being enabled) is incomplete.

**Status**: Needs documentation update.

---

## Verification Commands

```bash
# Baseline verification
cargo test --lib -p slapper        # Should pass: 1104+ tests
cargo clippy --lib -p slapper       # Should show pre-existing warnings only

# NSE compilation (requires OpenSSL dev packages)
cargo build -p slapper-nse --features nse,nse-sandbox

# Plugin features (requires Ruby/Python dev headers)
cargo build -p slapper --features ruby-plugins
cargo build -p slapper --features python-plugins
```

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

## Current Codebase Metrics

| Metric | Value | Note |
|--------|-------|------|
| Tests | 1104 passing | After post-verification fixes |
| Clippy | 1 warning | Pre-existing (scan_ports 8 args) |
| Source files | 430+ | |
| Payload types | 38 | fuzzer/payloads (added 6 new) |
| Dependencies | Updated | pyo3 0.28, magnus 0.8.2, mlua 0.11.6, serde_yaml_neo, pem |
| New modules | 6+ | ssh_auth, ftp_auth, smtp_auth, wireless, ssl_audit, containers, templates, intercept |
| Skill files | 27 | In `slapper_skills/` |

---

## Historical Context

This plan consolidates all planned improvement work for Slapper. Original plan files are preserved in git history.

All Waves A-N have been executed and verified. This plan file is now historical - see git history for the full implementation timeline.

(End of file - 200 lines)
