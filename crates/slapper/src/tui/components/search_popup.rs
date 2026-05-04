use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};
use crate::tc;
use super::centered_rect;

pub fn draw_search_popup(f: &mut Frame, area: Rect, query: &str) {
    let popup_width = 60;
    let popup_height = 5;

    let popup_area = centered_rect(popup_width, popup_height, area);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title("Search (press Esc to close, Enter to search)")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(tc!(accent)));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let search_content = if query.is_empty() {
        "Type to search...".to_string()
    } else {
        format!("Searching: {}", query)
    };

    let paragraph = Paragraph::new(search_content).style(Style::default().fg(tc!(text)));
    f.render_widget(paragraph, inner);
}
