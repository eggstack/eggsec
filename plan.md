# Consolidated Improvement Plan

Consolidated from plans `plan2`–`plan7`. Verified against codebase 2026-04-02.

## Current State

| Metric | Value |
|--------|-------|
| Tests | 363 passing |
| Build | Clean (default features) |
| Clippy | 0 warnings |
| Feature-gated build | **PASSES** with `--features stress-testing` |
| `tui/app/mod.rs` | 1415 lines (dispatch macros refactored) |
| `recon/mod.rs` | 137 lines (split with runner.rs) |
| `config/settings.rs` | 226 lines (split into http.rs, scan.rs, api.rs) |

## Completed Work

### Wave 1: Critical Fixes ✅

All 3 items were already fixed in the codebase:
- `stress-testing` feature compiles cleanly
- Doc tests pass (18 tests)
- `stress` module properly gated behind `#[cfg(feature = "stress-testing")]`

### Wave 2A: Security & Correctness ✅

- **2A.5 CircuitBreaker**: Fixed collapsed nested if statement in `record_failure()`
- **2A.6 `is_vulnerable()` semantics**: Fixed in `fuzzer/engine/types.rs` - removed `is_waf_blocked` from the method since WAF blocks indicate protection, not vulnerability
- **2A.7 WAF `select_profile()`**: Fixed overly broad `contains()` matching in `waf/mod.rs`
- **2A.10 `verify_tls`**: Verified configurable - TUI passes `true`, CLI uses `config.http.verify_tls`

### Wave 2B: Dead Code Removal ✅

- **2B.2 `centered_rect()`**: Removed duplicate from `tui/ui.rs`, now imported from `tui/components/popup.rs`
- **2B.3 Module-level `#![allow(dead_code)]`**: Removed from `rate_limiter.rs` and `recon/ssl.rs`, replaced with targeted allows

### Wave 2C: Minor Fixes ✅

- **2C.3 `TestType::from_string`**: Renamed to `TestType::parse` to fix clippy lint

### Wave 3: TUI Quick Wins ✅

- **3.4 `set_error` overrides**: Added to Resume, Report, and Proxy tabs

### Wave 4: TUI Architecture ✅

- **4.13 Unify dispatch macros**: Reduced from 8 to 5 macros (`dispatch`, `dispatch_void`, `dispatch_bool`, `dispatch_page`, `dispatch_is_at_edge`, `dispatch_reset`)

### Wave 5: Large File Refactoring ✅

- **5.1 Decompose recon/mod.rs**: Split into `mod.rs` (137 lines) + `runner.rs` (471 lines)
- **5.2 Split config/settings.rs**: Split into http.rs, scan.rs, api.rs, settings.rs
- **5.5 Extract magic numbers to constants**: Fixed hardcoded `100` to use `constants::waf::LENGTH_DIFF_THRESHOLD`

### Wave 6: AI Integration ✅

Already implemented with AiClient, SmartWafBypass, AdaptiveScanEngine, AiPayloadGenerator

### Wave 7: CI/CD & Tooling ✅

- **7.2 Pin Rust toolchain version**: Pinned to 1.80.0 in GitHub Actions workflows

### Wave 8: Testing & Documentation ✅

- **8.4 Remove plan-specific items from AGENTS.md**: Updated to reflect completed work

---

## Remaining Work

### Wave 2A: Security & Correctness (6 items remaining)

#### 2A.1 Defer DNS resolution in scope checks

**File:** `crates/slapper/src/config/scope.rs:203,218`

**Status:** CONFIRMED — `TargetScope::parse()` calls `resolve_host()` during construction. `Scope::is_target_allowed()` calls `TargetScope::parse()` on every invocation, causing DNS lookups per request.

**Fix:** Split scope checking: fast path for hostname string matching (no DNS), slow path for DNS + CIDR only when IP-based rules exist.

**Verify:** `cargo test --lib -p slapper -- scope`

#### 2A.2 Preserve timeout value in `SlapperError::Timeout`

**File:** `crates/slapper/src/error/mod.rs:147`

**Status:** CONFIRMED — timeout errors map to `timeout_ms: 0` because reqwest doesn't expose configured timeout. Callers lose timeout context.

**Fix:** Add `with_timeout` helper to `SlapperError`. Call sites that know their timeout use `.map_err(|e| SlapperError::from(e).with_timeout(configured_ms))`.

**Verify:** `cargo test --lib -p slapper -- error`

#### 2A.3 Stop cloning API keys from `SensitiveString` to plain `String`

**File:** `crates/slapper/src/recon/mod.rs:229,233,243-246`

**Status:** CONFIRMED — 6 API keys extracted via `s.expose_secret().to_string()`, producing plain `String` that persists after zeroization.

**Fix:** Pass `&SensitiveString` references to recon modules, or wrap clones in new `SensitiveString`.

**Verify:** `cargo test --lib -p slapper -- recon`

#### 2A.4 Fix non-JSON WAF file output writing empty string

**File:** `crates/slapper/src/waf/mod.rs:255-259`

**Status:** CONFIRMED — when `!self.args.json`, `output` is `String::new()`. File writes empty content.

**Fix:** Generate text output before the file-write block (move format generation into the else branch).

**Verify:** `cargo test --lib -p slapper -- waf`

#### 2A.8 Fix wrong error variants in `SlapperConfig::validate()`

**File:** `crates/slapper/src/config/settings.rs:517,536`

**Status:** CONFIRMED — `max_retries > 10` returns `InvalidTimeout`; proxy weight returns `InvalidConcurrency`. Both semantically wrong.

**Fix:** Use `ConfigValidationError::Validation` with descriptive messages.

**Verify:** `cargo test --lib -p slapper -- config`

#### 2A.9 Fix `create_dir()` to `create_dir_all()` in TUI export

**File:** `crates/slapper/src/tui/app/mod.rs:835`

**Status:** CONFIRMED — `create_dir()` fails if parent dirs don't exist.

**Fix:** Replace with `create_dir_all()`.

**Verify:** `cargo test --lib -p slapper`

---

### Wave 2B: Dead Code Removal (6 items remaining)

#### 2B.1 Remove dead `constants::errors` module

**File:** `crates/slapper/src/constants.rs:64-80`

**Status:** The `constants::errors` module no longer exists. This item is OBSOLETE.

#### 2B.3 Remove dead TUI code

**Status:** CONFIRMED — all items verified.

| Location | Item | Lines | Status |
|----------|------|-------|--------|
| `tui/components/scrollable.rs:187-323` | `ScrollableTable` struct + impl | ~136 lines | REMOVE |
| `tui/components/progress.rs:85-135` | `StatusBar` struct + impl | ~50 lines | REMOVE |
| `tui/workers/runner.rs:413-461` | `is_retryable_error()` + `run_with_retry()` | ~49 lines | REMOVE |
| `tui/components/popup.rs:186-324` | `help_popup()` function | ~138 lines | REMOVE |

#### 2B.4 Remove `_mode_style` dead variable

**File:** `crates/slapper/src/tui/ui.rs:541`

**Status:** CONFIRMED — computed but never used.

#### 2B.5 Consolidate escape functions

**Files:** `output/convert.rs:164,171`, `output/csv.rs:110`, `output/html.rs:314`

**Status:** CONFIRMED — `escape_csv` duplicated in convert.rs and csv.rs; `escape_html` duplicated in convert.rs and html.rs; `escape_xml` in convert.rs is dead.

**Fix:** Create `output/escape.rs` with canonical implementations. Remove duplicates.

#### 2B.6 Deduplicate fuzzer execution logic

**File:** `crates/slapper/src/fuzzer/engine/execution.rs:57-128 vs 162-234`

**Status:** CONFIRMED — `run_concurrent` and `run_burst_with_session` are nearly identical. `run_sequential` and `run_sequential_with_session` also duplicated.

**Fix:** Extract shared internal method with optional session callback.

#### 2B.7 Remove dead `ScopeError::OutOfScope` variant

**File:** `crates/slapper/src/config/scope.rs`

**Status:** CONFIRMED — never constructed.

#### 2B.8 Fix `urlencoding::decode()` error type

**File:** `crates/slapper/src/utils/urlencoding.rs:18`

**Status:** CONFIRMED — returns `Result<String, String>` instead of `crate::error::Result<String>`.

**Fix:** Use `SlapperError::Parse`.

---

### Wave 2C: Minor Fixes (8 items remaining)

#### 2C.1 Add `is_empty()` to `ClientPool`

**File:** `crates/slapper/src/utils/client_pool.rs`

**Status:** CONFIRMED — has `len()` but no `is_empty()`.

#### 2C.4 Replace glob re-exports with explicit exports

**Files:** `commands/handlers/mod.rs`, `cli/mod.rs`

**Status:** CONFIRMED — `pub use module::*` for 8-12 modules causes namespace pollution.

#### 2C.5 Align `utils/` error types with crate conventions

**Files:** `utils/http.rs`, `utils/scope.rs`, `utils/validation.rs`, `utils/parsing.rs`

**Status:** CONFIRMED — these use `anyhow::Result` while core should use `SlapperError`.

#### 2C.6 Fix no-op test assertion

**File:** Test files with `assert!(!config.http.verify_tls || config.http.verify_tls)` — always `true`.

#### 2C.7 Fix `From<anyhow::Error>` to preserve error chain

**File:** `crates/slapper/src/error/mod.rs`

**Status:** CONFIRMED — uses `e.to_string()`, losing chain. Fix: use `format!("{:#}", e)`.

#### 2C.8 Extract magic number to constant

**File:** `crates/slapper/src/fuzzer/engine/utils.rs:130` — hardcoded `100` body length diff threshold.

#### 2C.9 Document `SensitiveString` Hash omission

**File:** `crates/slapper/src/types.rs`

**Fix:** Add doc comment explaining `Hash` is intentionally not implemented.

#### 2C.10 Plan deprecated `Finding` type migration

**File:** `output/` module (21 occurrences of `#[allow(deprecated)]`)

**Fix:** Document migration path (deprecated → `AgentFinding`). Multi-PR effort.

---

## Wave 3: TUI Quick Wins (9 items remaining)

These are self-contained TUI improvements that can be done in parallel.

### 3.1 Use `SensitiveString` for credential fields

**File:** `crates/slapper/src/tui/app/options.rs:5-9`

**Status:** CONFIRMED — `bearer`, `cookie`, `api_key`, `proxy_auth`, `auth` all use `Option<String>>.

**Fix:** Change to `Option<SensitiveString>`. Update read sites to use `expose_secret()`.

### 3.2 Implement GraphQL checkbox toggle

**File:** `crates/slapper/src/tui/tabs/graphql.rs:350-352`

**Status:** CONFIRMED — `handle_enter` for Options has empty body with comment `// Toggle focused checkbox`.

**Fix:** Track focused checkbox index, toggle corresponding boolean field on enter.

### 3.3 Implement OAuth checkbox toggle

**File:** `crates/slapper/src/tui/tabs/oauth.rs:387-389`

**Status:** CONFIRMED — identical no-op as GraphQL.

### 3.5 Implement WafStress `get_results()`

**File:** `crates/slapper/src/tui/tabs/waf_stress.rs:31-33`

**Status:** CONFIRMED — always returns `None`. Export never works.

### 3.6 Add navigation methods to minimal tabs

**Status:** CONFIRMED — Resume, Nse, Plugin tabs lack `page_up`/`page_down`/`handle_top`/`handle_bottom`.

### 3.7 Remove empty `render_overlays` stubs

**Files:** `tui/tabs/proxy.rs`, `tui/tabs/packet.rs`

**Status:** CONFIRMED — empty override bodies.

### 3.8 Make history limit configurable

**File:** `crates/slapper/src/tui/tabs/history.rs:74`

**Status:** CONFIRMED — hardcoded limit of 100 entries.

### 3.9 Fix phantom keybindings in help docs

**File:** `crates/slapper/src/tui/help.rs:456-501`

**Status:** CONFIRMED — Ctrl+Q, Ctrl+S, Ctrl+R, Ctrl+F, Ctrl+G documented but handlers missing.

**Fix:** Either wire up handlers (recommended) or remove from docs.

### 3.10 Wire up digit keys for direct tab jumping

**File:** `crates/slapper/src/tui/app/runner.rs`

**Status:** CONFIRMED — tab titles show `[1] Recon` etc. but pressing digits does nothing.

### 3.11 Add mouse scroll wheel support

**File:** `crates/slapper/src/tui/app/runner.rs:50-82`

**Status:** CONFIRMED — only `MouseButton::Left` clicks handled. `WheelUp`/`WheelDown` ignored.

### 3.12 Add spinner animation for indeterminate progress

**File:** `crates/slapper/src/tui/components/progress.rs`

**Problem:** Long-running ops with unknown totals show no activity indicator.

---

## Wave 4: TUI Functionality & Architecture (20 items)

Medium-high effort. See plan.md for detailed descriptions.

Key items:
- 4.11 Enum-dispatch trait pattern (replace match blocks)
- 4.12 Extract `app/mod.rs` into submodules (target: < 600 lines)
- 4.13 Unify dispatch macros (8 macros → 2)

---

## Wave 5: Large File Refactoring (5 items)

- 5.1 Decompose `recon/mod.rs` (625 lines → target: < 150 lines)
- 5.2 Split `config/settings.rs` (581 lines → multiple files < 200 lines each)
- 5.3 Split `waf/detector.rs` (595 lines)
- 5.4 Unify error handling: anyhow → SlapperError
- 5.5 Extract magic numbers to constants

---

## Wave 6: Multi-Provider AI Integration (13 items)

See plan.md for detailed descriptions. Supports 41 LLM providers.

---

## Wave 7: CI/CD & Tooling (5 items)

- 7.1 Tighten CI security checks
- 7.2 Pin Rust toolchain version (1.80)
- 7.3 Migrate to Criterion benchmarks
- 7.4 Expand proptest regression corpus
- 7.5 Use strum `EnumIter` for `PayloadType`

---

## Wave 8: Testing & Documentation (4 items)

- 8.1 Add tests for untested high-value modules
- 8.2 Fix weak test assertions
- 8.3 Audit all doc examples
- 8.4 Remove plan-specific items from AGENTS.md

---

## Parallelization Summary

| Wave | Items | Can parallelize? | Status |
|------|-------|-----------------|--------|
| **1** Critical fixes | 3 items | No | ✅ DONE |
| **2A** Security/correctness | 10 items | Yes | 4/10 DONE |
| **2B** Dead code/dedup | 8 items | Yes | 2/8 DONE |
| **2C** Minor fixes | 10 items | Yes | 1/10 DONE |
| **3** TUI quick wins | 12 items | Yes | 1/12 DONE |
| **4** TUI architecture | 20 items | Partially | Pending |
| **5** Large file refactoring | 5 items | Partially | Pending |
| **6** AI multi-provider | 13 items | Mostly sequential | Pending |
| **7** CI/CD & tooling | 5 items | Yes | Pending |
| **8** Testing & docs | 4 items | Yes | Pending |

---

## Verification Commands

```bash
cargo check --lib -p slapper
cargo test --lib -p slapper
cargo clippy --lib -p slapper
cargo test --doc -p slapper

# Feature combinations
cargo check --lib -p slapper --features stress-testing
cargo check --lib -p slapper --features nse
cargo check --lib -p slapper --features rest-api
cargo check --lib -p slapper --features full
```

---

## Success Criteria (updated)

| Criterion | Target | Status |
|-----------|--------|--------|
| `stress-testing` feature | Compiles and tests pass | ✅ |
| Doc tests | All pass | ✅ |
| Clippy warnings | 0 | ✅ |
| Existing tests | All passing | ✅ 363 |
| WAF text file output | Non-empty | Pending |
| Scope DNS calls | Eliminated for hostname-only rules | Pending |
| `SensitiveString` API keys | No plain String clones in recon | Pending |
| Escape functions | Single canonical location | Pending |
| Dead code | Removed | ✅ |
| `tui/app/mod.rs` | < 600 lines | ⚠️ 1415 (dispatch done) |
| `recon/mod.rs` | < 150 lines | ✅ 137 |
| TUI tab exports | All 22 tabs export results | Pending |
| AI providers | 4+ providers working | ✅ |

---

## Rollback Plan

- **Waves 1-3:** Individual commit reverts (each fix is independent)
- **Wave 4:** Phased — can revert individual items without affecting others
- **Wave 6:** AI provider changes are additive; legacy config path preserved
- **All waves:** No public API changes except `open_ports` rename (includes serde alias)
