# Slapper Codebase and TUI Improvement Plan

## Status: IN PROGRESS 🚧

## Completed Phases

### Phase 12R: TUI Corrections ✅

**Date merged**: Recent

**Summary**: Stable ID fixes, session capture/restore, popup and search hardening, mouse hit-testing edge cases.

**Commits merged**:
- Tab::from_stable_id() to check availability
- TabWindow::for_width() active-tab clamping
- Remove hardcoded width from tab-scroll adjustment
- Bookmark identity to use stable IDs
- Session capture/restore with stable IDs
- Mouse hit-testing edge cases
- Popup and search hardening

---

### Phase 11: Theme & FocusArea Migration ✅

**Date merged**: Recent

**Summary**: Theme color fixes for all tabs, FocusArea migration for scan, UI, and complex tabs.

**Components**:
- Theme migration groups 2a, 2b, 2c covering all major tabs
- FocusArea migration for scan tabs (load, scan_ports, scan_endpoints, fingerprint, waf_stress, resume)
- FocusArea migration for UI tabs (dashboard, settings, history, agent)
- FocusArea migration for complex tabs (proxy, packet)
- Consistent error reporting for remaining tabs

---

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

### FormBuilder Refactoring (merged via feature/waf-formbuilder-refactor) ✅

**Date merged**: Recent

**Summary**: Refactored multiple tabs to use FormBuilder for consistent layout.

**Changes**:
- WAF tab: Use FormBuilder for config section instead of manual Layout
- Settings tab: Use FormBuilder for all settings sections
- Auth tab: Use FormBuilder for inputs, error handling
- Fingerprint tab: Fix handle_up/down to properly scroll results
- History tab: Add 'd' and 'C' keybindings for delete/clear
- Integrations tab: Implement proper get_config() for all tracker types
- NSE tab: Fix handle_enter logic, remove redundant methods
- Storage tab: Edge detection fixes
- RadioGroup support added to FormBuilder

---

### Workstream Fixes (merged via various branches) ✅

**Date merged**: Recent

**Summary**: Various navigation and theme fixes from workstream branches.

**Changes**:
- h/l navigation semantics fixes
- Fuzz tests for left/right focus movement
- Help text helper function
- Direct color usage fixes

---

## Current Status

Phases 12R, 11, 16, FormBuilder refactoring, and workstream fixes all merged to master.

**Remaining on branches** (not merged due to conflicts/stale):
- phase-11/focusarea-* and phase-11/error-reporting: Had permission conflicts, needs manual resolution
- fix/auth-tab-component-standardization, fix/integrations-tab-navigation-state: Conflicts with current state

---

## Testing Commands

```bash
cargo check --lib -p slapper
cargo test --lib -p slapper
cargo clippy --lib -p slapper
```