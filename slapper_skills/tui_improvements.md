---
name: tui_improvements
description: TUI (Terminal UI) patterns and recent improvements in Slapper
triggers:
  - TUI
  - terminal
  - ratatui
  - input field
  - focus area
  - auth tab
metadata:
  category: code_quality
  tools: [tui]
  scope: implementation
---

## Overview

This skill documents TUI patterns and recent improvements in Slapper's terminal UI implementation.

## Recent Updates (2026-04-29)

- AuthTab rewrite with InputGroup and FocusArea enum
- Help overlay keyboard shortcut fix ([h/l] → [n/p] for tab navigation)
- Error handling pattern added to AuthTab

## TUI Architecture

### Component Hierarchy

```
App
├── TabDispatcher (handles focus and input routing)
│   └── Tab implementations (ReconTab, AuthTab, etc.)
├── Components
│   ├── InputField - single input with validation
│   ├── InputGroup - collection of InputField with focus management
│   ├── ScrollableText - scrollable text display
│   ├── ProgressGauge - progress indicator
│   └── Checkbox - toggle option
└── Theme system (tc!() macro)
```

### FocusArea Pattern

All major tabs should implement an enum-based focus area system:

**Example** (from `tabs/recon.rs`):
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReconFocusArea {
    Inputs,
    Options,
    Results,
}

pub struct ReconTab {
    pub focus_area: ReconFocusArea,
    pub error_message: Option<String>,
    // ...
}
```

**Key methods**:
```rust
fn breadcrumb(&self) -> Option<Vec<&'static str>> {
    let focus = match self.focus_area {
        ReconFocusArea::Inputs => "Inputs",
        ReconFocusArea::Options => "Options",
        ReconFocusArea::Results => "Results",
    };
    Some(vec!["Recon", focus])
}
```

### Error Handling Pattern

Tabs should implement error handling following ReconTab's pattern:

```rust
pub struct SomeTab {
    pub state: AppState,
    pub error_message: Option<String>,
}

impl SomeTab {
    fn set_error(&mut self, msg: String) {
        self.state = AppState::Error(msg.clone());
        self.error_message = Some(msg);
    }

    fn reset(&mut self) {
        self.state = AppState::Idle;
        self.error_message = None;
    }
}
```

### InputGroup Usage

For tabs with multiple input fields, use InputGroup:

```rust
pub struct AuthTab {
    pub inputs: InputGroup,
    pub focus_area: AuthFocusArea,
}

impl AuthTab {
    fn current_input_index(&self) -> Option<usize> {
        match self.focus_area {
            AuthFocusArea::Target => Some(0),
            AuthFocusArea::Username => Some(1),
            AuthFocusArea::Password => Some(2),
            AuthFocusArea::Results => None,
        }
    }

    fn sync_input_focus(&mut self) {
        for (i, field) in self.inputs.fields.iter_mut().enumerate() {
            field.focused = Some(i) == self.current_input_index();
        }
    }
}
```

### Keyboard Navigation

**Current key bindings** (verified 2026-04-29):
- `h` / `l` - move cursor left/right within input
- `n` / `p` - next/prev tab
- `j` / `k` - navigate up/down
- `gg` / `G` - go to top/bottom
- `Ctrl+T` - toggle theme
- `Ctrl+P` - command palette
- `Space` - help overlay

**Help text in ui.rs** should match actual behavior.

### Theme System

Use the `tc!()` macro for all colors:

```rust
use crate::tui::theme::tc;

// Instead of:
widget.style(Style::default().fg(Color::Cyan))

// Use:
widget.style(tc!(primary))
```

## AuthTab Implementation

**Before** (basic, no proper input handling):
- Stored raw strings (target_url, username, password_list)
- handle_char() only appended to target_url
- No focus management
- No InputGroup

**After** (proper TUI pattern):
- Uses InputGroup with 3 fields (Target, Username, Password)
- AuthFocusArea enum for navigation
- Proper focus sync between focus_area and input fields
- Error handling with set_error() method

## Verification Commands

```bash
cargo test --lib -p slapper
cargo clippy --lib -p slapper
```

## Related Skills

- `performance_patterns` - Performance optimization patterns
- `security_fix_patterns` - Security vulnerability patterns