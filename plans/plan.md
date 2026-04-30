# Slapper Improvement Plan - COMPLETED

**Date**: 2026-04-30
**Status**: ALL PHASES COMPLETE
**Priority**: Historical Record

---

## Executive Summary

All phases (0-12) of the Slapper improvement plan have been completed and verified. The plan delivered:

- **1,130** passing tests (base library)
- **1,388** passing tests (full features with rest-api,ai-integration)
- **~5** clippy warnings (TUI-specific acceptable)
- **506** source files, **30** payload types, **29** TUI tabs

### Key Accomplishments by Phase

| Phase | Description | Key Deliverables |
|-------|-------------|------------------|
| 0 | Stabilization | Fixed 7 AI test failures |
| 1 | Critical & Security | grpc-api + stress-testing + packet-inspection compilation |
| 2 | TUI UX & Features | Global search, clipboard, pause/resume |
| 3 | Core Quality & Refactor | TCP_NODELAY, client pooling, async I/O, CookieStore |
| 4 | Performance & Hardening | FxHashMap, LRU regex cache (100 entries) |
| 5 | Feature Enhancements | AgentLogger, ConfigWatcher, StatefulFuzzer |
| 6 | Long-term Capabilities | Exploit framework, cloud scanning |
| 7 | Documentation | CI/CD templates |
| 8 | Pre-Open Source Polish | Alert fatigue fix, TUI perf, theme consistency |
| 9 | Dashboard & Alert Polish | Sparkline data, warm_cache, drop guard handlers |
| 10 | Portfolio Memory | Snapshot file pattern, portfolio health |
| 11 | TUI Modernization | FocusArea (29 tabs), error reporting, auto-insert |
| 12 | TUI Navigation Hardening | TabWindow helper, stable IDs, mouse hit-testing |

---

## Architecture Patterns (For Future Agents)

### Tab Indexing Model (Phase 12)

The TUI uses a unified tab indexing system:

```rust
Tab::all()                    // Returns &[Tab] with feature-gated tabs
Tab::visible_index()          // Position in Tab::all()
Tab::from_visible_index(idx)  // Get tab by position
Tab::stable_id()             // String ID ("recon", "scan_ports", etc.)
Tab::from_stable_id(id)      // Get tab from string ID (None if unavailable)
TabWindow::for_width(w, tab, offset)  // Compute visible window
```

**Anti-patterns to avoid:**
- Don't use `tab as usize` for tab indexing (enum discriminants != visible indexes)
- Don't use `Tab::all().len()` as visible count
- Don't divide tab area by total tab count for mouse hit-testing

### Event Loop Order (Phase 8)

```rust
loop {
    app.update();                              // 1. Process background tasks
    if app.needs_redraw {
        terminal.draw(|f| ui::draw(f, app))?; // 2. Render if needed
    }
    if event::poll(Duration::from_millis(100))? {  // 3. Poll input
        // handle input
    }
}
```

### Handler Registry Pattern (Phase 8)

```rust
pub async fn trigger_event(&mut self, event: SecurityEvent) -> Result<()> {
    let handlers = std::mem::take(&mut self.event_handlers);
    let result = (|| async {
        for handler in handlers.iter() {
            if handler.handles(&event) {
                handler.handle(&event, self).await?;
            }
        }
        Ok(())
    })().await;
    self.event_handlers = handlers;  // ALWAYS restore
    result
}
```

### Snapshot File Pattern (Phase 10)

```
Agent scan complete → LongitudinalMemory → write portfolio_snapshot.json
                                                       ↓
                               TUI Dashboard ← read on demand
```

### Session Persistence with Stable IDs (Phase 12)

```rust
pub struct SessionState {
    pub current_tab_id: Option<String>,    // Stable IDs (recon, dashboard, etc.)
    #[serde(default)]
    pub bookmarks: Vec<String>,             // Also stable IDs
    pub theme_name: String,
    #[serde(default)]
    pub legacy_current_tab: Option<usize>,  // Backward compatibility
    #[serde(default)]
    pub legacy_bookmarks: Vec<usize>,
}
```

### Popup Clamping (Phase 12)

```rust
pub fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let clamped_width = width.min(r.width.saturating_sub(2));
    let clamped_height = height.min(r.height.saturating_sub(2));
    // ... center in available space
}
```

---

## Important Guidelines

### SensitiveString

Credentials use `SensitiveString` from `types.rs`:
- Zeroizes on drop
- Constant-time equality via `subtle::ConstantTimeEq`
- `log_secret()` for safe logging with redaction
- Serializes transparently for config compatibility

### Circuit Breaker

`utils/circuit_breaker.rs` provides circuit breaker pattern:
- `CircuitBreaker` - individual breaker with Closed/Open/HalfOpen states
- `CircuitBreakerRegistry` - manages multiple breakers by name

### Theme System

Use `tc!` macro for theme colors:
```rust
use crate::tc;
tc!(primary), tc!(background), tc!(text), tc!(error), tc!(success), etc.
```

### Auto-Insert Mode

TUI automatically switches to Insert mode when Tab/Shift+Tab focuses an input.

---

## Verification Commands

```bash
# Run all library tests
cargo test --lib -p slapper

# Run specific phase tests
cargo test --lib -p slapper -- test_tab_visible_index test_tab_stable_id_roundtrip

# Lint
cargo clippy --lib -p slapper

# Check with features
cargo check --lib -p slapper --features rest-api,ai-integration
```

---

## Files of Interest

| Path | Purpose |
|------|---------|
| `tui/tabs/mod.rs` | Tab enum, TabWindow, stable_id, from_stable_id |
| `tui/app/navigation.rs` | Tab scrolling, next/prev/select |
| `tui/app/runner.rs` | Event loop, mouse hit-testing |
| `tui/session.rs` | Session state with stable IDs + legacy fallback |
| `agent/mod.rs` | trigger_event with proper handler restoration |
| `agent/memory.rs` | LongitudinalMemory, PortfolioSnapshot, warm_cache |
| `tui/tabs/dashboard.rs` | Portfolio health display |
| `tui/theme.rs` | tc! macro definition |
| `fuzzer/chain.rs` | LRU regex cache (100 entries) |