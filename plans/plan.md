# Slapper Consolidated Improvement Plan

## Overview

This plan consolidates three improvement initiatives:
1. **CLI Ergonomics** (plan2.md) - Standardize flags, add argument groups, modern CLI patterns
2. **AI Harness** (plan3.md) - Fix bugs, refactor code, improve architecture
3. **TUI Architecture** (plan4.md) - Trait-based dispatch, compliance improvements

**Current Codebase State:**
- 976 passing tests (974 without ai-integration)
- 0 clippy warnings
- 29 Tab variants (Recon through Vuln)
- Feature-gated AI module behind `ai-integration`
- TabState, TabRender, TabInput traits implemented
- 15+ compliance checks in security.rs

---

## Wave 1: Critical Bug Fixes

Items that must be completed first as they block compilation.

### Block A1: Fix Cache Serialization Bug
- **File**: `crates/slapper/src/ai/cache.rs:68`
- **Issue**: Instant timestamp restoration uses incorrect arithmetic
- **Problem**: `Instant::now()` is relative to boot, not system clock; `num_nanoseconds()` returns Option
- **Fix**: Use proper timestamp serialization that doesn't depend on boot-relative time

### Block A2: Add Error Conversion
- **Files**: `crates/slapper/src/ai/errors.rs`, `client.rs`, `planner.rs`
- **Issue**: `AiError` doesn't implement `From<reqwest::Error>`
- **Fix**: Add `impl From<reqwest::Error> for AiError`
- **Verification**: `cargo check --lib -p slapper --features ai-integration`

### Block A3: Remove Unused Import
- **File**: `crates/slapper/src/ai/cache.rs:1`
- **Issue**: Unused `use crate::ai::errors::Result;`
- **Fix**: Remove the import

**Parallelization**: Blocks A1-A3 are independent and can run in parallel.

---

## Wave 2: AI Module Refactoring

Depends on: Wave 1

### Block B1: Eliminate Code Duplication in AiClient
- **File**: `crates/slapper/src/ai/client.rs`
- **Issue**: Three nearly identical methods (analyze_findings, suggest_payloads, suggest_waf_bypass)
- **Fix**: Refactor to common method with parameterized request handling

### Block B2: Centralize API URL Handling
- **Files**: `client.rs`, `planner.rs`
- **Issue**: Hardcoded API URLs in multiple locations
- **Fix**: Use `AiConfig.base_url` centrally; remove duplication in planner.rs

### Block B3: Make Planner Use AiClient
- **File**: `crates/slapper/src/ai/planner.rs`
- **Issue**: Duplicates HTTP logic instead of reusing AiClient
- **Fix**: Refactor to use AiClient methods

### Block C1: Replace HashMap with AiCache (Payloads)
- **File**: `crates/slapper/src/ai/payloads.rs`
- **Issue**: Simple HashMap instead of robust AiCache
- **Fix**: Replace with AiCache with TTL and persistence

### Block C2: Replace Knowledge Base with AiCache
- **File**: `crates/slapper/src/ai/waf_bypass.rs`
- **Issue**: Custom Vec instead of AiCache
- **Fix**: Replace with AiCache

### Block C3: Add Typed Result Parsing
- **Files**: `client.rs`, `types.rs`
- **Issue**: analyze_findings returns raw Value
- **Fix**: Use defined `AiAnalysisResult` type

**Parallelization**: B1-B3 can be done in parallel after Wave 1; C1-C3 depend on B1-B3 completion.

---

## Wave 3: CLI Ergonomics Improvements

Independent from Waves 1-2.

### Block A: Flag Standardization

#### A1: Concurrency Flag
- **Files**: `cli/fuzz.rs`, `cli/http.rs`, `cli/scan.rs`
- **Issue**: Mixed `-c` vs `--concurrency` across commands
- **Fix**: Standardize all commands to use `-c`

#### A2: Output Flag
- **Files**: All `cli/*.rs` files with output arguments
- **Issue**: Mixed `-o`, `--output`, positional args
- **Fix**: Standardize on `-o`/`--output`, convert positional bools to `--json` flag

#### A3: Verbose/Quiet Flags
- **Files**: All commands in `cli/*.rs`
- **Issue**: Inconsistent verbose exposure; --quiet only on recon and ci
- **Fix**: Add `--verbose`/`-v` and `--quiet`/`-q` to all commands

### Block B: Argument Groups

#### B1: Port Scanning Grouping
- **File**: `cli/scan.rs`
- **Issue**: 15+ individual spoofing flags
- **Fix**: Create `SpoofOptions` struct with grouped arguments

#### B2: HTTP Auth Grouping
- **File**: `cli/mod.rs`
- **Issue**: `CommonHttpArgs` not using clap groups
- **Fix**: Refactor to use `#[group(args_tag = "auth")]`

### Block C: Modern CLI Patterns

#### C1: Add -y Confirmation Flag
- **Files**: `cli/auth.rs`, `cli/stress.rs`, `cli/misc.rs`
- **Issue**: No auto-confirmation for destructive commands
- **Fix**: Add `-y`/`--yes` flag to destructive commands

#### C2: Add --quiet Mode
- **Files**: All commands that produce output
- **Issue**: --quiet only on ci command
- **Fix**: Add to fuzz, scan, scan-ports, scan-endpoints, fingerprint, recon, load, waf, graphql, oauth, auth-test

### Block D: Help Improvements

#### D1: Categorize Help Output
- **File**: `cli/mod.rs`
- **Issue**: 35+ commands in flat list
- **Fix**: Add `after_help` with categories (Discovery, Attack, Infrastructure, Utilities)

#### D2: Add Missing Examples
- **Files**: `cli/misc.rs` (PluginArgs, RemoteArgs, ReportArgs)
- **Issue**: Missing `*_ABOUT` constants
- **Fix**: Add help text constants with examples

**Parallelization**: Within each block (A, B, C, D), items can run in parallel. Blocks are sequential (A→B→C→D).

---

## Wave 4: TUI Architecture Refactor

### Block A: Trait-Based State Access

The Tab enum has feature-gated variants. Methods must use `#[cfg]/#[cfg(not)]` pattern.

#### A1: Add as_tab_state() Method
- **File**: `crates/slapper/src/tui/tabs/mod.rs`
- **Issue**: No way to get state without matching
- **Fix**: Add `pub fn as_tab_state(&self, app: &App) -> &dyn TabState`

#### A2: Add as_tab_state_mut() Method
- **File**: Same as A1
- **Fix**: Add mutable variant

#### A3: Add as_tab_render() Method
- **File**: Same as A1
- **Fix**: Add render access method

#### A4: Handle Spawn Decision
- **Issue**: TabInput::handle_enter() returns `()`, but spawn logic needs to know if task started
- **Solution**: Hybrid approach - keep match for spawn logic, use traits for method calls

### Block B: Replace Match Statements in App

- **File**: `tui/app/mod.rs` (~1600 lines, 22 match statements)
- **B1**: Refactor `handle_enter()` - 22 arms
- **B2**: Refactor input handlers (handle_escape, handle_char, handle_backspace, etc.) - 11 methods × 22 arms
- **B3**: Refactor `set_error_for_current_tab()` - 22 arms
- **B4**: Refactor `get_progress_for_current_tab()` - 22 arms
- **B5**: Refactor `reset_current_tab()` - 22 arms
- **B6**: Refactor `is_current_tab_running()` - 22 arms

### Block C: Replace Match Statements in UI

- **File**: `tui/ui.rs` (~400 lines, 3 match statements)
- **C1**: Refactor `draw_breadcrumb()` - 135 lines
- **C2**: Refactor `draw_content()` - 140 lines
- **C3**: Refactor `get_status_text()` - 60 lines

### Block D: Replace Match Statements in Other Files

- **File**: `tui/app/state_update.rs` (2 match statements)
- **File**: `tui/app/export.rs` (3 match statements)
- **File**: `tui/app/navigation.rs` (1 match statement)

**Parallelization**: A1-A4 can be done in parallel (different methods). B1-B6 can be parallelized (different methods). C1-C3 parallel. D1-D3 parallel.

---

## Wave 5: Compliance Improvements

### Block E1: Real Severity Derivation
- **File**: `tui/workers/security.rs:37-130`
- **Issue**: Hardcoded severity levels regardless of context
- **Fix**: Derive severity from:
  - Actual scan results
  - Target URL analysis (protocol, path, parameters)
  - Security header presence with CVSS-like scoring
  - Production vs development classification

### Block E2: Target Classification
- **Issue**: No differentiation between production/development
- **Fix**: Add logic to classify targets:
  - Production vs Development
  - Public API vs Web Application
  - Authentication-present vs Anonymous

---

## Wave 6: Testing & Documentation

### Block F1: Add AI Module Tests
- **client.rs**: 0 tests → Add mock HTTP, circuit breaker tests
- **waf_bypass.rs**: 0 tests → Add bypass suggestion tests
- **payloads.rs**: 0 tests → Add payload generation tests

### Block F2: Add Documentation
- **client.rs**: Add doc comments to public methods (~30 LOC)
- **waf_bypass.rs**: Add doc comments (~20 LOC)
- **payloads.rs**: Add doc comments (~15 LOC)

### Block F3: Verification Tests
```bash
# Full test suite
cargo test --lib -p slapper

# TUI-specific tests
cargo test --test tui_tests -p slapper

# Clippy
cargo clippy --lib -p slapper

# Feature-gated tests
cargo test --lib -p slapper --features ai-integration
cargo test --lib -p slapper --features full
```

---

## Execution Summary

| Wave | Focus | Actual Status |
|------|-------|----------------|
| 1 | Critical Bug Fixes | DONE (all 3 items completed) |
| 2 | AI Module Refactoring | DONE |
| 3 | CLI Ergonomics | Incomplete (few flags standardized) |
| 4 | TUI Architecture | Not started |
| 5 | Compliance | Partially done (15+ checks, no classification) |
| 6 | Testing & Docs | Incomplete |

---

## Parallelization Strategy

### Phase 1: Independent Work (Can start immediately)
- **Agent 1**: Wave 3 (CLI Ergonomics)
- **Agent 2**: Wave 4 (TUI Architecture)
- **Agent 3**: Wave 5 (Compliance)

### Phase 2: After Wave 1
- **Agent 4**: Wave 2 (AI Module)

### Phase 3: Sequential
- Wave 6 requires completion of Waves 1-5

---

## Success Criteria

### Wave 1 (Bug Fixes) - COMPLETED
- [x] AiError implements From<reqwest::Error> (errors.rs:34-46)
- [x] Unused import removed from cache.rs
- [x] Cache serialization fixed - now uses DateTime<Utc> instead of Instant

### Wave 2 (AI Refactoring) - COMPLETED
- [x] AiClient code duplication eliminated (chat_completion_from_messages added)
- [x] AiCache already used in payloads.rs and waf_bypass.rs
- [x] Centralized API URL handling (api_url(), model() methods)
- [x] Typed result parsing (analyze_findings_typed added)
- [x] Planner uses AiClient methods (query_ai_for_plan, query_ai_for_adjustments)

### Wave 3 (CLI) - INCOMPLETE
- [x] All commands use `-c` for concurrency
- [x] All commands use `-o` for output
- [ ] --verbose flag not standardized across all commands
- [ ] --quiet flag only in http.rs and ci.rs
- [ ] -y/--yes flag not implemented on destructive commands
- [ ] Help categories not implemented

### Wave 4 (TUI) - NOT COMPLETED
- [ ] as_tab_state() method not implemented
- [ ] as_tab_state_mut() method not implemented  
- [ ] as_tab_render() method not implemented
- [ ] Match statements not refactored

### Wave 5 (Compliance) - PARTIALLY COMPLETED
- [x] 15+ checks implemented in security.rs
- [ ] Target classification (production vs dev) not fully implemented

### Wave 6 (Final) - INCOMPLETE
- [x] Clippy 0 warnings
- [x] Tests pass (976 tests)
- [ ] No AI module doc comments added

---

## Backward Compatibility

All changes MUST maintain backward compatibility:
1. **Deprecated flags**: Add new flags alongside old, mark old as deprecated
2. **Config file compatibility**: Existing configs must continue to work
3. **Version consideration**: Consider minor version bump after CLI changes

---

## Files to Modify

### Wave 1 (AI Bugs)
| File | Changes | Status |
|------|---------|--------|
| `ai/cache.rs` | Fix serialization, remove unused import | DONE |
| `ai/errors.rs` | Add From impl | DONE |

### Wave 2 (AI Refactor)
| File | Changes | Status |
|------|---------|--------|
| `ai/client.rs` | Refactor duplication, typed results | DONE (added chat_completion_from_messages, analyze_findings_typed) |
| `ai/planner.rs` | Use AiClient | DONE |
| `ai/payloads.rs` | Use AiCache | DONE (already using AiCache) |
| `ai/waf_bypass.rs` | Use AiCache | DONE (already using AiCache) |

### Wave 3 (CLI)
| File | Changes | Status |
|------|---------|--------|
| `cli/mod.rs` | Help categories, CommonHttpArgs refactor | NOT IMPLEMENTED |
| `cli/scan.rs` | Argument groups, -c/-o standardization | PARTIALLY DONE |
| `cli/fuzz.rs` | Flag standardization | PARTIALLY DONE |
| `cli/http.rs` | Flag standardization | PARTIALLY DONE |
| `cli/auth.rs` | Add -y flag | NOT IMPLEMENTED |
| `cli/stress.rs` | Add -y flag | NOT IMPLEMENTED |
| `cli/misc.rs` | Flag standardization, examples | PARTIALLY DONE |
| `cli/packet.rs` | Add --quiet | NOT IMPLEMENTED |
| `cli/cluster.rs` | Verify flags | NOT VERIFIED |
| `cli/ci.rs` | Already has --quiet | DONE |

### Wave 4 (TUI)
| File | Changes | Status |
|------|---------|--------|
| `tui/tabs/mod.rs` | Add dispatch methods | NOT IMPLEMENTED |
| `tui/app/mod.rs` | Replace 22 match statements | NOT IMPLEMENTED |
| `tui/ui.rs` | Replace 3 match statements | NOT IMPLEMENTED |
| `tui/app/state_update.rs` | Replace 2 statements | NOT IMPLEMENTED |
| `tui/app/export.rs` | Replace 3 statements | NOT IMPLEMENTED |
| `tui/app/navigation.rs` | Replace 1 statement | NOT IMPLEMENTED |

### Wave 5 (Compliance)
| File | Changes | Status |
|------|---------|--------|
| `tui/workers/security.rs` | Real severity derivation | PARTIALLY DONE (15+ checks done, classification not done) |

---

## Verification Commands

```bash
# Build
cargo build --release -p slapper

# Library tests
cargo test --lib -p slapper

# TUI tests
cargo test --test tui_tests -p slapper

# Clippy
cargo clippy --lib -p slapper

# Feature-gated checks
cargo check --lib -p slapper --features ai-integration
cargo check --lib -p slapper --features full

# Verify CLI help
./target/release/slapper --help
./target/release/slapper fuzz --help
./target/release/slapper scan-ports --help
```
