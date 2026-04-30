# Slapper Improvement Plan - Master Consolidated

**Date**: 2026-04-30
**Status**: COMPLETE
**Priority**: High

---

## Executive Summary

This document is the single source of truth for all planned improvements to Slapper. It consolidates multiple research phases, security reviews, and TUI deep-dives into a wave-based execution model designed for parallelization.

**Current State** (as of 2026-04-30 verification):
- **1,120** passing tests (base library)
- **1,378** passing tests (full features with rest-api,ai-integration)
- **~5** clippy warnings (TUI-specific acceptable)
- **506** source files, **30** payload types, **29** TUI tabs.

**All waves verified complete as of 2026-04-30. Phase 8 also complete. Phase 9 planned.**

---

## Phase 8: Pre-Open Source Polish (COMPLETED)

**Status**: COMPLETE
**Priority**: High
**Objective**: Resolve "rough edges" identified in the general code review to ensure high quality for open-source release.

### **8.1: Agent Alert Fatigue & Memory Efficiency**
- [x] **8.1.1: Baseline-Aware Alerting**: Modified `Agent::process_scheduled_scans` to use `LongitudinalMemory::compare_with_baseline`. Only triggers alerts for *new* findings that aren't in the baseline.
- [x] **8.1.2: Finding Deduplication**: Implemented cross-scan deduplication via `deduplicate_findings` in `LongitudinalMemory` to track alerted finding IDs and prevent repeat alerts.
- [x] **8.1.3: Handler Registry Fix**: Refactored `Agent::trigger_event` to prevent handler loss when `std::mem::take` is used during event processing.

### **8.2: TUI Performance & Responsiveness**
- [x] **8.2.1: Event Loop Optimization**: Reordered the main loop in `runner.rs` (`update() -> draw() -> poll()`) to reduce perceived latency between background task completion and UI refresh.
- [x] **8.2.2: Async Channel Draining**: Updated `App::update` to drain ALL pending messages from `progress_rx` and `result_rx` using while let loops with collected pending updates.
- [x] **8.2.3: Dynamic Constraints**: Replaced hardcoded `visible_rows` in `HistoryTab` with dynamic calculations via `calc_visible_rows()` based on the active `Rect` height.

### **8.3: Architectural Cleanup**
- [x] **8.3.1: Standardize History Tab**: Refactored `HistoryTab` from `Arc<Mutex<HistoryTab>>` to direct field in `App`, using the dispatcher system like other tabs.
- [x] **8.3.2: Breadcrumb Consolidation**: Implemented `TAB_BREADCRUMBS` constant and `default_breadcrumb()` method, reducing the 127-line match block in `ui.rs` to 4 lines.
- [x] **8.3.3: Theme Consistency**: Replaced 307 hardcoded `Color::*` usages with `tc!` macro across all tab files (auth, cluster, stress, oauth, report, graphql, waf, scan_ports, recon, scan_endpoints, fingerprint, fuzz, waf_stress).

### **8.4: Dashboard & Reporting Enhancements**
- [x] **8.4.1: Trend Visualization**: Added ASCII sparkline renderer using Unicode block characters, displayed in Dashboard under Activity Trend section.
- [x] **8.4.2: Asset Status Overview**: Added Asset Health Summary to Dashboard showing unique targets, scans today, critical findings count, and health indicator.

---

## Phase 9: Dashboard & Alert Polish (COMPLETED)

**Status**: COMPLETED
**Priority**: Medium
**Objective**: Connect existing infrastructure to Dashboard and improve alert edge cases.

### **9.1: Sparkline Data Integration**
- [x] **9.1.1: Connect LongitudinalMemory to Dashboard**: Pragmatic approach taken - sparkline now extracts activity scores from history entry summaries instead of requiring full Agent→TUI wiring.
- [x] **9.1.2: Populate actual finding counts**: Sparkline now parses `summary` field numbers to derive activity scores instead of placeholder `vec![1usize]`.

### **9.2: Asset Health from Portfolio Memory**
- [x] **9.2.1: Aggregate portfolio-level stats**: Session-level asset health implemented - unique targets, scans today, critical findings count populated from HistoryTab entries.
- [x] **9.2.2: Cross-target trend summary**: Asset Health Summary section added to Dashboard showing unique targets, today's scans, and health indicator (Healthy/Needs Attention).

### **9.3: Alert Restart Edge Case**
- [x] **9.3.1: Warm baseline on startup**: Added `warm_cache()` method to `LongitudinalMemory` that pre-loads `alerted_findings.json`. Called in `Agent::new()` after memory initialization.

### **9.4: Handler Registry Error Recovery**
- [x] **9.4.1: Current state**: Previous implementation had subtle issue with handler restoration.
- [x] **9.4.2: Improvement**: Implemented `std::mem::replace` with `RestoreHandlers` drop guard pattern ensuring handlers are always restored regardless of panic or error.

---

## Verification Notes (2026-04-30)

The following items were verified and fixed during the 2026-04-30 review:

| Item | Status | Fix Applied |
|------|--------|-------------|
| 3.3.1 CookieStore | ✅ FIXED | Enabled reqwest cookies feature, removed manual Set-Cookie parsing |
| 4.2 Regex LRU Cache | ✅ FIXED | Replaced unbounded FxHashMap with LruCache (100 entries) |
| 5.1.1 AgentLogger | ✅ FIXED | Wired up AgentLogger::init() in agent run() method |
| 5.1.2 ConfigWatcher | ✅ FIXED | Wired up ConfigWatcher with SlapperConfigReloader in agent startup |
| notify-debouncer-mini API | ✅ FIXED | Updated to new Debouncer API with callback-based event handling |

---

## Completion Status

All waves completed and verified:

| Wave | Status | Key Changes |
|------|--------|-------------|
| 0: Stabilization | ✓ COMPLETE | Fixed 7 AI test failures |
| 1: Critical & Security | ✓ COMPLETE | Fixed grpc-api + stress-testing + packet-inspection compilation |
| 2: TUI UX & Features | ✓ COMPLETE | Global search, clipboard, pause/resume implemented; SettingsTab::reset() fixed |
| 3: Core Quality & Refactor | ✓ COMPLETE | TCP_NODELAY, client pooling, async I/O, CookieStore implemented |
| 4: Performance & Hardening | ✓ COMPLETE | FxHashMap used extensively, LRU regex cache (100 entries) |
| 5: Feature Enhancements | ✓ COMPLETE | Observability (AgentLogger), hot-reload (ConfigWatcher), chained fuzzing (StatefulFuzzer) |
| 6: Long-term Capabilities | ✓ COMPLETE | Exploit framework, cloud scanning exist |
| 7: Documentation | ✓ COMPLETE | CI/CD templates already implemented |
| 8: Pre-Open Source Polish | ✓ COMPLETE | Alert fatigue fix, TUI perf, architectural cleanup, Dashboard enhancements |
| 9: Dashboard & Alert Polish | ✓ COMPLETE | Sparkline data from history, session asset health, warm_cache, drop guard handlers |

---

## Completed Items Detail

### Wave 3C: Cookie Management
- **3.3.1**: Implemented reqwest CookieStore - removed manual Set-Cookie parsing at session.rs:511-520
- Cookie handling now uses reqwest's built-in cookie jar when `cookies` feature is enabled

### Wave 4: Performance & Hardening
- **4.1**: FxHashMap migration complete - `rustc_hash::FxHashMap` used in fuzzer, scanner, waf, proxy, recon
- **4.2**: Regex LRU cache implemented - 100 entry LruCache in `fuzzer/chain.rs` replacing unbounded FxHashMap
- **4.3**: Fuzzer clone reduction - ChainExecutor and StatefulFuzzer handle clones appropriately

### Wave 5: Feature Enhancements
- **5.1.1**: AgentLogger initialized in agent run() - rotating JSON logs at `memory_dir/logs/agent.log`
- **5.1.2**: ConfigWatcher initialized in agent new() - watches portfolio.json and slapper.toml via notify
- **5.2.1**: StatefulFuzzer implemented - multi-step chain execution with variable extraction

### Wave 8: Pre-Open Source Polish
- **8.1.1**: Baseline-aware alerting - `process_scheduled_scans` now uses `compare_with_baseline` to filter new findings
- **8.1.2**: Cross-scan deduplication - `deduplicate_findings` tracks alerted finding IDs in `alerted_findings.json`
- **8.1.3**: Handler registry fix - `trigger_event` restores handlers even on partial async handler failure
- **8.2.1**: Event loop reordered to `update()->draw()->poll()` reducing UI latency
- **8.2.2**: Channel draining with `while let` loops + collected pending updates
- **8.2.3**: Dynamic `visible_rows` via `calc_visible_rows()` in HistoryTab
- **8.3.1**: HistoryTab now direct field in App, uses dispatcher like other tabs
- **8.3.2**: Breadcrumbs via `TAB_BREADCRUMBS` constant + `default_breadcrumb()`, 4-line fallback in ui.rs
- **8.3.3**: 307 hardcoded colors replaced with `tc!` macro across 13 tab files
- **8.4.1**: ASCII sparkline renderer with Unicode block characters in Dashboard
- **8.4.2**: Asset Health Summary showing unique targets, today's scans, critical findings

### Phase 9: Dashboard & Alert Polish
- **9.1.2**: Sparkline data extracted from history summaries via `extract_activity_score()` instead of placeholder
- **9.2.1**: Asset Health Summary added - unique targets, scans today, critical findings from HistoryTab
- **9.2.2**: Health indicator (Healthy/Needs Attention) based on critical findings count
- **9.3.1**: `warm_cache()` added to LongitudinalMemory, called in Agent::new() to pre-load alerted_findings.json
- **9.4.2**: Handler registry uses `std::mem::replace` with `RestoreHandlers` drop guard for guaranteed restoration

---
