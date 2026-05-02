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
