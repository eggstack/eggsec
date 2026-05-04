# TUI Improvement Plan

## Status: IN PROGRESS (Most phases completed)

## Completed Work

### Phase 1: Architectural Refactoring

**1.1 Tab Registry**: PARTIAL - Direct tab field access retained
- TabRegistry implementation explored but not integrated (dead code removed)
- Direct tab field access maintained for App struct
- state_update.rs uses direct tab field access (not registry)

**1.2 vertical_list_state**: NOT NEEDED - Field was already removed or never existed

### Phase 2: File Splitting / Maintainability

**2.1 Split app/mod.rs**: COMPLETE
- `app/notifications.rs` - Notification struct, NotificationSeverity
- `app/bookmarks.rs` - Bookmark helper functions
- `app/confirmation.rs` - PendingAction enum

**2.2 Extract Key Handlers**: COMPLETE
- `app/key_handler.rs` - KeyHandler struct with organized methods

**2.3 Help Content**: COMPLETE
- `app/help_config.rs` - Static help data extracted from help.rs

**2.4 Standardize Naming**: COMPLETE
- WafTab::set_detection_result() → set_results()

### Phase 3: UX/Usability

**3.1 Focus Indicators**: COMPLETE
- Theme colors `focus_normal`, `focus_input`, `focus_results` added
- InputField uses `focus_input` when focused

**3.2 Escape Behavior**: COMPLETE
- Mode indicator implemented in draw_status_bar() (ui.rs:737-754)
- Shows NORMAL/INSERT mode with color-coded badge
- Theme colors mode_normal/mode_insert defined for both dark/light themes

**3.3 Breadcrumb Bar**: ALREADY EXISTS - No work needed
- `ui.rs` already has `draw_breadcrumb()` function

**3.4 Quick Switch Panel**: COMPLETE
- Ctrl+G shows bookmarked tabs with fuzzy search
- `toggle_quick_switch()`, `close_quick_switch()` methods added

### Phase 4: Styling/Theming

**4.1 Theme Customization in Settings**: COMPLETE
- Theme section added to Settings with dark/light toggle
- 8 accent colors available
- `set_dark_mode()`, `set_accent_color()` methods added

### Phase 5: Error Handling

**5.1 Standardize Error Handling**: COMPLETE
- Empty catch-alls replaced with user-facing notifications
- `set_notification(msg, NotificationSeverity::Error)` for user-visible errors

---

## Remaining Work

### 4.3 Derive Help from Tab State: DEFERRED
- Would require updating `key_hints()` trait method across all tabs
- Current hardcoded help is functional

### 4.4 Command Palette Styling: DEFERRED
- Low priority - existing styling is adequate

---

## File Reference (Updated)

### Core TUI Files
- `crates/slapper/src/tui/app/mod.rs` - App struct (reduced size)
- `crates/slapper/src/tui/app/runner.rs` - Event loop (uses KeyHandler)
- `crates/slapper/src/tui/app/key_handler.rs` - Key handling methods
- `crates/slapper/src/tui/app/state_update.rs` - Background task handling
- `crates/slapper/src/tui/app/notifications.rs` - Notification types
- `crates/slapper/src/tui/app/bookmarks.rs` - Bookmark helpers
- `crates/slapper/src/tui/app/confirmation.rs` - PendingAction
- `crates/slapper/src/tui/app/help_config.rs` - Static help content

### Components
- `crates/slapper/src/tui/components/input.rs` - InputField with focus colors
- `crates/slapper/src/tui/components/selector.rs`
- `crates/slapper/src/tui/components/popup.rs`
- `crates/slapper/src/tui/components/palette.rs`
- `crates/slapper/src/tui/components/help_bar.rs`

### Other
- `crates/slapper/src/tui/help.rs` - Help system (content moved to help_config.rs)
- `crates/slapper/src/tui/ui.rs` - Main rendering, breadcrumbs already exist
- `crates/slapper/src/tui/theme.rs` - Theming with focus colors

---

## Success Criteria (Updated)

- ✅ app/mod.rs reduced (split into modules)
- ✅ runner.rs uses KeyHandler for organized key handling
- ✅ Focus indicators added to theme and InputField
- ✅ Mode indicator (NORMAL/INSERT) implemented in status bar
- ✅ Quick switch panel (Ctrl+G) implemented
- ✅ Escape behavior unified
- ✅ Theme customization exposed in Settings
- ✅ Error handling shows notifications to users
- ✅ Help content extracted to data file
- ✅ Direct tab field access maintained for render/input handling (acceptable)

---

## Testing Commands

```bash
cargo check --lib -p slapper
cargo test --lib -p slapper
cargo clippy --lib -p slapper
```