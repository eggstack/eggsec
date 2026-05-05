pub fn empty_state_paragraph(
    title: &'static str,
    text: impl Into<ratatui::text::Text<'static>>,
) -> ratatui::widgets::Paragraph<'static> {
    use ratatui::style::Style;
    use ratatui::widgets::{Block, Borders, Paragraph};

    Paragraph::new(text.into())
        .block(Block::default().borders(Borders::ALL).title(title))
        .style(Style::default().fg(crate::tc!(text_dim)))
}
