# Slapper Codebase and TUI Improvement Plan

## Status: IN PROGRESS

## Completed Phases

### Phase 1-5: TUI Architecture & Improvements ✅
- Architectural refactoring (file splitting, module extraction)
- UX/Usability improvements (focus indicators, mode indicator, quick switch)
- Styling/Theming (theme customization in Settings)
- Error handling (notifications for user-visible errors)
- Help system extraction (help data moved to help_config.rs)

### Phase 6: TUI Architecture & Performance ✅
- **Lazy Loading Tabs**: Added `tabs: HashMap<Tab, Box<dyn TabInput>>` field to App struct for lazy tab instantiation infrastructure
- **TabDispatcher Elimination**: Removed TabDispatcher boilerplate; tabs now use direct dynamic dispatch
- Fixed tab window calculation tests to match actual algorithm behavior

### Phase 7: State Management & Error Handling ✅
- **Typed Errors**: Migrated all tabs from `error_message: Option<String>` to `error: Option<TabError>`
- `TabError` enum provides structured error categories: Network, Auth, Config, Resource, Target, Internal, Unknown
- Error formatting moved to `render()` methods - errors now display via `error.message()`
- Recoverable error detection via `TabError::is_recoverable()` method
- 5 tabs updated: workflow, vuln, storage, plugin, nse

### Phase 8: Broader Codebase Modernization ✅
- **Async TUI Event Loop Integration**: Implemented using `tokio::select!` pattern
- Combined `crossterm::event::EventStream` for terminal events with Tokio streams
- Non-blocking event polling with proper timeout handling

### Phase 9: UI Component Library Enhancements ✅
- **FormBuilder Component**: Added in `tui/components/input.rs`
- Takes collection of `InputField`s and automatically calculates vertical layout chunks
- Supports FieldVariant enum for Input, Checkbox, and Selector types
- SettingsTab and WafTab refactoring planned for future iteration

## Actionable Tasks (Next Phases)

### Phase 10: Testing Rigor
- [ ] **Visual Regression Testing**: Enhance `tui::tabs::tests`.
  - **Context:** Current tests verify logic but cannot detect if layout changes push widgets off-screen.
  - **Action:** Add integration tests using `ratatui::backend::TestBackend`.
  - **Action:** Simulate keystrokes and assert against the exact 2D character buffer rendered to the terminal (e.g., verify that `Buffer::cell(x, y)` contains the expected text when an error is triggered).

---

## Deferred Items (Low Priority - Intentionally Deferred)

### Derive Help from Tab State
- Would require updating `key_hints()` trait method across all tabs
- Current hardcoded help in `help_config.rs` is functional
- **Status**: Low priority, deferred indefinitely

### Command Palette Styling
- Low priority - existing styling is adequate
- **Status**: Low priority, deferred indefinitely

---

## Testing Commands

```bash
cargo check --lib -p slapper
cargo test --lib -p slapper
cargo clippy --lib -p slapper
```