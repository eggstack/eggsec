---
name: tui_theme_system
description: "TUI theme system with dark/light presets using ratatui"
triggers:
  - theme
  - dark mode
  - light mode
  - colors
  - ui colors
metadata:
  category: TUI
  tools: [tui]
  scope: local
---

## Overview

Slapper's TUI includes a theme system that provides consistent color definitions across all UI components. Themes support both dark and light presets with full customization capability.

## Usage

### Available Theme Colors

The `ThemeColors` struct provides colors for all UI elements:
- `black`, `white`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`
- `bg_primary`, `bg_secondary`, `bg_tertiary` - background colors
- `fg_primary`, `fg_secondary`, `fg_tertiary` - foreground colors
- `border`, `highlight`, `selection` - UI element colors
- `severity_critical`, `severity_high`, `severity_medium`, `severity_low` - severity colors

### Using Themes in TUI Code

```rust
use slapper::tui::theme::{Theme, ThemeManager, ThemeColors, dark_theme, light_theme};

// Get current theme
let theme = Theme::current();

// Use preset themes
let dark = dark_theme();
let light = light_theme();

// ThemeManager for managing active theme
let mut manager = ThemeManager::new(dark_theme());
manager.set_theme(light_theme());
manager.toggle_theme();
```

### Macros for Easy Access

```rust
use slapper::tui::theme::{theme, tc};

// Get current theme
let current = theme!();

// Get specific color
let primary = tc!(fg_primary);
let critical = tc!(severity_critical);
```

### Implementing Custom Themes

```rust
let custom = Theme {
    name: "custom".to_string(),
    colors: ThemeColors {
        bg_primary: Color::Rgb(30, 30, 30),
        fg_primary: Color::Rgb(220, 220, 220),
        // ... other colors
    },
};
```

## Implementation

- `crates/slapper/src/tui/theme.rs` - Theme, ThemeColors, ThemeManager
- Uses `std::sync::LazyLock` for thread-safe global theme access

## Key Types

- `Theme` - Contains name and ThemeColors
- `ThemeColors` - All color definitions for UI
- `ThemeManager` - Manages active theme and switching

## Verification

```bash
cargo test --lib -p slapper -- tui::theme
```