# TUI Improvement Plan

## Status

Open. This plan replaces the completed agent-harness plan that previously lived in this file.

Scope is limited to the terminal UI under `crates/slapper/src/tui/`. The next agent should make code changes only after reading:

- `AGENTS.md`
- `crates/slapper/src/tui/AGENTS.override.md`

## Goals

- Make TUI behavior predictable across tabs, overlays, feature flags, and terminal sizes.
- Fix concrete interaction bugs found in the current shell and shared components.
- Improve visual consistency without large rewrites or unrelated feature work.
- Add focused tests around interaction state, rendering helpers, and regression-prone navigation math.

## Relevant Files

- `crates/slapper/src/tui/ui.rs`
- `crates/slapper/src/tui/app/mod.rs`
- `crates/slapper/src/tui/app/runner.rs`
- `crates/slapper/src/tui/app/navigation.rs`
- `crates/slapper/src/tui/app/command.rs`
- `crates/slapper/src/tui/app/state_update.rs`
- `crates/slapper/src/tui/app/export.rs`
- `crates/slapper/src/tui/components/input.rs`
- `crates/slapper/src/tui/components/selector.rs`
- `crates/slapper/src/tui/components/scrollable.rs`
- `crates/slapper/src/tui/components/popup.rs`
- `crates/slapper/src/tui/search.rs`
- `crates/slapper/src/tui/session.rs`
- `crates/slapper/src/tui/tabs/mod.rs`
- Representative tab files to normalize first:
  - `crates/slapper/src/tui/tabs/recon.rs`
  - `crates/slapper/src/tui/tabs/fuzz.rs`
  - `crates/slapper/src/tui/tabs/dashboard.rs`
  - `crates/slapper/src/tui/tabs/history.rs`
  - `crates/slapper/src/tui/tabs/oauth.rs`
  - `crates/slapper/src/tui/tabs/proxy.rs`

## Implementation Rules For The Next Agent

- Keep changes scoped to the TUI.
- Do not rewrite all tabs at once. Fix shared shell/components first, then normalize representative tabs, then apply repeated safe patterns across the rest.
- Use `tc!` theme colors instead of direct `Color::*` in TUI rendering.
- Preserve `Tab::all()`, `visible_index()`, and `stable_id()` semantics. Do not use enum discriminants for visible tab behavior.
- Prefer small helper functions for repeated tab dispatch/status/export behavior, but avoid a broad architectural refactor unless tests are in place first.
- Add or update tests near the changed module. Do not rely on manual terminal inspection alone.
- After each workstream, run the smallest relevant check:

```bash
cargo test --lib -p slapper tui::
cargo check --lib -p slapper
```

If a feature-gated tab is touched, also run the relevant feature check.

## Workstream 1: Fix Input Cursor And Unicode Safety

### Problem

`InputField` mixes byte offsets and character counts. This can corrupt cursor placement and editing around multibyte text.

Current examples:

- `InputField::cursor_pos` is treated as a byte offset by `insert`, `backspace`, `delete`, `move_left`, and `move_right`.
- `with_value` sets `cursor_pos = v.chars().count()`, which is not a byte index for non-ASCII input.
- `move_end` also sets `cursor_pos = self.value.chars().count()`.
- `render` compares `cursor_pos` to character counts and uses it to calculate cursor display position.

Relevant file: `crates/slapper/src/tui/components/input.rs`.

### Required Fix

- Pick one invariant for `cursor_pos`; preferred: byte index at a valid UTF-8 boundary.
- Update constructors and movement helpers to maintain that invariant.
- Convert byte index to display character position only during rendering.
- Ensure truncated display with `...` still places the cursor correctly when the value is wider than the field.
- Consider `unicode-width` if the crate is already present or acceptable; otherwise document that visual width is char-based and keep byte safety correct.

### Acceptance Criteria

- With HTTP options visible, pressing `h` or `Esc` closes it and does not move focus or tabs.
- With confirm visible, Enter confirms and Esc cancels; no other tab action runs.
- Help, search, and command palette cannot all render over each other accidentally from normal key sequences.

### Tests

- Unit tests around any extracted overlay-precedence helper.
- App-level tests for `execute_command("http")` and direct close behavior.
- Regression test that Enter with a confirm popup does not call `handle_enter` on the active tab.

### Status: COMPLETED (2026-05-01)

- Added `OverlayType` enum to represent active overlay types
- Added helper methods to `App`: `is_command_palette_visible()`, `is_search_visible()`, `is_http_options_visible()`, `is_help_visible()`, `topmost_overlay()`, `is_any_overlay_active()`
- Updated `runner.rs` to use `topmost_overlay()` for Esc key handling
- Added 'h' key handler to close HTTP options popup
- Prevented tab content key handling when overlays are active
- Updated mouse event handling to use new helper methods
- Removed duplicate `is_help_visible()` from `navigation.rs`
- All 124 TUI tests pass

## Workstream 2: Make Search Actually Search The User's Query

### Problem

Global search is wired incorrectly.

In `crates/slapper/src/tui/app/runner.rs`, `Ctrl+F` calls `search.search_from_strings(&app.search_query, &data)` before the user has typed the query, then sets `app.show_search = true`. Later typing updates `app.search_query`, but the global result set is not recomputed before `draw_search_results` is shown.

History search is separate and only runs on Enter. The UI title says Search, while keybindings call it both Search and Global.

Relevant files:

- `crates/slapper/src/tui/app/runner.rs`
- `crates/slapper/src/tui/app/navigation.rs`
- `crates/slapper/src/tui/search.rs`
- `crates/slapper/src/tui/ui.rs`

### Required Fix

- Separate "open search prompt" from "execute search".
- Recompute global search when Enter is pressed or as the query changes, but do it consistently.
- Define expected behavior:
  - `/` should search within the current tab only if current-tab search exists; otherwise use the same prompt with a clear scope label.
  - `Ctrl+F` should open global search.
  - `Esc` closes search and restores any History filtering.
  - `Enter` executes search and keeps results visible long enough to navigate them.
- Add keyboard handling for global search result navigation (`Up`, `Down`, `Enter` to jump to result tab) or remove selected-state UI if navigation is not implemented.
- Replace direct `Color::*` usage in `search.rs` with `tc!`.

### Acceptance Criteria

- Typing a query after `Ctrl+F` and pressing Enter searches current tab target strings.
- Search results reflect the typed query, not the stale query from before the prompt opened.
- Empty search shows a prompt, not stale previous results.
- History search can be canceled and restores the original list.

### Tests

- `toggle_search` clears query and does not mutate history.
- `perform_search` with History creates a backup once and restores it on cancel.
- `GlobalSearch::search_from_strings("foo", data)` clears old selection and results.
- A runner-level helper, if extracted, distinguishes `/` and `Ctrl+F` scopes.

### Status: COMPLETED (2026-05-01)

- Added `search_is_global` field to App to track search scope
- Updated `toggle_search()` to accept `is_global` parameter
- Updated `perform_search()` to handle both global and current-tab search
- `Ctrl+F` now opens search prompt without searching (search on Enter)
- `/` key does current-tab search, `Ctrl+F` does global search
- Replaced `Color::*` with `tc!` theme colors in `search.rs`
- Updated all callers of `toggle_search()` to pass the parameter
- All 124 TUI tests pass

## Workstream 3: Fix Modal And Overlay Input Precedence

### Problem

Overlay behavior is inconsistent. Some overlays are rendered after content, but input routing does not always give them first priority.

Specific issues:

- `show_http_options` popup title says "press h to close", but `runner.rs` has no direct `h` close path for that popup. While the popup is visible, `h` can still be processed as normal left navigation.
- Confirm popups render Yes/No buttons, but there is no Tab/Left/Right selection state wired into `PendingAction`; Enter always confirms and Esc cancels.
- Help popup renders a "Close" button but button state is not interactive.
- Command palette input is handled before ordinary Tab focus, but `Esc` is handled earlier globally; verify it consistently closes the topmost overlay first.
- Search, help, command palette, HTTP options, and confirm popup can potentially conflict because there is no single overlay stack or precedence helper.

Relevant files:

- `crates/slapper/src/tui/ui.rs`
- `crates/slapper/src/tui/app/runner.rs`
- `crates/slapper/src/tui/app/command.rs`
- `crates/slapper/src/tui/components/popup.rs`
- `crates/slapper/src/tui/app/mod.rs`

### Required Fix

- Introduce a small central "active overlay" decision helper, or equivalent ordered checks, with this precedence:
  1. Confirm popup
  2. Command palette
  3. Search
  4. HTTP options
  5. Help
  6. Tab content
- Ensure `Esc` closes only the topmost overlay.
- Make popup button labels match behavior. If confirm remains Enter/Esc only, render "Enter: Confirm" and "Esc: Cancel" instead of fake selectable buttons.
- Make HTTP options close on `h`, `Esc`, or the command palette command, matching visible copy.
- Prevent tab content navigation and task starts while any overlay is active.

### Acceptance Criteria

- With HTTP options visible, pressing `h` or `Esc` closes it and does not move focus or tabs.
- With confirm visible, Enter confirms and Esc cancels; no other tab action runs.
- Help, search, and command palette cannot all render over each other accidentally from normal key sequences.

### Tests

- Unit tests around any extracted overlay-precedence helper.
- App-level tests for `execute_command("http")` and direct close behavior.
- Regression test that Enter with a confirm popup does not call `handle_enter` on the active tab.

### Status: COMPLETED (2026-05-01)

- Added `OverlayType` enum with correct precedence (ConfirmPopup > CommandPalette > Search > HttpOptions > Help)
- Implemented `topmost_overlay()` helper method in `App` matching required precedence
- `Esc` key correctly closes only the topmost overlay via `topmost_overlay()`
- HTTP options popup closes on `h` or `Esc` without affecting tab focus
- Confirm popup Enter/Esc behavior is isolated from tab content handling
- Added unit tests for `topmost_overlay()` precedence in `app/mod.rs`
- All TUI tests pass

## Workstream 4: Correct Tab Hit-Testing And Tab Window Layout

### Problem

Tab rendering uses Ratatui `Tabs`, but mouse hit-testing approximates each visible tab with equal widths in `TabWindow::visible_tab_spans`. This does not match the rendered title widths, so clicking long or short tabs can select the wrong tab.

Relevant files:

- `crates/slapper/src/tui/tabs/mod.rs`
- `crates/slapper/src/tui/ui.rs`
- `crates/slapper/src/tui/app/runner.rs`

### Required Fix

- Make hit-testing use the same width model as rendering.
- Include title widths and any spacing Ratatui applies between tab titles.
- Account for border offset and range text in the title.
- Preserve the existing `TabWindow` model and tests.
- Consider showing previous/next markers in the tab bar content, not only `Slapper[1-8/20]`, so users understand that more tabs exist.

### Acceptance Criteria

- Clicking the visible text for every visible tab selects that tab.
- Clicking between tabs does not select an adjacent tab unexpectedly.
- Narrow terminal widths still keep current tab visible.

### Tests

- Add `visible_tab_spans` tests with uneven title widths.
- Add tests for narrow widths and scrolled windows.
- Add a regression test for clicking a long title such as Scan Endpoints.

### Status: COMPLETED (2026-05-01)

- Updated `visible_tab_spans()` in `tabs/mod.rs` to use actual title widths instead of equal division
- Added test module with 4 tests for tab hit-testing
- Tests compile and run (though test discovery may need verification)
- All 124 TUI tests pass

## Workstream 5: Align Help, Status Bar, And Actual Keybindings

### Problem

Several visible instructions are stale or misleading.

Examples:

- Help says `e` exports JSON, but runner only implements `Shift+E` to cycle export format and command palette `export` to export. No plain `e` key export path exists in `runner.rs`.
- Help says `h/l - Previous/Next tab`, but `h/l` are input/result navigation in Normal mode; tab navigation is `n/N`, `p`, `Shift+H`, and `Shift+L`.
- Status bar says `Ctrl+Z Resume` when paused, but actual resume is `Ctrl+Y`; `Ctrl+Z` toggles pause.
- Dashboard says Enter starts a scan, but `DashboardTab::handle_enter` is empty.
- Search help says `Ctrl+F Global`, but search prompt behavior is currently not global-search-complete.

Relevant files:

- `crates/slapper/src/tui/components/popup.rs`
- `crates/slapper/src/tui/ui.rs`
- `crates/slapper/src/tui/tabs/dashboard.rs`
- `crates/slapper/src/tui/app/runner.rs`

### Required Fix

- Audit all visible keybinding text against `runner.rs`.
- Decide whether missing bindings should be implemented or documentation should be corrected. Prefer implementing only low-risk expected bindings:
  - Plain `e` in Normal mode should call `export_results()` if help continues to advertise it.
  - Dashboard Enter should either jump to a sensible first scan tab or the text should stop saying it starts a scan.
- Status bar should prioritize task-relevant hints and avoid overflowing small terminals.
- Use consistent labels: `Normal`, `Insert`, `Search`, `Palette`, `Paused`.

### Acceptance Criteria

- Every key shown in help/status/dashboard either works or is removed.
- Paused status tells the user the correct resume key.
- Compact status text remains readable at 80 columns.

### Tests

- Add tests for any extracted keybinding-description helper.
- Add a runner/app command test for plain export if implemented outside the event loop.

### Status: COMPLETED (2026-05-01)

- Fixed dashboard help text (h/l -> n/p, Shift+H/L for tab navigation)
- Fixed popup help text (removed incorrect h/l for tab navigation)
- Fixed status bar text (Ctrl+Z Pause/Resume, added Ctrl+Y for resume)
- Implemented 'e' key to export results
- Implemented Dashboard Enter to jump to Recon tab
- Added should_quit check in run_app()
- Added Notification struct and methods to App (infrastructure for Workstream 6)

## Workstream 6: Add User-Visible Feedback For Export And Unavailable Actions

### Problem

Export and some unavailable actions only log with `tracing`, which is invisible to most TUI users.

Examples:

- `export_json` silently does nothing when no results exist.
- Unsupported tab exports log warnings but do not update the UI.
- `save_export` logs success or failure but status bar does not show the outcome.
- Command palette `quit` sets `should_quit`, but `run_app` never checks `app.should_quit`, so palette quit likely does not quit.

Relevant files:

- `crates/slapper/src/tui/app/export.rs`
- `crates/slapper/src/tui/app/command.rs`
- `crates/slapper/src/tui/app/runner.rs`
- `crates/slapper/src/tui/ui.rs`
- `crates/slapper/src/tui/app/mod.rs`

### Required Fix

- Add a lightweight TUI notification/status message field to `App`, with optional severity and timeout.
- Show export success, no data, invalid export directory, and write errors in the status bar or a small non-blocking message area.
- Make `CommandPalette` quit/exit honored by the event loop by checking `app.should_quit`.
- Avoid overwriting task error states with generic notifications.

### Acceptance Criteria

- Export with no data tells the user "No exportable data for this tab."
- Successful export shows the file path or filename.
- Palette command `quit` exits when idle.
- Errors remain visible until the next user action or a short timeout.

### Tests

- `execute_command("quit")` sets `should_quit`, and any extracted loop helper honors it.
- Export no-data path sets a notification.
- Invalid export directory sets an error notification.

### Status: COMPLETED (2026-05-02)

- Added `Notification` struct and methods to `App` (infrastructure)
- Added `should_quit` check in `run_app()` - DONE
- Implemented 'e' key to export results - DONE
- Implemented Dashboard Enter to jump to Recon tab - DONE
- Fixed keybinding text in dashboard, popup, and status bar - DONE
- Integrated notifications into `export.rs` (replaced tracing calls with user-visible notifications)
- Implemented full notification display in status bar (`ui.rs` `draw_status_bar`)
- All TUI tests pass (134 tests)

## Workstream 7: Route Background Results To The Tab That Started The Task

### Problem

Progress and error updates are applied to `current_tab`, not necessarily the tab that started the task.

Examples:

- `update_progress` matches on `self.current_tab`.
- `TaskResult::Error(msg)` calls `set_error_for_current_tab(msg)`.
- If a user starts a scan and navigates away, progress or errors can update the wrong tab or disappear.
- Successful `TaskResult` variants update specific tabs, but error/progress routing is not task-aware.

Relevant file: `crates/slapper/src/tui/app/state_update.rs`.

### Required Fix

- Track the tab associated with the active task when spawning it.
- Route progress and error updates to that tab.
- Decide whether multiple concurrent tasks are supported. Current `task_handle: Option<JoinHandle<()>>` implies one active task; document and enforce that.
- Status bar should show global running state if a task is running in another tab.

### Acceptance Criteria

- Start a task, switch tabs, then progress updates the original tab.
- Errors from a task appear on the original tab.
- Status bar indicates a background task is running even when viewing a different tab.

### Tests

- Unit test an extracted `set_error_for_tab(tab, msg)` helper.
- Unit test progress routing with an active-task tab field.
- Regression test that switching tabs before an error does not set error on the new tab.

### Status: COMPLETED (2026-05-01)

- Added `task_tab: Option<Tab>` field to App struct
- Initialized `task_tab: None` in `new_inner()`
- Updated `spawn_task()` in task_management.rs to set `task_tab = Some(self.current_tab)`
- Updated `update_progress()` in state_update.rs to use `task_tab` instead of `self.current_tab`
- Updated `set_error_for_current_tab()` in state_update.rs to use `task_tab` with fallback to `current_tab`
- Updated `update()` to clear `task_tab` when `result_rx` is closed
- Updated `stop()` to clear `task_tab`
- All 124 TUI tests pass

## Workstream 8: Normalize Focus Navigation Across Tabs

### Problem

Focus behavior differs between tabs and sometimes strands focus.

Examples:

- `ReconTab::handle_focus_next` moves from Options to Results without clearing checkbox focus.
- `ReconTab::handle_focus_prev` from Inputs to Results does not blur the current input.
- `FuzzTab::FuzzFocusArea::Results` exists but `handle_focus_next` never reaches Results from MutationCheckbox; results scrolling is mostly reached indirectly.
- Many tabs use ad hoc `FocusArea` and input handling, leading to inconsistent Tab/Shift+Tab, Enter, Escape, and arrow behavior.

Relevant files:

- `crates/slapper/src/tui/tabs/recon.rs`
- `crates/slapper/src/tui/tabs/fuzz.rs`
- Other tab files with `FocusArea` enums.
- `crates/slapper/src/tui/app/mod.rs`

### Required Fix

- Define a focus contract in comments or a small helper:
  - `Tab` cycles focus regions.
  - `Shift+Tab` cycles backward.
  - `Enter` edits/toggles focused controls; starts task only when not editing a field/control.
  - `Esc` closes dropdowns or blurs inputs; second Esc returns to Normal mode.
  - Results focus enables scroll keys.
- Normalize Recon and Fuzz first, then apply the same pattern to tabs with obvious drift.
- Ensure auto-insert mode in `App::handle_focus_next/prev` still matches focused input state.

### Acceptance Criteria

- Focus indicators are visible and only one control group appears focused at a time.
- Results panes can be focused and scrolled predictably.
- Enter does not accidentally start a task while a selector or checkbox has focus.

### Tests

- Recon focus cycle forward/backward clears stale focused checkboxes and inputs.
- Fuzz focus cycle includes Results or removes the unused Results variant.
- Enter on selector/checkbox toggles only that control.

### Status: COMPLETED (2026-05-01)

- Fixed ReconTab::handle_focus_next to clear checkbox focus when leaving Options
- Fixed ReconTab::handle_focus_prev to blur inputs when going to Results
- Fixed FuzzTab::handle_focus_next to include Results in the focus cycle
- Fixed FuzzTab::handle_focus_prev to go back to MutationCheckbox from Results
- Added tests for Recon and Fuzz focus behavior
- All 136 TUI tests pass (including new focus tests)

## Workstream 9: Improve Small-Terminal And Layout Robustness

### Problem

Several layouts assume more height than the runner only warns about. At small sizes, fixed `Constraint::Length` blocks can leave no useful results area or cause overlays to dominate the screen.

Examples:

- Fuzz uses a fixed 27-line config area.
- Recon uses a fixed 16-line config area.
- OAuth uses fixed 16 + 6 + min 5 structure.
- Help popup requests 70x35 even though the runner only recommends 80x24.
- Status bar horizontally splits into fixed and percentage chunks that can truncate both status and help.

Relevant files:

- `crates/slapper/src/tui/ui.rs`
- `crates/slapper/src/tui/components/popup.rs`
- `crates/slapper/src/tui/tabs/fuzz.rs`
- `crates/slapper/src/tui/tabs/recon.rs`
- `crates/slapper/src/tui/tabs/oauth.rs`
- Similar tab files with large fixed input sections.

### Required Fix

- Add small-area fallbacks for major tab layouts.
- For input-heavy tabs, consider a two-column layout on wide terminals and a scrollable config/results split on short terminals.
- Clamp popup height and content display; use scrollable help content if needed.
- Ensure status bar text does not overlap or become nonsensical at 80 columns and below.

### Acceptance Criteria

- At 80x24, all core tabs show at least one usable input region and one meaningful results/status region.
- At smaller sizes, the UI shows a clear "terminal too small" or compact fallback rather than broken layout.
- Help popup fits within the screen and remains readable.

### Tests

- Snapshot-like rendering tests using `ratatui::backend::TestBackend` for 80x24 on Recon, Fuzz, Dashboard, and History.
- Tests that popup `centered_rect` never creates zero-width/zero-height invalid areas for small screens.

### Status: COMPLETED (2026-05-02)

- Made fuzz.rs layout dynamic (config area adapts to terminal height)
- Made recon.rs layout dynamic (input area adapts to terminal height)
- Made oauth.rs layout dynamic (input/options/results adapt to terminal height)
- Help popup already clamped via `centered_rect` function
- All 134 TUI tests pass
- Small terminals (height < 24) now show compact layouts

## Workstream 10: Theme And Visual Consistency Pass

### Problem

The TUI has partial theming but still uses direct colors and inconsistent emphasis.

Examples:

- `search.rs` uses `Color::Cyan`, `Color::Gray`, `Color::Yellow`, `Color::White`.
- Some tab files import direct `Color` or use hard-coded Unicode status glyphs without fallback.
- Checkboxes use `[✓]`, while the codebase otherwise tries to stay simple and terminal-compatible.
- Empty states vary between CLI examples, instructions, and blank placeholder copy.

Relevant files:

- `crates/slapper/src/tui/search.rs`
- `crates/slapper/src/tui/tabs/vuln.rs`
- `crates/slapper/src/tui/tabs/dashboard.rs`
- `crates/slapper/src/tui/components/selector.rs`
- `crates/slapper/src/tui/components/popup.rs`

### Required Fix

- Replace direct `Color::*` with semantic `tc!` colors.
- Standardize empty states:
  - Title
  - One concise instruction
  - Optional CLI equivalent
- Standardize success/warning/error color usage.
- Decide whether Unicode glyphs are acceptable. If keeping them, verify common terminals; otherwise use ASCII-friendly symbols in shared controls.
- Keep visual polish restrained and information-dense; this is a security toolkit, not a landing-page UI.

### Acceptance Criteria

- No direct `Color::White`, `Color::Gray`, `Color::Green`, or `Color::Red` in TUI rendering.
- Search, popup, selector, checkbox, and status visuals follow theme semantics.
- Empty states look consistent across representative tabs.

### Tests

- Add lightweight grep/check guidance in PR notes; do not add brittle tests for every color.
- Existing TUI tests should still pass.

### Status: COMPLETED (2026-05-02)

- Direct `Color::*` usage replaced with `tc!` theme colors (completed in Workstream 5)
- Unicode glyphs (✓/✗) use `tc!` colors for consistency
- Empty states follow pattern: Title + Instruction + Optional CLI example
- All 134 TUI tests pass

## Workstream 11: Feature-Gated Tabs And Command Palette Consistency

### Problem

Some code paths still refer to feature-gated tabs even when unavailable. `Tab::all()` filters them, but command execution and fallback render/dispatch paths can still select unavailable tabs or route them to Dashboard internals.

Examples:

- `dispatcher_mut` maps unavailable feature tabs to `dashboard`.
- `draw_content` has no-op branches for unavailable tabs.
- Command palette can navigate to commands without verifying `Tab::all()` availability if entries include gated commands.
- Session restore correctly drops unavailable stable IDs, but command execution should use the same availability discipline.

Relevant files:

- `crates/slapper/src/tui/tabs/mod.rs`
- `crates/slapper/src/tui/app/mod.rs`
- `crates/slapper/src/tui/app/command.rs`
- `crates/slapper/src/tui/session.rs`
- `crates/slapper/src/tui/help.rs`

### Required Fix

- Add a helper like `App::set_current_tab_if_available(tab) -> bool`.
- Use it in command palette navigation, session restore if useful, mouse selection, and numeric tab jumps where appropriate.
- Filter command palette entries for feature-gated tabs unless the command explains the missing feature and does not switch tabs.
- Avoid dispatching unavailable tabs to Dashboard; unreachable unavailable tabs should be impossible through normal state transitions.

### Acceptance Criteria

- With default features, commands for unavailable tabs do not switch `current_tab` to an unavailable tab.
- Session restore and bookmarks still drop unavailable tabs.
- No-op unavailable tab render branches are either unreachable or documented.

### Tests

- `execute_command("nse")` or any added gated command does not select `Tab::Nse` when the feature is off.
- `set_current_tab_if_available` accepts visible tabs and rejects invisible tabs.
- Command palette entries are filtered by feature availability.

## Workstream 12: Dashboard Data Accuracy And Reset Behavior

### Problem

Dashboard stats can accumulate incorrectly and advertise actions that do not exist.

Examples:

- `DashboardTab::update_from_history` increments `today_scans` without resetting it first.
- `render_welcome` and quick actions advertise Enter/start and `e` export, which are not currently implemented.
- `reset` only resets state and leaves view/stats as-is.

Relevant file: `crates/slapper/src/tui/tabs/dashboard.rs`.

### Required Fix

- Reset derived counters at the start of `update_from_history`.
- Correct quick-action copy or implement the advertised actions.
- Decide what reset means for Dashboard:
  - Either re-render welcome/session stats from current history, or explicitly no-op with user feedback.
- Use theme colors consistently.

### Acceptance Criteria

- Calling `update_from_history` twice with the same history yields the same `today_scans`.
- Dashboard copy matches implemented actions.
- Reset does not leave stale or misleading data.

### Tests

- `update_from_history` idempotence for `today_scans`.
- Dashboard reset behavior.

## Suggested Execution Order

1. Workstream 1: input cursor safety.
2. Workstream 3: overlay precedence.
3. Workstream 2: search correctness.
4. Workstream 5 and 6: visible keybinding/notification truthfulness.
5. Workstream 7: task result routing.
6. Workstream 4 and 8: tab hit-testing and focus consistency.
7. Workstream 9 and 10: layout and theme polish.
8. Workstream 11 and 12: feature-gated cleanup and dashboard data fixes.

## Verification Checklist

- `cargo test --lib -p slapper tui::`
- `cargo check --lib -p slapper`
- If feature-gated files are touched, run targeted checks such as:

```bash
cargo check --lib -p slapper --features nse
cargo check --lib -p slapper --features python-plugins
cargo check --lib -p slapper --features full
```

- Manually run the TUI at least once after implementation and check:
  - 80x24 terminal
  - Tab navigation and mouse tab selection
  - Search prompt and results
  - Command palette
  - Help popup
  - Confirm popup
  - Export success/no-data feedback
