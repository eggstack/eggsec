use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use crate::tc;

pub fn draw_help_bar(f: &mut Frame, area: Rect, hints: Vec<(&'static str, &'static str)>) {
    let mut spans = Vec::new();

    for (i, (key, desc)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(
            format!(" {} ", key),
            Style::default()
                .fg(tc!(background))
                .bg(tc!(primary))
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(format!(" {} ", desc)));
    }

    let paragraph = Paragraph::new(Line::from(spans)).style(Style::default().fg(tc!(text_dim)));
    f.render_widget(paragraph, area);
}
