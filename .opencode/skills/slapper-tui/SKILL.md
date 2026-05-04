# Slapper TUI Skill

TUI module workflows and patterns for the terminal UI.

## Module Structure

```
crates/slapper/src/tui/
├── app/          # App state, event loop, command handling
│   ├── mod.rs           # App struct, notifications, helpers
│   ├── runner.rs        # Event loop, input handling
│   ├── key_handler.rs   # Key handling methods (extracted from mod.rs)
│   ├── state_update.rs  # Background task handling, result dispatch
│   ├── notifications.rs # Notification and NotificationSeverity types
│   ├── bookmarks.rs     # Bookmark helper functions
│   ├── confirmation.rs  # PendingAction enum
│   ├── help_config.rs   # Static help content
│   ├── navigation.rs    # Tab navigation, scrolling
│   ├── command.rs       # Command palette commands
│   ├── export.rs        # Export functionality
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

## Key Patterns

### Tab System

- `Tab::all()` - Returns available tabs for current feature set
- `Tab::visible_index(&self)` - Position in `Tab::all()`
- `App::set_current_tab_if_available(tab) -> bool` - Safe tab switching

### Traits

- `TabState` - State methods: `state()`, `progress()`, `reset()`, `set_error()`
- `TabInput` - Input handling: `handle_focus_next()`, `handle_char()`, etc.
- `TabRender` - Rendering: `render()`, `render_overlays()`

### Theming

Use `tc!` macro for all colors:
```rust
use crate::tc;
let style = Style::default().fg(tc!(text));
```

Semantic colors: `primary`, `secondary`, `accent`, `background`, `text`, `text_dim`, `success`, `warning`, `error`, `info`.

### Notifications

`App` has `notification: Option<Notification>` field:
```rust
// Set notification
app.notification = Some(Notification::new(
    "Exported to file.json".to_string(),
    NotificationSeverity::Success,
));

// Check if expired
if let Some(notif) = &app.notification {
    if notif.is_expired() {
        app.notification = None;
    }
}
```

### Dynamic Layouts

For small terminals, use dynamic constraints:
```rust
let config_height = if area.height <= 30 {
    ((area.height as f32 * 0.8) as u16).max(10).min(27)
} else {
    27
};

let chunks = Layout::default()
    .constraints([Constraint::Length(config_height), Constraint::Min(3)])
    .split(area);
```

## Testing

### Running TUI Tests
```bash
cargo test --lib -p slapper tui::
```

### Writing Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::tabs::Tab;

    #[test]
    fn test_something() {
        let mut app = create_test_app();
        // ... test logic
    }
}
```

### Test Coverage
- Tab focus navigation
- Layout rendering at various terminal sizes
- Event handling
- State updates

## Common Tasks

### Adding a New Tab
1. Create tab module in `tabs/`
2. Implement `TabState`, `TabInput`, `TabRender` traits
3. Add tab to `Tab` enum in `tabs/mod.rs`
4. Add rendering in `ui.rs` `draw_content()`
5. Add to `App::dispatcher_mut()` for event routing

### Fixing Layout Issues
1. Check for fixed `Constraint::Length` values
2. Replace with dynamic constraints based on `area.height`
3. Test at 80x24 and smaller terminals
4. Run `cargo test --lib -p slapper tui::`

### Adding Notifications
1. Set `app.notification = Some(Notification::new(...))`
2. Use `tc!` colors for severity
3. Test that notification displays in status bar

## Resources
- `crates/slapper/src/tui/AGENTS.override.md` - Detailed TUI patterns
- `plans/plan.md` - TUI improvement plan (most phases complete, 2 items deferred)
- `ARCHITECTURE.md` - Overall design

## Focus Indicators

InputField uses theme colors for focus states:
- `focus_normal` - Tab navigation highlight
- `focus_input` - Input field when focused
- `focus_results` - Results area highlight

## Mode Indicator

Status bar shows current input mode:
- **NORMAL** - Green badge, tab navigation active
- **INSERT** - Yellow/Red badge, input field focused

Use `app.mode` to check current mode (`InputMode::Normal` / `InputMode::Insert`).

## Quick Switch Panel

Ctrl+G opens bookmarked tabs with fuzzy search:
- `toggle_quick_switch()` / `close_quick_switch()` methods
- `get_quick_switch_results()` filters bookmarks by query
