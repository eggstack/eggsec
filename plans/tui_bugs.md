# TUI Bugs, Usability, and Theme Refinement Plan

Date: 2026-06-18

## Scope

This plan covers the TUI module described by `architecture/tui.md`, primarily `crates/eggsec-tui/src/`.

The deleted tracked version of this file listed several items that have since landed: action hints, no-result states, Settings theme metadata/preview, theme reload plumbing, `gg`, Ctrl-Space autocomplete, GraphQL/OAuth Enter fixes, selector deduplication, and theme display-name polish. This plan is a fresh follow-up based on the current code.

## Current Findings

1. **Settings Theme reload is not reachable through the advertised normal-mode key.**
   - Evidence: `tabs/settings/input.rs` sets `pending_theme_reload` from `SettingsTab::handle_char('r')`, but normal-mode `r` is decoded globally to `UiAction::ResetCurrent` in `app/key_handler.rs`, and `UiAction::ResetCurrent` requests `PendingAction::ResetTab` in `app/mod.rs`.
   - Impact: Settings > Theme says `Press [r] to reload themes`, but pressing `r` in normal mode opens a reset confirmation instead. Reload is effectively hidden behind insert-mode character dispatch.

2. **Settings action hints are stale for the Theme section.**
   - Evidence: `app/action_hints.rs::settings_hints()` always returns `r:reset`, while Theme section rendering says `r` reloads themes.
   - Impact: the footer/status bar can contradict the focused panel.

3. **Theme source metadata is inferred incorrectly.**
   - Evidence: `app/theme_runtime.rs::handle_theme_install_report()` labels the first `report.installed` loaded themes as `Packaged` and everything else as `Custom`.
   - Impact: on later launches/reloads where packaged themes already exist or the version marker short-circuits install (`installed == 0`), packaged themes can be shown as custom themes.

4. **Invalid theme files are logged but not represented in Settings metadata.**
   - Evidence: `theme/install.rs::load_themes_from_dir()` returns `Vec<Result<Theme, ThemeLoadError>>` without the file stem/path; `handle_theme_install_report()` logs `Err` results but cannot call `ThemeManager::mark_theme_invalid()` for the failed file.
   - Impact: Settings can show `0 invalid` even when broken `.toml` files exist in the theme directory.

5. **The Theme details pane does not report actual current-theme contrast warnings.**
   - Evidence: `theme/manager.rs::validate_contrast()` exists, but `tabs/settings/render.rs` renders `Contrast: OK` based only on `theme_invalid_count == 0`.
   - Impact: users can see `Contrast: OK` for a theme that has non-fatal contrast warnings or fallback-adjusted color pairs.

6. **Manual theme reload lacks success feedback and a distinct runtime reason.**
   - Evidence: `handle_theme_install_report()` only surfaces notifications on errors. Manual reload and startup load share the same `ThemeLoadState`.
   - Impact: after pressing reload, users may not know whether themes reloaded, no-op loaded, or were skipped because a loader was already running.

7. **Running-task hints only check `task_state.handle`.**
   - Evidence: `app/action_hints.rs::get_action_hints()` uses `app.task_state.handle.is_some()`, while `App::has_active_task()` also checks `tab`, `progress_rx`, and `result_rx`.
   - Impact: edge states such as stopping, paused drain states, or direct-launch task transitions can show normal tab hints instead of stop/resume hints.

8. **Launch semantics remain duplicated across tabs.**
   - Evidence: many `tabs/*.rs` implementations hand-roll `handle_enter()` with similar blur/toggle/open/confirm/start behavior, and `App::handle_enter()` has a retroactive policy gate for direct-launch tabs.
   - Impact: fixes like the recent GraphQL/OAuth Enter bug can reappear in future tabs. The retroactive direct-launch gate also briefly lets tab state enter `Running` before enforcement stops it for confirmation/denial.

9. **There is still residual unchecked-index risk in dynamic render paths.**
   - Evidence: production code still contains direct `fields[n]` or slice indexing in places like `tabs/fuzz.rs` render paths, `tabs/workflow.rs` field rendering, and `ui/shell.rs` visible title slicing.
   - Impact: most are protected by current construction invariants, but future field-count changes can turn UI edits into panics.

10. **Theme rendering is still split between explicit theme references and thread-local `tc!`.**
    - Evidence: `rg` reports 604 `tc!` call sites in `crates/eggsec-tui/src`, with large clusters in `tabs/intercept.rs`, `tabs/wireless.rs`, and `tabs/settings/render.rs`.
    - Impact: theme tests remain harder to localize, and runtime theme sync depends on the legacy thread-local bridge.

## Implementation Plan

### Phase 1: Fix Settings Theme Reload and Hints

1. Add an explicit `UiAction::ReloadThemes` or a Settings-aware branch before `ResetCurrent`.
   - When current tab is Settings, current section is Theme, no selector is open, and no task is active, normal-mode `r` should set `pending_theme_reload` instead of opening reset confirmation.
   - Keep reset available outside Theme. If needed, expose a clear alternate reset command through the command palette.

2. Update `get_action_hints()` so Settings hints are section-aware.
   - Theme section, selector closed: `r:reload Enter:themes Tab:next`.
   - Theme section, selector open: `Enter:select ↑↓:theme Esc:cancel`.
   - Non-theme Settings sections: keep `s:save r:reset Tab:next`.

3. Add regression tests:
   - normal-mode `r` in Settings > Theme sets the reload flag and does not set `pending_action`;
   - normal-mode `r` in Settings > HTTP still requests reset confirmation;
   - Theme action hints show reload, not reset;
   - open theme selector prevents reload and keeps selector navigation behavior.

### Phase 2: Make Theme Loading Metadata Correct

1. Replace `Vec<Result<Theme, ThemeLoadError>>` in `ThemeInstallReport` with structured records:
   - `id` / file stem;
   - path;
   - source hint (`Packaged` or `Custom`);
   - result;
   - load warnings.

2. Stop inferring `ThemeSource` from `installed` count.
   - Use a packaged theme ID/path set from generated packaged metadata, or include source while loading the directory.
   - Ensure existing packaged themes remain `Packaged` after startup, reload, and version-marker short-circuit.

3. Represent invalid and missing themes in `ThemeManager`.
   - Add or use a method that inserts `ThemeInfo { status: Invalid(reason), ... }` even when no `Theme` was registered.
   - Add `Missing` metadata for a restored/current theme placeholder when the theme cannot be found.

4. Track fallback-adjusted themes.
   - Have the loader return contrast/fallback warnings alongside the adjusted `Theme`, or expose a validation report so `ThemeManager` can set `ThemeLoadStatus::FallbackAdjusted`.

5. Update Settings Theme details.
   - Source/mode/status should come from the selected `ThemeInfo`.
   - Placeholder themes should render as `Missing`, not default to `Built-in · Dark`.
   - Invalid count should derive from actual invalid metadata, not just successful registrations.

6. Add tests for:
   - invalid `.toml` appears in metadata and increments invalid count;
   - existing packaged theme remains `Packaged` when no new themes are installed;
   - fallback-adjusted status is visible;
   - missing restored theme renders as a missing placeholder.

### Phase 3: Improve Theme Reload Feedback

1. Add a theme-load reason to `ThemeLoadState`, such as `Startup` vs `ManualReload`.

2. On manual reload:
   - show `Loading themes...` immediately;
   - on success, show loaded/invalid/skipped counts;
   - on no-op, say no new themes were found but the directory was scanned;
   - if a reload is already running, show a concise warning instead of silently ignoring it.

3. Decide and document packaged-theme reinstall semantics.
   - If deleted packaged themes should be restored on reload, bypass the version-marker short-circuit for manual reload.
   - If deletion is considered user opt-out, expose that in the Settings details text and keep startup fast.

### Phase 4: Consolidate Enter/Launch Semantics

1. Introduce a small return model for tab Enter handling, for example:
   - `Consumed`;
   - `InputBlurred`;
   - `SelectorOpened`;
   - `SelectorConfirmed`;
   - `WantsRun`;
   - `Noop`.

2. Migrate representative tabs first: Recon, Fuzz, GraphQL, OAuth, Packet, Settings, Wireless, DbPentest.

3. Move policy evaluation before a tab transitions into running state for direct-launch tabs where practical.

4. Keep the current `handle_enter()` trait method during migration by wrapping old behavior, then tighten once enough tabs use the new model.

5. Extend the existing `tabs/handle_enter_regression.rs` harness for feature-gated direct-launch tabs where compile features allow it.

### Phase 5: Harden UI State Access

1. Audit production direct indexing in `crates/eggsec-tui/src`.
   - Convert dynamic field/render paths to `get()`, `get_mut()`, `first()`, or zipped iteration.
   - Leave stable test setup indexing alone unless it hides production assumptions.

2. Add invariant tests for `TabWindow` and `draw_tabs()` so `all_tabs[window.start..window.end]` cannot panic after future tab-count changes.

3. Add render tests for shortened/malformed input groups on tabs that manually address fields by index.

4. Normalize component scroll clamping.
   - Ensure `Popup` and `ScrollableText` share empty-content and overlarge-offset semantics.
   - Add explicit tests for empty content, huge content, and huge scroll offsets.

### Phase 6: Refine Action Hints and Task Status

1. Change running-task hint detection to use `App::has_active_task()` or `task_status_summary()` instead of only `task_state.handle`.

2. Make hints reflect local widget state:
   - closed selector: `Enter:open`;
   - open selector: `Enter:select Esc:cancel`;
   - results area: `↑↓:scroll y:copy e:export`;
   - policy reason field: `Enter:confirm Bksp:edit Esc:cancel`;
   - direct-launch dry-run/live tabs: show whether Enter launches dry-run or requires policy confirmation.

3. Add status-bar rendering tests for paused, stopping, selector-open, and Settings Theme states.

### Phase 7: Continue Theme API Migration

1. Add explicit-theme render variants for reusable components:
   - `InputField`;
   - `InputGroup` / `FormBuilder`;
   - `Selector`;
   - `Checkbox` / `RadioGroup`;
   - `ProgressGauge`;
   - `ScrollableText`;
   - `Popup`;
   - `empty_state_paragraph`.

2. Keep existing render methods as compatibility wrappers during migration.

3. Convert call sites in this order:
   - Settings, shell-adjacent UI, and tests;
   - Dashboard/History;
   - high-use scan/fuzz/recon tabs;
   - large feature tabs such as Intercept and Wireless.

4. Add a low-friction guard for new code.
   - Start by failing only on new `tc!` usage in `components/` after component migration.
   - Later expand to `ui/`, then selected tabs.

### Phase 8: Registry and Documentation Cleanup

1. Extend `TabSpec` as the canonical source for title, description, stable ID, command visibility, help visibility, export visibility, and launch capability where practical.

2. Add consistency tests:
   - every visible tab has a spec;
   - every spec maps to a visible tab under the active feature set;
   - command-palette tab jumps target available tabs;
   - session restore never selects an unavailable feature-gated tab;
   - every tab with `supports_run` has a launch test or an explicit exemption.

3. Update `architecture/tui.md` and `crates/eggsec-tui/src/AGENTS.override.md` after implementation with:
   - corrected Settings Theme reload behavior;
   - theme metadata/reporting model;
   - explicit-theme rendering migration rule;
   - new tests and bug patterns.

## Verification Matrix

Run after each implementation batch:

```bash
cargo check -p eggsec-tui
cargo test -p eggsec-tui
```

Run before merging theme/feature-facing changes:

```bash
cargo check -p eggsec --features wireless,wireless-advanced
cargo check -p eggsec --features web-proxy
cargo check -p eggsec --features db-pentest
cargo check -p eggsec --features c2
```

## Acceptance Criteria

- Pressing `r` in Settings > Theme reloads themes in normal mode and does not request a reset.
- Settings hints match the focused section and widget state.
- Theme source, invalid, missing, and fallback-adjusted statuses are accurate after startup and manual reload.
- The Theme details pane reports actual selected-theme status and contrast warnings.
- Manual theme reload produces clear start/success/error/no-op feedback.
- Running/stopping/paused tasks always show task-appropriate hints.
- Representative direct-launch tabs do not enter running state before policy gating where the app has enough data to decide first.
- Production render paths no longer rely on unchecked dynamic field indexing.
- New component rendering APIs accept explicit `&Theme`, with `tc!` reduced and contained as a compatibility bridge.
