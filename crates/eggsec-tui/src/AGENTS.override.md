# TUI Module Override

Specialized guidance for the terminal UI module.

## Policy Enforcement Alignment (2026-06-11)

TUI now shares the exact `EnforcementContext`/`RequireConfirmation`/`ManualOverride` model as CLI. All target-bearing launches gated by central `enforcement.evaluate` before spawn (app/mod.rs:322, via `build_current_operation_descriptor`). For direct-launch tabs, `handle_enter()` evaluates policy BEFORE calling the dispatcher, so Deny/RequireConfirmation blocks before any side effect starts (the old post-dispatch retroactive gate has been removed). Wireless active deauth/disassoc special-cases dry-run as `SafeActive` so it launches without a prompt; live mode remains `Intrusive` and uses the same policy confirmation overlay. `RequireConfirmation` uses highest-precedence `OverlayType::PolicyConfirm` (mod.rs:1095) + `PendingPolicyConfirmation` (confirmation.rs:59, state.rs:20) with reason input; confirm path uses narrow `ManualOverride`, re-eval, and `with_manual_override_record` + `confirmation_class_strings` (stable kebab). `PendingAction` (confirmation.rs:4) for UI actions stays separate/lower precedence. See runner.rs:82 (init), app/mod.rs:324-393 (gates + request/confirm_policy), key_handler.rs:205 (PolicyConfirm handling).

## Enforcement Posture Model (Phase 5)

### TuiEnforcementState

`TuiEnforcementState` in `app/enforcement.rs` is the TUI-local enforcement posture model. It wraps `EnforcementContext`, `LoadedScope`, and preflight state for TUI-specific posture management.

```rust
pub struct TuiEnforcementState {
    surface: ExecutionSurface,       // TuiManual or TuiManualStrict
    loaded_scope: LoadedScope,
    enforcement: EnforcementContext,
    manual_override: ManualOverride,
    last_preflight: Option<TuiPreflightResult>,
}
```

**Accessing from App:** `App.enforcement_state: EnforcementFacade` wraps `TuiEnforcementState` and provides focused evaluation/approval methods. The facade owns the enforcement context, loaded scope, preflight cache, and cached approval token.

**Key methods:**
- `toggle_posture()` — switches between TuiManual (ManualPermissive) and TuiManualStrict (ManualGuarded)
- `preflight(target)` — advisory evaluation of a target against current posture
- `mode_label()` — returns "Manual" or "Guarded" for status bar
- `scope_label()` — returns scope provenance and rule counts
- `status_string()` — returns the full status bar posture string
- `is_guarded()` — true when in Guarded (strict) mode
- `honors_manual_override()` — true when in Manual (permissive) mode

### Toggle Behavior

`Ctrl+G` toggles between Manual and Guarded TUI postures. `TuiEnforcementState::toggle_posture()` switches the `surface` field between `TuiManual` and `TuiManualStrict`.

**Critical:** `TuiManualStrict` does NOT honor manual overrides. `TuiManual` does. This mirrors CLI `--strict-scope` semantics.

### Preflight Evaluation

`TuiPreflightResult` is an advisory evaluation displayed in the status bar. It does not gate execution. `TuiPreflightOutcomeKind` indicates whether the target will be allowed, warned, confirmed, or denied under the current posture.

### Status Bar Display

The status bar renders posture information using `mode_label()`, `scope_label()`, and `status_string()` from `TuiEnforcementState`. The mode indicator shows "Manual" or "Guarded" with appropriate theme styling.

### CLI-Equivalent Preview

The confirmation overlay includes a CLI-equivalent flag preview showing what flags would reproduce the current posture on the command line.

### Tests

22 unit tests cover `TuiEnforcementState` including toggle behavior, preflight evaluation, mode labels, scope labels, and guard/honors-override queries. 8 unit tests cover `EnforcementFacade` including try_approve, cached approval, confirm_override, and state accessors. Run with:

```bash
cargo test --lib -p eggsec-tui tui::app::enforcement
cargo test --lib -p eggsec-tui tui::app::enforcement_facade
cargo test --lib -p eggsec-tui tui::app::action_spec
```

### Audit Integration (Phase 10)

TUI enforcement decisions emit normalized `EnforcementAuditEvent` records via `eggsec::audit`. Audit events are emitted in:
- `handle_enter()` for direct-launch tab pre-dispatch evaluation
- `evaluate_policy_and_dispatch()` for post-dispatch evaluation
- `confirm_policy_action()` when manual override is accepted (with `confirmed=true`)
- `TuiEnforcementState::preflight()` for advisory preflight evaluation

Only executable operations produce audit events. TUI uses `ExecutionSurface::TuiManual` or `TuiManualStrict` for audit records.

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

## Phase 8: TUI Architecture Tightening (2026-06-30)

### EnforcementFacade Extraction

`EnforcementFacade` (`app/enforcement_facade.rs`) extracts enforcement evaluation and approval logic from `App` into a focused struct:

- `try_approve(desc)` — evaluate + audit + approve/reject
- `evaluate_and_try_approve(desc)` — consume cached approval or re-evaluate
- `take_cached_approval(desc)` — consume matching cached token
- `confirm_override(descriptor, classes, reason)` — build ManualOverride + approve
- `audit_confirmed_override(...)` — emit audit event for confirmed override
- Delegation methods: `toggle_posture()`, `mode_label()`, `status_string()`, `preflight()`, `enforcement()`, `loaded_scope()`

App retains UI-level flows (`request_policy_confirmation`, `confirm_policy_action`, `cancel_policy_action`) because they touch overlay state.

### TUI Action/Tab Metadata Registry

`TuiActionSpec` and `TuiTabSpec` (`app/action_spec.rs`) provide metadata-backed descriptors pointing to canonical `OperationMetadata`. Pilot: recon, scan-ports, fuzz, db-pentest. Tests verify metadata resolution, feature string validity, risk consistency, and domain reference validity.

### Module Structure

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
│   ├── bookmarks.rs    # Bookmark helper functions
│   ├── confirmation.rs  # PendingAction enum
│   ├── enforcement.rs   # TuiEnforcementState, TuiPreflightResult
│   ├── enforcement_facade.rs # EnforcementFacade (Phase 8 extraction)
│   ├── action_spec.rs   # TuiActionSpec, TuiTabSpec (Phase 8 metadata registry)
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
│   ├── contrast.rs     # Theme contrast validation (min 4.5:1)
│   ├── manager.rs      # ThemeManager
│   ├── style.rs        # Theme style methods
│   ├── loader.rs       # Parses .toml themes; shared named_color() for 27 colors
│   └── legacy.rs       # Thread-local macro (tc!)
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

### Tab Macros (Boilerplate Reduction)

The `tabs/macros.rs` module provides macros to eliminate repetitive `TabInput`/`TabState` implementations:

| Macro | Purpose | Generates |
|-------|---------|-----------|
| `tab_state_boilerplate!` | Common `TabState` delegation to `TabCore` | `state()`, `progress()`, `set_error()` |
| `tab_input_boilerplate!` | Basic `TabInput` methods delegating to `TabCore` | `handle_copy`, `handle_word_*`, `handle_home/end`, `handle_top/bottom`, `page_up/down`, `stop`, `primary_target` (11 methods) |
| `tab_input_2area!` | Extends boilerplate for 2-area tabs (Inputs/Results) | All of boilerplate + `handle_char`, `handle_backspace`, `handle_paste`, `handle_focus_next/prev`, `handle_up/down`, `handle_left/right`, `is_input_focused`, `is_at_left/right_edge` |
| `tab_input_3area!` | Extends boilerplate for 3-area tabs (Inputs/Options/Results) | All of 2area with 3-area focus cycling |

**Usage pattern for new tabs:**

```rust
use crate::{tab_input_3area, tab_state_boilerplate, tc};

impl TabState for MyTab {
    tab_state_boilerplate!(MyTab, core: core);
    fn reset(&mut self) { /* custom reset logic */ }
}

impl TabInput for MyTab {
    tab_input_3area!(
        MyTab, core: core, focus: focus_area,
        Inputs: MyFocusArea::Inputs,
        Options: MyFocusArea::Options,
        Results: MyFocusArea::Results
    );
    // Override only the methods that differ from the macro defaults:
    fn handle_enter(&mut self) { /* custom enter logic */ }
    fn handle_escape(&mut self) { /* custom escape logic */ }
}
```

**When to use which macro:**

- Use `tab_state_boilerplate!` for all tabs with `TabCore` (always saves 8 lines)
- Use `tab_input_2area!` for tabs with only Inputs/Results (no Options area) and standard input handling (no validation in `handle_char`/`handle_backspace`/`handle_paste`)
- Use `tab_input_3area!` for tabs with Inputs/Options/Results and standard input handling
- Use `tab_input_boilerplate!` when tabs need custom `handle_char`/`handle_backspace`/`handle_paste` (e.g., validation), custom checkbox navigation in Options, or custom focus cycling with index reset

**Helper functions in `core.rs`:**

- `field_as<T>(core, index, default)` - Parse field at index as `T`, returning default on failure
- `field_str(core, index)` - Return field value at index as `&str`
- `start_scan(core)` - Set Running, clear results/error; returns `false` if target empty
- `render_results_area(...)` - Standard 4-branch results rendering (Running/Error/Results/Empty)
- `render_input_fields(f, chunks, inputs, insert_mode)` - Render InputGroup fields into layout chunks (replaces duplicated `for (i, field) in inputs.fields.iter()` loops)

### Feature-Gated Tab Helpers

- `App::set_current_tab_if_available(tab: Tab) -> bool` - Set tab only if available for current feature set
- Use this helper for mouse selection, `select_tab()`, and session restore

### Numeric Tab Shortcuts (1-based)

Digit keys `1`-`9` and `0` provide direct tab jumping using 1-based visible indices:

| Key | Visible index | Action |
|-----|--------------|--------|
| `1` | 0 | First tab |
| `2` | 1 | Second tab |
| ... | ... | ... |
| `9` | 8 | Ninth tab |
| `0` | 9 | Tenth tab |

Implementation in `key_handler.rs:284-298`: `'1'..='9'` maps to `digit - 1` via `Tab::from_visible_index()`, `'0'` maps to index 9. Digits beyond available tab count are no-ops. Tests in `key_handler.rs:814-884` lock all mappings.

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

`Theme.name` is the canonical stable ID for the theme, selector labels are derived separately for display, `Ctrl+T` cycles all registered themes alphabetically, and `ThemeManager.current` is private.

Theme loading runs in a background thread; `ThemeLoadState` keeps the receiver, join handle, deferred restore request, and `ThemeLoadReason` (Startup or ManualReload) together so startup stays non-blocking. Manual reload (`ManualReload`) shows a "Loading themes..." notification immediately; `Startup` does not.

**Contrast validation** (`theme/contrast.rs`): Loaded themes are validated for minimum contrast ratio (4.5:1) on text/background and selected_text/selected pairs. Low-contrast themes trigger a fallback to the base theme with a warning (non-fatal). The shared `named_color()` function in `loader.rs` maps all 27 named CSS colors for consistent parsing across `parse_hex_color()` and `luminance()`.

**Explicit theme passing / ThemeLoadOutcome**: The background theme loader returns a `ThemeLoadOutcome` that carries pre-adjustment contrast warnings captured during parsing. This allows `FallbackAdjusted` status to accurately reflect the contrast warnings that triggered the fallback, rather than re-validating after fallback (which would lose the original warnings). The `ThemeLoadOutcome` is consumed by `handle_theme_install_report()` to populate `SettingsTab.theme_contrast_cache` per theme.

**SettingsTab.applied_theme_id**: `SettingsTab` tracks `applied_theme_id` (the theme active when Settings was opened) separately from the selector's current selection. This lets the preview show what the selected theme will look like while preserving the ability to revert on Escape. `ThemeManager.current_id()` provides the accessor for the currently active theme.

**Live preview refresh**: The Settings theme preview refreshes as the selector moves — `update_settings_theme_selector()` resolves the newly selected theme's colors into `SettingsTab.resolved_theme_colors`, which the render path reads on each frame. Never read `tc!()` directly for preview colors.

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

**Settings theme preview**: The Settings theme preview uses `resolved_theme_colors` from `SettingsTab` (not `tc!()`) to render preview swatches. `tc!()` reads the thread-local *applied* theme, which may differ from the theme being previewed in the selector. The `fg` helper in `render.rs:341-343` falls back to `tc!(text)` only when `resolved_theme_colors` is `None`:
```rust
let c = self.resolved_theme_colors.as_ref();
let fg = |get: fn(&ThemeColors) -> ratatui::style::Color| {
    c.map(get).unwrap_or_else(|| tc!(text))
};
```
Always use this pattern for Settings preview rendering — never read `tc!()` directly for preview colors.

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

**Dropdown rendering**: `FormBuilder::collect_dropdowns(area, viewport_height)` returns `Vec<DropdownInfo>` for all open selectors. Each `DropdownInfo` contains the anchor area, items, selected index, and computed dropdown rect (position + height with viewport-aware clamping). Settings `render.rs` iterates these to render dropdown overlays on top of the form body. The `Selector` component's `render()` method draws the dropdown list when `is_open()` is true — never call `render()` on a selector without a render path or the dropdown will not appear.

## InputGroup Stale-Focus Guard

`InputGroup` provides `valid_focused_index()` and `valid_focused_index_ref()` methods that return `Option<usize>` instead of raw `self.focused`. Always use these instead of direct `self.focused` indexing to protect against stale focus indices after fields are removed or the group is cleared:

```rust
// WRONG - stale focus can panic
let field = &mut self.fields[self.focused.unwrap()];

// CORRECT - valid_focused_index guards against stale focus
if let Some(idx) = self.valid_focused_index() {
    let field = &mut self.fields[idx];
}
```

This is especially important in `reset()` methods and focus transitions where the field count may change.

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

The `Tab` enum in `tabs/mod.rs` has exactly **33 variants**. Do not reference "30 tabs" or "31 payload types" — these are stale counts from earlier versions.

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

## TUI Architecture and Usability Pass (2026-06-11)

Completed the 10-phase plan in `docs/plans/tui-architecture-usability-pass.md` (using subagents for context isolation). Each phase compiles and passes `cargo test -p eggsec-tui` independently. Final TUI crate: 301 tests green. Workspace/all-features run before handoff (pre-existing non-TUI protobuf/codegen errors in eggsec lib).

Key new modules / surfaces (per phase):
- `app/action.rs`: `UiAction`, `CommandPaletteInput`, `QuickSwitchInput`. Decode in KeyHandler; `App::apply_action` is the mutation point for global UI actions.
- `app/overlay.rs`: `OverlayController` with single `decode(...)` routing fn that asks `topmost_overlay()` and owns all per-overlay input rules (PolicyConfirm/ConfirmPopup/CommandPalette/QuickSwitch/Search/Http/Help). Emits UiActions only; no mutations.
- `tabs/spec.rs`: `TabSpec` / `TabCategory` / `TabRiskGroup` (later extended with operation/direct_launch). Single source for title/stable/cli/desc/category/risk/feature/breadcrumb. `Tab` methods delegate. `visible_tab_specs()` mirrors `Tab::all()` construction.
- Delegated descriptors: `TabInput::primary_target` (default + impls), `Tab::operation_name`/`is_direct_launch`, `risk_from_group`, thin delegation in `build_current_operation_descriptor`/`current_tab_target`/`is_direct_launch_tab`. Enforcement stays central.
- Visibility (shell.rs + preflight helpers): status bar now shows enforcement mode, scope provenance (LoadedScope source), risk badge (from spec), per-target preflight (target/scope-match/risk/op/"will: run|warn|confirm|deny" via live `EnforcementContext::evaluate`). Advisory only.
- Global task strip: `TaskState.started_at`; status + help show active task tab/state/elapsed/hints even after nav away; pause/resume visible; quit-block not surprising.
- Palette complete (command.rs + help_config.rs): all keybound actions + required list (run-current, stop/pause/resume/jump-active, quick-switch, help-current, search/global, theme, cycle/export, copy-cli, settings, reload-scope stub, save contextual, clear/delete contextual). Disabled-with-reason for no-task / wrong-tab cases.
- Copy CLI (app/mod.rs + command.rs + utils): `copy_cli_equivalent` (cli_command + primary_target + safe options + --format + explicit --scope only); shell_escape; palette action; graceful clipboard fail; no broad bypass flags; tests for recon/scan-ports/intrusive/non-exec.
- Small-terminal (shell.rs + mod.rs + popups.rs + tests): breadcrumb tab bar on narrow, too-small (<~40x10) clear fallback (input/quit still work), popups clamped, policy confirm preserved, low-pri status dropped first; 60x20 usable; layout tests added.
- Semantic tokens (theme/palette.rs + builtin.rs + loader.rs + style.rs): 10 roles (safe/danger/muted/active_task/paused_task/scope_match/miss/policy_required/denied) + helpers (`style_for_risk`, `style_for_policy_outcome`, `style_for_task_state` etc.); adopted in preflight/status/task/policy paths; all themes + loader + cyber-red fallback + non-blocking load unchanged.

Overlay precedence (topmost_overlay + controller) is now PolicyConfirm > ConfirmPopup > CommandPalette > QuickSwitch > Search > HttpOptions > Help. Non-topmost never receive input; overlay-local keys never leak.

All acceptance criteria from the plan are met (decode/apply split testable; one overlay routing fn; single metadata truth + feature gating + stable_id roundtrips; descriptors delegated + risk from spec; manual visibility + preflight advisory; task strip visible after nav; palette action-complete with context; CLI copy with safe escape + no bypasses; small-terminal degraded + "too small" fallback + policy readable; semantic helpers used for scope/risk/task/policy).

Validation (run after substantial phases and at end):
```
cargo fmt --all
cargo check -p eggsec-tui
cargo test -p eggsec-tui
cargo check --workspace --all-features
cargo test --workspace --all-features
```

Update any future TUI changes to preserve the decode/apply split, delegate through TabSpec where metadata/risk/operation are needed, keep enforcement central, and surface manual posture/preflight/task state via the status paths.

## Session Fixes (2026-06-17)

- **Dead theme macro removed**: `theme!()` macro in `legacy.rs` was never used (only `tc!()`); removed
- **Dead style calls removed**: `style_for_risk()` and `scope_match()`/`scope_miss()` calls in `ui/shell.rs` assigned to `let _` (results discarded); removed
- **Hardcoded colors fixed**: `wireless.rs:157-241` had 15 hardcoded `Color::Red/Gray/Yellow/DarkGray/Cyan`; replaced with `tc!()` theme tokens (danger, text_dim, warning, muted, info). `intercept.rs:917` had `Color::Magenta`; replaced with `tc!(accent)`. `intercept.rs:1854` had `Color::Red`; replaced with `tc!(danger)`
- **handle_enter() Results guard**: `graphql.rs:471` and `oauth.rs:520` had empty `Results => {}` arms that fell through to `self.start()`; added `return;`. `db_pentest.rs:307` started unconditionally; added `is_running()` + `Results` focus guards
- **page_up/page_down guard**: `cluster.rs:732-738` missing `is_running()` guard; added
- **Session cleanup perf**: `session.rs:248` `sessions.remove(0)` O(n) changed to `swap_remove(0)` O(1)
- **Dead code cleanup**: Empty `if is_advanced {}` block in `app/mod.rs:485-487` removed; `let _ = d` PolicyDecision discard at `app/mod.rs:973` removed; stale `#[allow(unused_variables)]` in `workers/recon.rs:5` removed; redundant `#[cfg(feature)]`/`#[cfg(not(feature))]` pairs in `app/export.rs` collapsed

## Session Fixes (2026-06-17) - Deep Audit

- **UTF-8 panic fix**: `ui/shell.rs:201,341` used byte-offset slicing (`&status_text[..42]`, `&target[..25]`) which panics on multi-byte characters. Changed to character-aware truncation via `.chars().take(N).collect::<String>()`
- **handle_enter() scan-from-input**: `graphql.rs:459`, `oauth.rs:508`, `cluster.rs:573` all started scans when Enter was pressed in input fields (Inputs arm blurred but didn't return). Added `return;` after blur. `wireless.rs:690` added Results focus area early return guard
- **Theme loader luminance()**: Named colors like "black" returned 0.5 (neutral), incorrectly classified as Light mode. Extended `luminance()` to handle named colors
- **Theme loader has_any_color**: Check omitted `buttons` section — added `|| halloy.buttons.is_some()`
- **db_pentest handle_left/right**: Missing `is_running()` guard — added
- **proxy page_up/page_down**: Ignored `page_size` parameter, hardcoded 20. Changed to use parameter
- **graphql/oauth page_up/page_down**: Missing overrides — PageUp/PageDown were non-functional. Added delegates to `results_view`
- **auth handle_escape**: Transitions to Results instead of Target. Fixed to Target
- **runner.rs config error**: `.ok()` silently swallowed parse errors. Changed to `match` with `tracing::warn!`
- **workers/auth.rs**: Dead `if let Some(ref cred_file)` block removed
- **help_config.rs**: Stale Ctrl+T description "Cycle built-in theme" → "Cycle theme"
- **Dead code cleanup**: Removed stale `#[allow(dead_code)]` on `InputField.label` (is used), replaced blanket `#[allow(dead_code)]` on Popup impl with per-method annotations, added `#[allow(dead_code)]` on unused PopupKind variants, added doc comment on HelpContext placeholder

## Session Fixes (2026-06-18) - TUI Audit

- **graphql.rs handle_enter() fallthrough**: Options arm toggled checkbox then fell through to `self.start()`, silently starting a scan. Added `return;` after toggle
- **oauth.rs handle_enter() fallthrough**: Same pattern — Options arm fell through to `self.start()`. Added `return;` after toggle
- **intercept.rs truncate_str() UTF-8 panic**: Used byte-offset slicing `&s[..max_len]` which panics on multi-byte characters. Changed to character-aware truncation via `.chars().take()`
- **settings/main.rs Session max_focus_index**: Returned `1` but `session_inputs` has only 1 field (index 0). Changed to `0`
- **theme/loader.rs luminance() named colors**: `lightblue`/`lightred`/`darkgreen` etc. shared luminance values with base colors, misclassifying Light/Dark mode. Fixed to use distinct values (light* → 0.7-0.8, dark* → 0.2-0.4)
- **popup.rs scroll cast truncation**: `scroll_offset as u16` silently truncated values > 65535. Added `.min(u16::MAX as usize)` clamp
- **popup.rs button width u16 overflow**: Button width sum could overflow u16. Changed to `saturating_add`
- **session.rs swap_remove(0) order**: `cleanup_old_sessions` used `swap_remove(0)` which broke sorted order, deleting wrong sessions. Changed to `remove(0)`
- **db_pentest.rs allow_db_pentest hardcoded**: Worker unconditionally passed `allow_db_pentest: true`. Changed to pass `dry_run` value to respect lib safety gate
- **selector.rs height overflow**: Dropdown height calculation could overflow on extreme item counts. Added `.min(u16::MAX as usize - 2)` clamp
- **help_scroll_offset usize::MAX**: `HelpScrollBottom` set offset to `usize::MAX` which could cause unexpected behavior. Changed to `u16::MAX as usize`
- **ThemeInstallReport Clone data loss**: Lossy `Clone` impl silently dropped `loaded_themes` Vec. Removed impl (never cloned; consumed via channels)

## Session Fixes (2026-06-18) - TUI Audit

- **db_pentest worker missing timeout**: `run_db_pentest_cli()` called without `tokio::time::timeout` — hung database connections blocked TUI permanently. Wrapped in 60s timeout with three-arm match pattern
- **session.rs load_quick() quarantine**: Corrupt `quick_save.json` propagated error directly — hard failure, no recovery, session lost. Added quarantine logic matching `load_latest_session` pattern (rename to `.json.bad`, log warning, return `Ok(None)`)
- **intercept.rs page_up/page_down page_size**: Methods accepted `_page_size` parameter but hardcoded `20`. Changed to use the parameter
- **intercept.rs edit_modal reset**: `reset()` didn't clear `edit_modal` — stale modal state persisted after tab reset. Added `close_edit_modal()` call
- **packet.rs page_up/page_down**: Missing from `impl TabInput` — PageUp/PageDown keys were no-ops. Added both methods delegating to `results_view`
- **5 tabs missing handle_copy()**: `load.rs`, `report.rs`, `auth.rs`, `c2.rs`, `db_pentest.rs` silently ignored Ctrl+C. Added `handle_copy()` implementations
- **runner.rs redundant .map()**: Removed no-op `.map(|ls| ls)` identity transform
- **command.rs silent let _ =**: `set_current_tab_if_available` failure discarded. Changed to log on failure
- **workers/security.rs silent HTTP error**: Compliance preflight request error silently discarded. Added `tracing::debug!`
- **session.rs metadata error swallowing**: Double `.ok()` in tmp cleanup silently swallowed metadata errors. Changed to explicit `match` with logging

## Session Fixes (2026-06-18) - Critical Bug Audit

- **graphql.rs/oauth.rs handle_enter unreachable start()**: Both tabs had a `match` over all focus areas that returned/diverged on every arm, then called `self.start()` (unreachable). Refactored to follow the `fuzz.rs` pattern: `if Results return; if is_running stop+return; if Inputs focused blur+return; if Options toggle+return; else (Inputs with `is_focused() == false`) start()`. Users can now actually start GraphQL and OAuth scans from the TUI.
- **Popup::content() overflow**: `content.len() + 5` could overflow in release builds on huge content. Changed to `content.len().saturating_add(5)` to match the project-wide pattern.
- **ui/shell.rs identical if/else**: scope label had both branches returning `"out"`. Changed the non-compact branch to `"out-of-scope"` to match the `"in-scope"` pattern.
- **Theme polish**: `toggle_theme` notification now uses `display_theme_name()` (e.g. "Catppuccin Mocha" instead of "catppuccin-mocha"); `Selector::set_items_with_extra` now deduplicates by value; `theme/loader.rs` `luminance()` logs a `tracing::warn!` for unknown named colors; `Settings::set_available_themes` early-returns on empty list and uses clearer `[! id] (not installed)` placeholder prefix; settings Theme hint now describes the full theme story; `help_config` Ctrl+T and `theme` palette entries now say "next theme (alphabetical)".
- **Settings save hint footer**: persistent `[s] Save [Esc] Discard [Tab] Next field [↑↓] Section` at the bottom of the Settings tab (the `s` key was previously undiscoverable).
- **Terminal too small message**: clearer wording ("Resize your window or scroll horizontally").
- **Dead code warnings**: 16 TUI dead-code warnings reduced to 0 by adding `#[allow(dead_code)]` annotations with explanatory comments on forward-compat fields (HalloyBuffer/HalloyButtonStyle, PopupKind::Info/Warning/Error, TabSpec::category, theme constants, `decode_key_event`, etc.).
- **9 new unit tests**: 3 for `graphql::handle_enter`, 3 for `oauth::handle_enter`, 4 for `popup::content` (including the overflow guard). All 311 TUI tests pass.

## Phase 1-4 UI Patterns

### Action Hints Pattern

Status bar hints are context-aware via `get_action_hints()` in `app/action_hints.rs`. Priority order:
1. Running task hints (stop/pause/resume) — detected via `app.has_active_task()` (checks all task state fields, not just `task_state.handle`)
2. Overlay-specific hints (policy confirm, command palette, search, help, etc.)
3. Insert-mode hints (Esc/Tab/Enter)
4. Tab-specific normal-mode hints (varies by tab)

**Settings section-aware hints**: The Settings tab adapts hints based on the current section and theme selector state:
- Theme section (selector closed): `r:reload Enter:themes Tab:next`
- Theme section (selector open): `Enter:select ↑↓:theme Esc:cancel`
- Other sections: `s:save r:reset Tab:next`

**Help overlay keybindings**: The help overlay uses `j/k` for scrolling up/down and `g/G` for jumping to top/bottom (not `h/l` for pane navigation). `b`/`B` move word-backward in input fields. See `help_config.rs` for the full mapping.

New code should use this system instead of scattering hint strings:
```rust
use crate::app::action_hints::{get_action_hints, format_hints};

// In draw_status_bar():
let hints = get_action_hints(app);
let hint_text = format_hints(&hints);
```

Tab-specific hint functions (`settings_hints`, `history_hints`, `dashboard_hints`) return static `Vec<ActionHint>`. The `settings_hints` function checks `current_section` and `theme_selector.is_open()` to return context-appropriate hints. The `default_normal_hints` function adapts based on whether the current tab has a target set (shows "run" vs "focus"). See `app/action_hints.rs:19` for the dispatch function.

### Theme Metadata Pattern

`ThemeManager` provides structured metadata via `ThemeInfo`, `ThemeSource`, and `ThemeLoadStatus`:

```rust
// Query metadata for a theme
if let Some(info) = theme_manager.get_info("dark") {
    println!("Source: {:?}", info.source);     // BuiltIn | Packaged | Custom
    println!("Status: {:?}", info.status);     // Loaded | FallbackAdjusted | Invalid(..) | Missing
    println!("Mode: {:?}", info.mode);         // Dark | Light
}

// List all theme metadata sorted by display name
let all_info = theme_manager.get_all_info();

// Counts
theme_manager.theme_count();     // total registered
theme_manager.loaded_count();    // Loaded status only
theme_manager.invalid_count();   // Invalid status only
```

Theme names are canonicalized via `canonical_theme_id()` before lookup. The `display_theme_name()` function title-cases IDs for display (e.g., "catppuccin-mocha" → "Catppuccin Mocha").

**Source attribution**: `load_themes_from_dir()` in `theme/install.rs` accepts `packaged_ids: &FxHashSet<String>` (the set of canonical IDs from the archive) and determines `ThemeSource::Packaged` vs `ThemeSource::Custom` based on whether the file stem is in the packaged set. This correctly handles re-launches where packaged themes are already installed.

**Invalid theme tracking**: `ThemeManager::register_theme_invalid(id, source, reason)` inserts metadata with `ThemeLoadStatus::Invalid(reason)` for themes that fail to load, so Settings shows them with `Invalid` status instead of silently omitting them.

**Contrast validation**: `ThemeManager::validate_contrast(id)` returns per-theme `Vec<String>` of contrast warnings. Per-theme warnings are stored in `SettingsTab.theme_contrast_cache: FxHashMap<String, Vec<String>>` (keyed by canonical theme ID). `update_theme_metadata()` computes and populates this cache; the Settings Theme details pane reads warnings directly from the cache for the selected theme.

### Theme Reload Pattern

Normal-mode `r` in the Settings Theme section (with the theme selector closed) emits `UiAction::ReloadThemes` directly, bypassing the `PendingAction` confirmation flow. `apply_action` in `app/mod.rs` calls `spawn_theme_loader_with_reason(ThemeLoadReason::ManualReload)`, which shows a "Loading themes..." notification immediately and spawns the background loader. The insert-mode `r` path via `pending_theme_reload` on `SettingsTab` still works for backward compatibility:

```rust
// app/key_handler.rs - normal-mode 'r' in Settings > Theme
(KeyModifiers::NONE, KeyCode::Char('r')) => {
    if !app.has_active_task() {
        if app.current_tab == Tab::Settings
            && app.tabs.settings.current_section == SettingsSection::Theme
            && !app.tabs.settings.theme_selector.is_open()
        {
            vec![UiAction::ReloadThemes]
        } else {
            vec![UiAction::ResetCurrent]
        }
    } else {
        vec![]
    }
}

// app/mod.rs - apply_action for ReloadThemes
UiAction::ReloadThemes => {
    if !self.has_active_task() && self.current_tab == Tab::Settings {
        self.spawn_theme_loader_with_reason(
            crate::app::state::ThemeLoadReason::ManualReload,
        );
    }
}
```

`ThemeLoadReason` (Startup vs ManualReload) is tracked in `ThemeLoadState` and controls whether a notification is shown on dispatch (ManualReload: yes, Startup: no). Manual reload shows "Loading themes..." immediately, then success/no-op feedback when the loader completes.

### No-Result State Pattern

Empty states are rendered via `empty_state_paragraph()` from `components/empty_state.rs`:

```rust
use crate::components::empty_state_paragraph;

// In tab render:
let placeholder = empty_state_paragraph(
    "Results",
    "Results will appear here after running"
);
f.render_widget(placeholder, results_area);
```

The function creates a bordered `Paragraph` with `tc!(text_dim)` styling. All tabs use this pattern for their Results areas when no data is available. Overlays also show empty states:
- Command palette: "No matching commands" when query filters to nothing
- Quick switch: "No matching tabs"
- Search: "No results for 'query'" after a search completes with no matches

### TabSpec Capability Flags

`TabSpec` (in `tabs/spec.rs`) has capability flags that control UI behavior:

```rust
pub struct TabSpec {
    // ... metadata fields ...
    pub supports_run: bool,      // Can start a task (has inputs → Enter action)
    pub supports_export: bool,   // Shows in export menu
    pub supports_help: bool,     // Has help content
    pub has_settings: bool,      // Has configurable settings
}
```

Helper methods:
- `spec.can_start_task()` → `supports_run && !direct_launch` (tabs with direct_launch use pre-dispatch policy eval in handle_enter, not the standard run path)
- `spec.shows_in_export()` → delegates to `supports_export`

Assessment-category tabs always have `supports_run: true`. Use these flags in palette and UI code to conditionally enable/disable actions rather than hardcoding tab checks.

### Regression Test Pattern

Tab `handle_enter()` regression tests live in `tabs/handle_enter_regression.rs`. Each tab has tests covering all focus areas to prevent fallthrough bugs:

```rust
#[test]
fn graphql_enter_options_toggles_checkbox() {
    let mut tab = GraphQlTab::new();
    tab.focus_area = GraphQlFocusArea::Options;
    let before = tab.introspection_checkbox.checked;
    tab.handle_enter();
    assert_eq!(tab.introspection_checkbox.checked, !before);
    assert!(!tab.is_running());  // Must NOT start scan
}

#[test]
fn graphql_enter_results_no_op() {
    let mut tab = GraphQlTab::new();
    tab.focus_area = GraphQlFocusArea::Results;
    tab.handle_enter();
    assert!(!tab.is_running());
}

#[test]
fn graphql_enter_inputs_unfocused_starts_with_target() {
    let mut tab = GraphQlTab::new();
    tab.focus_area = GraphQlFocusArea::Inputs;
    tab.inputs.blur();
    tab.inputs.fields[0].value = "https://example.com/graphql".to_string();
    tab.handle_enter();
    assert!(tab.is_running());
}
```

The pattern verifies: (1) Options toggles without starting, (2) Results is a no-op, (3) Inputs focused blurs without starting, (4) Inputs unfocused with target starts the scan. When adding a new tab or modifying `handle_enter()`, add corresponding regression tests here.

### Popup Scroll Pattern

`Popup` scroll helpers (`components/popup.rs:100-124`) guard against empty content, matching `ScrollableText`'s pattern:

```rust
pub fn scroll_down(&mut self, amount: usize) {
    if self.content.is_empty() {
        self.scroll_offset = 0;
    } else {
        let max_scroll = self.content.len() - 1;
        self.scroll_offset = self.scroll_offset.saturating_add(amount).min(max_scroll);
    }
}

pub fn scroll_to_bottom(&mut self) {
    if self.content.is_empty() {
        self.scroll_offset = 0;
    } else {
        self.scroll_offset = self.content.len() - 1;
    }
}
```

`scroll_up` uses `saturating_sub` (safe on zero). Both `scroll_down` and `scroll_to_bottom` check `is_empty()` before computing `len() - 1` to avoid underflow. This is the canonical pattern for any scrollable component — always guard the `len() - 1` expression against empty collections.

## Session Fixes (2026-06-17) - TUI Bugs Plan

- **gg normal-mode sequence fixed**: Added `UiAction::BeginGgSequence` to `action.rs`; `decode_normal_mode_input` returns it for first `g`; `apply_action` sets `pending_key = Some(KeyCode::Char('g'))`. Second `g` in `handle_key_event` now correctly triggers `MoveTop`. 3 new regression tests.
- **Ctrl-Space autocomplete fixed**: Added `UiAction::Autocomplete` to `action.rs`; `decode_insert_mode_input` returns it for Ctrl-Space; `apply_action` calls `handle_autocomplete()` and sets `needs_redraw` on success. 2 new tests.
- **Settings theme selector notification**: `handle_enter()` now shows `Notification::new("Theme: ...", Info)` on successful theme change and `Notification::new("Theme not available: ...", Warning)` on failure, matching Ctrl+T feedback.
- **Settings footer wording**: Changed `[Esc] Discard` to `[Esc] Back` to match actual behavior (Escape navigates back, does not discard unsaved edits).
- **Theme loader warning dedup**: `luminance()` now uses `LazyLock<Mutex<FxHashSet>>` to warn once per unique unknown color name per process lifetime instead of per occurrence.
- **Documentation drift fixed**: `AGENTS.override.md` module path corrected from `crates/eggsec/src/tui/` to `crates/eggsec-tui/src/`; tab count corrected from "30" to "33".

## Settings & Selector Anti-patterns

### Do not open an embedded selector without a render path

Every `Selector` that can be opened must have a corresponding render path in its parent tab's `render()` method. If you add a new `Selector` field to a tab but forget to render it, the dropdown will open (state changes) but nothing will appear on screen. Verify that `render()` calls `selector.render()` (or `FormBuilder::collect_dropdowns()` for settings-style selectors) when `selector.is_open()`.

### Do not let normal-mode shortcuts leak through embedded modal controls

Embedded selectors are not overlays — `topmost_overlay()` returns `None` when only a selector is open. The `has_any_tab_selector_open()` guard in `decode_normal_mode_input` returns `UiAction::Noop` for all normal-mode keys when any selector is open, and passes `j`/`k` through for Vim-style navigation. If you add a new embedded selector to a tab, override `TabState::has_selector_open()` to return `true` when it is open.

### Do not add Settings fields without load/validate/apply tests

Every Settings input field must be covered by three test categories:
1. **Load test**: Verifies the field value is read from the config struct into the input field on tab open (e.g., `scan_rate_limit_loads_from_config`).
2. **Validate test**: Verifies the `validate()` method rejects invalid values (empty, zero, non-numeric) and accepts valid ones (e.g., `validate_zero_rate_limit_fails`, `validate_valid_rate_limit_passes`).
3. **Apply test**: Verifies the field value is written back to the config struct on save (e.g., `save_config_writes_rate_limit_per_second`).

Without all three, silent regressions can occur where the UI shows a value that doesn't match the actual config, or invalid values bypass validation.

### Theme preview uses resolved_theme_colors, not thread-local

The Settings theme preview renders preview swatches using `SettingsTab.resolved_theme_colors` (resolved from `ThemeManager`), not `tc!()`. The `tc!()` macro reads the thread-local *applied* theme, which may differ from the theme being previewed in the selector. The `fg` helper in `render.rs` falls back to `tc!(text)` only when `resolved_theme_colors` is `None`:
```rust
let c = self.resolved_theme_colors.as_ref();
let fg = |get: fn(&ThemeColors) -> ratatui::style::Color| {
    c.map(get).unwrap_or_else(|| tc!(text))
};
```
Never read `tc!()` directly for Settings theme preview colors.
