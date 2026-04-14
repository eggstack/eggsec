# Slapper Improvement Plan

This document tracks deferred and remaining work items. All completed items have been removed.

---

## Wave 4: Long-term (Deferred / Future Work)

### 4.1 Error Type Consolidation

**Status**: DEFERRED (per AGENTS.md policy)

**Severity**: LOW  
**Impact**: Maintainability

**Issue**: Three error types create friction:
- `SlapperError` (main crate, thiserror)
- `ConfigError` (config module)
- `anyhow::Result` (commands, TUI)

**Recommendation**: Standardize on `crate::error::Result` throughout

**Note**: This item is intentionally deferred. The current separation of error types (critical errors via `SlapperError`, recoverable/config errors via `ConfigError`, and exploratory code via `anyhow::Result`) serves different purposes. Consolidation was deemed counterproductive for this codebase.

**Estimated**: 8-12 hours

---

## Previously Completed

All other items from the original plan have been implemented:

- **Wave 1 (Critical)**: slapper-nse clippy fix, TLS NoVerifier flag, URL encoding, UTF-8 cursor fix, log injection mitigation
- **Wave 2 (High Priority)**: TUI dispatch refactor, Ruby unwrap fixes, slapper-ruby error handling, HttpSession serialization, grammar severity, AI logging, error audit
- **Wave 3 (Medium)**: XXE documentation, slapper-ruby dead code removal, duplicate ruby.rs removal, BaseFuzzConfig refactor, plugin tests
- **Wave 4 partial**: Dependency management (workspace deps), test coverage gaps, payload tagging standardization, documentation

See `AGENTS.md` for implementation history and lessons learned.

---

## Notes

- Run `cargo test --lib -p slapper` and `cargo clippy --lib -p slapper` to verify any changes
- Test count: 1155 passing
