# Slapper Improvement Plan - Deferred Items

**Date**: 2026-04-29
**Status**: DEFERRED (Most items complete)
**Priority**: High

---

## Executive Summary

**Current State** (verified 2026-04-29):
- 1,115 passing tests (base library)
- 1,238 passing tests (with full features)
- 7 pre-existing AI test failures (ai::planner, ai::waf_bypass, ai::client)
- ~21 clippy warnings (TUI-specific acceptable)
- 503 source files
- 30 payload types
- 29 TUI tabs

**Completed in 2026-04-29 session**:
- Fixed ai-integration compilation (search.rs, session.rs, fuzzer.rs)
- Fixed form_detector test (has_field_named bug)
- Added Default impl for 8 types via clippy auto-fix
- Verified A.1-A.4 (security patterns, config permissions, plugin timeout)
- Verified B.1-B.2 (compilation fixed, 7 pre-existing test failures remain)
- Verified D.7, D.8, E.8 (false positives - items already correct)
- Verified G-1 through G-17 (documentation all complete)

---

## Wave Organization

### Completed Waves: A, B, C (partial), D (partial), E (partial), F, G

### Deferred Waves: C.1-C.5, D.1-D.6, D.9-D.10, E.1, E.3, E.5-E.7, F.4

---

## Deferred Items

### Wave C: Performance Improvements (C.1-C.5)

**Status**: DEFERRED - Requires profiling to verify actual bottlenecks

| Item | Description | Notes |
|------|-------------|-------|
| C.1 | Clone Storm Fix | Fuzzer execution loop - 13+ clones per iteration |
| C.2 | FxHashMap Migration | 4 modules candidate |
| C.3 | AtomicU64 Counters | 3 locations in scanner |
| C.4 | String Allocations | Variable interpolation, URL formatting |
| C.5 | DashMap Migration | NOT needed unless profiling shows contention |

**Note**: Do NOT migrate to DashMap unless profiling shows lock contention is a problem.

---

### Wave D: TUI Improvements (D.1-D.6, D.9-D.10)

**Status**: DEFERRED - Complex refactoring required

| Item | Description | Notes |
|------|-------------|-------|
| D.1 | UTF-8 Cursor Position Bug | Byte offset vs character position |
| D.2 | Hardcoded Colors | Replace with tc!() theme macro |
| D.3 | AuthTab Rewrite | 250 lines - needs complete rewrite |
| D.4 | Help Overlay Fix | h/l description incorrect |
| D.5 | Missing Keyboard Shortcuts | Add undocumented shortcuts |
| D.6 | Inconsistent Error Handling | ReconTab pattern to adopt |
| D.9 | Validation Feedback | Auto-trigger validation on input |
| D.10 | Inconsistent Focus Patterns | Enum-based pattern for Auth/History |

**Verified as FALSE POSITIVES (no work needed)**:
- D.7: HistoryTab search - method EXISTS and is functional
- D.8: SettingsTab progress - 0.0 is CORRECT (no async work)

---

### Wave E: Feature Completion (E.1, E.3, E.5-E.7)

**Status**: DEFERRED - New capability development

| Item | Description | Notes |
|------|-------------|-------|
| E.1 | gRPC Implementation | Requires protobuf, tonic integration |
| E.3 | Empty Feature Consolidation | LOW priority |
| E.5 | PDF Pagination Fix | LOW priority |
| E.6 | Auto-Calibration System | ffuf-style smart calibration |
| E.7 | Subdomain Enumeration Enhancement | 40+ OSINT sources |

**Verified as ALREADY IMPLEMENTED (no work needed)**:
- E.8: Community templates - scanner/templates/ EXISTS with full implementation
- E.4.1: PluginRegistry thread safety - Already uses Arc<RwLock<Vec>>>
- E.4.2: AST-based security - Current implementation is regex-based (not a bug)

**Verified as COMPLETE**:
- E.24: LongitudinalMemory max_scans_per_target - FIXED
- E.25: Agent handle_status_impl - Shows AI enabled, memory dir, target counts
- E.26: Skill System Improvements - version field ADDED
- E-20: TUI Spinner Animation - spinner_tick ADDED
- E-17: TUI Confirmation Dialogs - PopupKind::Destructive ADDED
- E-28: TargetsCommand Update variant - ADDED

---

### Wave F: Documentation (F.4)

**Status**: DEFERRED

| Item | Description | Notes |
|------|-------------|-------|
| F.4 | Skills Standardization | Review slapper_skills/ for format consistency |

**Verified as COMPLETE**:
- F.1: Payload types (22→30) - FIXED
- F.1.2: Recon modules (18→30+) - FIXED
- F.2.1: VULNERABILITY_GUIDE.md - CREATED
- F.2.2: SCAN_STRATEGY.md - CREATED
- F.3: README, CAPABILITIES, USAGE expanded - FIXED
- G-1 through G-17: All documentation items COMPLETE

---

### Wave B: Code Quality (B.4)

**Status**: DEFERRED (minor cleanup)

| Item | Description | Notes |
|------|-------------|-------|
| B.4 | Remove Dead Code | ParsedDependency struct, is_input_focused method |

---

## Pre-existing Test Failures (7 total)

These failures are pre-existing and will be addressed separately:

1. `ai::client::tests::test_extract_content_valid_response` - line count assertion (expects 3, gets 4)
2. `ai::planner::tests::test_parse_modifications_from_text_add_stage` - keyword extraction issue
3. `ai::planner::tests::test_parse_modifications_from_text_reduce_duration` - keyword matching
4. `ai::planner::tests::test_parse_modifications_multiple_types` - keyword matching
5. `ai::planner::tests::test_planner_cache_clear` - cache behavior
6. `ai::planner::tests::test_record_outcome_updates_success_rate` - cache entry creation
7. `ai::waf_bypass::tests::test_record_success_adds_to_knowledge_base` - knowledge base state

---

## Verification Commands

```bash
# Base build and tests
cargo check --lib -p slapper
cargo test --lib -p slapper

# Full features
cargo test --lib -p slapper --features rest-api,ai-integration

# Clippy
cargo clippy --lib -p slapper

# Plugin tests
cargo test -p slapper-plugin --features python-plugins,ruby-plugins

# Build verification
cargo build --release -p slapper
cargo build --release -p slapper --features full
```

---

## Dependencies Summary

**Sequential Dependencies**:
- Wave E requires A, B, C, D to be largely complete

**Verification Required** (before work):
- D.7 (HistoryTab search) - method already exists
- D.8 (SettingsTab progress) - 0.0 is correct behavior
- E.8 (Templates) - already implemented
- E.4.2 (AST vs regex) - current implementation is intentionally regex-based

---

*Last updated: 2026-04-29*
*Status: DEFERRED - Most items complete*
*Verified by: main agent session 2026-04-29*