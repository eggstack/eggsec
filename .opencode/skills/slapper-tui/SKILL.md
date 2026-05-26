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

## is_at_left_edge Checkbox Guard (Critical - 2026-05-26 Session)

Always add `is_empty()` guard for checkbox arrays in `is_at_left_edge()` and `is_at_right_edge()`:

```rust
// WRONG - if checkboxes is empty, focused_checkbox_index==0 still evaluates incorrectly
fn is_at_left_edge(&self) -> bool {
    self.focused_checkbox_index == 0
}

// CORRECT - guards against empty checkbox array
fn is_at_left_edge(&self) -> bool {
    self.checkbox_array.is_empty() || self.focused_checkbox_index == 0
}

fn is_at_right_edge(&self) -> bool {
    self.checkbox_array.is_empty()
        || self.focused_checkbox_index >= self.checkbox_array.len().saturating_sub(1)
}
```

Files fixed with this pattern:
- `waf.rs:588-596` (2026-05-26)
- `recon.rs:677-687` (2026-05-26)
- `hunt.rs:514,524` (2026-05-26)
- `browser.rs:473,483` (2026-05-26)
- `compliance.rs:417,428` (2026-05-26)

## Session Fixes (2026-05-30 Continuation Session)

### is_running() Guards Added

All 29 tabs now properly guard input handlers with `!self.is_running()`. This prevents input during running state:

| Tab | handle_char | handle_backspace | handle_paste |
|-----|-------------|------------------|-------------|
| stress.rs | ✅ Fixed | ✅ Fixed | ✅ Fixed |
| compliance.rs | ✅ Fixed | ✅ Fixed | ✅ Fixed |
| storage.rs | ✅ Fixed | ✅ Fixed | ✅ Fixed |
| integrations.rs | ✅ Fixed | ✅ Fixed | ✅ Fixed |
| workflow.rs | ✅ Fixed | ✅ Fixed | ✅ Fixed |
| vuln.rs | ✅ Fixed | ✅ Fixed | ✅ Fixed |
| oauth.rs | ✅ Fixed (char) | ✅ Already had | ✅ Already had |
| auth.rs | ✅ Fixed | ✅ Fixed | ✅ Fixed |
| cluster.rs | ✅ Fixed | ✅ Fixed | ✅ Fixed |
| graphql.rs | ✅ Fixed | ✅ Already had | ✅ Already had |

### Other TUI Fixes

- **stress.rs:195-206**: Fixed bounds check from `>3` to `>1` for fields[1]
- **load.rs:367-376**: Fixed bounds check from `>5` to `>=5` for fields[4]
- **recon.rs:309-318**: Removed dead code path `visible_rows == 0`

## Recent Fixes (2026-05-30 Session)

- **fingerprint.rs:290-298**: Fixed `handle_focus_prev()` integer underflow - added `is_empty()` check before `fields.len() - 1`
- **scan_endpoints.rs:333-335**: Fixed `handle_focus_prev()` to use `focus_prev()` with `is_empty()` guard
- **fuzz.rs:477-497**: Added `config_chunks.len() >= 7` guard before accessing config_chunks[3-6]
- **fuzz.rs:583-591**: Added `config_chunks.len() >= 6` guard and `.get()` pattern for dropdown info
- **plugin.rs:250-252**: Added `input_chunks.first()` check before accessing `input_chunks[0]`
- **graphql.rs:296-300**: Added `options_chunks.len() >= 4` guard for checkbox renders
- **oauth.rs:341-345**: Added `options_chunks.len() >= 4` guard for checkbox renders
- **nse.rs:393-401**: Fixed `is_at_left_edge()` - changed `<=` to `==` for left edge detection
- **integrations.rs:334**: Removed redundant `.map(|s| s)` identity map
- **workflow.rs:322-337**: Added `field_chunks.get(i)` bounds check for idx==5/6 branches
- **scan_ports.rs:166-171**: Moved `is_empty()` check outside loop - was unreachable dead code

## Additional Fixes (2026-05-26 Session)

- **stress.rs:195-206**: Fixed missing bounds check in reset() - added individual `if len > N` guards for fields[1-3]
- **scan_ports.rs:172-186**: Fixed validation to check ALL targets (not just first) and validate port range per target
- **waf.rs:598-606**: Fixed `is_at_right_edge()` - added `is_empty()` guard + `saturating_sub(1)` for empty checkboxes
- **waf.rs:588-596**: Fixed `is_at_left_edge()` - added `is_empty()` guard for empty checkboxes
- **fuzz.rs:128-134**: Refactored redundant `match` to `let...else` syntax for session None check
- **vuln.rs:419-423**: Fixed `field_chunks[i]` bounds - use `if let Some(chunk) = field_chunks.get(i)`
- **recon.rs:677-687**: Fixed `is_at_right_edge()` for Options - added `is_empty()` guard + `saturating_sub(1)`
- **oauth.rs:400-404**: Added `!self.is_running()` guard to `handle_backspace()`

## Recent Fixes (2026-05-29 Evening Session)

- **tabs/scan_ports.rs:333**: Fixed `input_chunks[4]` direct indexing - use `.get(4)` for UDP checkbox
- **tabs/recon.rs:399**: Fixed field render loop to use `input_chunks.get(i)` pattern
- **tabs/fuzz.rs:477**: Added `config_chunks.len() >= 3` check before accessing fields
- **tabs/hunt.rs:277**: Fixed field render loop to use `input_chunks.get(i)` pattern
- **tabs/browser.rs:242**: Fixed field render loop to use `input_chunks.get(i)` pattern
- **tabs/storage.rs:320,337**: Fixed `config_chunks[i+1]` and `query_chunks[i+1]` to use `.get(i+1)`
- **tabs/integrations.rs:344**: Fixed `field_chunks[i]` to use `.get(i)` pattern
- **tabs/workflow.rs:333**: Fixed `field_chunks[i]` bounds check
- **tabs/dashboard.rs:198-206**: Cached `e.summary.to_lowercase()` per entry in filter
- **tabs/history.rs:197-202**: Cached lowercased fields per entry in search()
- **app/mod.rs:696-698**: Cached tab title/stable_id/description lowercase in quick switch
- **components/input.rs:97**: Cached `s.to_lowercase()` per candidate in autocomplete
- **tabs/scan.rs:259**: Added `.max(1)` guard for progress calculation
- **tabs/integrations.rs:331**: Removed redundant `.map(|s| s)` identity map
- **workers/security.rs:119-121**: Fixed empty if block - added `findings.push(Severity::Info)`
- **app/task_runtime.rs:74**: Changed from silent `let _ = send()` to proper error check with warn
- **app/task_runtime.rs:68-79**: Refactored empty Ok arm match to `if let Err(e)` pattern
- **app/state_update.rs:68**: Changed log level from `debug!` to `warn!` for unhandled variants

## Earlier Fixes (2026-05-29)

- **popup.rs render() bounds**: Fixed `popup.rs:129-167` to use `if let Some(chunk) = chunks.get(0)` and `if let Some(button_area) = chunks.get(1)` instead of direct indexing
- **workers/api.rs double map_err**: Fixed `api.rs:339` - removed duplicate `??` that caused unreachable error handling
- **workers/recon.rs division guard**: Fixed `recon.rs:133` - added `total_stages.max(1)` guard for progress calculation
- **workers/security.rs error logging**: Fixed `security.rs:227,235` to use `tracing::warn!` for operational failures
- **tabs/load.rs reset() bounds**: Fixed `load.rs:367-374` to use bounds check before direct field access
- **tabs/fuzz.rs bounds**: Fixed `fuzz.rs:511` config_chunks[7] to use `.get()` pattern; fixed tests at 934,987,991
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

### Send Error Handling

Workers must properly handle send errors on progress and result channels:

```rust
// WRONG - silent error suppression
let _ = result_tx.send(TaskResult::LoadTest(results)).await;
let _ = progress_tx.send((requests, requests)).await;

// CORRECT - proper error handling with warn logging
if let Err(e) = result_tx.send(TaskResult::LoadTest(results)).await {
    tracing::warn!("Failed to send load test results: {}", e);
}
if let Err(e) = progress_tx.send((requests, requests)).await {
    tracing::warn!("Failed to send progress: {}", e);
}
```

Files fixed (2026-05-31 session): api.rs (15), fuzzer.rs (8), network.rs (13), plugin.rs (10), recon.rs (12), scanner.rs (9), security.rs (27)

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

## Session Fixes (2026-05-25)

### Tokio Spawn Error Handling

Workers now properly check JoinHandle results to detect panics:

**network.rs:159-170** - Packet capture task result handling:
```rust
match handle_result {
    Err(e) => {
        tracing::warn!("Packet capture handle timed out: {}", e);
    }
    Ok(Err(e)) => {
        if e.is_panic() {
            tracing::warn!("Packet capture task panicked: {:?}", e);
        } else {
            tracing::warn!("Packet capture task failed: {}", e);
        }
    }
    Ok(Ok(())) => {
        tracing::debug!("Packet capture task completed successfully");
    }
}
```

**recon.rs:176-215** - Progress handle checked in all match arms:
```rust
if let Err(e) = progress_handle.await {
    if e.is_panic() {
        tracing::warn!("Progress tracking task panicked: {:?}", e);
    }
}
```

### Bounds Checks Added (2026-05-25)

| File | Line(s) | Fix |
|------|---------|-----|
| `tabs/fuzz.rs` | 128-134 | Replaced `.expect()` with `if let Some(s) = ...` + warn |
| `tabs/fuzz.rs` | 471-473 | Added `if self.inputs.fields.len() > 2` guard |
| `tabs/scan_ports.rs` | 167-192 | Added `is_empty()` and `len() < 2` guards |
| `tabs/scan_endpoints.rs` | 255-258 | Added `if len > 1` / `if len > 2` guards |
| `tabs/fingerprint.rs` | 212-215 | Added `if len > 1` / `if len > 2` guards |
| `tabs/waf_stress.rs` | 148-151 | Added `if len > 1` / `if len > 2` guards |
| `tabs/packet.rs` | 533 | Added `if len > 2` guard |
| `tabs/settings/main.rs` | 267-289, 291-325, 431-446 | Replaced direct indexing with `.get().map().unwrap_or()` |
| `tabs/workflow.rs` | 332 | Added `if idx < self.inputs.fields.len()` guard |
| `tabs/vuln.rs` | 420 | Added `if idx < self.inputs.fields.len()` guard |
| `tabs/integrations.rs` | 3 | Removed duplicate `use crate::tc;` |

### to_lowercase() Caching

**security.rs:115-121** - Avoid redundant allocations:
```rust
if let Some(v) = headers.get("cache-control").and_then(|v| v.to_str().ok()) {
    let lower = v.to_lowercase();
    if lower.contains("no-cache") || lower.contains("no-store") {
    }
}
```

**history.rs:168-179** - Cache lowercase comparison value:
```rust
if scan_type.is_empty() {
    return self.entries.iter().collect();
}
let scan_type_lower = scan_type.to_lowercase();
self.entries
    .iter()
    .filter(|e| e.scan_type.to_lowercase().contains(&scan_type_lower))
    .collect()
```

## Recent Fixes (2026-05-31 Session)

### Direct Array Access Fixed

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `compliance.rs` | 232 | `input_chunks[2]` direct access | `if let Some(framework_area) = input_chunks.get(2)` |
| `recon.rs` | 404 | `input_chunks[2]` direct access | `let Some(options_area) = input_chunks.get(2) else { return; }` |
| `browser.rs` | 247 | `input_chunks[2]` direct access | `let Some(cb_area) = input_chunks.get(2) else { return; }` |
| `hunt.rs` | 282 | `input_chunks[3]` direct access | `let Some(cb_area) = input_chunks.get(3) else { return; }` |

### is_running() Guards Added (Additional Tabs)

| Tab | handle_char | handle_backspace | handle_paste |
|-----|-------------|------------------|------------|
| packet.rs | ✅ Fixed | ✅ Fixed | ✅ Fixed |
| cluster.rs | ✅ Fixed | ✅ Fixed | ✅ Fixed |
| proxy.rs | ✅ Fixed | ✅ Fixed | ✅ Fixed |
| nse.rs | ✅ Fixed | ✅ Fixed | ✅ Fixed |
| plugin.rs | ✅ Fixed | ✅ Fixed | ✅ Fixed |
| report.rs | ✅ Fixed | ✅ Fixed | ✅ Fixed |

### is_at_*_edge Guards for Selectors

| File | Lines | Fix |
|------|-------|-----|
| `nse.rs` | 396, 405 | `self.script_selector.items.is_empty() ||` guard added |
| `storage.rs` | 578, 588 | `self.mode_selector.items.is_empty() ||` guard added |
| `integrations.rs` | 555, 565 | `self.tracker_selector.items.is_empty() ||` guard added |

### Silent Error Suppression Fixed

| File | Line | Pattern Fixed |
|------|-------|---------------|
| `cluster.rs` | 436 | `let _ = view_selector.confirm()` → `if .is_none() { warn }` |
| `packet.rs` | 749 | `let _ = view_selector.confirm()` → `if .is_none() { warn }` |
| `load.rs` | 568 | `let _ = test_type_selector.confirm()` → `if .is_none() { warn }` |
| `settings/input.rs` | 189, 213, 224 | proxy_rotation, severity, accent_color selectors |
| `session.rs` | 109, 176 | read_dir and remove_file errors now logged |
| `report.rs` | 461 | view_selector.confirm() now uses proper error handling |

### Test Fix

- **key_handler.rs:440-457**: `clamp_quick_switch_selection()` now re-fetches fresh results via `get_quick_switch_results()` instead of using stale parameter.

## Deep Dive Session Fixes (2026-05-31 Evening)

### settings/input.rs - is_running() Guards

All 8 input handlers now properly guard with `!self.is_running()`:

| Handler | Line |
|---------|------|
| handle_char | 36 |
| handle_backspace | 53 |
| handle_paste | 70 |
| handle_enter | 165 |
| handle_up | 269 |
| handle_down | 316 |
| handle_left | 364 |
| handle_right | 396 |

### Edge Detection is_empty() Guards

| File | Lines | Selector |
|------|-------|----------|
| `stress.rs` | 455, 464 | `type_selector` |
| `workflow.rs` | 524, 533 | `mode_selector` |
| `packet.rs` | 840, 855 | `view_selector` |
| `proxy.rs` | 649, 663 | `view_selector` |
| `cluster.rs` | 552, 570 | `view_selector` |
| `scan_ports.rs` | 491, 500 | InputGroup delegation |
| `scan_endpoints.rs` | 458, 467 | InputGroup delegation |
| `fingerprint.rs` | 405, 414 | InputGroup delegation |

### history.rs is_running() Guard

- **history.rs:431**: Added `is_running()` guard to `handle_char` for hotkeys 'd' (delete) and 'C' (clear all)

### components/input.rs InputGroup Edge Guards

Fixed `InputGroup::is_at_left_edge()` and `is_at_right_edge()` to have proper `is_empty()` guards:

```rust
pub fn is_at_left_edge(&self) -> bool {
    if let Some(idx) = self.focused {
        !self.fields.is_empty() && idx < self.fields.len() && self.fields[idx].is_at_left_edge()
    } else {
        true
    }
}
```

### key_handler.rs Ctrl+V Guard

- **key_handler.rs:65-72**: Added `!app.has_active_task()` guard to Ctrl+V paste handler

### Worker Silent Error Suppression Fixed (88 occurrences)

| File | Count |
|------|-------|
| api.rs | 15 |
| security.rs | 27 |
| recon.rs | 12 |
| network.rs | 13 |
| plugin.rs | 10 |
| scanner.rs | 9 |
| fuzzer.rs | 8 |

### network.rs Log Level Fix

- **network.rs:172**: Changed `tracing::info!` to `tracing::debug!` for successful packet capture completion
