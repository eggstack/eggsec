---
name: tui_improvements
description: TUI (Terminal UI) patterns and recent improvements in Slapper
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

This skill documents TUI patterns and recent improvements in Slapper's terminal UI implementation, updated for Phase 11 completion.

## Phase 11 Updates (2026-04-30)

### Completed Items

- **11.1.1**: Total theming migration - 400+ hardcoded `Color::*` usages replaced with `tc!()` macro
- **11.1.2**: Improved InputField scrolling - viewport approach with proper edge handling
- **11.1.3**: Unified Selector behavior - consistent theme styling
- **11.2.1**: FocusArea standardization - 13 tabs migrated to FocusArea enum pattern
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

**Key bindings** (verified 2026-04-30):
- `n` / `N` or `p` - next/prev tab (cycles through all available tabs)
- `Shift+H` / `Shift+L` - previous/next tab
- `1-9`, `0` - direct tab selection (limited to single digit)
- Mouse click on tab - selects that tab if visible

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

## Verification Commands

```bash
# Verify theme migration (should find 0 Color::* usages in TUI)
grep -r "Color::" crates/slapper/src/tui/tabs/*.rs crates/slapper/src/tui/components/*.rs | wc -l

# Run TUI tests
cargo test --lib -p slapper -- tui

# Run clippy on TUI
cargo clippy --lib -p slapper -- -A clippy::all -W clippy::pedantic
```

## Related Skills

- `tui_theme_system` - Theme system and color mapping
- `performance_patterns` - Performance optimization patterns
- `security_fix_patterns` - Security vulnerability patterns