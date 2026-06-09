# TUI Module Override

Specialized guidance for the terminal UI module.

## Recent Fixes (2026-05-29)

- **handle_enter() dispatcher caching**: `dispatcher_mut()` now cached to reduce 4 calls to 1 per Enter keypress
- **Theme restoration**: SessionManager restores theme when loading sessions; packaged themes can be retried after the background loader finishes, with the deferred restore kept in `ThemeLoadState`, without blocking startup
- **Settings save merge**: TUI settings now merge into the loaded config and preserve non-exposed sections
- **waf.rs checkbox bounds check**: Fixed `waf.rs:519` to guard against out-of-bounds index when toggling technique checkboxes (matching `recon.rs:588-590` pattern)
- **workers/security.rs error logging**: Fixed `security.rs:227,235` to use `tracing::warn!` instead of `tracing::debug!` for expected failure cases (finding list operations)
- **tabs/load.rs reset() bounds**: Fixed `load.rs:367-374` to use bounds check `if self.inputs.fields.len() > 5` before direct field access
- **tabs/fuzz.rs reset() bounds**: Fixed `fuzz.rs:404-413` to use bounds check `if self.inputs.fields.len() > 6` before direct field access
- **tabs/scan.rs render() bounds**: Fixed `scan.rs:306-307` to use bounds check `if self.inputs.fields.len() >= 2` before direct field access
- **components/input.rs can_move bounds**: Fixed `input.rs:680-694` to add `idx < self.fields.len()` check in `can_move_left()` and `can_move_right()`
- **app/mod.rs unused import**: Removed unused `FxHashMap` import from `app/mod.rs:40`

## Recent Fixes (2026-05-25)

- **vuln.rs edge detection bounds**: Fixed `vuln.rs:602,613` to use `.first().map(...).unwrap_or(true)` pattern for `is_at_left_edge()` and `is_at_right_edge()` instead of direct `fields[0]` indexing which could panic if fields is empty.
- **scrollable.rs scroll_down empty lines**: Fixed `scrollable.rs:57-59` to handle empty lines case explicitly. Previously `saturating_sub(1)` on empty len would result in `usize::MAX`, causing incorrect scroll offset.
- **api.rs worker error logging**: Fixed `api.rs:57,134` to use `tracing::warn!` instead of `tracing::debug!` for GraphQL request failures. Operational errors should be logged at warn level for proper visibility.

- **vuln.rs edge detection bounds**: Fixed `vuln.rs:602,613` to use `.first().map(...).unwrap_or(true)` pattern for `is_at_left_edge()` and `is_at_right_edge()` instead of direct `fields[0]` indexing which could panic if fields is empty.
- **scrollable.rs scroll_down empty lines**: Fixed `scrollable.rs:57-59` to handle empty lines case explicitly. Previously `saturating_sub(1)` on empty len would result in `usize::MAX`, causing incorrect scroll offset.
- **api.rs worker error logging**: Fixed `api.rs:57,134` to use `tracing::warn!` instead of `tracing::debug!` for GraphQL request failures. Operational errors should be logged at warn level for proper visibility.

## Recent Features (2026-05-25)

- **Configurable auto-save interval**: Settings > Session panel now allows configuring auto-save interval (previously hardcoded to 30s)

## Module Structure

```
crates/slapper/src/tui/
├── app/          # App state, event loop, command handling
│   ├── mod.rs           # App struct, notifications, helpers
│   ├── state.rs         # OverlayState, SearchState, QuickSwitchState, TaskState, ThemeLoadState
│   ├── tab_store.rs     # TabStore - owns all 29 tab instances
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
│   ├── theme_runtime.rs # Theme loader lifecycle helpers
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
│   └── ...
├── theme/        # Theme system
│   ├── mod.rs          # Module re-exports
│   ├── palette.rs      # ThemeMode, Theme, ThemeColors
│   ├── builtin.rs      # dark_theme(), light_theme()
│   ├── manager.rs      # ThemeManager
│   ├── style.rs        # Theme style methods
│   └── legacy.rs       # Thread-local macros (tc!, theme!)
├── ui/           # Rendering layer
│   ├── mod.rs          # draw(), LAYOUT_MARGIN, TAB_BAR_HEIGHT
│   ├── shell.rs        # draw_tabs, draw_breadcrumb, draw_content, draw_status_bar
│   ├── popups.rs       # draw_http_options_popup, draw_command_palette, draw_search_popup, draw_quick_switch
│   └── tests.rs        # UI rendering tests
├── search.rs     # Global search
└── help.rs       # HelpManager
```

## Event Loop Order

`runner.rs` follows `update() -> draw() -> input-check` order:
- `update()` processes background task results first
- `draw()` renders only if `needs_redraw` (or pending redraw) is set
- Input is read via non-blocking `EventStream::next().now_or_never()`
- If no event is available, loop sleeps for 10ms

## Quick Switch Panel

Ctrl+X shows ALL tabs with fuzzy search:

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

// Get all tabs filtered by query (searches title, stable_id, description)
pub fn get_quick_switch_results(&self) -> Vec<&'static Tab> {
    let query = self.quick_switch_query.to_lowercase();
    Tab::all().iter()
        .filter(|tab| {
            if query.is_empty() {
                true
            } else {
                tab.title().to_lowercase().contains(&query) ||
                tab.stable_id().contains(&query) ||
                tab.description().to_lowercase().contains(&query)
            }
        })
        .collect()
}
```

**Navigation within quick switch:**
- `Up/Down` - Navigate results
- `PageUp/PageDown` or `Ctrl+U/D` - Jump 10 items
- `Home/End` - Go to first/last item
- `Enter` - Select and switch to tab
- `Esc` - Close without switching
- `Backspace` - Delete last character of filter
- Regular characters filter the list

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

`Theme.name` is the canonical stable ID for the theme, selector labels are derived separately for display, `Ctrl+T` cycles the built-in theme trio only, and `ThemeManager.current` is private.

Theme loading runs in a background thread; `ThemeLoadState` keeps the receiver, join handle, and deferred restore request together so startup stays non-blocking.

New rendering code should prefer explicit `&Theme` parameters:
```rust
pub fn draw_widget(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let style = Style::default().fg(theme.colors.text);
}
```

For tab renderers and components that still use `tc!` macro:
```rust
use crate::tc;
let style = Style::default().fg(tc!(text));
```

**Semantic mapping:**
| Old | Theme |
|-----|-------|
| `Color::White` | `tc!(text)` or `theme.colors.text` |
| `Color::Gray` | `tc!(text_dim)` or `theme.colors.text_dim` |
| `Color::Green` | `tc!(success)` or `theme.colors.success` |
| `Color::Red` | `tc!(error)` or `theme.colors.error` |

**HTTP status:** 200-299 → `success`, 400-499 → `warning`, 500-599 → `error`

## FocusArea Enum Pattern

Tabs use `FocusArea` enum for navigation between Inputs/Options/Results areas.

## Overlay Precedence

Use `OverlayType` enum and `topmost_overlay()` helper for overlay precedence:

```rust
pub enum OverlayType {
    ConfirmPopup,   // Highest priority
    CommandPalette,
    QuickSwitch,
    Search,
    HttpOptions,
    Help,           // Lowest priority
}

pub fn topmost_overlay(&self) -> Option<OverlayType> {
    if self.is_confirm_popup_visible() {
        Some(OverlayType::ConfirmPopup)
    } else if self.is_command_palette_visible() {
        Some(OverlayType::CommandPalette)
    } else if self.is_quick_switch_visible() {
        Some(OverlayType::QuickSwitch)
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

## Confirmation System

Use `PendingAction` enum for destructive/confirmation actions:

```rust
pub enum PendingAction {
    ResetTab,
    SaveSettings,
    DeleteHistoryEntry,
    ClearHistory,
}

impl PendingAction {
    pub fn message(&self) -> (&str, &str) { ... }
    pub fn execute(&self, app: &mut App) { ... }
}
```

Request confirmation before executing:
```rust
app.request_confirmation(PendingAction::ResetTab);
```

Confirm/cancel in event handlers:
```rust
app.confirm_action();  // Executes the pending action
app.cancel_action();   // Dismisses without executing
```

## Help System Architecture

Help content is statically defined in `help_config.rs` and referenced via `HelpManager`:

- `help_config.rs::get_static_help_data()` - Returns `StaticHelpData` with sections per Tab
- `HelpManager` in `help.rs` - Runtime help state, keyboard shortcuts, pagination
- Help overlay rendered via `draw_help_overlay()` in `ui.rs`

**Help text helper:**
```rust
fn get_help_text(app: &App, area: Rect) -> String {
    if app.pending_action.is_some() {
        return "[Enter] Confirm [Esc] Cancel".to_string();
    }
    // ... overlay-specific help
}
```

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

## Selector API (Hardened in recent refactor)

Selector now has explicit interaction methods:
```rust
// State queries
selector.is_open() -> bool
selector.is_focused() -> bool

// Explicit control
selector.open()           // opens dropdown
selector.close()          // closes dropdown  
selector.confirm() -> Option<&SelectorItem>  // commits selection, returns item, closes
selector.cancel()         // closes without changing selection

// Navigation
selector.move_next()      // moves selection down (when open)
selector.move_prev()      // moves selection up (when open)

// Legacy methods (still work but explicit is preferred)
selector.expand()         // same as open()
selector.collapse()       // same as close()
selector.next()           // same as move_next()
selector.prev()           // same as move_prev()
```

Selector contract:
- Focus does not automatically open
- Enter on closed selector opens it
- Enter on open selector commits and closes
- Esc closes without committing
- Up/Down only move selection when open

## Common Bug Patterns

### Division by Zero in Progress

When computing progress as a ratio, always guard against empty collections:

```rust
fn progress(&self) -> f64 {
    if self.stages.is_empty() {
        return 0.0;
    }
    let completed = self.stages.iter().filter(...).count();
    (completed as f64 / self.stages.len() as f64) * 100.0
}
```

### ScrollableText Scroll Offset

Guard against empty lines when calculating scroll offset:

```rust
let scroll_offset = if self.lines.is_empty() {
    0
} else {
    self.scroll_offset.min(self.lines.len() - 1)
};
```

### TaskResult Handling

When routing TaskResult through multiple handlers, use early return pattern:

```rust
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

### Worker Error Handling

When reading response bodies in workers, log errors instead of silent suppression:

```rust
let response_text = match response.text().await {
    Ok(text) => text,
    Err(e) => {
        tracing::debug!("Failed to read response body: {}", e);
        String::new()
    }
};
```

### History Export Serialization

Handle errors explicitly when serializing history:

```rust
match serde_json::to_string_pretty(&export_data) {
    Ok(s) => s,
    Err(e) => {
        tracing::debug!("Failed to serialize history export: {}", e);
        String::new()
    }
}
```

### FxHashMap/FxHashSet Usage

For performance, use `rustc_hash::FxHashMap` and `rustc_hash::FxHashSet` instead of `std::collections::HashMap/HashSet`:

```rust
use rustc_hash::{FxHashMap, FxHashSet};

// In struct definitions
pub tabs: FxHashMap<Tab, Box<dyn TabInput>>,
pub bookmarks: FxHashSet<String>,

// In initialization
let mut map = FxHashMap::default();
let mut set = FxHashSet::default();
```

Files affected by HashMap→FxHashMap migration (2026-05-22):
- `app/mod.rs` - App.tabs, App.bookmarks
- `app/bookmarks.rs` - toggle_bookmark, is_bookmarked, get_bookmarked_tab_ids
- `app/help_config.rs` - StaticHelpData.sections
- `help.rs` - HelpManager uses FxHashMap
- `theme.rs` - ThemeManager.themes
- `tabs/dashboard.rs` - PortfolioSnapshot.findings_by_severity

### Key Binding Conflict Prevention

When adding key bindings in `key_handler.rs`, avoid duplicate patterns in the same match arm:

```rust
// WRONG - 'e' appears twice, second arm is unreachable
(KeyModifiers::NONE, KeyCode::Char('w')) => app.handle_word_forward(),
(KeyModifiers::NONE, KeyCode::Char('e')) => app.export_results(),
(KeyModifiers::NONE, KeyCode::Char('e')) => app.handle_word_forward(), // unreachable!

// CORRECT - unique bindings
(KeyModifiers::NONE, KeyCode::Char('w')) => app.handle_word_forward(),
(KeyModifiers::NONE, KeyCode::Char('e')) => app.export_results(),
```

The compiler may warn about unreachable patterns, but always verify manually when editing key handling code.

### Bounds Check for Array Access

When accessing arrays/vectors via index in handle_enter or similar, always validate bounds:

```rust
// WRONG - could panic if index >= len
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
self.inputs.fields[1].cursor_pos = 11;
    }
}
```

## Worker Patterns

### Silent Send Error Handling

Workers use the proper error-handling pattern for channel sends:

```rust
if let Err(e) = result_tx.send(TaskResult::LoadTest(results)).await {
    tracing::warn!("Failed to send load test results: {}", e);
}
if let Err(e) = progress_tx.send((requests, requests)).await {
    tracing::warn!("Failed to send progress: {}", e);
}
```

If the main loop has been dropped (app closed), the send fails — but this is logged at `warn` level for diagnostics. For critical failures that should abort the task, workers return `Err(...)` which propagates to `run()` and is handled at the TaskRunner level.

DO NOT use `let _ = channel.send(...).await` — this silently drops errors. See `architecture/tui.md` for the full pattern.

### Error String Matching in Retry Logic

The retry logic in `workers/recon.rs` uses string matching to determine if an error is retryable:

```rust
let is_retryable = error_str.contains("timeout")
    || error_str.contains("connection")
    || error_str.contains("temporary")
    || error_str.contains("reset")
    || error_str.contains("broken pipe");
```

This pattern is fragile but intentional because:
- It catches common retryable error messages from various sources
- Lowercase conversion handles different error casing
- Adding more patterns is straightforward if needed

When adding new error types, prefer adding to this list rather than creating new error variants.

## Tab Count

The `Tab` enum in `tui/tabs/mod.rs` has exactly **28 variants**. Do not reference "29 tabs" or "31 payload types" — these are stale counts from earlier versions.

## Uniform Look & Feel (Completed)

All TUI uniform look & feel items have been implemented:
- Popup content text styling applied at `popup.rs:130`
- Notification warning color corrected at `ui.rs:579`
- Results borders standardized (all tabs pass `None`)
- Bordered input blocks added to all tab files
- `empty_state_paragraph()` added to all empty-state tabs
- Scrollbar theme styling applied in `scrollable.rs`

If visual consistency issues are found, verify the actual `.rs` files before creating new fix items.

## Session Fixes (2026-06-07)

### wireless.rs Fixes

- **wireless.rs:229**: Direct `fields[0]` access without bounds check — changed to `if let (Some(chunk), Some(field)) = (input_chunks.first(), self.inputs.fields.first())`
- **wireless.rs:334-337**: `handle_up()` missing `is_running()` guard — added guard
- **wireless.rs:340-343**: `handle_down()` missing `is_running()` guard — added guard
- **wireless.rs:358-363**: `page_up()` missing `is_running()` guard — added guard
- **wireless.rs:366-372**: `page_down()` missing `is_running()` guard — added guard
- **wireless.rs:262-373**: Added missing trait implementations: `handle_word_forward`, `handle_word_backward`, `handle_home`, `handle_end`, `handle_top`, `handle_bottom`, `handle_copy`

### components/input.rs Fixes

- **input.rs:149-168**: `move_left()`/`move_right()` returned `true` even when cursor didn't move — moved `true` inside `if let Some` block so it only returns true on actual movement

### fuzz.rs Fixes

- **fuzz.rs:1109-1171**: 6 orphaned test functions defined outside `mod tests` block — moved all tests inside `mod tests` and fixed direct `fields[0]` access to use `.first()`

### Clippy .get(0) → .first() Fixes

Changed `.get(0)` to `.first()` across 13 files (19 occurrences): fuzz.rs, graphql.rs, load.rs, oauth.rs, packet.rs, proxy.rs, report.rs, resume.rs, scan.rs, settings/render.rs, stress.rs, waf.rs, waf_stress.rs
