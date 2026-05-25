# TUI (Terminal User Interface)

Slapper includes a powerful real-time Terminal User Interface (TUI) built with the `ratatui` crate. It provides an interactive way to monitor and control ongoing security scans across 29 different tabs.

## Core Components (`src/tui/`)

### App & UI (`app/`)

Manages the overall application state, event loop, and rendering.

| File | Purpose |
|------|---------|
| `mod.rs` | `App` struct - central state container holding all tabs, mode, overlays, theme |
| `runner.rs` | Main event loop using crossterm/ratatui |
| `key_handler.rs` | Priority-based key processing (pending combos → overlays → global → mode) |
| `dispatch.rs` | Routes input to current tab via `TabDispatcher` |
| `state_update.rs` | Async task result handling and routing |
| `task_management.rs` | Maps tabs to `TaskConfig` for background execution |
| `task_runtime.rs` | Task lifecycle management (spawn, stop, clear) |

### Tabs (`tabs/`)

29 specialized tabs for different security testing functions:

| Tab | File | Purpose |
|-----|------|---------|
| Recon | `recon.rs` | Domain/IP reconnaissance (DNS, WHOIS, SSL, tech detection) |
| Scan | `scan.rs` | Multi-stage security assessment pipeline |
| Scan Ports | `scan_ports.rs` | TCP port scanning |
| Scan Endpoints | `scan_endpoints.rs` | Sensitive endpoint discovery |
| Fingerprint | `fingerprint.rs` | Service fingerprinting (AMAP-style) |
| Fuzz | `fuzz.rs` | Security fuzzing with 31 payload types |
| WAF | `waf.rs` | WAF detection and bypass |
| WAF Stress | `waf_stress.rs` | Comprehensive WAF stress testing |
| Load | `load.rs` | HTTP load testing |
| Stress | `stress.rs` | Stress/load testing |
| Packet | `packet.rs` | Packet capture, send, traceroute |
| GraphQL | `graphql.rs` | GraphQL security testing |
| OAuth | `oauth.rs` | OAuth/OIDC vulnerability testing |
| Cluster | `cluster.rs` | Distributed scanning cluster management |
| Proxy | `proxy.rs` | Proxy pool management |
| NSE | `nse.rs` | Nmap NSE script execution |
| Plugin | `plugin.rs` | Python/Ruby security plugin runner |
| Hunt | `hunt.rs` | Intelligent vulnerability hunting |
| Browser | `browser.rs` | Headless browser security testing |
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

**Tab Traits** (`tabs/mod.rs`):
- `TabState` - State: `state()`, `progress()`, `reset()`, `set_error()`
- `TabInput` - Input: `handle_focus_next()`, `handle_char()`, `handle_enter()`, etc.
- `TabRender` - Rendering: `render()`, `render_overlays()`, `breadcrumb()`

### Components (`components/`)

Reusable UI primitives:

| Component | File | Purpose |
|-----------|------|---------|
| `InputField` | `input.rs` | Text input with cursor, validation, UTF-8 handling |
| `InputGroup` | `input.rs` | Group of inputs with focus navigation |
| `Selector` | `selector.rs` | Dropdown selector with keyboard navigation |
| `Checkbox` | `selector.rs` | Toggle checkbox |
| `RadioGroup` | `selector.rs` | Radio button group |
| `ProgressGauge` | `progress.rs` | Animated progress bar with spinner |
| `ScrollableText` | `scrollable.rs` | Scrollable text with scrollbar |
| `Popup` | `popup.rs` | Modal dialogs (confirm, help, info) |

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

### Theme (`theme.rs`)

`ThemeManager` holds dark/light themes with 30+ color fields.

Use `tc!` macro for theme colors:
```rust
use crate::tc;
let style = Style::default().fg(tc!(text));
```

### Session Management (`session.rs`)

`SessionManager` auto-saves at the configured interval (default 30 seconds) to JSON in `~/.slapper/sessions/`, writes a quick-save on exit, and restores the saved theme name when loading sessions.

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
| `Ctrl+T` | Toggle light/dark theme |
| `Ctrl+Z` | Pause/resume active task updates |
| `Ctrl+Y` | Resume when paused, otherwise copy |
| `Space` | Toggle help |
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
- `theme.rs` - ThemeManager.themes
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
