# Code Quality Improvement Plan

## Relationship to Other Plans

This plan covers **code quality and consistency improvements** for the core library. See also:
- `plan.md` — Extension systems improvement plan (Python, Ruby, NSE plugins)

---

## Overview

This plan addresses minor code quality improvements identified during the codebase review. The codebase is production-quality; these are low-priority refinements for consistency and maintainability.

**Priority:** Low
**Estimated Effort:** 4-6 hours total (larger scope than initially estimated)
**Risk:** Low (internal refactors with no behavioral or API changes)

---

## Current State Assessment

| Metric | Value |
|--------|-------|
| Tests | 328 passing |
| Build | Clean compilation |
| Clippy | Zero warnings |
| anyhow::Result usages | 110 matches across ~55 files |
| SlapperError variants | 17 variants defined |

### anyhow Usage Distribution

| Category | Files | Notes |
|----------|-------|-------|
| Core library (waf, scanner, proxy, recon, fuzzer, loadtest) | ~20 | Migration candidates |
| Command handlers | ~10 | Acceptable (binary-facing) |
| TUI module | ~2 | Acceptable (UI code) |
| main.rs | 1 | Acceptable (binary entry) |
| Doc examples | ~10 | Should be updated to match |
| Test code | ~10 | Acceptable (testing only) |

---

## Recommendation 1: Unify Error Handling to Use `crate::error::Result<T>`

### Problem

Some modules use `anyhow::Result` while the library exports a custom `SlapperError` type via `crate::error::Result<T>`. This inconsistency means:
- Mixed error handling patterns across the codebase
- `anyhow::Error` doesn't provide structured error matching via `matches!()`
- Users of the library cannot pattern-match on specific error variants from modules using `anyhow`

### Scope Analysis

**Core library modules using `anyhow::Result` (migration candidates):**

| Module | Key Files | Approx. Usages |
|--------|-----------|-----------------|
| waf | `waf/detector.rs`, `waf/bypass/*.rs` | 4 |
| scanner | `scanner/ports/mod.rs`, `scanner/endpoints.rs`, `scanner/fingerprint.rs`, `scanner/udp_fingerprint.rs` | 5 |
| proxy | `proxy/mod.rs`, `proxy/pool.rs`, `proxy/health.rs` | 3 |
| recon | `recon/mod.rs`, `recon/dns_records.rs`, `recon/subdomain.rs`, `recon/ssl.rs`, `recon/cve.rs`, etc. | 12 |
| fuzzer | `fuzzer/mod.rs`, `fuzzer/engine/*.rs` | 7 |
| loadtest | `loadtest/mod.rs` | 1 |
| stress | `stress/*.rs` | 5 |
| pipeline | `pipeline/mod.rs`, `pipeline/executor.rs`, `pipeline/report.rs` | 4 |
| distributed | `distributed/mod.rs`, `distributed/worker.rs`, `distributed/remote.rs` | 6 |
| output | `output/report.rs` | 1 |

**Not migrating (acceptable anyhow usage):**
- `main.rs` — Binary entry point
- `commands/handlers/*.rs` — ~10 files (binary-facing)
- `tui/**/*.rs` — ~2 files
- `utils/privilege.rs`, `utils/output.rs`, `utils/scope.rs` — Utilities
- Test code and doc examples in non-migrated files

### Task 1.1: Add Missing Error Variants

**File:** `crates/slapper/src/error/mod.rs`

Current variants (17 total): Config, InvalidTarget, Network, RequestFailed, Timeout, RateLimited, ScanFailed, Payload, Output, **Plugin**, ScopeViolation, Io, HttpStatus, Http, Parse, Validation, AddressParse, Runtime, Cancelled

Add these new variants:

```rust
#[error("Proxy error: {0}")]
Proxy(String),

#[error("Fingerprint error: {0}")]
Fingerprint(String),

#[error("Reconnaissance error: {0}")]
Recon(String),

#[error("Load test error: {0}")]
LoadTest(String),
```

Note: `Plugin(String)` already exists at line 39 - no need to add.

Also add `From` impls for common external errors:
```rust
impl From<hickory_resolver::error::ResolveError> for SlapperError {
    fn from(e: hickory_resolver::error::ResolveError) -> Self {
        SlapperError::Network(format!("DNS resolution error: {}", e))
    }
}

impl From<reqwest::header::InvalidHeaderValue> for SlapperError {
    fn from(e: reqwest::header::InvalidHeaderValue) -> Self {
        SlapperError::Http(format!("Invalid header value: {}", e))
    }
}
```

### Task 1.2: Migrate Core Library Modules

**Scope:** Focus on core library modules where structured errors provide value to library users. Acceptable to keep `anyhow` in command handlers, TUI, and main.rs (binary-facing code).

**Migration priority (leaf → root):**

| Priority | Module | Files | Rationale |
|----------|--------|-------|-----------|
| 1 | waf | `waf/detector.rs`, `waf/bypass/*.rs` | Core detection logic |
| 2 | scanner | `scanner/ports/mod.rs`, `scanner/endpoints.rs`, etc. | Public API |
| 3 | proxy | `proxy/mod.rs`, `proxy/pool.rs`, `proxy/health.rs` | Public API |
| 4 | recon | `recon/*.rs` (8 files) | Public API |
| 5 | fuzzer | `fuzzer/mod.rs`, `fuzzer/engine/*.rs` | Public API |
| 6 | loadtest | `loadtest/mod.rs` | Public API |

**NOT migrating (acceptable anyhow usage):**
- `main.rs` — Binary entry point
- `commands/handlers/*.rs` — Command handlers (binary-facing)
- `tui/**/*.rs` — TUI code
- `utils/privilege.rs` — Utility functions
- Test code (`#[cfg(test)]`)

**For each module:**
1. Change import: `use anyhow::Result` → `use crate::error::Result`
2. Replace `anyhow::anyhow!()` / `anyhow::bail!()` with appropriate `SlapperError` variant
3. Preserve error messages
4. Run tests: `cargo test --lib -p slapper`

### Task 1.3: Update Documentation Examples

Update doc examples to use `crate::error::Result` instead of `anyhow::Result`:

**Files with doc examples using anyhow:**
- `fuzzer/mod.rs:39`
- `scanner/mod.rs:30,53`
- `waf/mod.rs:30,52`
- `recon/mod.rs:22`
- `loadtest/mod.rs:18`
- `pipeline/mod.rs:19`
- `distributed/mod.rs:20,32`
- `utils/mod.rs:20`

Example change:
```rust
// Before:
/// # async fn example() -> anyhow::Result<()> {

// After:
/// # async fn example() -> slapper::error::Result<()> {
```

### Task 1.4: Document anyhow Acceptance Policy

Add to `lib.rs` doc comment:

```rust
//! ## Error Handling
//!
//! Public API functions return `crate::error::Result<T>` with structured
//! `SlapperError` variants for pattern matching.
//!
//! `anyhow::Result` is used internally in:
//! - Command handlers (binary-facing)
//! - TUI code
//! - Binary entry point (`main.rs`)
//! - Test code
```

### Verification

```bash
# Confirm no anyhow::Result in lib public APIs
rg "pub.*anyhow::Result" crates/slapper/src/

# Run full test suite
cargo test --lib -p slapper
cargo clippy --lib -p slapper
```

---

## Recommendation 2: Split `waf/detector.rs` into Smaller Modules

### Problem

`waf/detector.rs` is 595 lines and contains multiple concerns:
- `WafDetectionResult` struct (lines 9-18)
- `WafSignatureLower` struct (lines 20-24)
- `WafDetector` struct with 4 async methods (lines 26-297)
- `ResponseDiff` struct with method (lines 299-329)
- 20+ unit tests (lines 331-595)

### Proposed Structure

```
waf/
├── mod.rs                  # Re-exports (update paths)
├── detector/
│   ├── mod.rs              # WafDetector struct, new(), re-exports
│   ├── detect.rs           # detect(), normalize_url()
│   ├── block_check.rs      # check_waf_block()
│   ├── compare.rs          # compare_responses(), ResponseDiff
│   └── types.rs            # WafDetectionResult, WafSignatureLower
├── waf_patterns.rs         # (existing, unchanged)
├── bypass/                 # (existing, unchanged)
└── stress.rs               # (existing, unchanged)
```

### Task 2.1: Create `waf/detector/` Directory Structure

```bash
mkdir -p crates/slapper/src/waf/detector
```

### Task 2.2: Extract Types to `detector/types.rs`

**Move from `detector.rs`:**
- `WafDetectionResult` struct (lines 9-18)
- `WafSignatureLower` struct (lines 20-24)
- `ResponseDiff` struct (lines 299-309)
- `impl ResponseDiff` block (lines 311-329)

### Task 2.3: Create `detector/mod.rs` - Core Struct

**Move from `detector.rs`:**
- `WafDetector` struct definition (lines 26-30)
- `impl WafDetector::new()` (lines 32-61)
- Re-export types: `pub use types::*;`
- Module declarations: `mod types; mod detect; mod block_check; mod compare;`

### Task 2.4: Create `detector/detect.rs` - Detection Logic

**Move from `detector.rs`:**
- `WafDetector::detect()` method (lines 63-189)
- `WafDetector::normalize_url()` method (lines 191-198)

### Task 2.5: Create `detector/block_check.rs`

**Move from `detector.rs`:**
- `WafDetector::check_waf_block()` method (lines 200-227)

### Task 2.6: Create `detector/compare.rs`

**Move from `detector.rs`:**
- `WafDetector::compare_responses()` method (lines 229-297)

### Task 2.7: Move Unit Tests

**Option A:** Place tests alongside their code in each submodule
**Option B:** Keep all tests in `detector/mod.rs` under `#[cfg(test)]`

**Recommended:** Option A for better organization

### Task 2.8: Update `waf/mod.rs` Exports

Update any re-export paths that changed.

### Verification

```bash
cargo test -p slapper --lib -- waf
cargo check -p slapper
cargo clippy --lib -p slapper
```

---

## Recommendation 3: Consolidate anyhow Usage (Superseded)

This recommendation has been consolidated into Recommendation 1 (Tasks 1.2-1.4) which covers:
- Migrating core library modules from anyhow to typed errors
- Documenting acceptable anyhow usage boundaries
- Updating doc examples

No separate work items needed.

---

## Implementation Order

| Order | Task | Risk | Files Changed |
|-------|------|------|---------------|
| 1 | Split `waf/detector.rs` | Low | 5-6 files |
| 2 | Add error variants to `SlapperError` | Low | 1 file |
| 3 | Migrate `waf` module errors | Low | 4-5 files |
| 4 | Migrate `scanner` module errors | Low | 4-5 files |
| 5 | Migrate `proxy` module errors | Low | 3 files |
| 6 | Migrate `recon` module errors | Low | 10-12 files |
| 7 | Migrate `fuzzer` module errors | Medium | 5-7 files |
| 8 | Migrate `loadtest` + `stress` errors | Low | 6 files |
| 9 | Migrate `pipeline` + `distributed` errors | Low | 5 files |
| 10 | Migrate `output` module errors | Low | 1 file |
| 11 | Update doc examples | Low | 8-10 files |
| 12 | Document anyhow policy in lib.rs | Low | 1 file |
| 13 | Final verification | - | - |

**Total estimated changes:** ~55-60 files

---

## Verification Commands

After each module migration:

```bash
# Check compilation
cargo check --lib -p slapper

# Run library tests
cargo test --lib -p slapper

# Lint
cargo clippy --lib -p slapper
```

After all tasks:

```bash
# Confirm anyhow usage reduced in core modules
rg "use anyhow" crates/slapper/src/{waf,scanner,proxy,recon,fuzzer,loadtest,stress,pipeline,distributed,output}/ | wc -l
# Target: <10 (vs ~55 currently)

# Confirm waf/detector split (each file < 200 lines)
wc -l crates/slapper/src/waf/detector/*.rs

# Full test suite
cargo test --lib -p slapper

# Lint with warnings as errors
cargo clippy --lib -p slapper -- -D warnings
```

---

## Success Criteria

| Criterion | Target |
|-----------|--------|
| `anyhow::Result` in core library modules | < 10 (from ~55) |
| `waf/detector.rs` max file length | < 200 lines |
| `SlapperError` variants | +4 (Proxy, Fingerprint, Recon, LoadTest) |
| New `From` impls | +2 (ResolveError, InvalidHeaderValue) |
| Doc examples using anyhow | Updated to use `crate::error::Result` |
| Clippy warnings | 0 |
| All 328+ existing tests | Passing |

---

## Rollback Plan

All changes are internal refactors with no API changes:
- No public struct/enum modifications
- No CLI argument changes
- No configuration format changes

If any change causes issues, revert the specific commit for that task.
