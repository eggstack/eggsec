# TUI Style, Usability, And Navigation Improvement Plan

## Status

Open. This file has been pruned: previously completed workstreams were removed from the active plan. The items below are not complete yet, or are intentionally deferred until the implementing agent can verify them in code.

Scope is limited to the terminal UI under `crates/slapper/src/tui/`.

Before making changes, read:

- `AGENTS.md`
- `crates/slapper/src/tui/AGENTS.override.md`

## Goals

- Make TUI navigation predictable across normal mode, insert mode, overlays, tabs, selectors, checkboxes, and results panes.
- Ensure `h` and `l` work correctly as left/right in-pane navigation, and do not conflict with tab switching or overlay close behavior.
- Fix search and overlay interaction bugs that make visible keybindings lie to users.
- Improve visual hierarchy and readability without broad rewrites.
- Add focused regression tests so future TUI changes do not re-break navigation.

## Current Baseline

Last verified command:

```bash
cargo test --lib -p slapper tui::
```

Result: `136 passed`, with existing warnings unrelated to this plan.

This means the current TUI unit/render tests pass, but the tests do not cover several important event-loop paths and usability contracts described below.

## Important Existing Constraints

- Use `Tab::all()`, `Tab::visible_index()`, and stable IDs for tab availability. Do not use enum discriminants for visible tab navigation.
- Use `App::set_current_tab_if_available(tab)` when switching tabs from commands, mouse, session restore, or numeric selection.
- Preserve overlay precedence:
  1. Confirm popup
  2. Command palette
  3. Search
  4. HTTP options
  5. Help
  6. Tab content
- `InputField::cursor_pos` should remain a byte index at a valid UTF-8 boundary. Convert to display/char position only during rendering.
- Use `tc!` semantic theme colors in TUI rendering.
- Keep changes scoped and incremental. Fix shared shell behavior first, then representative tabs, then repeated patterns.

## Workstream 1: Fix Search Event Routing And Scope

**Status: COMPLETED** (2026-05-02, commit on branch `fix/workstream1-search-routing`)

### Changes Made
- Fixed `Ctrl+F` handler in `runner.rs` to use `toggle_search(true)` instead of directly setting `app.show_search = true`
- Updated overlay guard to properly handle `Enter` (calls `perform_search()`), `Backspace` (pops from query), and `Ctrl+U` (clears query) when search is visible
- Removed redundant search handling from later in the match statement (Enter handler and Char handler)
- Added tests for `search_is_global` flag, search query manipulation, and `perform_search()` with empty query

### Problem

Search is currently blocked by event routing in `crates/slapper/src/tui/app/runner.rs`.

Specific issues:

- The overlay guard at roughly `runner.rs:356` catches `app.is_search_visible()` before the later `Enter`, `Backspace`, and search character handlers.
- While search is open, plain characters append to `app.search_query`, but `Enter` does not run `perform_search()` and `Backspace` does not edit the query.
- `Ctrl+F` directly sets `app.show_search = true` around `runner.rs:246`, bypassing `toggle_search(true)`, so `search_is_global` is not reliably set.
- `/` should open current-tab search and `Ctrl+F` should open global search. The current implementation does not consistently enforce that distinction.

### Required Fix

- Route all key input through the active overlay first. For `OverlayType::Search`, handle:
  - `Esc`: close search and restore any History backup.
  - `Enter`: run `perform_search()`.
  - `Backspace`: remove one char from `search_query`.
  - Plain chars: append to `search_query`.
  - Optional: `Ctrl+U` clears the query if that fits existing style.
- Replace direct `app.show_search = true` in the `Ctrl+F` handler with `app.toggle_search(true)` or an explicit helper that sets `search_is_global = true` and clears stale query/results.
- Ensure `/` uses `toggle_search(false)`.
- Clear stale global search results when opening a new empty global search prompt.
- Keep History local search behavior intact: cancel should restore the original entries.

### Acceptance Criteria

- Pressing `Ctrl+F`, typing a query, and pressing `Enter` performs global search using the typed query.
- Pressing `/`, typing a query, and pressing `Enter` performs local/current-tab search where supported.
- `Backspace` works while the search prompt is open.
- `Esc` closes search and restores History filtering if applicable.
- Empty queries do not show stale previous results.

### Tests

- Add unit tests for a small extracted search-key handler if one is introduced.
- Add app-level tests for `toggle_search(true)` and `toggle_search(false)` setting `search_is_global` correctly.
- Add regression tests for `perform_search()` clearing or replacing stale global results.
- If event-loop logic remains hard to unit test, extract a pure helper that takes key/modifier plus overlay state and returns the intended action.

## Workstream 2: Define And Enforce `h/l` Navigation Semantics

### Problem

The user specifically asked to ensure `h/l` works correctly for navigation. Current behavior is partly correct but inconsistent across tabs and easy to regress.

Current global behavior:

- In Normal mode, `h` and Left call `app.handle_left()`.
- In Normal mode, `l` and Right call `app.handle_right()`.
- `n`, `N`, `p`, `Shift+H`, and `Shift+L` switch tabs.
- In Insert mode, `h` and `l` should type literal characters into focused inputs.
- When HTTP options is visible, `h` closes that popup and must not move focus.

Risks found:

- `App::handle_left()` and `App::handle_right()` check `is_at_left_edge()` / `is_at_right_edge()` before calling tab handlers. Some tab implementations return `true` for non-input regions or incomplete edge state, which can make `h/l` feel dead.
- `handle_left_or_prev_tab()` and `handle_right_or_next_tab()` exist in `app/mod.rs` but are not wired to the main `h/l` path. Confirm whether they are unused; remove them if dead or use them deliberately.
- Several tabs implement `handle_left` / `handle_right` ad hoc. Some use left/right to move between controls, some move within inputs, some return `true` without meaningful movement.
- Recon has `is_at_right_edge()` comparing byte cursor to `field.value.chars().count()`, which is wrong for non-ASCII input and can block or allow `l` incorrectly.
- Fuzz implements left/right across controls but does not implement `is_at_left_edge()` / `is_at_right_edge()`, so defaults may not reflect actual focus movement.

### Required Contract

Document this contract near `TabInput` or in a short helper comment:

- `h` / Left in Normal mode means "move left within the current focused region."
- `l` / Right in Normal mode means "move right within the current focused region."
- `h/l` must not switch tabs. Tab switching remains `n`, `N`, `p`, `Shift+H`, and `Shift+L`.
- In Insert mode, `h/l` are text input and must not navigate.
- If a selector/dropdown is focused, `h/l` should change selector value only if the selector supports horizontal movement. If unsupported, they should return `false` without changing unrelated focus.
- If an input is focused, `h/l` should move the input cursor. Edge checks must use byte-position invariants correctly.
- If a checkbox group is focused, `h/l` should move among checkboxes only when the checkboxes are visually arranged horizontally. For vertical checkbox lists, prefer `j/k` or Up/Down and let `h/l` return `false`.
- If Results is focused, `h/l` should either do nothing or move horizontal result focus if implemented. Scrolling remains `j/k`, Up/Down, PageUp/PageDown, `Ctrl+U`, and `Ctrl+D`.
- Overlay-active states always win over tab content. For example, `h` closes HTTP options; it does not navigate the active tab.

### Required Fix

- Audit all `impl TabInput for ...` implementations under `crates/slapper/src/tui/tabs/`.
- For each tab, verify `handle_left`, `handle_right`, `is_at_left_edge`, and `is_at_right_edge` match the contract.
- Fix byte-vs-char edge checks. Any input cursor edge check should compare against `field.value.len()` if `cursor_pos` is the byte index.
- For tabs with selector components, inspect `crates/slapper/src/tui/components/selector.rs`; `Selector::handle_left()` and `handle_right()` are currently empty around `selector.rs:238`. Either implement meaningful selector left/right behavior or stop calling these no-op methods from tabs.
- Decide whether `App::handle_left()` should call tab `handle_left()` even when `is_at_left_edge()` says true. Current pre-check can hide useful tab-specific behavior if edge predicates are inaccurate. A safer pattern may be: call `handle_left()` and let the tab return whether it moved.
- If keeping the pre-check, add tests proving every representative tab reports edge state correctly.
- Remove or wire `handle_left_or_prev_tab()` and `handle_right_or_next_tab()` intentionally. Do not leave misleading unused helpers with names suggesting tab switching.

### Representative Tabs To Normalize First

- `crates/slapper/src/tui/tabs/recon.rs`
- `crates/slapper/src/tui/tabs/fuzz.rs`
- `crates/slapper/src/tui/tabs/load.rs`
- `crates/slapper/src/tui/tabs/proxy.rs`
- `crates/slapper/src/tui/tabs/packet.rs`
- `crates/slapper/src/tui/tabs/settings/input.rs`
- `crates/slapper/src/tui/tabs/history.rs`

After those are correct, apply the same pattern to feature-gated tabs:

- `browser.rs`
- `compliance.rs`
- `integrations.rs`
- `workflow.rs`
- `vuln.rs`
- `nse.rs`
- `plugin.rs`
- `storage.rs`
- `hunt.rs`

### Acceptance Criteria

- In Normal mode, `h/l` move within focused controls consistently and never switch tabs.
- In Insert mode, typing `h` or `l` inserts text into the focused field.
- In HTTP options overlay, `h` closes only the overlay.
- On an input containing non-ASCII text, repeated `l` reaches the true end and repeated `h` reaches the beginning without getting stuck or splitting a character.
- On selector-focused tabs, `h/l` either visibly change the selector value or do nothing predictably; they must not silently move unrelated focus.
- On checkbox groups, `h/l` behavior matches the visual layout.

### Tests

- Add tests for App-level mode behavior: Normal `h/l` dispatches movement; Insert `h/l` inserts characters.
- Add Recon regression test for non-ASCII cursor edge behavior.
- Add Fuzz tests for left/right focus movement across `PayloadSelector`, `ModeSelector`, `TargetSelector`, and `MutationCheckbox`.
- Add Load/Proxy/Packet selector tests if selector left/right is implemented.
- Add a test that HTTP options visible plus `h` closes the overlay without changing the current tab or focus region.

## Workstream 3: Align Help And Status Text With Real Navigation

### Problem

Visible hints are too long and some are ambiguous.

Current areas:

- `crates/slapper/src/tui/ui.rs` status bar around `draw_status_bar`.
- `crates/slapper/src/tui/components/popup.rs` help popup.
- Per-tab empty states and quick-action text.

Known issues:

- The status bar help string is much wider than the help chunk at 80 columns. At roughly `ui.rs:628`, the chunks reserve fixed mode width plus percentage status/help widths; the help string can be truncated to around 30 columns.
- The help text should make clear that `h/l` are in-pane navigation, while tabs use `n/p` and `Shift+H/L`.
- Paused state should show the correct resume key. `Ctrl+Z` toggles pause; `Ctrl+Y` resumes if paused.
- Help text should not advertise commands that are blocked by overlays or current mode.

### Required Fix

- Create a concise keybinding summary helper based on mode and overlay state. This can live in `ui.rs` or a small helper module.
- Prefer short, accurate status hints over exhaustive shortcut lists.
- At 80 columns, show only the most important actions:
  - Normal: `[n/p] Tabs [h/j/k/l] Move [/] Search [Space] Help [q] Quit`
  - Insert: `[Esc] Normal [Ctrl+V] Paste`
  - Search: `[Enter] Search [Backspace] Edit [Esc] Close`
  - Palette: `[Enter] Run [Up/Down] Select [Esc] Close`
  - Confirm: `[Enter] Confirm [Esc] Cancel`
- Keep detailed keybindings in the help popup, but audit every line against `runner.rs`.
- Ensure help popup explicitly says `h/l` are left/right within the current pane, not previous/next tab.

### Acceptance Criteria

- Every visible keybinding either works or is removed.
- `h/l` help copy matches the navigation contract from Workstream 2.
- Status bar is readable at 80x24 and does not depend on a huge third column.
- Overlay-specific hints appear when overlays are active.

### Tests

- Unit test any keybinding summary helper for Normal, Insert, Search, Palette, Confirm, and Help contexts.
- Render tests using `TestBackend` for 80x24 should assert no panic and optionally inspect buffer text for the compact hints.

## Workstream 4: Improve Small-Terminal Layout Robustness

### Problem

Some tabs still reserve too much fixed height or width for common terminal sizes.

Examples:

- `crates/slapper/src/tui/tabs/load.rs` reserves `6 + 15` rows before results, leaving almost no results area after global chrome on a 24-row terminal.
- `crates/slapper/src/tui/tabs/settings/render.rs` reserves a fixed 20-column sidebar, which is heavy on narrow terminals.
- Similar fixed-height input regions remain in tabs such as `scan_ports.rs`, `scan_endpoints.rs`, `waf.rs`, `packet.rs`, `graphql.rs`, `report.rs`, and `cluster.rs`.
- The runner only warns below 80x24; it does not prevent broken layouts.

### Required Fix

- Add dynamic layout helpers for common patterns:
  - Input section plus results section.
  - Selector plus input block plus results section.
  - Sidebar plus content.
- For height-constrained terminals, preserve at least one usable input/control area and one meaningful status/results area.
- For very small screens, show a clear compact fallback or "terminal too small" message instead of rendering cramped controls.
- Avoid copying Fuzz's exact dynamic layout everywhere if a helper can capture the pattern cleanly.

### Acceptance Criteria

- At 80x24, core tabs render a visible input/control region and visible results/status region.
- At 60x20 and 40x20, core tabs do not panic and do not render obviously incoherent layouts.
- Settings remains navigable on narrow terminals.
- Load, Recon, Fuzz, History, Dashboard, Settings, Proxy, and Packet have explicit render tests.

### Tests

- Add or extend `ratatui::backend::TestBackend` render tests for:
  - 80x24
  - 60x20
  - 40x20
  - 120x24
- Include late/scrolled tab positions so the tab strip remains covered.

## Workstream 5: Theme And Visual Consistency Pass

### Problem

The TUI has semantic theme colors, but visual weight and contrast are inconsistent.

Examples:

- Light theme uses `LightGreen` surface and `LightBlue` borders/inactive tabs, which can look noisy and low-contrast.
- `Theme::style_for_mode()` hardcodes `Color::Black` foreground for both modes.
- The status bar uses `tc!(background)` as text over mode colors; verify contrast in both themes.
- Empty states vary in tone and density.

### Required Fix

- Review `crates/slapper/src/tui/theme.rs` and adjust light/dark theme colors conservatively.
- Replace hardcoded foregrounds in reusable theme methods with semantic colors where possible.
- Keep the UI information-dense and work-focused. Avoid decorative styling.
- Standardize empty-state structure:
  - Short title.
  - One concise action sentence.
  - Optional CLI equivalent.
- Run a grep for direct `Color::*` in TUI rendering. Direct colors are acceptable in theme definitions; elsewhere prefer `tc!`.

### Acceptance Criteria

- Normal and Insert mode labels have readable contrast in dark and light themes.
- Inactive tabs are visible but clearly secondary.
- Empty states are consistent across representative tabs.
- Direct `Color::White`, `Color::Gray`, `Color::Green`, and `Color::Red` are not used in rendering outside theme definitions unless there is a documented reason.

### Tests

- Existing TUI tests should pass.
- Add small unit tests only if helper behavior changes. Do not add brittle color snapshot tests unless the project already uses them.

## Workstream 6: Command Palette And Overlay Polish

### Problem

Overlay infrastructure exists, but behavior needs usability polish.

Areas to verify:

- Command palette should not allow tab-content actions while open.
- Search results should be navigable if a selected state is shown.
- Confirm popups render button-like UI, but actual behavior is Enter/Esc only.
- HTTP options title says `press h to close`; that should remain true after the event routing changes.

### Required Fix

- Audit `runner.rs` overlay handling after Workstream 1 so every overlay has a clear key path.
- Make confirm popup labels match behavior. If buttons are not selectable, copy should say `Enter: Confirm` and `Esc: Cancel`.
- If search results show selection, wire Up/Down and Enter to navigate or remove selected-state visuals.
- Make mouse scroll behavior explicit when overlays are open. Current code ignores command palette scroll, and page-scrolls content only when overlays are not active. Keep or revise intentionally.

### Acceptance Criteria

- Overlay key handling is centralized or at least visibly ordered by `topmost_overlay()`.
- No tab task starts or focus moves while an overlay is active.
- Popup copy accurately describes available actions.
- Search, palette, HTTP options, help, and confirm overlays are mutually sane when opened and closed in sequence.

### Tests

- Add app-level tests for overlay precedence and key action mapping.
- Add search overlay key tests if a pure helper is extracted.
- Add confirm popup test: Enter confirms, Esc cancels, `h/l` do not leak to tab content while confirm is visible.

## Workstream 7: Feature-Gated And Cached Tab Rendering Audit

### Problem

Tab availability is mostly handled correctly, but there is one suspicious rendering pattern:

- `draw_tabs()` in `crates/slapper/src/tui/ui.rs` uses a static `LazyLock<Vec<Line>>` built from `Tab::all()`.
- `Tab::all()` itself is feature-set dependent but stable per binary. This is likely safe in normal builds, but the static cache can hide assumptions in tests or future dynamic availability changes.

### Required Fix

- Decide whether static tab title caching is worth keeping. Simpler and safer: build visible titles directly from `Tab::all()[window.start..window.end]` each render.
- Ensure the visible title list and `TabWindow` always come from the same `Tab::all()` view.
- Keep mouse hit testing aligned with rendered titles.

### Acceptance Criteria

- Tab strip rendering uses the same tab list as navigation/hit-testing.
- Feature-gated builds do not show unavailable tabs.
- Existing tab window and mouse hit tests still pass.

### Tests

- Existing tab window tests should continue passing.
- If possible, add a test around `draw_tabs` helper or `TabWindow` to assert visible titles correspond to visible tabs.

## Suggested Execution Order

1. Workstream 1: search event routing and scope. This is a concrete bug and affects visible keybindings.
2. Workstream 2: `h/l` navigation contract and representative tab fixes.
3. Workstream 3: help/status text alignment with the corrected navigation model.
4. Workstream 6: overlay polish after search routing is fixed.
5. Workstream 4: small-terminal layout robustness.
6. Workstream 5: theme and visual consistency.
7. Workstream 7: feature-gated/cached tab rendering audit.

## Verification Checklist

Run after each meaningful batch:

```bash
cargo test --lib -p slapper tui::
cargo check --lib -p slapper
```

If feature-gated files are touched, also run targeted checks:

```bash
cargo check --lib -p slapper --features nse
cargo check --lib -p slapper --features python-plugins
cargo check --lib -p slapper --features full
```

Manual smoke test before handoff:

- Launch the TUI at 80x24.
- Verify `n`, `N`, `p`, `Shift+H`, and `Shift+L` switch tabs.
- Verify `h/l` move within the current focused control and do not switch tabs.
- Verify Insert mode typing `h` and `l` inserts characters.
- Verify `/` search and `Ctrl+F` global search both accept input, support Backspace, run on Enter, and close on Esc.
- Verify command palette opens/closes and does not leak keys to tab content.
- Verify HTTP options closes with `h` and Esc.
- Verify confirm popup Enter/Esc behavior.
- Verify Help popup copy matches actual keys.
- Verify Load, Recon, Fuzz, Settings, History, Dashboard, Proxy, and Packet at 80x24 and 60x20.
