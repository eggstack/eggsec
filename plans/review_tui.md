# TUI Architecture Review

**Document:** architecture/tui.md
**Reviewed:** 2026-06-02
**Accuracy:** Medium
**Lines Reviewed:** 1849

## Verified Claims

### Core Structure
- **Tab enum has 28 variants (0-27)**: Verified at `crates/slapper/src/tui/tabs/mod.rs:79-109`
- **OverlayType enum (6 variants)**: Verified at `crates/slapper/src/tui/app/mod.rs:804-811`
- **InputMode enum (Normal, Insert)**: Verified at `crates/slapper/src/tui/app/input.rs:2-6`
- **PendingAction enum (4 variants)**: Verified at `crates/slapper/src/tui/app/confirmation.rs:4-9`
- **NotificationSeverity enum (4 variants)**: Verified at `crates/slapper/src/tui/app/notifications.rs:2-7`
- **AppState enum (4 variants)**: Verified at `crates/slapper/src/tui/tabs/mod.rs:822-827`
- **TabInput trait methods**: Verified at `crates/slapper/src/tui/tabs/mod.rs:849-887` (27 methods total: 25 own + 2 from TabState without defaults)

### Entry Point
- **handle_no_command() calls tui::run()**: Verified at `crates/slapper/src/commands/handlers/mod.rs:197-205`
- **Terminal check before launch**: Verified at `crates/slapper/src/commands/handlers/mod.rs:198`
- **No --tui flag exists**: Confirmed via grep - no matches for `--tui` in codebase

### Session Management
- **SessionManager auto-save interval default 30s**: Verified at `crates/slapper/src/tui/session.rs:46`
- **Session save to ~/.slapper/sessions/**: Verified at `crates/slapper/src/tui/session.rs:54-56`

### Workers
- **7 worker files**: Verified at `crates/slapper/src/tui/workers/mod.rs:1-8` (api, fuzzer, network, recon, runner, scanner, security)
- **TaskConfig/TaskResult exported from runner**: Verified at `crates/slapper/src/tui/workers/mod.rs:9`

### Key Bindings (selected verification)
- **Ctrl+C interrupt/quit**: Verified at `crates/slapper/src/tui/app/key_handler.rs:47`
- **Ctrl+P command palette**: Verified at `crates/slapper/src/tui/app/key_handler.rs:65`
- **Ctrl+X quick switch**: Verified at `crates/slapper/src/tui/app/key_handler.rs:48-52`
- **Ctrl+F global search**: Verified at `crates/slapper/src/tui/app/key_handler.rs:66`
- **Space toggle help**: Verified at `crates/slapper/src/tui/app/key_handler.rs:110`
- **hjkl navigation**: Verified at `crates/slapper/src/tui/app/key_handler.rs:119-122`
- **i enter insert mode**: Verified at `crates/slapper/src/tui/app/key_handler.rs:108`
- **Esc close overlay**: Verified at `crates/slapper/src/tui/app/key_handler.rs:63`
- **q quit**: Verified at `crates/slapper/src/tui/app/key_handler.rs:109`
- **g/G top/bottom**: Verified at `crates/slapper/src/tui/app/key_handler.rs:124-125`
- **n/N next/prev tab**: Verified at `crates/slapper/src/tui/app/key_handler.rs:128-129`
- **e export results**: Verified at `crates/slapper/src/tui/app/key_handler.rs:138`
- **s save settings**: Verified at `crates/slapper/src/tui/app/key_handler.rs:136`

### Components
- **12 component files**: Verified via glob (progress.rs, selector.rs, popup.rs, palette.rs, scrollable.rs, input.rs, notifications.rs, mod.rs, http_options.rs, help_bar.rs, search_popup.rs, empty_state.rs)

### UI Draw Functions
- **draw() function exists**: Verified at `crates/slapper/src/tui/ui.rs:17`
- **draw_tabs(), draw_breadcrumb(), draw_content(), draw_status_bar()**: Verified at `crates/slapper/src/tui/ui.rs:31-34`

## Discrepancies

### 1. TabState trait claims 4 methods but has 5
**Documented:** "Inherits from TabState (4 methods): state(), progress(), reset(), set_error()" (line 106)
**Actual:** TabState trait has 5 methods: `state()`, `progress()`, `is_running()`, `reset()`, `set_error()` (`crates/slapper/src/tui/tabs/mod.rs:830-838`)

### 2. Tab count mismatch in documentation table
**Documented:** "28 specialized tabs for different security testing functions" (line 32) and a table listing tabs from Recon to Settings
**Actual:** The table lists 27 tabs, not 28. The enum has 28 variants, but the documentation table appears to be missing one entry.
**Location:** `architecture/tui.md:34-63`

### 3. Enum ordering doesn't match Tab::all() ordering
**Documented:** Tab ordering implied by table and enum discriminants (Nse at position 17 in enum)
**Actual:** In `Tab::all()` (`crates/slapper/src/tui/tabs/mod.rs:211-287`), Nse is appended at the end (after Dashboard) under `#[cfg(feature = "nse")]`, not at position 17 where the enum discriminant places it. Similarly, Settings (enum 18) comes before History (enum 19) in the enum but History comes before Settings in `all()`.
**Impact:** Low - This affects tab cycling order and visible index when NSE feature is enabled

### 4. Tab Traits section location
**Documented:** Tab Traits section (lines 65-70) appears to be inside the tabs table
**Actual:** The "**Tab Traits**" header at line 65 appears after the table but visually could be confused as part of it

## Bugs Found

### 1. Overlay precedence not enforced in key_handler
**Description:** The document states overlay precedence (highest first): ConfirmPopup, CommandPalette, QuickSwitch, Search, HttpOptions, Help. However, `handle_topmost_overlay()` in `key_handler.rs` processes overlays sequentially and returns on the first match, meaning the order of checking determines precedence, not an explicit priority.
**Location:** `crates/slapper/src/tui/app/key_handler.rs:176-277`
**Severity:** Low - In practice, only one overlay is active at a time due to App using separate boolean fields

### 2. Tab::all() inconsistent ordering with enum discriminants
**Description:** When NSE feature is enabled, Nse tab is appended to the end of `Tab::all()` but has enum discriminant 17. This means `Tab::from_discriminant(17)` returns Nse, but `Tab::all()[17]` returns Settings (when NSE enabled).
**Location:** `crates/slapper/src/tui/tabs/mod.rs:211-333`
**Severity:** Low - Session persistence uses stable IDs, not discriminants

## Improvement Opportunities

### 1. Document TabInput method required vs default count
**Description:** The TabInput trait section claims "TabInput Interface (27 methods)" but doesn't clearly distinguish between required methods (11) and default methods (16). Consider clarifying.
**Priority:** Low

### 2. Add feature-gated tab count clarification
**Description:** The document says "28 tabs" but this is only true when all 8 conditional features are enabled. The base count is 20 tabs. Consider noting this conditional count.
**Priority:** Low

### 3. Clarify TabState inheritance count
**Description:** Document says TabState has 4 methods but it actually has 5 (including `is_running()`).
**Priority:** Medium

## Stale Items

### 1. Historical bug fix tables
**Description:** The document contains extensive historical bug fix tables from sessions spanning 2026-05-30 to 2026-06-10. These are valuable for understanding evolution but:
- Many entries are marked "fixed" which means they're already resolved
- The tables are very long (800+ lines) and may become stale
- The "Session Fixes" headers (lines 886-1849) could be moved to a separate historical document

**Recommended action:** Consider moving historical fix logs to a separate `ARCHITECTURE_TUI_HISTORY.md` file to keep the main architecture document focused on current state.

### 2. Bug Patterns section may be outdated
**Description:** The "Bug Patterns to Avoid" section (lines 327-884) documents patterns that have been fixed. While educational, some patterns may no longer be relevant if they're now enforced via lints or removed from the codebase.
**Recommended action:** Verify which patterns are still possible issues vs which are now caught by lints or fixed in the codebase.

## Code Interrogation Findings

### 1. Feature-gated tabs create variable tab count
**Finding:** The actual visible tab count depends on which features are enabled:
- Minimum (no features): 20 tabs
- Maximum (all features): 28 tabs
- The `Tab::all()` function correctly handles this via `#[cfg]` attributes

### 2. AuthTab is not a TUI tab
**Finding:** Confirmed at `crates/slapper/src/tui/tabs/auth.rs` - AuthTab exists, implements TabInput/TabState, but is NOT part of the Tab enum and is only accessible via CLI. Document correctly notes this.

### 3. Quick switch clamping re-fetches results
**Finding:** The document describes the "correct" pattern for quick switch clamping (re-fetching fresh results). Verified this pattern exists at `crates/slapper/src/tui/app/key_handler.rs:407-414`.

### 4. Key handler uses sequential overlay checking
**Finding:** Only one overlay can be active at a time due to App using separate boolean fields (`show_help`, `show_quick_switch`, `show_search`, `show_http_options`, `command_palette: Option<CommandPalette>`, `pending_action: Option<PendingAction>`). The overlay precedence in the document is therefore mostly theoretical.

## Summary

| Category | Count |
|----------|-------|
| Total lines in document | 1849 |
| Verified accurate claims | ~40 |
| Discrepancies found | 3 |
| Bugs found | 2 |
| Improvement opportunities | 3 |
| Stale items | 2 |

**Overall Assessment:** The document is generally accurate and comprehensive, covering the TUI architecture well. The main issues are:
1. TabState method count mismatch (4 documented vs 5 actual)
2. Tab table listing 27 tabs but claiming 28
3. Historical fix tables making the document very long and potentially stale

The document correctly captures the major architectural decisions, key bindings, event loop structure, and component organization. The bug pattern section is educational but may benefit from being trimmed or separated.