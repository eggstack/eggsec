# Slapper Improvement Plan - Master Consolidated

**Date**: 2026-04-30
**Status**: IN PROGRESS
**Priority**: High

---

## Executive Summary

This document is the single source of truth for all planned improvements to Slapper. It consolidates multiple research phases, security reviews, and TUI deep-dives into a wave-based execution model designed for parallelization.

**Current State** (as of 2026-04-30 verification):
- **1,120** passing tests (base library)
- **1,378** passing tests (full features with rest-api,ai-integration)
- **~5** clippy warnings (TUI-specific acceptable)
- **506** source files, **30** payload types, **29** TUI tabs.

**All waves verified complete as of 2026-04-30.**

---

## Phase 8: Pre-Open Source Polish (Planned)

**Status**: PLANNED
**Priority**: High
**Objective**: Resolve "rough edges" identified in the general code review to ensure high quality for open-source release.

### **8.1: Agent Alert Fatigue & Memory Efficiency**
- [ ] **8.1.1: Baseline-Aware Alerting**: Modify `Agent::process_scheduled_scans` to use `LongitudinalMemory::compare_with_baseline`. Only trigger alerts for *new* findings that aren't in the baseline.
- [ ] **8.1.2: Finding Deduplication**: Implement cross-scan deduplication in `handle_findings` to ensure persistent vulnerabilities don't trigger repeat alerts on every scan.
- [ ] **8.1.3: Handler Registry Fix**: Refactor `Agent::trigger_event` to prevent handler loss when `std::mem::take` is used during event processing.

### **8.2: TUI Performance & Responsiveness**
- [ ] **8.2.1: Event Loop Optimization**: Reorder the main loop in `runner.rs` (`update() -> draw() -> poll()`) to reduce perceived latency between background task completion and UI refresh.
- [ ] **8.2.2: Async Channel Draining**: Update `App::update` to use `while let Ok(...)` for `progress_rx` and `result_rx` to prevent UI lag when high-volume scan data arrives.
- [ ] **8.2.3: Dynamic Constraints**: Replace hardcoded `visible_rows` (e.g., in `HistoryTab`) with dynamic calculations based on the active `Rect` height.

### **8.3: Architectural Cleanup**
- [ ] **8.3.1: Standardize History Tab**: Refactor `HistoryTab` to fit the `TabInput` trait more cleanly, removing the need for `Arc<Mutex>`-specific special cases in `App` and `ui.rs`.
- [ ] **8.3.2: Breadcrumb Consolidation**: Implement a shared `Breadcrumb` trait or helper to centralize breadcrumb logic and reduce the massive `match` block in `ui.rs`.
- [ ] **8.3.3: Theme Consistency**: Audit all TUI components to ensure 100% compliance with the `tc!` macro for color selection.

### **8.4: Dashboard & Reporting Enhancements**
- [ ] **8.4.1: Trend Visualization**: Add sparklines or mini-charts to the Dashboard using `LongitudinalMemory::analyze_temporal_patterns`.
- [ ] **8.4.2: Asset Status Overview**: Implement a high-level "Asset Health" summary in the Dashboard for a quick view of the security state across the entire portfolio.

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

---
