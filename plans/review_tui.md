# TUI Architecture Review

**Document:** architecture/tui.md
**Reviewed:** 2026-05-31
**Accuracy:** Medium
**Lines Reviewed:** 1715

## Verified Claims

- [Tab count = 28]: Tab enum has 28 variants (Recon=0 through Vuln=27) (`crates/slapper/src/tui/tabs/mod.rs:80-109`)
- [Tab enum definition]: All 28 variants match documented names and discriminants (`crates/slapper/src/tui/tabs/mod.rs:80-109`)
- [Tab titles]: All 28 tab titles match documented names (`crates/slapper/src/tui/tabs/mod.rs:112-143`)
- [Tab descriptions]: All 28 tab descriptions match documented purposes (`crates/slapper/src/tui/tabs/mod.rs:178-209`)
- [Tab traits exist]: `TabState`, `TabInput`, `TabRender` traits all exist (`crates/slapper/src/tui/tabs/mod.rs:830-887`)
- [TabState methods]: `state()`, `progress()`, `reset()`, `set_error()` all exist (`crates/slapper/src/tui/tabs/mod.rs:830-838`)
- [TabInput methods]: All documented methods exist plus additional ones (`crates/slapper/src/tui/tabs/mod.rs:849-887`)
- [TabRender methods]: `render()`, `render_overlays()`, `breadcrumb()` all exist (`crates/slapper/src/tui/tabs/mod.rs:840-846`)
- [App struct location]: `App` struct in `app/mod.rs` (`crates/slapper/src/tui/app/mod.rs:43-117`)
- [App struct holds all tabs]: All 28 tab fields present in App struct (`crates/slapper/src/tui/app/mod.rs:50-95`)
- [KeyHandler location]: `KeyHandler` struct in `app/key_handler.rs` (`crates/slapper/src/tui/app/key_handler.rs:10`)
- [KeyHandler priority-based]: Key processing follows pending combos -> overlays -> global -> mode (`crates/slapper/src/tui/app/key_handler.rs:17-43`)
- [TabDispatcher location]: `TabDispatcher` enum in `app/dispatch.rs` (`crates/slapper/src/tui/app/dispatch.rs:5-8`)
- [TabDispatcher routes input]: Routes to current tab via `TabInput` trait (`crates/slapper/src/tui/app/dispatch.rs:19-50+`)
- [state_update.rs location]: Async task result handling in `app/state_update.rs` (`crates/slapper/src/tui/app/state_update.rs:1-52`)
- [task_management.rs location]: `TabTaskConfigSource` trait in `app/task_management.rs` (`crates/slapper/src/tui/app/task_management.rs:1-9`)
- [task_runtime.rs location]: Task lifecycle management in `app/task_runtime.rs` (`crates/slapper/src/tui/app/task_runtime.rs:1-142`)
- [All 28 tab files exist]: All documented tab files confirmed present in `tabs/` directory
- [Tab file list complete]: All tab files match the documented table (recon.rs through settings/main.rs)
- [Component files exist]: InputField, InputGroup, Selector, Checkbox, RadioGroup, ProgressGauge, ScrollableText, Popup all confirmed
- [Worker files exist]: All 7 worker files confirmed (api.rs, fuzzer.rs, network.rs, recon.rs, runner.rs, scanner.rs, security.rs)
- [TaskConfig enum]: `TaskConfig` enum exists in `workers/runner.rs` with all documented variants (`crates/slapper/src/tui/workers/runner.rs:7-100+`)
- [TaskResult enum]: `TaskResult` enum exists in `workers/runner.rs` (`crates/slapper/src/tui/workers/runner.rs`)
- [TaskRunner exists]: `TaskRunner` struct in `workers/runner.rs` (`crates/slapper/src/tui/workers/runner.rs`)
- [Communication flow]: Tab -> TaskConfig -> spawn_task -> TaskRunner -> progress_rx/result_rx -> App verified (`crates/slapper/src/tui/app/task_runtime.rs:53-141`)
- [SharedHistory type]: `pub type SharedHistory = Arc<Mutex<HistoryTab>>` exists (`crates/slapper/src/tui/state/mod.rs:7`)
- [ThemeManager location]: `ThemeManager` in `theme.rs` (`crates/slapper/src/tui/theme.rs:178-181`)
- [ThemeManager holds themes]: Uses `FxHashMap<String, Theme>` for themes (`crates/slapper/src/tui/theme.rs:179`)
- [Dark/light themes]: Both `dark_theme()` and `light_theme()` functions exist (`crates/slapper/src/tui/theme.rs:60,97`)
- [tc! macro]: `tc!` macro exists for theme color access (`crates/slapper/src/tui/theme.rs:271-276`)
- [SessionManager location]: `SessionManager` in `session.rs` (`crates/slapper/src/tui/session.rs:66-68`)
- [Auto-save default 30s]: `auto_save_interval_secs: 30` confirmed (`crates/slapper/src/tui/session.rs:45`)
- [Quick-save on exit]: `save_quick()` called in runner.rs on exit (`crates/slapper/src/tui/app/runner.rs:61-63`)
- [Session restores theme]: `restore_session` calls `set_theme(&state.theme_name)` (`crates/slapper/src/tui/session.rs:166`)
- [Entry point]: `handle_no_command()` in `commands/handlers/mod.rs` calls `tui::run()` (`crates/slapper/src/commands/handlers/mod.rs:197-199`)
- [No --tui flag]: Correctly documented that TUI launches automatically when no subcommand and stdout is terminal
- [Key bindings verified]:
  - `Ctrl+C` interrupt/quit (`key_handler.rs:47,195-201`)
  - `Ctrl+P` command palette (`key_handler.rs:65`)
  - `Ctrl+X` quick switch (`key_handler.rs:48-51`)
  - `Ctrl+F` global search (`key_handler.rs:66`)
  - `Ctrl+T` toggle theme (`key_handler.rs:68`)
  - `Ctrl+Z` pause/resume (`key_handler.rs:67`)
  - `Ctrl+Y` resume/copy (`key_handler.rs:78-86`)
  - `Space` toggle help (`key_handler.rs:110`)
  - `hjkl` navigation (`key_handler.rs:119-122`)
  - `i` enter insert mode (`key_handler.rs:108`)
  - `Esc` return to normal mode (`key_handler.rs:63`)
  - `q` quit when no active task (`key_handler.rs:109,248-252`)
  - `g/G` go to top/bottom (`key_handler.rs:124-125`)
  - `n/N` next/prev tab (`key_handler.rs:128-129`)
  - `p` previous tab (`key_handler.rs:130`)
  - `e` export results (`key_handler.rs:138`)
  - `s` save settings (`key_handler.rs:136`)
- [Architecture diagram flow]: EventStream -> KeyHandler -> App method -> TabDispatcher -> TabInput -> TaskRunner confirmed
- [FxHashMap in app/mod.rs]: `bookmarks: FxHashSet<String>` confirmed (`crates/slapper/src/tui/app/mod.rs:110`)
- [FxHashMap in bookmarks.rs]: `FxHashSet` usage confirmed (`crates/slapper/src/tui/app/bookmarks.rs:2,4,13,17`)
- [FxHashMap in help_config.rs]: `StaticHelpData.sections: FxHashMap<Tab, HelpSection>` confirmed (`crates/slapper/src/tui/app/help_config.rs:7-8`)
- [FxHashMap in theme.rs]: `ThemeManager.themes: FxHashMap<String, Theme>` confirmed (`crates/slapper/src/tui/theme.rs:179`)
- [FxHashMap in dashboard.rs]: `PortfolioSnapshot.findings_by_severity: FxHashMap<String, usize>` confirmed (`crates/slapper/src/tui/tabs/dashboard.rs:18`)
- [Tab dispatch via enum match]: `Tab::as_tab_input()` uses exhaustive enum match, not HashMap (`crates/slapper/src/tui/tabs/mod.rs:757-812`)
- [AppState enum]: `Idle`, `Running`, `Completed`, `Error(String)` variants confirmed (`crates/slapper/src/tui/tabs/mod.rs:822-827`)
- [TabError enum]: All 7 variants confirmed: Network, Auth, Config, Resource, Target, Internal, Unknown (`crates/slapper/src/tui/app/tab_error.rs:4-12`)
- [TabError::is_recoverable()]: Method exists and checks Network, Auth, Resource (`crates/slapper/src/tui/app/tab_error.rs:27-32`)
- [OverlayType enum]: ConfirmPopup, CommandPalette, QuickSwitch, Search, HttpOptions, Help confirmed (`crates/slapper/src/tui/app/mod.rs:803-811`)
- [PendingAction enum]: ResetTab, SaveSettings, DeleteHistoryEntry, ClearHistory confirmed (`crates/slapper/src/tui/app/confirmation.rs:4-9`)
- [InputMode enum]: Normal, Insert confirmed (`crates/slapper/src/tui/app/input.rs:1-6`)
- [runner.rs event loop]: Uses crossterm EventStream with key/mouse/paste handling (`crates/slapper/src/tui/app/runner.rs:165-222`)
- [ui.rs draw function]: `draw_tabs()`, `draw_breadcrumb()`, `draw_content()`, `draw_status_bar()` confirmed (`crates/slapper/src/tui/ui.rs:17-34`)

## Discrepancies

- [Documented as "app/ has 7 files"]: Document lists only 7 files in `app/` table but actual directory has 18 files. Missing: `bookmarks.rs`, `command.rs`, `confirmation.rs`, `error.rs`, `export.rs`, `help_config.rs`, `input.rs`, `navigation.rs`, `notifications.rs`, `options.rs`, `tab_error.rs` (`crates/slapper/src/tui/app/`)

- [Documented as "7 components"]: Document lists 7 components but `components/` directory has 12 files. Missing: `empty_state.rs`, `help_bar.rs`, `http_options.rs`, `notifications.rs`, `palette.rs`, `search_popup.rs` (`crates/slapper/src/tui/components/`)

- [SharedHistory uses std::sync::Mutex]: Document shows `Arc<Mutex<HistoryTab>>` without specifying mutex type. Actual implementation uses `parking_lot::Mutex`, not `std::sync::Mutex` (`crates/slapper/src/tui/state/mod.rs:4`)

- [Session path "~/.slapper/sessions/"]: Document states sessions are saved to `~/.slapper/sessions/`. Actual implementation uses `directories::ProjectDirs::from("com", "slapper", "slapper")` which resolves to platform-specific paths (e.g., `~/.local/share/slapper/sessions/` on Linux). The `~/.slapper/sessions/` path is only the fallback (`crates/slapper/src/tui/session.rs:53-57`)

- [ThemeColors "30+ color fields"]: Document claims "30+ color fields" but `ThemeColors` struct has exactly 29 fields (`crates/slapper/src/tui/theme.rs:23-52`)

- [HelpManager.sections]: Document states `help.rs - HelpManager.sections` uses FxHashMap. Actual `HelpManager` struct has `content: HelpContent` field, and `HelpContent` has `sections: FxHashMap<Tab, HelpSection>`. The field path is `HelpManager.content.sections`, not `HelpManager.sections` (`crates/slapper/src/tui/help.rs:208-211`)

- [TabState trait missing is_running()]: Document lists `TabState` methods as `state()`, `progress()`, `reset()`, `set_error()` but omits `is_running()` which has a default implementation (`crates/slapper/src/tui/tabs/mod.rs:833-835`)

- [TabInput trait incomplete]: Document lists only `handle_focus_next()`, `handle_char()`, `handle_enter()` etc. but the actual trait has 25+ methods including `handle_delete()`, `handle_paste()`, `handle_copy()`, `handle_word_forward()`, `handle_word_backward()`, `handle_home()`, `handle_end()`, `handle_top()`, `handle_bottom()`, `handle_autocomplete()`, `handle_search()`, `is_input_focused()`, `is_at_left_edge()`, `is_at_right_edge()`, `stop()`, `page_up()`, `page_down()` (`crates/slapper/src/tui/tabs/mod.rs:849-887`)

- [search_backup type]: Document doesn't mention `search_backup` field. Actual type is `Option<VecDeque<HistoryEntry>>`, not a simple Vec (`crates/slapper/src/tui/app/mod.rs:79`)

- [Documented workers table incomplete]: Document lists 7 workers but workers/ directory has 8 files (7 workers + mod.rs). The `mod.rs` is not listed (`crates/slapper/src/tui/workers/`)

- [Missing modules from Core Components section]: Document doesn't mention `search.rs` (GlobalSearch), `ui.rs` (draw functions), or `utils/` directory (clipboard, fuzzy matching) which are part of the TUI core (`crates/slapper/src/tui/`)

- [auth.rs tab not documented]: `tabs/auth.rs` file exists with `AuthTab` struct implementing `TabInput`/`TabRender`/`TabState`, but is NOT listed in the 28 tabs table and is NOT in the `Tab` enum. This appears to be a standalone authentication testing tab that may be unused or experimental (`crates/slapper/src/tui/tabs/auth.rs`)

## Bugs Found

- [No bugs in architecture document]: The document is descriptive and does not contain code bugs. The "Bug Patterns to Avoid" section correctly identifies real patterns found and fixed in the codebase.

## Improvement Opportunities

- [Document app/ module completeness]: The `app/` table should list all 18 files, not just 7. Missing files include important modules: `tab_error.rs` (TabError enum), `confirmation.rs` (PendingAction), `input.rs` (InputMode), `navigation.rs` (tab switching, search, help toggle), `bookmarks.rs`, `command.rs` (command palette), `export.rs`, `error.rs`, `help_config.rs`, `notifications.rs`, `options.rs` (priority: high)

- [Document components/ completeness]: The components table should list all 12 files. Missing components: `empty_state.rs` (empty state rendering), `help_bar.rs` (context help bar), `http_options.rs` (HTTP options popup), `notifications.rs` (notification display), `palette.rs` (command palette rendering), `search_popup.rs` (search popup rendering) (priority: high)

- [Document utils/ and search.rs]: The `utils/` directory (clipboard.rs, fuzzy.rs) and `search.rs` module (GlobalSearch, SearchResult) are not documented but are part of the TUI core. Add a section for these (priority: medium)

- [Document ui.rs]: The `ui.rs` module contains the `draw()` function and all UI rendering logic (draw_tabs, draw_breadcrumb, draw_content, draw_status_bar, draw_command_palette, draw_search_popup, draw_http_options_popup, draw_quick_switch). This is a critical module that should be documented (priority: high)

- [Clarify SharedHistory mutex type]: Document should specify that `SharedHistory` uses `parking_lot::Mutex` not `std::sync::Mutex` for clarity (priority: low)

- [Fix session path documentation]: Document should clarify that the session path uses `directories::ProjectDirs` with platform-specific resolution, and `~/.slapper/sessions/` is only the fallback (priority: medium)

- [Fix ThemeColors count]: Change "30+ color fields" to "29 color fields" for accuracy (priority: low)

- [Fix HelpManager field path]: Change `help.rs - HelpManager.sections` to `help.rs - HelpManager.content.sections` or better yet, document the full chain: `HelpManager` -> `HelpContent` -> `sections: FxHashMap<Tab, HelpSection>` (priority: low)

- [Document is_running() in TabState]: Add `is_running()` to the TabState trait documentation with note that it has a default implementation (priority: low)

- [Document full TabInput interface]: The TabInput trait documentation should list all 25+ methods, not just a few examples. This is important for developers adding new tabs (priority: high)

- [Document auth.rs status]: Add a note explaining that `auth.rs` exists as a tab file but is not part of the `Tab` enum. Clarify whether it's experimental, deprecated, or planned for future integration (priority: medium)

- [Document OverlayType and PendingAction]: The document mentions overlays but doesn't document the `OverlayType` enum (6 variants) or `PendingAction` enum (4 variants) which are important for understanding the overlay system (priority: medium)

- [Document InputMode]: The `InputMode` enum (Normal, Insert) is central to the key handling system but not documented (priority: medium)

- [Document AppState enum]: The `AppState` enum (Idle, Running, Completed, Error) is used throughout but only implicitly documented (priority: low)

- [Add missing cross-references]: Document should cross-reference `architecture/config.md` for session config details and `architecture/output.md` for export functionality (priority: low)

## Stale Items

- [Session fix logs are extensive]: Lines 548-1715 contain detailed session fix logs from multiple audit sessions (2026-05-30 through 2026-06-10). These are valuable historical records but make the document very long (1167 lines of fix logs out of 1715 total). Consider moving these to a separate `TUI_AUDIT_LOG.md` file or summarizing them into a "Known Issues Fixed" section (priority: medium)

- [Duplicate session entries]: There are multiple "Session Fixes (2026-05-31)" and "Session Fixes (2026-06-10)" sections. These could be consolidated to reduce redundancy (priority: low)

- [Bug pattern examples length]: The "Bug Patterns to Avoid" section (lines 193-750) is very long at 557 lines. While valuable, it could be condensed or moved to a separate reference document, keeping only the most common patterns in the main architecture doc (priority: low)

## Statistics Verification

| Claim | Document Value | Actual Value | Match |
|-------|---------------|--------------|-------|
| Tab count | 28 | 28 (Tab enum variants) | YES |
| Tab files | 28 | 29 (.rs files, includes auth.rs) | PARTIAL |
| Component files | 7 listed | 12 actual files | NO - underdocumented |
| App module files | 7 listed | 18 actual files | NO - underdocumented |
| Worker files | 7 listed | 7 + mod.rs = 8 | PARTIAL |
| Theme color fields | "30+" | 29 | NO - slight overcount |
| TabError variants | Not listed | 7 (Network, Auth, Config, Resource, Target, Internal, Unknown) | N/A |
| OverlayType variants | Not listed | 6 (ConfirmPopup, CommandPalette, QuickSwitch, Search, HttpOptions, Help) | N/A |
| PendingAction variants | Not listed | 4 (ResetTab, SaveSettings, DeleteHistoryEntry, ClearHistory) | N/A |
| AppState variants | Not listed | 4 (Idle, Running, Completed, Error) | N/A |
| Auto-save interval | 30 seconds | 30 seconds | YES |
| Key bindings | 18 listed | 18+ verified | YES |

## Cross-Reference Validation

- [architecture/cli_commands.md]: Referenced but not verified in this review
- [architecture/config.md]: Referenced but not verified in this review
- [architecture/output.md]: Referenced but not verified in this review

## Summary

The TUI architecture document is fundamentally accurate in its core claims about the 28-tab system, key bindings, component structure, and architectural flow. The main areas for improvement are:

1. **Completeness**: The `app/`, `components/`, and `utils/` directories are significantly underdocumented (18 files listed as 7, 12 listed as 7, and 0 listed respectively)
2. **Precision**: A few specific claims are slightly inaccurate (session path, theme color count, HelpManager field path)
3. **Organization**: The document is very long (1715 lines) with 67% being fix logs. Consider restructuring to separate architecture documentation from audit history
4. **Missing modules**: `ui.rs`, `search.rs`, `utils/`, and several `app/` modules are not documented despite being core to the TUI

Overall accuracy rating: **Medium** - Core architectural claims are correct, but documentation coverage of the full module structure is incomplete.
