# Slapper Codebase and TUI Improvement Plan

## Status: IN PROGRESS 🚧

## Completed Phases

### Phase 16: TUI Visual Feedback & Focus Fixes ✅

**Date completed**: Phase 16

**Background**: Users have reported that it's difficult to tell which checkbox or item is currently selected for interaction (checking/unchecking) in certain tabs like the Recon tab. Investigation revealed that the Recon tab incorrectly renders ALL checkboxes with a focus style when the Options area is focused, and the visual feedback for focused items is generally too subtle.

**Tasks completed**:

1. **Improve Checkbox Visual Feedback** ✅
   - File: `crates/slapper/src/tui/components/selector.rs`
   - Component: `Checkbox`
   - Implementation: Add `> ` prefix when focused, use `Modifier::BOLD` and `tc!(focus_input)` color

2. **Improve Selector Visual Feedback** ✅
   - File: `crates/slapper/src/tui/components/selector.rs`
   - Component: `Selector`
   - Implementation: Add `>` prefix when focused, use `Modifier::BOLD` and `tc!(focus_input)` color

3. **Fix Recon Tab Focus Logic** ✅
   - File: `crates/slapper/src/tui/tabs/recon.rs`
   - Changed from `is_options_focused` (area-wide) to `is_options_focused && cb.focused` (item-specific)

4. **Audit Other Tabs** ✅
   - WafTab, ProxyTab, ReportTab all correctly manage focus state

5. **Standardize InputField Focus** ✅
   - InputField already had proper focus styling with `tc!(focus_input)` and `BOLD`

**Verification**: All 146 TUI tests pass.

---

## Current Status

Phase 16 completed. No further phases are currently defined in this plan.

---

## Testing Commands

```bash
cargo check --lib -p slapper
cargo test --lib -p slapper
cargo clippy --lib -p slapper
```