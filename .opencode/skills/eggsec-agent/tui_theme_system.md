---
name: tui_theme_system
description: "TUI theme system with dark/light presets using ratatui - updated for Phase 11"
triggers:
  - theme
  - dark mode
  - light mode
  - colors
  - tc! macro
  - tc!()
metadata:
  category: TUI
  tools: [tui]
  scope: local
---

## Overview

Eggsec's TUI includes a theme system that provides consistent color definitions across all UI components. **As of Phase 11**, all hardcoded `Color::*` usages have been migrated to the `tc!()` macro, ensuring theme-consistent UI throughout all 29 tabs and 5 components.

## Usage

### Available Theme Colors

The `ThemeColors` struct (in `tui/theme.rs`) provides these theme fields:
- **Core**: `primary`, `secondary`, `accent`, `background`, `foreground`
- **Surface**: `surface`, `border`, `border_focused`
- **Text**: `text`, `text_dim`, `text_bright`
- **Status**: `success`, `warning`, `error`, `info`
- **Selection**: `selected`, `selected_text`, `highlight`
- **Mode**: `mode_normal`, `mode_insert`, `tab_active`, `tab_inactive`
- **Status Bar**: `status_running`, `status_idle`, `status_error`

### Using the tc! Macro

**Important**: All TUI code should use the `tc!` macro instead of hardcoded `Color::*`.

```rust
use crate::tc;

// Get a theme color
let border_color = tc!(border);
let focused_border = tc!(border_focused);
let error_color = tc!(error);
let success_text = tc!(success);
```

### Semantic Color Mapping

When migrating from hardcoded `Color::*`, use this mapping:

| Hardcoded | Theme Field | Usage |
|-----------|-------------|-------|
| `Color::Yellow` (focused) | `tc!(border_focused)` | Focused element borders |
| `Color::Gray/DarkGray` | `tc!(border)` or `tc!(text_dim)` | Unfocused borders, placeholder text |
| `Color::Green` | `tc!(success)` | Success states, 2xx HTTP |
| `Color::Red` | `tc!(error)` | Error states, 5xx HTTP |
| `Color::Yellow` (warning) | `tc!(warning)` or `tc!(accent)` | Warning states, 4xx HTTP |
| `Color::Cyan` | `tc!(info)` | Info states, 3xx HTTP |
| `Color::Blue` | `tc!(secondary)` | Secondary actions |
| `Color::White` | `tc!(text)` | Primary text |
| `Color::Black` (on selected) | `tc!(selected_text)` | Text on selected bg |

### Using Themes in TUI Code

```rust
use ratatui::style::{Style, Modifier};
use crate::tc;

// Create a styled element
let border_style = if focused {
    Style::default().fg(tc!(border_focused))
} else {
    Style::default().fg(tc!(border))
};

// Combine with other style properties
let text_style = Style::default()
    .fg(tc!(text))
    .add_modifier(Modifier::BOLD);
```

## FocusArea Pattern

Tabs should use a `FocusArea` enum for navigation between logical areas. See `tui_improvements.md` skill for details.

## Auto-Insert Mode

As of Phase 11, the TUI automatically switches to Insert mode when Tab/Shift+Tab focuses an input field. See `tui_improvements.md` skill for details.

## Implementation

- `crates/eggsec/src/tui/theme.rs` - Theme, ThemeColors, ThemeManager, `tc!` macro
- Uses `std::cell::RefCell` with `thread_local!` for thread-safe theme access
- `#[macro_export]` makes `tc!` available crate-wide

## Key Types

- `Theme` - Contains name and ThemeColors
- `ThemeColors` - All color definitions for UI
- `ThemeManager` - Manages active theme and switching
- `ThemeMode` - Enum: Dark, Light

## Verification

```bash
# Verify theme migration (should find 0 Color::* usages)
grep -r "Color::" crates/eggsec/src/tui/tabs/*.rs crates/eggsec/src/tui/components/*.rs

# Run TUI tests
cargo test --lib -p eggsec -- tui
```