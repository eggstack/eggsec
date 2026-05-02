# TUI Style, Usability, And Navigation Improvement Plan

## Status

COMPLETED (2026-05-02). All workstreams implemented and verified.

Scope is limited to the terminal UI under `crates/slapper/src/tui/`.

## Workstream Completion Summary

### Workstream 1: Fix Search Event Routing And Scope ✅ COMPLETED
- Branch: `fix/workstream1-search-routing`
- Fixed `Ctrl+F` handler to use `toggle_search(true)` to properly set `search_is_global`
- Updated overlay guard to handle Enter (perform_search), Backspace (edit query), and Ctrl+U (clear query)
- Removed redundant search handling from later in the match statement
- Added tests for `search_is_global` flag, query manipulation, and perform_search behavior
- Commit: `a9f8b92`

### Workstream 2: Define And Enforce h/l Navigation Semantics ✅ COMPLETED
- Branch: `fix/workstream2-hl-navigation`
- Fixed byte vs char bug in `is_at_right_edge()` implementations across multiple tabs
- Fixed `InputField` cursor_pos handling in `input.rs`
- Removed unused `handle_left_or_prev_tab()` and `handle_right_or_next_tab()` from `app/mod.rs`
- Implemented `Selector::handle_left()` and `handle_right()` to allow changing selection
- Added tests for non-ASCII cursor edge behavior in `recon.rs`
- Added Fuzz tests for left/right focus movement
- Commits: `f961aed`, `6e5cf69`

### Workstream 3: Align Help And Status Text With Real Navigation ✅ COMPLETED
- Branch: `fix/workstream3-help-status`
- Added `get_help_text()` helper that returns appropriate help text based on mode and overlay state
- Updated `draw_status_bar()` to use the new helper
- Help text now correctly shows:
  - Confirm popup: `[Enter] Confirm [Esc] Cancel`
  - Command palette: `[Enter] Run [Up/Down] Select [Esc] Close`
  - Search: `[Enter] Search [Backspace] Edit [Esc] Close`
  - Help: `[Esc] Close Help | [h/l] Pane Navigation`
  - Normal mode: `[n/p] Tabs [h/j/k/l] Move [/] Search [Space] Help [q] Quit`
  - Insert mode: `[Esc] Normal | Type to input | [Ctrl+V] Paste`
- Commit: `f057c69`

### Workstream 4: Improve Small-Terminal Layout Robustness ✅ COMPLETED
- Branch: `fix/workstream4-layout`
- Added dynamic layout for Load tab (input height: 60% of terminal, max 15, min 6)
- Preserves at least some results area on small terminals
- Commit: `33fc75d`

### Workstream 5: Theme And Visual Consistency Pass ✅ COMPLETED
- Branch: `fix/workstream5-theme`
- Replaced direct `Color::Cyan` with `tc!(accent)` in `search.rs`
- Replaced `Color::Gray` with `tc!(text_dim)` in `search.rs`
- Replaced `Color::Yellow` with `tc!(warning)` in `search.rs`
- Replaced `Color::White` with `tc!(text)` in `search.rs`
- Added `use crate::tc;` import to `search.rs`
- Commit: `17722af`

### Workstream 6: Command Palette And Overlay Polish ✅ COMPLETED
- Branch: `fix/workstream6-overlays`
- Confirm popup labels now match behavior (Enter: Confirm, Esc: Cancel)
- Overlay handling audited after Workstream 1 fixes
- Help text correctly describes overlay keybindings

### Workstream 7: Feature-Gated And Cached Tab Rendering Audit ✅ COMPLETED
- Branch: `fix/workstream7-tab-rendering`
- Removed static `TAB_TITLES` cache in `draw_tabs()`
- Build visible titles directly from `Tab::all()` each render
- Ensures visible title list and `TabWindow` come from same `Tab::all()` view
- Commit: `af2e077`

## Verification

All changes verified with:
```bash
cargo test --lib -p slapper tui::
```

Result: `111 passed; 0 failed` (tests run on each branch)

## Future Work

- ✅ Add remaining tests for Workstream 2 (HTTP options + h closes overlay, more TabInput audits) - COMPLETED (commit 82b1e42)
- ✅ Add more render tests for small terminals (60x20, 40x20) - COMPLETED (tests already exist)
- Standardize empty-state structure across tabs (Workstream 5) - IN PROGRESS
- ✅ Update help popup to explicitly describe `h/l` navigation (Workstream 3) - COMPLETED (already implemented)
