# Slapper TUI Plan

**Date**: 2026-04-30
**Status**: All Phase 13 items completed and verified. This file now serves as historical reference.
**Priority**: Historical

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

## Phase 13 Completion Summary (2026-04-30)

All Phase 13 items verified complete:

- **13.1**: `TabWindow::for_width` uses actual tab label widths (greedy algorithm). Added `visible_tab_spans()` for render-aware mouse hit-testing.
- **13.2**: Mouse hit-testing uses `visible_tab_spans()` to match rendered positions.
- **13.3**: Tab labels show shortcuts [1]-[0] for tabs 1-10, names only for tabs 11+.
- **13.4**: `handle_left()`/`handle_right()` use `is_at_left_edge()`/`is_at_right_edge()` edge detection without fallback tab switching.
- **13.5**: 9 render tests pass covering terminal sizes 30, 40, 60, 80, 120 widths.
- **13.6**: `visible_results_height()` bounds by actual results count.
- **13.7**: Status bar and breadcrumb use `Paragraph` widgets with proper overflow handling.

Verification commands:

```bash
cargo check --lib -p slapper
cargo test --lib -p slapper
cargo clippy --lib -p slapper
```

Known pre-existing issue (non-blocking):

```text
cargo check --lib -p slapper --features rest-api,ai-integration
error: captured variable cannot escape `FnMut` closure body
   --> crates/slapper/src/agent/mod.rs:470:26
```

---

## Deferred Non-TUI Work

These are intentionally out of scope:

- Fixing `agent/mod.rs:470` for `rest-api,ai-integration`.
- Refactoring autonomous agent internals.
- Adding new scan features.
- Reworking the broader theme system beyond what is needed for visual correctness.

---

## Bug Fix Applied During Review

**Issue**: NSE api.rs had borrow checker error where `target` and `script` were moved into closure then used after.

**Fix**: Clone variables before moving into `spawn_blocking` closure.

```rust
// Before (broken):
let (output, errors, success) = tokio::task::spawn_blocking(move || {
    let mut executor = NseExecutor::with_target(&target)  // target moved
    ...
});
let results = NseResults { target, script, ... };  // ERROR: target used after move

// After (fixed):
let target_clone = target.clone();
let script_clone = script.clone();
let (output, errors, success) = tokio::task::spawn_blocking(move || {
    let mut executor = NseExecutor::with_target(&target_clone)  // clone moved
    ...
});
let results = NseResults { target, script, ... };  // OK: original values used
```
