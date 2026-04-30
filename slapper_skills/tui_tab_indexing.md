---
name: tui_tab_indexing
description: "TUI tab indexing model with stable IDs and TabWindow"
triggers:
  - tab
  - navigation
  - tabwindow
  - visible_index
  - stable_id
  - mouse hit-testing
metadata:
  category: TUI
  tools: [tui]
  scope: local
---

## Overview

Slapper's TUI uses a unified tab indexing system to handle tab navigation, rendering, mouse selection, bookmarks, and session persistence correctly across base and feature-gated builds.

**Key Concepts:**
- `Tab` enum variant - In-memory active tab identity
- `Tab::all()` position - Visible/runtime tab index in current feature set
- `Tab::stable_id()` - Persistent identity string for sessions/bookmarks
- `tab as usize` - Enum discriminant only (AVOID for navigation)

## Core Types

```rust
// In crates/slapper/src/tui/tabs/mod.rs

// Tab enum with stable_id() and from_stable_id()
pub enum Tab {
    Recon = 0,
    Load = 1,
    // ... up to Vuln = 28
}

// Get all tabs available in current feature set
Tab::all() -> &'static [Tab]

// Get position in Tab::all() (not enum discriminant!)
tab.visible_index() -> Option<usize>

// Get stable string ID for persistence
tab.stable_id() -> &'static str  // e.g., "dashboard", "settings"

// Restore tab from stable ID (checks availability!)
Tab::from_stable_id("dashboard") -> Option<Tab>
```

## TabWindow Helper

`TabWindow` computes the visible window of tabs for the current terminal width:

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
    // Create window for given terminal width
    pub fn for_width(term_width: u16, current_tab: Tab, previous_offset: u16) -> Self;
}
```

## TabWindow Usage

```rust
use crate::tui::tabs::{Tab, TabWindow};

// Create window based on terminal width
let window = TabWindow::for_width(80, app.current_tab, app.tab_scroll_offset);

// Render visible tabs
for (i, tab) in Tab::all()[window.start..window.end].iter().enumerate() {
    let is_selected = i == window.selected_visible;
    // render tab with appropriate style
}

// Mouse click handling
let local_index = (click_x.saturating_sub(tab_area.x)) / tab_width;
let clicked_tab_index = window.start + local_index;
if let Some(tab) = Tab::from_visible_index(clicked_tab_index) {
    app.current_tab = tab;
}
```

## Anti-Patterns to Avoid

```rust
// WRONG: Using enum discriminant as visible index
app.current_tab = 5 as Tab;  // This is NOT the 5th visible tab!

// WRONG: Using Tab::all().len() as visible count
let count = Tab::all().len();  // Not all tabs may be available!

// WRONG: Dividing tab area by total tab count
let tab_width = area.width / Tab::all().len() as u16;  // Unequal!
```

## Correct Patterns

```rust
// CORRECT: Use visible_index() and from_visible_index()
let idx = app.current_tab.visible_index().unwrap_or(0);
if let Some(tab) = Tab::from_visible_index(idx) {
    app.current_tab = tab;
}

// CORRECT: Use TabWindow for rendering and mouse handling
let window = TabWindow::for_width(area.width, app.current_tab, app.tab_scroll_offset);

// CORRECT: Use stable_id for persistence
let id = app.current_tab.stable_id();  // "dashboard"
// Store this string, not the index!
```

## Terminal Width Tracking

`App` tracks terminal width to ensure keyboard navigation uses the same width as rendering:

```rust
pub struct App {
    pub last_terminal_width: u16,  // Updated in ui::draw()
    pub tab_scroll_offset: u16,
    // ...
}

// Navigation uses stored width
fn adjust_tab_scroll(&mut self) {
    let window = TabWindow::for_width(
        self.last_terminal_width,
        self.current_tab,
        self.tab_scroll_offset
    );
}
```

## Bookmarks with Stable IDs

Bookmarks are stored as `HashSet<String>` of stable IDs:

```rust
pub struct App {
    pub bookmarks: std::collections::HashSet<String>,
}

// Toggle bookmark using Tab
app.toggle_bookmark(Tab::Dashboard);

// Check if bookmarked
app.is_bookmarked(Tab::Settings);

// Get bookmark IDs for persistence
let ids = app.get_bookmarked_tab_ids();  // Vec<String>
```

## Session Persistence

Session state uses stable IDs for forward compatibility:

```rust
pub struct SessionState {
    pub current_tab_id: Option<String>,  // Stable ID, not index!
    pub bookmarks: Vec<String>,            // Stable IDs!
    // Legacy numeric fields for backward compatibility
    pub legacy_current_tab: Option<usize>,
    pub legacy_bookmarks: Vec<usize>,
}
```

## Feature-Gated Tabs

Tabs like NSE, Plugin, Hunt, etc. are only available when their feature flag is enabled. `Tab::from_stable_id()` returns `None` if the tab isn't available in the current build:

```rust
// This returns None in a build without the nse feature
let tab = Tab::from_stable_id("nse");  // None if no nse feature

// So session restore falls back gracefully
let tab = Tab::from_stable_id(id).unwrap_or(Tab::Recon);
```

## Testing with TUI

Use `App::new_for_testing()` in unit tests to avoid ambient session file dependencies:

```rust
#[test]
fn test_tab_navigation() {
    let app = App::new_for_testing(create_shared_history());
    assert_eq!(app.current_tab, Tab::Recon);  // Always starts on Recon in tests
}
```

## Implementation Files

- `crates/slapper/src/tui/tabs/mod.rs` - Tab enum, TabWindow, stable IDs
- `crates/slapper/src/tui/app/navigation.rs` - Keyboard navigation, adjust_tab_scroll
- `crates/slapper/src/tui/app/runner.rs` - Mouse hit-testing
- `crates/slapper/src/tui/app/mod.rs` - App struct with bookmarks
- `crates/slapper/src/tui/session.rs` - Session capture/restore

## Verification

```bash
cargo test --lib -p slapper -- test_tab_window
cargo test --lib -p slapper -- test_bookmark_api_uses_stable_ids
```
