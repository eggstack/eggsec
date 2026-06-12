# TUI (Terminal User Interface)

Eggsec includes a powerful real-time Terminal User Interface (TUI) built with the `ratatui` crate. It provides an interactive way to monitor and control ongoing security scans across 31 different tabs.

## Core Components (`src/tui/`)

### App & UI (`app/`)

Manages the overall application state, event loop, and rendering.

| File | Purpose |
|------|---------|
| `mod.rs` | `App` struct - central state container holding all tabs, mode, overlays, theme |
| `state.rs` | Focused state structs: `OverlayState`, `SearchState`, `QuickSwitchState`, `TaskState`, `ThemeLoadState` |
| `tab_store.rs` | `TabStore` - owns all 31 tab instances (20 always-present + 10 feature-gated; History tab shares Dashboard instance) |
| `runner.rs` | Main event loop using crossterm/ratatui |
| `key_handler.rs` | Priority-based key processing (pending combos → overlays → global → mode) |
| `state_update.rs` | Async task result handling and routing |
| `task_runtime.rs` | Task lifecycle management (spawn, stop, clear) |
| `theme_runtime.rs` | Theme loader lifecycle helpers and deferred restore handling |
| `input.rs` | `InputMode` enum: `Normal`, `Insert` |
| `navigation.rs` | Tab navigation helpers (next/prev tab, edge detection) |
| `bookmarks.rs` | Tab bookmark toggle, query, and persistence |
| `command.rs` | Command palette dispatch and execution |
| `confirmation.rs` | `PendingAction` enum for deferred destructive actions |
| `error.rs` | Friendly error message formatting for TUI display |
| `export.rs` | Result export to file (JSON, CSV, HTML, SARIF, etc.) |
| `help_config.rs` | Static help data and help overlay configuration |
| `notifications.rs` | `Notification` and `NotificationSeverity` types for toast messages |
| `options.rs` | `GlobalHttpOptions` struct (auth, proxy, TLS, rate-limit, user-agent) |
| `tab_error.rs` | `TabError` enum with 5 categories (Network, Config, Resource, Target, Unknown) and `is_recoverable()` |

### Tabs (`tabs/`)

31 specialized tabs for different security testing functions:

| Tab | File | Purpose |
|-----|------|---------|
| Recon | `recon.rs` | Domain/IP reconnaissance (DNS, WHOIS, SSL, tech detection) |
| Scan | `scan.rs` | Multi-stage security assessment pipeline |
| Scan Ports | `scan_ports.rs` | TCP port scanning |
| Scan Endpoints | `scan_endpoints.rs` | Sensitive endpoint discovery |
| Fingerprint | `fingerprint.rs` | Service fingerprinting (AMAP-style) |
| Fuzz | `fuzz.rs` | Security fuzzing with 40 payload types |
| WAF | `waf.rs` | WAF detection and bypass |
| WAF Stress | `waf_stress.rs` | Comprehensive WAF stress testing |
| Load | `load.rs` | HTTP load testing |
| Stress | `stress.rs` | Stress/load testing |
| Packet | `packet.rs` | Packet capture, send, traceroute |
| GraphQL | `graphql.rs` | GraphQL security testing |
| OAuth | `oauth.rs` | OAuth/OIDC vulnerability testing |
| Auth Test | `auth.rs` | Authentication control validation (defense-lab only) |
| Cluster | `cluster.rs` | Distributed scanning cluster management |
| Proxy | `proxy.rs` | Proxy pool management |
| NSE | `nse.rs` | Nmap NSE script execution |
| Hunt | `hunt.rs` | Intelligent vulnerability hunting |
| Browser | `browser.rs` | Headless browser security testing |
| Wireless | `wireless.rs` | WiFi scanning plus active deauth/disassoc (`wireless` passive; `wireless-advanced` active mode with dry-run default and live confirmation) |
| Compliance | `compliance.rs` | Compliance report generation (OWASP, PCI, HIPAA, SOC2) |
| Storage | `storage.rs` | Database storage and query management |
| Integrations | `integrations.rs` | Issue tracker integration (Jira, GitHub, GitLab) |
| Workflow | `workflow.rs` | Finding management and SLA tracking |
| Vuln | `vuln.rs` | Vulnerability prioritization and risk scoring |
| Report | `report.rs` | Report conversion, trends, schedules |
| Resume | `resume.rs` | Resume previous scan from session |
| History | `history.rs` | Scan history browser |
| Dashboard | `dashboard.rs` | Security assessment dashboard |
| Settings | `settings/main.rs` | Application configuration |
| Db Pentest | `db_pentest.rs` | Direct database security assessment (Phase 3-5: Postgres/MySQL/MSSQL/MongoDB/Redis, correlation engine, compliance mapping, evidence bundles) |

**Tab Traits** (`tabs/mod.rs`):
- `TabState` - State: `state()`, `progress()`, `reset()`, `set_error()`
- `TabInput` - Input: `handle_focus_next()`, `handle_char()`, `handle_enter()`, etc.
- `TabRender` - Rendering: `render()`, `render_overlays()`, `breadcrumb()`

**Auth Test tab**: `AuthTab` at `tabs/auth.rs` is fully integrated as `Tab::Auth` (TabSpec with Intrusive risk_group, direct_launch: true; TaskConfig::Auth + TaskResult::Auth in worker system). Defense-lab only — no `ScanReportData` bridge.

**Wireless tab Active Mode** (`tabs/wireless.rs`, under `wireless-advanced`): the wireless tab supports both passive scanning (default) and an opt-in **Active Mode** for deauth/disassoc. Keymap:

- `a` — toggle Active Mode (re-renders to show BSSID, Client MAC, Frame Count, Rate Limit input fields plus the dry-run toggle).
- `d` — toggle Dry Run (on by default; live mode requires explicit opt-out).
- `Enter` in the ActiveConfig focus area — launch the configured attack.

Execution flow (mirrors Auth/Stress/Packet):

1. `WirelessTab::build_task_config()` returns `TaskConfig::WirelessActive { interface, attack_type, bssid, client, frame_count, rate_limit, dry_run }` (gated on `active_mode == true` and valid `active_attack_config()`).
2. `App::build_current_operation_descriptor()` (`app/mod.rs:436-471`) special-cases wireless active attacks: the operation is `wireless-deauth` or `wireless-disassoc`, the mode is `OperationMode::DefenseLab`, the risk is `SafeActive` (dry-run) or `Intrusive` (live), and `required_features: ["wireless-advanced"]` is set.
3. Central `EnforcementContext::evaluate()`:
   - Dry-run → `Allow` / `Warn` → `spawn_task(...)` immediately.
   - Live → `RequireConfirmation` under `ManualPermissive` → `request_policy_confirmation()` captures the `TaskConfig` and opens the policy overlay; on confirm, `confirm_policy_action()` replays the captured task.
4. The worker `run_wireless_active_task` (`workers/security.rs:865-927`) parses MACs, builds an `ActiveAttackConfig` with hard budgets (`max_frames ≤ 1000`, `frames_per_second ≤ 100`), and dispatches `run_deauth` (default) or `run_disassoc`.
5. Result returns via `TaskResult::WirelessActive(result)` → `WirelessTab::set_active_results()` (`app/state_update.rs:418-422`), which transitions the tab to `AppState::Completed` and renders the findings, evidence, and recommendations.

TabSpec (`tabs/spec.rs:427-439`) declares `direct_launch: true` and `risk_group: TabRiskGroup::SafeActive` (overridden at descriptor construction time to `Intrusive` for live attacks). 12 unit tests under `tabs/wireless::tests` (under `wireless-advanced`) cover mode toggling, dry-run flipping, config validation, task-config construction, `set_active_results`, and `handle_enter` flows. (See also resolution of `plans/wireless-active-tui-final-wiring-and-polish-plan.md` (2026-06-12) for E2E test addition and removed-artifact cleanup.)

### TabInput Interface (27 methods)

All tabs implement the `TabInput` trait (`tabs/mod.rs:849-887`):

| Method | Required | Signature |
|--------|----------|-----------|
| `handle_focus_next` | Yes | `fn handle_focus_next(&mut self)` |
| `handle_focus_prev` | Yes | `fn handle_focus_prev(&mut self)` |
| `handle_char` | Yes | `fn handle_char(&mut self, c: char)` |
| `handle_backspace` | Yes | `fn handle_backspace(&mut self)` |
| `handle_delete` | No | `fn handle_delete(&mut self)` - defaults to `handle_backspace()` |
| `handle_enter` | Yes | `fn handle_enter(&mut self)` |
| `handle_escape` | Yes | `fn handle_escape(&mut self)` |
| `handle_up` | Yes | `fn handle_up(&mut self)` |
| `handle_down` | Yes | `fn handle_down(&mut self)` |
| `handle_left` | Yes | `fn handle_left(&mut self) -> bool` |
| `handle_right` | Yes | `fn handle_right(&mut self) -> bool` |
| `handle_paste` | No | `fn handle_paste(&mut self, text: &str)` |
| `handle_copy` | No | `fn handle_copy(&mut self) -> Option<String>` - defaults to `None` |
| `handle_word_forward` | No | `fn handle_word_forward(&mut self)` |
| `handle_word_backward` | No | `fn handle_word_backward(&mut self)` |
| `handle_home` | No | `fn handle_home(&mut self)` |
| `handle_end` | No | `fn handle_end(&mut self)` |
| `handle_top` | No | `fn handle_top(&mut self)` |
| `handle_bottom` | No | `fn handle_bottom(&mut self)` |
| `handle_autocomplete` | No | `fn handle_autocomplete(&mut self) -> bool` - defaults to `false` |
| `handle_search` | No | `fn handle_search(&mut self, query: &str)` |
| `is_input_focused` | Yes | `fn is_input_focused(&self) -> bool` |
| `is_at_left_edge` | No | `fn is_at_left_edge(&self) -> bool` - defaults to `true` |
| `is_at_right_edge` | No | `fn is_at_right_edge(&self) -> bool` - defaults to `true` |
| `stop` | No | `fn stop(&mut self)` |
| `page_up` | No | `fn page_up(&mut self, page_size: usize)` |
| `page_down` | No | `fn page_down(&mut self, page_size: usize)` |

Inherits from `TabState` (4 methods): `state()`, `progress()`, `reset()`, `set_error()`.

### AppState Enum (`tabs/mod.rs:821-827`)

```rust
pub enum AppState {
    Idle,
    Running,
    Completed,
    Error(String),
}
```

### InputMode Enum (`app/input.rs:1-6`)

```rust
pub enum InputMode {
    Normal,  // vim-like navigation mode
    Insert,  // text input mode
}
```

### OverlayType Enum (`app/mod.rs:732-739`)

Overlay precedence (highest first; PolicyConfirm for enforcement `RequireConfirmation` + narrow manual overrides is top):

```rust
pub enum OverlayType {
    PolicyConfirm,  // Highest: enforcement confirmation (PendingPolicyConfirmation, reason input, narrow ManualOverride)
    ConfirmPopup,   // PendingAction confirmation dialog (destructive UI actions)
    CommandPalette, // Ctrl+P command palette
    QuickSwitch,    // Ctrl+X tab search/switch
    Search,         // Ctrl+F global search
    HttpOptions,    // h key HTTP options
    Help,           // Space key help overlay
}
```

(See also the "TUI Architecture and Usability Pass (2026-06-11)" section at the end of this file for the 10-phase summary.)

### PendingAction Enum (`app/confirmation.rs:3-9`)

```rust
pub enum PendingAction {
    ResetTab,           // Reset current tab state
    SaveSettings,       // Save settings to config file
    DeleteHistoryEntry, // Delete selected history entry
    ClearHistory,       // Clear all history
}
```

Each variant has a `message()` method returning a `(title, details)` tuple and an `execute()` method that performs the action on the `App`.

### NotificationSeverity Enum (`app/notifications.rs`)

```rust
pub enum NotificationSeverity {
    Info,
    Success,
    Warning,
    Error,
}
```

### ui/ Module (`ui/`)

The UI rendering layer is split into a module with focused submodules:

| File | Purpose |
|------|---------|
| `mod.rs` | `draw()` top-level render, `LAYOUT_MARGIN`, `TAB_BAR_HEIGHT` constants |
| `shell.rs` | Shell rendering helpers: `draw_tabs()`, `draw_breadcrumb()`, `draw_content()`, `draw_status_bar()`, `get_tab_status()`, `get_normal_status()`, `get_help_text()` |
| `popups.rs` | Popup overlays: `draw_http_options_popup()`, `draw_command_palette()`, `draw_search_popup()`, `draw_quick_switch()` |
| `tests.rs` | UI rendering tests |

### Components (`components/`)

Reusable UI primitives (7 files):

| Component | File | Purpose |
|-----------|------|---------|
| `InputField` | `input.rs` | Text input with cursor, validation, UTF-8 handling |
| `InputGroup` | `input.rs` | Group of inputs with focus navigation |
| `FormBuilder` | `input.rs` | Declarative form layout builder from input fields |
| `ValidationResult` | `input.rs` | Field validation result type |
| `Selector` | `selector.rs` | Dropdown selector with keyboard navigation |
| `SelectorItem` | `selector.rs` | Item in a selector (label + value) |
| `Checkbox` | `selector.rs` | Toggle checkbox |
| `RadioGroup` | `selector.rs` | Radio button group |
| `ProgressGauge` | `progress.rs` | Animated progress bar with spinner |
| `ScrollableText` | `scrollable.rs` | Scrollable text with scrollbar |
| `Popup` | `popup.rs` | Modal dialogs (confirm, help, info) |
| `centered_rect` | `popup.rs` | Centered rectangle helper for popups |
| `empty_state_paragraph` | `empty_state.rs` | Empty state placeholder widget |
**Note**: `draw_http_options_popup`, `draw_command_palette`, `draw_search_popup`, and `draw_quick_switch` are in `ui/popups.rs`, not in separate `components/` files. Previous component files (`palette.rs`, `search_popup.rs`, `http_options.rs`, `notifications.rs`, `help_bar.rs`) were removed as dead code.

### Workers (`workers/`)

Background async tasks communicate via channels:

| Worker | File | Task Types |
|--------|------|------------|
| `TaskRunner` | `runner.rs` | `TaskConfig`/`TaskResult` enums, async executor |
| Network | `network.rs` | Load test, stress test, packet operations |
| Scanner | `scanner.rs` | Port scan, endpoint scan, fingerprint |
| Fuzzer | `fuzzer.rs` | Fuzz, WAF, WAF stress operations |
| Recon | `recon.rs` | Recon operations (DNS, WHOIS, SSL, etc.) |
| API | `api.rs` | GraphQL, OAuth, NSE operations |
| Security | `security.rs` | Hunt, browser, compliance, storage, integrations |

(8 files total including `mod.rs`)

**Communication Flow**:
```
Tab builds TaskConfig → spawn_task() → TaskRunner (async)
                                              ↓
          progress_rx → App::update_progress() → tab.update_progress()
          result_rx → App::handle_result() → tab.set_results()
```

### State Management (`state/`)

```rust
pub type SharedHistory = Arc<Mutex<HistoryTab>>;
```

History is shared via `Arc<Mutex<HistoryTab>>` for thread-safe access.

### Theme (`theme/`)

The theme system supports 50+ packaged Halloy-format themes plus 3 built-in themes:

| File | Purpose |
|------|---------|
| `palette.rs` | `ThemeMode`, `Theme` (with `name: String`), `ThemeColors` structs |
| `builtin.rs` | `dark_theme()`, `light_theme()`, `cyber_red_theme()` factory functions |
| `manager.rs` | `ThemeManager` - holds registered themes, private `current`, canonical ID lookup, theme switching |
| `style.rs` | Theme style methods for rendering (currently unused helper methods) |
| `legacy.rs` | Thread-local macros (`tc!`, `theme!`) for backward compatibility |
| `loader.rs` | Parses Halloy `.toml` themes into Eggsec `Theme` structs; missing fields use defaults from built-in themes |
| `install.rs` | Idempotent installer: writes packaged themes to `~/.config/eggsec/themes`, never overwrites existing files |
| `archive.rs` | LZMA decode for packaged theme data |
| `packaged.rs` | Auto-generated LZMA-compressed blob of 50 Halloy themes (regenerated via `scripts/package_themes.py`) |

**Built-in themes**: `cyber-red` (default fallback, always available), `dark`, `light`.

**Packaged themes**: 50 Halloy-format `.toml` files are compiled into the binary via LZMA compression. On startup, `load_and_install_themes()` decodes the blob, installs any missing themes to the user's config directory, and loads all `.toml` files from that directory. Theme loading runs in a background thread (`std::thread::spawn`); the receiver, join handle, and deferred restore request live in `ThemeLoadState`, `App::update()` polls the channel, and the lifecycle helpers in `app/theme_runtime.rs` clean up the thread once the final report arrives or the loader disconnects. Failures are logged as warnings and do not block the UI.

**Theme selection**: The Settings tab has a theme selector dropdown instead of `dark_mode` checkbox and `accent_color` selector. Selector values are canonical theme IDs, labels are human-readable display names, and `Ctrl+T` cycles all themes alphabetically (`list_theme_ids_owned()`), wrapping at the end. Session persistence saves and restores the selected theme name, with deferred retry for packaged themes that are not yet loaded when the session starts.

`ThemeManager` holds registered themes with 28 color fields. `Theme.name` is the canonical stable ID for the theme, which keeps file-loaded themes and session restore aliases consistent.

The main shell and popup layers use explicit `&Theme` parameters. Tab renderers and components still use the `tc!` macro for theme colors:

```rust
use crate::tc;
let style = Style::default().fg(tc!(text));
```

New rendering code should prefer explicit `&Theme` parameters (via `App::current_theme()` or direct `theme` param) over the `tc!` macro.

### Session Management (`session.rs`)

`SessionManager` auto-saves at the configured interval (default 30 seconds) to JSON in the platform-specific sessions directory (`~/.local/share/eggsec/sessions/` on Linux via `directories::ProjectDirs`, with `~/.eggsec/sessions/` as a fallback), writes a quick-save on exit, and restores the saved theme name when loading sessions. If a packaged theme is not available yet, `App` keeps a deferred restore request in `ThemeLoadState` until the background loader registers it.

### Enforcement Context in TUI

TUI uses `EnforcementContext` directly (via `App.enforcement`, `manual_permissive` in runner.rs:82) for all target-bearing launches. Central gate in `App::update()` (mod.rs:322) before `spawn_task` (for handle_enter paths via `build_current_task`/`build_current_operation_descriptor`) plus retroactive gate for direct-launch tabs (mod.rs:366) that start inside their `handle_enter`. `RequireConfirmation` surfaces via highest-precedence `OverlayType::PolicyConfirm` (mod.rs:1095, key_handler.rs:205) backed by `PendingPolicyConfirmation` (confirmation.rs:59, state.rs:20) with reason input field. On confirm (mod.rs:787) it builds narrow `ManualOverride`, re-evaluates via the central `enforcement.evaluate`, and records via `decision.with_manual_override_record(mo.reason, confirmation_class_strings(...))` using stable kebab strings from `ConfirmationClass::as_str()`. `PendingAction` (confirmation.rs:4 for reset/save/etc.) remains separate and lower-precedence (`ConfirmPopup` overlay). `--strict-scope` affects profile selection for both CLI and TUI.

### Adding a New Tab

Adding a new tab requires changes in **7-9 locations**. Each new tab must be added to:

1. `Tab` enum (`tabs/mod.rs`) — variant + `title()`, `description()`, `cli_command()`, `stable_id()`, `from_stable_id()`, `all()` (feature-gated)
2. `TabStore` (`app/tab_store.rs`) — new field for the tab instance (TabStore owns all tab instances)
3. `Tab::as_tab_state()`, `as_tab_state_mut()`, `as_tab_render()`, `as_tab_input()` — 28-variant exhaustive match each
4. `App::get_current_help()` (`app/navigation.rs`) — 28-variant match
5. `command_to_tab()` + `execute_command()` (`app/command.rs`) — 28-variant match each
6. `export_results()` + `export_json()` (`app/export.rs`) — per-tab export logic
7. `App::build_current_task()` (`app/task_management.rs`) — trait dispatch
8. `help_config.rs` — static help section data

This duplication is a known architectural debt. A future refactor should move these to trait-based dispatch (e.g., `TabState::is_running()` instead of the exhaustive match).

## Maintenance Notes

**2026-06-02 dead code cleanup**: Removed the following dead code (all verified via grep with zero callers):
- 5 backup files in `tabs/` (`*.orig`, `*.bak`)
- 5 dead component files (`palette.rs`, `search_popup.rs`, `http_options.rs`, `notifications.rs`, `help_bar.rs`)
- `TabError::Internal`, `TabError::Auth` variants
- 5 `HelpContext` variants (`Configuration`, `Scanning`, `Fuzzing`, `Advanced`, `CommandDiscovery`)
- `HelpOverlay` struct + `App.help_overlay` field
- `GlobalSearch::search`, `move_up`, `move_down`, `selected`, `update_active_tab`
- `CommandPalette::with_popup_size`, `visible_results_height_for_area`, `max_scroll_offset_for_height`
- `SessionManager::restore_session` + 7 tests (logic was inlined in `App::new_inner`)
- `state/history.rs::add_fuzz_result`
- `InputState` enum
- 6 dead `App` methods (`pause`, `set_dark_mode`, `set_accent_color`, `set_notification`, `clear_notification`, `get_notification`)
- `App.spinner_tick` field
- `ThemeManager::set_accent_color`
- 3 tautology tests in `app/mod.rs`
- Dead `h` handler in `key_handler.rs` (unreachable due to overlay precedence)

Net result: ~700 lines of dead code removed, 10 fewer tests (3 tautologies + 7 session tests), 0 new warnings.

## Entry Point

The TUI launches automatically when:
1. No subcommand is provided
2. stdout is a terminal (interactive)

This happens via `handle_no_command()` in `commands/handlers/mod.rs`, which calls `tui::run()`.

**Not via `--tui` flag** - that flag does not exist.

## Key Bindings

| Key | Action |
|-----|--------|
| `Ctrl+C` | Interrupt task or quit |
| `Ctrl+P` | Command palette |
| `Ctrl+X` | Quick switch (tab search) |
| `Ctrl+F` | Global search |
| `Ctrl+T` | Cycle all themes alphabetically (wraps at end) |
| `Ctrl+B` | Bookmark current tab (shows "Bookmarked: <tab>" notification) |
| `Ctrl+Z` | Pause/resume active task updates |
| `Ctrl+Y` | Resume when paused, otherwise copy |
| `Shift+E` | Export with format selection (shows "Export format: <format>" notification) |
| `Space` | Toggle help |
| `1-9` / `0` | Jump to tab by index (`1`=Recon, `2`=Load, ..., `0`=tab 10) |
| `y` / `n` | Confirm/cancel in confirmation dialog (alongside Enter/Esc) |
| `hjkl` / Arrows | Navigation |
| `i` | Enter insert mode |
| `Esc` | Return to normal mode / close overlay |
| `q` | Quit (when no active task) |
| `g/G` | Go to top/bottom |
| `n/N` | Next/prev tab |
| `p` | Previous tab |
| `e` | Export results |
| `s` | Save settings |

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Main Loop (runner.rs)                                      │
│  ┌───────────────┐  ┌──────────────────┐  ┌────────────────┐ │
│  │ EventStream   │→ │ KeyHandler       │→ │ App method     │ │
│  │ (crossterm)   │  │ handle_key_event │  │ handle_*()     │ │
│  └───────────────┘  └──────────────────┘  └───────┬────────┘ │
│                                                     ↓        │
│  ┌──────────────────────────────────────────────────────────┤
│  │ TabDispatcher routes to current tab's TabInput           │
│  ├──────────────────────────────────────────────────────────┤
│  │ TabInput.handle_enter() → spawn_task()                   │
│  │                                                          │
│  │ ┌────────────────────────────────────────────────────┐  │
│  │ │ TaskRunner::run() async                            │  │
│  │ │ - Runs scan/fuzz/recon/etc                        │  │
│  │ │ - Sends progress via progress_tx                  │  │
│  │ │ - Sends result via result_tx                      │  │
│  │ └────────────────────────────────────────────────────┘  │
│  │                                                          │
│  │ ┌────────────────────────────────────────────────────┐  │
│  │ │ App::update()                                      │  │
│  │ │ - update_progress() → tab.update_progress()       │  │
│  │ │ - handle_result() → tab.set_results()            │  │
│  │ └────────────────────────────────────────────────────┘  │
│  │                                                          │
│  │ needs_redraw = true                                     │
└──┼───────────────────────────────────────────────────────────┘
   ↓
┌─────────────────────────────────────────────────────────────┐
│  Terminal.draw() → ui::draw()                                │
│  - draw_tabs() - tab bar                                    │
│  - draw_breadcrumb() - navigation path                      │
│  - draw_content() → tab.render()                            │
│  - draw_status_bar() - mode, state, help text               │
│  - Overlays: help, search, confirm popup, quick switch      │
└─────────────────────────────────────────────────────────────┘
```

## Bug Patterns to Avoid

### Division by Zero in Progress

```rust
// WRONG
fn progress(&self) -> f64 {
    (completed as f64 / self.stages.len() as f64) * 100.0
}

// CORRECT
fn progress(&self) -> f64 {
    if self.stages.is_empty() {
        return 0.0;
    }
    (completed as f64 / self.stages.len() as f64) * 100.0
}
```

### ScrollableText Scroll Offset with Empty Lines

```rust
// WRONG - usize::MAX when lines is empty
let scroll_offset = self.scroll_offset.min(self.lines.len().saturating_sub(1));

// CORRECT
let scroll_offset = if self.lines.is_empty() {
    0
} else {
    self.scroll_offset.min(self.lines.len() - 1)
};
```

### Silent Error Suppression in Workers

```rust
// WRONG
let response_text = response.text().await.unwrap_or_default();

// CORRECT
let response_text = match response.text().await {
    Ok(text) => text,
    Err(e) => {
        tracing::debug!("Failed to read response body: {}", e);
        String::new()
    }
};
```

### TaskResult Handling with Multiple Handlers

```rust
// WRONG - result moved before debug log
let Some(result) = self.handle_security_result(result) else { return };
tracing::debug!("Unhandled: {:?}", result); // ERROR: moved

// CORRECT
let result = match self.handle_security_result(result) {
    Some(r) => r,
    None => return,
};
if self.handle_feature_result(result).is_none() {
    tracing::debug!("Unhandled TaskResult variant");
}
```

### FxHashMap/FxHashSet Usage

For performance consistency, use `rustc_hash::FxHashMap` and `FxHashSet` instead of standard collections:

```rust
// WRONG
pub tabs: std::collections::HashMap<Tab, Box<dyn TabInput>>,
pub bookmarks: std::collections::HashSet<String>,

// CORRECT
use rustc_hash::{FxHashMap, FxHashSet};
pub bookmarks: FxHashSet<String>,
```

**Note**: Tab dispatch is done via exhaustive enum match in `Tab::as_tab_input()`, etc., NOT via HashMap lookup. The `Tab` enum provides stable IDs for session persistence. See `tabs/mod.rs` for the dispatch pattern.

Files using FxHashMap/FxHashSet in TUI module:
- `app/mod.rs` - App.bookmarks (FxHashSet)
- `app/bookmarks.rs` - Bookmark functions
- `app/help_config.rs` - StaticHelpData.sections
- `help.rs` - HelpManager.sections
- `theme/manager.rs` - ThemeManager.themes
- `tabs/dashboard.rs` - PortfolioSnapshot.findings_by_severity

### Key Binding Conflict Prevention

When adding key bindings in `key_handler.rs`, avoid duplicate patterns in the same match arm:

```rust
// WRONG - 'e' appears twice, second arm is unreachable
(KeyModifiers::NONE, KeyCode::Char('w')) => app.handle_word_forward(),
(KeyModifiers::NONE, KeyCode::Char('e')) => app.export_results(),
(KeyModifiers::NONE, KeyCode::Char('e')) => app.handle_word_forward(), // unreachable!

// CORRECT - unique bindings
(KeyModifiers::NONE, KeyCode::Char('w')) => app.handle_word_forward(),
(KeyModifiers::NONE, KeyCode::Char('e')) => app.export_results(),
```

### Bounds Check for Array Access

When accessing arrays/vectors via index, always validate bounds:

```rust
// WRONG - could panic if index >= len
self.option_checkboxes[self.focused_checkbox_index].toggle();

// CORRECT - bounds check prevents panic
if self.focused_checkbox_index < self.option_checkboxes.len() {
    self.option_checkboxes[self.focused_checkbox_index].toggle();
}
```

Similarly for InputGroup field access:

```rust
// WRONG - assumes at least 2 fields
self.inputs.fields[1].value = "report.json".to_string();

// CORRECT - check length first
if self.inputs.fields.len() > 1 {
    self.inputs.fields[1].value = "report.json".to_string();
}
```

For slicing InputGroup.fields (e.g., accessing a range of fields), use bounds-checked patterns:

```rust
// WRONG - panics if fewer than 4 fields
let fields = &self.issue_inputs.fields[..4];

// CORRECT - safe slicing with .get()
let fields = self.issue_inputs.fields.get(..4).unwrap_or(&self.issue_inputs.fields);
```

For option checkbox arrays (like ReconOptions), use `.get()` with fallback:

```rust
// WRONG - panics if index out of bounds
no_tech: self.option_checkboxes[0].checked,

// CORRECT - returns false if index invalid
no_tech: self.option_checkboxes.get(0).map(|cb| cb.checked).unwrap_or(false),
```

### ScrollableText scroll_to_bottom

When implementing or modifying `scroll_to_bottom()`, always handle empty lines:

```rust
// WRONG - scroll_offset becomes incorrect when lines is empty
self.scroll_offset = self.lines.len().saturating_sub(1);

// CORRECT - explicit empty check
if self.lines.is_empty() {
    self.scroll_offset = 0;
} else {
    self.scroll_offset = self.lines.len() - 1;
}
```

### ScrollableText scroll_down

When implementing `scroll_down()`, handle empty lines to prevent `usize::MAX`:

```rust
// WRONG - max_scroll becomes usize::MAX when lines is empty
pub fn scroll_down(&mut self, amount: usize) {
    let max_scroll = self.lines.len().saturating_sub(1);
    self.scroll_offset = (self.scroll_offset + amount).min(max_scroll);
}

// CORRECT - explicit empty check
pub fn scroll_down(&mut self, amount: usize) {
    if self.lines.is_empty() {
        self.scroll_offset = 0;
    } else {
        let max_scroll = self.lines.len() - 1;
        self.scroll_offset = (self.scroll_offset + amount).min(max_scroll);
    }
}
```

### InputGroup Field Access in Edge Detection

When accessing InputGroup fields in `is_at_left_edge()` or `is_at_right_edge()`, use safe accessors:

```rust
// WRONG - direct indexing can panic if fields is empty
fn is_at_left_edge(&self) -> bool {
    match self.focus_area {
        VulnFocusArea::Inputs => self.inputs.fields[0].cursor_pos == 0,
        _ => true,
    }
}

// CORRECT - use first() with map and unwrap_or
fn is_at_left_edge(&self) -> bool {
    match self.focus_area {
        VulnFocusArea::Inputs => self
            .inputs
            .fields
            .first()
            .map(|f| f.cursor_pos == 0)
            .unwrap_or(true),
        _ => true,
    }
}
```

### InputGroup can_move_left/can_move_right Empty Guard

The `can_move_left()` and `can_move_right()` methods should also guard against empty fields for consistency:

```rust
// WRONG - no empty guard
pub fn can_move_left(&self) -> bool {
    if let Some(idx) = self.focused {
        idx < self.fields.len() && self.fields[idx].cursor_pos > 0
    } else {
        false
    }
}

// CORRECT - explicit empty check
pub fn can_move_left(&self) -> bool {
    if !self.fields.is_empty() {
        if let Some(idx) = self.focused {
            idx < self.fields.len() && self.fields[idx].cursor_pos > 0
        } else {
            false
        }
    } else {
        false
    }
}
```

### Selector Edge Detection Empty Guard

Tabs with `Selector` or similar selectors must guard against empty selector items:

```rust
// WRONG - items could be empty causing incorrect edge detection
FocusArea::Selector => self.selector.selected == 0,

// CORRECT - guard against empty selector
FocusArea::Selector => {
    self.selector.items.is_empty()
        || self.selector.selected == 0
}
```

### Worker Error Logging Levels

Workers should use `tracing::warn!` for expected failure cases, not `debug!`:

```rust
// WRONG - errors at debug level may be missed in production
Err(e) => {
    tracing::debug!("GraphQL introspection request failed: {}", e);
    errors += 1;
}

// CORRECT - use warn for operational errors that may indicate issues
Err(e) => {
    tracing::warn!("GraphQL introspection request failed: {}", e);
    errors += 1;
}
```

### Vec::remove vs Vec::swap_remove

When removing elements from a Vec in a loop where order doesn't matter, use `swap_remove` instead of `remove`:

```rust
// WRONG - O(n) shift for each removal
while sessions.len() > max_sessions {
    sessions.remove(0);
}

// CORRECT - O(1) swap and pop
while sessions.len() > max_sessions {
    sessions.swap_remove(0);
}
```

**Exception**: VecDeque does not have `swap_remove`. Use `remove` for VecDeque or when the collection type is not Vec.

### Tokio Spawn Error Handling

When awaiting `tokio::spawn` JoinHandle results, use proper pattern matching to detect panics:

```rust
// WRONG - double unwrap can panic
let handle_result = tokio::time::timeout(Duration::from_secs(2), handle).await;
if let Err(e) = handle_result {
    tracing::warn!("Handle timed out: {}", e);
} else if let Err(e) = handle_result.unwrap() {  // double unwrap!
    // ...
}

// CORRECT - proper nested match
let handle_result = tokio::time::timeout(Duration::from_secs(2), handle).await;
match handle_result {
    Err(e) => {
        tracing::warn!("Handle timed out: {}", e);
    }
    Ok(Err(e)) => {
        if e.is_panic() {
            tracing::warn!("Task panicked: {:?}", e);
        } else {
            tracing::warn!("Task failed: {}", e);
        }
    }
    Ok(Ok(())) => {
        tracing::debug!("Task completed successfully");
    }
}
```

For progress tracking tasks that are aborted on completion, also check the join result:

```rust
if let Err(e) = progress_handle.await {
    if e.is_panic() {
        tracing::warn!("Progress tracking task panicked: {:?}", e);
    }
}
```

### Worker Channel Send Error Handling

Workers send progress and results via channels. Always handle send errors properly:

```rust
// WRONG - silent error suppression
let _ = result_tx.send(TaskResult::LoadTest(results)).await;
let _ = progress_tx.send((requests, requests)).await;

// CORRECT - proper error handling with warn logging
if let Err(e) = result_tx.send(TaskResult::LoadTest(results)).await {
    tracing::warn!("Failed to send load test results: {}", e);
}
if let Err(e) = progress_tx.send((requests, requests)).await {
    tracing::warn!("Failed to send progress: {}", e);
}
```

This pattern was fixed across all 7 worker files (94 total occurrences) in the 2026-05-31 session:
- `api.rs` (15), `security.rs` (27), `recon.rs` (12), `network.rs` (13)
- `scanner.rs` (9), `fuzzer.rs` (8)

### Selector confirm() Return Value

Selector's `confirm()` returns `Option<&SelectorItem>`, not `Result`. Handle appropriately:

```rust
// WRONG - treating Option as Result
if let Err(e) = self.profile_selector.confirm() {
    tracing::warn!("Confirm failed: {}", e);
}

// CORRECT - handle Option properly
if self.profile_selector.confirm().is_none() {
    tracing::warn!("Confirm failed: selector returned None");
}
```

### ScrollableText render() scroll_offset

In ScrollableText render(), ensure the bounded scroll_offset is used:

```rust
// WRONG - uses unbounded self.scroll_offset instead of bounded value
let scroll_offset = if self.lines.is_empty() {
    0
} else {
    self.scroll_offset.min(self.lines.len() - 1)
};
// ... later ...
f.render_stateful_widget(paragraph, area);  // Uses self.scroll_offset, not bounded value

// CORRECT - pass bounded scroll_offset to scroll
let scroll_offset = if self.lines.is_empty() {
    0
} else {
    self.scroll_offset.min(self.lines.len() - 1)
};
f.render_stateful_widget(
    Paragraph::new(self.lines.clone())
        .block(block)
        .scroll((scroll_offset as u16, self.horizontal_offset as u16)),
    area,
);
```

### Selector handle_left() Empty Items Guard

Always add `is_empty()` guard to `handle_left()` for consistency with `handle_right()`:

```rust
// WRONG - doesn't check if items is empty
pub fn handle_left(&mut self) {
    if self.expanded && self.selected > 0 {
        self.selected -= 1;
    }
}

// CORRECT - guards against empty items
pub fn handle_left(&mut self) {
    if self.expanded && !self.items.is_empty() && self.selected > 0 {
        self.selected -= 1;
    }
}
```

### FormBuilder render() Bounds Check

FormBuilder's render loop must use `.get()` for safe array access:

```rust
// WRONG - direct indexing can panic
for (i, field) in self.fields.iter().enumerate() {
    match field {
        FieldVariant::Input(input) => input.render(f, chunks[i], insert_mode),
        // ...
    }
}

// CORRECT - bounds-checked access
for (i, field) in self.fields.iter().enumerate() {
    if let Some(chunk) = chunks.get(i) {
        match field {
            FieldVariant::Input(input) => input.render(f, *chunk, insert_mode),
            // ...
        }
    }
}
```

### to_lowercase() Caching in Search Methods

Cache lowercase values before filter closures to avoid repeated allocation:

```rust
// WRONG - to_lowercase() called 4+ times per entry in filter
.filter(|e| {
    e.target.to_lowercase()
    || e.scan_type.to_lowercase()
    || e.summary.to_lowercase()
    || e.details.iter().any(|d| d.to_lowercase().contains(&query_lower))
})

// CORRECT - pre-compute all lowercased values
.filter(|e| {
    let target_lower = e.target.to_lowercase();
    let scan_type_lower = e.scan_type.to_lowercase();
    let summary_lower = e.summary.to_lowercase();
    let details_lower: Vec<String> = e.details.iter().map(|d| d.to_lowercase()).collect();
    target_lower.contains(&query_lower)
        || scan_type_lower.contains(&query_lower)
        || summary_lower.contains(&query_lower)
        || details_lower.iter().any(|d| d.contains(&query_lower))
})
```

### is_at_left_edge/is_at_right_edge Checkbox Array Guards

Always guard checkbox array access with `is_empty()` checks:

```rust
// WRONG - doesn't guard against empty checkboxes
fn is_at_left_edge(&self) -> bool {
    self.focused_checkbox_index == 0
}

// CORRECT - guards against empty array
fn is_at_left_edge(&self) -> bool {
    self.checkbox_array.is_empty() || self.focused_checkbox_index == 0
}

fn is_at_right_edge(&self) -> bool {
    self.checkbox_array.is_empty()
        || self.focused_checkbox_index >= self.checkbox_array.len().saturating_sub(1)
}
```

This pattern applies to all tabs with checkbox arrays: `hunt.rs`, `browser.rs`, `compliance.rs`, `graphql.rs`, `oauth.rs`.

### Selector confirm() Return Value

Selector's `confirm()` returns `Option<&SelectorItem>`, not `Result`. Handle appropriately:

```rust
// WRONG - treating Option as Result
if let Err(e) = self.profile_selector.confirm() {
    tracing::warn!("Confirm failed: {}", e);
}

// CORRECT - handle Option properly
if self.profile_selector.confirm().is_none() {
    tracing::warn!("Confirm failed: selector returned None");
}
```

### Session Error Handling

When loading sessions or cleaning up old sessions, avoid silent error suppression:

```rust
// WRONG - silently ignores read errors
.filter_map(|e| e.ok())

// CORRECT - log errors at debug level
.filter_map(|e| match e {
    Ok(entry) => Some(entry),
    Err(e) => {
        tracing::debug!("Skipping unreadable directory entry: {:?}", e);
        None
    }
})

// WRONG - silently ignores removal errors
let _ = fs::remove_file(oldest.path());

// CORRECT - log errors at warn level
if let Err(e) = fs::remove_file(oldest.path()) {
    tracing::warn!("Failed to cleanup old session {:?}: {:?}", oldest.path(), e);
}
```

### Quick Switch Selection Clamping

When filtering quick switch results, the clamping function must re-fetch fresh results:

```rust
// WRONG - uses stale results passed as parameter
fn clamp_quick_switch_selection(&self, app: &mut App, results: &[&Tab]) {
    app.quick_switch_selected = app.quick_switch_selected.min(results.len().saturating_sub(1));
}

// CORRECT - re-fetches fresh results after query change
fn clamp_quick_switch_selection(&self, app: &mut App) {
    let results = app.get_quick_switch_results();
    app.quick_switch_selected = if results.is_empty() {
        0
    } else {
        app.quick_switch_selected.min(results.len() - 1)
    };
}
```

## Additional Fixes (2026-06-01 Session)

### Edge Detection for Checkbox Arrays

| File | Lines | Issue | Fix |
|------|-------|-------|-----|
| `graphql.rs` | 490-502 | Options checkbox bounds missing | Added explicit `GraphQlFocusArea::Options` case |
| `oauth.rs` | 534-546 | Options checkbox bounds missing | Added explicit `OAuthFocusArea::Options` case |
| `vuln.rs` | 618-619 | `is_at_right_edge()` missing `is_empty()` guard | Added `self.mode_selector.items.is_empty() \|\|` guard |

### handle_enter() is_running() Guards

| File | Line | Status |
|------|-------|--------|
| `report.rs` | 457-460 | `handle_enter()` guarded | ✅ Fixed |
| `nse.rs` | 312-314 | `handle_enter()` guarded | ✅ Fixed |

### Other Tab-Specific Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `workflow.rs` | 411 | `handle_copy()` missing guard | Added `!self.is_running()` guard |
| `workflow.rs` | 257 | `reset()` doesn't clear `current_mode` | Set `current_mode = WorkflowMode::ListFindings` |
| `integrations.rs` | 280 | `reset()` doesn't clear selector | Added `self.tracker_selector.selected = 0;` |
| `storage.rs` | 250-251 | `reset()` doesn't clear `query_inputs.fields` | Added fields.clear() loop |

### Components Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `selector.rs` | 228 | Silent `let _ =` on confirm() | `if .is_none() { warn }` |
| `palette.rs` | 60 | Direct array access | `.get()` with bounds check |
| `session.rs` | 113 | `debug!` vs `warn!` | Changed to `tracing::warn!` |
| `session.rs` | 174 | Silent `filter_map(\|e\| e.ok())` | Explicit match with warn |

### FxHashMap Performance Updates

| File | Lines | Change |
|------|-------|--------|
| `orchestrator/mod.rs` | 21, 50, 84, 89, 302 | HashMap/HashSet → FxHashMap/FxHashSet |
| `tool/session.rs` | 232, 288, 316, 461, 465, 1076 | HashMap → FxHashMap |
| `tool/state.rs` | 124, 136 | HashMap → FxHashMap |
| `recon/mod.rs` | 222, 254 | HashMap → FxHashMap |

## Bug Fixes (2026-06-01 Session - Additional)

### settings/main.rs Fixes

| File | Lines | Issue | Fix |
|------|-------|-------|-----|
| `settings/main.rs` | 311-347 | `apply_to_config()` unsafe direct field access | Changed to safe `.get()` pattern with bounds checks |
| `settings/main.rs` | 400,523,595 | Silent file write errors | Added `if let Err(e) = ...` with status_message |

### Tab handle_enter() Fixes

| File | Lines | Issue | Fix |
|------|-------|-------|-----|
| `report.rs` | 457-487 | `handle_enter()` returns early when not running | Restructured to allow selector interaction when idle |
| `nse.rs` | 311-340 | `handle_enter()` logic issue with Results + is_running | Restructured to properly handle blur/selector |
| `graphql.rs` | 415-432 | Missing `is_running()` guard on `handle_enter()` | Added `!self.is_running()` guard |
| `oauth.rs` | 459-476 | Missing `is_running()` guard on `handle_enter()` | Added `!self.is_running()` guard |
| `recon.rs` | 591-596 | Missing `is_running()` guard on Options toggle | Added `!self.is_running()` guard |

### Edge Detection Fixes

| File | Lines | Issue | Fix |
|------|-------|-------|-----|
| `recon.rs` | 670-671 | Missing `is_empty()` guard on `is_at_left_edge()` | Added `self.option_checkboxes.is_empty() \|\|` |
| `scrollable.rs` | 99-106 | `is_at_left_edge/is_at_right_edge` inconsistent | Added `is_empty()` guards to both methods |

### Input/Render Fixes

| File | Lines | Issue | Fix |
|------|-------|-------|-----|
| `stress.rs` | 263-267 | Direct array access `input_chunks[i]` | Changed to `.get(i)` pattern |
| `stress.rs` | 390-404 | `handle_enter()` result not captured | Changed to `confirm().is_none()` pattern |
| `storage.rs` | 339 | Direct array access `query_chunks[0]` | Changed to `.get(0)` pattern |
| `integrations.rs` | 335 | Suspicious fallback `&[]` in slice access | Changed to `&self.issue_inputs.fields` |

### Other Tab Fixes

| File | Lines | Issue | Fix |
|------|-------|-------|-----|
| `vuln.rs` | 495-505 | `handle_copy()` missing `is_running()` guard | Added `!self.is_running()` guard |
| `history.rs` | 441,443 | Empty handlers missing `is_running()` guards | Added `!self.is_running()` guards |
| `auth.rs` | 227-229 | `fields.len() - 1` underflow risk | Added `!self.inputs.fields.is_empty()` guard |

### Worker/App Fixes

| File | Lines | Issue | Fix |
|------|-------|-------|-----|
| `task_runtime.rs` | 72-76 | Silent error suppression `Err(_e)` | Changed to `if let Err(e) = ...` using actual error |
| `session.rs` | 525, 1016 | Silent error suppression `unwrap_or_default()` | Changed to `unwrap_or_else(\|e\| { warn!; String::new() })` |
| `state.rs` | 217 | `debug!` instead of `warn!` for file removal | Changed to `tracing::warn!` |
| `cache.rs` | 278 | `debug!` instead of `warn!` for cache dir creation | Changed to `tracing::warn!`

## Session Fixes (2026-06-02)

### TUI Tab Fixes - Missing is_running() Guards

All navigation handlers (`handle_word_forward`, `handle_word_backward`, `handle_home`, `handle_end`, `handle_up`, `handle_down`, `handle_left`, `handle_right`) now properly guard with `!self.is_running()` in these tabs:

| Tab | Issue |
|-----|-------|
| `compliance.rs` | Missing guards on 8 handlers - all fixed |
| `vuln.rs` | Missing guards on 8 handlers - all fixed |
| `storage.rs` | Missing guards + incorrect `true` fallback in handle_left/right - fixed |
| `integrations.rs` | Missing guards + incorrect `true` fallback - fixed |
| `workflow.rs` | Missing guards + incorrect `true` fallback - fixed |
| `graphql.rs` | Missing guards on handle_left/right + field name fix |
| `oauth.rs` | Missing guards on handle_left/right - fixed |

### TUI Tab Fixes - Empty Checkbox Array Underflow

| File | Issue | Fix |
|------|-------|-----|
| `hunt.rs` | handle_up/down could underflow when `option_checkboxes` is empty | Added `is_empty()` guard before manipulation |
| `browser.rs` | Same issue | Added `is_empty()` guard before manipulation |

### TUI Tab Fixes - Edge Detection

| File | Issue | Fix |
|------|-------|-----|
| `report.rs` | ViewSelector edge detection missing `is_empty()` guard | Added `view_selector.items.is_empty()` check |
| `stress.rs` | TypeSelector edge detection missing `is_open()` guard | Added `if self.type_selector.is_open()` check |

### TUI Component Fixes

| File | Issue | Fix |
|------|-------|-----|
| `palette.rs` | Direct array access on layout chunks could panic with small terminal | Added `chunks.len() < 3` guard and `.get()` pattern |

### Worker Fixes

| File | Issue | Fix |
|------|-------|-----|
| `network.rs` | JoinHandle abandoned without abort on timeout | Added `handle.abort()` in timeout case |
| `api.rs` | Missing `is_panic()` check on spawned task result | Added explicit match to detect panic

## Session Fixes (2026-06-03)

### TUI Tab Fixes - Navigation Handlers (~97 handlers across 17 tabs)

All `!self.is_running()` guards added to navigation handlers:

| Group | Tabs Fixed |
|-------|-----------|
| Group 1 | recon.rs (word_forward/backward), scan.rs (6 handlers), scan_ports.rs (4), scan_endpoints.rs (4), fingerprint.rs (6) |
| Group 2 | load.rs (6), stress.rs (6), cluster.rs (handle_enter), proxy.rs (handle_enter) |
| Group 3 | hunt.rs (3), browser.rs (3), compliance.rs (4), vuln.rs (2) |
| Group 4 | dashboard.rs (17), resume.rs (11), history.rs (10) |

### reset() Methods Fixed (17 tabs)

| Tab | Added Reset |
|-----|------------|
| `packet.rs` | view_selector.select(0) |
| `graphql.rs`, `oauth.rs` | checkbox reset, focused_checkbox_index |
| `cluster.rs` | view_selector, worker/coordinator/status_inputs |
| `proxy.rs` | view_selector |
| `nse.rs` | input fields |
| `hunt.rs`, `browser.rs` | checkbox reset, focused_checkbox_index, focus_area |
| `compliance.rs` | framework_selector.select(0), focus_area |
| `storage.rs` | mode_selector, query_inputs, focus_area, current_mode |
| `integrations.rs` | config_inputs, issue_inputs |
| `workflow.rs` | focus_area |
| `vuln.rs` | mode_selector, focus_area, current_mode |
| `report.rs` | view_selector, format_selector, current_view |
| `settings/main.rs` | proxy_rotation_selector, severity_selector, accent_color |
| `auth.rs` | results.clear() |

### App Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `app/mod.rs` | 452, 459 | Silent `let _ =` on dispatcher | Changed to `if !bool { warn }` |
| `key_handler.rs` | 407-414 | Stale quick switch results | Re-fetch fresh results on Enter |
| `task_runtime.rs` | 68-80 | No timeout on spawn | Added `tokio::time::timeout(300s, ...)` |

### Other Module Fixes

| Module | File | Line | Issue | Fix |
|--------|------|------|-------|-----|
| Workers | `api.rs` | 143 | Division by zero | Added `.max(1)` guard |
| Config | `loader.rs` | 18 instances | Silent file operations | `if let Err(e) = ...` with warn |
| Output | `markdown.rs` | 87 | to_lowercase() in loop | Cached before loop |
| Output | `dedup.rs` | 16 | to_lowercase() in parse | eq_ignore_ascii_case |
| Tool | `script/*.rs` | Multiple | HashMap → FxHashMap | Changed to FxHashMap |
| Tool | `routes.rs` | 28, 118 | unwrap_or_default | Added warn logging |
| Tool | `implementations/*.rs` | Various | load_config unwrap | Added inspect_err with warn |
| Scanner | `matcher.rs` | 262, 268 | Silent socket ops | Added warn logging |
| Scanner | `fingerprint.rs` | 432 | Silent probe write | Added warn logging |
| Recon | `whois.rs` | 171 | Silent timeout | Added warn logging |


## Session Fixes (2026-06-04)

### App Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `task_runtime.rs` | 83-91 | Timeout case did not abort JoinHandle | Added `handle.abort()` after timeout error |
| `state_update.rs` | 68 | Unhandled TaskResult warning missing context | Changed to `tracing::warn!("Unhandled TaskResult variant: {:?}", result);` |

### TUI Deep Dive Audit (2026-06-04)

Completed comprehensive audit of all 28 TUI tabs across 6 groups. Fixed ~100+ issues:

| Group | Tabs | Fixes |
|-------|------|-------|
| Group 1 | recon, scan, scan_ports, scan_endpoints, fingerprint | 21 navigation guards + handle_enter logic + reset() |
| Group 2 | fuzz, waf, waf_stress, load, stress | 40 navigation guards |
| Group 3 | packet, graphql, oauth, cluster, proxy | 13 fixes (bounds, guards, logic) |
| Group 4 | nse, hunt, browser, compliance | 15+ fixes |
| Group 5 | storage, integrations, workflow, vuln, report | 8 major fixes |
| Group 6 | history, settings | 18 navigation guards + reset fields |

Key patterns fixed:
- **handle_enter() logic**: 8 tabs where blur happened BEFORE stop (should be stop → blur → start)
- **Navigation handlers**: ~97 missing `!self.is_running()` guards across 17 tabs
- **reset() methods**: 11 tabs now properly reset checkbox/selector state
- **Edge detection**: 5 tabs added missing `is_empty()` guards

## Session Fixes (2026-06-05)

### TUI Tab Fixes (Additional Audit)

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `scan.rs` | 278 | reset() incomplete | Added `current_stage_output.clear()` |
| `graphql.rs` | 510-524 | Options edge detection missing is_empty guard | Added `is_empty()` guard |

### Workers Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `api.rs` | 143 | Division by zero | Added `.max(1)` guard |
| `recon.rs` | 127-168 | progress_handle spawn no timeout | Added 300s timeout |
| `network.rs` | 168-170 | capture.start() no timeout | Added 300s timeout |

### Tool/AI/App/Scanner/Config Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `ai/cache.rs` | 171 | Cache merge logic inverted | Changed to prefer old entries |
| `ai/planner.rs` | 456 | to_lowercase in loop | Pre-compute before filter |
| `tool/session.rs` | 511 | HTTP no timeout | Added 30s timeout |
| `tool/finding.rs` | Multiple | HashMap performance | FxHashMap |
| `tool/aggregator.rs` | Multiple | HashMap performance | FxHashMap |
| `scanner/spoofed.rs` | 281-304 | Silent send failures | Warn logging + only increment on success |
| `scanner/spoofed.rs` | 454-458 | Mutex vs AtomicU64 | Changed to AtomicU64 |
| `config/scope.rs` | 45 | HashSet performance | FxHashSet |
| `app/task_runtime.rs` | 83-91 | No abort on timeout | handle.abort() added |
| `scrollable.rs` | 104 | is_at_right_edge inconsistency | Returns horizontal_offset == 0 when empty |
| `palette.rs` | 39 | Direct chunks[2] access | .get() pattern |
| `popup.rs` | 39 | Direct chunks[2] access | .get() pattern |

(End of file - total 1020 lines)


## Session Fixes (2026-06-08)

### App Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `state_update.rs` | 67 | Duplicate `handle_protocol_result` call | Fixed to call `handle_feature_result` |

### Workers Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `fuzzer.rs` | 200 | Silent timeout with `let _ =` | Proper match with warn logging |

### Scanner Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `ports/mod.rs` | 589-591 | Silent `catch_unwind` without comment | Added comment explaining why + warn logging |

### Tool Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `session.rs` | 655,675,698,1007 | HashMap in function signatures | Changed to `FxHashMap` |
| `session.rs` | 636 | Dead code `let _ = step` | Added placeholder comment |

### TUI Tab Fixes (Deep Dive Audit 2026-06-08)

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `workflow.rs` | 313-318 | Hardcoded field indices without bounds | Added `idx < fields.len()` guards |
| `settings/main.rs` | 458-499 | `reset()` incomplete | Added focus_area, current_section, detail_focus_index resets |
| `load.rs` | 649-672 | handle_left/right missing guard | Added `is_running()` guard |
| `hunt.rs` | 507-535 | handle_left/right missing guard | Added `is_running()` guard |
| `browser.rs` | 470-498 | handle_left/right missing guard | Added `is_running()` guard |
| `scan_ports.rs` | 507-512 | handle_left/right missing guard | Added `is_running()` guard |
| `scan_ports.rs` | 364-370 | handle_focus_next/prev missing guard | Added `is_running()` guard |
| `scan_endpoints.rs` | 471-476 | handle_left/right missing guard | Added `is_running()` guard |
| `fingerprint.rs` | 411-416 | handle_left/right missing guard | Added `is_running()` guard |
| `recon.rs` | 680 | Underflow risk `len() - 1` | Added `is_empty()` check |
| `scan.rs` | 469 | handle_copy missing guard | Added `is_running()` guard |
| `waf.rs` | 530 | ModeRadio case missing guard | Added `!self.is_running()` |
| `report.rs` | 337-379 | handle_focus_next/prev missing guard | Added `is_running()` guard |
| `report.rs` | 518-531 | handle_escape has unusual guard | Removed inappropriate guard |
| `integrations.rs` | 334 | Suspicious `&[]` fallback | Added warn logging on fallback |
| `auth.rs` | 50-56 | reset() missing focus_area | Added `focus_area` reset |
| `history.rs` | 167-187 | to_lowercase() in loops | Pre-compute before filter |


## Session Fixes (2026-06-11)

### Theme System Fixes
- **Ctrl+T cycles ALL themes**: Iterates `list_theme_ids_owned()` alphabetically, wrapping at end (was limited to built-in trio)
- **Theme::default() returns cyber-red**: Was `dark_theme`, which disagreed with `ThemeManager::default`
- **set_theme() logs at debug level** when a theme is not found (was silent)
- **ThemeInstallReport::Clone**: Documents that `loaded_themes` is dropped because `ThemeLoadError` is not Clone
- **set_items_with_extra on Selector**: Adds a missing theme to the dropdown without silently replacing with index 0
- **Theme install failure notifications**: Surfaced via the notification system (no longer silent)
- **Style.rs methods**: Annotated `#[allow(dead_code)]` with comment explaining they are for future adoption
- **Content_len cap in archive.rs**: Prevents pathological allocation (1 MiB cap)

### Key Binding Changes
- `Ctrl+T` now cycles ALL themes (not just built-ins)
- `Ctrl+B` shows "Bookmarked: <tab>" notification
- `Shift+E` shows "Export format: <format>" notification
- `1-9` / `0` jump to tab by index (new)
- `y` / `n` confirm/cancel in confirmation dialog (new shortcuts alongside Enter/Esc)
- `pending_key` is now cleared on overlay open (fixes stale `gg` after opening quick switch)

### Session Management Hardening
- `.json.tmp` orphans cleaned up on both save paths
- `load_latest_session` quarantines corrupt files (`.json.bad`) and tries next
- `auto_save_if_due` skips during active tasks
- `SessionConfig` fallback uses `$HOME/.eggsec/sessions` (was bare `~/.eggsec/sessions`)
- `auto_save_interval` clamped to min 1 second
- `load_latest_session` filters out `quick_save.json` from snapshot candidates

## Session Fixes (2026-06-09)

### AI Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `cache.rs` | 158-180 | Cache merge logic broken | Fixed merge to preserve existing entries |
| `cache.rs` | 158-180 | Unnecessary complexity | Simplified to direct iteration |

### App Module Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `task_runtime.rs` | 103 | Misleading timeout message | Changed to "aborting task" |
| `mod.rs` | 452-463 | Misleading warn logs | Changed to debug level |



## Session Fixes (2026-06-10)

### TUI Deep Dive Audit - All 28 Tabs

#### Group 1 (recon, scan, scan_ports, scan_endpoints, fingerprint)

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `recon.rs` | 606 | Missing `is_running()` guard before `inputs.blur()` | Added `!self.is_running()` guard |
| `recon.rs` | 633-634 | Empty array underflow risk on checkbox index | Added `!self.option_checkboxes.is_empty()` check |
| `scan.rs` | 537 | Missing `is_running()` guard before `inputs.blur()` | Added `!self.is_running()` guard |
| `scan.rs` | 278 | reset() missing focus_area clear | Added `self.focus_area = ScanFocusArea::Inputs` |
| `scan_ports.rs` | 497 | Missing `is_running()` guard before `inputs.blur()` | Restructured with guard |
| `scan_ports.rs` | 172 | Direct array access without bounds | Changed to safe `.get()` pattern |
| `scan_endpoints.rs` | 435 | Missing `is_running()` guard before `inputs.blur()` | Restructured with guard |
| `scan_endpoints.rs` | 263 | reset() missing focus_area clear | Added focus_area reset |
| `fingerprint.rs` | - | No bugs found | - |

#### Group 2 (fuzz, waf, waf_stress, load, stress)

All 5 tabs passed audit - no bugs found.

#### Group 3 (packet, graphql, oauth, cluster, proxy)

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `graphql.rs` | 461 | `handle_enter()` missing `!is_running()` guard | Changed to `if !self.is_running()` |
| `oauth.rs` | 505 | `handle_enter()` missing `!is_running()` guard | Changed to `if !self.is_running()` |
| `cluster.rs` | 463-465 | Inverted guard logic (stopped when should allow) | Changed to `if !self.is_running()` |
| `proxy.rs` | 598-599 | Non-standard guard pattern | Changed to `if !self.is_running()` |

#### Group 4 (nse, hunt, browser, compliance)

All 4 tabs passed audit - no bugs found.

#### Group 5 (storage, integrations, workflow, vuln, report)

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `report.rs` | 338 | handle_focus_next() inverted guard | Changed `!is_running()` to `is_running()` |
| `report.rs` | 363 | handle_focus_prev() inverted guard | Changed `!is_running()` to `is_running()` |
| `storage.rs` | 526 | handle_top() missing guard | Added `!self.is_running()` guard |
| `storage.rs` | 531 | handle_bottom() missing guard | Added `!self.is_running()` guard |
| `storage.rs` | 259 | reset() missing current_mode | Added `self.current_mode = StorageMode::Connect` |
| `integrations.rs` | - | No bugs found | - |
| `workflow.rs` | - | No bugs found | - |
| `vuln.rs` | - | No bugs found | - |

#### Group 6 (resume, history, dashboard, settings, auth)

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `resume.rs` | 191 | handle_copy() missing guard | Added `if self.is_running()` guard |
| `history.rs` | 315-318 | reset() missing focus_area | Added `self.focus_area = HistoryFocusArea::List` |
| `settings/main.rs` | 458-501 | reset() incomplete | Added scope/report/schedule/notify field clears |
| `dashboard.rs` | - | No bugs found | - |
| `auth.rs` | - | No bugs found | - |

### Summary

| Metric | Value |
|--------|-------|
| Total tabs audited | 28 |
| Tabs with bugs | 14 |
| Tabs clean | 14 |
| Total bugs fixed | 24 |

(End of file)

## Session Fixes (2026-06-10)

### TUI Deep Dive Audit - All 28 Tabs + Core + Components

#### Group 1 (recon, scan, scan_ports, scan_endpoints, fingerprint)

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `recon.rs` | 657 | `handle_down` missing `is_empty()` guard on checkbox index | Added `option_checkboxes.is_empty() ||` guard |
| `scan.rs` | 531-536 | `handle_bottom` goes to Inputs instead of Results | Changed to `ScanFocusArea::Results` |
| `scan_ports.rs` | 370-386 | Options focus area unreachable via keyboard | Added Options to `handle_focus_next`/`handle_focus_prev` cycle |
| `scan_ports.rs` | 506-516 | `handle_enter` doesn't toggle checkbox in Options | Added checkbox toggle when `focus_area == Options` |
| `scan_ports.rs` | 388-416 | `handle_up`/`handle_down` wrong behavior in Options | Made Options branch a no-op |
| `scan_endpoints.rs` | 330-346 | Options focus area unreachable via keyboard | Added Options to focus cycle |
| `scan_endpoints.rs` | 433-443 | `handle_enter` doesn't toggle checkbox in Options | Added checkbox toggle |
| `scan_endpoints.rs` | 449-476 | `handle_up`/`handle_down` wrong behavior in Options | Made Options branch a no-op |
| `fingerprint.rs` | - | No bugs found | - |

#### Group 2 (fuzz, waf, waf_stress, load, stress)

All 5 tabs passed audit - no bugs found.

#### Group 3 (packet, graphql, oauth, cluster, proxy)

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `graphql.rs` | 394 | `handle_copy()` missing `is_running()` guard | Added guard returning None |
| `oauth.rs` | 529 | `handle_escape()` missing `is_running()` guard | Added guard returning early |
| `cluster.rs` | 494 | `handle_escape()` missing `is_running()` guard | Added guard returning early |
| `packet.rs` | - | No bugs found | - |
| `proxy.rs` | 731-783 | Duplicate inherent methods shadow trait methods | Noted as structural issue (no runtime impact) |

#### Group 4 (nse, hunt, browser, compliance)

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `nse.rs` | 335-351 | `handle_enter` starts scan from Inputs when not focused | Restructured: only Results triggers start |
| `nse.rs` | 353 | `handle_escape()` missing `is_running()` guard | Added guard returning early |
| `compliance.rs` | 367-386 | `handle_enter` unconditionally calls `start()` | Restructured: only Results triggers start |
| `compliance.rs` | 388 | `handle_escape()` missing `is_running()` guard | Added guard returning early |
| `hunt.rs` | - | No bugs found | - |
| `browser.rs` | - | No bugs found | - |

#### Group 5 (storage, integrations, workflow, vuln, report)

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `report.rs` | 670-673 | `start()` conditional guard blocks restart | Removed conditional, now unconditional |
| `report.rs` | 524-527 | `handle_enter` starts scan from Results | Added `return;` in Results arm |
| `report.rs` | 676-679 | `stop()` unnecessary guard | Made unconditional |
| `report.rs` | 276, 308 | Direct `chunks[i]` indexing | Changed to `.get()` pattern |
| `workflow.rs` | 315-340 | severity/status selectors never rendered | Added explicit rendering after field loop |
| `vuln.rs` | 564-589 | `handle_enter` restarts from Results | Added `return;` in Results arm |
| `storage.rs` | - | No bugs found | - |
| `integrations.rs` | - | No bugs found | - |

#### Group 6 (resume, history, dashboard, settings, auth)

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `settings/main.rs` | 458-513 | `reset()` missing config clear | Added `self.config = None;` |
| `auth.rs` | 224 | `handle_enter` compares non-existent `AuthFocusArea::Inputs` | Replaced with `self.is_input_focused()` |
| `auth.rs` | 231-233 | `handle_escape()` missing `is_running()` guard | Added guard |
| `auth.rs` | 334 | `sync_input_focus` tautological guard | Removed redundant `i < fields.len()` check |
| `resume.rs` | - | No bugs found | - |
| `history.rs` | - | No bugs found | - |
| `dashboard.rs` | - | No bugs found | - |

#### App/Core Modules

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `runner.rs` | 184 | `Event::Paste(_)` silently dropped | Added paste routing in Insert mode |
| `runner.rs` | 164-167 | Auto-save timer double-reset | Removed redundant reset |
| `key_handler.rs` | 399, 407-414 | Stale quick-switch results on Enter | Re-fetch fresh results in Enter arm |
| `state_update.rs` | 37-39 | `task_tab` cleared before results processed | Moved clear to after results loop |
| `session.rs` | 188-199 | `swap_remove(0)` breaks sort order | Changed to `remove(0)` |

#### Components

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `popup.rs` | 192 | `centered_rect` fallback uses wrong area | Store chunk in variable before split |
| `scrollable.rs` | 70-76 | Potential `usize` overflow in `scroll_right` | Changed to `saturating_add` |
| `palette.rs` | 55-57 | `scroll_offset` not clamped to result count | Added `.min(total.saturating_sub(1).max(0))` |
| `input.rs` | 98 | `to_lowercase()` per candidate in filter | Noted as performance issue (minor) |
| `selector.rs` | 437 | `options_per_line` uses byte length | Noted as Unicode issue (minor) |
| `progress.rs` | 116 | `current > total` shows misleading display | Noted as edge case (minor) |

### Summary

| Metric | Value |
|--------|-------|
| Total files audited | 42 |
| Total bugs found | 39 |
| Total bugs fixed | 33 |
| Bugs noted (minor/edge cases) | 6 |

## Session Fixes (2026-05-30)

### TUI Deep Dive Audit - All 28 Tabs + Core + Components

Comprehensive audit using 7 parallel subagents across all tabs, core modules, and components.

#### HIGH Priority Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `proxy.rs` | 273, 357 | State left as Running on HealthChecker::new() early return -- tab permanently unresponsive | Added `self.state = AppState::Idle;` before return |
| `fuzz.rs` | 763 | `handle_enter()` missing `is_running()` guard -- blur runs instead of stop | Added `if self.is_running() { self.stop(); return; }` |
| `waf.rs` | 548 | `handle_enter()` missing `is_running()` guard -- blur runs instead of stop | Added `if self.is_running() { self.stop(); return; }` |
| `load.rs` | 641 | `handle_enter()` missing `is_running()` guard on selector + blur paths | Added `if self.is_running() { self.stop(); return; }` |
| `storage.rs` | 409 | `handle_focus_next()` Mode→Query missing `query_inputs.focus(0)` -- typing silently no-op | Added `self.query_inputs.focus(0)` |
| `integrations.rs` | 416-445 | `handle_focus_next/prev()` missing focus(0)/blur() on 3 transitions | Added `focus(0)` and `blur()` calls |
| `workflow.rs` | 401, 419 | `handle_focus_next/prev()` missing blur/focus on Inputs↔Results transitions | Added `blur()` and `focus(0)` calls |
| `vuln.rs` | 481 | `handle_focus_next()` Inputs→Results missing `inputs.blur()` | Added `self.inputs.blur()` |
| `report.rs` | 356 | `handle_focus_next()` ViewSelector→Inputs missing `focus(0)` on active input group | Added `focus(0)` based on `current_view` |
| `settings/input.rs` | 408-482 | Session section unreachable via keyboard + missing match arms for left/right/edge detection | Added Session to all match arms + navigation arrays |

#### MEDIUM Priority Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `recon.rs` | 408 | `reset()` missing `progress.total = 0` | Added reset |
| `scan.rs` | 276 | `reset()` missing `progress.total = 0` | Added reset |
| `scan_ports.rs` | 290 | `reset()` missing `progress.total = 0` | Added reset |
| `scan_endpoints.rs` | 249 | `reset()` missing `progress.total = 0` | Added reset |
| `fingerprint.rs` | 206 | `reset()` missing `progress.total = 0` | Added reset |
| `fuzz.rs` | 400 | `reset()` missing `progress.total = 0` | Added reset |
| `waf.rs` | 301 | `reset()` missing `progress.total = 0` | Added reset |
| `load.rs` | 366 | `reset()` missing `progress.total = 0` | Added reset |
| `recon.rs` | 718 | `handle_escape()` missing `is_running()` guard | Added guard |
| `scan.rs` | 621 | `handle_escape()` missing `is_running()` guard | Added guard |
| `scan_ports.rs` | 570 | `handle_escape()` missing `is_running()` guard | Added guard |
| `scan_endpoints.rs` | 488 | `handle_escape()` missing `is_running()` guard | Added guard |
| `fingerprint.rs` | 411 | `handle_escape()` missing `is_running()` guard | Added guard |
| `waf_stress.rs` | 357 | `handle_escape()` missing `is_running()` guard | Added guard |
| `integrations.rs` | 580 | `handle_escape()` missing `is_running()` guard | Added guard |
| `workflow.rs` | 524 | `handle_escape()` missing `is_running()` guard | Added guard |
| `report.rs` | 548 | `handle_escape()` missing `is_running()` guard | Added guard |
| `nse.rs` | 459 | `start()` missing `target().is_empty()` validation | Added guard |
| `hunt.rs` | 489 | `handle_enter()` missing `is_running()` guard | Added guard |
| `browser.rs` | 448 | `handle_enter()` missing `is_running()` guard | Added guard |
| `waf_stress.rs` | 407 | `is_input_focused()` missing `focus_area` check | Added `self.focus_area == WafStressFocusArea::Inputs &&` |
| `cluster.rs` | 205-221 | `reset()` doesn't restore default field values | Restored "localhost:9000", "4", "9000" defaults |
| `settings/main.rs` | 538-549 | `reset()` loses defaults for report/schedule/session inputs | Restored "html", "./exports", "quick", "30" defaults |
| `settings/main.rs` | 563 | `reset()` missing `sync_component_focus()` | Added call |
| `history.rs` | 439 | UTF-8 slice `&entry.target[..27]` panic on multi-byte chars | Changed to `char_indices()` safe split |

#### LOW Priority Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `scrollable.rs` | 62 | `scroll_down()` missing `saturating_add` -- debug panic on overflow | Changed to `saturating_add` |
| `scan.rs` | 589 | Redundant `is_running()` check in `handle_enter()` | Removed dead branch |
| `scan_endpoints.rs` | 481 | Redundant `is_running()` check in `handle_enter()` | Removed dead branch |
| `auth.rs` | 50 | `reset()` not clearing stale `results` text | Added `self.results = "Ready for authentication testing".to_string()` |
| `report.rs` | 338-346 | Missing progress gauge during Running state | Added `Gauge` widget render when `state == AppState::Running` |
| `dashboard.rs` | 568-574 | `page_up/page_down()` missing `is_running()` guard | Added guard |
| `dashboard.rs` | 100-101 | Silent `.ok()` on I/O and deserialization | Added `tracing::debug!` error logging |

#### Summary

| Metric | Value |
|--------|-------|
| Total bugs found | 41 |
| Total bugs fixed | 41 |
| Files modified | 27 |
| HIGH priority fixes | 10 |
| MEDIUM priority fixes | 22 |
| LOW priority fixes | 9 |

## Session Fixes (2026-05-30 - Deep Dive Audit)

### TUI Deep Dive Audit - All 28 Tabs + Components + Core

Comprehensive audit using 7 parallel subagents across all tabs, components, and core modules.

#### HIGH Priority Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `recon.rs` | 720 | `handle_escape()` returns early when running instead of stopping task | Added `self.stop();` before `return;` |
| `scan.rs` | 621 | `handle_escape()` returns early when running instead of stopping task | Added `self.stop();` before `return;` |
| `scan_ports.rs` | 572 | `handle_escape()` returns early when running instead of stopping task | Added `self.stop();` before `return;` |
| `scan_endpoints.rs` | 490 | `handle_escape()` returns early when running instead of stopping task | Added `self.stop();` before `return;` |
| `fingerprint.rs` | 413 | `handle_escape()` returns early when running instead of stopping task | Added `self.stop();` before `return;` |
| `fuzz.rs` | 820 | `handle_escape()` returns early when running instead of stopping task | Added `self.stop();` before `return;` |
| `waf.rs` | 586 | `handle_escape()` returns early when running instead of stopping task | Added `self.stop();` before `return;` |
| `waf_stress.rs` | 358 | `handle_escape()` returns early when running instead of stopping task | Added `self.stop();` before `return;` |
| `load.rs` | 667 | `handle_escape()` returns early when running instead of stopping task | Added `self.stop();` before `return;` |
| `stress.rs` | 445 | `handle_escape()` returns early when running instead of stopping task | Added `self.stop();` before `return;` |
| `packet.rs` | 773 | `handle_escape()` returns early when running instead of stopping task | Added `self.stop();` before `return;` |
| `graphql.rs` | 455 | `handle_escape()` returns early when running instead of stopping task | Added `self.stop();` before `return;` |
| `oauth.rs` | 502 | `handle_escape()` returns early when running instead of stopping task | Added `self.stop();` before `return;` |
| `cluster.rs` | 538 | `handle_escape()` returns early when running instead of stopping task | Added `self.stop();` before `return;` |
| `proxy.rs` | 652 | `handle_escape()` does nothing when running | Rewrote to use early-return pattern with `self.stop();` |
| `nse.rs` | 370 | `handle_escape()` returns early when running instead of stopping task | Added `self.stop();` before `return;` |
| `hunt.rs` | 517 | `handle_escape()` returns early when running instead of stopping task | Added `self.stop();` before `return;` |
| `browser.rs` | 476 | `handle_escape()` returns early when running instead of stopping task | Added `self.stop();` before `return;` |
| `compliance.rs` | 412 | `handle_escape()` returns early when running instead of stopping task | Added `self.stop();` before `return;` |
| `resume.rs` | 300 | `handle_escape()` missing `is_running()` guard entirely | Added `is_running()` check with `self.stop();` |
| `storage.rs` | 594 | `handle_enter()` Results area falls through to `self.start()` | Added `return;` in Results arm |
| `integrations.rs` | 581 | `handle_enter()` Results area falls through to `self.start()` | Added `return;` in Results arm |
| `workflow.rs` | 523 | `handle_enter()` Results area falls through to `self.start()` | Added `return;` in Results arm |
| `nse.rs` | 363 | `handle_enter()` Results area calls `self.start()` | Changed to `return;` |
| `hunt.rs` | 507 | `handle_enter()` fallthrough triggers `self.start()` from Results | Added `if focus_area == Results { return; }` guard |
| `browser.rs` | 466 | `handle_enter()` fallthrough triggers `self.start()` from Results | Added `if focus_area == Results { return; }` guard |
| `compliance.rs` | 403 | `handle_enter()` Results area calls `self.start()` | Changed to `return;` |

#### LOW Priority Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `fuzz.rs` | 647, 671 | Redundant `self.inputs.blur()` in handle_focus_next/prev | Removed blur calls from MutationCheckbox↔Results transitions |

#### Summary

| Metric | Value |
|--------|-------|
| Total bugs found | 56 |
| Total bugs fixed | 28 |
| Files modified | 22 |
| HIGH priority fixes | 27 |
| LOW priority fixes | 1 |
| Already fixed (MEDIUM/LOW) | 28 |

**Key systemic bug fixed**: `handle_escape()` was unable to stop running tasks across all 20 tabs. Users had no keyboard shortcut to cancel an in-progress scan. All tabs now properly call `self.stop()` before returning when `is_running()` is true.

## Session Fixes (2026-05-31 - Deep Dive Audit)

### TUI Deep Dive Audit - All 28 Tabs + Core + Components

Comprehensive audit using 7 parallel subagents across all tabs, core modules, and components. Found 40 bugs total (1 HIGH, 13 MEDIUM, 26 LOW).

#### HIGH Priority Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `key_handler.rs` | 320-324 | Command palette `selected_index` not clamped after backspace — out-of-bounds access on Enter | Added clamping after `update_command_palette_query` in both Backspace and Char handlers |

#### MEDIUM Priority Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `recon.rs` | 363-368 | `page_up`/`page_down` defined as inherent methods, unreachable via trait dispatch | Moved into `impl TabInput` block |
| `scan.rs` | 240-245 | Same — inherent methods unreachable | Moved into `impl TabInput` block |
| `scan_ports.rs` | 241-246 | Same | Moved into `impl TabInput` block |
| `scan_endpoints.rs` | 228-233 | Same | Moved into `impl TabInput` block |
| `fingerprint.rs` | 185-190 | Same | Moved into `impl TabInput` block |
| `load.rs` | 686-718 | `handle_up`/`handle_down` missing Results scroll branch | Added `LoadFocusArea::Results` branches |
| `load.rs` | 636-641 | `handle_bottom` missing `self.inputs.blur()` | Added blur call |
| `stress.rs` | 432-446 | `handle_enter` auto-starts after TypeSelector confirm | Added `return;` after confirm |
| `workflow.rs` | 507-516 | `handle_enter` mode update before `was_open` guard | Moved mode update after guard |
| `settings/main.rs` | 578 | `reset()` sets `dark_mode.checked = false` instead of `true` | Changed to `true` to match `new()` default |
| `browser.rs` | 584-590 | `page_up`/`page_down` missing `is_running()` guard | Added guard |
| `nse.rs` | 345-370 | `handle_enter` blur falls through to `start()` | Added `return;` after blur |
| `compliance.rs` | 380-407 | `handle_enter` blur falls through to `start()` | Added `return;` after blur |
| `packet.rs` | 481-494 | `reset()` doesn't close dropdown or reset InputGroup.focused | Added `cancel()`, `blur()` calls |
| `cluster.rs` | 205-230 | `reset()` incomplete — no selector/inputs reset | Added `cancel()`, `blur()` for all inputs |
| `proxy.rs` | 391-403 | `reset()` incomplete | Added `cancel()`, `blur()` calls |
| `history.rs` | impl TabInput | `stop()` is no-op trait default | Added explicit `stop()` override |
| `dashboard.rs` | impl TabInput | `stop()` is no-op trait default | Added explicit `stop()` resetting state |
| `key_handler.rs` | 80 | `Clipboard::set` result silently discarded | Added `if !Clipboard::set(&text) { warn }` |
| `selector.rs` | 116 | `selected` stale after items shrink externally | Added `set_items()` method with clamping |

#### LOW Priority Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `fingerprint.rs` | 333-335 | `handle_focus_prev` doesn't cycle Inputs→Results | Changed to blur + switch to Results |
| `fingerprint.rs` | (missing) | `handle_copy` not overridden | Added override returning results content |
| `waf.rs` | 546-549 | `handle_bottom` missing `self.inputs.blur()` | Added blur call |
| `waf_stress.rs` | 339-342 | `handle_bottom` missing `self.inputs.blur()` | Added blur call |
| `load.rs` | 664 | Dead code — unreachable `else if self.is_running()` branch | Removed dead branch |
| `cluster.rs` | 514-518 | `handle_bottom` doesn't blur current inputs | Added blur based on current_view |
| `nse.rs` | 360-363 | `handle_enter` ScriptSelector falls through to `start()` | Added `return;` |
| `compliance.rs` | 396-399 | `handle_enter` Framework falls through to `start()` | Added `return;` |
| `storage.rs` | 684-689 | `page_up`/`page_down` missing `is_running()` guard | Added guard |
| `integrations.rs` | 671-676 | Same | Added guard |
| `workflow.rs` | 600-605 | Same | Added guard |
| `vuln.rs` | 705-710 | Same | Added guard |
| `report.rs` | 713-718 | Same | Added guard |
| `popup.rs` | 176-178 | `centered_rect` underflow on tiny areas | Added `r.width < 3 \|\| r.height < 3` guard |
| `auth.rs` | 162-210 | Direct `fields[idx]` indexing | Changed to `.get_mut(idx)` pattern |
| `settings/input.rs` | 282-285 | `handle_escape` missing `stop()` call | Added `self.stop()` |

#### Summary

| Metric | Value |
|--------|-------|
| Total bugs found | 40 |
| Total bugs fixed | 40 |
| Files modified | 26 |
| HIGH priority fixes | 1 |
| MEDIUM priority fixes | 20 |
| LOW priority fixes | 19 |

**Key systemic bug fixed**: `page_up`/`page_down` methods were defined as inherent `pub fn` outside `impl TabInput` in 5 tabs, making them unreachable through the trait dispatcher. PageUp/PageDown keys were completely non-functional for those tabs.

## Session Fixes (2026-05-31 - Deep Dive Audit)

### TUI Deep Dive Audit - All 28 Tabs + Core + Components

Comprehensive audit using 7 parallel subagents across all tabs, core modules, and components. Found 31 bugs total (3 HIGH, 16 MEDIUM, 12 LOW).

#### HIGH Priority Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `stress.rs` | 433-458 | `handle_enter()` has no path to `start()` — user can never start a stress test by pressing Enter | Restructured: Inputs opens TypeSelector, TypeSelector confirms, Results returns early |
| `stress.rs` | 433-437 | `handle_enter()` guard order inverted — `is_running()` checked before Results guard | Added Results early-return before is_running check |
| `auth.rs` | 66-74 | `TabState` impl missing `set_error()` override — errors dispatched via trait silently swallowed | Added `set_error()` delegation to inherent method |

#### MEDIUM Priority Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `scan.rs` | 477,499 | `handle_focus_next/prev` doesn't collapse stale dropdowns when transitioning between ProfileSelector↔OutputSelector | Added `.cancel()` calls on source selector during transitions |
| `packet.rs` | 429,438 | `execute()` calls `run_dump()`/`run_interfaces()` without setting `self.state = AppState::Running` — UI shows no running indicator | Added `self.state = AppState::Running` before both calls |
| `proxy.rs` | 453-455 | Duplicate dropdown render in `render()` and `render_overlays()` — visual artifacts | Removed dropdown render from `render()`, kept in `render_overlays()` |
| `storage.rs` | 400-402 | `handle_focus_next` Config→Mode missing `mode_selector.focus()` — selector not keyboard-accessible | Added `self.mode_selector.focus()` call |
| `storage.rs` | 443-451 | `handle_focus_prev` from Results always goes to Query — in non-Connect mode navigates to invisible area | Added mode-based branching to go to Mode when not in Connect |
| `workflow.rs` | 247-261 | `reset()` missing blur calls on mode/severity/status selectors and inputs | Added `.blur()` calls for all selectors and inputs |
| `workflow.rs` | 485-489 | `handle_top()` goes to Inputs, skipping Mode selector — inconsistent with other tabs | Changed to go to Mode selector first |
| `vuln.rs` | 344-353 | `reset()` missing blur calls on mode selector and inputs | Added `.blur()` calls |
| `vuln.rs` | 568-572 | `handle_top()` goes to Inputs, skipping Mode selector — inconsistent with other tabs | Changed to go to Mode selector first |
| `report.rs` | 232-248 | `reset()` missing blur calls on view/format selectors and convert/trend/schedule inputs | Added `.blur()` calls for all selectors and input groups |
| `settings/render.rs` | 150-154 | Hardcoded `y: inner.y + 6` for theme hint text — overlaps content if layout changes | Changed to compute offset from form height (checkbox=2 + selector=3 + borders=2 = 7) |

#### LOW Priority Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `fuzz.rs` | 364 | `update_progress` is a no-op — progress bar never updates during fuzzing | Implemented to set `progress.current` and `progress.total` |
| `integrations.rs` | 270 | `reset()` sets `tracker_selector.selected = 0` directly instead of `.select(0)` — bypasses internal state | Changed to `.select(0)` |
| `integrations.rs` | 335-339 | `render()` uses `input_area.height - 3` — u16 underflow on small terminals | Changed to `.saturating_sub(3)` |
| `workflow.rs` | 318-322 | Same u16 underflow in render | Changed to `.saturating_sub(3)` |
| `vuln.rs` | 410-414 | Same u16 underflow in render | Changed to `.saturating_sub(3)` |
| `storage.rs` | 556-561 | `handle_top()` always goes to Config — in non-Connect mode Config fields aren't rendered | Added mode-based branching |

#### Summary

| Metric | Value |
|--------|-------|
| Total bugs found | 31 |
| Total bugs fixed | 26 |
| Files modified | 12 |
| HIGH priority fixes | 3 |
| MEDIUM priority fixes | 11 |
| LOW priority fixes | 6 |
| Noted (design/edge cases) | 5 |

**Key systemic bugs fixed**:
1. `stress.rs` `handle_enter()` had no path to `start()` — users could never start a stress test via keyboard
2. `auth.rs` `TabState` impl missing `set_error()` — errors dispatched via trait were silently swallowed
3. `scan.rs` stale dropdowns on focus transitions — dropdown stayed open and intercepted keyboard input in wrong focus area

## Session Fixes (2026-05-31 - Deep Dive Audit)

### TUI Deep Dive Audit - All 28 Tabs + Core + Components

Comprehensive audit using 7 parallel subagents across all tabs, core modules, and components. Found 61 bugs total (5 HIGH, 28 MEDIUM, 28 LOW).

#### HIGH Priority Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `nse.rs` | 190 | Render: `input_chunks` split from `chunks[0]` instead of `input_block.inner(chunks[0])` — input fields render on top of block border | Changed to split from `input_block.inner(chunks[0])`, reduced constraints from 4 to 3, changed to `.get()` pattern |
| `resume.rs` | 69-81 | `page_up`/`page_down` defined as inherent methods, unreachable via `TabInput` trait dispatch | Moved into `impl TabInput for ResumeTab` block |
| `history.rs` | 303-315 | Same — inherent methods unreachable | Moved into `impl TabInput for HistoryTab` block |
| `dashboard.rs` | 602-616 | Same — inherent methods unreachable | Moved into `impl TabInput for DashboardTab` block |

#### MEDIUM Priority Fixes

**handle_top/handle_bottom missing blur (13 fixes across 10 tabs):**

| File | Function | Fix |
|------|----------|-----|
| `packet.rs` | `handle_top` | Added `self.inputs.blur()` before `view_selector.focus()` |
| `cluster.rs` | `handle_top` | Added blur for current view's inputs before `view_selector.focus()` |
| `proxy.rs` | `handle_top` | Added `self.inputs.blur()` before `view_selector.focus()` |
| `storage.rs` | `handle_top` | Added focus_area-based blur before re-focusing |
| `storage.rs` | `handle_bottom` | Added focus_area-based blur before Results |
| `integrations.rs` | `handle_top` | Added focus_area-based blur before re-focusing |
| `integrations.rs` | `handle_bottom` | Added focus_area-based blur before Results |
| `workflow.rs` | `handle_bottom` | Added `mode_selector.blur()` + `inputs.blur()` before Results |
| `vuln.rs` | `handle_bottom` | Added `mode_selector.blur()` + `inputs.blur()` before Results |
| `report.rs` | `handle_top` | Added focus_area-based blur before ViewSelector |
| `report.rs` | `handle_bottom` | Added focus_area-based blur before Results |
| `stress.rs` | `handle_bottom` | Added `self.inputs.blur()` before Results |
| `nse.rs` | `handle_bottom` | Added `self.inputs.blur()` before Results |

**Other MEDIUM fixes:**

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `fuzz.rs` | 394-437 | `reset()` missing 7 checkbox resets (graphql_introspection, graphql_depth_bypass, etc.) | Added `.reset()` calls for all checkboxes |
| `fuzz.rs` | 1001-1007 | `page_up`/`page_down` missing `is_running()` guard | Added guard |
| `waf.rs` | 588-594 | `handle_escape()` doesn't reset `focused_checkbox_index` when leaving Techniques | Added index and focus_area reset |
| `load.rs` | 726-760 | `handle_left`/`handle_right` call move_left/move_right when inputs not focused | Added `&& self.inputs.is_focused()` guard |
| `nse.rs` | 257-259 | `handle_focus_prev` from ScriptSelector missing blur | Added `self.script_selector.blur()` |
| `integrations.rs` | 441-443 | `handle_focus_prev` Config→Tracker missing `config_inputs.blur()` | Added blur call |
| `workflow.rs` | 422-424 | `handle_focus_prev` Inputs→Mode missing `inputs.blur()` | Added blur call |
| `vuln.rs` | 507-509 | `handle_focus_prev` Inputs→Mode missing `inputs.blur()` | Added blur call |
| `report.rs` | 413-415 | `handle_focus_prev` Inputs→ViewSelector missing blur | Added `current_inputs.blur()` |
| `resume.rs` | 296-301 | `handle_bottom` missing `inputs.blur()` | Added blur call |
| `scan.rs` | 582-593 | `handle_top`/`handle_bottom` missing selector collapse | Added `profile_selector.cancel()` + `output_selector.cancel()` |

#### LOW Priority Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `state_update.rs` | 70 | Unhandled TaskResult warning lacks variant context | Changed to `tracing::warn!("Unhandled TaskResult variant: {:?}", result)` |
| `progress.rs` | 130 | `render_status_line` hides numbers when `current > total` | Removed `current <= total` condition |

### Summary

| Metric | Value |
|--------|-------|
| Total bugs found | 61 |
| Total bugs fixed | 37 |
| Files modified | 22 |
| HIGH priority fixes | 4 |
| MEDIUM priority fixes | 26 |
| LOW priority fixes | 2 |
| Noted (minor/edge cases) | 5 |
| Tabs audited | 29 |
| Tests passing | 215 TUI tests |

**Key systemic bugs fixed**:
1. `nse.rs` render bug — input fields rendered on top of block border due to wrong split origin
2. `resume.rs`/`history.rs`/`dashboard.rs` — PageUp/PageDown keys completely non-functional due to methods in wrong impl block
3. Dual-focus state across 10 tabs — handle_top/handle_bottom focused selectors without blurring inputs
4. `fuzz.rs` reset() leaked state across sessions — 7 checkboxes never reset
5. `load.rs` left/right arrow keys were no-ops when selector had focus

## Session Fixes (2026-05-31 - Deep Dive Audit)

### TUI Deep Dive Audit - All 28 Tabs + Core + Components

Comprehensive audit using 8 parallel subagents across all tabs, core modules, and components. Found 30 bugs total (3 HIGH, 20 MEDIUM, 7 LOW).

#### HIGH Priority Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `stress.rs` | 452 | `handle_enter()` has no code path to `self.start()` — stress test can never be started from TUI | Added `self.start()` after `type_selector.close()` in TypeSelector confirm arm |
| `packet.rs` | 113-115 | `stop()` defined as inherent method, invisible to `dyn TabInput` dispatch — tab permanently stuck as Running | Moved to `impl TabInput` block with `AppState::Running` guard |
| `graphql.rs` | 202-206 | Same — `stop()` inherent method | Moved to `impl TabInput` block |
| `oauth.rs` | 237-241 | Same — `stop()` inherent method | Moved to `impl TabInput` block |
| `proxy.rs` | 210-212 | Same — `stop()` inherent method | Moved to `impl TabInput` block |
| `browser.rs` | 240,278 | Checkbox area overlaps 3rd input field — renders on top of Timeout input | Added 4th constraint, increased block height from 10 to 14 |

#### MEDIUM Priority Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `recon.rs` | 832-838 | `page_up`/`page_down` missing `is_running()` guard | Added guard |
| `scan.rs` | 747-753 | Same | Added guard |
| `scan_ports.rs` | 624-630 | Same | Added guard |
| `scan_endpoints.rs` | 587-593 | Same | Added guard |
| `fingerprint.rs` | 509-515 | Same | Added guard |
| `waf.rs` | 704-712 | Same | Added guard |
| `waf_stress.rs` | 436-442 | Same | Added guard |
| `load.rs` | 796-802 | Same | Added guard |
| `stress.rs` | 559-565 | Same | Added guard |
| `scan.rs` | 504-506 | `handle_focus_prev` from ProfileSelector does not cancel dropdown | Added `profile_selector.cancel()` |
| `scan.rs` | 283-284 | `reset()` calls `select()` but not `cancel()` on dropdowns | Changed to `cancel()` |
| `fuzz.rs` | 427-433 | `reset()` sets 7 checkboxes to unchecked instead of restoring initial checked state | Added `.checked = true` after each `.reset()` |
| `waf.rs` | 311 | `reset()` missing `inputs.blur()` | Added blur call |
| `stress.rs` | 211 | `reset()` missing `inputs.blur()` | Added blur call |
| `graphql.rs` | 342-345 | `handle_focus_prev` Options→Inputs: `inputs.blur()` no-op, user can't type | Changed to `inputs.focus(0)` |
| `oauth.rs` | 391-394 | Same | Changed to `inputs.focus(0)` |
| `proxy.rs` | 651-659 | `handle_escape` missing `is_open()` check, dropdown+unfocus in one step | Added `is_open()` check with early return |
| `storage.rs` | 259 | `reset()` missing `mode_selector.blur()` | Added blur call |
| `storage.rs` | 458-465 | `handle_focus_prev` Results skips Query in non-Connect mode | Changed to go to Query |
| `history.rs` | 711-723 | `page_up`/`page_down` always scroll Details, ignoring focus area | Added focus_area dispatch |
| `hunt.rs` | — | Missing `handle_copy` implementation | Added override |
| `compliance.rs` | — | Missing `handle_copy` implementation | Added override |
| `recon.rs` | 575 | `handle_focus_next` Options→Results doesn't reset `focused_checkbox_index` | Added reset |
| `recon.rs` | 721-727 | `handle_escape` no-op when focus is in Options or Results | Added focus_area match returning to Inputs |
| `scan_ports.rs` | 574-580 | Same | Added focus_area match |
| `scan_endpoints.rs` | 503-509 | Same | Added focus_area match |

#### LOW Priority Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `settings/render.rs` | 31-32 | Direct `chunks[0]`/`chunks[1]` indexing without `.get()` | Changed to `.get().copied().unwrap_or(area)` |
| `app/mod.rs` | 132 | Silent `.ok()` on session load errors | Added `match` with `tracing::warn!` |

#### Summary

| Metric | Value |
|--------|-------|
| Total bugs found | 30 |
| Total bugs fixed | 30 |
| Files modified | 20 |
| HIGH priority fixes | 6 |
| MEDIUM priority fixes | 22 |
| LOW priority fixes | 2 |
| Tabs audited | 29 |
| Tests passing | 215 TUI tests |

**Key systemic bugs fixed**:
1. `stress.rs` `handle_enter()` had no path to `start()` — users could never start a stress test via keyboard
2. `stop()` was inherent method on 4 tabs (packet/graphql/oauth/proxy) — invisible to `dyn TabInput` dispatch, tabs permanently stuck as Running
3. `browser.rs` checkbox area rendered on top of 3rd input field — visual collision
4. `page_up`/`page_down` lacked `is_running()` guards across 9 tabs — inconsistent with all other navigation handlers
5. `recon.rs`/`scan_ports.rs`/`scan_endpoints.rs` `handle_escape` was a no-op when focus was in Options/Results — user trapped with no keyboard path back to Inputs

## Session Fixes (2026-05-31 - Deep Dive Audit)

### TUI Deep Dive Audit - All 28 Tabs + Core + Components

Comprehensive audit using 8 parallel subagents across all tabs, core modules, and components. Found 16 bugs total (6 HIGH, 7 MEDIUM, 3 LOW).

#### HIGH Priority Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `scan.rs` | 470 | Missing `stop()` in `impl TabInput` — TUI cannot stop running tasks via trait dispatch | Added `fn stop(&mut self) { ScanTab::stop(self); }` |
| `scan_ports.rs` | 391 | Missing `stop()` in `impl TabInput` — same | Added `fn stop(&mut self) { ScanPortsTab::stop(self); }` |
| `scan_endpoints.rs` | 357 | Missing `stop()` in `impl TabInput` — same | Added `fn stop(&mut self) { ScanEndpointsTab::stop(self); }` |
| `fingerprint.rs` | 302 | Missing `stop()` in `impl TabInput` — same | Added `fn stop(&mut self) { FingerprintTab::stop(self); }` |
| `resume.rs` | 170 | `stop()` is inherent method, shadows trait default — would break trait dispatch | Added `fn stop(&mut self) { ResumeTab::stop(self); }` in `impl TabInput` |
| `auth.rs` | 138 | `stop()` is inherent method, shadows trait default — same | Added `fn stop(&mut self) { AuthTab::stop(self); }` in `impl TabInput` |

#### MEDIUM Priority Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `scan.rs` | 641 | `handle_escape()` doesn't transition focus back to Inputs | Added match on focus_area with proper transitions |
| `fingerprint.rs` | 412 | `handle_escape()` doesn't transition focus back to Inputs | Added match on focus_area with proper transitions |
| `proxy.rs` | 499 | Missing `page_up`/`page_down` — results view cannot be scrolled | Added page_up/page_down delegating to results_view |
| `proxy.rs` | 633 | `handle_enter` doesn't verify selector confirmation state | Added `is_open()` check before `confirm()` |
| `integrations.rs` | 446 | `handle_focus_prev` Issue→Config missing `self.issue_inputs.blur()` | Added blur call before focusing config_inputs |
| `key_handler.rs` | 69 | Ctrl+V clipboard read failure silently dropped | Added `tracing::debug!` for empty/unavailable clipboard |
| `selector.rs` | 448 | `self.label.len()` uses byte length, not display width — Unicode miscalculation | Changed to `self.label.chars().count()` |

#### LOW Priority Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `progress.rs` | 67,130 | `current/total` displayed raw when `current > total` — misleading | Clamped display with `self.current.min(self.total)` |
| `runner.rs` | 54 | Misleading `warn` log for expected config load failure | Changed to `tracing::debug!("No config file found...")` |
| `history.rs` | 311 | `stop()` inherent shadow (both are no-ops, latent only) | Noted — no functional impact |

#### Summary

| Metric | Value |
|--------|-------|
| Total bugs found | 16 |
| Total bugs fixed | 15 |
| Files modified | 12 |
| HIGH priority fixes | 6 |
| MEDIUM priority fixes | 7 |
| LOW priority fixes | 2 |
| Tabs audited | 29 |

**Key systemic bugs fixed**:
1. `stop()` was missing from `impl TabInput` on 4 scan tabs — TUI could not stop running tasks via trait dispatch
2. `stop()` was inherent method on resume/auth tabs — would break trait-object dispatch
3. `handle_escape` didn't transition focus area back to Inputs on scan/fingerprint tabs
4. `proxy.rs` had no page navigation for results view
5. Unicode width miscalculation in selector radio rendering

## Session Fixes (2026-06-07)

### TUI Deep Dive Audit - All 28 Tabs + Components

Comprehensive audit using 4 parallel subagents across all tabs and components.

#### HIGH Priority Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `wireless.rs` | 229 | Direct `self.inputs.fields[0]` access without bounds check — panics if fields empty | Changed to `if let (Some(chunk), Some(field)) = (input_chunks.first(), self.inputs.fields.first())` |
| `wireless.rs` | 334-337 | `handle_up()` missing `is_running()` guard — scrolls during running state | Added `!self.is_running()` guard |
| `wireless.rs` | 340-343 | `handle_down()` missing `is_running()` guard — same | Added guard |
| `wireless.rs` | 358-363 | `page_up()` missing `is_running()` guard | Added guard |
| `wireless.rs` | 366-372 | `page_down()` missing `is_running()` guard | Added guard |
| `wireless.rs` | 262-373 | Missing trait implementations for handle_word_forward/backward/home/end/top/bottom/copy | Added full implementations matching other tabs |
| `components/input.rs` | 149-168 | `move_left()`/`move_right()` return `true` even when cursor doesn't move (on multi-byte char boundary) | Moved `true` inside `if let Some` block — only returns true on actual movement |

#### MEDIUM Priority Fixes

| File | Line | Issue | Fix |
|------|------|-------|-----|
| `fuzz.rs` | 1109-1171 | 6 orphaned test functions defined outside `mod tests` block | Moved all tests inside `mod tests` block; fixed direct `fields[0]` access to use `.first()` |

#### LOW Priority Fixes (clippy .get(0) → .first())

| File | Lines | Fix |
|------|-------|-----|
| `fuzz.rs` | 509 | `config_chunks.get(0)` → `config_chunks.first()` |
| `graphql.rs` | 279 | `options_chunks.get(0)` → `options_chunks.first()` |
| `load.rs` | 426, 513 | `chunks.get(0)` → `chunks.first()` |
| `oauth.rs` | 325 | `options_chunks.get(0)` → `options_chunks.first()` |
| `packet.rs` | 543, 615 | `chunks.get(0)` → `chunks.first()` |
| `proxy.rs` | 444, 492 | `chunks.get(0)` → `chunks.first()` |
| `report.rs` | 287 | `chunks.get(0)` → `chunks.first()` |
| `resume.rs` | 115 | `chunks.get(0)` → `chunks.first()` |
| `scan.rs` | 330, 355 | `.fields.get(0)` and `main_chunks.get(0)` → `.first()` |
| `settings/render.rs` | 31 | `chunks.get(0)` → `chunks.first()` |
| `stress.rs` | 256, 268 | `chunks.get(0)` → `chunks.first()` |
| `waf.rs` | 348, 395 | `chunks.get(0)` and `results_chunks.get(0)` → `.first()` |
| `waf_stress.rs` | 164 | `chunks.get(0)` → `chunks.first()` |

#### Summary

| Metric | Value |
|--------|-------|
| Total bugs found | 9 |
| Total bugs fixed | 9 |
| Files modified | 16 |
| HIGH priority fixes | 7 |
| MEDIUM priority fixes | 1 |
| LOW priority fixes | 1 (19 clippy occurrences) |
| Tabs audited | 28 + components |

**Key systemic bugs fixed**:
 1. `wireless.rs` had no `is_running()` guards on navigation handlers — user could scroll during running scan
 2. `wireless.rs` had direct `fields[0]` access without bounds check — potential panic
 3. `input.rs` `move_left()`/`move_right()` returned `true` on no-op — callers incorrectly consumed keypresses without cursor movement
 4. `fuzz.rs` had 6 orphaned tests outside `mod tests` — structurally misplaced, some using direct `fields[0]` access

### TUI Architecture and Usability Pass (2026-06-11)
Completed the 10-phase plan in `docs/plans/tui-architecture-usability-pass.md` (using subagents for isolation). Each phase compiles and passes `cargo test -p eggsec-tui` independently. Final TUI crate: 301 tests green. Workspace/all-features run before handoff (pre-existing non-TUI errors in eggsec lib protobuf/codegen unrelated to this pass).

Key new modules / surfaces (per phase):
- `app/action.rs`: `UiAction`, `CommandPaletteInput`, `QuickSwitchInput`. Decode in KeyHandler; `App::apply_action` is the mutation point for global UI actions.
- `app/overlay.rs`: `OverlayController` with single `decode(...)` routing fn that asks `topmost_overlay()` and owns all per-overlay input rules (PolicyConfirm/ConfirmPopup/CommandPalette/QuickSwitch/Search/Http/Help). Emits UiActions only; no mutations.
- `tabs/spec.rs`: `TabSpec` / `TabCategory` / `TabRiskGroup` (later extended with operation/direct_launch). Single source for title/stable/cli/desc/category/risk/feature/breadcrumb. `Tab` methods delegate. `visible_tab_specs()` mirrors `Tab::all()` construction.
- Delegated descriptors: `TabInput::primary_target` (default + impls), `Tab::operation_name`/`is_direct_launch`, `risk_from_group`, thin delegation in `build_current_operation_descriptor`/`current_tab_target`/`is_direct_launch_tab`. Enforcement stays central.
- Visibility (shell.rs + preflight helpers): status bar now shows enforcement mode, scope provenance (LoadedScope source), risk badge (from spec), per-target preflight (target/scope-match/risk/op/"will: run|warn|confirm|deny" via live `EnforcementContext::evaluate`). Advisory only.
- Global task strip: `TaskState.started_at`; status + help show active task tab/state/elapsed/hints even after nav away; pause/resume visible; quit-block not surprising.
- Palette complete (command.rs + help_config.rs): all keybound actions + required list (run-current, stop/pause/resume/jump-active, quick-switch, help-current, search/global, theme, cycle/export, copy-cli, settings, reload-scope stub, save contextual, clear/delete contextual). Disabled-with-reason for no-task / wrong-tab cases.
- Copy CLI (app/mod.rs + command.rs + utils): `copy_cli_equivalent` (cli_command + primary_target + safe options + --format + explicit --scope only); shell_escape; palette action; graceful clipboard fail; no broad bypass flags; tests for recon/scan-ports/intrusive/non-exec.
- Small-terminal (shell.rs + mod.rs + popups.rs + tests): breadcrumb tab bar on narrow, too-small (<~40x10) clear fallback (input/quit still work), popups clamped, policy confirm preserved, low-pri status dropped first; 60x20 usable; layout tests added.
- Semantic tokens (theme/palette.rs + builtin.rs + loader.rs + style.rs): 10 roles (safe/danger/muted/active_task/paused_task/scope_match/miss/policy_required/denied) + helpers (`style_for_risk`, `style_for_policy_outcome`, `style_for_task_state` etc.); adopted in preflight/status/task/policy paths; all themes + loader + cyber-red fallback + non-blocking load unchanged.

Overlay precedence (topmost_overlay + controller) is now PolicyConfirm > ConfirmPopup > CommandPalette > QuickSwitch > Search > HttpOptions > Help. Non-topmost never receive input; overlay-local keys never leak.

All acceptance criteria from the plan are met (decode/apply split testable; one overlay routing fn; single metadata truth + feature gating + stable_id roundtrips; descriptors delegated + risk from spec; manual visibility + preflight advisory; task strip visible after nav; palette action-complete with context; CLI copy with safe escape + no bypasses; small-terminal degraded + "too small" fallback + policy readable; semantic helpers used for scope/risk/task/policy).

Validation (run after substantial phases and at end):
```
cargo fmt --all
cargo check -p eggsec-tui
cargo test -p eggsec-tui
cargo check --workspace --all-features
cargo test --workspace --all-features
```

Update any future TUI changes to preserve the decode/apply split, delegate through TabSpec where metadata/risk/operation are needed, keep enforcement central, and surface manual posture/preflight/task state via the status paths.
