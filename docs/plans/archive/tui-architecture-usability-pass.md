# Eggsec TUI Architecture and Usability Handoff Plan

**Status: Completed 2026-06-11.** All 10 phases implemented using subagents for isolation. TUI crate checks and tests green after each phase and at end (`cargo fmt --all; cargo check -p eggsec-tui; cargo test -p eggsec-tui -- --test-threads=1`). Workspace/all-features run before handoff (pre-existing non-TUI errors in eggsec lib protobuf/codegen unrelated to this pass). README, AGENTS.md, AGENTS.override.md (TUI), and architecture/tui.md updated. See commit for full diff.

## Purpose

This plan is for a focused TUI architecture and usability pass in `crates/eggsec-tui`. The goal is not to rewrite the terminal UI. The current TUI already has the right high-level crate boundary and several useful primitives: session restore, background theme loading, command palette, quick switch, bookmarks, overlay precedence, tab windowing, and shared enforcement context. The work here should reduce central coordination pressure, make manual-mode discretion clearer to the operator, and improve discoverability without changing scanner semantics.

The important architectural direction is that `eggsec-tui` should remain an adapter over Eggsec capabilities. It should orchestrate user interaction, display state, and collect operator intent. It should not own core scanner behavior, policy semantics, network semantics, or output/report semantics.

## Current Observations

The current workspace cleanly separates the TUI crate from core, CLI, output, NSE, tool-core, and agent crates. Preserve that boundary.

The current `App` type has accumulated a large amount of cross-cutting state: current tab, input mode, session manager, theme manager, tab store, HTTP options, history, overlays, search, quick switch, task state, export format, help manager, command palette, redraw flag, tab scrolling, bookmarks, theme load state, enforcement context, loaded scope, and config path. This is acceptable for a small TUI, but Eggsec now has enough tabs and enough policy-sensitive behavior that central mutation will become brittle.

`KeyHandler` currently maps raw crossterm keys directly into app mutations. It handles global shortcuts, normal-mode behavior, insert-mode behavior, overlay routing, command palette behavior, quick switch behavior, search behavior, HTTP options behavior, help scrolling, confirmation behavior, paste/copy, task interruption, quit behavior, theme toggling, and tab switching. This should be split into intent decoding and app mutation.

The overlay precedence model is good and should be preserved: policy confirmation, confirm popup, command palette, quick switch, search, HTTP options, help. The problem is not the model; the problem is that overlay input behavior is spread across the key handler and app methods.

`Tab` currently owns or derives a lot of metadata through repeated match blocks: title, CLI command, description, stable ID, reverse stable ID, feature-gated visible ordering, discriminant mapping, next/previous navigation, and tab trait dispatch. This makes additions and refactors repetitive.

`TabStore` is a concrete field-per-tab struct. This is simple and type-safe, but it pushes repeated dispatch logic into `Tab` and `App`. Avoid forcing a dynamic plugin system unless necessary, but add a registry/spec layer so metadata and policy-facing classification are not repeated.

The TUI starts in a manual-permissive enforcement context, which is the right default for human-driven CLI/TUI operation. This must stay distinct from stricter agent-controlled operation. The TUI should surface the current enforcement posture clearly so user discretion is visible, auditable, and not surprising.

## Non-Goals

Do not rewrite the TUI from scratch.

Do not replace ratatui/crossterm.

Do not move scanner logic into the TUI crate.

Do not loosen central enforcement semantics. Manual-mode UI may use user discretion, but the underlying policy evaluator remains the authority.

Do not remove current features such as command palette, quick switch, bookmarks, session restore, theme loading, or existing keyboard behavior unless a replacement is implemented and tested.

Do not introduce a general-purpose runtime plugin system for tabs in this pass.

## Phase 1: Introduce a UI Action Layer

Add a new module, likely `crates/eggsec-tui/src/app/action.rs`, containing a `UiAction` enum. The first pass should model existing behavior without changing UX.

Suggested shape:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    Noop,
    Quit,
    StopActiveTask { message: String },
    ToggleHelp,
    ToggleCommandPalette,
    ToggleQuickSwitch,
    CloseQuickSwitch,
    ToggleSearch { global: bool },
    ToggleTheme,
    TogglePause,
    Resume,
    FocusNext,
    FocusPrev,
    PageUp,
    PageDown,
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    MoveTop,
    MoveBottom,
    MoveWordForward,
    MoveWordBackward,
    Home,
    End,
    Enter,
    Escape,
    EnterInsertMode,
    InputChar(char),
    Backspace,
    Delete,
    Paste(String),
    Copy,
    SelectTab(crate::tabs::Tab),
    NextTab,
    PrevTab,
    ToggleBookmark(crate::tabs::Tab),
    CycleExportFormat,
    ExportResults,
    ResetCurrent,
    SaveSettings,
    DeleteHistoryEntry,
    ConfirmPendingAction,
    CancelPendingAction,
    ConfirmPolicyAction,
    CancelPolicyAction,
    CommandPaletteInput(crate::app::command::PaletteInput),
    QuickSwitchInput(crate::app::navigation::QuickSwitchInput),
}
```

The exact enum does not need to match this precisely. The requirement is that raw key decoding and app mutation become separate.

Change `KeyHandler::handle_key_event` so it returns either `UiAction` or a small batch of actions, instead of directly mutating `App`. Then add `App::apply_action(action)` or `App::apply_actions(actions)` as the mutation point.

Keep a compatibility path during migration if necessary: `KeyHandler` can initially call an internal `decode_key_event(app_view, key) -> UiAction` and then immediately call `app.apply_action(action)`. The important outcome is a testable decode layer.

Acceptance criteria:

- Existing key behavior remains unchanged.
- Existing key-handler tests still pass or are updated to test the decoded action and the app mutation separately.
- `KeyHandler` no longer directly contains most business mutations such as `app.current_tab = ...`, `app.should_quit = true`, `app.overlay.notification = ...`, or `app.spawn_task(...)`.
- `App::apply_action` is the main mutation point for global UI actions.

Recommended tests:

- `Ctrl-C` decodes to stop-active-task when any task is active and quit when no task is active.
- `q` in normal mode decodes to quit only when no task is active.
- quick switch Down is overlay-local and does not route to tab content.
- search `Ctrl-U` clears the search query and does not page content.
- confirm popup blocks navigation keys.
- normal-mode Backspace/Delete do not edit fields.

## Phase 2: Extract Overlay Controller

Add `crates/eggsec-tui/src/app/overlay.rs` or expand the existing overlay/state modules with an `OverlayController`. The controller should own overlay-local input rules and emit `UiAction`s.

Preserve the current overlay precedence exactly:

1. Policy confirmation
2. Confirm popup
3. Command palette
4. Quick switch
5. Search
6. HTTP options
7. Help

Move these behaviors out of `KeyHandler` where practical:

- Policy confirmation: Enter confirms, Esc cancels, chars edit reason input, Backspace/Delete remove characters.
- Confirm popup: Enter/y confirms, Esc/n cancels.
- Command palette: query edits, selection movement, Enter selection, Tab/BackTab movement, Esc close.
- Quick switch: query edits, selection movement, paging, Home/End, Enter select, Esc close.
- Search: query edits, Enter or Ctrl-F performs search, Ctrl-U clears query.
- Help: scrolling with arrows, j/k, PageUp/PageDown, g/G.
- HTTP options close behavior.

Acceptance criteria:

- There is one overlay routing function that asks the app for the topmost overlay and dispatches to the matching overlay handler.
- Non-topmost overlays do not receive input.
- Overlay-local keys do not leak to tab content.
- Existing overlay tests continue to pass or become more direct.

## Phase 3: Add TabSpec Registry

Create a tab metadata registry so tab metadata is not spread across repeated match blocks.

Suggested module: `crates/eggsec-tui/src/tabs/spec.rs`.

Suggested types:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabCategory {
    Assessment,
    Traffic,
    Workflow,
    Reporting,
    Configuration,
    History,
    Dashboard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabRiskGroup {
    Passive,
    SafeActive,
    Intrusive,
    Administrative,
}

#[derive(Debug, Clone, Copy)]
pub struct TabSpec {
    pub tab: Tab,
    pub stable_id: &'static str,
    pub title: &'static str,
    pub cli_command: &'static str,
    pub description: &'static str,
    pub category: TabCategory,
    pub risk_group: TabRiskGroup,
    pub feature: Option<&'static str>,
}
```

Use this registry to back:

- `Tab::title()`
- `Tab::cli_command()`
- `Tab::description()`
- `Tab::stable_id()`
- `Tab::from_stable_id()`
- `Tab::all()` visible ordering
- quick switch search metadata
- command palette tab entries
- help and breadcrumb labels where possible

Be careful with feature-gated tabs. The registry should only expose enabled tabs through `Tab::all()`, but it can still include all enum variants internally if useful. Do not break session restoration from stable IDs. Hidden feature-gated tabs should not restore as current tab unless enabled.

Acceptance criteria:

- There is a single source of truth for title, stable ID, CLI command, description, category, and risk group.
- Existing session restore using stable IDs still works.
- Feature-gated tabs remain hidden when their feature is disabled.
- Quick switch fuzzy search still searches title, stable ID, and description.
- Numeric tab switching still follows visible tab ordering.

## Phase 4: Move Operation Descriptor Construction Closer to Tabs

Currently the central app builds operation descriptors by matching on current tab and manually mapping operation names, risks, target extraction, and features. This should move closer to each tab or to a tab adapter layer.

Add a trait such as:

```rust
pub trait TabOperation {
    fn operation_descriptor(&self) -> Option<eggsec::config::OperationDescriptor>;
    fn primary_target(&self) -> Option<String>;
    fn build_task_config(&self) -> Option<crate::workers::TaskConfig>;
    fn is_direct_launch(&self) -> bool { false }
}
```

This does not need to replace all existing traits immediately. A compatibility implementation can initially delegate to existing tab methods. The important direction is that `App` should not maintain a giant risk/target/operation table indefinitely.

Keep enforcement centralized. The app should still call the shared `EnforcementContext::evaluate(...)`, request policy confirmation, and spawn/deny as appropriate. Only descriptor construction and tab-specific metadata should move out of the central app.

Manual-mode behavior requirement:

- TUI remains manual-permissive by default.
- Out-of-scope or target-expansion cases may require confirmation rather than becoming hard boundaries, consistent with operator discretion.
- Higher-risk classes still require narrow confirmation semantics and audit logging.
- Agent/MCP/autonomous enforcement semantics are not changed by this TUI pass.

Acceptance criteria:

- `App::build_current_operation_descriptor` is either removed or reduced to a simple delegation.
- `App::current_tab_target` is either removed or reduced to delegation.
- Risk classification is derived from tab operation metadata or `TabSpec`, not repeated centrally.
- Policy confirmation behavior remains unchanged from the operator perspective.
- Direct-launch tabs still pass through enforcement before they are allowed to continue, or are stopped and replayed through a safe path.

## Phase 5: Improve Manual-Mode Scope and Risk Visibility

Add persistent operator-facing indicators for enforcement posture and target preflight.

Minimum UI additions:

- Status bar segment showing enforcement mode, for example `Mode: manual-permissive`.
- Status bar or breadcrumb segment showing whether a scope file is loaded, default-empty, or missing.
- Per-target preflight display on target-bearing tabs before launch.
- Risk badge for current tab/action: passive, safe-active, intrusive, administrative.
- Confirmation-required preview where possible.

Preflight display should include:

- Current target string.
- Parsed target status if available.
- Scope match result: in-scope, out-of-scope, explicit exclusion, unknown/no scope.
- Risk group.
- Operation name.
- Whether Enter will run immediately, warn, request confirmation, or deny.

The TUI should not block ordinary manual edits or navigation based on this preview. It is an operator aid. Actual policy evaluation remains authoritative at launch time.

Acceptance criteria:

- A user can see before launching whether the current target/action is likely to require confirmation.
- Manual-permissive mode is visible in the status/breadcrumb area.
- Empty/default scope is not silently ambiguous.
- Policy confirmation popup still appears when required.
- Status text is concise and does not crowd small terminals.

## Phase 6: Add Global Task Strip or Task Drawer

The TUI already tracks global task state so the app does not report ready while a task is running on another tab. Build on that with a visible task strip or drawer.

Minimum task strip:

- Active task tab name.
- Running/paused/stopping state.
- Elapsed time if available.
- Last event or latest status line if available.
- Key hints: `Ctrl-C stop`, `Ctrl-Z pause`, `Ctrl-Y resume`.

Optional drawer:

- A command palette action or shortcut to jump to the active task tab.
- A compact task log view.
- Last error/warning from task.

Acceptance criteria:

- When any task is active, the UI clearly indicates that a task is active even after navigating away from its tab.
- Quit-blocking behavior is visible and not surprising.
- Pause/resume state is visible.
- Stop behavior is still safe and graceful.

## Phase 7: Make Command Palette Action-Complete

The command palette should become the main discoverability surface. Every keybound global action should also be available from the command palette.

Add command palette entries for at least:

- Run current tab/action.
- Stop active task.
- Pause active task.
- Resume active task.
- Jump to active task.
- Open quick switch.
- Open help for current tab.
- Open search.
- Open global search.
- Toggle theme.
- Cycle export format.
- Export results.
- Copy CLI equivalent.
- Open settings.
- Reload scope/config if supported.
- Save settings when on Settings tab.
- Clear history / delete history entry when on History tab.

Commands should be context-aware. Irrelevant commands can be hidden or disabled with an explanation. Prefer disabled-with-reason for potentially confusing cases, such as `Stop active task` when no task exists.

Acceptance criteria:

- A user can operate the TUI primarily through command palette and Enter.
- Command palette entries include short descriptions and current shortcuts where applicable.
- Context-specific commands do not cause no-op confusion.
- Tests cover command selection for at least one global action, one tab action, and one unavailable/disabled command.

## Phase 8: Add Copy CLI Equivalent

Use the existing `Tab::cli_command()` concept as the foundation, but generate the actual command for the current form state.

Each executable tab should be able to produce a CLI equivalent that includes:

- Command name.
- Current target.
- Relevant tab options.
- Output format if applicable.
- Scope/config path if applicable.
- Policy/manual confirmation flags only if they correspond to explicit user choices and are safe to copy.

Add a command palette action and possibly a normal-mode keybinding for copying this command.

Acceptance criteria:

- Target-bearing tabs can produce a useful CLI equivalent.
- Clipboard failure is handled gracefully and surfaced to the user.
- Generated command escapes shell-sensitive values safely.
- Generated command does not include broad unsafe bypass flags by default.
- Tests cover at least recon, scan-ports, one intrusive tab, and one non-executable tab.

## Phase 9: Small-Terminal Layout Degradation

The runner currently warns under recommended size. Add actual degraded layouts.

Behavior targets:

- Under narrow width, collapse full tab bar into breadcrumb plus quick-switch hint.
- Under narrow width, hide low-priority status segments first.
- Under short height, reduce help/context blocks and favor current form/result content.
- Avoid drawing popups larger than the viewport.
- Preserve policy confirmation readability even in small terminals.

Acceptance criteria:

- 80x24 remains good.
- 100x30 or larger remains visually unchanged except for intentional improvements.
- 60x20 is usable for navigation and simple runs.
- Very small terminals render a clear “terminal too small” fallback instead of garbled UI.
- Snapshot/unit tests cover layout calculations where practical.

## Phase 10: Semantic Styling Tokens

Do not over-invest in visual redesign. Add semantic style helpers so risk and policy state are consistently represented across themes.

Suggested semantic roles:

- `safe`
- `warning`
- `danger`
- `muted`
- `active_task`
- `paused_task`
- `scope_match`
- `scope_miss`
- `policy_required`
- `policy_denied`

Acceptance criteria:

- Scope/risk/task states use semantic style helpers instead of ad hoc colors.
- Existing themes continue to work.
- Cyber Red remains the safe fallback.
- Theme loading remains non-blocking and non-fatal.

## Suggested Implementation Order

1. Add `UiAction` and split key decoding from mutation.
2. Extract overlay controller while preserving overlay precedence and tests.
3. Add `TabSpec` registry and migrate metadata reads.
4. Move operation descriptor and target extraction toward tab-local delegation.
5. Add manual-mode scope/risk preflight indicators.
6. Add global task strip/drawer.
7. Expand command palette to cover all keybound actions.
8. Add copy CLI equivalent.
9. Improve small-terminal degraded layouts.
10. Add semantic style helpers.

Each phase should compile and pass tests independently. Avoid large, unreviewable commits.

## Validation Commands

Run these after each substantial phase:

```bash
cargo fmt --all
cargo check -p eggsec-tui
cargo test -p eggsec-tui
cargo check --workspace --all-features
cargo test --workspace --all-features
```

If all-features is too slow during inner-loop work, at minimum run the TUI crate checks first and run workspace/all-features before final handoff.

## Manual Smoke Tests

Run the TUI in a normal terminal and verify:

- Startup is not visibly slower.
- Theme loading remains non-blocking.
- Cyber Red fallback still works.
- Quick switch opens, filters, moves selection, and selects tabs.
- Command palette opens, filters, moves selection, and runs actions.
- Help overlay scrolls and closes correctly.
- Search overlay edits query, clears query, and performs search.
- Confirm popup blocks background navigation.
- Policy confirmation blocks background navigation and accepts/cancels correctly.
- Starting a task shows global task state.
- Navigating away from a running task still shows active task state.
- `Ctrl-C` stops task before quitting.
- `q` does not quit while a task is active.
- Manual-permissive enforcement posture is visible.
- Out-of-scope or ambiguous target cases show advisory/preflight state before launch and confirmation when required.
- Small terminal sizes degrade gracefully.

## Safety and Policy Constraints

This pass must preserve the distinction between manual human operation and agent/autonomous operation.

For manual CLI/TUI operation, the user should have more discretion and clearer warnings. This means advisory preflight, explicit confirmation, visible scope state, and audit-friendly reason capture.

For MCP, autonomous, and agent-controlled operation, strict enforcement remains mandatory. Do not generalize TUI manual-permissive defaults into agent paths.

Do not add broad “disable enforcement” UI affordances. If an operation needs a manual override, the override should be narrow, visible, and auditable.

## Expected End State

After this pass, the TUI should feel less like a large collection of tab-specific forms and more like a coherent operator console. The implementation should have clearer layers:

- Raw input decoding.
- UI action application.
- Overlay-local routing.
- Tab metadata registry.
- Tab-local operation descriptor generation.
- Central enforcement evaluation.
- Central task state display.
- Discoverable command palette actions.

The result should be easier to maintain, safer to extend, and clearer for human operators using Eggsec in manual CLI/TUI workflows.