---
name: checkbox-focus-pattern
description: Pattern for managing checkbox focus in TUI tabs with multiple checkboxes
triggers:
  - checkbox
  - focus area
  - multiple checkboxes
  - technique selection
metadata:
  category: tui_patterns
  tools: [tui]
  scope: implementation
---

## Problem

When a tab has multiple checkboxes (like WAF techniques), you need to track which one is focused for navigation. Using `checkbox.focused` directly in a loop with `i == 0` only allows the first checkbox to show focus.

## Solution: Dedicated Index Field

```rust
pub struct WafTab {
    pub technique_checkboxes: Vec<Checkbox>,
    pub focused_checkbox_index: usize,
    // ...
}

impl WafTab {
    pub fn new() -> Self {
        // ...
        focused_checkbox_index: 0,
        // ...
    }
}
```

## Navigation Methods

```rust
impl TabInput for WafTab {
    fn handle_focus_next(&mut self) {
        self.focused_checkbox_index = 0;
        // transition to next focus area
    }

    fn handle_left(&mut self) -> bool {
        if self.focused_checkbox_index == 0 {
            false  // at left edge, don't move
        } else {
            self.focused_checkbox_index -= 1;
            true
        }
    }

    fn handle_right(&mut self) -> bool {
        if self.focused_checkbox_index >= self.technique_checkboxes.len() - 1 {
            false  // at right edge
        } else {
            self.focused_checkbox_index += 1;
            true
        }
    }

    fn is_at_left_edge(&self) -> bool {
        self.technique_checkboxes.is_empty() || self.focused_checkbox_index == 0
    }

    fn is_at_right_edge(&self) -> bool {
        self.technique_checkboxes.is_empty()
            || self.focused_checkbox_index >= self.technique_checkboxes.len().saturating_sub(1)
    }
}
```

## Render Method

```rust
for (i, cb) in self.technique_checkboxes.iter().enumerate() {
    let mut checkbox = cb.clone();
    checkbox.focused = self.focus_area == WafFocusArea::Techniques 
        && i == self.focused_checkbox_index;
    checkbox.render(f, config_chunks[2 + i]);
}
```

## Key Points

1. **Don't modify Checkbox.focused in navigation** - Let render() set it based on index
2. **Use saturating_sub for left edge** - Prevents underflow
3. **Reset index on focus change** - When leaving Techniques area, reset to 0
4. **Edge checks use index** - `is_at_left_edge` returns `index == 0`
5. **Always validate bounds in handle_enter** - Direct array access can panic:
   ```rust
   // WRONG - could panic
   self.option_checkboxes[self.focused_checkbox_index].toggle();
   
   // CORRECT - bounds check prevents panic
   if self.focused_checkbox_index < self.option_checkboxes.len() {
       self.option_checkboxes[self.focused_checkbox_index].toggle();
   }
   ```

## Anti-Pattern

This is WRONG - only first checkbox gets focus:

```rust
// WRONG
checkbox.focused = self.focus_area == WafFocusArea::Techniques && i == 0;

// Also wrong - modifying focused directly in navigation
fn handle_left(&mut self) {
    for cb in &mut self.technique_checkboxes {
        if cb.focused {
            cb.focused = false;
            // find prev and set focused...
        }
    }
}
```