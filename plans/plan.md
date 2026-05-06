# Slapper Codebase and TUI Improvement Plan

## Status: IN PROGRESS 🚧

## Completed Phases

### Phase 1-16: TUI Visual Feedback & Focus Fixes ✅
- Phase 16 completed: Improved checkbox/selector visual feedback, fixed Recon tab focus logic
- Verified with 146 TUI tests passing

---

## Phase 17: TUI Visual Feedback & Focus Fixes 🚧

### Background
Users have reported that it's difficult to tell which checkbox or item is currently selected for interaction (checking/unchecking) in certain tabs like the Recon tab. Investigation revealed that the Recon tab incorrectly renders ALL checkboxes with a focus style when the Options area is focused, and the visual feedback for focused items is generally too subtle.

### Tasks

#### 1. Improve Checkbox Visual Feedback ✅
- **File**: `crates/slapper/src/tui/components/selector.rs`
- **Component**: `Checkbox`
- **Action**: Update `render_with_focus` to provide clear visual feedback for focused state.
- **Implementation Detail**:
    - Add a `> ` prefix when `focused` is true (and `  ` when false to maintain alignment).
    - Add `Modifier::BOLD` to the style when `focused` is true.
    - Change text color to `tc!(focus_input)` when focused.

#### 2. Improve Selector Visual Feedback ✅
- **File**: `crates/slapper/src/tui/components/selector.rs`
- **Component**: `Selector`
- **Action**: Update `render` method to make it clearer when the selector itself is focused (before expansion).
- **Implementation Detail**:
    - If `focused` is true, add a `>` prefix to the displayed text (e.g., `> [Value] ▼`).
    - Use `Modifier::BOLD` and `tc!(focus_input)` color for the displayed text when `focused` is true.

#### 3. Fix Recon Tab Focus Logic ✅
- **File**: `crates/slapper/src/tui/tabs/recon.rs`
- **Action**: Update `render` method to only pass `focused = true` to the *specifically* focused checkbox index.
- **Implementation Detail**:
    - Changed from passing `is_options_focused` (area-wide focus) to all 16 checkboxes.
    - Now passes `is_options_focused && cb.focused` so only the item that will be toggled by `Enter` is visually highlighted.
- **Verification**: Checkbox-specific focus rendering confirmed.

#### 4. Audit and Fix Other Tabs ✅
- **Action**: Review other tabs for similar "area-wide" focus rendering patterns.
- **Target Tabs**:
    - `WafTab`: Already correct (uses `i == self.focused_checkbox_index`).
    - `ProxyTab`: Uses `Selector`, focused state correctly managed via `is_focused()` method.
    - `ReportTab`: Uses `Selector`, focused state correctly managed via clone with set `focused` field.

#### 5. Standardize InputField Focus ✅
- **File**: `crates/slapper/src/tui/components/input.rs`
- **Action**: Ensure `InputField` rendering matches the new bold/color patterns for consistency.
- **Verification**: InputField already uses `tc!(focus_input)` color and `Modifier::BOLD` for focused state - already standardized.

### Verification
- Run `cargo test --lib -p slapper tui::` to ensure no regressions in existing tests.
- All 146 TUI tests pass.
- Manually verify in TUI that the `>` indicator appears correctly when navigating checkboxes and selectors.
- Verified that only one checkbox in the Recon tab is highlighted at a time.

---

## Testing Commands

```bash
cargo check --lib -p slapper
cargo test --lib -p slapper
cargo clippy --lib -p slapper
```
