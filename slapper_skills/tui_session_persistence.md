---
name: tui_session_persistence
description: "Session auto-persistence for TUI state recovery using stable IDs"
triggers:
  - session
  - persistence
  - save state
  - restore
  - bookmark
metadata:
  category: TUI
  tools: [tui]
  scope: local
---

## Overview

Slapper's TUI includes session persistence that automatically saves and restores UI state using stable string IDs for tabs and bookmarks. This enables crash recovery and preserves user preferences across sessions.

**Key Change (Phase 12R):** Session state now uses stable IDs (`HashSet<String>`) for bookmarks and `Option<String>` for current tab, not numeric indexes. This ensures bookmarks and tab state persist correctly across feature-gated builds.

## Usage

```rust
use slapper::tui::session::{SessionManager, SessionState};
use slapper::tui::tabs::Tab;

// Create session manager
let manager = SessionManager::new(SessionConfig::default());

// Session state uses stable IDs internally
let state = manager.capture_state(&app);
// state.current_tab_id = Some("dashboard".to_string())
// state.bookmarks = ["settings", "history"].iter().map(|s| s.to_string()).collect()

// Load saved session
if let Some(saved) = manager.load_quick() {
    manager.restore_session(&mut app, &saved);
}

// Save current session
let path = manager.save_quick(&app)?;
```

## Session State Contents

The `SessionState` captures:
- Current tab stable ID (`current_tab_id: Option<String>`)
- Bookmarked tab stable IDs (`bookmarks: Vec<String>`)
- Active theme name
- Legacy numeric fields for backward compatibility

## Implementation

- `crates/slapper/src/tui/session.rs` - SessionManager and SessionState
- `crates/slapper/src/tui/app/mod.rs` - `App::bookmarks` is now `HashSet<String>`

## Key Methods

- `SessionManager::new(config)` - Creates new session manager with config
- `manager.save_session(&app)` - Save state to timestamped file
- `manager.save_quick(&app)` - Save to quick-save slot (crash recovery)
- `manager.load_quick()` - Load from quick-save slot
- `manager.restore_session(&mut app, &state)` - Restore session state
- `app.get_bookmarked_tab_ids()` - Returns `Vec<String>` of stable IDs
- `app.toggle_bookmark(tab: Tab)` - Toggle bookmark using Tab enum
- `app.is_bookmarked(tab: Tab)` - Check if tab is bookmarked

## Bookmarks API

Bookmarks now use stable IDs instead of numeric indexes:

```rust
// Toggle bookmark for a specific tab
app.toggle_bookmark(Tab::Dashboard);

// Check if a tab is bookmarked
if app.is_bookmarked(Tab::Settings) {
    // ...
}

// Get all bookmarked tab IDs
let bookmarks = app.get_bookmarked_tab_ids();
// Returns: ["dashboard", "settings"]
```

## Backward Compatibility

Legacy numeric fields are maintained for reading old session files:
- `legacy_current_tab: Option<usize>` - Previous enum discriminant
- `legacy_bookmarks: Vec<usize>` - Previous numeric bookmarks

On restore, both stable ID and legacy fields are checked, with stable IDs preferred.

## File Location

Session state is stored in the user's data directory:
- Linux: `~/.local/share/slapper/sessions/`

## Quick Save Feature

The quick-save slot (`save_quick`/`load_quick`) automatically persists state periodically and on exit. Use this for crash recovery.

## Verification

```bash
cargo test --lib -p slapper -- session
# or
cargo test --lib -p slapper -- test_bookmark_api_uses_stable_ids
```
