# TUI Bugs, Usability, and Theme Refinement Plan

**Date**: 2026-06-18
**Status**: Detailed Plan - Ready for Implementation
**Module**: `crates/eggsec-tui/src/`
**Primary Reference**: `architecture/tui.md`

## 1. Executive Summary

The TUI module is in a healthy baseline state: `cargo check -p eggsec-tui` passes, and `cargo test -p eggsec-tui` passes with 488 unit tests plus 0 doctests. The current architecture already covers many recent regressions around overlay routing, action hints, settings theme selection, small terminal rendering, and `handle_enter()` reachability.

The remaining work should be a focused polish pass, not a rewrite. The highest-value fixes are:

1. Correct numeric tab jump semantics (`1` should select the first visible tab, not the second).
2. Make the Settings theme detail/preview pane describe the selected theme, not sometimes the currently applied theme.
3. Reserve Settings footer/status rows so validation and save messages remain visible on small panes.
4. Reduce theme-system drift by moving high-traffic components from thread-local `tc!()` access toward explicit `&Theme` rendering.
5. Clean up current warning debt in the TUI crate.

## 2. Interrogation Notes

Reviewed:

- `architecture/tui.md`
- `crates/eggsec-tui/src/AGENTS.override.md`
- `crates/eggsec-tui/src/app/key_handler.rs`
- `crates/eggsec-tui/src/app/mod.rs`
- `crates/eggsec-tui/src/app/navigation.rs`
- `crates/eggsec-tui/src/app/overlay.rs`
- `crates/eggsec-tui/src/app/action_hints.rs`
- `crates/eggsec-tui/src/app/theme_runtime.rs`
- `crates/eggsec-tui/src/tabs/mod.rs`
- `crates/eggsec-tui/src/tabs/spec.rs`
- `crates/eggsec-tui/src/tabs/settings/{main,input,render}.rs`
- `crates/eggsec-tui/src/theme/{manager,loader,palette,legacy,mod}.rs`
- `crates/eggsec-tui/src/ui/{mod,shell,popups,tests}.rs`
- `crates/eggsec-tui/src/components/{input,selector,popup,scrollable}.rs`
- `crates/eggsec-tui/src/session.rs`

Validation run:

- `cargo check -p eggsec-tui`: passed.
- `cargo test -p eggsec-tui`: passed, 488 tests.

Current warnings observed:

- `crates/eggsec-tui/src/app/theme_runtime.rs`: unused `ThemeSource` import.
- `crates/eggsec-tui/src/app/action_hints.rs`: unused test import `AppState`.
- `crates/eggsec-tui/src/tabs/mod.rs`: unused test re-export `all_specs`.
- `crates/eggsec-tui/src/tabs/spec.rs`: `has_settings`, and in lib builds several `TabSpec` capability fields/helper methods, are currently warning as unused.

## 3. High Priority Bug Fixes

### 3.1 Numeric Tab Jump Off By One

**Evidence**: `architecture/tui.md` documents `1-9 / 0` as `1=Recon`, `2=Load`, and `0=tab 10`. In `crates/eggsec-tui/src/app/key_handler.rs`, normal-mode numeric handling uses:

- `c.to_digit(10).unwrap() as usize`
- `Tab::from_index(idx)`
- `0` maps to `Tab::from_index(9)`

Because `Tab::from_index()` is a visible zero-based index into `Tab::all()`, pressing `1` currently selects visible index 1 (`Load`) instead of visible index 0 (`Recon`). Pressing `9` and `0` both effectively land near the tenth slot behavior instead of preserving documented 1-based shortcuts.

**Fix**:

1. Change numeric decode to map `'1'..='9'` to `digit - 1`.
2. Keep `'0'` mapped to visible index 9.
3. Prefer a small helper such as `visible_shortcut_index(c: char) -> Option<usize>` to make the 1-based rule explicit.
4. Keep `Tab::from_visible_index()` as the public semantic name in the key handler, even if it delegates to `from_index()`.

**Tests**:

1. Add decode/apply tests proving `1` selects `Tab::Recon`, `2` selects `Tab::Load`, and `0` selects the tenth visible tab.
2. Add a feature-agnostic test that uses `Tab::all()` so behavior remains correct with optional tabs enabled.
3. Add a regression test that numeric tab jumps call the visible-index path, not enum discriminants.

### 3.2 Theme Details Describe The Wrong Theme While Browsing

**Evidence**:

- `App::handle_theme_install_report()` computes `contrast_warnings` for `self.theme_manager.current_name()`.
- `SettingsTab::render()` labels the Theme pane from `theme_selector.selected_value()`.
- The preview row uses `tc!()` tokens, so it renders the currently applied thread-local theme even while the selector highlight has moved to another theme.

This means a user can browse a theme in the dropdown and see metadata/preview text that appears to describe the highlighted theme but is actually based on the applied theme.

**Fix**:

1. Store contrast warnings by theme ID in Settings metadata, not as one global `Vec<String>`.
2. In Theme render, resolve the selected theme ID to either:
   - the selected loaded theme, for preview and contrast display; or
   - an explicit unavailable/invalid state for placeholder and invalid entries.
3. Label the pane clearly with both selected and applied state when they differ, for example `Selected: X` and `Applied: Y`.
4. Change the theme preview row to use explicit `Theme` colors for the selected loaded theme. Do not use `tc!()` in this preview.
5. If the selected entry is invalid or missing, render a small unavailable state instead of a misleading palette preview.

**Tests**:

1. Add a Settings render test where current theme is `cyber-red`, selector highlights `light`, and the preview uses `light` colors.
2. Add a unit test for per-theme contrast warning selection.
3. Add a render test for a missing placeholder entry that shows the unavailable marker and does not claim contrast is OK.

### 3.3 Settings Footer And Status Layout Can Collide Or Hide Feedback

**Evidence**: `SettingsTab::render()` draws normal section content into `inner`, then draws `status_message` at `inner.y + inner.height.saturating_sub(2)` and a persistent footer at `inner.y + inner.height.saturating_sub(1)`. The section renderers do not reserve those rows ahead of time, and the Theme pane independently computes metadata, selector, preview, and hint rows in the same full `inner` area.

**Fix**:

1. Split the Settings content area into `body`, optional `status`, and `footer` rows before rendering any section.
2. Render all section forms and theme content inside `body` only.
3. Truncate long status messages with a stable helper so they do not overwrite the UI on narrow terminals.
4. Use severity-aware status styling: validation errors/warnings should not use success color.

**Tests**:

1. Add 60x20 and 80x24 Settings render tests with long validation status text.
2. Add a Theme-section render test proving footer remains visible and preview/hint content does not occupy the footer row.

## 4. Usability Improvements

### 4.1 Make Theme Operations More Discoverable And Predictable

Tasks:

1. In Settings > Theme, show separate applied and selected state whenever the dropdown is open or the selected item differs from the applied theme.
2. Add a status message after successful theme reload that includes invalid theme count if nonzero.
3. When selecting an invalid/missing theme placeholder, keep the dropdown open or emit a clear warning without implying the selection was applied.
4. Include theme source/status in the selector label or adjacent detail line: Built-in, Packaged, Custom, Invalid, FallbackAdjusted.
5. Add command palette entries for theme reload and "open theme directory" only if a safe non-GUI action exists. Do not spawn a file manager from TUI without explicit approval.

### 4.2 Improve Keyboard Help Consistency

Tasks:

1. Add numeric tab shortcuts to action hints only when width allows, or add them to Help overlay keymap.
2. Make Settings Theme hints distinguish `Enter: open/select`, `r: reload`, and `Ctrl+T: cycle`.
3. Add tests ensuring action hints do not advertise unavailable actions during active tasks or policy confirmation.

### 4.3 Improve Global Search And Quick Switch Feedback

Tasks:

1. Show result counts in the Search popup the same way Quick Switch already does.
2. Keep Quick Switch selected index clamped after feature-gated tabs change across sessions.
3. Consider showing stable IDs in Quick Switch for ambiguous names only, keeping the default list readable.

### 4.4 Clarify Running Task Visibility

Tasks:

1. Keep the current global active-task status behavior.
2. Add an optional inline marker in the tab bar for the active task tab when the user navigates away.
3. Add a render test proving the active-task tab marker does not break compact/narrow tab mode.

## 5. Theme System Refinement

### 5.1 Move Core Components Away From Thread-Local Theme Access

The architecture now recommends explicit `&Theme` parameters for new rendering code, but many components and tabs still use `tc!()` and `theme::legacy::current_theme()`.

Recommended order:

1. Convert `FormBuilder` to accept and pass `&Theme` through to `InputField`, `Checkbox`, and `Selector`.
2. Convert Settings rendering first, since theme preview correctness depends on explicit theme selection.
3. Convert popups and reusable components next: `Popup`, `ScrollableText`, `ProgressGauge`, `empty_state_paragraph`.
4. Leave tab-specific `tc!()` migration for a later broad visual cleanup unless touching those tabs for another bug.

Acceptance criteria:

1. `SettingsTab::render()` can render a selected theme preview without changing thread-local global state.
2. Existing component `render()` methods may stay as compatibility wrappers, but new call sites should prefer `render_with_theme`.
3. Add tests proving `sync_theme_to_thread_local()` is not required for Settings theme preview correctness.

### 5.2 Strengthen Theme Metadata Model

Tasks:

1. Add `contrast_warnings: Vec<String>` or `contrast_status` to `ThemeInfo`, or add a `ThemeDiagnostics` map keyed by canonical ID.
2. Preserve load error details for invalid themes in Settings, but truncate them in UI to avoid layout overflow.
3. Distinguish `FallbackAdjusted` from `Loaded` in the selector/details pane.
4. Add a `loaded_theme_count` accessor or include loaded/invalid/fallback counts in Settings metadata, so the UI does not infer status from `theme_info_cache.len()`.

### 5.3 Expand Contrast Validation Coverage

Current validation checks `text/background` and `selected_text/selected`. Add warnings for high-use semantic pairs:

1. `text_dim/background`
2. `warning/background`
3. `error/background`
4. `success/background`
5. `mode_normal/background`
6. `mode_insert/background`
7. `focus_input/background`

Keep this non-fatal at first. Do not reject custom themes solely because a semantic accent is low contrast.

### 5.4 Theme Reload Lifecycle

Tasks:

1. Remove unused `ThemeSource` import in `theme_runtime.rs`.
2. Ensure manual reload reports mention loaded, invalid, and fallback-adjusted counts.
3. Add a test for reload while a deferred theme restore exists and user has not changed theme.
4. Add a test for reload after user changes theme, proving deferred restore is still suppressed.

## 6. Warning And Maintenance Cleanup

Tasks:

1. Remove unused imports:
   - `crate::theme::ThemeSource` in `app/theme_runtime.rs`
   - `crate::tabs::AppState` in `app/action_hints.rs` tests
   - unused `all_specs` test re-export in `tabs/mod.rs`, or use it in a registry integrity test.
2. Decide whether `TabSpec::{supports_run,supports_export,supports_help,has_settings}` are runtime metadata or test-only metadata:
   - if runtime, wire them into action hints/export/help gating where appropriate;
   - if test-only/future-only, annotate with clear `#[allow(dead_code)]` comments.
3. Keep the `cargo check -p eggsec-tui` warning baseline from growing.

## 7. Implementation Order

1. Numeric tab shortcut bug and tests.
2. Warning cleanup with no behavior change.
3. Settings layout split for body/status/footer and tests.
4. Theme selected-vs-applied metadata and preview correction.
5. Per-theme diagnostics and contrast coverage.
6. Component explicit-theme migration for Settings/FormBuilder path.
7. Optional usability polish: active task tab marker, richer search counts, refined hints.

This order fixes the clearest behavioral bug first, keeps the tree easy to validate, then moves into theme-model work that touches more rendering code.

## 8. Verification Matrix

Run after each implementation slice:

```bash
cargo check -p eggsec-tui
cargo test -p eggsec-tui
```

Run after theme-related slices:

```bash
cargo test -p eggsec-tui theme::
cargo test -p eggsec-tui tabs::settings
cargo test -p eggsec-tui ui::
```

Recommended visual regression additions:

1. Settings Theme at 60x20, 80x24, and 120x40.
2. Settings with selector open and selected theme different from applied theme.
3. Numeric tab jump behavior under the default feature set.
4. Quick Switch and tab bar with active task on a non-current tab.

## 9. Success Criteria

1. Numeric shortcuts match documented keymap exactly.
2. Settings Theme pane never labels current-theme diagnostics as selected-theme diagnostics.
3. Theme preview can render selected loaded themes without mutating global thread-local theme state.
4. Settings status and footer remain visible and non-overlapping at supported terminal sizes.
5. `cargo check -p eggsec-tui` and `cargo test -p eggsec-tui` pass.
6. TUI warning count does not increase; ideally the current TUI warnings are removed or explicitly justified.
7. Architecture docs can be updated afterward with no contradiction between documented keymap, theme preview behavior, and implementation.

## 10. Follow-Up Documentation

After implementation, update:

1. `architecture/tui.md` with corrected numeric shortcut semantics and theme diagnostics behavior.
2. `crates/eggsec-tui/src/AGENTS.override.md` with any new theme-rendering guidance.
3. Any keymap/help text that currently omits numeric shortcuts or theme reload behavior.
