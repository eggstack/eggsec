---
name: tui_improvements
description: TUI (Terminal UI) patterns and recent improvements in Eggsec
triggers:
  - TUI
  - terminal
  - ratatui
  - input field
  - focus area
  - auto-insert mode
  - input scrolling
metadata:
  category: code_quality
  tools: [tui]
  scope: implementation
---

## Overview

This skill documents TUI patterns and recent improvements in Eggsec's terminal UI implementation, updated for Phase 15 completion.

## Phase 15: WAF Tab Checkbox Focus Fix (2026-05-05) ✅

### Completed Items

- **15.1**: WAF Tab Checkbox Focus - Added `focused_checkbox_index` to properly track which checkbox is focused
- Settings tab already uses proper `InputField::render()` - no changes needed

## Phase 11 Updates (2026-04-30)

### Completed Items

- **11.1.1**: Total theming migration - 400+ hardcoded `Color::*` usages replaced with `tc!()` macro
- **11.1.2**: Improved InputField scrolling - viewport approach with proper edge handling
- **11.1.3**: Unified Selector behavior - consistent theme styling
- **11.2.1**: FocusArea standardization - 29 tabs migrated to FocusArea enum pattern
- **11.2.2**: Consistent error reporting - 7 tabs now have `error_message` field
- **11.3.1**: Auto-insert mode - Tab/Shift+Tab auto-switches to Insert mode when focusing inputs

## Phase 12 Updates (2026-04-30)

### Completed Items

- **12.1**: Centralized Tab Indexing - `Tab::visible_index()`, `Tab::from_visible_index()`, `Tab::stable_id()`, `Tab::from_stable_id()`
- **12.2**: TabWindow helper - `TabWindow::for_width()` pure function for deterministic window calculation
- **12.3**: Fixed keyboard navigation - `adjust_tab_scroll()` now uses TabWindow instead of hardcoded `visible_count = 10`
- **12.4**: Fixed mouse hit-testing - uses TabWindow for accurate click-to-tab mapping
- **12.5**: Session persistence - stable IDs with backward compatibility for legacy numeric indexes
- **12.7**: Popup layout hardening - `centered_rect()` now clamps to terminal area
- **12.8**: Added 10 focused tests for tab indexing and window calculation

### TabIndexing System

The TUI now uses a unified tab indexing system:

```rust
// In tabs/mod.rs
pub fn visible_index(&self) -> Option<usize>  // Position in Tab::all()
pub fn stable_id(&self) -> &'static str        // "recon", "dashboard", etc.
pub fn from_stable_id(id: &str) -> Option<Tab> // Feature-gated safe lookup

// TabWindow helper for rendering and navigation
pub struct TabWindow {
    pub start: usize,           // Start index in Tab::all()
    pub end: usize,             // End index in Tab::all()
    pub selected_visible: usize, // Selected index within visible window
    pub max_visible: usize,
    pub total_tabs: usize,
    pub has_prev: bool,
    pub has_next: bool,
}

impl TabWindow {
    pub fn for_width(term_width: u16, current_tab: Tab, previous_offset: u16) -> Self;
    pub fn range_text(&self) -> String;  // "[1-7/20]" style
}
```

**Anti-patterns to avoid**:
- Don't use `tab as usize` for indexing (enum discriminants != visible indexes)
- Don't use `Tab::all().len()` as visible count
- Don't divide tab area by total tab count for mouse hit-testing

### Keyboard Navigation

**Key bindings** (verified 2026-05-25):
- `n` / `Shift+L` - next tab
- `N` / `p` / `Shift+H` - previous tab
- `1-9`, `0` - direct tab selection (limited to single digit)
- Mouse click on tab - selects that tab if visible

## Phase 13 Updates (2026-04-30)

### Completed Items

- **13.1**: Render-aware capacity - `TabWindow::for_width` uses actual label widths, not fixed minimum
- **13.2**: TabSpan helper - `visible_tab_spans()` provides accurate mouse hit-testing
- **13.3**: Fixed tab labels - tabs 11+ no longer show implied keyboard shortcuts
- **13.4**: Edge-based navigation - `handle_left/handle_right` use `is_at_edge()` checks
- **13.5**: Render tests - 9 tests covering various terminal sizes (30, 40, 60, 80, 120)
- **13.6**: Overlay hardening - `visible_results_height()` bounds by actual result count
- **13.7**: Status bar audit - already uses Paragraph widgets with proper overflow handling

### Render-Aware Tab Capacity

```rust
// TabWindow now uses greedy algorithm with actual tab label widths
let tab_widths: Vec<usize> = all_tabs.iter().map(|t| t.title().len()).collect();
let mut max_visible = 0;
let mut cum_width = 0;
for (i, &w) in tab_widths.iter().enumerate() {
    cum_width += w;
    if cum_width > available_width && i > 0 {
        break;
    }
    max_visible = i + 1;
}
```

### TabSpan for Mouse Hit-Testing

```rust
pub struct TabSpan {
    pub tab: Tab,
    pub global_index: usize,
    pub x_start: u16,
    pub x_end: u16,
}

// Mouse click handling
for span in spans {
    if click_x >= span.x_start && click_x < span.x_end {
        app.current_tab = span.tab;
        break;
    }
}
```

### Tab Label Shortcuts

Only tabs 1-10 show numeric shortcuts:
```rust
"[1] Recon", "[2] Load", ..., "[9] Scan", "[0] Resume",  // 10 tabs with shortcuts
"Proxy", "Packet", "GraphQL", ...                           // No shortcuts for tabs 11+
```

### Navigation Changes

`handle_left()` and `handle_right()` now stop at edge instead of switching tabs:
```rust
pub fn handle_left(&mut self) {
    // ...
    if self.dispatcher_mut().is_at_left_edge() {
        return;  // Don't fall back to prev_tab
    }
    let _ = self.dispatcher_mut().handle_left();
}
```

### Session Persistence

Session state now uses stable string IDs instead of numeric indexes:

```rust
pub struct SessionState {
    pub current_tab_id: Option<String>,  // e.g., Some("dashboard")
    pub bookmarks: Vec<String>,           // e.g., vec!["recon", "settings"]
    pub legacy_current_tab: Option<usize>, // For backward compat
    pub legacy_bookmarks: Vec<usize>,     // For backward compat
}
```

## TUI Architecture

### Component Hierarchy

```
App
├── TabDispatcher (handles focus and input routing)
│   └── Tab implementations (ReconTab, AuthTab, etc.)
├── Components
│   ├── InputField - single input with validation and viewport scrolling
│   ├── InputGroup - collection of InputField with focus management
│   ├── ScrollableText - scrollable text display
│   ├── ProgressGauge - progress indicator with theme colors
│   ├── Selector - dropdown selector with theme-consistent styling
│   └── Checkbox - toggle option
└── Theme system (tc!() macro)
```

### FocusArea Pattern

All tabs should implement an enum-based focus area system:

**Standard Pattern** (from `tabs/auth.rs`):
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AuthFocusArea {
    Target,
    Username,
    Password,
    Results,
}

pub struct AuthTab {
    pub inputs: InputGroup,
    pub state: AppState,
    pub focus_area: AuthFocusArea,
    pub error_message: Option<String>,
}

impl TabInput for AuthTab {
    fn handle_up(&mut self) {
        self.focus_area = match self.focus_area {
            AuthFocusArea::Results => AuthFocusArea::Password,
            AuthFocusArea::Password => AuthFocusArea::Username,
            AuthFocusArea::Username => AuthFocusArea::Target,
            AuthFocusArea::Target => {
                self.inputs.focus_prev();
                if !self.inputs.is_focused() {
                    self.inputs.focus(self.inputs.fields.len() - 1);
                }
                AuthFocusArea::Target
            }
        };
    }

    fn handle_down(&mut self) {
        self.focus_area = match self.focus_area {
            AuthFocusArea::Target => AuthFocusArea::Username,
            AuthFocusArea::Username => AuthFocusArea::Password,
            AuthFocusArea::Password => AuthFocusArea::Results,
            AuthFocusArea::Results => AuthFocusArea::Results,
        };
    }

    fn set_error(&mut self, msg: String) {
        self.state = AppState::Error(msg.clone());
        self.error_message = Some(msg);
    }
}
```

### Error Handling Pattern

```rust
pub struct SomeTab {
    pub state: AppState,
    pub error_message: Option<String>,
}

impl SomeTab {
    fn set_error(&mut self, msg: String) {
        self.state = AppState::Error(msg.clone());
        self.error_message = Some(msg);
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.error_message = None;
    }
}
```

### InputField Viewport Scrolling

The InputField component now properly handles scrolling when the text exceeds the visible width:

```rust
// Viewport calculation in InputField::render
let display_value = if let Some(w) = self.width {
    let available = (w as usize).saturating_sub(2);
    let char_count = self.value.chars().count();
    if char_count > available {
        // Center viewport on cursor with edge clamping
        let start = if cursor_char_pos <= available / 2 {
            0
        } else if cursor_char_pos >= char_count - available / 2 {
            (char_count.saturating_sub(available)).max(0)
        } else {
            cursor_char_pos.saturating_sub(available / 2)
        };
        // Show "..." prefix only if scrolled past start
        // Show "..." suffix only if more content after visible
    }
}
```

### Auto-Insert Mode

As of Phase 11, the TUI automatically switches to Insert mode when an input is focused:

```rust
// In app/mod.rs - handle_focus_next
pub fn handle_focus_next(&mut self) {
    // ... existing logic ...
    self.dispatcher_mut().handle_focus_next();
    if self.dispatcher_mut().is_input_focused() {
        self.mode = InputMode::Insert;
    } else {
        self.mode = InputMode::Normal;
    }
}
```

Users can still manually toggle with `i` in Normal mode.

### Keyboard Navigation

**Key bindings** (verified 2026-04-30):
- `h` / `l` or `←` / `→` - move cursor left/right within input
- `j` / `k` or `↓` / `↑` - navigate up/down lists
- `n` / `p` or Tab/Shift-Tab - next/prev tab
- `gg` / `G` - go to top/bottom
- `Ctrl+T` - toggle theme
- `Ctrl+P` - command palette
- `Space` - help overlay
- `i` - enter Insert mode (from Normal mode)
- `Esc` - return to Normal mode

### Theme System

Use the `tc!()` macro for all colors - see `tui_theme_system.md` skill for details.

## Phase 14 Updates (2026-05-05)

### Completed Items

#### 1. WAF Tab Checkbox Focus Fix ✅
- Added `focused_checkbox_index` to track which checkbox is focused
- Fixed render logic: `checkbox.focused = self.focus_area == WafFocusArea::Techniques && i == self.focused_checkbox_index`
- Updated navigation methods to manage `focused_checkbox_index` instead of modifying Checkbox.focused directly
- Pattern: Use dedicated index field when managing multiple checkboxes in a FocusArea

#### 2. Auth Tab Component Standardization ✅
- Replaced manual text construction with proper `InputField::render()` calls
- Fixed layout constraints to properly accommodate 3 input fields
- Error now displays in separate block

#### 3. Integrations Tab Navigation Fix ✅
- `handle_focus_next` now routes Config/Issue based on `current_mode`
- `handle_focus_prev` now routes Results to Config or Issue based on `current_mode`
- `get_config()` now returns actual integration configuration based on selected tracker

#### 4. NSE Tab Redundancy Cleanup ✅
- Removed duplicate methods (handle_word_forward/backward, handle_home/end, page_up/down, handle_top/bottom)
- These are properly handled by the TabInput trait implementation
- Added `start()` method to match WafTab pattern
- Updated `handle_enter` to trigger start/stop based on state

#### 5. Fingerprint Tab Scrolling Fix ✅
- `handle_up/handle_down` now properly handle Results focus area
- `handle_focus_next/prev` now properly switch between Inputs and Results focus areas

#### 6. History Tab Keybindings ✅
- 'd' or 'D': delete selected history entry
- 'c' or 'C' (in List focus): clear all history entries

#### 7. Storage Tab Edge Detection Fix ✅
- `is_at_left_edge/is_at_right_edge` now use `config_inputs.is_at_left_edge()` / `config_inputs.is_at_right_edge()`
- Previously hardcoded to use field[0] regardless of focused field

#### 8. TaskResult Integration (Storage) ✅
- `state_update.rs` now uses `storage.set_scans()` and `storage.set_findings()` instead of direct field assignment
- Ensures proper state management and UI updates

### Checkbox Focus Pattern

When managing multiple checkboxes in a focus area, use a dedicated index field:

```rust
pub struct WafTab {
    pub technique_checkboxes: Vec<Checkbox>,
    pub focused_checkbox_index: usize,
    // ...
}

impl TabInput for WafTab {
    fn handle_focus_next(&mut self) {
        self.focused_checkbox_index = 0;
        // ...
    }

    fn handle_left(&mut self) -> bool {
        if self.focused_checkbox_index == 0 {
            false
        } else {
            self.focused_checkbox_index = self.focused_checkbox_index.saturating_sub(1);
            true
        }
    }
}

// In render:
for (i, cb) in self.technique_checkboxes.iter().enumerate() {
    let mut checkbox = cb.clone();
    checkbox.focused = self.focus_area == WafFocusArea::Techniques && i == self.focused_checkbox_index;
    checkbox.render(f, config_chunks[2 + i]);
}
```

## Verification Commands

```bash
# Verify theme migration (should find 0 Color::* usages in TUI)
grep -r "Color::" crates/eggsec/src/tui/tabs/*.rs crates/eggsec/src/tui/components/*.rs | wc -l

# Run TUI tests
cargo test --lib -p eggsec -- tui

# Run clippy on TUI
cargo clippy --lib -p eggsec -- -A clippy::all -W clippy::pedantic
```

## Related Skills

- `tui_theme_system` - Theme system and color mapping
- `performance_patterns` - Performance optimization patterns
- `security_fix_patterns` - Security vulnerability patterns
