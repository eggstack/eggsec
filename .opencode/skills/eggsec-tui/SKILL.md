---
name: eggsec-tui
description: "Terminal UI module workflows - use when working with TUI tabs, event loop, key handling, overlays, theming, notifications, tab rendering, or visual regression testing."
---

# Eggsec TUI Skill

TUI module workflows and patterns for the terminal UI.

## Module Structure

```
crates/eggsec-tui/src/
├── app/          # App state, event loop, command handling
│   ├── mod.rs           # App struct, notifications, helpers
│   ├── action.rs        # Action dispatch
│   ├── action_hints.rs  # Context-aware status bar hints
│   ├── apply.rs         # State application helpers
│   ├── bookmarks.rs     # Bookmark management
│   ├── command.rs       # Command palette commands
│   ├── confirmation.rs  # PendingAction enum
│   ├── dispatch.rs      # Command dispatch
│   ├── error.rs         # Error handling
│   ├── export.rs        # Export functionality
│   ├── help_config.rs   # Static help content
│   ├── input.rs         # Input handling
│   ├── key_handler.rs   # Key handling methods
│   ├── navigation.rs    # Tab navigation, scrolling
│   ├── notifications.rs # Notification and NotificationSeverity types
│   ├── operation.rs     # Operation metadata integration
│   ├── options.rs       # Options management
│   ├── overlay.rs       # Overlay management
│   ├── runner.rs        # Event loop, input handling
│   ├── state.rs         # OverlayState, SearchState, QuickSwitchState, TaskState, ThemeLoadState
│   ├── state_update.rs  # Background task handling, result dispatch
│   ├── tab_store.rs     # TabStore - owns all 33 tab instances
│   ├── task_management.rs # Task lifecycle management
│   ├── task_runtime.rs  # Task runtime helpers
│   ├── task_dispatcher.rs # TuiTaskDispatcher (TaskDispatcher impl)
│   ├── runtime_adapter/ # Phase 4: runtime event reducer
│   │   └── mod.rs       # TuiRuntimeAdapter, TuiAction, reduce/apply pattern
│   └── theme_runtime.rs # Theme loader lifecycle helpers
├── tabs/         # Individual tab implementations (33 tabs)
│   ├── mod.rs          # Tab enum, TabState/TabInput/TabRender traits
│   ├── spec.rs         # TabSpec registry (title, stable_id, category, risk, capabilities)
│   └── ...
├── components/   # Reusable UI components
│   ├── input/           # InputField with focus colors (mod.rs, input_field.rs, input_group.rs, form_builder.rs)
│   ├── selector.rs      # Selector dropdown
│   ├── popup.rs         # Popup overlays
│   └── empty_state.rs   # empty_state_paragraph() for consistent empty states
├── theme/        # Theme system (50+ packaged themes via LZMA)
│   ├── palette.rs      # ThemeMode, Theme, ThemeColors
│   ├── builtin.rs      # dark_theme(), light_theme()
│   ├── manager.rs      # ThemeManager, ThemeInfo
│   ├── style.rs        # Theme style methods
│   └── legacy.rs       # Thread-local macro (tc!)
├── ui/           # Rendering layer
│   ├── mod.rs          # draw(), LAYOUT_MARGIN, TAB_BAR_HEIGHT
│   ├── shell.rs        # draw_tabs, draw_breadcrumb, draw_content, draw_status_bar
│   ├── popups.rs       # Overlay rendering
│   └── tests.rs        # UI rendering tests
├── search.rs     # Global search
└── help.rs       # HelpManager
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

50+ Halloy-format themes packaged via LZMA. `cyber-red` fallback always available. `Theme::default()` returns `cyber-red`.

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

`Ctrl+T` cycles all themes alphabetically. After modifying `themes/*.toml`, run `python3 scripts/package_themes.py` to regenerate `crates/eggsec-tui/src/theme/packaged.rs`.

### Notifications
```rust
app.notification = Some(Notification::new("Exported".to_string(), NotificationSeverity::Success));
```

### Dynamic Layouts
```rust
let config_height = if area.height <= 30 {
    ((area.height as f32 * 0.8) as u16).max(10).min(27)
} else { 27 };
```

## Action Hints

Priority order for hint resolution:
1. **Running task** — `C:stop Z:pause`
2. **Overlay-specific** — PolicyConfirm, ConfirmPopup, CommandPalette, QuickSwitch, Search, Help, HttpOptions
3. **Insert mode** — `Esc:normal Tab:next Enter:confirm`
4. **Tab-specific normal mode** — Settings, History, Dashboard
5. **Default normal** — `Enter:run n/p:tabs /:search`

## TabSpec Capabilities

`TabSpec` in `tabs/spec.rs` is the single source of truth for tab metadata. Key fields: `tab`, `stable_id`, `title`, `category`, `risk_group`, `feature`, `direct_launch`, `supports_run`, `supports_export`.

`direct_launch` tabs start work inside their own `handle_enter`. `handle_enter()` now evaluates policy BEFORE calling the dispatcher, so Deny/RequireConfirmation blocks before any side effect starts. The old post-dispatch retroactive policy gate has been removed.

## Enforcement Posture Model

`EnforcementFacade` in `app/enforcement_facade.rs` wraps `TuiEnforcementState` and provides focused enforcement evaluation and approval methods.

- **Manual** (default): `TuiManual` / `ManualPermissive`. Warnings for scope ambiguity; `RequireConfirmation` with confirm/override for discretion cases. Manual overrides honored.
- **Guarded**: `TuiManualStrict` / `ManualGuarded`. Hard enforcement, no discretion, no manual overrides.

**Ctrl+G** toggles between Manual and Guarded. `toggle_posture()` switches the surface field.

**Preflight**: Advisory evaluation of a target via `TuiEnforcementState::preflight()`. Uses the shared `preflight_operation()` helper from `config::policy_decision` — the same function called by CLI, REST, MCP, and agent surfaces. `TuiPreflightResult::from_outcome()` now accepts `&ExecutionPolicy` parameter, computing confirmation classes using the active policy (not `Default::default()`). Does not gate execution.

**Status bar**: Shows mode label ("Manual"/"Guarded"), scope provenance, rule counts, and preflight outcome.

**TUI Action/Tab Metadata Registry**: `TuiActionSpec` and `TuiTabSpec` in `app/action_spec.rs` provide metadata-backed descriptors that point to canonical `OperationMetadata` entries. Pilot covers recon, scan-ports, fuzz, and db-pentest. Tests verify metadata resolution, feature string validity, and risk consistency.

**Tests:**
```bash
cargo test --lib -p eggsec-tui tui::app::enforcement
cargo test --lib -p eggsec-tui tui::app::action_spec
cargo test --lib -p eggsec-tui tui::app::enforcement_facade
```

## Overlay Precedence
```rust
pub enum OverlayType {
    PolicyConfirm,  // Highest — EnforcementContext RequireConfirmation
    ConfirmPopup,   // PendingAction for UI actions
    CommandPalette,
    QuickSwitch,
    Search,
    HttpOptions,
    Help,           // Lowest
}
```

## Confirmation System

Two separate flows:
1. **PendingAction** — UI actions (reset/save/delete). Renders via `ConfirmPopup`.
2. **PendingPolicyConfirmation** — Policy `RequireConfirmation` from `EnforcementContext::evaluate()`. Highest precedence. Builds narrow `ManualOverride` on confirm. `confirm_policy_action()` now sets `mo.allow_out_of_scope = true` for `OutOfScope` and `TargetExpansion` classes, mirroring CLI `--allow-out-of-scope` behavior. `assume_yes` remains `false` (TUI confirm is narrow, not broad).

## TabError System
```rust
pub enum TabError {
    Network(String), Auth(String), Config(String),
    Resource(String), Target(String), Internal(String), Unknown(String),
}
```

`TabError::is_recoverable()` checks for Network/Auth/Resource errors.

## Settings Tab

Saving merges exposed fields into loaded config (non-exposed sections preserved). Fields: Timeout, Max Retries, Retry Delay, Max Redirects, Concurrency, Rate Limit, Port Timeout.

## Key Defensive Patterns

### Bounds Check
```rust
if let Some(chunk) = chunks.get(i) { /* use chunk */ }
```

### is_at_left_edge Checkbox Guard
```rust
fn is_at_left_edge(&self) -> bool {
    self.checkbox_array.is_empty() || self.focused_checkbox_index == 0
}
```

### Division by Zero Prevention
```rust
fn progress(&self) -> f64 {
    if self.stages.is_empty() { return 0.0; }
    // ...
}
```

### Worker Send Error Handling
```rust
if let Err(e) = result_tx.send(result).await {
    tracing::warn!("Failed to send results: {}", e);
}
```

## Common Tasks

### Adding a New Tab
1. Create tab module in `crates/eggsec-tui/src/tabs/`
2. Implement `TabState`, `TabInput`, `TabRender` traits
3. Add tab to `Tab` enum in `tabs/mod.rs`
4. Add to `TabStore`, `draw_content()`, `dispatcher_mut()`

## Testing

```bash
cargo test --lib -p eggsec-tui tui::
```

### Visual Regression Tests
```rust
let backend = TestBackend::new(80, 24);
let mut terminal = Terminal::new(backend).unwrap();
terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
let buf = terminal.backend().buffer();
```

Test at: 80x24 (standard), 120x40 (wide), 60x20 (compact), 40x12 (narrow), 30x10 (degradation).

## Resources
- `crates/eggsec-tui/src/AGENTS.override.md` - Detailed TUI patterns
- `architecture/tui.md` - TUI architecture, event loop, overlays, session handling
- `architecture/config.md` - Config loading and TUI settings save semantics
