# Eggsec TUI Packaged Themes Correction Pass

## Purpose

This plan corrects the issues found after the first packaged Halloy theme implementation pass.

The previous pass successfully added the major primitives:

- `base64` and `lzma-rs` dependencies in `eggsec-tui`.
- `theme/archive.rs`, `theme/install.rs`, `theme/loader.rs`, and generated `theme/packaged.rs`.
- `scripts/package_themes.py` for deterministic theme packaging.
- Native in-binary `cyber-red` fallback theme.
- `ThemeManager` support for registering file-loaded themes.
- Initial Settings tab `theme_selector` replacement for the older dark/accent controls.

However, several integration and UX issues remain. This pass should make the feature robust enough to merge/use:

- Theme install/load must not block apparent TUI startup.
- Settings selector population and theme application must be complete and compile-checked.
- Theme selector dropdown focus must work correctly.
- File install details should be tightened.
- Stale Theme section text should be updated.
- The fallback path must stay Cyber Red-first and non-fatal.

## Non-goals

Do not redesign the Halloy mapping layer unless a clear parse bug is found.

Do not replace the custom archive format.

Do not remove native `cyber-red` fallback.

Do not overwrite existing user theme files.

Do not make packaged theme installation a hard startup dependency.

Do not add a full theme marketplace/plugin system.

## Baseline checks

Before changes, run:

```bash
cargo fmt --all -- --check
cargo check -p eggsec-tui
cargo check -p eggsec-cli
cargo test -p eggsec-tui
```

If `cargo check -p eggsec-tui` currently fails because `update_settings_theme_selector()` or related theme-selector wiring is missing, treat that as an expected target of this pass. Record the exact errors before fixing.

Also run:

```bash
rg "update_settings_theme_selector|pending_theme_name|take_pending_theme|theme_selector" crates/eggsec-tui/src
rg "load_and_install_themes" crates/eggsec-tui/src
```

Use this to identify all current call sites.

## Phase 1: Make startup use Cyber Red immediately and defer filesystem work

Current issue: `App::new_inner()` calls `theme::install::load_and_install_themes()` directly during app construction. That call can decode the embedded archive, create directories, write files, read the theme directory, and parse TOML before the TUI is visible. This violates the requirement that theme installation must not block or visibly slow startup.

Required behavior:

1. `App::new_inner()` should construct `ThemeManager::new()` and render using in-binary `cyber-red` immediately.
2. Theme install/load should happen after startup in a background path.
3. Theme install/load failures must be logged and optionally shown as a low-severity notification, but must not abort app creation.
4. When theme loading completes, registered themes should be merged into `ThemeManager`, the Settings selector should refresh, and the app should mark `needs_redraw = true`.

Preferred implementation:

Add theme loading to the existing update/event polling architecture. Introduce a channel/handle field in app state, for example:

```rust
pub struct ThemeLoadState {
    pub handle: Option<tokio::task::JoinHandle<()>>,
    pub result_rx: Option<tokio::sync::mpsc::Receiver<crate::theme::install::ThemeInstallReport>>,
    pub attempted: bool,
}
```

This can live in `app/state.rs` or directly on `App` if keeping the diff smaller.

At startup:

```rust
let mut app = Self { theme_manager: ThemeManager::new(), ... };
app.update_settings_theme_selector();
app.spawn_theme_loader();
```

`spawn_theme_loader()` should:

- create an mpsc channel,
- run `theme::install::load_and_install_themes()` inside `tokio::task::spawn_blocking` if a runtime exists,
- send the report back to App,
- never panic on send failure.

Use `spawn_blocking` rather than `tokio::spawn`, because filesystem work and decompression are blocking CPU/IO work.

If there is no runtime available in some test path, either skip background loading in `new_for_testing()` or guard with a helper that only starts loading in normal TUI construction.

Acceptance criteria:

- `App::new_inner()` no longer calls `load_and_install_themes()` synchronously.
- The first usable theme is always native `cyber-red` from `ThemeManager::new()`.
- App construction does not depend on theme directory access.
- Background load completion merges themes and refreshes Settings.
- Background load errors do not make the TUI quit.

## Phase 2: Add theme-load report handling in `App::update()`

Current `App::update()` already polls task progress/results. Add a similar poll for theme load results.

Suggested flow:

```rust
if let Some(rx) = self.theme_load.result_rx.as_mut() {
    while let Ok(report) = rx.try_recv() {
        self.handle_theme_install_report(report);
        dirty = true;
    }
}
```

Add:

```rust
fn handle_theme_install_report(&mut self, report: ThemeInstallReport) {
    for result in report.loaded_themes {
        match result {
            Ok(theme) => self.theme_manager.register_theme(theme),
            Err(err) => tracing::warn!(error = %err, "failed to load theme"),
        }
    }
    self.update_settings_theme_selector();

    // If a restored theme name was unavailable at startup, try it again here.
    // Otherwise preserve current theme.
}
```

If session restore can reference a theme that was not loaded at startup, add an `Option<String>` field such as `deferred_theme_name`. Set it when `state.theme_name` fails during initial restore. After loading themes, attempt to apply it once.

Important: do not switch away from Cyber Red just because themes finished loading. Preserve current theme unless:

- a deferred restored theme name becomes available, or
- the user has selected a theme.

Acceptance criteria:

- Loaded themes appear in the Settings selector after background loading completes.
- Current theme is preserved unless a deferred restored theme becomes available.
- Unknown restored theme names produce warnings, not failures.

## Phase 3: Implement or repair `update_settings_theme_selector()`

Current issue: `App::new_inner()` and `handle_enter()` call `update_settings_theme_selector()`, but repository search did not find the method. If absent, `eggsec-tui` will not compile. If present elsewhere, verify it works correctly.

Add a method on `App`:

```rust
pub(crate) fn update_settings_theme_selector(&mut self) {
    let current = self.theme_manager.current_name().to_string();
    let items = self
        .theme_manager
        .list_themes()
        .into_iter()
        .map(|name| SelectorItem::new(display_theme_name(name), name))
        .collect();
    self.tabs.settings.set_available_themes(items, &current);
}
```

Add helper:

```rust
fn display_theme_name(id: &str) -> String {
    id.split(['-', '_'])
      .filter(|s| !s.is_empty())
      .map(capitalize_first)
      .collect::<Vec<_>>()
      .join(" ")
}
```

Or keep labels equal to ids for now if simpler. Prefer readable labels for Halloy filenames.

On `SettingsTab`, add:

```rust
pub fn set_available_themes(&mut self, items: Vec<SelectorItem>, current_theme: &str) {
    self.theme_selector.set_items(items);
    self.theme_selector.select_by_value(current_theme);
}
```

Ensure `cyber-red` is always present even if no file themes load.

Acceptance criteria:

- `cargo check -p eggsec-tui` does not fail on missing `update_settings_theme_selector()`.
- Settings selector contains at least `cyber-red`, `dark`, and `light` immediately.
- After background load, selector contains packaged/user themes.
- Current theme remains selected.

## Phase 4: Wire selector changes to pending theme application

Current issue: `SettingsTab` has `pending_theme_name` and `take_pending_theme()`, but no obvious path sets `pending_theme_name` when the selector selection changes.

Find the `TabInput` implementation for `SettingsTab`. Update selector input handling so that when the Theme section is active and `theme_selector` changes/commits, it stores the selected value:

```rust
if self.current_section == SettingsSection::Theme {
    if let Some(value) = self.theme_selector.selected_value() {
        self.pending_theme_name = Some(value.to_string());
    }
}
```

Where to set this depends on current selector semantics:

- If `Enter` toggles/commits selector choices, set it on Enter when the selector is expanded and a value is selected.
- If Up/Down changes the selected value while expanded, either apply on Enter only or set pending on each selection movement. Prefer apply-on-Enter to avoid rapid theme churn.
- If `TabDispatcher::handle_enter()` returns after calling the tab input, ensure `App::handle_enter()` sees the pending value.

After applying a selected theme in `App::handle_enter()`:

```rust
if self.theme_manager.set_theme(&theme_name) {
    crate::theme::sync_theme_to_thread_local(self.theme_manager.current());
    self.update_settings_theme_selector();
    self.needs_redraw = true;
} else {
    tracing::warn!(theme = %theme_name, "unknown theme selected");
}
```

This logic already partially exists; verify it is reachable.

Acceptance criteria:

- User can navigate to Settings -> Theme, open selector, choose a theme, press Enter, and the active theme changes.
- Selector selection remains synced to active theme.
- Unknown/stale selected theme names warn but do not panic.
- Add a unit test if practical for `SettingsTab` pending theme behavior.

## Phase 5: Fix theme selector dropdown focus/close behavior

Current issue: `sync_component_focus()` resets `theme_selector.focused = false` and then immediately closes the selector based on the now-false state. This likely prevents the dropdown from staying open.

Add a `keep_theme_selector_open` boolean before resetting controls:

```rust
let keep_theme_selector_open =
    is_detail && self.current_section == SettingsSection::Theme && idx == 0;
```

Then:

```rust
self.theme_selector.focused = false;
if !keep_theme_selector_open {
    self.theme_selector.close();
}
```

In the Theme branch:

```rust
self.theme_selector.focused = true;
```

If existing selector focus semantics use `focus_open()`, follow that convention instead.

Acceptance criteria:

- Opening the theme dropdown does not immediately close after focus sync.
- Up/Down navigation works while expanded.
- Esc/collapse behavior still works.

## Phase 6: Fix installer atomic-write temp path and robustness

Current issue: `atomic_write()` builds temp files with `".{} .tmp"`, which includes an unintended space.

Change to a collision-resistant temp path:

```rust
let tmp = parent.join(format!(
    ".{}.{}.tmp",
    dest.file_name().and_then(|n| n.to_str()).unwrap_or("theme"),
    std::process::id()
));
```

Even better, include a small counter or timestamp if multiple writes can target the same filename concurrently. The installer normally writes sequentially, so pid is sufficient for this pass.

After writing:

- optionally call `file.sync_all()` before rename if implemented with `File`,
- rename to destination,
- best-effort remove temp file on failure.

Keep behavior non-fatal.

Acceptance criteria:

- Temp file name has no accidental spaces.
- Failed writes do not leave many stale temp files when easily avoidable.
- Existing files are still skipped and never overwritten.

## Phase 7: Align archive path validation with component-based safety

Current archive validation rejects any string containing `..`, which is safe but overly broad. The installer has better component-based validation. Align archive validation with component checks.

Replace:

```rust
if path.contains("..") { ... }
```

with logic based on `Path::components()`:

```rust
let path = Path::new(path_str);
if path.is_absolute() { reject }
if !path.components().all(|c| matches!(c, Component::Normal(_))) { reject }
```

Still require `.toml` extension.

Acceptance criteria:

- `../escape.toml` is rejected.
- absolute paths are rejected.
- `foo/../../bar.toml` is rejected.
- benign names containing literal dots, such as `my..theme.toml`, are accepted if otherwise normal.
- Tests cover both rejection and benign double-dot filename acceptance.

## Phase 8: Update stale Theme settings text

Current hint says: `Use Ctrl+T to toggle between dark/light themes`. That is stale because the new model uses a selector and includes Cyber Red plus Halloy themes.

Replace with something like:

```text
Bundled themes are installed when possible; Cyber Red is always available as fallback.
```

Optionally show the resolved theme directory if available, but do not require filesystem access in render code.

Acceptance criteria:

- Theme Settings UI no longer describes the old dark/light-only model.
- Text reflects non-fatal bundled theme behavior.

## Phase 9: Review `Ctrl+T` theme toggle behavior

`Ctrl+T` currently calls `app.toggle_theme()`. Verify what it now does.

Options:

1. Keep `Ctrl+T` as a quick cycle through built-in themes only: `cyber-red -> dark -> light -> cyber-red`.
2. Change `Ctrl+T` to cycle through all registered themes.
3. Remove/de-emphasize it from help text and make Settings selector the primary path.

Preferred: cycle through all registered themes only if deterministic ordering is stable and selector sync is reliable. Otherwise keep it as a built-in quick toggle and document it as such.

Acceptance criteria:

- `Ctrl+T` behavior is deterministic.
- Settings selector syncs after `Ctrl+T` changes the theme.
- Help/hint text does not contradict actual behavior.

## Phase 10: Add focused tests

Add tests for the correction points:

Theme manager / app:

- `ThemeManager::new()` defaults to `cyber-red`.
- `update_settings_theme_selector()` populates at least `cyber-red`, `dark`, and `light`.
- Selecting a known theme updates manager current name.
- Selecting an unknown theme does not panic and preserves current theme.

Settings tab:

- Theme selector is focused in Theme section.
- Theme selector does not close immediately when it should remain open.
- `set_available_themes()` selects current theme.
- Theme commit sets `pending_theme_name`.

Archive/install:

- Temp write does not overwrite existing files.
- Component-based validation rejects traversal.
- Component-based validation allows benign double-dot filenames.

Startup behavior:

- `App::new_for_testing()` does not run filesystem theme installation.
- App starts with `cyber-red` even when theme loading has not completed.

Avoid brittle full-screen snapshot tests unless the project already uses them.

## Phase 11: Validation commands

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

Run packaging determinism check:

```bash
python3 scripts/package_themes.py
git diff -- crates/eggsec-tui/src/theme/packaged.rs
python3 scripts/package_themes.py
git diff -- crates/eggsec-tui/src/theme/packaged.rs
```

The second run should produce no diff.

## Acceptance criteria for the whole pass

This pass is complete when:

- `cargo check -p eggsec-tui` passes.
- TUI startup does not synchronously install/load packaged themes from `App::new_inner()`.
- App starts immediately with native Cyber Red fallback.
- Theme install/load is best-effort, non-fatal, and handled after app construction.
- Settings selector is populated and synced from `ThemeManager`.
- User selection from Settings can actually apply a theme.
- Theme selector dropdown focus/open behavior works.
- Existing user theme files are not overwritten.
- Atomic temp filenames are cleaned up.
- Theme Settings text describes the new bundled-theme/fallback model.

## Implementation notes for smaller models

Prefer a compile-clean mechanical pass over broad redesign.

Do not touch the generated `packaged.rs` manually. Regenerate it only through `scripts/package_themes.py`.

Keep all theme loading failures as warnings/reports, never panics.

Do not make Settings render code perform filesystem work.

Keep Cyber Red native and always registered before any file-loaded theme work.
