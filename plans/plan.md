# Slapper Improvement Plan

This document consolidates code quality improvements and security hardening tasks for the slapper security testing toolkit.

## Overview

| Crate | Source Files | Tests | Clippy Status |
|-------|-------------|-------|---------------|
| `slapper` (main) | 406 | ~976 | ✅ 0 warnings |
| `slapper-nse` | ~170+ | - | ❌ 46 errors |
| `slapper-ruby` | 9 | - | ⚠️ 2 warnings |
| `slapper-plugin` | 3 | - | ✅ Clean |

---

## Wave 1: Critical Security & Build Issues

Items that block clean builds or have security impact. These should be done first.

### 1.1 slapper-nse: Clippy Errors (46 errors)

**Severity**: CRITICAL  
**Impact**: Prevents clean builds

**Current State**: Last verified 2026-04-09 - 46 clippy errors (43x unused_io_amount, 2x redundant_comparisons, 534 warnings)

**Fix Strategy**:
1. Run `cargo clippy --fix -p slapper-nse --allow-dirty` to auto-fix
2. Manually fix remaining issues

**Files**: `slapper-nse/src/libraries/*.rs`, `slapper-nse/src/brute.rs`  
**Estimated**: 2-4 hours

---

### 1.2 TLS Security: NoVerifier

**Severity**: CRITICAL  
**Security Impact**: Man-in-the-middle attacks possible

**Location**: `crates/slapper/src/distributed/io.rs:147-220`

**Issue**: TLS verification is completely disabled via `NoVerifier`. Documented as "for internal use" but poses security risk.

**Recommended Fix**: Add feature flag `insecure-tls` that must be explicitly enabled + runtime warning

**Files**: `crates/slapper/src/distributed/io.rs`  
**Estimated**: 2 hours

---

### 1.3 URL-Embedded API Keys

**Severity**: HIGH  
**Security Impact**: API keys with special characters could cause parsing errors, information disclosure in server logs

**Locations**:

| File | Line | Pattern |
|------|------|---------|
| `recon/wayback.rs` | 63-64 | `&api_key={}` directly in URL |
| `recon/threatintel.rs` | 275 | `?key={}` in Shodan URL |
| `recon/geolocation.rs` | 426 | `?key={}` in ipapi URL |

**Fix**: Use `urlencoding` crate for proper encoding

**Files**: `recon/wayback.rs`, `recon/threatintel.rs`, `recon/geolocation.rs`  
**Estimated**: 1-2 hours

---

### 1.4 TUI: UTF-8 Cursor Position Bug

**Severity**: HIGH  
**Bug Status**: Confirmed - causes panic on non-ASCII input

**Location**: `crates/slapper/src/tui/components/input.rs:37,69,125`

**Issue**: `self.cursor_pos = value.len()` uses byte length instead of character count. Present in `set_value()`, `backspace()`, and `delete()` methods.

**Fix**: Change to `value.chars().count()`

**Files**: `tui/components/input.rs`  
**Estimated**: 30 minutes

---

### 1.5 Log Injection Mitigation

**Severity**: HIGH  
**Security Impact**: URLs logged without sanitization could inject control characters

**Locations** (9 logging calls with user-controlled input):
- `fuzzer/engine/core.rs:188`
- `recon/runner.rs:405`
- `scanner/endpoints.rs:357`
- `scanner/ports/mod.rs:98,138`
- `waf/mod.rs:185`
- `pipeline/mod.rs:77`
- `stress/warning.rs:30`

**Fix**:
1. Create `utils/logging.rs` with `sanitize_for_logging()`:
   - Strip ANSI escape sequences (CSI codes)
   - Remove control characters (0x00-0x1F except \n, \r, \t)
   - Truncate to configurable max length (default: 500 chars)
2. Update all 9 logging locations to use sanitization

**Files**: New `utils/logging.rs`, update existing files  
**Estimated**: 4-6 hours

---

## Wave 2: High Priority Issues

Items that impact maintainability, correctness, or have potential runtime failures.

### 2.1 TUI: Massive App Struct with Duplicated Dispatch

**Severity**: HIGH  
**Impact**: Maintainability - 1,975 line file with 20+ nearly identical dispatch methods

**Location**: `crates/slapper/src/tui/app/mod.rs`

**Current Problem**: Each handler method contains identical 29-arm match statement. Adding a new tab requires updating 20+ methods.

**Recommended Fix**: Create a `TabDispatcher` struct that holds a `Box<dyn TabState>` and delegates all methods dynamically.

**Note**: The dead `tui/app/dispatch.rs` (3-line comment) can be removed as part of this work.

**Files**: Create `tui/app/dispatch.rs`, modify `mod.rs`  
**Estimated**: 8-12 hours

---

### 2.2 slapper-plugin: Ruby .unwrap() Panics

**Severity**: HIGH  
**Impact**: Application panics if Ruby VM not initialized

**Location**: `crates/slapper-plugin/src/ruby.rs:51,77,94`

**Issue**: `Ruby::get().unwrap()` panics instead of returning error

**Fix**: Replace with proper error handling (or remove module - see 3.5)

**Files**: `slapper-plugin/src/ruby.rs`  
**Estimated**: 1 hour (or 30 min if removing)

---

### 2.3 slapper-ruby: Ignored Errors

**Severity**: HIGH  
**Impact**: Silent failures difficult to debug

**Locations**:
| File | Lines | Issue |
|------|-------|-------|
| `api.rs` | 732-866 | `let _ = hash.aset(...)` ignores Result |
| `bridge.rs` | 55,62 | `let _ = resp.send(...)` ignores send failures |

**Fix**: Add proper error handling or logging

**Files**: `slapper-ruby/src/api.rs`, `slapper-ruby/src/bridge.rs`  
**Estimated**: 2-3 hours

---

### 2.4 Fuzzer: HttpSession Serialization Bug

**Severity**: HIGH  
**Bug**: Session persistence broken - still present in codebase

**Location**: `crates/slapper/src/fuzzer/state.rs:16-17`

**Issue**: `headers` field marked with `#[serde(skip)]` - always empty after deserialization

**Fix Options**:
- A: Remove `headers` field if unused
- B: Implement custom serialization for HeaderMap
- C: Store headers as Vec<(String, String)> with custom serde

**Recommended**: Option B - custom Serialize/Deserialize for HeaderMap

**Files**: `fuzzer/state.rs`  
**Estimated**: 2 hours

---

### 2.5 Fuzzer: Grammar Payload Severity Hardcoded

**Severity**: MEDIUM  
**Impact**: Inaccurate severity ratings

**Location**: `crates/slapper/src/fuzzer/engine/core.rs:234`

**Issue**: Grammar payloads always use `Severity::Medium` regardless of grammar type (JWT, SSTI, XXE, etc.)

**Fix**: Map grammar type to appropriate severity (JWT=High, SSTI=Critical, XXE=High, etc.)

**Files**: `fuzzer/engine/core.rs`  
**Estimated**: 1 hour

---

### 2.6 Fuzzer: AI Generator Silent Failure

**Severity**: MEDIUM  
**Impact**: Poor UX - users don't know AI generation failed

**Location**: `crates/slapper/src/fuzzer/engine/core.rs:244-252`

**Issue**: AI generation errors are silently swallowed

**Fix**: Add `tracing::warn!` for failures

**Files**: `fuzzer/engine/core.rs`  
**Estimated**: 30 minutes

---

### 2.7 Error Handling Audit

**Severity**: MEDIUM  
**Impact**: 963+ unwraps could cause panics

**High-risk areas**:
- Config loading (`config/loader.rs`)
- Session restore (`pipeline/session.rs`)
- API request handling (`tool/protocol/rest.rs`)

**Fix**: Replace `.unwrap()` with `?` or match statements with proper error context

**Files**: Multiple  
**Estimated**: 8-10 hours

---

## Wave 3: Medium Priority

Items that improve code quality and fix known bugs.

### 3.1 XXE Defense Documentation

**Severity**: LOW  
**Impact**: Documentation improvement

**Task**: Document XML generation safety in output modules:
- `output/junit.rs` - explain quick_xml Writer is write-only, no parsing
- `output/sarif.rs` - same
- `fuzzer/payloads/xxe.rs` - clarify payloads are for testing only

**Estimated**: 1-2 hours

---

### 3.2 slapper-ruby: Dead Code

**Severity**: LOW  
**Impact**: Unused code clutters codebase

**Locations**: 
- `api.rs:577` - Static `MSF_CLIENT` never used
- `api.rs:580` - Struct `MsfClientState` never constructed

**Fix**: Remove unused code or add `#[allow(dead_code)]`

**Files**: `slapper-ruby/src/api.rs`  
**Estimated**: 15 minutes

---

### 3.3 Duplicate Ruby Code

**Severity**: LOW  
**Impact**: Confusion - two Ruby implementations exist

**Issue**: 
- `slapper-plugin/src/ruby.rs` - module exists, has panic-prone `.unwrap()` calls
- `slapper-ruby/` - full-featured crate with thread-safe message-passing

**Fix**: Remove `slapper-plugin/src/ruby.rs` since slapper-ruby is the production implementation

**Files**: Remove `crates/slapper-plugin/src/ruby.rs`  
**Estimated**: 30 minutes

---

### 3.4 Commands: 8-Parameter Function

**Severity**: LOW  
**Style**: Code smell

**Location**: `crates/slapper/src/commands/fuzz_convert.rs:4`

**Issue**: `base_fuzz_args` has 8 parameters

**Fix**: Use config struct or builder pattern

**Files**: `commands/fuzz_convert.rs`  
**Estimated**: 1-2 hours

---

### 3.5 slapper-plugin: No Unit Tests

**Severity**: MEDIUM  
**Impact**: No regression protection

**Fix**: Add tests for:
- Plugin trait implementations
- PluginRegistry discovery
- Error handling in PluginManager

**Files**: `slapper-plugin/src/lib.rs`  
**Estimated**: 4-6 hours

---

## Wave 4: Long-term Improvements

Lower priority items that improve maintainability and security posture.

### 4.1 Error Type Consolidation

**Severity**: LOW  
**Impact**: Maintainability

**Issue**: Three error types create friction:
- `SlapperError` (main crate, thiserror)
- `ConfigError` (config module)
- `anyhow::Result` (commands, TUI)

**Recommendation**: Standardize on `crate::error::Result` throughout

**Estimated**: 8-12 hours

---

### 4.2 Dependency Management

**Severity**: LOW  
**Impact**: Security and stability

**Recommendations**:
1. Add workspace-level pinned deps in `[workspace.dependencies]`
2. Add `cargo-audit` to CI pipeline
3. Consider `cargo-deny` for dependency governance

**Estimated**: 2-3 hours

---

### 4.3 Test Coverage Gaps

**Severity**: MEDIUM  
**Impact**: Regression protection

**Missing Test Coverage**:
- Circuit breaker (`utils/circuit_breaker.rs`)
- TUI state management
- Scope validation edge cases
- WAF bypass strategies

**Estimated**: 8-16 hours

---

### 4.4 Payload Tagging Standardization

**Severity**: LOW  
**Impact**: Code consistency

**Issue**: `sqli.rs` and `xss.rs` have post-processing loops for dynamic tagging, but other modules don't follow this pattern

**Recommendation**: Standardize or remove dynamic tagging

**Estimated**: 2-3 hours

---

### 4.5 Documentation & Configuration

**Severity**: LOW  
**Impact**: User guidance

**Tasks**:
1. Document `--insecure` flag usage and risks in `utils/http.rs`
2. Create `docs/security.md` with security-relevant config options
3. Update AGENTS.md with security findings

**Estimated**: 2-4 hours

---

## Implementation Order

### Phase 1: Critical (Wave 1) - Do First
- [ ] 1.1 Fix slapper-nse clippy errors
- [ ] 1.2 Add TLS NoVerifier feature flag
- [ ] 1.3 Add URL encoding for API keys
- [ ] 1.4 Fix TUI UTF-8 cursor bug
- [ ] 1.5 Log injection mitigation

### Phase 2: High Priority (Wave 2)
- [ ] 2.1 Refactor TUI dispatch (and remove dead dispatch.rs)
- [ ] 2.2 Fix slapper-plugin Ruby .unwrap() panics (or remove module)
- [ ] 2.3 Fix slapper-ruby ignored errors
- [ ] 2.4 Fix fuzzer HttpSession serialization
- [ ] 2.5 Fix grammar payload severity
- [ ] 2.6 Add AI generation error logging
- [ ] 2.7 Error handling audit

### Phase 3: Medium Priority (Wave 3)
- [ ] 3.1 Document XXE safety
- [ ] 3.2 Clean up dead code in slapper-ruby
- [ ] 3.3 Remove duplicate ruby.rs (slapper-plugin)
- [ ] 3.4 Refactor 8-parameter function
- [ ] 3.5 Add plugin tests

### Phase 4: Long-term (Wave 4)
- [ ] 4.1 Consolidate error types
- [ ] 4.2 Improve dependency management
- [ ] 4.3 Fill test coverage gaps
- [ ] 4.4 Standardize payload tagging
- [ ] 4.5 Documentation

---

## Parallelization Notes

### Blocks That Can Run in Parallel (within each wave)

**Wave 1** - All independent:
- 1.1 (slapper-nse)
- 1.2 (TLS)
- 1.3 (URL encoding)
- 1.4 (UTF-8 bug)
- 1.5 (Log injection)

**Wave 2** - All independent (can start after Wave 1):
- 2.1 (TUI dispatch)
- 2.2 (Ruby unwrap)
- 2.3 (slapper-ruby errors)
- 2.4 (Session serialization)
- 2.5 (Grammar severity)
- 2.6 (AI logging)
- 2.7 (Error handling)

**Wave 3 & 4** - All independent, can run in parallel once earlier waves complete

---

## Notes

- Items marked as "already done" in AGENTS.md (grammar fuzzer labeling, TabState traits, PortScanResults u32, ResponseSeverity Ord, ConfigError) are verified as implemented
- The main `slapper` crate is in good shape (0 clippy warnings)
- slapper-nse has the most technical debt due to its large library collection
- All changes require `cargo test` and `cargo clippy` verification