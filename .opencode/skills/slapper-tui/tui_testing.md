# TUI Testing Patterns

Testing patterns for the Slapper TUI module.

## Test Module Location

Place tests in `#[cfg(test)] mod tests` within the same file as the implementation:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::tabs::Tab;

    #[test]
    fn test_something() {
        // test logic
    }
}
```

## Common Test Patterns

### Testing InputField Cursor Handling

```rust
#[test]
fn test_cursor_uses_byte_index() {
    let field = InputField::new("Test").with_value("éx"); // é = 2 bytes
    assert_eq!(field.cursor_pos, "éx".len()); // 3, not chars().count() = 2
}

#[test]
fn test_backspace_deletes_character_not_byte() {
    let mut field = InputField::new("Test").with_value("éx");
    field.focused = true;
    field.backspace();
    assert_eq!(field.value, "é"); // Should delete 'x', not half of 'é'
}
```

### Testing Tab Hit-Testing

```rust
#[test]
fn test_visible_tab_spans_uneven_widths() {
    let tab_window = TabWindow {
        start: 0, end: 5, selected_visible: 0,
        max_visible: 5, total_tabs: 20,
        has_prev: false, has_next: true,
    };
    let spans = tab_window.visible_tab_spans(80);

    // Verify click detection
    let click_x = spans[3].x_start;
    let clicked_tab = spans.iter()
        .find(|s| click_x >= s.x_start && click_x < s.x_end)
        .map(|s| s.tab)
        .unwrap();
    assert_eq!(clicked_tab, Tab::ScanEndpoints);
}
```

### Testing Overlay Precedence

```rust
#[test]
fn test_topmost_overlay_confirm_precedence() {
    let mut app = create_test_app();
    app.show_help = true;
    app.show_search = true;

    app.request_confirmation(PendingAction::ResetTab);
    assert_eq!(app.topmost_overlay(), Some(OverlayType::ConfirmPopup));
}
```

### Testing Notifications

```rust
#[test]
fn test_notification_set_and_expire() {
    let mut app = create_test_app();
    app.set_notification("Test".to_string(), NotificationSeverity::Info);
    assert!(app.get_notification().is_some());

    // Simulate expiration
    app.notification.as_mut().unwrap().created_at -= std::time::Duration::from_secs(10);
    assert!(app.get_notification().is_none()); // Expired
}
```

## Running TUI Tests

```bash
# Run all TUI tests
cargo test --lib -p slapper tui::

# Run specific module tests
cargo test --lib -p slapper tui::components::input
cargo test --lib -p slapper tui::tabs::mod
cargo test --lib -p slapper tui::app::mod

# Run with output
cargo test --lib -p slapper tui:: -- --nocapture
```

## Test Coverage Goals

- Input cursor handling (byte vs char positions)
- Tab hit-testing (visible_tab_spans)
- Overlay precedence (topmost_overlay)
- Focus navigation (handle_focus_next/prev)
- Notification display and expiration
- Export success/error paths
- Feature-gated tab availability

### Testing Selector Behavior

```rust
#[test]
fn test_selector_open_close_confirm() {
    let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
    
    // Closed by default
    assert!(!selector.is_open());
    
    // Open via open()
    selector.open();
    assert!(selector.is_open());
    
    // confirm() returns item and closes
    let item = selector.confirm();
    assert!(item.is_some());
    assert_eq!(item.unwrap().value, "A"); // Default selection
    assert!(!selector.is_open());
}

#[test]
fn test_selector_up_down_only_work_when_open() {
    let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
    
    selector.selected = 2;
    
    // Up does nothing when closed
    selector.handle_up();
    assert_eq!(selector.selected, 2);
    
    // Open it
    selector.open();
    
    // Now up works
    selector.handle_up();
    assert_eq!(selector.selected, 1);
}

#[test]
fn test_selector_cancel_closes_without_change() {
    let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
    selector.expand();
    selector.select(2); // Selected "C"
    
    selector.cancel();
    
    assert!(!selector.is_open());
    assert_eq!(selector.selected, 2, "Selection should not change on cancel");
}
```

### Testing Tab Navigation via Command Palette

```rust
#[test]
fn test_command_to_tab_cluster() {
    use crate::tui::app::command::command_to_tab;
    assert_eq!(command_to_tab("cluster"), Some(Tab::Cluster));
}

#[test]
fn test_execute_command_navigates_to_cluster() {
    let mut app = create_test_app();
    app.current_tab = Tab::Fuzz;
    app.execute_command("cluster");
    assert_eq!(app.current_tab, Tab::Cluster);
}
```

### Testing ScrollableText with Empty Lines

```rust
#[test]
fn test_scroll_to_bottom_handles_empty_lines() {
    let mut scrollable = ScrollableText::new("Test");
    // Empty case should not panic
    scrollable.scroll_to_bottom();
    assert_eq!(scrollable.scroll_offset, 0);
}

#[test]
fn test_scroll_offset_with_empty_lines() {
    let mut scrollable = ScrollableText::new("Test");
    scrollable.scroll_offset = 100; // Set past bounds
    scrollable.scroll_down(1);
    // Should clamp to valid range, not overflow
    assert!(scrollable.scroll_offset <= scrollable.lines.len().saturating_sub(1));
}
```

### Testing Checkbox Array Bounds

```rust
#[test]
fn test_get_options_handles_empty_checkboxes() {
    let tab = ReconTab::new();
    // Should not panic even if option_checkboxes is empty or small
    let options = tab.get_options();
    // All values should be false (default) if array access fails
    assert!(!options.no_tech);
    assert!(!options.no_dns);
}
```
