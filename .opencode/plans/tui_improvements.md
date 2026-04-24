# TUI Deep Dive - Implementation Plan

**Date**: 2026-04-23
**Status**: PLANNING
**Review Source**: Deep dive analysis via 10 subagents investigating all TUI subsystems

---

## Overview

This plan documents TUI improvements identified during a comprehensive review covering:
- 60+ TUI source files
- 39 tab variants (21 always present, 18 feature-gated)
- 29 tab fields in App struct
- Key files: `app/mod.rs` (967 lines), `app/runner.rs` (441 lines), `ui.rs` (818 lines), `help.rs` (772 lines)

### Investigation Summary

| Area | Status | Priority | Effort |
|------|--------|----------|--------|
| Global Search | ✅ Researched | High | Medium |
| Copy/Paste/Clipboard | ✅ Researched | High | Medium |
| Keyboard Macros | ✅ Researched | Medium | Medium |
| Tab Customization | ✅ Researched | Medium | Medium |
| Session State Export | ✅ Researched | Medium | Medium |
| Pause/Resume | ✅ Researched | Medium | Medium |
| Multi-Target Mode | ✅ Researched | Low | High |
| UI Settings Persistence | ✅ Researched | Medium | Medium |
| Code Quality (dispatcher) | ✅ Researched | High | Medium |
| Help Externalization | ✅ Researched | Low | Low |

---

## Phase 1: High Priority UX Improvements

### 1.1: Global Search (`Ctrl+F`)

**Goal**: Enable searching across all tabs and results, not just History.

**Current State**:
- `/` key triggers search but only works in History tab
- Search implementation at `tui/app/navigation.rs:46-81`
- Simple substring matching in `tui/tabs/history.rs:170-186`

**Implementation**:

| Step | File | Change |
|------|------|--------|
| 1 | `tui/app/mod.rs` | Add `global_search: GlobalSearchState` field |
| 2 | `tui/app/runner.rs` | Add `Ctrl+F` keybinding at line ~160 |
| 3 | `tui/search.rs` | **NEW** - `GlobalSearchState`, `SearchResult` types |
| 4 | `tui/ui.rs` | Add `draw_global_search_overlay()` (similar to command palette) |
| 5 | `tui/tabs/mod.rs` | Add `searchable(&self, query: &str) -> Vec<SearchResult>` to traits |

**Key Bindings**:
- `Ctrl+F` - Open global search overlay
- `Esc` - Close search
- `Enter` - Navigate to selected result
- `n/N` - Next/previous match (vim-style)

**Searchable Content**:
1. History entries (already searchable)
2. Current tab results (per-tab implementation)
3. Settings/commands (Help integration)
4. Findings across tabs

**Estimated Effort**: 3-4 days

---

### 1.2: Clipboard/Copy-Paste Support

**Goal**: Enable copying results, URLs, findings to system clipboard.

**Current State**:
- No clipboard integration exists
- Results can only be viewed, not copied

**Implementation**:

| Step | File | Change |
|------|------|--------|
| 1 | `Cargo.toml` | Add `arboard = "3.4"` dependency |
| 2 | `tui/utils/clipboard.rs` | **NEW** - `ClipboardManager` wrapper |
| 3 | `tui/components/selection.rs` | **NEW** - `Selection` state and highlight |
| 4 | `tui/app/input.rs` | Add `Visual` and `VisualLine` variants to `InputMode` |
| 5 | `tui/app/runner.rs` | Add `v`, `V`, `y`, `p` keybindings |
| 6 | `tui/app/mod.rs` | Add `clipboard: ClipboardManager`, `selection: Option<Selection>` |
| 7 | `tui/components/scrollable.rs` | Add selection highlight rendering |
| 8 | `tui/help.rs` | Document new shortcuts |

**Key Bindings**:
- `v` - Enter visual mode (character selection)
- `V` - Enter visual line mode (line selection)
- `y` - Yank (copy) selection
- `yy` - Yank current line
- `p` - Paste after cursor
- `P` - Paste before cursor

**Mouse Integration**:
- Left drag - Create selection
- Double-click - Select word
- Triple-click - Select line
- Right-click - Paste

**Estimated Effort**: 4-5 days

---

### 1.3: Progress Pause/Resume (`Ctrl+Z` / `Ctrl+Y`)

**Goal**: Allow pausing and resuming long-running scans.

**Current State**:
- Help already shows `Space` for "Pause/Resume" but has no implementation
- `App::stop()` at `tui/app/mod.rs:422-427` does hard abort via `handle.abort()`
- No pause state exists

**Implementation**:

| Step | File | Change |
|------|------|--------|
| 1 | `tui/workers/runner.rs` | Add `pause_token: Option<Arc<PauseToken>>` to `TaskRunner` |
| 2 | `tui/workers/pause.rs` | **NEW** - `PauseToken` struct with `Arc<AtomicBool>` + `Notify` |
| 3 | `tui/app/mod.rs` | Add `pause()`, `resume()`, `is_paused()` methods |
| 4 | `tui/app/runner.rs` | Add `Ctrl+Z` (pause) and `Ctrl+Y` (resume) handlers |
| 5 | `tui/tabs/mod.rs` | Add `Paused` variant to `AppState` or `is_paused()` method |
| 6 | `tui/workers/scanner.rs` | Add pause checks between ports |
| 7 | `tui/workers/fuzzer.rs` | Add pause checks between payloads |
| 8 | `tui/ui.rs` | Show `[PAUSED]` indicator in status bar |

**Pause Capability by Worker**:

| Worker | Pause Support | Notes |
|--------|--------------|-------|
| Port Scan | High | Already iterates, can pause between ports |
| Endpoint Scan | High | Similar to port scan |
| Load Test | Medium | Duration-based, could pause timer |
| Fuzz | Medium | `FuzzEngine` needs session pause support |
| Recon | Low | Atomic with retries |
| Pipeline | High | Stage-based, natural pause points |

**Estimated Effort**: 4-5 days

---

## Phase 2: Medium Priority UX Improvements

### 2.1: Keyboard Macros (`q` record, `@` replay)

**Goal**: Record and replay key sequences for repetitive workflows.

**Current State**:
- No macro system exists
- Key handling in `tui/app/runner.rs:131-441`

**Implementation**:

| Step | File | Change |
|------|------|--------|
| 1 | `tui/app/macro.rs` | **NEW** - `KeyboardMacro`, `MacroState` enum |
| 2 | `tui/app/mod.rs` | Add `macros: HashMap<char, KeyboardMacro>`, `macro_state: MacroState` |
| 3 | `tui/app/runner.rs` | Intercept `q` for recording, `@` for replay |
| 4 | `tui/help.rs` | Document macro shortcuts |

**Key Bindings**:
- `q<char>` - Start recording to register (e.g., `qq`, `qa`)
- `Esc` - Stop recording
- `@<char>` - Replay macro (e.g., `@q`, `@a`)
- `@@` - Replay last macro

**Registers**: 35 named registers (a-z lowercase, A-Z uppercase, 0-9)

**Estimated Effort**: 3-4 days

---

### 2.2: Tab Customization (reorder, hide, groups)

**Goal**: Allow users to customize tab visibility and order.

**Current State**:
- 39 tabs with fixed order at `tui/tabs/mod.rs:113-160`
- Tab bar rendered at `tui/ui.rs:255-271`
- Mouse click handling at `tui/app/runner.rs:120-127`

**Implementation**:

| Step | File | Change |
|------|------|--------|
| 1 | `tui/app/tab_config.rs` | **NEW** - `TabConfig` struct |
| 2 | `tui/tabs/mod.rs` | Add `Tab::id()` and `Tab::from_id()` methods |
| 3 | `tui/app/mod.rs` | Add `tab_config: TabConfig` field |
| 4 | `tui/ui.rs` | Update `draw_tabs()` to respect visible tabs |
| 5 | `tui/app/navigation.rs` | Update `next_tab()`/`prev_tab()` for filtered lists |
| 6 | `tui/app/runner.rs` | Update numeric key handling to use visible tabs |
| 7 | `config/settings.rs` | Add `ui.tabs` section to `SlapperConfig` |

**Configuration**:
```toml
[ui.tabs]
# Hide rarely-used tabs by default
hidden = ["Cluster", "Plan", "Ci", "Agent", "Serve", "Mcp", "Notify", "Sbom", "Icmp", "Traceroute"]

# Custom order (optional)
# order = ["Recon", "ScanPorts", "ScanEndpoints", ...]

# Tab groups for visual organization
[ui.tabs.groups]
recon = ["Recon", "Fingerprint"]
scan = ["ScanPorts", "ScanEndpoints", "Scan"]
```

**Estimated Effort**: 4-5 days

---

### 2.3: Session State Export (`Ctrl+S`)

**Goal**: Save mid-scan session state for later resume.

**Current State**:
- `HistoryTab` has `export()` method at `tui/tabs/history.rs:36-48`
- `PipelineSession` shows serialization pattern at `crates/slapper/src/pipeline/session.rs`
- `SettingsTab` save/load at `tui/tabs/settings.rs:210-231`

**Implementation**:

| Step | File | Change |
|------|------|--------|
| 1 | `tui/session.rs` | **NEW** - `SessionSnapshot`, `TabStateSnapshot` enums |
| 2 | `tui/app/mod.rs` | Add `export_session()` and `import_session()` methods |
| 3 | `tui/app/runner.rs` | Add `Ctrl+S` handler |
| 4 | `tui/app/command.rs` | Add `save-session` and `load-session` commands |
| 5 | `tui/tabs/resume.rs` | Enhance to handle full session restore |

**Session Snapshot Structure**:
```rust
pub struct SessionSnapshot {
    pub version: u32,
    pub timestamp: String,
    pub current_tab: String,
    pub tab_states: HashMap<String, TabStateSnapshot>,
    pub http_options: GlobalHttpOptions,
}

pub enum TabStateSnapshot {
    Recon { target: String, options: ReconOptions, ... },
    Fuzz { target_url: String, payload_type: String, ... },
    // ... variants for each tab
}
```

**Estimated Effort**: 3-4 days

---

### 2.4: UI Settings Persistence

**Goal**: Persist color themes, layout density, default tab.

**Current State**:
- All colors hardcoded inline throughout `tui/ui.rs` and `tui/components/*.rs`
- No theme system exists
- Default tab hardcoded at `tui/app/mod.rs:162`

**Implementation**:

| Step | File | Change |
|------|------|--------|
| 1 | `config/ui_settings.rs` | **NEW** - `UiSettings`, `UiTheme`, `LayoutDensity` |
| 2 | `tui/theme.rs` | **NEW** - `Theme` struct with preset themes |
| 3 | `config/settings.rs` | Add `ui: UiSettings` field to `SlapperConfig` |
| 4 | `tui/app/mod.rs` | Add `ui_settings: UiSettings` field |
| 5 | `tui/app/runner.rs` | Load UI settings on startup |
| 6 | `tui/ui.rs` | Replace hardcoded colors with theme values |
| 7 | `tui/components/*.rs` | Replace hardcoded colors with theme values |
| 8 | `tui/tabs/settings.rs` | Add UI settings section to settings tab |

**Preset Themes**:
- Dark (current default)
- Light
- Monokai
- Dracula
- Nord

**Settings**:
```toml
[ui]
theme = "dark"           # dark, light, monokai, dracula, nord
density = "comfortable"  # compact, comfortable, spacious
default_tab = "Recon"
page_size = 10
```

**Estimated Effort**: 5-6 days

---

## Phase 3: Lower Priority Improvements

### 3.1: Multi-Target Mode

**Goal**: Run scans against multiple targets with queue display.

**Current State**:
- `FuzzTab::targets()` and `ScanEndpointsTab::targets()` already split comma/newline
- But `is_multi_target()` exists and returns true - not implemented

**Implementation Effort**: HIGH (requires new tab + queue system)

**Recommendation**: Defer to Phase 4 unless high demand.

---

### 3.2: Help Content Externalization

**Goal**: Move help content to external YAML/Markdown files.

**Current State**:
- 772 lines in `tui/help.rs` with embedded content
- `slapper_skills/` uses YAML frontmatter + Markdown pattern

**Implementation**:

| Step | File | Change |
|------|------|--------|
| 1 | `tui/help_loader.rs` | **NEW** - `HelpLoader` similar to `SkillLoader` |
| 2 | `slapper_help/*.md` | **NEW** - Ship default help files |
| 3 | `tui/help.rs` | Modify `HelpContent::default()` to try external files first |

**Estimated Effort**: 2-3 days

---

## Phase 4: Code Quality (Enabler)

### 4.1: Dispatcher Match Arm Refactoring

**Goal**: Eliminate 1,500+ lines of repetitive match statements.

**Current Problem**:
- 8 locations with 40+ arm match statements
- `dispatcher_mut()` - 76 arms
- `as_tab_state()` - 76 arms
- `as_tab_render()` - 76 arms
- etc.

**Recommended Solution**: `enum_dispatch` crate

```rust
// Add to Cargo.toml
enum_dispatch = "0.3"

// In tui/tabs/mod.rs
#[enum_dispatch]
enum TabStateVariant {
    ReconTab,
    LoadTab,
    // ... all 39 variants
}

#[enum_dispatch(TabStateVariant)]
trait TabState {
    fn state(&self) -> AppState;
    fn progress(&self) -> f64;
    fn reset(&mut self);
}
```

**Benefits**:
- Eliminates 5 dispatch locations completely
- 10x faster than `dyn Trait` (benchmarks)
- Native `#[cfg(...)]` feature gate support
- No runtime dependency (compile-time macro)

**Files to Change**:
1. `Cargo.toml` - Add dependency
2. `tui/tabs/mod.rs` - Create variant enums, apply `#[enum_dispatch]`
3. `tui/app/mod.rs` - Update `dispatcher_mut()`, remove `as_tab_*` calls
4. `tui/app/state_update.rs` - Update `handle_result()`, `update_progress()`
5. `tui/app/task_management.rs` - Update `TaskBuilder` impls

**Estimated Effort**: 4-5 days

---

## Implementation Order Recommendation

Based on UX impact and dependencies:

| Order | Feature | Phase | Days | Dependency |
|-------|---------|-------|------|------------|
| 1 | Global Search | Phase 1 | 3-4 | None |
| 2 | Clipboard | Phase 1 | 4-5 | None |
| 3 | Pause/Resume | Phase 1 | 4-5 | None |
| 4 | Keyboard Macros | Phase 2 | 3-4 | None |
| 5 | Tab Customization | Phase 2 | 4-5 | None |
| 6 | Session Export | Phase 2 | 3-4 | None |
| 7 | UI Settings | Phase 2 | 5-6 | None |
| 8 | Help Externalization | Phase 3 | 2-3 | None |
| 9 | Multi-Target | Phase 3 | TBD | Tab Customization |
| 10 | Dispatcher Refactor | Phase 4 | 4-5 | None |

**Total Estimated Effort**: 32-44 days

---

## Risk Factors

1. **Feature-gated tabs** complicate every change - 18 tabs have `#[cfg(...)]` gates
2. **Tab trait system** is deeply integrated - changes ripple through many files
3. **Worker pause support** varies - some workers may not support true pause
4. **Theme system** requires changing 60+ files with hardcoded colors

## Mitigation Strategies

1. **Incremental implementation** - Each feature is self-contained
2. **Feature flag testing** - Build with `--all-features` regularly
3. **Fallback modes** - Workers that can't pause still support stop
4. **Scripted color replacement** - Use `sed` for bulk color refactoring

---

## Files to Create

| File | Purpose |
|------|---------|
| `tui/search.rs` | Global search state |
| `tui/utils/clipboard.rs` | Clipboard wrapper |
| `tui/components/selection.rs` | Selection state |
| `tui/workers/pause.rs` | PauseToken implementation |
| `tui/app/macro.rs` | Keyboard macro state |
| `tui/app/tab_config.rs` | Tab customization config |
| `tui/session.rs` | Session snapshot types |
| `tui/theme.rs` | Theme definitions |
| `config/ui_settings.rs` | UI settings config |
| `tui/help_loader.rs` | External help loading |
| `slapper_help/*.md` | Default help content files |

---

## Key Technical Decisions Made

1. **Clipboard**: Use `arboard` (pure Rust, no C deps, cross-platform)
2. **Theme**: TOML config + `Theme` struct with preset palettes
3. **Dispatch refactor**: Use `enum_dispatch` crate (compile-time, no runtime cost)
4. **Search**: Overlay popup pattern (like command palette)
5. **Macros**: Register-based like vim (35 registers)

---

*Last updated: 2026-04-23*
*Reviewer: Claude Code (subagent investigation)*
