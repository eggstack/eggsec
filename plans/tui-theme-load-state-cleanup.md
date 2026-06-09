# Slapper TUI Theme Load State Cleanup Plan

## Purpose

Validation now passes for the packaged Halloy theme work. This follow-up is structural cleanup only.

The current implementation is functionally sound:

- The TUI starts with native `cyber-red` and does not block on packaged theme install/load.
- Background theme loading runs after app construction.
- Loaded themes are merged into `ThemeManager` from the app update loop.
- Deferred restored themes are retried after background load.
- User theme changes are protected from delayed restore.
- Settings theme selector values are canonical and labels are readable.
- `Ctrl+T` has deterministic built-in cycling.

This pass should reduce `App` field clutter by moving theme-loading runtime state into a dedicated state struct and tightening naming/documentation around the packaged-theme runtime path. It should not change behavior.

## Non-goals

Do not change packaged theme archive format.

Do not change Halloy TOML parsing or color mapping.

Do not change default theme behavior.

Do not make theme install/load synchronous.

Do not add or remove packaged themes.

Do not edit generated `crates/slapper-tui/src/theme/packaged.rs` by hand.

Do not redesign Settings UI.

## Baseline validation

The user reported validation passes. Still run the baseline before editing to catch drift:

```bash
cargo fmt --all -- --check
cargo check -p slapper-tui
cargo test -p slapper-tui
```

Also inspect current fields and call sites:

```bash
rg "theme_load_rx|theme_load_handle|deferred_theme_name|theme_changed_by_user" crates/slapper-tui/src/app
rg "spawn_theme_loader|join_theme_loader_handle|handle_theme_install_report" crates/slapper-tui/src/app
```

## Current state to preserve

`App` currently owns these theme runtime fields directly:

```rust
pub theme_load_rx: Option<std::sync::mpsc::Receiver<crate::theme::install::ThemeInstallReport>>,
pub theme_load_handle: Option<std::thread::JoinHandle<()>>,
pub deferred_theme_name: Option<String>,
pub theme_changed_by_user: bool,
```

`spawn_theme_loader()` has a duplicate-spawn guard and starts a background thread.

`App::update()` polls `theme_load_rx` and calls `handle_theme_install_report()` plus `join_theme_loader_handle()`.

`handle_theme_install_report()` registers loaded themes, applies a deferred restored theme if appropriate, and refreshes Settings.

This behavior should remain unchanged.

## Phase 1: Add `ThemeLoadState`

Add a small state type, preferably in `crates/slapper-tui/src/app/state.rs` near the other grouped state structs:

```rust
pub struct ThemeLoadState {
    pub rx: Option<std::sync::mpsc::Receiver<crate::theme::install::ThemeInstallReport>>,
    pub handle: Option<std::thread::JoinHandle<()>>,
    pub deferred_theme_name: Option<String>,
    pub changed_by_user: bool,
}

impl Default for ThemeLoadState {
    fn default() -> Self {
        Self {
            rx: None,
            handle: None,
            deferred_theme_name: None,
            changed_by_user: false,
        }
    }
}
```

If `Clone`/`Debug` derives are not possible because of `Receiver` or `JoinHandle`, do not force them. Keep the type simple.

Add small helper methods if they reduce app-level boilerplate:

```rust
impl ThemeLoadState {
    pub fn is_running(&self) -> bool {
        self.rx.is_some() || self.handle.is_some()
    }

    pub fn mark_user_changed(&mut self) {
        self.changed_by_user = true;
        self.deferred_theme_name = None;
    }
}
```

Acceptance criteria:

- `ThemeLoadState::default()` is available.
- No behavior changes yet beyond introducing the type.

## Phase 2: Replace direct `App` fields

Replace the four direct `App` fields with one grouped field:

```rust
pub theme_load: ThemeLoadState,
```

Update imports/exports in `app/mod.rs`:

```rust
pub use state::{OverlayState, QuickSwitchState, SearchState, TaskState, ThemeLoadState};
```

Update `App::new_for_testing()` and `App::new_inner()`:

```rust
theme_load: ThemeLoadState::default(),
```

Update restore logic:

```rust
app.theme_load.deferred_theme_name = Some(state.theme_name.clone());
```

Update user-change tracking:

```rust
self.theme_load.changed_by_user = true;
```

or use `self.theme_load.mark_user_changed()`.

Acceptance criteria:

- The direct fields no longer exist on `App`.
- Initial behavior is unchanged.
- Tests and compile errors guide any missed field references.

## Phase 3: Move loader lifecycle helpers to a focused module if useful

Currently `spawn_theme_loader()`, `join_theme_loader_handle()`, and `handle_theme_install_report()` live in `app/mod.rs`. Consider moving them into a dedicated module:

```text
crates/slapper-tui/src/app/theme_runtime.rs
```

Add in `app/mod.rs`:

```rust
pub(crate) mod theme_runtime;
```

Move the implementation block methods there:

```rust
impl super::App {
    pub fn spawn_theme_loader(&mut self) { ... }
    pub(crate) fn join_theme_loader_handle(&mut self) { ... }
    pub fn handle_theme_install_report(&mut self, report: ThemeInstallReport) { ... }
}
```

This is optional, but recommended if `app/mod.rs` is getting too large. Keep methods on `App` so callers do not need broader rewiring.

Acceptance criteria:

- Theme runtime lifecycle code is grouped either in `ThemeLoadState` helpers or `app/theme_runtime.rs`.
- `app/mod.rs` is slightly smaller and easier to scan.
- Public behavior is unchanged.

## Phase 4: Update `App::update()` polling code

Update state polling in `crates/slapper-tui/src/app/state_update.rs` from direct fields to grouped state:

```rust
if let Some(rx) = self.theme_load.rx.take() {
    match rx.try_recv() {
        Ok(report) => {
            self.handle_theme_install_report(report);
            self.join_theme_loader_handle();
            dirty = true;
        }
        Err(std::sync::mpsc::TryRecvError::Empty) => {
            self.theme_load.rx = Some(rx);
        }
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            tracing::warn!("Theme loading thread disconnected without sending report");
            self.join_theme_loader_handle();
        }
    }
}
```

If helper methods are added to `ThemeLoadState`, prefer using them for readability.

Acceptance criteria:

- Polling behavior is unchanged.
- Receiver is restored on `Empty`.
- Handle is joined after report or disconnect.
- No blocking join occurs before report/disconnect.

## Phase 5: Tighten naming/comments

Add comments that clarify the theme-load path is intentionally non-blocking and best-effort. Suggested comment on `ThemeLoadState`:

```rust
/// Runtime state for best-effort packaged/user theme loading.
/// The TUI must remain usable with built-in themes even if this loader fails.
```

Add comment near deferred restore:

```rust
// Saved sessions can reference packaged/user themes that are not registered until
// the background loader finishes. Defer one retry instead of blocking startup.
```

Avoid excessive comments elsewhere.

Acceptance criteria:

- Future maintainers can quickly see why theme loading is decoupled from startup.
- Comments do not duplicate obvious code.

## Phase 6: Add focused regression tests

Add or update tests to protect the structural behavior.

Recommended tests:

1. `ThemeLoadState::default()` has no receiver/handle, no deferred theme, and `changed_by_user == false`.
2. `ThemeLoadState::is_running()` returns true if either receiver or handle exists. If constructing a real thread in a test is awkward, skip this helper test.
3. `App::new_for_testing()` has `theme_load.rx == None`, `theme_load.handle == None`, and current theme `cyber-red`.
4. `spawn_theme_loader()` does not create a duplicate loader if one is already running. This can be tested lightly by checking state before/after, but avoid flaky thread timing.
5. `handle_theme_install_report()` clears deferred theme when `changed_by_user == true`.
6. `handle_theme_install_report()` applies a deferred theme when a matching theme is included in the report and `changed_by_user == false`.

Keep tests deterministic. Do not rely on actual user config directories when avoidable.

Acceptance criteria:

- New tests pass reliably.
- No test spawns long-running background work that can hang the suite.

## Phase 7: Validation

Run:

```bash
cargo fmt --all
cargo check -p slapper-tui
cargo check -p slapper-cli
cargo test -p slapper-tui
cargo check -p slapper-tui --features nse
cargo check -p slapper-cli --features nse
```

If practical:

```bash
cargo check -p slapper-cli --features full
```

Packaging should not need regeneration for this pass. If `packaged.rs` changes, verify it was generated by:

```bash
python3 scripts/package_themes.py
```

## Acceptance criteria for the full pass

This pass is complete when:

- `App` has a single `theme_load: ThemeLoadState` field instead of four direct theme-loader fields.
- Theme loading remains non-blocking and best-effort.
- Deferred restored-theme retry behavior is unchanged.
- User theme changes still suppress delayed restore.
- Loader thread lifecycle cleanup still happens on report or disconnect.
- Validation commands pass.

## Implementation notes for smaller models

This is a refactor-only cleanup. Keep changes mechanical.

Do not change user-visible behavior.

Do not touch packaged theme generation unless validation reveals a generated-file drift.

Let compiler errors guide all field-reference updates.
