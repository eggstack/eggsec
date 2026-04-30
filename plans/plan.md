# Slapper TUI Improvement Plan

**Date**: 2026-04-30
**Status**: Active plan for the next TUI hardening pass
**Priority**: High

---

## Executive Summary

The previous Phase 12 work stabilized the TUI tab identity model: `Tab::all()`, visible indexes, stable IDs, bookmarks, session restore, and basic tab-window clamping are considered complete. Do not re-open that work unless a regression is found.

The next problem is visual and interaction correctness. The current implementation can prove that the active tab is inside the logical `TabWindow`, but it does not prove that the rendered tab bar fits, that labels are readable, that mouse clicks map to what the user sees, or that narrow terminals remain usable.

This plan challenges the TUI as a rendered terminal interface, not just as state logic.

---

## Current Known Good Baseline

These items were completed before this plan and should be treated as baseline behavior:

- Runtime tab identity uses `Tab` variants.
- Runtime visible tab order comes from `Tab::all()`.
- Persistent tab identity uses `Tab::stable_id()`.
- `Tab::from_stable_id()` filters unavailable feature-gated tabs.
- `TabWindow::for_width()` clamps the active tab into the logical visible window.
- `App::last_tab_area_width` tracks the actual tab bar width after layout margin.
- Keyboard tab scroll adjustment uses `last_tab_area_width`.
- Command palette scroll height tracks the clamped render content height.
- Tests should use `App::new_for_testing()` to avoid ambient session files.

Recent verification:

```bash
cargo check --lib -p slapper
cargo test --lib -p slapper
cargo clippy --lib -p slapper
```

Known unrelated issue:

```text
cargo check --lib -p slapper --features rest-api,ai-integration
error: captured variable cannot escape `FnMut` closure body
   --> crates/slapper/src/agent/mod.rs:470:26
```

That feature-check failure is non-TUI and pre-existing. Do not conflate it with TUI work.

---

## Core TUI Rules

Use these meanings consistently:

| Concept | Meaning | Use For |
|---------|---------|---------|
| `Tab` enum variant | In-memory tab identity | Runtime state |
| `Tab::all()` position | Runtime visible index in current feature set | Rendering, keyboard selection, mouse selection |
| `Tab::stable_id()` | Persistent identity string | Sessions, bookmarks |
| `tab as usize` | Enum discriminant only | Legacy migration only |

Do not use `tab as usize` for visible navigation, rendering, bookmarks, or new session data.

Geometry rule:

- Any code that calculates tab visibility, rendered tab labels, or mouse hit targets must use the same effective tab-bar area.
- Tests must include actual rendered buffers where visual correctness matters. Pure state tests are not enough for the remaining TUI problems.

---

## Files of Interest

| Path | Why It Matters |
|------|----------------|
| `crates/slapper/src/tui/tabs/mod.rs` | `Tab`, tab titles, stable IDs, visible indexes, `TabWindow` |
| `crates/slapper/src/tui/ui.rs` | Top-level layout, tab rendering, status bar, command/search/HTTP popups |
| `crates/slapper/src/tui/app/navigation.rs` | Keyboard tab navigation and scroll adjustment |
| `crates/slapper/src/tui/app/runner.rs` | Event loop, mouse tab hit-testing, shortcuts |
| `crates/slapper/src/tui/app/mod.rs` | App-level navigation dispatch and fallback tab switching |
| `crates/slapper/src/tui/help.rs` | Command palette state and scroll behavior |
| `crates/slapper/src/tui/components/popup.rs` | Shared popup geometry helper |
| `crates/slapper/src/tui/search.rs` | Search overlay layout and result rendering |
| `crates/slapper/src/tui/tabs/*` | Per-tab focus and left/right behavior |

---

## Phase 13: Render-Accurate TUI Hardening

### 13.1: Replace Fixed Tab Capacity With Render-Aware Capacity

**Problem**: `TabWindow::for_width()` currently estimates capacity using a fixed minimum width. This keeps the active tab logically visible but does not guarantee the real labels fit. Labels such as `[4] Scan Endpoints`, `[8] WAF Stress`, and `[20] Settings` are wider than the fixed estimate.

**Goal**: The tab window should be based on the actual displayed tab labels, available inner width, and any range indicator/title text.

**Implementation guidance**:

- Inspect how Ratatui `Tabs` renders labels before changing the model.
- Prefer a helper that computes visible tabs by summing display widths of actual tab titles.
- Use Unicode/display-width aware measurement if labels may contain non-ASCII in the future. For current ASCII titles, `.len()` is acceptable only if documented.
- Account for tab block borders and title text. The title `Slapper[1-7/20]` consumes visible border/title space even if it does not reduce the inner tab content area in the same way as labels.
- Preserve the guarantee that the current tab is always included.
- Avoid hiding all tabs. Even at very narrow widths, show at least the active tab, possibly with a compact title.

**Acceptance criteria**:

- At terminal widths `30`, `40`, `60`, `80`, and `120`, the rendered tab row does not overflow or smear labels together.
- The selected tab label is visible and corresponds to `app.current_tab`.
- With all optional feature-gated tabs enabled, late tabs such as `Vuln`, `Workflow`, `Integrations`, and `Browser` can be reached and shown.
- Existing visible-index semantics remain unchanged.

### 13.2: Make Tab Mouse Hit-Testing Match Rendered Labels

**Problem**: Mouse hit-testing divides the tab area evenly by `window.max_visible`, but rendered tab labels are variable width. A click can select a different tab than the one under the pointer.

**Goal**: Mouse hit-testing should use the same geometry as tab rendering.

**Implementation guidance**:

- Create one shared helper that returns visible tab spans: `{ tab, global_visible_index, x_start, x_end }`.
- Use that helper in rendering tests and mouse hit-testing.
- Ignore clicks on borders, title text, range text, padding, and empty space.
- Route successful mouse tab selection through the same tab-switch path used by keyboard navigation so search cleanup and scroll adjustment stay consistent.
- Keep mouse disabled while help, search, HTTP options, command palette, or confirmation popups are visible.

**Acceptance criteria**:

- Clicking each visible tab selects exactly that tab after the tab window has scrolled.
- Clicking between labels, on the border, or on the title does not select an unrelated tab.
- Mouse behavior remains stable at `40x20`, `60x20`, and `80x24`.

### 13.3: Fix Tab Shortcut Semantics and Labels

**Problem**: Tab titles show numeric labels past 10, for example `[11] Proxy`, but keyboard shortcuts only support `1` through `9` and `0` for the tenth visible tab. The displayed labels imply direct shortcuts that do not exist.

**Goal**: Make tab labels truthful and useful.

**Options**:

- Option A: Keep shortcut labels only for tabs `1` through `9` and `0`; remove numeric labels from later tabs.
- Option B: Show visible position labels only when they are actually actionable.
- Option C: Replace shortcut-style labels with compact names and document tab switching in the status bar.

**Implementation guidance**:

- Avoid displaying `[11]`, `[12]`, etc. unless multi-digit tab shortcuts are implemented.
- If compact labels are introduced, make sure command palette and help still expose full tab names.
- Keep `Tab::stable_id()` and `Tab::all()` untouched.

**Acceptance criteria**:

- The rendered tab bar no longer implies unsupported shortcuts.
- `1` through `9` and `0` behavior remains deterministic.
- Help/status text accurately describes tab navigation.

### 13.4: Separate Intra-Tab Left/Right From Tab Switching

**Problem**: `App::handle_left()` and `App::handle_right()` fall back to previous/next tab when the current tab handler returns `false`. That makes incomplete per-tab focus handling feel like accidental tab switching.

**Goal**: Make horizontal movement predictable.

**Implementation guidance**:

- Audit every `TabInput::handle_left()` and `handle_right()` implementation.
- Decide whether left/right should move within a tab, switch tabs, or only switch tabs through explicit shortcuts such as `n/p`, `Shift+H/L`, or dedicated commands.
- If fallback tab switching remains, require each tab to explicitly report edge state via `is_at_left_edge()` and `is_at_right_edge()` rather than using `false` as an ambiguous signal.
- Avoid changing text input behavior in Insert mode.

**Acceptance criteria**:

- Pressing left/right inside horizontal controls does not unexpectedly switch tabs.
- Pressing left/right on passive tabs behaves consistently across `History`, `Dashboard`, and simple status-only tabs.
- Tab switching has an explicit, documented path.

### 13.5: Add Buffer-Level Render Tests for Narrow Screens

**Problem**: Current tests mostly validate state. The remaining bugs are visual, so tests need to inspect rendered buffers.

**Goal**: Add focused render tests that catch tab overflow, label mismatch, popup clipping, and status bar crowding.

**Implementation guidance**:

- Use Ratatui `TestBackend` or the repo's existing test style if one exists.
- Construct `App::new_for_testing(history)` and render through `ui::draw()`.
- Test widths: `30`, `40`, `60`, `80`, `120`.
- Test heights: `12`, `20`, `24`.
- Include a case where `current_tab` is near the end of `Tab::all()`.
- Include a stale `tab_scroll_offset` case.
- Assertions should check stable properties, not exact full-screen snapshots that will churn on harmless copy changes.

**Suggested assertions**:

- The active tab title or compact active label appears in the tab row.
- The tab row contains no obvious repeated truncation artifacts.
- The status bar renders mode text and does not overwrite the whole help text area.
- Command palette selected row remains within the rendered list area.
- Search and HTTP option popups do not produce zero-width or zero-height inner areas.

**Acceptance criteria**:

- New render tests fail against at least one currently known visual risk before fixes, or clearly document why they pass.
- Render tests pass after Phase 13 fixes.
- Tests are deterministic and do not depend on local session files.

### 13.6: Harden Overlay Layouts Under Small Terminal Sizes

**Problem**: Popup helpers clamp the outer rectangle, but internal layout still assumes enough rows for title, query, pagination, list, and buttons. This prevents panics in many cases but does not guarantee usable overlays.

**Goal**: Overlays should degrade coherently at small sizes.

**Targets**:

- Command palette
- Search popup
- Global search results
- HTTP options popup
- Confirmation popup
- Help popup

**Implementation guidance**:

- Clamp internal result counts to actual renderable rows, not requested popup height.
- Avoid minimum visible rows that exceed the actual list area.
- Use wrapping or truncation for long titles and command descriptions.
- Consider compact titles for narrow popups.
- Ensure `centered_rect()` never returns unusable dimensions when the terminal area is non-zero.

**Acceptance criteria**:

- At `40x12`, overlays do not panic and do not render controls outside their popup area.
- At `60x20`, command palette navigation keeps the selected item visible.
- Long command descriptions and search queries do not corrupt adjacent UI.

### 13.7: Audit Status Bar and Breadcrumb Width Behavior

**Problem**: The status bar uses percentage chunks and variable-length help text. Breadcrumbs can also become long for feature-rich tabs. These areas can visually crowd or truncate important state.

**Goal**: Status and breadcrumb areas should preserve high-value information first.

**Implementation guidance**:

- Prioritize mode, current task state, and error/running indicators over long help text.
- Use explicit compact variants at narrow widths.
- Check whether bookmark count and command palette hints are still useful when appended to already-long help strings.
- Render breadcrumbs with graceful truncation when the path exceeds available width.

**Acceptance criteria**:

- At `40x20`, mode and status remain readable.
- At `80x24`, help text does not overwrite status text.
- Long breadcrumbs do not hide the current tab context entirely.

---

## Verification Matrix

Run this matrix before marking Phase 13 complete.

| Scenario | Expected Result |
|----------|-----------------|
| Base build, `80x24`, repeated `n` | Every available tab becomes active, visible, and correctly highlighted |
| Base build, `60x20`, repeated `n` | Active tab remains visible; labels do not collide |
| Base build, `40x20`, repeated `n` | UI degrades coherently; active tab remains identifiable |
| Base build, `30x12`, open overlays | No panic; overlays remain bounded |
| Full feature build, late tabs | Late tabs are reachable and visible |
| Stale `tab_scroll_offset` near end | First render clamps around current tab |
| Mouse click after tab scroll | Clicked rendered tab is selected |
| Mouse click on border/title/empty tab area | No unrelated tab selection |
| Command palette at `60x20` | Selection remains visible while navigating |
| Command palette at `40x12` | No panic; visible rows match actual area |
| Search result with Unicode content | No panic; truncation is character safe |
| Long status/help text at `40x20` | Mode and status remain readable |

Feature combinations to check where practical:

```bash
cargo check --lib -p slapper
cargo test --lib -p slapper
cargo check --lib -p slapper --features python-plugins,ruby-plugins
cargo check --lib -p slapper --features nse
cargo check --lib -p slapper --features database,compliance,external-integrations,finding-workflow,vuln-management,headless-browser
```

Do not require `rest-api,ai-integration` to pass for TUI sign-off until the known `agent/mod.rs:470` issue is fixed separately.

---

## Completion Criteria

Phase 13 is complete when:

- [x] Tab rendering uses real label geometry or a documented compact-label model.
- [x] Mouse hit-testing matches rendered tab spans.
- [x] Tab labels no longer imply unsupported shortcuts.
- [x] Left/right behavior is predictable and documented.
- [x] Narrow-screen render tests cover the tab bar and overlays.
- [x] The verification matrix passes or any residual limitations are documented with exact terminal sizes and behavior.

**Phase 13 Implementation Summary (2026-04-30):**

- **13.1**: `TabWindow::for_width` now uses actual tab label widths instead of fixed min_tab_width=8. Added `visible_tab_spans()` for render-aware capacity.
- **13.2**: Mouse hit-testing now uses `visible_tab_spans()` to match rendered tab positions.
- **13.3**: Tab labels updated - tabs 1-10 show keyboard shortcuts [1]-[0], tabs 11+ show names without numeric prefixes.
- **13.4**: `handle_left()`/`handle_right()` now use `is_at_left_edge()`/`is_at_right_edge()` instead of fallback tab switching.
- **13.5**: Added 9 render tests covering various terminal sizes (30, 40, 60, 80, 120 widths).
- **13.6**: `visible_results_height()` now properly bounds by actual results count instead of always returning >= 5.
- **13.7**: Status bar and breadcrumb already use proper Paragraph widgets with overflow handling.

---

## Deferred Non-TUI Work

These are intentionally out of scope for this TUI pass:

- Fixing `agent/mod.rs:470` for `rest-api,ai-integration`.
- Refactoring autonomous agent internals.
- Adding new scan features.
- Reworking the broader theme system beyond what is needed for visual correctness.
