---
name: tui_session_persistence
description: "Session auto-persistence for TUI state recovery"
triggers:
  - session
  - persistence
  - save state
  - restore
metadata:
  category: TUI
  tools: [tui]
  scope: local
---

## Overview

Slapper's TUI includes session persistence that automatically saves and restores UI state. This enables crash recovery and preserves user preferences across sessions.

## Usage

```rust
use slapper::tui::session::{SessionManager, SessionState};

// Create session manager
let manager = SessionManager::new();

// Save current session state
let state = SessionState {
    current_tab: 5,
    bookmarks: vec![1, 3, 7],
    theme: "dark".to_string(),
    paused: false,
    // ... other state
};
manager.save_session(&state)?;

// Load saved session
if let Some(saved) = manager.load_quick() {
    // Restore tab, bookmarks, etc.
}

// Restore session explicitly
manager.restore_session(&saved_state)?;
```

## Session State Contents

The `SessionState` captures:
- Current tab index
- Bookmarked tabs
- Active theme
- Pause state
- Quick save slot for crash recovery

## Implementation

- `crates/slapper/src/tui/session.rs` - SessionManager and SessionState

## Key Methods

- `SessionManager::new()` - Creates new session manager
- `manager.save_session(state)` - Save state to default location
- `manager.load_quick()` - Load from quick-save slot (crash recovery)
- `manager.restore_session(state)` - Restore explicit state

## Quick Save Feature

The quick-save slot (`load_quick`) automatically persists state periodically and on exit. Use this for crash recovery:

```rust
// On startup
if let Some(state) = manager.load_quick() {
    restore_session(&state);
}
```

## File Location

Session state is stored in the user's config directory:
- Linux: `~/.config/slapper/session.json`

## Verification

```bash
cargo test --lib -p slapper -- session
```