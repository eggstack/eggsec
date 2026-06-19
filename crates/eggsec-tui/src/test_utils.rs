/// Shared test utilities for the TUI crate.

/// Converts a `ratatui::Buffer` to a `String` for assertion in visual tests.
/// Each row is separated by a newline.
pub fn buffer_to_text(buf: &ratatui::buffer::Buffer) -> String {
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

pub fn test_theme() -> crate::theme::Theme {
    crate::theme::Theme::default()
}
