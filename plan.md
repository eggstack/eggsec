# Slapper Consolidated Improvement Plan

Consolidated from CODE_REVIEW_PLAN.md, plan2.md, plan3.md, plan4.md, and plan5.md on 2026-03-31.

## Current Status

| Metric | Value |
|--------|-------|
| Tests | 350 passing |
| Build | Clean compilation |
| Clippy | 1 warning (MSRV `is_multiple_of`, non-blocking) |
| Doctests | 14 pass, 1 ignored, 0 fail |
| `SlapperError` variants | 23 (Fingerprint added) |
| `waf/detector/` split | Complete (6 files, all <200 lines) |
| `anyhow::Result` in core | 0 (policy documented in lib.rs) |
| Doc examples using `anyhow::Result` | 0 |
| `once_cell` in slapper | 0 |
| `mcp-server` feature | Removed |
| MSRV | 1.80 |
| Large files (>1000 lines) | 1 file (tui/workers/runner.rs 1192) |

## Already Complete

These items from the source plans are confirmed done:

- `waf/detector.rs` split into `waf/detector/` directory (6 files, all <200 lines)
- `SlapperError` has 23 variants (Proxy, Recon, LoadTest, Fingerprint added)
- Core library modules migrated from `anyhow::Result` to `crate::error::Result`
- `lib.rs` documents anyhow usage policy (lines 39-48)
- Severity import paths in `fuzzer/engine/` use correct re-export path
- `unreachable!` in `fuzzer/chain.rs:148` already replaced with error return
- NSE `duration_since` unwraps already replaced with `unwrap_or_default()`
- Ruby plugins zero warnings with `--features ruby-plugins`
- prost/prost-build both at 0.13.5
- Config reloading uses `ctx.config` directly (no `load_config()` re-reads)
- Port scanner records open/closed/filtered states
- `InvalidHeaderValue` `From` impl added
- Doc examples use `slapper::error::Result` (0 using `anyhow::Result`)
- Unused `_config` parameters removed from `fuzzer::run_cli`, `fuzzer::run_waf_stress`, `waf::run_cli`
- Deprecated `mcp-server` feature removed
- `thiserror` upgraded to 2.x
- `once_cell` replaced with `std::sync::LazyLock` (17 files)
- MSRV set to 1.80 in workspace root + 4 crates
- `native-tls` migrated to `rustls` (distributed/io.rs, distributed/remote.rs, recon/ssl.rs)
- `tool/protocol/mcp.rs` (1710 lines) split into `tool/protocol/mcp/` (6 files, largest 890 lines)
- `tui/app.rs` (2193 lines) split into `tui/app/` (5 files, largest 1192 lines in runner.rs)
- `docs/FEATURES.md` updated with complete feature flag documentation
- `SlapperError` doc examples expanded with helper method usage
- CI workflow updated with plugin feature checks
- Feature flag integration test added (`tests/feature_tests.rs`)

---

## Phase 1: Critical Fixes (High Priority)

Independent tasks — run in parallel with 3 sub-agents.

### 1.1 Fix prost/prost-build Version Mismatch

**Problem:** `prost` 0.13 declared alongside `prost-build` 0.12. Lockfile resolves both versions, doubling compilation time and risking incompatibility.

**File:** `crates/slapper/Cargo.toml`

Upgrade prost-build from 0.12 to 0.13 (backward-compatible):
```toml
[build-dependencies.prost-build]
version = "0.13"
optional = true
```

If tonic 0.12 doesn't support prost 0.13, upgrade tonic to 0.13 as well.

**Verify:**
```bash
cargo update -p prost-build -p prost -p tonic
cargo check -p slapper --features grpc-api
cargo test -p slapper --features grpc-api
```

**Effort:** 15 min | **Risk:** Low

### 1.2 Fix Config Reloading in Command Handlers

**Problem:** `handle_report` (4 call sites) and `handle_proxy` (3 call sites via `stress.rs`) call `load_config(ctx.config_path())` to re-read config from disk, bypassing already-loaded `ctx.config`.

**Files:**
- `crates/slapper/src/commands/handlers/report.rs` (lines 82, 102, 126, 142)
- `crates/slapper/src/commands/handlers/stress.rs` (lines 71, 101, 127)

Replace each `load_config(ctx.config_path())?` with `&ctx.config` (read-only) or `ctx.config.clone()` (if mutation needed). Remove unused `load_config` imports.

**Verify:**
```bash
cargo check -p slapper --features full
cargo test -p slapper --features full
```

**Effort:** 20 min | **Risk:** Low

### 1.3 Fix Port Scanner Error Swallowing

**Problem:** `scanner/ports/mod.rs:301-317` — the async spawn block only records open ports (`Ok(Ok(_))`). Closed and filtered ports are silently dropped, making it impossible to distinguish "filtered" from "closed".

**File:** `crates/slapper/src/scanner/ports/mod.rs`

Add match arms for each outcome:
- `Ok(Ok(_))` → port open (already handled)
- `Ok(Err(_))` → connection refused → port closed
- `Err(_)` → timeout → port filtered

**Verify:**
```bash
cargo check -p slapper
cargo test -p slapper -- scanner
```

**Effort:** 30 min | **Risk:** Low

---

## Phase 2: Quick Fixes (Medium Priority)

All independent — run in parallel with sub-agents.

### 2.1 Add `Fingerprint` Error Variant

**Problem:** `scanner/fingerprint.rs` modules lack a dedicated error variant for categorization.

**File:** `crates/slapper/src/error/mod.rs`

Add after existing variants:
```rust
#[error("Fingerprint error: {0}")]
Fingerprint(String),
```

**Effort:** 5 min | **Risk:** None

### 2.2 Add `InvalidHeaderValue` From Implementation

**Problem:** No `From<reqwest::header::InvalidHeaderValue>` impl, requiring manual `.map_err()` at every call site.

**File:** `crates/slapper/src/error/mod.rs`

```rust
impl From<reqwest::header::InvalidHeaderValue> for SlapperError {
    fn from(e: reqwest::header::InvalidHeaderValue) -> Self {
        SlapperError::Http(format!("Invalid header value: {}", e))
    }
}
```

**Effort:** 5 min | **Risk:** None

### 2.3 Update Doc Examples (11 instances)

**Problem:** 11 doc examples in core library modules still use `anyhow::Result` instead of `slapper::error::Result`.

**Files (8 mod.rs files):**

| File | Lines |
|------|-------|
| `fuzzer/mod.rs` | 39 |
| `waf/mod.rs` | 30, 52 |
| `scanner/mod.rs` | 30, 53 |
| `recon/mod.rs` | 22 |
| `pipeline/mod.rs` | 19 |
| `loadtest/mod.rs` | 18 |
| `distributed/mod.rs` | 20, 32 |
| `utils/mod.rs` | 20 |

Change `anyhow::Result<()>` → `slapper::error::Result<()>` in each.

**Verify:** `cargo test --doc -p slapper`

**Effort:** 15 min | **Risk:** None

### 2.4 Remove Unused `_config` Parameters

**Problem:** Three `run_cli` functions accept `_config: &SlapperConfig` but never use it.

**Files:**
- `crates/slapper/src/fuzzer/mod.rs:134` — `run_cli(args, _config)`
- `crates/slapper/src/fuzzer/mod.rs:153` — `run_waf_stress(args, _config)`
- `crates/slapper/src/waf/mod.rs:102` — `run_cli(args, _config)`

**Callers to update (~10 sites):**
- `commands/handlers/fuzz.rs:6,11,16`
- `commands/fuzz_convert.rs:85,90`
- `pipeline/executor.rs:447`
- `distributed/worker.rs:401`
- `tool/implementations/fuzzer.rs:156`
- `tool/implementations/waf.rs:112,132,145`

**Verify:**
```bash
cargo check -p slapper --features full
cargo test -p slapper --features full
```

**Effort:** 15 min | **Risk:** Low (internal callers only)

### 2.5 Remove Deprecated `mcp-server` Feature

**Problem:** `mcp-server` feature is marked DEPRECATED but still present as alias for `rest-api`.

**File:** `crates/slapper/Cargo.toml`

Remove `mcp-server = ["rest-api"]` line. Update 4 `#[cfg(feature = "mcp-server")]` references:
- `cli/mod.rs:120`
- `commands/handlers/mod.rs:100`
- `commands/handlers/notify.rs:93`
- `cli/misc.rs:253`

**Effort:** 10 min | **Risk:** Low

---

## Phase 3: Dependency Updates (Medium Priority)

All independent except 3.2 depends on 3.3 (LazyLock requires MSRV 1.80+).

### 3.1 Upgrade `thiserror` to 2.x

**Problem:** `thiserror` 1.x used; 2.x has improved derive macros and faster compilation.

**File:** `crates/slapper/Cargo.toml`

Change `thiserror = "1"` → `thiserror = "2"`.

API is backward-compatible for the `#[derive(Error)]` patterns used here.

**Verify:**
```bash
cargo check -p slapper --features full
cargo test -p slapper --features full
cargo clippy --lib -p slapper
```

**Effort:** 10 min | **Risk:** Low

### 3.2 Replace `once_cell` with `std::sync::LazyLock`

**Problem:** `once_cell` used in 3 files in slapper + 14 files in slapper-nse. Since Rust 1.80, `std::sync::LazyLock` provides identical functionality.

**Depends on:** Phase 3.3 (MSRV must be 1.80+).

**slapper files (3):**
- `crates/slapper/src/fuzzer/detection/aho_corasick.rs`
- `crates/slapper/src/utils/service_detection.rs`
- `crates/slapper/src/recon/secrets.rs`

**slapper-nse files (14):** Libraries in `crates/slapper-nse/src/libraries/`

Replace `use once_cell::sync::Lazy` → `use std::sync::LazyLock` and `Lazy<T>` → `LazyLock<T>`. Remove `once_cell` from both Cargo.toml files.

**Verify:**
```bash
cargo check -p slapper --features full
cargo test -p slapper --features full
```

**Effort:** 20 min | **Risk:** None

### 3.3 Add MSRV to Workspace

**Problem:** No `rust-version` field in any Cargo.toml.

**File:** `Cargo.toml` (workspace root) + 4 crate Cargo.toml files

Add `rust-version = "1.80"` to `[workspace.package]` and propagate to each crate with `rust-version.workspace = true`.

**Effort:** 5 min | **Risk:** None

### 3.4 Investigate `native-tls` Necessity

**Problem:** `native-tls` 0.2.18 and `tokio-native-tls` 0.3 declared alongside `reqwest` with `rustls-tls`. Two TLS backends increase compile time and binary size.

Audit with `grep -rn "native_tls\|tokio_native_tls" crates/slapper/src/`. Remove direct declarations if only used transitively.

**Effort:** 10 min | **Risk:** Low

---

## Phase 4: CI and Documentation (Medium Priority)

All independent — run in parallel.

### 4.1 Add Plugin Feature Checks to CI

**Problem:** `python-plugins` and `ruby-plugins` never tested in CI. Breakage goes undetected.

**File:** `.github/workflows/test.yml`

Add `check-plugins` job with `cargo check -p slapper --features python-plugins` and `cargo check -p slapper --features ruby-plugins` (no runtime needed, compile-only).

**Effort:** 15 min | **Risk:** None

### 4.2 Create Feature Flag Documentation

**Problem:** 14 feature flags with complex dependencies not documented in one place.

Document in `docs/features.md` or `ARCHITECTURE.md`:
- Feature hierarchy and dependencies
- Which modules each feature enables
- Build time impact notes

**Effort:** 1 hour | **Risk:** None

---

## Phase 5: Large File Refactoring (Lower Priority)

Independent of each other — can parallelize.

### 5.1 Split `tui/app.rs` (2,193 lines)

**Structure:**
```
tui/
├── app/
│   ├── mod.rs          # App struct, new(), re-exports
│   ├── runner.rs       # run(), run_app() main loop
│   ├── error.rs        # make_friendly_error()
│   ├── input.rs        # InputMode enum, input handling
│   ├── events.rs       # Mouse/keyboard event handlers
│   └── options.rs      # GlobalHttpOptions struct
```

**Effort:** 2-3 hours | **Risk:** Medium (complex UI state management)

### 5.2 Split `tool/protocol/mcp.rs` (1,710 lines)

**Structure:**
```
tool/protocol/
├── mcp/
│   ├── mod.rs          # McpServer struct, public API
│   ├── auth.rs         # Authentication, API key validation
│   ├── handlers.rs     # Request/response handlers
│   ├── streaming.rs    # SSE streaming implementation
│   └── types.rs        # McpError, request/response types
```

**Effort:** 2 hours | **Risk:** Low (well-defined API boundaries)

---

## Phase 6: Testing Improvements (Medium Priority)

### 6.1 Add `error::Result` Doc Example

Add doc example in `error/mod.rs` demonstrating proper usage of `SlapperError` variants.

**Effort:** 15 min | **Risk:** None

### 6.2 Add Feature Flag Integration Test

Add `tests/feature_tests.rs` verifying feature flag interactions compile correctly.

**Effort:** 30 min | **Risk:** None

### 6.3 Expand Test Coverage

- Property-based tests for parsing modules (proptest)
- Expand negative tests in `tests/negative_tests.rs`
- Chaos testing: inject network failures, timeouts, malformed responses
- Increase coverage for `config/` and `utils/` to 80%

**Effort:** 3 days | **Risk:** Low

---

## Phase 7: Architecture Improvements (Future Work)

These are larger initiatives for future planning.

### 7.1 Scope Enforcement Audit
Audit all command handlers for scope checks before network activity. Move DNS resolution after scope validation. Add integration tests for scope bypass attempts.

**Priority:** High | **Effort:** 2 days

### 7.2 External API Circuit Breaker
Implement `CircuitBreaker` struct for NVD CVE lookups, geolocation, and threat intel APIs. Add to config and tracing metrics.

**Priority:** High | **Effort:** 2 days

### 7.3 Sensitive Data Logging Audit
Audit all logging for credential exposure. Add `SensitiveString::log_secret()` helper. Add `--redact-logs` runtime flag.

**Priority:** High | **Effort:** 1 day

### 7.4 Payload Lazy Loading
Refactor `fuzzer/payloads/` to use `once_cell::sync::Lazy` (or `LazyLock`). Add feature flags for specific payload categories. Implement streaming for large wordlists.

**Priority:** Medium | **Effort:** 2 days

### 7.5 Performance Optimizations
Review connection pool config, add connection reuse metrics, implement request batching for recon modules, profile memory usage.

**Priority:** Medium | **Effort:** 2 days

### 7.6 Truncation Function Cleanup
Rename `truncate` → `strip_controls` and `truncate_simple` → `preserve_all` for clarity. Currently 2 files import `truncate_simple` directly (`loadtest/metrics.rs:1`, `scanner/endpoints.rs:3`). Add unit tests documenting expected behavior for each function.

**Priority:** Medium | **Effort:** 1 day

---

## Verification Commands

After each phase:
```bash
cargo check -p slapper --features full
cargo test -p slapper --features full
cargo clippy --lib -p slapper
cargo test --doc -p slapper
```

Final verification:
```bash
# prost versions aligned
grep "prost" Cargo.lock | sort -u

# once_cell removed from slapper
grep -rn "once_cell" crates/slapper/src/ --include="*.rs" | wc -l  # Expected: 0

# No unused _config params
grep -rn "_config.*SlapperConfig" crates/slapper/src/ --include="*.rs" | wc -l  # Expected: 0

# No deprecated features
grep "mcp-server" crates/slapper/Cargo.toml  # Expected: no match

# Full test suite
cargo test -p slapper --features full

# Lint
cargo clippy -p slapper --features full -- -D warnings
```

---

## Parallelization Strategy

Use sub-agents to parallelize independent work:

| Sub-Agent | Phases | Rationale |
|-----------|--------|-----------|
| Agent 1 | 1.1 prost/tonic fix | Cargo.toml edit + lockfile update |
| Agent 2 | 1.2 config reloading fix | 2 files, 7 call sites |
| Agent 3 | 1.3 port scanner fix | 1 file, match arm changes |
| Agent 4 | 2.1 + 2.2 + 2.3 error variants + doc examples | All in error/mod.rs + 8 mod.rs files |
| Agent 5 | 2.4 + 2.5 unused params + mcp-server removal | ~15 files (3 func defs + ~10 callers + 4 cfg refs) |
| Agent 6 | 3.1 + 3.2 + 3.3 dependency updates | Cargo.toml changes (run after 3.3 for LazyLock) |
| Agent 7 | 3.4 + 4.1 + 4.2 CI + docs + audit | Non-code changes |
| Agent 8 | 5.1 tui/app.rs split | Large refactor, isolated |
| Agent 9 | 5.2 mcp.rs split | Large refactor, isolated |

**Note:** Sub-agents within a group are sequential; groups can run in parallel. Agent 6 should run 3.3 (MSRV) first, then 3.2 (LazyLock).

---

## Success Criteria

| Criterion | Status |
|-----------|--------|
| prost/prost-build versions | ✅ Matching (both 0.13) |
| Config reloading | ✅ Uses `ctx.config` directly |
| Port scanner errors | ✅ No silently dropped results |
| `Fingerprint` variant | ✅ Added |
| `InvalidHeaderValue` From impl | ✅ Added |
| Doc examples using `anyhow` | ✅ 0 |
| Unused `_config` parameters | ✅ Removed |
| `mcp-server` feature | ✅ Removed |
| `thiserror` version | ✅ 2.x |
| `once_cell` in slapper | ✅ 0 |
| MSRV declared | ✅ Yes (1.80) |
| Plugin CI checks | ✅ Present |
| Feature flag docs | ✅ Complete |
| `native-tls` → `rustls` | ✅ Migrated |
| `mcp.rs` split | ✅ 6 files, largest 890 lines |
| `tui/app.rs` split | ✅ 5 files |
| Clippy warnings | 1 (MSRV `is_multiple_of`, non-blocking) |
| All tests | 350+ passing |

---

## Remaining Work

### Phase 7: Architecture Improvements (Future Work)

These are larger initiatives for future planning:

- **7.1** Scope Enforcement Audit (2 days)
- **7.2** External API Circuit Breaker (2 days)
- **7.3** Sensitive Data Logging Audit (1 day)
- **7.4** Payload Lazy Loading (2 days)
- **7.5** Performance Optimizations (2 days)
- **7.6** Truncation Function Cleanup (1 day)
