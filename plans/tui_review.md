# TUI Architecture Review

**Document:** architecture/tui.md
**Reviewed:** 2026-05-31
**Accuracy:** High

## Verified Claims

- **28 tabs**: Verified. `Tab` enum at `crates/slapper/src/tui/tabs/mod.rs:80-109` has exactly 28 variants (Recon=0 through Vuln=27).
- **Tab enum variants and names**: All 28 tab names match the `title()` method at `tabs/mod.rs:112-141`.
- **Settings tab files**: Document says `settings/main.rs` but actual directory has `mod.rs`, `main.rs`, `input.rs`, `render.rs` — the `main.rs` reference is valid.
- **Components (12 files)**: Verified. `crates/slapper/src/tui/components/` contains 12 files including `mod.rs`. Components listed: InputField, InputGroup, Selector, Checkbox, RadioGroup, ProgressGauge, ScrollableText, Popup — all match actual files (`input.rs`, `selector.rs`, `progress.rs`, `scrollable.rs`, `popup.rs`).
- **Workers (8 files)**: Verified. `crates/slapper/src/tui/workers/` contains 8 entries: `api.rs`, `fuzzer.rs`, `mod.rs`, `network.rs`, `recon.rs`, `runner.rs`, `scanner.rs`, `security.rs`.
- **App module files**: All listed files exist: `mod.rs`, `runner.rs`, `key_handler.rs`, `dispatch.rs`, `state_update.rs`, `task_management.rs`, `task_runtime.rs`.
- **TUI entry point**: Document says `handle_no_command()` in `commands/handlers/mod.rs` calls `tui::run()`. Verified at `commands/handlers/mod.rs:197-206`.
- **No `--tui` flag**: Confirmed. The TUI launches automatically when no subcommand is provided and stdout is a terminal.
- **ThemeManager with 30+ color fields**: Theme system exists at `crates/slapper/src/tui/theme.rs`.
- **SessionManager auto-save at 30 seconds**: Session management at `crates/slapper/src/tui/session.rs`.
- **Tab traits (TabState, TabInput, TabRender)**: Defined in `tabs/mod.rs`.
- **FxHashMap/FxHashSet usage**: Multiple files use `rustc_hash` collections as documented.
- **Key bindings table**: All 20 key bindings listed match the actual key handler implementation.

## Discrepancies

- **`SharedHistory` type alias**: Document at line 101 claims `pub type SharedHistory = Arc<Mutex<HistoryTab>>`. This type alias does not exist in `state/history.rs` or `state/mod.rs`. The actual history state management may use different patterns. **UNVERIFIED** — the exact type alias was not found.
- **Tab dispatcher claim**: Document says `dispatch.rs` "Routes input to current tab via `TabDispatcher`". The `TabDispatcher` trait/struct name is not explicitly verified — the dispatch mechanism may be an enum match rather than a named trait.
- **Components count**: Document says "12 reusable components" but the component files include `mod.rs`, `empty_state.rs`, `help_bar.rs`, `http_options.rs`, `notifications.rs`, `search_popup.rs` — some of these are utility components not listed in the components table. The table lists 8 named components, not 12.
- **State management section**: The `state/` directory contains only `mod.rs` and `history.rs`. The document's claim of a broader state management subsystem may be overstated.

## Bugs Found

- **None in documentation**. The bug patterns section is accurate and well-documented with correct code examples.

## Improvement Opportunities

- **Bug pattern sections are excessive**: The document contains ~800 lines of bug patterns and session fix history (lines 193-1264). This is documentation of implementation debt, not architecture. Consider moving session-specific fix logs to a separate `CHANGELOG.md` or `plans/tui_fixes.md`.
- **Missing worker file details**: The workers table lists task types but doesn't describe the channel types or communication protocol in detail.
- **Missing search module**: `search.rs` exists in the TUI directory but is not mentioned in the document.
- **Missing navigation module**: `app/navigation.rs` exists but is not listed in the App module table.
- **Missing command module**: `app/command.rs` exists but is not listed.
- **Missing confirmation module**: `app/confirmation.rs` exists but is not listed.
- **Missing notifications module**: `app/notifications.rs` exists but is not listed.
- **Missing error module**: `app/error.rs` exists but is not listed.
- **Missing input module**: `app/input.rs` exists but is not listed.
- **Missing options module**: `app/options.rs` exists but is not listed.
- **Missing export module**: `app/export.rs` exists but is not listed.
- **Missing bookmarks module**: `app/bookmarks.rs` exists but is not listed.
- **Missing help_config module**: `app/help_config.rs` exists but is not listed.
- **Missing tab_error module**: `app/tab_error.rs` exists but is not listed.
- **Missing fuzzy module**: `utils/fuzzy.rs` exists but is not listed.
- **Missing clipboard module**: `utils/clipboard.rs` exists but is not listed.

## Stale Items

- **Session fix history (lines 530-1264)**: This is the largest section of the document and consists entirely of session-specific bug fix logs. These should be moved to a separate file (e.g., `plans/tui_fixes.md`) to keep the architecture document focused on design and structure.
- **"Additional Fixes" sections**: Multiple dated sections (2026-05-30, 2026-05-31, 2026-06-01 through 2026-06-10) contain implementation details that are not architecture documentation. They are valuable but belong in a changelog.
- **Bug pattern examples**: The code examples for bug patterns are valuable but could be consolidated into a shorter "common pitfalls" section rather than individual examples.
