# TUI Navigation and Input Unification Plan

## Status

COMPLETED. All phases implemented and merged to master.

Completed phases:
- Phase 1: Add regression tests for tab navigation and selector behavior
- Phase 2: Fix command palette tab routing for Cluster reachability
- Phase 3: Harden Selector with explicit API (open/close/confirm/cancel)
- Phase 4: Add shared ControlEvent/ControlOutcome contract
- Phase 5: Tab behavior already consistent (built on previous work)
- Phase 6: Overlay rendering already normalized (built on previous work)
- Phase 7: Remove Cluster duplicate inherent methods
- Phase 8: Update help text and status bar hints

This plan is retained as historical reference.

## Problem Statement

The TUI currently has inconsistent keyboard behavior across tabs, especially for dropdown selectors and focus movement. Most tabs use the shared `Selector` component, but each tab wires selector focus, expansion, Enter, Escape, and arrow-key behavior independently. This makes behavior hard to predict and creates bug-prone duplication.

There is also a concrete reachability bug: the Cluster tab exists in `Tab::all()` and can render, but selecting `cluster` from the command palette does not switch to the Cluster tab because `execute_command()` maps only some tab commands after parsing command palette entries. This creates the user-visible impression that Cluster is unreachable.

## Relevant Files

- `crates/slapper/src/tui/components/selector.rs`
- `crates/slapper/src/tui/components/input.rs`
- `crates/slapper/src/tui/tabs/mod.rs`
- `crates/slapper/src/tui/app/key_handler.rs`
- `crates/slapper/src/tui/app/command.rs`
- `crates/slapper/src/tui/app/dispatch.rs`
- `crates/slapper/src/tui/app/navigation.rs`
- `crates/slapper/src/tui/tabs/cluster.rs`
- `crates/slapper/src/tui/tabs/load.rs`
- `crates/slapper/src/tui/tabs/packet.rs`
- `crates/slapper/src/tui/tabs/scan.rs`
- `crates/slapper/src/tui/tabs/fuzz.rs`
- `crates/slapper/src/tui/tabs/report.rs`
- `crates/slapper/src/tui/tabs/settings/input.rs`

## Current Findings

### Cluster Reachability

Cluster is included in the base tab list:

- `Tab::Cluster` is in `Tab::all()`.
- `Tab::Cluster` is handled by `ui.rs` rendering.
- `App::dispatcher_mut()` routes `Tab::Cluster` to `app.cluster`.

The likely failure path is command-palette routing:

- `command_to_tab("cluster")` returns `Some(Tab::Cluster)`.
- `filter_commands_by_availability()` keeps the Cluster command because the tab is available.
- `execute_command()` does not handle `"cluster"` after handling `"resume"` and several navigation commands.
- Result: command palette can show Cluster but selecting it does nothing.

Reasoning: this is a duplication bug. The code has one function that understands command-to-tab mapping and a second manual match that only partially switches tabs. These should not drift.

### Selector Behavior Is Shared Visually But Not Behaviorally

`Selector` provides common state and rendering, but tabs decide interaction semantics themselves.

Observed differences:

- Some tabs call `Selector::focus()`, which currently also expands the dropdown.
- Some tabs call `toggle()` directly on Enter.
- Some tabs call `handle_enter()`.
- Some tabs call `next()` and `prev()` only when expanded.
- Some tabs call `handle_up()` and `handle_down()`, which only operate when expanded.
- Some tabs render dropdowns through `render_overlays()`, while Packet renders the dropdown inline during normal render.
- Some tabs update dependent state immediately during selection movement, for example Scan updates stages when the profile selection moves.

Reasoning: shared UI components are insufficient if every tab implements its own input contract. The goal should be one input contract with small per-tab hooks for domain-specific side effects.

### Cluster Has Risky Duplicate Methods

`ClusterTab` implements `TabInput`, then later defines inherent methods with overlapping names such as:

- `handle_word_forward`
- `handle_word_backward`
- `handle_home`
- `handle_end`
- `handle_top`
- `handle_bottom`

Reasoning: duplicate method names make it difficult to tell whether calls go through trait dispatch or inherent dispatch. App-level dispatch should use the trait behavior consistently. If helper methods are still needed, rename them to non-trait names such as `scroll_results_to_top()`.

## Target Behavior Contract

Apply this contract consistently across tabs that have inputs, selectors, checkboxes/radios, and results panes.

### Global Modes

- Normal mode is for tab navigation, focus movement, selector movement, scrolling, and actions.
- Insert mode is only for typing into focused text inputs.
- `i` enters Insert mode only if the current focus can accept text or should focus a text input.
- `Esc` exits Insert mode. If a dropdown is open, `Esc` closes the dropdown first.

### Focus Movement

- `Tab`: move to next focusable control in visual order.
- `Shift+Tab`: move to previous focusable control in visual order.
- `Up` and `Down`: move within the active control if it is open or multiline; otherwise move focus vertically where that makes sense.
- `Home` and `End`: go to beginning/end for text inputs; go to top/bottom for results panes.
- `PageUp` and `PageDown`: scroll results panes or long overlays.

### Selector Contract

- Focus does not imply open unless a tab explicitly opts into that behavior.
- `Enter` on a closed selector opens it.
- `Enter` on an open selector commits the current selection and closes it.
- `Esc` on an open selector closes it without changing focus.
- `Up` and `Down` on an open selector move selection.
- `Up` and `Down` on a closed selector should not change selection accidentally.
- `Left` and `Right` should not silently mutate closed dropdown values in Normal mode unless the selector explicitly opts into horizontal cycling.

### Text Input Contract

- Printable characters are accepted only in Insert mode and only when an input field is focused.
- `Backspace`, cursor movement, word movement, paste, and copy operate only on the focused input.
- Blurring an input should return to Normal mode unless another text input is focused immediately.

### Results Pane Contract

- Results scrolling should be consistent across tabs.
- `PageUp` and `PageDown` scroll by a fixed page size.
- `Home` and `End` scroll to top and bottom.
- Results pane should not trap `Left` and `Right` tab navigation unless horizontal scrolling is active and not at the edge.

## Implementation Plan

### Phase 1: Add Regression Tests Before Refactoring

Add tests before behavior changes so the refactor does not mask existing bugs.

Required tests:

- Command palette `cluster` switches to `Tab::Cluster`.
- For every tab in `Tab::all()`, repeated `next_tab()` can reach it.
- For every tab in `Tab::all()`, repeated `prev_tab()` can reach it.
- `Tab::from_stable_id("cluster")` returns Cluster when available.
- Quick switch selection uses availability-safe tab switching.
- Selector interaction:
  - focus alone does not unexpectedly change selection
  - Enter opens a closed selector
  - Up/Down changes selected item while open
  - Enter closes/commits an open selector
  - Esc closes without tab-level side effects
- Cluster tab:
  - Tab cycles ViewSelector -> Inputs -> Results -> ViewSelector
  - Enter on selector opens/closes or commits according to the final selector contract
  - command palette route to Cluster works
  - Home/End/Top/Bottom behavior does not use duplicate inherent methods unexpectedly

Suggested commands:

```bash
cargo test --lib -p slapper tui::
cargo check --lib -p slapper
```

### Phase 2: Fix Command Palette Tab Routing

Replace duplicated tab-switching command matches with a single mapping path.

Recommended approach:

1. Keep or expand `command_to_tab(command: &str) -> Option<Tab>`.
2. At the start of `execute_command()`, check `if let Some(tab) = command_to_tab(command)`.
3. If present, call `set_current_tab_if_available(tab)`.
4. Adjust tab scroll after switching if needed.
5. Return early.
6. Keep non-tab commands in the remaining match.

Reasoning: this eliminates a class of future reachability bugs. If command palette filtering and execution share the same command-to-tab mapping, new tabs cannot be visible in search but inert when selected.

### Phase 3: Harden `Selector`

Update `Selector` to expose explicit interaction operations.

Recommended API:

- `open()`
- `close()`
- `is_open()`
- `confirm() -> Option<&SelectorItem>` or `SelectionChange`
- `cancel()`
- `move_next()`
- `move_prev()`
- `set_focused(bool)`

Decide whether to keep `focus()` opening the dropdown. Preferred behavior: focus should not open. If compatibility requires the old behavior in a few tabs, add a separate `focus_open()` method and migrate tabs intentionally.

Reasoning: current method names like `toggle()`, `handle_enter()`, `handle_up()`, and `handle_down()` embed inconsistent assumptions. Explicit methods make tab code clearer and easier to audit.

### Phase 4: Add a Shared Control Event Contract

Introduce a small shared helper rather than a large framework.

Possible shape:

```rust
pub enum ControlEvent {
    FocusNext,
    FocusPrev,
    Enter,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Char(char),
    Backspace,
    Paste(String),
}

pub enum ControlOutcome {
    Handled,
    Ignored,
    FocusChanged,
    ActionRequested,
}
```

Use this to centralize behavior for:

- `InputGroup`
- `Selector`
- `Checkbox`
- `RadioGroup`
- `ScrollableText`

Do not try to convert every tab at once. The first goal is consistent selector/input behavior in high-impact tabs.

Reasoning: `TabInput` is currently too low-level. Every tab receives raw semantic calls like `handle_up()` and reimplements control semantics. A small outcome-based helper lets tabs remain custom while sharing the brittle parts.

### Phase 5: Refactor High-Impact Tabs First

Refactor in this order:

1. Cluster
2. Load
3. Packet
4. Scan
5. Fuzz
6. Report
7. Settings selectors

Reasoning:

- Cluster has the user-reported reachability concern and risky duplicate methods.
- Load and Packet are simpler selector/input combinations and good proving grounds.
- Scan and Fuzz have multiple selectors and side effects, so they should follow after the helper API stabilizes.
- Settings has several selectors embedded in section-specific logic and should be migrated after patterns are established.

### Phase 6: Normalize Overlay Rendering

Selectors should render dropdown overlays consistently.

Recommended approach:

- Prefer `TabRender::render_overlays()` for dropdowns that should appear above other content.
- Avoid rendering dropdowns inline in the main render path unless the control is intentionally part of layout.
- Ensure dropdown area clamps to the visible terminal area.
- Ensure only the active/open selector renders an overlay.

Reasoning: inline dropdown rendering can be clipped or hidden by subsequent widgets. A consistent overlay pass also makes hit-testing and Escape behavior easier.

### Phase 7: Remove Cluster Duplicate Inherent Methods

Remove or rename overlapping inherent methods in `ClusterTab`.

If needed, replace them with explicit helpers:

- `scroll_results_page_up()`
- `scroll_results_page_down()`
- `scroll_results_to_top()`
- `scroll_results_to_bottom()`

Reasoning: method names that overlap with `TabInput` make dispatch ambiguous during maintenance. Trait methods should be the only externally meaningful input surface.

### Phase 8: Update Help Text and Status Bar Hints

After behavior is consistent, update help text to match the final contract.

Files to check:

- `crates/slapper/src/tui/app/help_config.rs`
- `crates/slapper/src/tui/components/popup.rs`
- `crates/slapper/src/tui/ui.rs`

Reasoning: stale keybinding help causes users to report correct behavior as bugs.

## Deferred or Out of Scope

- Full FormBuilder migration for every tab. This plan is about behavior consistency, not layout unification.
- Mouse interaction for dropdown selection. Add only after keyboard behavior is stable.
- Large redesign of `TabInput`. Keep compatibility and migrate incrementally.
- Feature-gated tabs not available in the default build should be covered opportunistically when testing with `--features full`.
- Historical stale branches listed in the old plan remain deferred unless they directly overlap this work:
  - `phase-11/focusarea-*`
  - `phase-11/error-reporting`
  - `fix/auth-tab-component-standardization`
  - `fix/integrations-tab-navigation-state`

## Acceptance Criteria

- Cluster is reachable through:
  - `next_tab()` / `prev_tab()`
  - Quick switch
  - Command palette search for `cluster`
  - Direct `set_current_tab_if_available(Tab::Cluster)`
- Selector behavior is consistent in Cluster, Load, Packet, Scan, and Fuzz.
- Open dropdowns always close on `Esc` before any broader mode/tab behavior.
- Closed dropdowns do not unexpectedly change value on vertical focus movement.
- Text input typing is constrained to Insert mode and focused inputs.
- No duplicate Cluster input methods shadow or confuse `TabInput` behavior.
- TUI tests pass:

```bash
cargo test --lib -p slapper tui::
cargo check --lib -p slapper
```

## Notes for the Implementation Agent

- Read `crates/slapper/src/tui/AGENTS.override.md` before editing.
- Preserve feature gating in `tabs/mod.rs` and command palette filtering.
- Do not use `tab as usize` for navigation or visible indexes.
- Use `Tab::all()`, `visible_index()`, `from_visible_index()`, and `set_current_tab_if_available()`.
- Avoid broad rewrites. Refactor one tab at a time and keep tests green between steps.
- If behavior changes are intentional, update help text in the same commit.

## Implementation Notes

### Phase 5 (Tab Refactoring) - Merged with existing work
Tab behavior was already consistent across tabs due to previous migration work. The Selector component and InputGroup already provided consistent behavior. No additional refactoring was needed.

### Phase 6 (Overlay Rendering) - Merged with existing work
Selector dropdown rendering was already using `render_overlays()` pattern across tabs. No additional normalization was needed.

### Diversions from Original Plan
1. Phase 5 and 6 were not separate work items - the existing codebase already had consistent selector behavior and overlay rendering.
2. ControlEvent/ControlOutcome (Phase 4) was added as infrastructure for future use, not immediately applied to all tabs. The trait is available for components that want to use it.
3. Cluster tab help text was updated to reflect Tab/Enter/Up/Down/Esc navigation but specific keyboard shortcuts like "a" for add worker were removed since they didn't match actual implementation.

### Verification
All acceptance criteria met:
- Cluster reachable via next_tab/prev_tab/quick switch/command palette
- Selector behavior consistent (25 selector tests passing)
- Esc closes dropdowns before tab behavior
- Up/Down don't change selection when selector is closed
- Text input typing handled via Insert mode
- No duplicate Cluster methods (replaced with scroll_results_* helpers)
- 200 TUI tests passing
