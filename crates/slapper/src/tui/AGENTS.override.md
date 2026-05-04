# TUI Module Override

Specialized guidance for the terminal UI module.

## Module Structure

```
crates/slapper/src/tui/
├── app/          # App state, event loop, command handling
│   ├── mod.rs           # App struct, notifications, helpers
│   ├── runner.rs        # Event loop, input handling
│   ├── key_handler.rs   # Key handling methods (extracted from mod.rs)
│   ├── state_update.rs  # Background task handling, result dispatch
│   ├── notifications.rs # Notification and NotificationSeverity types
│   ├── bookmarks.rs    # Bookmark helper functions
│   ├── confirmation.rs  # PendingAction enum
│   ├── help_config.rs   # Static help content
│   ├── navigation.rs   # Tab navigation, scrolling
│   ├── command.rs      # Command palette commands
│   ├── export.rs       # Export functionality
│   └── ...
├── tabs/         # Individual tab implementations
│   ├── mod.rs          # Tab enum, TabState/TabInput/TabRender traits
│   ├── dashboard.rs    # Dashboard tab
│   ├── fuzz.rs         # Fuzz tab
│   └── ...
├── components/   # Reusable UI components
│   ├── input.rs         # InputField with focus colors
│   ├── selector.rs      # Selector dropdown
│   ├── popup.rs         # Popup overlays
│   ├── palette.rs       # Command palette
│   ├── help_bar.rs      # Help bar component
│   └── ...
├── theme.rs      # Theme system (tc! macro)
├── search.rs     # Global search
└── ui.rs         # Main rendering, status bar with mode indicator
```

## Event Loop Order

`runner.rs` follows `update() -> draw() -> poll()` order:
- `update()` processes background task results first
- `draw()` renders only if `needs_redraw` is set
- `poll()` waits for user input with 100ms timeout

## Quick Switch Panel

Ctrl+G shows bookmarked tabs with fuzzy search:

```rust
// Toggle quick switch
pub fn toggle_quick_switch(&mut self) {
    if self.is_any_overlay_active() {
        return;
    }
    self.show_quick_switch = true;
    self.quick_switch_query.clear();
    self.quick_switch_selected = 0;
    self.needs_redraw = true;
}

// Get filtered bookmarked tabs
pub fn get_quick_switch_results(&self) -> Vec<&'static Tab> {
    let query = self.quick_switch_query.to_lowercase();
    Tab::all().iter()
        .filter(|tab| self.bookmarks.contains(&tab.stable_id().to_string()))
        .filter(|tab| {
            if query.is_empty() {
                true
            } else {
                tab.title().to_lowercase().contains(&query) ||
                tab.stable_id().contains(&query)
            }
        })
        .collect()
}
```

## Mode Indicator

Status bar (leftmost section) shows current input mode as a colored badge:
- **NORMAL** shown in green (`tc!(mode_normal)`) when in Normal mode
- **INSERT** shown in yellow/red (`tc!(mode_insert)`) when in Insert mode

Theme colors defined in `ThemeColors` struct:
```rust
pub struct ThemeColors {
    // ...
    pub mode_normal: Color,
    pub mode_insert: Color,
}
```

Render in ui.rs `draw_status_bar()`:
```rust
let mode_text = match app.mode {
    super::InputMode::Normal => "NORMAL",
    super::InputMode::Insert => "INSERT",
};
let mode_color = match app.mode {
    super::InputMode::Normal => tc!(mode_normal),
    super::InputMode::Insert => tc!(mode_insert),
};
```

`App::update` drains ALL pending messages from `progress_rx` and `result_rx`:
- Uses collected `pending_updates` / `pending_results` vectors
- Avoids borrow checker issues

## Tab System

### TabIndexing Model (`tui/tabs/mod.rs`)

- `Tab::all()` - Returns available tabs for current feature set
- `Tab::visible_index(&self)` - Position in `Tab::all()`
- `Tab::from_visible_index(index: usize)` - Tab by position
- `Tab::stable_id(&self)` - String ID for persistence
- `Tab::from_stable_id(id: &str)` - Tab from string ID
- `Tab::from_discriminant(discriminant: usize)` - Enum discriminant mapping

### TabWindow Helper

```rust
pub struct TabWindow {
    pub start: usize,
    pub end: usize,
    pub selected_visible: usize,
    pub max_visible: usize,
    pub total_tabs: usize,
    pub has_prev: bool,
    pub has_next: bool,
}
```

### Anti-patterns

- Don't use `tab as usize` (enum discriminants != visible indexes)
- Don't use `Tab::all().len()` as visible count
- Don't divide tab area by total tab count for mouse hit-testing

### Feature-Gated Tab Helpers

- `App::set_current_tab_if_available(tab: Tab) -> bool` - Set tab only if available for current feature set
- Use this helper for mouse selection, `select_tab()`, and session restore

## Notification System

`App` has a `notification: Option<Notification>` field for user-visible feedback.

```rust
pub struct Notification {
    pub message: String,
    pub severity: NotificationSeverity,
    pub created_at: std::time::Instant,
    pub timeout_secs: u64,
}

pub enum NotificationSeverity {
    Info,
    Success,
    Warning,
    Error,
}
```

- Set `app.notification = Some(Notification::new(msg, severity))` to show user feedback
- `Notification::is_expired()` returns true after `timeout_secs` seconds
- Status bar in `ui.rs` displays active notifications

## Dynamic Layout Pattern

For tabs with fixed-height sections, use dynamic constraints:

```rust
// Adapt config area to terminal height
let config_height = if area.height <= 30 {
    ((area.height as f32 * 0.8) as u16).max(10).min(27)
} else {
    27
};

let chunks = Layout::default()
    .constraints([Constraint::Length(config_height), Constraint::Min(3)])
    .split(area);
```

This ensures small terminals (< 24 rows) still show usable UI.

## Theme

`tc!` macro in `tui/theme.rs`:

```rust
tc!(field_name)  // primary, secondary, accent, background, text, etc.
```

**Semantic mapping:**
| Old | Theme |
|-----|-------|
| `Color::White` | `tc!(text)` |
| `Color::Gray` | `tc!(text_dim)` |
| `Color::Green` | `tc!(success)` |
| `Color::Red` | `tc!(error)` |

**HTTP status:** 200-299 → `tc!(success)`, 400-499 → `tc!(warning)`, 500-599 → `tc!(error)`

## FocusArea Enum Pattern

Tabs use `FocusArea` enum for navigation between Inputs/Options/Results areas.

## Overlay Precedence

Use `OverlayType` enum and `topmost_overlay()` helper for overlay precedence:

```rust
pub enum OverlayType {
    ConfirmPopup,   // Highest priority
    CommandPalette,
    Search,
    HttpOptions,
    Help,           // Lowest priority
}

pub fn topmost_overlay(&self) -> Option<OverlayType> {
    if self.is_confirm_popup_visible() {
        Some(OverlayType::ConfirmPopup)
    } else if self.is_command_palette_visible() {
        Some(OverlayType::CommandPalette)
    } else if self.is_search_visible() {
        Some(OverlayType::Search)
    } else if self.is_http_options_visible() {
        Some(OverlayType::HttpOptions)
    } else if self.is_help_visible() {
        Some(OverlayType::Help)
    } else {
        None
    }
}
```

Always use `topmost_overlay()` in event handling to ensure correct Esc key behavior.

## Background Task Routing

Use `task_tab: Option<Tab>` field to route background task results to the correct tab:

```rust
// When spawning task
self.task_tab = Some(self.current_tab);

// When processing results
let tab = self.task_tab.unwrap_or(self.current_tab);

// When task completes
self.task_tab = None;
```

## Input Cursor Invariant

`InputField::cursor_pos` uses byte index (not character count):
- Use `value.len()` for end position
- Use `c.len_utf8()` when incrementing
- Use `prev.len_utf8()` when decrementing
- Convert to char position only during rendering via `byte_to_char_pos()`

## Help Text Helper

Use `get_help_text()` helper in `ui.rs` for context-sensitive help:
```rust
fn get_help_text(app: &App, area: Rect) -> String {
    // Check overlays first (highest precedence)
    if app.pending_action.is_some() {
        return "[Enter] Confirm [Esc] Cancel".to_string();
    }
    // Then command palette, search, help, etc.
    // Finally, mode-specific help (Normal/Insert)
}
```

This ensures help text always matches current overlay and mode state.

## Dynamic Layout Pattern (Extended)

For tabs with fixed-height sections, use dynamic constraints based on terminal height:
```rust
let input_height = if area.height <= 24 {
    ((area.height as f32 * 0.6) as u16).max(6).min(15)
} else {
    15
};

let results_height = if area.height <= 24 {
    ((area.height as f32 * 0.4) as u16).max(3)
} else {
    0
};

let chunks = Layout::default()
    .constraints([
        Constraint::Length(6),  // Selector or header
        Constraint::Length(input_height),
        Constraint::Min(results_height),
    ])
    .split(area);
```

## Static Cache Removal

Avoid static caching for tab titles. Build from `Tab::all()` each render:
```rust
// Don't do this:
// static TAB_TITLES: LazyLock<Vec<Line>> = ...;

// Do this:
let all_tabs: Vec<Line> = Tab::all().iter().map(|t| Line::from(t.title())).collect();
let visible_titles: Vec<Line> = all_tabs[window.start..window.end].to_vec();
```

Ensures visible title list and `TabWindow` always come from same `Tab::all()` view.

## InputField Byte Index Invariant (Extended)

Always use byte length (not character count) for `cursor_pos` comparisons:
```rust
// Wrong:
field.cursor_pos >= field.value.chars().count()

// Correct:
field.cursor_pos >= field.value.len()
```

This affects `is_at_right_edge()` implementations in all tabs.