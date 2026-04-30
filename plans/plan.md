# Slapper Improvement Plan - Active TUI Corrections

**Date**: 2026-04-30
**Status**: Phase 12 attempted; corrective iteration required
**Priority**: High

---

## Executive Summary

Previous improvement waves are treated as historical/completed. This plan now tracks only the remaining corrective work for the TUI tab/navigation hardening effort.

The first Phase 12 implementation moved in the right direction by adding:

- `Tab::visible_index()`
- `Tab::from_visible_index()`
- `Tab::stable_id()`
- `Tab::from_stable_id()`
- `TabWindow::for_width()`
- Initial mouse hit-testing changes
- Initial session fields for stable IDs
- Popup size clamping attempts

However, review found that the implementation still mixes tab enum discriminants, visible indexes, and stable IDs in several paths. The next iteration should not broaden scope; it should correct these specific model inconsistencies and harden the remaining small-screen/Unicode cases.

---

## Phase 12R: TUI Tab Model Correction (ACTIVE)

**Objective**: Make tab navigation, rendering, mouse selection, bookmarks, and session persistence use one consistent model across base and feature-gated builds.

### Core Rule

Use these meanings consistently:

| Concept | Meaning | Use For |
|---------|---------|---------|
| `Tab` enum variant | In-memory active tab identity | Runtime state |
| `Tab::all()` position | Visible/runtime tab index in current feature set | Rendering, keyboard selection, mouse selection |
| `Tab::stable_id()` | Persistent identity string | Sessions, bookmarks |
| `tab as usize` | Enum discriminant only | Avoid for navigation/persistence |

**Do not use `tab as usize` for tab navigation, visible selection, bookmarks, or session state.**

---

## 12R.1: Fix Stable ID Availability

**Problem**: `Tab::from_stable_id()` currently returns tabs even when the tab is not available in the current feature set. Example: `"nse"` can restore `Tab::Nse` in a build without the `nse` feature.

**Files**:

- `crates/slapper/src/tui/tabs/mod.rs`
- Existing tests in `crates/slapper/src/tui/app/navigation.rs` or a new local test module

**Tasks**:

- [ ] Keep the match from stable ID to enum variant.
- [ ] After matching, check `tab.visible_index().is_some()`.
- [ ] Return `Some(tab)` only when the tab exists in `Tab::all()`.
- [ ] Return `None` for feature-gated tabs that are unavailable in the current build.
- [ ] Add tests for available stable IDs.
- [ ] Add tests documenting unavailable IDs when feature flags are absent, where practical.

**Acceptance Criteria**:

- [ ] `Tab::from_stable_id("settings") == Some(Tab::Settings)` in base builds.
- [ ] Feature-gated IDs do not restore unavailable tabs.
- [ ] Session restore gracefully falls back when saved tab ID is unavailable.

---

## 12R.2: Fix `TabWindow` Active-Tab Clamping

**Problem**: `TabWindow::for_width()` accepts `current_tab`, but if `previous_offset` points to a window that does not contain the current tab, it leaves the window stale and clamps `selected_visible` to another visible slot. That can highlight a tab that is not the active tab.

**Files**:

- `crates/slapper/src/tui/tabs/mod.rs`
- `crates/slapper/src/tui/ui.rs`
- `crates/slapper/src/tui/app/navigation.rs`
- `crates/slapper/src/tui/app/runner.rs`

**Tasks**:

- [ ] Compute `current_idx = current_tab.visible_index().unwrap_or(0)`.
- [ ] Compute `max_visible` from width.
- [ ] Start from `previous_offset`, clamped to valid range.
- [ ] If `current_idx < start`, set `start = current_idx`.
- [ ] If `current_idx >= start + max_visible`, set `start = current_idx + 1 - max_visible`.
- [ ] Recompute `end = min(start + max_visible, total_tabs)`.
- [ ] Set `selected_visible = current_idx - start`; do not silently select a different visible tab.
- [ ] Add tests for stale offsets before and after the active tab.

**Acceptance Criteria**:

- [ ] For every available tab and widths `40`, `60`, `80`, `100`, `120`, `window.start <= current_idx < window.end`.
- [ ] `selected_visible` always points at `current_tab`.
- [ ] Repeated `n` and `N` never show content for one tab while highlighting another.

---

## 12R.3: Remove Hardcoded Width From Scroll Adjustment

**Problem**: `App::adjust_tab_scroll()` currently calls `TabWindow::for_width(80, ...)`. That desynchronizes narrow terminals from rendering and mouse behavior.

**Files**:

- `crates/slapper/src/tui/app/navigation.rs`
- `crates/slapper/src/tui/app/mod.rs`
- `crates/slapper/src/tui/app/runner.rs`
- `crates/slapper/src/tui/ui.rs`

**Preferred Approach**:

- Track `last_tab_area_width: u16` or `last_terminal_width: u16` in `App`.
- Update it during draw and mouse handling.
- Make `adjust_tab_scroll()` use that stored width.

**Alternative Approach**:

- Replace `adjust_tab_scroll()` with `adjust_tab_scroll_for_width(width: u16)`.
- Call it from paths that know width.
- Keep a no-argument fallback only for tests, with an explicit documented default.

**Tasks**:

- [ ] Add width state or pass width explicitly.
- [ ] Remove production use of hardcoded `80`.
- [ ] Ensure keyboard navigation updates scroll using the same width as rendering.
- [ ] Ensure initial restored tab is visible on first draw.
- [ ] Update tests to cover narrow width behavior.

**Acceptance Criteria**:

- [ ] At `60x20`, keyboard navigation keeps the active tab visible.
- [ ] At `40x20`, the active tab remains visible even when only a few tabs fit.
- [ ] No production path relies on a fixed width of `80` for tab-scroll correctness.

---

## 12R.4: Fix Bookmark Identity

**Problem**: Bookmarks still store numeric indexes, and `Ctrl+B` still passes `app.current_tab as usize`. This preserves the original discriminant/visible-index bug.

**Files**:

- `crates/slapper/src/tui/app/mod.rs`
- `crates/slapper/src/tui/app/runner.rs`
- `crates/slapper/src/tui/session.rs`
- Any UI/status code that displays bookmark counts

**Tasks**:

- [ ] Change `App::bookmarks` from `HashSet<usize>` to `HashSet<String>` or `HashSet<Tab>` if serialization is handled separately.
- [ ] Prefer `HashSet<String>` using `Tab::stable_id()` for persistence simplicity.
- [ ] Replace `toggle_bookmark(tab_index: usize)` with `toggle_bookmark(tab: Tab)` or `toggle_current_bookmark()`.
- [ ] Replace `is_bookmarked(tab_index: usize)` with stable-ID or `Tab` based API.
- [ ] Replace `get_bookmarked_tabs() -> Vec<usize>` with `get_bookmarked_tab_ids() -> Vec<String>` or equivalent.
- [ ] Remove `app.current_tab as usize` from bookmark paths.
- [ ] Preserve backward compatibility for old `legacy_bookmarks`.

**Acceptance Criteria**:

- [ ] Bookmarking `Settings`, `History`, or `Dashboard` in a base build persists and restores the same tab.
- [ ] Bookmarking late feature-gated tabs persists by stable ID.
- [ ] Restoring unavailable feature-gated bookmark IDs drops them safely.
- [ ] No bookmark path uses enum discriminants.

---

## 12R.5: Fix Session Capture and Restore

**Problem**: Session state now has stable-ID fields, but capture still writes legacy discriminants and converts bookmark numbers through `Tab::from_index()`. Because bookmarks may contain discriminants, this can save the wrong bookmark ID.

**Files**:

- `crates/slapper/src/tui/session.rs`
- `crates/slapper/src/tui/app/mod.rs`

**Tasks**:

- [ ] Keep `current_tab_id: Option<String>` as the primary field.
- [ ] Keep legacy numeric fields only for backward-compatible reads.
- [ ] During capture, write `current_tab_id = app.current_tab.stable_id()`.
- [ ] During capture, write bookmarks directly from stable bookmark state.
- [ ] Do not derive primary bookmark IDs through `Tab::from_index()` unless migrating old numeric state.
- [ ] During restore, prefer stable IDs.
- [ ] During legacy restore, interpret old numeric values carefully:
  - If old values were visible indexes, use `from_visible_index`.
  - If old values were enum discriminants, map through a discriminant-specific helper.
  - If ambiguity cannot be resolved, document the fallback and drop invalid entries safely.
- [ ] Call tab-scroll adjustment after restore or ensure first render clamps active tab into view.

**Acceptance Criteria**:

- [ ] New session files persist stable tab IDs and stable bookmark IDs.
- [ ] Old numeric session files do not panic.
- [ ] Session saved on `Dashboard` restores `Dashboard`.
- [ ] Session saved with unavailable tab ID falls back to `Recon` or first available tab.

---

## 12R.6: Fix Mouse Hit-Testing Edge Cases

**Problem**: Mouse hit-testing now uses `TabWindow`, which is directionally correct, but it still divides the full tab area evenly by `window.max_visible`. Ratatui `Tabs` labels are not guaranteed to occupy equal-width slots, and border/title areas may still map to a tab.

**Files**:

- `crates/slapper/src/tui/app/runner.rs`
- Potential shared helper near `TabWindow`

**Tasks**:

- [ ] Guard against `tab_width == 0`.
- [ ] Account for tab bar x offset and border interior consistently.
- [ ] Ignore clicks on the border/title area.
- [ ] If keeping equal-width approximation, document it and test basic correctness after scrolling.
- [ ] Prefer a helper that maps click x-position to visible tab using the same title widths used for rendering, if practical.
- [ ] Ensure click selection calls the same tab-switch method used by keyboard navigation so search cleanup and scroll adjustment remain consistent.

**Acceptance Criteria**:

- [ ] Clicking visible tab N selects that visible tab after scrolling.
- [ ] Clicking border/title area does not select an unrelated tab.
- [ ] Mouse behavior remains disabled while modal overlays are visible.

---

## 12R.7: Complete Popup and Search Hardening

**Problem**: Shared popup clamping was started, but search still has a duplicate `centered_rect`, fixed table widths, and Unicode-unsafe byte slicing.

**Files**:

- `crates/slapper/src/tui/components/popup.rs`
- `crates/slapper/src/tui/search.rs`
- `crates/slapper/src/tui/ui.rs`

**Tasks**:

- [ ] Replace `search.rs` local `centered_rect` with the shared clamped helper.
- [ ] Keep clamped width/height at least `1` when the terminal area is non-zero.
- [ ] Avoid underflow when computing inner rects after clamping.
- [ ] Replace `&r.content[..40]` with character-based truncation.
- [ ] Make search result column widths degrade below 80 columns.
- [ ] Make command palette visible row count derive from actual popup height instead of hardcoded `14`.
- [ ] Verify HTTP options, command palette, search popup, and confirm popup at `60x20` and `40x20`.

**Acceptance Criteria**:

- [ ] Unicode search result content cannot panic rendering.
- [ ] Popups render coherently on small terminals.
- [ ] Selection remains visible in command palette at reduced height.

---

## 12R.8: Tests Required Before Marking Complete

**Unit Tests**:

- [ ] `Tab::visible_index()` matches `Tab::all()` positions.
- [ ] `Tab::from_visible_index()` round-trips visible indexes.
- [ ] `Tab::stable_id()` round-trips only for available tabs.
- [ ] `TabWindow::for_width()` always contains current tab.
- [ ] `TabWindow::for_width()` handles stale previous offsets.
- [ ] Bookmark APIs persist stable IDs.
- [ ] Session restore prefers stable IDs and handles unavailable IDs.
- [ ] Unicode truncation handles multi-byte text.

**Manual Verification Matrix**:

| Scenario | Expected Result |
|----------|-----------------|
| Base build, `80x24`, repeated `n` | Every available tab becomes active and visible |
| Base build, `60x20`, repeated `n` | Active tab remains visible despite reduced capacity |
| Base build, `40x20`, repeated `n` | UI degrades coherently; active tab is visible |
| Full feature build, late tabs | Late tabs highlight/render/status correctly |
| Mouse click after tab scroll | Clicked visible tab is selected |
| Bookmark Dashboard and restart | Dashboard remains bookmarked |
| Save session on Dashboard and restart | Dashboard restores as current tab |
| Save session with unavailable feature tab | Falls back cleanly |
| Search result with Unicode content | No panic; text truncates safely |
| Command palette at `60x20` | Overlay clamps and selected item remains visible |

**Verification Commands**:

```bash
cargo check --lib -p slapper
cargo test --lib -p slapper
cargo check --lib -p slapper --features rest-api,ai-integration
```

---

## Recommended Implementation Order

1. Fix `Tab::from_stable_id()` availability filtering.
2. Fix `TabWindow::for_width()` so the active tab is always inside the window.
3. Remove the hardcoded width from tab-scroll adjustment.
4. Convert bookmarks to stable IDs end to end.
5. Repair session capture/restore around stable IDs and legacy migration.
6. Tighten mouse hit-testing.
7. Complete popup/search hardening.
8. Add/repair tests.
9. Run the verification commands and manual matrix.

---

## Files of Interest

| Path | Why It Matters |
|------|----------------|
| `crates/slapper/src/tui/tabs/mod.rs` | `Tab`, stable IDs, visible indexes, `TabWindow` |
| `crates/slapper/src/tui/app/navigation.rs` | Keyboard tab navigation and scroll adjustment |
| `crates/slapper/src/tui/app/runner.rs` | Event loop, mouse tab hit-testing, bookmark shortcut |
| `crates/slapper/src/tui/app/mod.rs` | App state, bookmark storage, restored tab state |
| `crates/slapper/src/tui/session.rs` | Stable session persistence and legacy migration |
| `crates/slapper/src/tui/ui.rs` | Tab rendering, command/search/HTTP popups |
| `crates/slapper/src/tui/components/popup.rs` | Shared popup clamping helper |
| `crates/slapper/src/tui/search.rs` | Search popup layout and Unicode-safe truncation |

---

## Historical Completed Work

Previous waves are not tracked in detail here anymore. They are considered complete unless a regression is discovered:

- Agent alert fatigue and handler restoration
- TUI event loop order and channel draining
- Dashboard sparkline and portfolio snapshot work
- TUI theme migration and FocusArea standardization
- Initial Phase 12 scaffolding for tab stable IDs and `TabWindow`
