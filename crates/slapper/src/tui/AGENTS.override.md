# TUI Module Override

Specialized guidance for the terminal UI module.

## Event Loop Order

`runner.rs` follows `update() -> draw() -> poll()` order:
- `update()` processes background task results first
- `draw()` renders only if `needs_redraw` is set
- `poll()` waits for user input with 100ms timeout

## Channel Draining

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

## Auto-Insert Mode

Automatically switches to Insert mode when Tab/Shift+Tab focuses an input.