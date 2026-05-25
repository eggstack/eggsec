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

## Recent Fixes (2026-05-29)

- **workers/security.rs error logging**: Fixed `security.rs:227,235` to use `tracing::warn!` for operational failures
- **tabs/load.rs reset() bounds**: Fixed `load.rs:367-374` to use bounds check before direct field access
- **tabs/fuzz.rs reset() bounds**: Fixed `fuzz.rs:404-413` to use bounds check before direct field access
- **tabs/scan.rs render() bounds**: Fixed `scan.rs:306-307` to use bounds check before direct field access
- **components/input.rs can_move bounds**: Fixed `input.rs:680-694` to add bounds checks in navigation helpers
- **app/mod.rs unused import**: Removed unused `FxHashMap` import

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

### Division by Zero Prevention

When computing progress as a ratio, always guard against empty collections:

```rust
// WRONG - panics if stages is empty
fn progress(&self) -> f64 {
    let completed = self.stages.iter().filter(...).count();
    (completed as f64 / self.stages.len() as f64) * 100.0
}

// CORRECT - returns 0.0 when empty
fn progress(&self) -> f64 {
    if self.stages.is_empty() {
        return 0.0;
    }
    let completed = self.stages.iter().filter(...).count();
    (completed as f64 / self.stages.len() as f64) * 100.0
}
```

### ScrollableText Empty Lines Prevention

When calculating scroll offset, guard against empty lines:

```rust
// WRONG - usize::MAX when lines is empty
let scroll_offset = self.scroll_offset.min(self.lines.len().saturating_sub(1));

// CORRECT - returns 0 when empty
let scroll_offset = if self.lines.is_empty() {
    0
} else {
    self.scroll_offset.min(self.lines.len() - 1)
};
```

### Error Handling in Workers

Avoid silent error suppression when reading response bodies:

```rust
// WRONG - silently returns empty string on error
let response_text = response.text().await.unwrap_or_default();

// CORRECT - logs the error at debug level
let response_text = match response.text().await {
    Ok(text) => text,
    Err(e) => {
        tracing::debug!("Failed to read response body: {}", e);
        String::new()
    }
};
```

### TaskResult Handling

When routing TaskResult through multiple handlers, avoid use-after-move:

```rust
// WRONG - result is moved and can't be used in debug log
let Some(result) = self.handle_security_result(result) else { return };
let Some(result) = self.handle_protocol_result(result) else { return };
tracing::debug!("Unhandled: {:?}", result); // ERROR: result already moved

// CORRECT - use early return pattern that doesn't consume result
let result = match self.handle_security_result(result) {
    Some(r) => r,
    None => return,
};
let result = match self.handle_protocol_result(result) {
    Some(r) => r,
    None => return,
};
if self.handle_feature_result(result).is_none() {
    tracing::debug!("Unhandled TaskResult variant");
}
```

### History Export Error Handling

Handle serialization errors explicitly:

```rust
// WRONG - silently returns empty string
serde_json::to_string_pretty(&export_data).unwrap_or_default()

// CORRECT - logs at debug level
match serde_json::to_string_pretty(&export_data) {
    Ok(s) => s,
    Err(e) => {
        tracing::debug!("Failed to serialize history export: {}", e);
        String::new()
    }
}
```

### Bounds Check for Array Access

When accessing arrays/vectors via index, always validate bounds to prevent panic:

```rust
// WRONG - panics if index >= len
self.option_checkboxes[self.focused_checkbox_index].toggle();

// CORRECT - bounds check prevents panic
if self.focused_checkbox_index < self.option_checkboxes.len() {
    self.option_checkboxes[self.focused_checkbox_index].toggle();
}
```

Similarly for InputGroup field access:

```rust
// WRONG - assumes at least 2 fields
self.inputs.fields[1].value = "report.json".to_string();

// CORRECT - check length first
if self.inputs.fields.len() > 1 {
    self.inputs.fields[1].value = "report.json".to_string();
}
```

### Bounds Check for Option Checkbox Arrays

When accessing checkbox arrays by index, use `.get()` with fallback:

```rust
// WRONG - panics if index out of bounds
no_tech: self.option_checkboxes[0].checked,

// CORRECT - returns false if index invalid
no_tech: self.option_checkboxes.get(0).map(|cb| cb.checked).unwrap_or(false),
```

### ScrollableText Empty Lines Handling

When implementing `scroll_to_bottom()` or calculating max scroll offset:

```rust
// WRONG - scroll_offset becomes usize::MAX when lines is empty
self.scroll_offset = self.lines.len().saturating_sub(1);

// CORRECT - explicitly handle empty case
if self.lines.is_empty() {
    self.scroll_offset = 0;
} else {
    self.scroll_offset = self.lines.len() - 1;
}
```

In render, calculate scroll_offset safely:

```rust
// WRONG - usize::MAX when lines is empty
let scroll_offset = self.scroll_offset.min(self.lines.len().saturating_sub(1));

// CORRECT - explicit empty check
let scroll_offset = if self.lines.is_empty() {
    0
} else {
    self.scroll_offset.min(self.lines.len() - 1)
};
```

## Resources
- `crates/slapper/src/tui/AGENTS.override.md` - Detailed TUI patterns
- `architecture/tui.md` - TUI architecture, event loop, overlays, and session handling
- `architecture/config.md` - Config loading and TUI settings save semantics

## Focus Indicators

InputField uses theme colors for focus states:
- `focus_normal` - Tab navigation highlight
- `focus_input` - Input field when focused
- `focus_results` - Results area highlight

## Bounds Check for Checkbox Arrays (Critical)

When accessing checkbox arrays via index in handle_enter, always validate bounds:

```rust
// WRONG - could panic if index >= len
self.technique_checkboxes[self.focused_checkbox_index].toggle();

// CORRECT - bounds check prevents panic
if self.focused_checkbox_index < self.technique_checkboxes.len() {
    self.technique_checkboxes[self.focused_checkbox_index].toggle();
}
```

This pattern was missing in `waf.rs:519` and was fixed to match the pattern already correctly used in `recon.rs:588-590`.

## Checkbox Array Patterns by Tab

| Tab | Checkbox Field | Bounds Check Location | Status |
|-----|---------------|---------------------|--------|
| `recon.rs` | `option_checkboxes` | 588-590 | ✅ Safe |
| `waf.rs` | `technique_checkboxes` | 519, 311-316 (fixed) | ✅ Safe |
| `hunt.rs` | `option_checkboxes` | get_config() (fixed 2026-05-25) | ✅ Safe |
| `browser.rs` | `option_checkboxes` | get_config() (fixed 2026-05-25) | ✅ Safe |
| `fuzz.rs` | Individual checkboxes (not array) | N/A | ✅ Safe |

For option checkbox arrays, use `.get()` with fallback when constructing options:

```rust
// WRONG - panics if index out of bounds
no_tech: self.option_checkboxes[0].checked,

// CORRECT - returns false if index invalid
no_tech: self.option_checkboxes.get(0).map(|cb| cb.checked).unwrap_or(false),
```

For mutable access (e.g., in reset methods), use `.get_mut()`:

```rust
// WRONG - panics if index out of bounds and cannot assign to & reference
if let Some(cb) = self.technique_checkboxes.get(1) {
    cb.checked = true; // ERROR: cannot assign to `cb.checked`, which is behind a `&` reference
}

// CORRECT - use get_mut for mutable access
if let Some(cb) = self.technique_checkboxes.get_mut(1) {
    cb.checked = true;
}
```

## Mode Indicator

Status bar shows current input mode:
- **NORMAL** - Green badge, tab navigation active
- **INSERT** - Yellow/Red badge, input field focused

Use `app.mode` to check current mode (`InputMode::Normal` / `InputMode::Insert`).

## Quick Switch Panel

Ctrl+X opens all tabs with fuzzy search:
- `toggle_quick_switch()` / `close_quick_switch()` methods
- `get_quick_switch_results()` filters by title, stable ID, or description

## Overlay Precedence

When multiple overlays are active, use `topmost_overlay()` to determine which handles input:

```rust
pub enum OverlayType {
    ConfirmPopup,   // Highest priority
    CommandPalette,
    QuickSwitch,
    Search,
    HttpOptions,
    Help,           // Lowest priority
}
```

## Confirmation System

Use `PendingAction` for destructive/confirmation actions:
```rust
app.request_confirmation(PendingAction::ResetTab);
// Later: app.confirm_action() or app.cancel_action()
```

## Help System

Help content is extracted to `help_config.rs::get_static_help_data()`:
- Returns `StaticHelpData` with `sections: HashMap<Tab, HelpSection>`
- Each `HelpSection` contains title, content, and commands list
- `HelpManager` in `help.rs` handles runtime state and rendering

## TabError System

Tabs use structured error handling via `TabError` enum in `tui/app/tab_error.rs`:
```rust
pub enum TabError {
    Network(String),
    Auth(String),
    Config(String),
    Resource(String),
    Target(String),
    Internal(String),
    Unknown(String),
}
```

- `set_error(error: TabError)` method on TabState trait
- `TabError::is_recoverable()` checks for Network/Auth/Resource errors
- `TabError::message()` returns the error string for display
- Error display happens in render() method: `error.message()`

## Visual Regression Testing

Use `TestBackend` for render tests:
```rust
use ratatui::backend::TestBackend;
use ratatui::Terminal;

let backend = TestBackend::new(80, 24);
let mut terminal = Terminal::new(backend).unwrap();
terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
let buf = terminal.backend().buffer();
// Check buf.content for expected symbols
```

## Settings Tab (tabs/settings/main.rs)

The Settings tab is a "quick settings" interface, but saving now merges the exposed fields into the loaded config instead of rebuilding from defaults. Non-exposed sections are preserved, including `profiles`, `schedule`, `remote`, `ai`, `search`, `alert_channels`, and other untouched fields.

### Settings Input Fields
- Timeout (s) - maps to `http.timeout_secs`
- Max Retries - maps to `http.max_retries`
- Retry Delay (ms) - maps to `http.retry_delay_ms` (added 2026-05-22)
- Max Redirects - maps to `http.max_redirects`
- Default Concurrency - maps to `scan.default_concurrency`
- Rate Limit (req/s) - maps to `scan.rate_limit_per_second`
- Port Timeout (s) - maps to `scan.port_timeout_secs` (default is 2, not 300)

## Selector API

Selector provides explicit methods for dropdown interaction:
```rust
// State
selector.is_open() -> bool
selector.is_focused() -> bool

// Control
selector.open()           // Opens dropdown
selector.close()          // Closes dropdown
selector.confirm() -> Option<&SelectorItem>  // Commits selection, returns item
selector.cancel()         // Closes without changing

// Navigation
selector.move_next()      // Moves selection down (when open)
selector.move_prev()      // Moves selection up (when open)
```

Key behaviors:
- `focus()` sets focused=true only (does NOT open dropdown)
- `focus_open()` sets focused=true AND opens dropdown
- `handle_enter()` on closed selector opens it; on open selector commits and closes
- Esc closes without committing
- Up/Down only move selection when open (no-op when closed)
- Left/Right navigation does NOT mutate closed selector selection

## Worker Patterns

### Silent Send Error Handling

Workers use `let _ = channel.send(...).await` for progress and result channels. This is intentional:

```rust
// Worker send pattern - intentional silent suppression
let results = runner.run().await?;
let _ = result_tx.send(TaskResult::LoadTest(results)).await;
let _ = progress_tx.send((requests, requests)).await;
Ok(())
```

If the main loop has been dropped (app closed), there's no point propagating the error. Progress updates that fail don't affect the final result. For critical failures, workers return `Err(...)` which is handled at the TaskRunner level.

### Error String Matching in Retry Logic

The retry logic in `workers/recon.rs` uses string matching to determine retryable errors:

```rust
let is_retryable = error_str.contains("timeout")
    || error_str.contains("connection")
    || error_str.contains("temporary")
    || error_str.contains("reset")
    || error_str.contains("broken pipe");
```

This is intentional but fragile. It catches common retryable error messages from various sources. Add more patterns as needed rather than creating new error variants.

## InputGroup Field Slice Bounds

When accessing slices of `InputGroup.fields`, always use bounds-checked slice patterns:

```rust
// WRONG - panics if fewer than 4 fields
let fields = &self.issue_inputs.fields[..4];

// CORRECT - safe slicing with .get()
let fields = match self.current_mode {
    IntegrationsMode::CreateIssue => {
        self.issue_inputs.fields.get(..4).unwrap_or(&self.issue_inputs.fields)
    }
    IntegrationsMode::SearchIssues => {
        self.issue_inputs.fields.get(4..).unwrap_or(&[])
    }
};
```

This pattern was fixed in `integrations.rs:329-338` (2026-05-25).
