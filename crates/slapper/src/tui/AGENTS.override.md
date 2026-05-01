# TUI Module Override

Specialized guidance for the terminal UI module.

## Event Loop Order

`runner.rs` follows `update() -> draw() -> poll()` order:
- `update()` processes background task results first
- `draw()` renders only if `needs_redraw` is set
- `poll()` waits for user input with 100ms timeout

## Channel Draining

`App::update` drains ALL pending messages from `progress_rx` and `result_rx`:
- Uses collected `pending_updates` / `pending_results` vectors
- Avoids borrow checker issues

## Tab System

### TabIndexing Model (`tui/tabs/mod.rs`)

- `Tab::all()` - Returns available tabs for current feature set
- `Tab::visible_index(&self)` - Position in `Tab::all()`
- `Tab::from_visible_index(index: usize)` - Tab by position
- `Tab::stable_id(&self)` - String ID for persistence
- `Tab::from_stable_id(id: &str)` - Tab from string ID
- `Tab::from_discriminant(discriminant: usize)` - Enum discriminant mapping

### TabWindow Helper

```rust
pub struct TabWindow {
    pub start: usize,
    pub end: usize,
    pub selected_visible: usize,
    pub max_visible: usize,
    pub total_tabs: usize,
    pub has_prev: bool,
    pub has_next: bool,
}
```

### Anti-patterns

- Don't use `tab as usize` (enum discriminants != visible indexes)
- Don't use `Tab::all().len()` as visible count
- Don't divide tab area by total tab count for mouse hit-testing

## Theming

`tc!` macro in `tui/theme.rs`:

```rust
tc!(field_name)  // primary, secondary, accent, background, text, etc.
```

**Semantic mapping:**
| Old | Theme |
|-----|-------|
| `Color::White` | `tc!(text)` |
| `Color::Gray` | `tc!(text_dim)` |
| `Color::Green` | `tc!(success)` |
| `Color::Red` | `tc!(error)` |

**HTTP status:** 200-299 → `tc!(success)`, 400-499 → `tc!(warning)`, 500-599 → `tc!(error)`

## FocusArea Enum Pattern

Tabs use `FocusArea` enum for navigation between Inputs/Options/Results areas.

## Auto-Insert Mode

Automatically switches to Insert mode when Tab/Shift+Tab focuses an input.