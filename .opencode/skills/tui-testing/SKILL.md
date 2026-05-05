# TUI Testing Skill

## Description

Guidance for testing terminal UI changes in the Slapper project.

## Test Commands

```bash
# Run all TUI tests
cargo test --lib -p slapper tui::

# Run specific tab tests
cargo test --lib -p slapper tui::tabs::recon
cargo test --lib -p slapper tui::tabs::fuzz

# Run app tests
cargo test --lib -p slapper tui::app::

# Run render tests at various terminal sizes
cargo test --lib -p slapper tui::app::navigation::render_tests
```

## Key Test Patterns

### 1. Non-ASCII Cursor Edge Behavior

```rust
#[test]
fn test_is_at_right_edge_non_ascii() {
    let mut tab = SomeTab::default();
    tab.focus_area = SomeFocusArea::Inputs;
    tab.inputs.fields[0].value = "café".to_string();  // 5 bytes, 4 chars
    tab.inputs.fields[0].cursor_pos = tab.inputs.fields[0].value.len();  // 5 (byte len)
    assert!(tab.is_at_right_edge());
}
```

### 2. Help Text Generation

```rust
#[test]
fn test_help_text_normal_mode() {
    let app = create_test_app();
    let help = get_help_text(&app, Rect::new(0, 0, 80, 24));
    assert!(help.contains("[h/j/k/l]"));
}
```

### 3. Overlay Key Handling

```rust
#[test]
fn test_overlay_close_on_esc() {
    let mut app = create_test_app();
    app.show_search = true;
    // Simulate Esc key
    // assert!(...);
}
```

## Common Bugs to Watch For

1. **Byte vs Char bug**: `cursor_pos` is byte index, don't compare with `chars().count()`
   - Wrong: `cursor_pos >= value.chars().count()`
   - Correct: `cursor_pos >= value.len()`

2. **Static cache**: Avoid `static LazyLock<Vec<T>>` for tab titles
   - Build from `Tab::all()` each render instead

3. **Overlay precedence**: Always use `topmost_overlay()` in event handling
   - Confirm popup > Command palette > Search > HTTP options > Help

## Visual Regression Testing

Add visual regression tests using `TestBackend`:
```rust
#[test]
fn test_render_tab_has_content() {
    let mut app = create_test_app();
    app.current_tab = Tab::SomeTab;
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
    
    let buf = terminal.backend().buffer();
    // Check that buffer has content (not all spaces)
    let has_content = buf.content.iter().any(|cell| {
        cell.symbol().len() > 0 && cell.symbol() != " "
    });
    assert!(has_content, "Tab should render some content");
}
```

### Guidelines for Visual Regression Tests
- Use `TestBackend::new(width, height)` for different terminal sizes
- Check that buffer has content, not exact text matches
- Text content may vary based on theme and terminal size
- Test that tabs render without panicking at various sizes (80x24, 120x24, 40x20, etc.)
- Avoid checking for specific text like "Slapper", "NORMAL", etc. as content varies

## Render Tests

Always add render tests for new tabs or major layout changes:

```rust
#[test]
fn test_render_at_80x24_no_panic() {
    let mut app = create_test_app();
    app.current_tab = Tab::SomeTab;
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
}
```

## Key Test Patterns

### 1. Non-ASCII Cursor Edge Behavior

```rust
#[test]
fn test_is_at_right_edge_non_ascii() {
    let mut tab = SomeTab::default();
    tab.focus_area = SomeFocusArea::Inputs;
    tab.inputs.fields[0].value = "café".to_string();  // 5 bytes, 4 chars
    tab.inputs.fields[0].cursor_pos = tab.inputs.fields[0].value.len();  // 5 (byte len)
    assert!(tab.is_at_right_edge());
}
```

### 2. Help Text Generation

```rust
#[test]
fn test_help_text_normal_mode() {
    let app = create_test_app();
    let help = get_help_text(&app, Rect::new(0, 0, 80, 24));
    assert!(help.contains("[h/j/k/l]"));
}
```

### 3. Overlay Key Handling

```rust
#[test]
fn test_overlay_close_on_esc() {
    let mut app = create_test_app();
    app.show_search = true;
    // Simulate Esc key
    // assert!(...);
}
```

## Common Bugs to Watch For

1. **Byte vs Char bug**: `cursor_pos` is byte index, don't compare with `chars().count()`
   - Wrong: `cursor_pos >= value.chars().count()`
   - Correct: `cursor_pos >= value.len()`

2. **Static cache**: Avoid `static LazyLock<Vec<T>>` for tab titles
   - Build from `Tab::all()` each render instead

3. **Overlay precedence**: Always use `topmost_overlay()` in event handling
   - Confirm popup > Command palette > Search > HTTP options > Help

## Render Tests

Always add render tests for new tabs or major layout changes:

```rust
#[test]
fn test_render_at_80x24_no_panic() {
    let mut app = create_test_app();
    app.current_tab = Tab::SomeTab;
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui::draw(f, &mut app)).unwrap();
}
```
