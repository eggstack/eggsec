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

## Actionable Tasks (Next Phases)

### Phase 7: State Management & Error Handling
- [ ] **Typed Errors over Strings**: Upgrade `error_message` across all tabs.
  - **Context:** The recent standardization added `pub error_message: Option<String>`, but string-based errors strip contextual data that could be used for programmatic recovery.
  - **Action:** Change `error_message` to hold a structured error type (`Option<anyhow::Error>` or a custom `SlapperError` enum).
  - **Action:** Move the string formatting into the `render()` method.
  - **Action:** Explore auto-recovery mechanisms based on error type (e.g., auto-reconnect prompts for broken proxy connections).

### Phase 8: Broader Codebase Modernization
- [ ] **Async TUI Event Loop Integration**: Execute the async transition (referencing `TOKIO_MIGRATION_PLAN.md`).
  - **Context:** The TUI likely polls `progress_rx` and `result_rx` synchronously, which can lead to stuttering during heavy I/O.
  - **Action:** Implement an asynchronous event loop using `tokio::select!`.
  - **Action:** Combine `crossterm::event::EventStream` for terminal events with Tokio `mpsc::Receiver` streams for background workers.

### Phase 9: UI Component Library Enhancements
- [ ] **Form & Layout Builders**: Create higher-level compositional wrappers in `crates/slapper/src/tui/components/`.
  - **Context:** Complex tabs like `SettingsTab` and `WafTab` have massive `render()` methods filled with manual layout constraints and chunk splitting.
  - **Action:** Implement a `FormBuilder` component that takes a collection of `InputField`s and automatically calculates the vertical layout chunks.
  - **Action:** Refactor `SettingsTab` and `WafTab` to use these builders, standardizing form layout across the application.

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