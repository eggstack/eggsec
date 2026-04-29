# Slapper Improvement Plan - Deferred Items

**Date**: 2026-04-29
**Status**: MOSTLY COMPLETE
**Priority**: High

---

## Executive Summary

**Current State** (verified 2026-04-29):
- 1,115 passing tests (base library)
- 1,364 passing tests (with full features - ai-integration)
- 7 pre-existing AI test failures (ai::planner, ai::waf_bypass, ai::client)
- ~21 clippy warnings (TUI-specific acceptable)
- 503 source files
- 30 payload types
- 29 TUI tabs

**Completed in 2026-04-29 session**:
- Fixed ai-integration compilation errors (portfolio_path, AttackSurface, SkillMetadata, AlertRouter default)
- Fixed D.4: Help overlay [h/l] → [n/p] description in ui.rs
- Fixed D.6/D.10: Rewrote AuthTab with proper error handling and FocusArea enum
- Fixed E.7: Removed stub alexa subdomain query from enumerate()
- Verified E.6: Auto-Calibration System already implemented (fuzzer/calibration.rs)
- Verified Wave C items (C.3 AtomicU64 done, C.5 DashMap addressed, C.1/C.2 deferred per plan)

---

## Wave Organization

### Completed Waves: A, B (partial), C (partial), D (partial), E (partial), F (partial), G

### Deferred Items

**Wave C (Performance)**:
- C.1: Clone Storm Fix - Fuzzer execution loop - deferred (normal async pattern)
- C.2: FxHashMap Migration - 4 modules candidate - deferred (lower priority)
- C.3: AtomicU64 Counters - ALREADY FIXED (all 3 locations use AtomicU64)
- C.4: String Allocations - NOT A BUG (vague plan item)
- C.5: DashMap Migration - ALREADY ADDRESSED (per plan guidance)

**Wave D (TUI)**:
- D.1: UTF-8 Cursor Position Bug - deferred (complex refactoring)
- D.2: Hardcoded Colors - deferred (many files, lower priority)
- D.3: AuthTab Rewrite - COMPLETED
- D.4: Help Overlay Fix - COMPLETED
- D.5: Missing Keyboard Shortcuts - deferred (unclear what is needed)
- D.6: Inconsistent Error Handling - COMPLETED
- D.7: HistoryTab search - FALSE POSITIVE (method EXISTS)
- D.8: SettingsTab progress - FALSE POSITIVE (0.0 is CORRECT)
- D.9: Validation Feedback - deferred (feature not implemented)
- D.10: Inconsistent Focus Patterns - COMPLETED

**Wave E (Features)**:
- E.1: gRPC Implementation - deferred (stub methods, infrastructure exists)
- E.3: Empty Feature Consolidation - deferred (no candidates found)
- E.5: PDF Pagination Fix - deferred (low priority)
- E.6: Auto-Calibration System - ALREADY IMPLEMENTED
- E.7: Subdomain Enumeration - COMPLETED (removed stub alexa source)

**Wave F (Documentation)**:
- F.4: Skills Standardization - pending review

**Wave B (Code Quality)**:
- B.4: Remove Dead Code - deferred (ParsedDependency struct, is_input_focused method)

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

**Verified False Positives**:
- D.7: HistoryTab search - method EXISTS
- D.8: SettingsTab progress - 0.0 is CORRECT
- E.8: Templates - scanner/templates/ EXISTS
- E.4.2: AST vs regex - current implementation is intentionally regex-based

---

*Last updated: 2026-04-29*
*Status: MOSTLY COMPLETE*
*Verified by: main agent session 2026-04-29*