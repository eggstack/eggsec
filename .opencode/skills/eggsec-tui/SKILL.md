# Eggsec TUI Skill

TUI module workflows and patterns for the terminal UI.

## Module Structure

```
crates/eggsec-tui/src/
├── app/          # App state, event loop, command handling
│   ├── mod.rs           # App struct, notifications, helpers
│   ├── state.rs         # OverlayState, SearchState, QuickSwitchState, TaskState, ThemeLoadState
│   ├── tab_store.rs     # TabStore - owns all 33 tab instances
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

## Session Fixes (2026-06-11)

### Theme System Fixes
- **Ctrl+T cycles ALL themes**: Iterates `list_theme_ids_owned()` alphabetically, wrapping at end (was limited to built-in trio)
- **Theme::default() returns cyber-red**: Was `dark_theme`, which disagreed with `ThemeManager::default`
- **set_theme() logs at debug level** when a theme is not found (was silent)
- **Theme install failure notifications**: Surfaced via the notification system (no longer silent)
- **set_items_with_extra on Selector**: Adds missing theme to dropdown without replacing with index 0
- **Content_len cap in archive.rs**: Prevents pathological allocation (1 MiB cap)
- **Style.rs methods**: Annotated `#[allow(dead_code)]` for future adoption

### Session Management Hardening
- **Corrupt session quarantine**: `.json.bad` files tried next in `load_latest_session`
- **Orphan cleanup**: `.json.tmp` orphans cleaned on both save paths
- **Auto-save skips active tasks**: `auto_save_if_due` defers during running tasks
- **Fallback path fix**: `SessionConfig` fallback uses `$HOME/.eggsec/sessions` (was bare `~`)
- **Interval clamp**: `auto_save_interval` clamped to min 1 second
- **Snapshot filtering**: `quick_save.json` excluded from session snapshot candidates

### Key Binding Changes
- `Ctrl+T` cycles all themes (not just built-ins)
- `Ctrl+B` shows "Bookmarked: <tab>" notification
- `Shift+E` shows "Export format: <format>" notification
- `1-9` / `0` jump to tab by index
- `y` / `n` confirm/cancel in confirmation dialog
- `pending_key` cleared on overlay open (fixes stale `gg` after opening quick switch)

## Session Fixes (2026-06-17)

### Theme System Improvements
- **luminance() 3-char hex fix**: `loader.rs` now expands `#FFF` to `#FFFFFF` before computing luminance (was returning 0.5 for all 3-char hex)
- **Dead style methods removed**: `style_for_tab`, `style_for_mode`, `style_for_status` removed from `style.rs` (never called)
- **Dead manager methods removed**: `register_theme_if_absent` (deprecated) and `set_current_by_name` (tests-only) removed from `manager.rs`
- **Theme toggle logging**: `toggle()` now logs debug on `set_theme` failure instead of `let _ =`

### Worker Error Handling
- All `let _ =` on channel sends in `security.rs`, `c2_worker.rs`, `intercept_worker.rs`, `db_pentest.rs` now use `if let Err(e) = ... { tracing::warn!(...) }`

### Dead Code Removal
- **key_handler.rs**: Removed 20 dead shim methods (~180 lines) - `handle_global_shortcuts`, `handle_mode_specific_input`, `handle_normal_mode_input`, `handle_insert_mode_input`, `handle_topmost_overlay`, `handle_ctrl_c`, `handle_ctrl_f`, `handle_escape`, `handle_enter_insert_mode`, `handle_quit`, `handle_reset`, `handle_save_settings`, `handle_delete_entry`, `handle_enter`, `decode_command_palette`, `handle_command_palette`, `decode_overlay_input`, `handle_overlay_input`, `decode_quick_switch`, `handle_quick_switch`
- **overlay.rs**: Removed 3 dead transition shim methods (~25 lines)
- **settings/main.rs**: Removed dead `sync_with_theme` and `sync_theme_selector`

### Session Management
- Quarantine rename and orphan cleanup in `session.rs` now log errors instead of silent `let _ =`

### Policy Enforcement Alignment (2026-06-11)
- TUI now uses the shared `EnforcementContext::evaluate()` (via `App.enforcement` initialized to `manual_permissive` in runner.rs) for **all** target-bearing launches. Matches the CLI model exactly (narrow `--yes` semantics, dedicated `--allow-*` flags, stable kebab audit strings).
- **Central gate**: in `handle_enter` / before `spawn_task` (via `build_current_task` + `build_current_operation_descriptor` producing `OperationDescriptor`).
- **Retroactive gate**: for direct-launch tabs (packet views, stress, cluster, wireless, oauth, nse, hunt, browser, etc.) that start work inside their own `handle_enter`/`run_*` — if they enter `is_running()`, evaluate; on `RequireConfirmation` we stop the tab and open the policy overlay.
- `RequireConfirmation` surfaces via highest-precedence `OverlayType::PolicyConfirm` (backed by `PendingPolicyConfirmation` in confirmation.rs + state.rs, with reason_input for the manual override reason).
- On confirm: builds narrow `ManualOverride` (the confirm itself satisfies out-of-scope/target-expansion; other classes get their specific allow_* flags), re-evaluates via the central `enforcement.evaluate`, records audit via `decision.with_manual_override_record(reason, confirmation_class_strings(...))` using kebab strings from `ConfirmationClass::as_str()`, then spawns the captured `TaskConfig` (or re-dispatches for direct tabs) if permitted.
- `PendingAction` (ResetTab/SaveSettings/DeleteHistoryEntry/ClearHistory) + `ConfirmPopup` overlay remain completely separate and lower precedence (for pure UI actions).
- See: architecture/tui.md (Enforcement Context section), crates/eggsec-tui/src/AGENTS.override.md (Policy Enforcement Alignment), app/{mod,confirmation,state,key_handler,runner}.rs, ui/mod.rs, and the 2026-06-10 manual discretion ergonomics plan (CLI baseline that TUI now mirrors).

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

All 33 tabs now properly guard input handlers with `!self.is_running()`. This prevents input during running state:

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

50+ Halloy-format themes are packaged into the binary via LZMA compression. Packaged theme names are canonicalized to stable IDs, selector labels are human-readable, and the `cyber-red` fallback theme is always available in-code, independent of file system access.

New code should prefer explicit `&Theme` parameters:
```rust
pub fn draw_widget(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let style = Style::default().fg(theme.colors.text);
}
```

For tab renderers and components, `tc!` macro is still valid:
```rust
use crate::tc;
let style = Style::default().fg(tc!(text));
```

Semantic colors: `primary`, `secondary`, `accent`, `background`, `text`, `text_dim`, `success`, `warning`, `error`, `info`.

The Settings tab has a theme selector dropdown. `Ctrl+T` cycles the built-in theme trio only, while the selector exposes canonical values with readable labels. Theme loading runs in a background thread; `ThemeLoadState` keeps the receiver, join handle, and deferred restore request together so startup stays non-blocking. After modifying `themes/*.toml`, run `python3 scripts/package_themes.py` to regenerate `crates/eggsec-tui/src/theme/packaged.rs`. The script is deterministic.

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
cargo test --lib -p eggsec-tui tui::
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
1. Create tab module in `crates/eggsec-tui/src/tabs/`
2. Implement `TabState`, `TabInput`, `TabRender` traits
3. Add tab to `Tab` enum in `tabs/mod.rs`
4. Add tab instance to `TabStore` in `app/tab_store.rs`
5. Add rendering in `ui/shell.rs` `draw_content()`
6. Add to `App::dispatcher_mut()` for event routing

### Fixing Layout Issues
1. Check for fixed `Constraint::Length` values
2. Replace with dynamic constraints based on `area.height`
3. Test at 80x24 and smaller terminals
4. Run `cargo test --lib -p eggsec-tui tui::`

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
- `crates/eggsec-tui/src/AGENTS.override.md` - Detailed TUI patterns
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
    PolicyConfirm,  // Highest priority — RequireConfirmation from EnforcementContext (PendingPolicyConfirmation with reason input)
    ConfirmPopup,   // PendingAction for UI actions (reset/save/delete/clear)
    CommandPalette,
    QuickSwitch,
    Search,
    HttpOptions,
    Help,           // Lowest priority
}
```

`PolicyConfirm` is handled first in `key_handler.rs` (`handle_topmost_overlay`, `handle_enter` wrapper, `handle_escape`). It is backed by `PendingPolicyConfirmation` (message + reason_input + captured `TaskConfig` or direct-tab context). Confirming builds a narrow `ManualOverride`, re-evaluates via the central `enforcement.evaluate`, records audit with kebab `confirmation_class_strings`, and proceeds only if permitted. `ConfirmPopup` (for `PendingAction`) is now second-highest.

## Confirmation System

Two separate confirmation flows (they do not share state or precedence):

1. **PendingAction** (pure UI / destructive actions: reset tab, save settings, delete history entry, clear history):
   ```rust
   app.request_confirmation(PendingAction::ResetTab);
   // Later: app.confirm_action() or app.cancel_action()
   ```
   Renders via `ConfirmPopup`. Lower precedence than `PolicyConfirm`. `y` / `n` or Enter/Esc in the dialog.

2. **PendingPolicyConfirmation** (policy / `RequireConfirmation` outcome from `EnforcementContext::evaluate` on target-bearing operations):
   - Automatically triggered by the central gate (in `handle_enter` before `spawn_task`, via `build_current_operation_descriptor`) and retroactive gate (in `update()` for direct-launch tabs like packet/stress/cluster/wireless/oauth/nse/hunt/browser that start inside their own handlers).
   - `app.request_policy_confirmation(descriptor, decision, required_classes, captured_task_config)`.
   - Rich message (from `PendingPolicyConfirmation::message()`) includes operation, risk, target, kebab-case confirmation classes (via `ConfirmationClass::as_str()` + `confirmation_class_strings`), denied_reasons/warnings, and a live reason input line (analogous to `--manual-override-reason`).
   - Typing edits the reason; Enter = confirm (builds narrow `ManualOverride` — the act of confirming satisfies low-risk scope classes; other classes get their dedicated `allow_*` flags), Esc = cancel (no launch).
   - On successful confirm: `tracing::warn!` audit line, `with_manual_override_record`, notification, then spawn the captured task (or re-dispatch for direct tabs).
   - Highest precedence overlay (`OverlayType::PolicyConfirm`).

See: `app/confirmation.rs` (both enums), `state.rs`, `key_handler.rs:205+`, `mod.rs:322+` (gates + `request/confirm/cancel_policy_action` + `build_current_operation_descriptor`), `runner.rs:82` (init), `ui/mod.rs` (render), `architecture/tui.md` (Enforcement Context), and `crates/eggsec-tui/src/AGENTS.override.md`. This mirrors the CLI `evaluate_and_enforce_operation` + narrow MO semantics from the 2026-06-10 ergonomics cleanup (f245db52).

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

Files fixed (2026-05-31 session): api.rs (15), fuzzer.rs (8), network.rs (13), recon.rs (12), scanner.rs (9), security.rs (27)

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
| scanner.rs | 9 |
| fuzzer.rs | 8 |

### network.rs Log Level Fix

- **network.rs:172**: Changed `tracing::info!` to `tracing::debug!` for successful packet capture completion

## Deep Dive Session (2026-06-01)

### Additional TUI Component Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `selector.rs` | 228 | Silent `let _ =` on confirm() | `if .is_none() { warn }` pattern |
| `palette.rs` | 60 | Direct array access | `.get()` with bounds check |

### TUI session.rs Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `session.rs` | 113 | `debug!` instead of `warn!` | `tracing::warn!` |
| `session.rs` | 174 | `filter_map(\e\| e.ok())` | Explicit match with warn |

### Tab Edge Detection Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `graphql.rs` | 490-502 | Options checkbox bounds missing | Added explicit Options case |
| `oauth.rs` | 534-546 | Options checkbox bounds missing | Added explicit Options case |
| `vuln.rs` | 618-619 | `is_at_right_edge()` missing `is_empty()` guard | Added `items.is_empty() \|\|` guard |
| **`vuln.rs`** | **603-614** | **`is_at_left_edge()` missing `is_empty()` guard** | **Added `items.is_empty() \|\|` guard for Mode selector** |

### handle_enter() is_running() Guards

| File | Line | Status |
|------|-------|--------|
| `report.rs` | 457 | ✅ Fixed |
| `nse.rs` | 311 | ✅ Fixed (removed inverted guard) |

### Other Tab Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `workflow.rs` | 411 | `handle_copy()` missing guard | Added `!self.is_running()` guard |
| `workflow.rs` | 257 | `reset()` doesn't clear mode | Reset `current_mode` |
| `integrations.rs` | 280 | `reset()` doesn't clear selector | Reset `tracker_selector.selected` |
| `storage.rs` | 250 | `reset()` doesn't clear fields | Added fields.clear() loop |
| `load.rs` | 377 | `reset()` doesn't clear selector | Added `test_type_selector.select(0)` |
| `stress.rs` | 206 | `reset()` doesn't clear selector | Added `type_selector.select(0)` |
| `proxy.rs` | 660-669 | `is_at_right_edge()` missing `is_open()` guard | Added `is_open()` check |
| `proxy.rs` | 624-640 | `handle_left/right()` missing `is_open()` check | Added `is_open()` guard |

### selector.rs handle_left Empty Items Guard

Added `!self.items.is_empty()` guard to `handle_left()` for consistency with `handle_right()` (line 247).

## Session Fixes (2026-05-26 Session)

### TUI Edge Detection Fixes

| File | Lines | Fix |
|------|-------|-----|
| `input.rs` | 684-698 | Wrapped `can_move_left()` and `can_move_right()` in `if !self.fields.is_empty()` check |

### Non-TUI Module Fixes

| File | Line | Fix |
|------|------|-----|
| `tool/protocol/rest.rs` | 260 | Silent WS channel send fixed |
| `tool/agents/lifecycle.rs` | 341 | Silent event send fixed |
| `distributed/remote.rs` | 116 | Silent shutdown send fixed |
| `scanner/ports/mod.rs` | 580 | `debug!` → `warn!` for progress dropped |
| `scanner/fingerprint.rs` | 306 | `debug!` → `warn!` for progress dropped |
| `scanner/endpoints.rs` | 828 | `debug!` → `warn!` for progress dropped |
| `scanner/ports/spoofed.rs` | 451 | `debug!` → `warn!` for progress dropped |
| `scanner/templates/marketplace.rs` | 208-209 | Silent filter_map fixed |
| `recon/git_secrets.rs` | 287 | Silent filter_map fixed |

### key_handler.rs Ctrl+x Guard

- **key_handler.rs:48**: Added `if !app.has_active_task(")` guard to Ctrl+x (quick switch) to prevent activation during running tasks

## Load Tab Fixes (2026-05-26 Evening Session)

### Edge Detection is_empty() Guards

| File | Lines | Selector/Field |
|------|-------|----------------|
| `load.rs` | 652-665, 667-681 | `test_type_selector` (is_empty guards added) |

### load.rs update_progress validation

- **load.rs:321-324**: Fixed validation in `update_progress()` - added `completed.min(total)` and `total.max(1)` guards to prevent invalid progress values.

### network.rs Worker Timeout Wrappers

- **network.rs:9-46**: Added timeout wrapper (300s) to `run_load_test()` - `tokio::time::timeout()` with proper error handling
- **network.rs:85-98**: Added timeout wrapper (600s) to `run_stress_test()` - `tokio::time::timeout()` with proper error handling
- **network.rs:22-24**: Added initial progress send `(0, requests)` at start of load test
- **network.rs:27-36**: Restructured error handling to convert `EggsecError` to `anyhow::Error` for compatibility

## Deep Dive Session Fixes (2026-06-01)

### settings/main.rs Fixes

| File | Lines | Issue | Fix |
|------|-------|-------|-----|
| `settings/main.rs` | 311-347 | `apply_to_config()` unsafe direct field access | Changed to safe `.get()` pattern with bounds checks |
| `settings/main.rs` | 400,523,595 | Silent file write errors | Added `if let Err(e) = ...` with status_message |

### Tab handle_enter() Restructuring

| File | Lines | Issue | Fix |
|------|-------|-------|-----|
| `report.rs` | 457-487 | `handle_enter()` returns early when not running | Restructured to allow selector interaction when idle + proper start/stop |
| `nse.rs` | 311-340 | `handle_enter()` logic issue with Results + is_running | Restructured to properly handle blur/selector |

### Missing is_running() Guards on handle_enter

| File | Lines | Status |
|------|-------|--------|
| `graphql.rs` | 415-432 | ✅ Added `!self.is_running()` guard |
| `oauth.rs` | 459-476 | ✅ Added `!self.is_running()` guard |
| `recon.rs` | 591-596 | ✅ Added `!self.is_running()` guard |

### Edge Detection Fixes

| File | Lines | Issue | Fix |
|------|-------|-------|-----|
| `recon.rs` | 670-671 | Missing `is_empty()` guard on `is_at_left_edge()` | Added `self.option_checkboxes.is_empty() \|\|` |
| `scrollable.rs` | 99-106 | `is_at_left_edge/is_at_right_edge` inconsistent | Added `is_empty()` guards to both methods |

### Input/Render Fixes

| File | Lines | Issue | Fix |
|------|-------|-------|-----|
| `stress.rs` | 263-267 | Direct array access `input_chunks[i]` | Changed to `.get(i)` pattern |
| `stress.rs` | 390-404 | `handle_enter()` result not captured | Changed to `confirm().is_none()` pattern |
| `storage.rs` | 339 | Direct array access `query_chunks[0]` | Changed to `.get(0)` pattern |
| `integrations.rs` | 335 | Suspicious fallback `&[]` in slice access | Changed to `&self.issue_inputs.fields` |

### Other Tab Fixes

| File | Lines | Issue | Fix |
|------|-------|-------|-----|
| `vuln.rs` | 495-505 | `handle_copy()` missing `is_running()` guard | Added `!self.is_running()` guard + improved logic |
| `history.rs` | 441,443 | Empty handlers missing `is_running()` guards | Added `!self.is_running()` guards |
| `auth.rs` | 227-229 | `fields.len() - 1` underflow risk | Added `!self.inputs.fields.is_empty()` guard |

### Worker/App Fixes

| File | Lines | Issue | Fix |
|------|-------|-------|-----|
| `task_runtime.rs` | 72-76 | Silent error suppression `Err(_e)` | Changed to `if let Err(e) = ...` using actual error |

### Tool/AI Module Fixes

| File | Lines | Issue | Fix |
|------|-------|-------|-----|
| `session.rs` | 525, 1016 | Silent error suppression `unwrap_or_default()` | Changed to `unwrap_or_else(\|e\| { warn!; String::new() })` |
| `state.rs` | 217 | `debug!` instead of `warn!` for file removal | Changed to `tracing::warn!` |
| `cache.rs` | 278 | `debug!` instead of `warn!` for cache dir creation | Changed to `tracing::warn!`

## Session Fixes (2026-06-03 - Deep Dive Audit)

### fuzz.rs Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `fuzz.rs` | 419-425 | `reset()` didn't clear 7 checkboxes | Added graphql_introspection, graphql_depth_bypass, graphql_alias_overload, oauth_redirect_test, oauth_scope_test, oauth_state_test, oauth_grant_test resets |
| `fuzz.rs` | 744-747 | `handle_enter()` MutationCheckbox toggle missing guard | Added `!self.is_running()` guard |

### hunt.rs and browser.rs Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `hunt.rs` | 229-243 | `reset()` didn't clear option_checkboxes, focused_checkbox_index, focus_area | Added loop to reset checkboxes, reset index and focus_area |
| `browser.rs` | 195-209 | Same issue | Same fix pattern |

### stress.rs Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `stress.rs` | 390-406 | `handle_enter()` missing `is_running()` guard | Added guard at start to stop running tasks |
| `stress.rs` | 459-471 | `is_at_left_edge()` `true` fallback without `is_empty()` check | Changed to check `items.is_empty()` even when selector closed |
| `stress.rs` | 473-486 | `is_at_right_edge()` `true` fallback without `is_empty()` check | Same fix pattern |

### scan_ports.rs Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `scan_ports.rs` | 463-471 | `handle_enter()` early return when inputs focused skipped `is_running()` check | Restructured to check `is_running()` even when inputs were focused |

### Navigation Handler Guards (16 tabs fixed via subagents)

| Tab | Handlers Fixed |
|-----|---------------|
| `scan.rs` | 8 handlers |
| `scan_ports.rs` | 8 handlers |
| `fingerprint.rs` | 10 handlers |
| `waf.rs` | 12 handlers |
| `waf_stress.rs` | 11 handlers |
| `graphql.rs` | 8 handlers |
| `oauth.rs` | 8 handlers |
| `cluster.rs` | 9 handlers |
| `proxy.rs` | 13 handlers |
| `nse.rs` | 8 handlers |
| `hunt.rs` | 6 handlers |
| `browser.rs` | 6 handlers |
| `report.rs` | 12 handlers |
| `vuln.rs` | 2 handlers |
| `integrations.rs` | 1 handler (handle_copy) |
| `recon.rs` | 5 handlers |

### storage.rs Bounds Fix

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `storage.rs` | 317-322 | Direct `config_chunks[0]` access without bounds check | Changed to `if let Some(chunk) = config_chunks.first()` pattern |

### Additional Audit Findings (Not Fixed - Low Priority)

| File | Line | Issue | Severity |
|------|------|-------|----------|
| `tool/protocol/mcp/handlers/server.rs` | 35-36, 58-59 | Uses HashMap instead of FxHashMap | Low - hot path but not critical |
| `fuzzer/detection/analyzer.rs` | 231-234 | Potential panic on empty/single-element vector | Medium - guarded by earlier is_empty check at line 212 |
| `recon/subdomain.rs` | 178, 240, 262 | Silent `ok()` on semaphore/acquire/join | Low - test/fallback code |
(End file - total 1003 lines)

## Session Fixes (2026-06-03 - Additional Audit)

### Additional Tabs Fixed (Second Wave)

| Tab | Handlers Fixed |
|-----|---------------|
| `recon.rs` | word_forward, word_backward (lines 549-558) |
| `scan.rs` | word_forward, word_backward, home, end, top, bottom (lines 472-507) |
| `scan_ports.rs` | word_forward, word_backward, home, end (lines 426-451) |
| `scan_endpoints.rs` | word_forward, word_backward, home, end (lines 369-394) |
| `fingerprint.rs` | word_forward, word_backward, home, end, top, bottom (lines 323-370) |
| `load.rs` | word_forward, word_backward, home, end, top, bottom (lines 527-561) |
| `stress.rs` | word_forward, word_backward, home, end, top, bottom (lines 353-387) |
| `cluster.rs` | handle_enter (line 440) |
| `proxy.rs` | handle_enter (line 591) |
| `hunt.rs` | handle_enter (Options), handle_up/down (Options) (lines 444-488) |
| `browser.rs` | handle_enter (Options), handle_up/down (Options) (lines 407-451) |
| `compliance.rs` | handle_top, handle_bottom, handle_left, handle_right (lines 343-421) |
| `vuln.rs` | handle_top, handle_bottom (lines 540-551) |
| `dashboard.rs` | All 17 TabInput handlers (lines 492-562) |
| `resume.rs` | 12 navigation handlers (lines 195-272) |
| `history.rs` | 10 navigation handlers (lines 418-517) |

### reset() Methods Fixed (17 tabs)

| Tab | Added Reset |
|-----|------------|
| `packet.rs` | view_selector |
| `graphql.rs` | checkbox reset, focused_checkbox_index |
| `oauth.rs` | checkbox reset, focused_checkbox_index |
| `cluster.rs` | view_selector, worker/coordinator/status_inputs |
| `proxy.rs` | view_selector |
| `nse.rs` | input fields |
| `hunt.rs` | checkbox reset, focused_checkbox_index, focus_area |
| `browser.rs` | checkbox reset, focused_checkbox_index, focus_area |
| `compliance.rs` | framework_selector.select(0), focus_area |
| `storage.rs` | mode_selector, query_inputs, focus_area, current_mode |
| `integrations.rs` | config_inputs, issue_inputs |
| `workflow.rs` | focus_area |
| `vuln.rs` | mode_selector, focus_area, current_mode |
| `report.rs` | view_selector, format_selector, current_view |
| `settings/main.rs` | proxy_rotation_selector, severity_selector, accent_color |
| `auth.rs` | results.clear() |

### App Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `mod.rs` | 452, 459 | Silent `let _ =` on dispatcher | Changed to `if !bool { warn }` |
| `key_handler.rs` | 407-414 | Stale quick switch results | Re-fetch fresh results on Enter |
| `task_runtime.rs` | 68-80 | No timeout on spawn | Added `tokio::time::timeout(300s, ...)` |

### Workers/Config Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `workers/api.rs` | 143 | Division by zero | Added `.max(1)` guard |
| `config/loader.rs` | 18 instances | Silent file operations | Changed to `if let Err(e) = ...` with warn |

### Output Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `output/markdown.rs` | 87 | to_lowercase() in loop | Cached before loop |
| `output/dedup.rs` | 16 | to_lowercase() in parse | Changed to eq_ignore_ascii_case |

### Tool Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `tool/scripting.rs` | 23,24,68 | HashMap → FxHashMap | Changed to FxHashMap |
| `tool/finding.rs` | 19 | HashMap → FxHashMap | Changed to FxHashMap |
| `tool/agents/aggregator.rs` | Multiple | HashMap → FxHashMap | Changed to FxHashMap |
| `tool/agents/registry.rs` | 28 | HashMap → FxHashMap | Changed to FxHashMap |
| `tool/openapi.rs` | Multiple | HashMap → FxHashMap | Changed to FxHashMap |
| `tool/ratelimit.rs` | Multiple | HashMap → FxHashMap | Changed to FxHashMap |
| `tool/implementations/*.rs` | Various | HashMap → FxHashMap | Changed to FxHashMap |
| `routes.rs` | 28, 118 | unwrap_or_default | Added warn logging |
| `implementations/scanner.rs` | 129,148,175 | load_config unwrap | Added inspect_err with warn |

### Scanner Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `scanner/templates/matcher.rs` | 262, 268 | Silent socket ops | Added warn logging |
| `scanner/fingerprint.rs` | 432 | Silent probe write | Added warn logging |
| `recon/whois.rs` | 171 | Silent timeout | Added warn logging |

### Additional Audit Findings (Low Priority)

| File | Line | Issue | Severity |
|------|------|-------|----------|
| `tool/protocol/mcp/handlers/server.rs` | 35-36, 58-59 | HashMap vs FxHashMap | Low |
| `fuzzer/detection/analyzer.rs` | 231-234 | Empty vector panic risk | Medium (guarded) |
| `recon/subdomain.rs` | 178,240,262 | Silent ok() | Low |


## Session Fixes (2026-06-04)

### App Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `task_runtime.rs` | 83-91 | Timeout case did not abort JoinHandle | Added `handle.abort()` after timeout error |
| `state_update.rs` | 68 | Unhandled TaskResult warning missing context | Changed to `tracing::warn!("Unhandled TaskResult variant: {:?}", result);` |

### TUI Deep Dive Audit (2026-06-04)

Completed comprehensive audit of all 29 TUI tabs across 6 groups. Fixed ~100+ issues:

| Group | Tabs | Fixes |
|-------|------|-------|
| Group 1 | recon, scan, scan_ports, scan_endpoints, fingerprint | 21 navigation guards + handle_enter logic + reset() |
| Group 2 | fuzz, waf, waf_stress, load, stress | 40 navigation guards |
| Group 3 | packet, graphql, oauth, cluster, proxy | 13 fixes (bounds, guards, logic) |
| Group 4 | nse, hunt, browser, compliance | 15+ fixes |
| Group 5 | storage, integrations, workflow, vuln, report | 8 major fixes |
| Group 6 | history, settings | 18 navigation guards + reset fields |

Key patterns fixed:
- **handle_enter() logic**: 8 tabs where blur happened BEFORE stop (should be stop → blur → start)
- **Navigation handlers**: ~97 missing `!self.is_running()` guards across 17 tabs
- **reset() methods**: 11 tabs now properly reset checkbox/selector state
- **Edge detection**: 5 tabs added missing `is_empty()` guards

## Session Fixes (2026-06-05 - Additional Audit)

### Additional Bugs Fixed

| Category | File | Line | Issue | Fix |
|----------|------|------|-------|-----|
| **Direct array access** | recon.rs:602 | Direct `[]` access | `.get_mut()` with is_empty guard |
| **Direct array access** | scan.rs:307-309 | Direct `[]` access | `.get()` with len >= 2 guard |
| **Direct array access** | scan_ports.rs:284-295 | Direct `[]` access | `.get_mut()` for fields 1-3 |
| **Direct array access** | scan_endpoints.rs:255-262 | Direct `[]` access | `.get_mut()` for fields 1-2 |
| **Direct array access** | fingerprint.rs:212-219 | Direct `[]` access | `.get_mut()` for fields 1-2 |
| **Direct array access** | waf.rs:524-528 | Direct `[]` access | `.get_mut()` with is_empty guard |
| **Direct array access** | stress.rs:195-206 | Improper bounds | `len() >= 4` before any field access |
| **Direct array access** | settings/main.rs:267-307 | Direct `[]` access | `.get_mut()` for all field access |
| **Direct array access** | settings/main.rs:458-488 | Direct `[]` access | `.get_mut()` for all field access |
| **Edge detection** | graphql.rs:510-524 | Missing is_empty guard | Added `is_empty()` guard for Options |
| **handle_enter() blur order** | compliance.rs:355-370 | blur → stop | stop → blur → start |
| **handle_enter() blur order** | storage.rs:528-552 | blur → stop | stop → blur → start |
| **handle_enter() blur order** | resume.rs:256-264 | blur → stop | stop → blur → start |
| **handle_enter() blur order** | report.rs:469-503 | blur → stop | stop → blur → start |
| **to_lowercase caching** | history.rs:197-200 | In filter closure | Pre-compute per entry before filter |
| **to_lowercase caching** | dashboard.rs:197,228-229 | Called twice per entry | Cached once per entry in entry_lowers |
| **Suspicious fallback** | integrations.rs:334-337 | Wrong fallback | Changed to `&[]` |

### Components Fixed

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `scrollable.rs` | 104 | `is_at_right_edge()` returns `true` when empty | Now returns `horizontal_offset == 0` for consistency |
| `palette.rs` | 39 | Direct `chunks[2]` access | `.get()` pattern |
| `popup.rs` | 39 | Direct `chunks[2]` access | `.get()` pattern |

### Config Module Fixed

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `scope.rs` | 45 | `HashSet` instead of `FxHashSet` | Changed to `FxHashSet` |


## Session Fixes (2026-06-06 - Deep Dive Audit)

### TUI Tab Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `fingerprint.rs` | 373 | `handle_enter()` blur before stop | Reordered to stop → blur → start |
| `scan_ports.rs` | 422 | `handle_copy()` missing `is_running()` guard | Added guard |
| `scan_ports.rs` | 296 | `reset()` doesn't reset `udp_checkbox` | Added `udp_checkbox.checked = false` |
| `scan_endpoints.rs` | 359 | `handle_copy()` missing `is_running()` guard | Added guard |
| `scan_endpoints.rs` | 263 | `reset()` doesn't reset `include_404_checkbox` | Added `checkbox.checked = true` |
| `fuzz.rs` | 824,845 | `handle_left/right` missing `is_running()` guard | Added early return |
| `waf.rs` | 572,587 | `handle_left/right` missing guard + `is_empty()` + wrong fallback | Fixed all issues |
| `waf.rs` | 296 | `reset()` doesn't reset `mode_radio` | Added `mode_radio.select(0)` |
| `waf_stress.rs` | 358,367 | `handle_left/right` missing guard | Added early return |
| `load.rs` | 527-562 | 6 navigation handlers missing `is_running()` guard | Added guards |
| `load.rs` | 367 | Direct array access in reset() | Changed to `.get_mut()` pattern |
| `stress.rs` | 207 | `reset()` missing `focus_area` reset | Added reset |
| `packet.rs` | 726-761 | 6 navigation handlers missing guard | Added `is_running()` guards |
| `graphql.rs` | 391-427 | 6 navigation handlers missing guard | Added `is_running()` guards |
| `graphql.rs` | 165 | `reset()` doesn't clear input fields | Added field clearing loop |
| `oauth.rs` | 439-474 | 6 navigation handlers missing guard | Added `is_running()` guards |
| `oauth.rs` | 200 | `reset()` doesn't reset checkboxes | Added checkbox resets |
| `cluster.rs` | 205 | `reset()` doesn't clear InputGroups | Added clearing loops |
| `proxy.rs` | 388 | `reset()` doesn't reset `current_view` and `view_selector` | Added resets |
| `proxy.rs` | 725-759 | 6 duplicate navigation methods missing guard | Added `is_running()` guards |
| `nse.rs` | 324 | `handle_enter()` early return issue | Restructured to remove early returns |
| `hunt.rs` | 229 | `reset()` doesn't reset checkboxes | Added checkbox reset loop |
| `browser.rs` | 195 | `reset()` doesn't reset checkboxes | Added checkbox reset loop |
| `compliance.rs` | 345 | `handle_top/bottom` missing `is_running()` guard | Added guards |
| `report.rs` | 224 | `reset()` incomplete - selectors, view, inputs not reset | Added all resets |
| `settings/input.rs` | 100-162 | 6 navigation handlers missing `is_running()` guard | Added guards |

### Workers Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `recon.rs` | 166 | Silent `let _ =` on progress send | Changed to `if let Err(e) = ...` |
| `recon.rs` | 62 | `pipeline.run()` missing timeout | Added 300s timeout wrapper |
| `scanner.rs` | 37 | `scan_ports()` missing timeout | Added 60s timeout wrapper |
| `scanner.rs` | 89 | `scan_endpoints()` missing timeout | Added 60s timeout wrapper |
| `scanner.rs` | 133 | `fingerprint_services()` missing timeout | Added 60s timeout wrapper |
| `fuzzer.rs` | 89 | `engine.run_return_session()` missing timeout | Added 60s timeout wrapper |
| `fuzzer.rs` | 110 | `detector.detect()` missing timeout | Added 30s timeout wrapper |
| `fuzzer.rs` | 153 | `bypass_engine.run_bypasses()` missing timeout | Added 60s timeout wrapper |
| `fuzzer.rs` | 200 | `fuzzer_run_waf_stress()` missing timeout | Added 60s timeout wrapper |
| `security.rs` | 23 | `run_hunt()` missing timeout | Added 60s timeout wrapper |
| `security.rs` | 45 | `run_browser_scan()` missing timeout | Added 60s timeout wrapper |
| `security.rs` | 205 | `generate_compliance_report()` missing timeout | Added 60s timeout wrapper |
| `network.rs` | 279 | `traceroute.run()` missing timeout | Added 60s timeout wrapper |

### AI Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `cache.rs` | 168-175 | Cache merge logic prefers NEW instead of OLD | Changed to `entry().or_insert_with()` |
| `cache.rs` | 323-337 | CacheKeyBuilder doesn't sanitize null bytes | Added `.replace('\x00', "")` |
| `planner.rs` | 410-416 | `to_lowercase()` in sentence loop | Cached before loop |
| `planner.rs` | 446-458 | `to_lowercase()` per stage in filter | Cached inside closure |

### Scanner Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `spoofed.rs` | 353,391-396 | Counter increments on failed send | Only increment on success |

### Tool Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `scripting.rs` | 6,23,24,68,76 | `HashMap` → `FxHashMap` | Changed to FxHashMap |
| `orchestrator/mod.rs` | 229 | Silent `let _ =` on progress send | Changed to `if let Err(e) = ...` |

### Components Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `scrollable.rs` | 70 | `scroll_right()` missing `is_empty()` guard | Added explicit empty check |
| `selector.rs` | 228 | Misleading `warn!` for valid `None` | Changed to `debug!` level |

### App Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `state_update.rs` | 67 | `handle_security_result` called twice in chain | Changed second to `handle_feature_result` |


## Session Fixes (2026-06-08 - Additional Audit)

### App Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `state_update.rs` | 67 | Duplicate `handle_protocol_result` call | Fixed to call `handle_feature_result` |
| `task_runtime.rs` | 102-104 | Timeout case aborts inner but not outer handle | Only inner needs abort; outer completes with error |

### Workers Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `fuzzer.rs` | 200 | Silent timeout with `let _ =` | Proper match with warn logging |

### Scanner Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `ports/mod.rs` | 589-591 | Silent `catch_unwind` without comment | Added comment explaining why + warn logging |

### Tool Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `session.rs` | 655,675,698,1007 | HashMap in function signatures | Changed to `FxHashMap` |
| `session.rs` | 636 | Dead code `let _ = step` | Added placeholder comment |

### TUI Tab Fixes (Deep Dive Audit 2026-06-08)

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `workflow.rs` | 313-318 | Hardcoded field indices without bounds | Added `idx < fields.len()` guards |
| `settings/main.rs` | 458-499 | `reset()` incomplete | Added focus_area, current_section, detail_focus_index resets |
| `load.rs` | 649-672 | handle_left/right missing guard | Added `is_running()` guard |
| `hunt.rs` | 507-535 | handle_left/right missing guard | Added `is_running()` guard |
| `browser.rs` | 470-498 | handle_left/right missing guard | Added `is_running()` guard |
| `scan_ports.rs` | 507-512 | handle_left/right missing guard | Added `is_running()` guard |
| `scan_ports.rs` | 364-370 | handle_focus_next/prev missing guard | Added `is_running()` guard |
| `scan_endpoints.rs` | 471-476 | handle_left/right missing guard | Added `is_running()` guard |
| `fingerprint.rs` | 411-416 | handle_left/right missing guard | Added `is_running()` guard |
| `recon.rs` | 680 | Underflow risk `len() - 1` | Added `is_empty()` check |
| `scan.rs` | 469 | handle_copy missing guard | Added `is_running()` guard |
| `waf.rs` | 530 | ModeRadio case missing guard | Added `!self.is_running()` |
| `report.rs` | 337-379 | handle_focus_next/prev missing guard | Added `is_running()` guard |
| `report.rs` | 518-531 | handle_escape has unusual guard | Removed inappropriate guard |
| `integrations.rs` | 334 | Suspicious `&[]` fallback | Added warn logging on fallback |
| `auth.rs` | 50-56 | reset() missing focus_area | Added `focus_area` reset |
| `history.rs` | 167-187 | to_lowercase() in loops | Pre-compute before filter |


## Session Fixes (2026-06-09)

### AI Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `cache.rs` | 158-180 | Cache merge logic broken | Fixed merge to preserve existing entries |
| `cache.rs` | 158-180 | Unnecessary complexity | Simplified to direct iteration |

### App Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `task_runtime.rs` | 103 | Misleading timeout message | Changed to "aborting task" |
| `mod.rs` | 452-463 | Misleading warn logs | Changed to debug level |
