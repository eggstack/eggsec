# AGENTS.md

Guidelines for AI agents working on this codebase.

## Project Overview

Slapper is a Rust-based security testing toolkit. See `README.md` for features and `ARCHITECTURE.md` for design details.

## Quick Reference

### Build & Test Commands

```bash
# Check compilation
cargo check --lib -p slapper

# Run library tests
cargo test --lib -p slapper

# Run specific integration test
cargo test --test negative_tests -p slapper
cargo test --test scanner_tests -p slapper

# Lint
cargo clippy --lib -p slapper

# Build release
cargo build --release -p slapper

# Test specific features
cargo check --lib -p slapper --features rest-api,ai-integration
cargo check --lib -p slapper --features python-plugins,ruby-plugins
```

### Code Organization

```
crates/slapper/
├── src/
│   ├── agent/         # Autonomous agent (event loop, portfolio, memory, alerts, skills)
│   ├── cli/           # Command-line argument parsing
│   ├── commands/      # Command handlers
│   ├── config/        # Configuration (SlapperConfig, PathsConfig, Scope)
│   ├── constants.rs   # Centralized constants (WAF, HTTP, scan, etc.)
│   ├── types.rs       # Shared types (Severity, SensitiveString)
│   ├── fuzzer/        # Fuzzing engine (30 payload types)
│   │   ├── chain.rs   # ChainExecutor (with LRU regex cache)
│   │   ├── detection/ # TimingAnalyzer (lock-free with atomics)
│   │   └── payloads/
│   │       └── macros.rs  # payload_vec! macro
│   ├── scanner/       # Port scanning, endpoint discovery
│   │   ├── templates/ # Nuclei-style template engine
│   │   └── ports/     # Port scanning (mod.rs + spoofed.rs)
│   ├── waf/           # WAF detection and bypass
│   ├── recon/         # Reconnaissance modules
│   │   ├── auth/      # Multi-protocol auth testing (ssh_auth, ftp_auth, smtp_auth)
│   │   └── dependency_scan/  # Split by ecosystem (npm, cargo, go)
│   ├── output/        # Report generation (JSON, HTML, SARIF, JUnit)
│   ├── wireless/      # Wireless security testing (WiFi scanning, auth testing)
│   ├── tool/          # Tool abstraction layer
│   │   ├── implementations/  # Tool implementations (recon, scanner, fuzzer, waf, search, etc.)
│   │   └── protocol/
│   │       ├── mcp/   # MCP server (handlers/server.rs, handlers/helpers.rs)
│   │       ├── openai/  # OpenAI-compatible chat completions
│   │       ├── rest.rs  # REST API (scope validation implemented)
│   │       └── grpc.rs  # gRPC service
│   ├── proxy/         # Proxy modules (to_log_key() for safe logging)
│   │   └── intercept/ # Intercepting proxy with dynamic SSL certs
│   ├── stress/        # Stress testing (raw_udp module integrated)
│   ├── tui/           # Terminal UI (ratatui 0.30 + crossterm 0.29)
│   │   ├── app/       # App struct split into submodules (dispatch, navigation, command, export, state_update, task_management)
│   │   ├── tabs/      # 29 tab implementations (settings/ split into main.rs, render.rs, input.rs)
│   │   └── workers/   # Background task workers
│   └── utils/         # Utilities (circuit_breaker, http, formatting, network)
├── tests/             # Integration tests
└── Cargo.toml
```

**Note:** The `slapper_skills/` directory in the project root contains skill files for use with the autonomous agent. This is distinct from the codebase itself which agents work on.

### Key Types

- `SlapperConfig` - Main configuration (use `config::load_config()`)
- `PathsConfig` - Directory paths (flattened into SlapperConfig)
- `SpoofConfig` - IP spoofing settings
- `FuzzEngine` - Main fuzzing engine (returns `Result`)
- `PayloadType` - Enum of 30 payload categories
- `Severity` - Canonical severity rating (in `types.rs`, re-exported everywhere)
- `SensitiveString` - Zeroized credential wrapper (in `types.rs`)

### Feature Flags

- `stress-testing` - Enables ICMP probing, IP spoofing, raw sockets
- `packet-inspection` - Packet capture features
- `python-plugins` / `ruby-plugins` - Plugin language support
- `rest-api` / `grpc-api` - API server integration
- `nse` - Nmap NSE script support
- `nse-sandbox` - NSE sandbox mode (restricts `io.popen`, `os.setenv`, filesystem access)
- `ai-integration` - AI/LLM features (autonomous agent, skill system, payload generation)
- `ws-api` - WebSocket support for pub/sub
- `full` - All features combined

Note: `mcp-server` feature has been removed. Use `rest-api` instead.

### PyO3 Dependency

- Current version: 0.28 (supports Python 3.14)
- In `crates/slapper-plugin/Cargo.toml`: `pyo3 = { version = "0.28", features = ["auto-initialize"], optional = true }`
- Breaking changes: `Python::with_gil` renamed to `Python::attach` in 0.26; `Bound` API introduced in 0.21 is now standard; GIL lifetime constraints tightened

## Codebase Health

### Current Metrics

| Metric | Value | Note |
|--------|-------|------|
| Tests | 1130 passing | Base library tests |
| Tests | 1388 passing | With rest-api,ai-integration |
| Clippy | ~5 warnings | TUI-specific acceptable |
| Source files | 506 |
| Payload types | 30 |
| Tabs | 29 |

### Severity Enum (Unified)

Single canonical definition in `types.rs`. All other modules re-export from it:

| Re-export path | Source |
|---------------|--------|
| `fuzzer::payloads::Severity` | `pub use crate::types::Severity` |
| `waf::types::Severity` | `pub use crate::types::Severity` |
| `config::Severity` | `pub use crate::types::Severity` |
| `recon::secrets::Severity` | `pub use crate::types::Severity` |
| `output::agent::Severity` | `pub use crate::types::Severity` |
| `output::trend::Severity` | `pub use crate::types::Severity` |

The `tool/response.rs` module uses a separate `ResponseSeverity` enum with an extra `None` variant for API compatibility.

**When adding new code:** re-export from `crate::types::Severity`. Do not create a new definition.

### Tool Abstraction Layer (Already Exists)

`tool/traits.rs:117` has `SecurityTool` trait, `tool/registry.rs:9` has `ToolRegistry`. These are feature-gated behind `tool-api` (enabled by `rest-api`, `grpc-api`, `nse`). Do not re-implement.

### SensitiveString

Credentials (API keys, passwords, PSKs, webhook secrets) use `SensitiveString` from `types.rs`:
- Zeroizes on drop
- `expose_secret()` borrows the inner string
- `into_secret()` consumes and returns the inner string
- `log_secret()` logs safely with redaction option
- `for_logging()` creates display-safe wrapper for logging
- `Debug` and `Display` show `[REDACTED]`
- Constant-time equality (via `subtle::ConstantTimeEq`)
- Serializes transparently for config file compatibility
- `len()` returns the length of the inner string
- `as_bytes()` returns raw bytes (for proxy auth encoding)
- `is_empty()` checks if empty

**Note:** Proxy credentials (`SocksProxy`, `HttpConnectProxy`) now use `SensitiveString` for secure storage.

### Circuit Breaker

`utils/circuit_breaker.rs` provides circuit breaker pattern for external API resilience:
- `CircuitBreaker` - individual breaker with state (Closed/Open/HalfOpen)
- `CircuitBreakerRegistry` - manages multiple breakers by name (each AI client creates its own breaker directly)
- Tracks failure/success counts, total calls, failure rate
- Exposes `total_calls()`, `total_failures()`, `failure_rate()` methods

### Truncation Functions

Two truncation utilities in `utils/formatting.rs`:
- `strip_controls` - removes control characters (recommended)
- `preserve_all` - preserves all characters

Both use `.chars().take()` for safe character-based truncation (no byte slicing panic risk).

### Macros

`fuzzer/payloads/macros.rs` defines `payload_vec!` for building payload vectors from inline data. Reduces repetitive `for` loops in payload modules (e.g., sqli.rs went from 8 loops to 1 macro call).

### WAF Constants

`constants::waf` module has scoring and detection constants. Use these instead of magic numbers in WAF-related code.

### TLS

`rustls` (0.23) + `tokio-rustls` (0.26) is the only TLS backend for the main `slapper` crate. `native-tls` has been removed.
- `distributed/io.rs` — `StreamWrapper` enum with `Plain`, `TlsClient`, `TlsServer` variants
- `TlsServer::from_pem(cert_path, key_path)` — loads PEM cert + key files
- `TlsClient::new(domain)` — creates client with `NoVerifier` (insecure, for internal use)
- `recon/ssl.rs` uses `rustls_pki_types::CertificateDer` for cert extraction

**Dependency Versions (as of 2026-04-24):**
- axum: 0.8.x
- tonic: 0.14.x
- prost: 0.14.x

**Exception:** `slapper-nse` retains `native-tls` (OpenSSL) for Nmap NSE script compatibility. This is intentional — Nmap scripts expect OpenSSL-based TLS behavior. Do not migrate `slapper-nse` to `rustls`.

### Spoofed Scanner

`scanner/ports/spoofed.rs` contains raw socket scanning (feature-gated). `scan_ports()` delegates to `spoofed::scan_ports_spoofed()` when spoof enabled. Packet trace uses `OnceLock<Mutex<File>>` for thread-safe file writing.

**Note:** The `raw_udp` module in `stress/udp.rs:20-117` is integrated — `run_udp_flood()` calls `run_udp_flood_spoofed()` which uses `raw_udp::build_udp_packet` when IP spoofing is enabled on Unix.

## Planning

- `plans/plan.md` — Master Consolidated Improvement Plan
  - Phase 12R (TUI Tab Model Correction) COMPLETED as of 2026-04-30
  - All Phase 12R items verified: stable IDs, TabWindow clamping, width tracking, bookmarks, session restore, mouse hit-testing, popup hardening
  - Contains architecture patterns useful for future work:
    - TabIndexing Model (Phase 12, corrected in Phase 12R)
    - Event Loop Order (Phase 8)
    - Handler Registry Pattern (Phase 8)
    - Snapshot File Pattern (Phase 10)
    - Session Persistence with Stable IDs (Phase 12R)
    - Popup Clamping (Phase 12R)

## Important Guidelines

### Codebase Verification Required

When implementing changes or reviewing plan items, verify actual state rather than assuming plan accuracy:
- Payload type count: 30 (verified via `fuzzer/payloads/mod.rs`)
- Recon module count: 31 (verified)
- Test count: 1134 base, 1388 with full features (verified 2026-04-30)
- Use `rg` to confirm file paths and line numbers exist
- Run `cargo test --lib -p slapper` after each change

### notify-debouncer-mini API

When using `notify-debouncer-mini` in config watching code:
- Version 0.5+ uses callback-based API, NOT channel-based
- Use `new_debouncer(duration, |res: DebounceEventResult| { ... })` with callback
- Access watcher via `debouncer.watcher()` method
- Debouncer struct stores the RecommendedWatcher internally

### Agent Observability

The `agent/logging.rs` module provides `AgentLogger` for file-based logging:
- Call `AgentLogger::init(log_dir)` early in agent startup
- Logs written to `log_dir/agent.log` with daily rotation
- JSON format with thread IDs, file/line info

### Agent Config Hot-Reloading

The `agent/config_watcher.rs` module provides `ConfigWatcher`:
- Use `SlapperConfigReloader` for portfolio + main config paths
- Watcher gracefully handles missing files via `.ok()`
- Requires `ConfigReloader` trait implementation for custom reload logic

### Cookie Management

Reqwest handles cookies automatically when `cookies` feature is enabled:
- Enable via `features = [..., "cookies"]` in Cargo.toml
- Client's internal cookie jar manages Set-Cookie automatically
- Manual cookie header construction is NOT needed when using reqwest Client

### Regex Caching

Use LRU cache for regex patterns to prevent unbounded memory growth:
- `lru = "0.18"` crate available for use
- Recommended cache size: 100 entries (use `NonZeroUsizer`)
- Access via `cache.put(key, value)` and `cache.get(&key)`

### Agent Alert Fatigue Prevention

The agent has built-in mechanisms to prevent alert fatigue:

**Baseline-Aware Alerting:**
- `Agent::process_scheduled_scans` uses `LongitudinalMemory::compare_with_baseline` to filter findings
- Only NEW findings (not in baseline) trigger alerts
- Resolved findings are tracked separately

**Cross-Scan Deduplication:**
- `LongitudinalMemory::deduplicate_findings` prevents repeat alerts for same finding IDs
- Alerted finding IDs stored in `alerted_findings.json` in memory directory
- Uses `load_alerted_findings()` and `save_alerted_findings()` for persistence

**Handler Registry Safety:**
- `Agent::trigger_event` uses deferred restoration pattern for handler safety
- Handlers are taken, processed, then ALWAYS restored regardless of panic/error
- This prevents handler loss during event processing failures

### TUI Event Loop Order

The main TUI event loop in `runner.rs` follows `update() -> draw() -> poll()` order:
- `update()` is called first to process background task results
- `draw()` renders the UI only if `needs_redraw` is set
- `poll()` waits for user input with 100ms timeout

**Channel Draining:**
- `App::update` drains ALL pending messages from `progress_rx` and `result_rx` using while-let loops
- Uses collected `pending_updates` / `pending_results` vectors to avoid borrow checker issues

**Dynamic visible_rows:**
- `HistoryTab::calc_visible_rows(area: Rect)` calculates visible rows based on actual Rect height
- Called during render to provide dynamic sizing based on terminal size

### Breadcrumb System

Breadcrumb display is centralized via `TAB_BREADCRUMBS` constant in `tui/tabs/mod.rs`:
- `Tab::default_breadcrumb()` returns static labels for most tabs
- Custom breadcrumb implementations exist for: Recon, Fuzz, WAF, Proxy, Packet, Hunt, Browser, Compliance, Storage, Integrations, Workflow, Vuln
- `ui.rs::draw_breadcrumb` uses `unwrap_or_else` to fall back to default

### Dashboard Enhancements

Dashboard includes trend visualization and asset health overview:
- `render_sparkline()` - ASCII sparkline renderer using Unicode block characters (▁▂▃▄▅▆▇█)
- Asset Health Summary shows: unique targets, today's scans, critical findings count, health indicator

### TUI Theming

The TUI uses a theme system via the `tc!` macro (defined in `tui/theme.rs`):
- Add `use crate::tc;` to imports
- Usage: `tc!(field_name)` where field_name is one of: primary, secondary, accent, background, foreground, surface, border, border_focused, text, text_dim, text_bright, success, warning, error, info, selected, selected_text, highlight, mode_normal, mode_insert, tab_active, tab_inactive, status_running, status_idle, status_error

**Semantic mapping for Color replacements:**
- Text: `Color::White` → `tc!(text)`, `Color::Gray/DarkGray` → `tc!(text_dim)`
- Borders: `Color::Yellow` focused → `tc!(border_focused)`, `Color::Gray/DarkGray` → `tc!(border)`
- Status: `Color::Green` → `tc!(success)`, `Color::Red` → `tc!(error)`, `Color::Yellow` → `tc!(warning)` or `tc!(accent)`, `Color::Cyan` → `tc!(info)`
- HTTP status: 200-299 → `tc!(success)`, 300-399 → `tc!(info)`, 400-499 → `tc!(warning)`, 500-599 → `tc!(error)`

### FocusArea Enum Pattern

Tabs should use a `FocusArea` enum for navigation between logical areas:
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TabFocusArea {
    Inputs,
    Options,
    Results,
}

pub struct Tab {
    pub focus_area: TabFocusArea,
    pub error_message: Option<String>,
    // ... other fields
}

impl Tab {
    pub fn new() -> Self {
        Self {
            focus_area: TabFocusArea::Inputs,
            error_message: None,
            // ...
        }
    }
}

impl TabInput for Tab {
    fn handle_up(&mut self) {
        self.focus_area = match self.focus_area {
            TabFocusArea::Options => TabFocusArea::Inputs,
            TabFocusArea::Results => TabFocusArea::Options,
            TabFocusArea::Inputs => TabFocusArea::Results,
        };
    }

    fn handle_down(&mut self) {
        self.focus_area = match self.focus_area {
            TabFocusArea::Inputs => TabFocusArea::Options,
            TabFocusArea::Options => TabFocusArea::Results,
            TabFocusArea::Results => TabFocusArea::Inputs,
        };
    }

    fn set_error(&mut self, msg: String) {
        self.state = AppState::Error(msg.clone());
        self.error_message = Some(msg);
    }
}
```

### TabIndexing Model

The TUI uses a unified tab indexing system to handle feature-gated tabs correctly:

**Key Types** (in `tui/tabs/mod.rs`):
- `Tab::all()` - Returns `&'static [Tab]` with only available tabs for current feature set
- `Tab::visible_index(&self) -> Option<usize>` - Returns position in `Tab::all()`
- `Tab::from_visible_index(index: usize) -> Option<Tab>` - Returns tab by position
- `Tab::stable_id(&self) -> &'static str` - Returns string ID for persistence (`"recon"`, `"dashboard"`, etc.)
- `Tab::from_stable_id(id: &str) -> Option<Tab>` - Returns tab from string ID (None if tab unavailable in feature set)

**TabWindow Helper** (in `tui/tabs/mod.rs`):
```rust
pub struct TabWindow {
    pub start: usize,           // Start index in Tab::all()
    pub end: usize,             // End index in Tab::all()
    pub selected_visible: usize, // Selected index within visible window
    pub max_visible: usize,     // Max tabs that fit in current width
    pub total_tabs: usize,      // Total tabs in Tab::all()
    pub has_prev: bool,         // True if there are hidden tabs before
    pub has_next: bool,         // True if there are hidden tabs after
}

impl TabWindow {
    pub fn for_width(term_width: u16, current_tab: Tab, previous_offset: u16) -> Self;
    pub fn range_text(&self) -> String;  // Returns "[1-7/20]" style text
}
```

**Usage**:
- UI rendering: `TabWindow::for_width(area.width, app.current_tab, app.tab_scroll_offset)`
- Navigation: Uses `TabWindow` instead of hardcoded `visible_count = 10`
- Mouse hit-testing: Uses `TabWindow` to map click position to correct tab
- Session persistence: Uses `stable_id` for forward compatibility

- `App::new(history: SharedHistory)` - Runtime constructor; restores session state
- `App::new_for_testing(history: SharedHistory)` - Test constructor; does NOT restore session
- Use `App::new_for_testing()` in all unit tests to avoid ambient session file dependencies

**Anti-patterns to avoid**:
- Don't use `tab as usize` for tab indexing (enum discriminants != visible indexes)
- Don't use `Tab::all().len()` as visible count (not all tabs may be available)
- Don't divide tab area by total tab count for mouse hit-testing

---

### Auto-Insert Mode

The TUI automatically switches to Insert mode when Tab/Shift+Tab focuses an input:
- `handle_focus_next()` and `handle_focus_prev()` in `App` check `is_input_focused()` after navigation
- If input is focused, mode is set to `InputMode::Insert`; otherwise `InputMode::Normal`
- Users can still manually toggle with `i` key in Normal mode

---

*End of AGENTS.md*