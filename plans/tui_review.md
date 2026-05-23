# TUI Architecture Review

**Date:** 2026-05-23  
**Reviewer:** Architecture Review  
**Document:** `architecture/tui.md`

---

## Executive Summary

The TUI architecture document accurately describes the implementation with 29 tabs, event-driven key handling, async task execution via `TaskRunner`, and session persistence. The codebase follows most documented patterns. However, 14 instances of `unwrap_or_default()` anti-pattern exist in TUI code, conflicting with the documented best practices.

---

## Verified Claims

### Core Components

| Claim | Implementation | Status |
|-------|----------------|--------|
| `App` struct central state container | `crates/slapper/src/tui/app/mod.rs:45` - holds all tabs, mode, overlays, theme | VERIFIED |
| `runner.rs` main event loop | `crates/slapper/src/tui/app/runner.rs:146` - uses crossterm/ratatui EventStream | VERIFIED |
| `KeyHandler` priority-based key processing | `crates/slapper/src/tui/app/key_handler.rs:17-43` - pending combos → overlays → global → mode | VERIFIED |
| `TabDispatcher` routes input to current tab | `crates/slapper/src/tui/app/dispatch.rs:11` - `new()` and `new_locked()` | VERIFIED |
| `state_update.rs` async task result handling | `crates/slapper/src/tui/app/state_update.rs:15-50` - `update()` drains channels | VERIFIED |
| `task_management.rs` maps tabs to TaskConfig | `crates/slapper/src/tui/app/task_management.rs:11-65` - `TabTaskConfigSource` trait | VERIFIED |
| `task_runtime.rs` task lifecycle | `crates/slapper/src/tui/app/task_runtime.rs:4-81` - spawn, stop, clear | VERIFIED |

### Tab Traits

| Claim | Implementation | Status |
|-------|----------------|--------|
| `TabState` - state, progress, reset, set_error | `crates/slapper/src/tui/tabs/mod.rs:864-872` - trait definition | VERIFIED |
| `TabInput` - handle_focus_next, handle_char, handle_enter | `crates/slapper/src/tui/tabs/mod.rs:883-921` - trait definition | VERIFIED |
| `TabRender` - render, render_overlays, breadcrumb | `crates/slapper/src/tui/tabs/mod.rs:874-880` - trait definition | VERIFIED |

### Theme System

| Claim | Implementation | Status |
|-------|----------------|--------|
| `ThemeManager` holds dark/light themes | `crates/slapper/src/tui/theme.rs:178-180` - `themes: FxHashMap<String, Theme>` | VERIFIED |
| `tc!` macro for theme colors | `crates/slapper/src/tui/theme.rs:263-268` - macro exports | VERIFIED |
| 30+ color fields | `crates/slapper/src/tui/theme.rs:23-52` - `ThemeColors` struct has 30 fields | VERIFIED |

### Session Management

| Claim | Implementation | Status |
|-------|----------------|--------|
| Auto-save every 30 seconds | `crates/slapper/src/tui/session.rs:45` - `auto_save_interval_secs: 30` | VERIFIED |
| Saves to JSON in `~/.slapper/sessions/` | `crates/slapper/src/tui/session.rs:54-56` - uses `directories::ProjectDirs` | VERIFIED |

### Entry Point

| Claim | Implementation | Status |
|-------|----------------|--------|
| No `--tui` flag - TUI launches when no subcommand | `crates/slapper/src/commands/handlers/mod.rs:155-163` - `handle_no_command()` checks terminal | VERIFIED |
| Calls `tui::run()` | `crates/slapper/src/commands/handlers/mod.rs:157` - `crate::tui::run(cli.config.clone())?` | VERIFIED |

### FxHashMap/FxHashSet Usage

| Claim | Implementation | Status |
|-------|----------------|--------|
| `app/mod.rs` - App.tabs, App.bookmarks | `crates/slapper/src/tui/app/mod.rs:113` - `bookmarks: FxHashSet<String>` | VERIFIED (tabs not present - was removed per bug fix) |
| `bookmarks.rs` | `crates/slapper/src/tui/app/bookmarks.rs:2` - uses `FxHashSet` | VERIFIED |
| `help_config.rs` - StaticHelpData.sections | `crates/slapper/src/tui/app/help_config.rs` - uses FxHashMap | VERIFIED |
| `help.rs` - HelpManager.sections | `crates/slapper/src/tui/help.rs` - uses FxHashMap | VERIFIED |
| `theme.rs` - ThemeManager.themes | `crates/slapper/src/tui/theme.rs:179` - `themes: FxHashMap<String, Theme>` | VERIFIED |
| `dashboard.rs` - PortfolioSnapshot.findings_by_severity | `crates/slapper/src/tui/tabs/dashboard.rs` - uses FxHashMap | VERIFIED |

### Architecture Diagram

The event loop flow matches implementation:
- EventStream → KeyHandler → App.handle_* methods → TabDispatcher → TabInput
- TaskRunner sends progress via `progress_tx` → `App::update_progress()` → tab.update_progress()
- Result via `result_tx` → `App::handle_result()` → tab.set_results()
- `needs_redraw = true` triggers Terminal.draw() → ui::draw()

---

## Discrepancies

### 1. Tabs Not Stored in HashMap
**Doc says:** `pub tabs: std::collections::HashMap<Tab, Box<dyn TabInput>>`  
**Actual:** Tabs are individual struct fields on `App` (e.g., `pub recon: tabs::ReconTab`, `pub fuzz: tabs::FuzzTab`)  
**Impact:** Low - design decision, not a bug. The dispatch pattern via `Tab::as_tab_input()` works correctly.  
**Reference:** `crates/slapper/src/tui/app/mod.rs:45-121`

### 2. State Management
**Doc says:** `pub type SharedHistory = Arc<Mutex<HistoryTab>>` in `state/`  
**Actual:** The type alias `SharedHistory` is defined in `crates/slapper/src/tui/state/mod.rs` but the actual usage pattern uses direct `Arc<Mutex<HistoryTab>>`  
**Impact:** None - behavior is correct

### 3. Tab Count Mismatch with Feature Flags
**Doc claims:** 29 tabs  
**Actual:** Tab enum has 29 variants (0-28), but actual visible tabs depend on feature flags  
**Verification:**
- Base tabs without features: 22 tabs (Recon, Load, ScanPorts, ScanEndpoints, Fingerprint, Fuzz, Waf, WafStress, Scan, Resume, Proxy, Packet, GraphQl, OAuth, Cluster, Stress, Report, Settings, History, Dashboard + 2 feature-gated)
- With all features enabled: 29 tabs
**Impact:** None - documentation is accurate for full feature build

---

## Bugs Found

### 1. Multiple `unwrap_or_default()` in TUI Codebase
**Severity:** Medium  
**Location:** Multiple files (14 total matches)

The architecture document explicitly lists "Silent Error Suppression in Workers" as a bug pattern to avoid (lines 223-236), but TUI code uses this anti-pattern:

| File | Line | Context |
|------|------|---------|
| `state_update.rs` | 145, 157 | `r.waf_name.clone().unwrap_or_default()` |
| `export.rs` | 331, 348 | `CsvExporter::export_ports(&ports).unwrap_or_default()` |
| `settings/main.rs` | 521 | `self.config.clone().unwrap_or_default()` |
| `key_handler.rs` | 325 | Command palette query clone |
| `dashboard.rs` | 324 | Portfolio snapshot field |
| `scrollable.rs` | 42 | Style unwrap_or_default |
| `ui.rs` | 223 | Help text unwrap |
| `integrations.rs` | 135, 149 | Integration config fields |
| `load.rs` | 155 | Load test results |
| `selector.rs` | 55 | Dropdown state |
| `palette.rs` | 74 | Command palette results |

**Recommendation:** Replace with explicit match and tracing at appropriate level

### 2. ScrollableText Scroll Offset Handling (False Positive)
**Note:** The architecture document shows this as a bug pattern:
```rust
// WRONG - usize::MAX when lines is empty
let scroll_offset = self.scroll_offset.min(self.lines.len().saturating_sub(1));
```

The actual implementation in `scrollable.rs:135-139` correctly handles this:
```rust
let scroll_offset = if self.lines.is_empty() {
    0
} else {
    self.scroll_offset.min(self.lines.len() - 1)
};
```

**Status:** NOT A BUG - the documented "WRONG" code is correctly avoided

### 3. Dispatcher Repeated Calls
**Location:** `crates/slapper/src/tui/app/mod.rs:371-382`

In `handle_enter()`, `dispatcher_mut()` is called 4 times:
```rust
self.dispatcher_mut().handle_enter();
self.mode = if self.dispatcher_mut().is_input_focused() {  // 2nd call
    InputMode::Insert
} else {
    InputMode::Normal
};

if self.dispatcher_mut().is_running() {  // 3rd call
    if let Some(task_config) = self.build_current_task() {
        self.spawn_task(Some(task_config));
    }
}
```

**Severity:** Low  
**Impact:** Performance - creates new dispatcher each call  
**Recommendation:** Cache the dispatcher result

### 4. TaskRunner `run()` Returns anyhow::Result but Discards
**Location:** `crates/slapper/src/tui/app/task_runtime.rs:69-78`

```rust
self.task_handle = Some(tokio::spawn(async move {
    match runner.run().await {
        Ok(_) => {}  // DISCARDED
        Err(e) => {
            let friendly_error = super::make_friendly_error(&e);
            tracing::error!("Task failed: {}", friendly_error);
            let _ = error_tx
                .send(workers::TaskResult::Error(friendly_error))
                .await;
        }
    }
}));
```

The `Ok(_)` result is silently discarded. While task success is communicated via `result_tx`, the actual result value is lost.

**Severity:** Low  
**Recommendation:** Log at trace level for debugging

---

## Improvement Opportunities

### High Priority

#### 1. Cache TabDispatcher in handle_enter
**File:** `crates/slapper/src/tui/app/mod.rs:371-382`  
**Impact:** Reduces dispatcher creation from 4x to 1x per Enter keypress  
**Effort:** Low - extract to local variable

#### 2. Replace unwrap_or_default() with Explicit Error Handling
**Files:** 14 instances across TUI  
**Impact:** Better error visibility, follows documented best practices  
**Effort:** Medium - need to determine appropriate logging level for each

### Medium Priority

#### 3. Tab Dispatcher Pattern Could Use Static Dispatch
**Current:** `Tab::as_tab_input()` uses dynamic dispatch via `&mut dyn TabInput`  
**Potential:** The enum has 29 variants - could consider static dispatch for performance-critical paths  
**Impact:** Performance gain possible in key handling hot path  
**Effort:** High - significant refactoring

#### 4. SessionManager Theme Restore Not Implemented
**Observation:** `session.rs` captures `theme_name: "dark".to_string()` but doesn't actually restore theme  
**Reference:** `crates/slapper/src/tui/session.rs:153`

### Low Priority

#### 5. Missing Tests for Overlay Precedence
**Observation:** UI has multiple overlays but no tests verify precedence order  
**Recommendation:** Add tests for `topmost_overlay()` precedence

#### 6. Command Palette Results Could Use FxHashMap
**File:** `crates/slapper/src/tui/components/palette.rs`  
**Current:** Uses `Vec` for command lookup  
**Potential:** Pre-filter with FxHashMap for faster search

---

## Priority Summary

| Finding | Priority | Type |
|---------|----------|------|
| Cache TabDispatcher in handle_enter | High | Performance |
| Replace unwrap_or_default() instances | High | Code Quality |
| TabDispatcher static dispatch consideration | Medium | Performance |
| Session theme restore implementation | Medium | Feature Gap |
| Overlay precedence tests | Low | Test Coverage |
| Command palette optimization | Low | Performance |

---

## Conclusion

The TUI architecture is well-documented and largely matches implementation. The codebase demonstrates good practices (division-by-zero guards in progress calculations, FxHashMap usage, proper scroll offset handling). Main opportunities for improvement are reducing dispatcher recreation and addressing the `unwrap_or_default()` instances that contradict documented best practices.

