# TUI Bugs, Usability, and Theme Refinement Plan

Date: 2026-06-18

## Scope

This plan covers the module described in `architecture/tui.md`, with primary code under `crates/eggsec-tui/src/`.

The current TUI is functional and has several recent fixes already recorded in `architecture/tui.md` and `crates/eggsec-tui/src/AGENTS.override.md`: overlay precedence, `gg`, Ctrl-Space autocomplete, Settings save hints, GraphQL/OAuth `handle_enter`, theme display names, selector deduplication, deferred theme restore, and packaged theme loading. The plan below is therefore a follow-up hardening and refinement pass, not a rewrite.

## Findings From Interrogation

1. `handle_enter` behavior is still duplicated across many tabs.
   - Evidence: each tab hand-rolls focus blur, selector confirm, result guards, running guards, and `start()` transitions.
   - Risk: regressions like the recently fixed GraphQL/OAuth unreachable-start and fallthrough bugs can reappear in other tabs or future tabs.
   - Relevant files: `crates/eggsec-tui/src/tabs/*.rs`, `crates/eggsec-tui/src/app/mod.rs`, `crates/eggsec-tui/src/app/task_management.rs`.

2. Some component edge-case fixes are inconsistent.
   - `ScrollableText` explicitly handles empty content before bottom/down scrolls.
   - `Popup` uses safe arithmetic for content height but its scroll helpers still use a slightly different pattern.
   - Risk is low today, but the two scrollable primitives should share the same empty-content and clamping semantics.
   - Relevant files: `crates/eggsec-tui/src/components/scrollable.rs`, `crates/eggsec-tui/src/components/popup.rs`.

3. The theme system has two rendering paths.
   - Shell and popup layers increasingly pass explicit `&Theme`.
   - Many tabs/components still use the `tc!` thread-local macro.
   - Risk: missed synchronization bugs when changing themes, harder visual tests, and harder future removal of legacy global theme state.
   - Relevant files: `crates/eggsec-tui/src/theme/legacy.rs`, `crates/eggsec-tui/src/ui/*.rs`, `crates/eggsec-tui/src/tabs/*.rs`, `crates/eggsec-tui/src/components/*.rs`.

4. Theme loading works, but user feedback is sparse.
   - Errors produce a warning notification.
   - Successful background loading silently expands the Settings selector.
   - Users cannot reload themes from the TUI after editing files in `~/.config/eggsec/themes/`.
   - Relevant files: `crates/eggsec-tui/src/app/theme_runtime.rs`, `crates/eggsec-tui/src/tabs/settings/*`, `crates/eggsec-tui/src/theme/install.rs`.

5. Theme metadata is underpowered.
   - Theme IDs and display names are available, but the selector has no mode/source/status metadata.
   - Placeholders for missing current themes are clearer now, but there is no details pane explaining whether a theme is built-in, packaged, custom, loaded, invalid, or fallback-adjusted.
   - Relevant files: `crates/eggsec-tui/src/theme/manager.rs`, `crates/eggsec-tui/src/theme/loader.rs`, `crates/eggsec-tui/src/tabs/settings/main.rs`, `crates/eggsec-tui/src/tabs/settings/render.rs`.

6. The tab registry remains high-maintenance.
   - `architecture/tui.md` documents 7-9 locations for a new tab.
   - Risk: feature-gated tab availability, help text, command palette entries, export routing, and task construction can drift.
   - Relevant files: `crates/eggsec-tui/src/tabs/mod.rs`, `crates/eggsec-tui/src/tabs/spec.rs`, `crates/eggsec-tui/src/app/tab_store.rs`, `crates/eggsec-tui/src/app/command.rs`, `crates/eggsec-tui/src/app/export.rs`, `crates/eggsec-tui/src/app/navigation.rs`.

7. Visual and input regression coverage exists but is uneven.
   - Recent key handling tests are strong for overlays, `gg`, autocomplete, and several tab-specific `handle_enter` flows.
   - Coverage should expand to small terminal layouts, theme selector states, policy confirmation reason input, and no-result quick switch/search behavior.

## Implementation Plan

### Phase 1: Regression Harness and Bug Sweep

1. Add a table-driven `handle_enter` regression harness for representative tab patterns:
   - simple input-only launch tabs;
   - tabs with selector focus;
   - tabs with option checkbox focus;
   - direct-launch tabs;
   - results-focused tabs.

2. Add tests proving Enter never starts a task when it only blurred an input or confirmed/toggled a selector/checkbox.

3. Audit remaining direct field/checkbox indexing in production tab code. Convert risky production reads/writes to `.get()`, `.get_mut()`, `.first()`, or small helpers. Leave test-only direct indexing alone unless it obscures intent.

4. Normalize `Popup` scroll helpers to match `ScrollableText` empty-content behavior, then add explicit empty-content tests for `scroll_down`, `scroll_to_bottom`, and render with very large `scroll_offset`.

5. Run:
   - `cargo test -p eggsec-tui`
   - `cargo check -p eggsec-tui`

### Phase 2: Input and Navigation Usability

1. Add a small action hint model per tab/focus area instead of scattering static hint strings in render methods.

2. Make status-bar hints context-aware:
   - input focused: edit, paste, autocomplete, blur;
   - selector open: confirm/cancel/move;
   - results focused: scroll/copy/export;
   - task running: stop/pause/resume.

3. Improve discoverability for direct-launch and policy-gated tabs:
   - show dry-run/live status clearly;
   - show whether Enter will edit, toggle, or launch;
   - show pending policy confirmation reason affordance.

4. Add no-result states for command palette, quick switch, and search that are visually distinct from an empty list.

5. Add visual regression tests with `TestBackend` for:
   - 80x24 and narrow terminal layouts;
   - Settings Theme section;
   - policy confirmation overlay;
   - quick switch with no matches;
   - command palette with a disabled command selected.

### Phase 3: Theme System Refinement

1. Introduce `ThemeInfo` metadata in `ThemeManager`:
   - canonical ID;
   - display name;
   - mode;
   - source: built-in, packaged, custom, placeholder;
   - load status: loaded, invalid, missing, fallback-adjusted.

2. Keep the existing `Theme` rendering API stable, but make Settings consume `ThemeInfo` instead of `(id, label)` tuples.

3. Add a Settings Theme details pane:
   - current theme display name and ID;
   - source/mode;
   - number of loaded themes;
   - invalid theme count;
   - theme directory path;
   - contrast validation result.

4. Add a reload action for themes from Settings:
   - key: `r` only when the Theme section is focused and no selector is open;
   - call the existing background loader;
   - show `Loading themes...`, then `Themes loaded: N` or warning details.

5. Add a theme preview row rendered with semantic tokens:
   - normal text;
   - selected text;
   - success/warning/error/info;
   - safe/danger;
   - policy required/denied.

6. Expand contrast validation:
   - keep current text/background and selected pairs;
   - add focus border/background, warning/background, error/background, success/background, and policy token checks;
   - report warnings without rejecting otherwise usable custom themes unless core text contrast fails.

7. Start migrating renderers from `tc!` to explicit `&Theme`:
   - first components (`InputField`, `Selector`, `ScrollableText`, `Popup`);
   - then high-traffic shell-adjacent tabs (`Dashboard`, `Settings`, `History`);
   - keep `tc!` available during migration but add a lint-style grep check in the plan notes for new code.

### Phase 4: Registry and Command Palette Cleanup

1. Extend `TabSpec` so more tab metadata lives in one place:
   - title;
   - description;
   - stable ID;
   - category;
   - risk group;
   - direct-launch flag;
   - default command/help/export capability flags.

2. Add consistency tests:
   - every `Tab::all()` item has a `TabSpec`;
   - every available tab has help;
   - every command-palette tab jump points to an available tab;
   - feature-gated tabs never restore into unavailable variants.

3. Reduce exhaustive-match drift where practical without changing public behavior:
   - keep `TabStore` ownership as-is initially;
   - move command/help/export capability decisions to `TabSpec`;
   - only defer deeper trait-object registry work if the first step becomes too invasive.

### Phase 5: Verification and Documentation

1. Update `architecture/tui.md` after implementation with:
   - new theme metadata flow;
   - theme reload behavior;
   - explicit-theme migration rule;
   - any changed key bindings.

2. Update `crates/eggsec-tui/src/AGENTS.override.md` with the new bug patterns and preferred helpers.

3. Verification matrix:
   - `cargo check -p eggsec-tui`
   - `cargo test -p eggsec-tui`
   - `cargo check -p eggsec --features wireless,wireless-advanced`
   - `cargo check -p eggsec --features web-proxy`
   - `cargo check -p eggsec --features db-pentest`
   - `cargo check -p eggsec --features c2`

## Acceptance Criteria

- Enter key behavior is covered by table-driven tests for each major tab interaction pattern.
- No production TUI tab relies on unchecked indexing for mutable input/selector/checkbox paths where a malformed or future-shortened collection could panic.
- Settings Theme shows source, mode, loaded count, invalid count, theme directory, contrast status, and a semantic preview.
- Users can reload themes from the TUI without restart.
- Theme load success and failure both produce useful user-visible feedback.
- New component rendering APIs accept explicit `&Theme`; `tc!` remains only as a compatibility bridge during migration.
- Small terminal visual tests cover the main shell, Settings Theme, and top-priority overlays.
