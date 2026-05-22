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
| Fuzz | `fuzz.rs` | Security fuzzing with 30 payload types |
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

`SessionManager` auto-saves every 30 seconds to JSON in `~/.slapper/sessions/`.

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
| `Space` | Toggle help |
| `hjkl` / Arrows | Navigation |
| `i` | Enter insert mode |
| `Esc` | Return to normal mode / close overlay |
| `q` | Quit (when no active task) |
| `g/G` | Go to top/bottom |
| `n/N` | Next/prev tab |
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