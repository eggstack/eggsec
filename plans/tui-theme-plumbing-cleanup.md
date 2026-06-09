# Eggsec TUI Theme Plumbing Cleanup Plan

## Purpose

This plan follows the first TUI simplification pass. The prior pass successfully reduced `App` size, introduced grouped UI state, moved tab ownership into `TabStore`, split `theme.rs` into a module tree, and split the old monolithic `ui.rs` into shell and popup modules.

This pass should finish the architectural preparation needed before implementing Halloy-style themes. It should not add Halloy parsing, external theme discovery, or user-facing theme file configuration yet. The goal is to remove most remaining implicit theme access, make the theme type suitable for non-static themes, and leave a clean surface for the future loader/import pass.

## Non-goals

Do not implement Halloy theme parsing.

Do not add runtime theme directory scanning.

Do not add a new user-facing theme config format.

Do not redesign the Eggsec TUI visually.

Do not rewrite all tab rendering.

Do not remove the legacy `tc!`/`theme!` macros until all current usages are gone or trivially isolated.

Do not change CLI behavior or feature flags except for comments or strictly mechanical cleanup.

## Current state to assume

`crates/eggsec-tui/src/app/mod.rs` now has a thinner `App` with `tabs: TabStore`, `overlay: OverlayState`, `search: SearchState`, `quick_switch: QuickSwitchState`, and `task_state: TaskState`.

`crates/eggsec-tui/src/app/state.rs` contains focused state structs with default tests.

`crates/eggsec-tui/src/app/tab_store.rs` owns the concrete tab instances and preserves feature-gated tabs.

`crates/eggsec-tui/src/theme/` now contains `builtin.rs`, `legacy.rs`, `manager.rs`, `palette.rs`, and `style.rs`.

`crates/eggsec-tui/src/ui/` now contains `mod.rs`, `shell.rs`, `popups.rs`, and tests.

The main shell already passes `&Theme` explicitly into `draw_tabs`, `draw_breadcrumb`, and `draw_status_bar`, but `draw_content` still delegates to tab renderers without explicit theme plumbing.

`ui/shell.rs` still uses `tc!` in helper functions such as `get_tab_status` and `get_normal_status`.

`ui/popups.rs` still uses `tc!` broadly.

`Theme` currently uses `name: &'static str`, which is fine for built-ins but not ideal for future file-loaded themes.

`ThemeManager.current` is currently `pub(crate)` so `legacy.rs` can mutate it for thread-local compatibility.

## Phase 1: Baseline validation

Run and record baseline results before editing:

```bash
cargo fmt --all -- --check
cargo check -p eggsec-tui
cargo check -p eggsec-cli
cargo test -p eggsec-tui
cargo check -p eggsec-tui --features nse
cargo check -p eggsec-cli --features nse
```

If `--features full` is practical in the current environment, also run:

```bash
cargo check -p eggsec-cli --features full
```

If any check fails before changes, document the failure and continue only if it is clearly unrelated to this pass.

## Phase 2: Make `Theme` usable for non-static themes

Change `Theme.name` from `&'static str` to a type compatible with dynamically loaded themes.

Preferred option:

```rust
pub struct Theme {
    pub mode: ThemeMode,
    pub name: String,
    pub colors: ThemeColors,
}
```

Alternative acceptable option:

```rust
pub struct Theme {
    pub mode: ThemeMode,
    pub name: std::borrow::Cow<'static, str>,
    pub colors: ThemeColors,
}
```

Use `String` unless the diff strongly favors `Cow`.

Update built-in themes mechanically:

```rust
name: "dark".to_string(),
name: "light".to_string(),
```

Acceptance criteria:

- Existing dark/light themes still work.
- `ThemeManager::list_themes()` still returns stable names.
- Session theme restore still uses the theme name string as before.
- `cargo check -p eggsec-tui` passes.

## Phase 3: Tighten `ThemeManager` mutation boundaries

Remove or minimize direct mutation of `ThemeManager.current` outside `manager.rs`.

Add a method such as:

```rust
impl ThemeManager {
    pub(crate) fn set_current_for_legacy_sync(&mut self, theme: &Theme) {
        self.current = theme.clone();
    }
}
```

or a more neutral method:

```rust
pub(crate) fn set_current_unchecked(&mut self, theme: Theme) {
    self.current = theme;
}
```

Then update `theme/legacy.rs` to call that method rather than mutating `current` directly.

After this, make `ThemeManager.current` private if possible:

```rust
current: Theme,
```

Acceptance criteria:

- `legacy.rs` no longer reaches into `ThemeManager.current` directly.
- `ThemeManager` owns its own mutation paths.
- Existing tests still pass.

## Phase 4: Introduce an explicit render-theme helper type if useful

Decide whether passing `&Theme` directly is sufficient or whether a small wrapper improves clarity.

Acceptable simple path: continue passing `&Theme`.

Preferred path if it reduces verbosity: add a lightweight type in `theme/style.rs` or `theme/render.rs`:

```rust
pub struct ThemeStyles<'a> {
    pub theme: &'a Theme,
}

impl<'a> ThemeStyles<'a> {
    pub fn new(theme: &'a Theme) -> Self { Self { theme } }
    pub fn color(&self, token: ThemeColorToken) -> Color { ... } // optional, only if useful
}
```

Do not overbuild a token enum in this pass unless it clearly reduces duplication. The future Halloy pass needs semantic color fields more than a complex style abstraction.

Acceptance criteria:

- The chosen theme plumbing style is consistent in `ui/mod.rs`, `ui/shell.rs`, and `ui/popups.rs`.
- The code remains simpler than before.

## Phase 5: Remove `tc!` from shell-level status helpers

`ui/shell.rs` currently passes `&Theme` into main shell renderers, but helper functions still call `tc!`.

Update helper signatures so theme is explicit:

```rust
pub fn get_tab_status(state: &crate::tabs::AppState, theme: &Theme) -> (String, ratatui::style::Color)

pub fn get_normal_status(app: &App, theme: &Theme) -> (String, ratatui::style::Color)
```

Then update `draw_status_bar` to call:

```rust
get_normal_status(app, theme)
```

Replace mappings:

- `tc!(status_idle)` -> `theme.colors.status_idle`
- `tc!(status_running)` -> `theme.colors.status_running`
- `tc!(success)` -> `theme.colors.success`
- `tc!(error)` -> `theme.colors.error`

Acceptance criteria:

- `ui/shell.rs` no longer imports `crate::tc`.
- Shell rendering and status helpers are fully explicit-theme.
- `cargo check -p eggsec-tui` passes.

## Phase 6: Remove `tc!` from popup rendering

Update `ui/popups.rs` to accept explicit theme references.

Change the popup draw signatures:

```rust
pub fn draw_http_options_popup(f: &mut Frame, app: &App, theme: &Theme)
pub fn draw_command_palette(f: &mut Frame, app: &mut App, theme: &Theme)
pub fn draw_search_popup(f: &mut Frame, app: &App, theme: &Theme)
pub fn draw_quick_switch(f: &mut Frame, app: &mut App, theme: &Theme)
```

Update calls from `ui/mod.rs`, which already has:

```rust
let theme = app.theme_manager.current().clone();
```

Replace all `tc!(...)` usage in `popups.rs` with `theme.colors.<field>`.

Acceptance criteria:

- `ui/popups.rs` no longer imports `crate::tc`.
- All popup rendering uses explicit theme access.
- Visual semantics are unchanged.
- `cargo check -p eggsec-tui` passes.

## Phase 7: Audit remaining `tc!` usage and categorize it

Search for remaining macro uses:

```bash
rg "tc!|theme!" crates/eggsec-tui/src
```

Categorize each remaining use into one of three groups:

1. Easy render helper migration: function can accept `&Theme` without broad churn.
2. Tab-renderer migration candidate: requires changing `TabRender` or many tab render signatures.
3. Legacy compatibility hold: keep temporarily, with a comment or issue note.

For group 1, migrate now.

For group 2, do not rewrite everything in this pass. Instead, create a small internal note in the plan outcome or code comments describing what trait/signature change is needed later.

For group 3, leave in place but ensure all remaining macro usage is outside the main shell and popup layers.

Acceptance criteria:

- Main shell and popup layers have no `tc!`/`theme!` dependency.
- Remaining macro usage is documented by location and rationale.
- No new macro uses are introduced.

## Phase 8: Add a narrow theme context path for future tab render migration

Prepare for a later pass where tab renderers receive theme explicitly.

Do not force the full migration unless it is small. Instead, introduce one of these low-risk seams:

Option A: Add helper on `App`:

```rust
impl App {
    pub fn current_theme(&self) -> &Theme {
        self.theme_manager.current()
    }
}
```

Option B: Add a render context type:

```rust
pub struct RenderContext<'a> {
    pub theme: &'a Theme,
    pub insert_mode: bool,
}
```

Only choose option B if it can be introduced without changing every tab renderer immediately.

The likely best choice for this pass is option A. It creates a consistent access point and reduces direct `theme_manager.current()` calls.

Acceptance criteria:

- Future tab render migration has an obvious seam.
- No broad tab-render trait rewrite is required in this pass.

## Phase 9: Strengthen tests around theme behavior

Add or update tests in `theme/manager.rs`, `theme/palette.rs`, or a dedicated `theme/tests.rs`.

Recommended tests:

- Built-in theme names are owned strings and remain `dark`/`light`.
- `set_theme("light")` updates current name and mode.
- failed `set_theme` preserves current name and mode.
- legacy sync updates thread-local current theme through a method, not field access.
- shell status helpers return the same colors as theme fields.

For popup rendering, avoid brittle full snapshot tests. If a simple `TestBackend` smoke test exists or is easy to add, render the main UI with help/search/quick-switch overlays enabled and assert it does not panic.

Acceptance criteria:

- `cargo test -p eggsec-tui` passes.
- Tests cover the new dynamic-name and mutation-boundary behavior.

## Phase 10: Final validation

Run:

```bash
cargo fmt --all
cargo check -p eggsec-tui
cargo check -p eggsec-cli
cargo test -p eggsec-tui
cargo check -p eggsec-tui --features nse
cargo check -p eggsec-cli --features nse
```

If practical:

```bash
cargo check -p eggsec-cli --features full
```

Document any unrelated failures clearly.

## Expected end state

After this pass:

- `Theme.name` supports future dynamically loaded themes.
- `ThemeManager.current` is no longer directly mutated outside `manager.rs`.
- `ui/shell.rs` has no `tc!` dependency.
- `ui/popups.rs` has no `tc!` dependency.
- Any remaining `tc!` usage is limited to deeper tab rendering or explicitly documented legacy holdouts.
- The future Halloy-style theming pass can focus on config parsing, theme discovery, color conversion, fallbacks, and user-facing settings rather than structural cleanup.

## Notes for the implementing model

Keep this pass small and mechanical. The first simplification pass already did the major shape change. This pass should make the theme plumbing cleaner without destabilizing the TUI.

Prefer explicit `&Theme` parameters in rendering functions.

Do not introduce a large abstraction layer just because Halloy theming is planned. Eggsec needs a clear semantic palette and explicit plumbing first.

Run `cargo check -p eggsec-tui` after each phase.
