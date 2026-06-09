# Eggsec TUI Packaged Themes Final Polish Plan

## Purpose

This is a narrow follow-up to the packaged Halloy theme correction pass. The previous pass fixed the major startup and selector issues:

- `App::new_inner()` now starts immediately with in-memory themes and calls `spawn_theme_loader()` instead of synchronously installing/loading packaged themes.
- Theme loading now runs in a background thread and reports back through `theme_load_rx`.
- `App::update()` polls the theme-load receiver and merges loaded themes.
- `update_settings_theme_selector()` exists and refreshes the Settings theme selector from `ThemeManager`.
- Settings theme selector confirmation now sets `pending_theme_name`, and `App::handle_enter()` applies the selected theme.
- Theme selector dropdown focus behavior was fixed.
- Archive path validation and installer temp path handling were improved.
- Theme Settings help text now describes bundled-theme fallback behavior.

This pass should finish the remaining polish: deferred restored-theme retry, background thread cleanup, selector display labels, and validation/test hardening.

## Non-goals

Do not redesign packaged theme loading.

Do not change the custom archive format.

Do not remove native `cyber-red`.

Do not add another dependency unless tests already require it and the dependency is dev-only.

Do not make filesystem theme loading synchronous again.

Do not turn theme install/load failures into startup failures.

## Baseline checks

Run before editing:

```bash
cargo fmt --all -- --check
cargo check -p eggsec-tui
cargo check -p eggsec-cli
cargo test -p eggsec-tui
```

Also inspect the relevant current code:

```bash
rg "theme_load_rx|theme_load_handle|spawn_theme_loader|handle_theme_install_report|update_settings_theme_selector" crates/eggsec-tui/src/app
rg "theme_selector|pending_theme_name|set_available_themes|sync_theme_selector" crates/eggsec-tui/src/tabs/settings
rg "ThemeManager::new|cyber-red|register_theme" crates/eggsec-tui/src/theme
```

If `cargo check -p eggsec-tui` fails, fix compile errors first before making behavioral changes.

## Phase 1: Add deferred restored-theme retry

Current issue: `App::new_inner()` attempts to restore `state.theme_name` before packaged/user themes have loaded. If the restored theme is a packaged/user theme, `set_theme()` fails and the app keeps Cyber Red even if the theme becomes available after the background load completes.

Add a field to `App`:

```rust
pub deferred_theme_name: Option<String>,
```

Initialize it to `None` in both `App::new_for_testing()` and `App::new_inner()`.

In `App::new_inner()`, change restore logic:

```rust
if let Some(state) = &restored_state {
    if app.theme_manager.set_theme(&state.theme_name) {
        crate::theme::sync_theme_to_thread_local(app.theme_manager.current());
    } else {
        tracing::warn!(theme = %state.theme_name, "theme unavailable at startup; will retry after theme load");
        app.deferred_theme_name = Some(state.theme_name.clone());
        crate::theme::sync_theme_to_thread_local(app.theme_manager.current());
    }
}
```

In `handle_theme_install_report()`, after registering loaded themes but before updating the selector, retry once:

```rust
if let Some(theme_name) = self.deferred_theme_name.take() {
    if self.theme_manager.set_theme(&theme_name) {
        crate::theme::sync_theme_to_thread_local(self.theme_manager.current());
        tracing::info!(theme = %theme_name, "restored deferred theme after theme load");
    } else {
        tracing::warn!(theme = %theme_name, "deferred theme still unavailable after theme load");
    }
}
```

Important behavior:

- Do not switch to a newly loaded theme unless it matches the deferred restored theme.
- If the user manually selected a different theme before background loading completed, do not override the user's choice. To support this, add a boolean or compare current theme. A simple safe approach:

```rust
let current_before_load = self.theme_manager.current_name().to_string();
let may_apply_deferred = current_before_load == "cyber-red";
```

Better approach: add `theme_changed_by_user: bool` and set it when the Settings selector or `Ctrl+T` changes theme. Only apply deferred theme if `!theme_changed_by_user`.

Acceptance criteria:

- Saved sessions referencing packaged/user themes restore correctly after background load.
- Unknown saved theme names warn once and keep Cyber Red.
- User-selected themes are not overwritten by delayed restore.

## Phase 2: Clean up background thread handle lifecycle

Current issue: `theme_load_handle` is stored but not cleared or joined after the report is received.

When `App::update()` receives a theme install report, clean up the handle:

```rust
if let Some(handle) = self.theme_load_handle.take() {
    if let Err(err) = handle.join() {
        tracing::warn!(?err, "theme loading thread panicked");
    }
}
```

Because the report has already been received, joining should be non-blocking or effectively immediate. If the sender can send before all work is truly complete, fix that invariant first: the thread should only send the report as its final action.

When `TryRecvError::Disconnected` occurs, also take/join/drop the handle and log a warning.

Acceptance criteria:

- `theme_load_handle` becomes `None` after successful report handling.
- Disconnected/panic cases are logged and do not affect TUI startup.
- No blocking join occurs before the report is received.

## Phase 3: Prevent duplicate theme-loader spawn

Add a guard in `spawn_theme_loader()`:

```rust
if self.theme_load_rx.is_some() || self.theme_load_handle.is_some() {
    tracing::debug!("theme loader already running");
    return;
}
```

This prevents accidental duplicate thread creation if the method is called from future paths.

Acceptance criteria:

- Calling `spawn_theme_loader()` twice does not create two loaders.
- Existing startup behavior remains unchanged.

## Phase 4: Improve theme display labels

Current selector labels are raw IDs because `update_settings_theme_selector()` maps `(id, id)`. Improve display labels without changing stable values.

Add helper near `update_settings_theme_selector()` or in a theme UI helper module:

```rust
fn display_theme_name(id: &str) -> String {
    id.trim_end_matches(".toml")
        .split(['-', '_', ' '])
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut chars = s.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
```

Keep selector values as canonical theme IDs exactly matching `Theme.name`.

Examples:

- `cyber-red` -> `Cyber Red`
- `dark` -> `Dark`
- `catppuccin-mocha` -> `Catppuccin Mocha`
- `Cyber Red` -> `Cyber Red`

Acceptance criteria:

- Settings dropdown remains value-stable.
- User sees readable labels.
- Selecting a theme still passes the canonical ID/value to `ThemeManager::set_theme()`.

## Phase 5: Normalize theme IDs from filenames only if safe

Current loader uses the file stem directly as `Theme.name`. Depending on packaged file names, this may produce names like `Cyber Red` instead of `cyber-red`. Decide whether to normalize IDs.

Preferred approach:

- Keep existing names if many tests/session files already depend on them.
- Otherwise add a small canonicalization helper:

```rust
pub fn canonical_theme_id(name: &str) -> String {
    name.trim()
        .trim_end_matches(".toml")
        .to_lowercase()
        .replace(' ', "-")
        .replace('_', "-")
}
```

If canonicalizing, apply it consistently:

- native Cyber Red is `cyber-red`,
- packaged `Cyber Red.toml` maps to `cyber-red`,
- selector values use canonical IDs,
- session restore uses canonical IDs.

Caution: changing IDs may affect existing saved sessions. If this is a concern, support alias restore:

```rust
if !set_theme(saved) {
    set_theme(&canonical_theme_id(saved))
}
```

Acceptance criteria:

- No duplicate Cyber Red entry appears as both `cyber-red` and `Cyber Red`.
- File themes have stable IDs.
- Existing saved sessions using old names get a best-effort alias fallback.

## Phase 6: Ensure `Ctrl+T` behavior is deterministic and selector-synced

Current `toggle_theme()` calls `theme_manager.toggle()`, syncs the thread-local theme, and refreshes the selector. Confirm `ThemeManager::toggle()` behavior now that Cyber Red and loaded themes exist.

Options:

1. Keep `toggle()` as dark/light only. If so, rename UI/help language to “toggle dark/light” and leave Cyber Red/packaged themes to Settings selector.
2. Change `toggle()` to cycle through built-in themes only: `cyber-red -> dark -> light -> cyber-red`.
3. Change `toggle()` to cycle through all registered themes sorted by ID.

Preferred for now: built-in cycle only, because cycling through 50 packaged themes with `Ctrl+T` is likely noisy.

Implement deterministically in `ThemeManager`:

```rust
pub fn toggle(&mut self) {
    let next = match self.current.name.as_str() {
        "cyber-red" => "dark",
        "dark" => "light",
        _ => "cyber-red",
    };
    let _ = self.set_theme(next);
}
```

Then make sure `App::toggle_theme()` calls `update_settings_theme_selector()` and marks redraw if needed.

If tracking user changes for deferred restore, set `theme_changed_by_user = true` in `toggle_theme()`.

Acceptance criteria:

- `Ctrl+T` is deterministic.
- Selector stays synced after `Ctrl+T`.
- Deferred restored theme does not override a user `Ctrl+T` choice.

## Phase 7: Add or harden tests

Add focused tests instead of broad snapshot tests.

Suggested tests:

### App/theme loading

- `App::new_for_testing()` starts with `cyber-red` and does not spawn a theme loader.
- `App::new()`/`new_inner()` initializes theme loader state without synchronously loading themes.
- `handle_theme_install_report()` registers loaded themes and refreshes selector.
- Deferred theme restore applies when the loaded theme appears.
- Deferred theme restore does not override a user-selected theme.
- Theme load disconnected path clears handle/receiver and does not panic.

### Settings selector

- `set_available_themes()` preserves selected current value.
- `theme_selector.confirm()` sets `pending_theme_name`.
- Theme selector remains open through `sync_component_focus()` when it should.
- Theme labels are readable while values remain canonical.

### Theme manager

- `toggle()` sequence is deterministic.
- `cyber-red` remains native and cannot be overwritten by file-loaded theme.
- Canonical ID helper, if added, maps `Cyber Red.toml`, `Cyber Red`, `cyber_red`, and `cyber-red` consistently.

### Archive/install regression

- Existing files are not overwritten.
- Benign double-dot file names are accepted if that behavior was implemented.
- Path traversal is rejected.

Acceptance criteria:

- `cargo test -p eggsec-tui` passes.
- Tests specifically cover the deferred restore and user override behavior.

## Phase 8: Validation commands

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

## Acceptance criteria for the full pass

This pass is complete when:

- Saved packaged/user theme names can restore after background loading completes.
- Delayed restore does not override user theme changes.
- Theme loader thread handle is cleaned up after completion/disconnect.
- Duplicate loader spawn is guarded.
- Settings selector labels are readable while values remain stable.
- `Ctrl+T` behavior is deterministic and selector-synced.
- `cargo check -p eggsec-tui` and `cargo test -p eggsec-tui` pass.

## Implementation notes for smaller models

Keep this pass narrow. The core packaged-theme functionality already exists.

Do not reintroduce synchronous filesystem work in `App::new_inner()`.

Do not panic on theme load/install failures.

Prefer preserving Cyber Red fallback over strict theme restore semantics.

Do not manually edit generated `packaged.rs`; use `scripts/package_themes.py` if regeneration is necessary.
