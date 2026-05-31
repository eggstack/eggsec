use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use crate::tc;
use crate::tui::help::CommandPalette;
use super::centered_rect;

pub fn draw_command_palette(f: &mut Frame, area: Rect, palette: &mut CommandPalette) {
    let popup_area = centered_rect(palette.popup_width, palette.popup_height, area);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title("Command Palette (Ctrl+P to close, Up/Down to navigate, Enter to select)")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(tc!(highlight)));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

    if chunks.len() < 3 {
        return;
    }

    let content_height = if let Some(chunk) = chunks.get(2) {
        chunk.height
    } else {
        return;
    };
    palette.update_content_height(content_height);

    // Query input
    let query_paragraph = Paragraph::new(format!("Query: {}", palette.query))
        .style(Style::default().fg(tc!(text)).bg(tc!(surface)));
    if let Some(chunk) = chunks.get(0) {
        f.render_widget(query_paragraph, *chunk);
    }

    // Pagination
    let visible_height = palette.visible_results_height();
    let total = palette.results.len();
    let scroll_offset = palette.scroll_offset.min(total.saturating_sub(visible_height).max(0));
    let start = scroll_offset;
    let end = (start + visible_height).min(total);
    let status_text = if total > 0 {
        format!("{}/{}", end.min(total), total)
    } else {
        "0/0".to_string()
    };
    let status_paragraph =
        Paragraph::new(status_text.as_str()).style(Style::default().fg(tc!(text_dim)));
    if let Some(chunk) = chunks.get(1) {
        f.render_widget(status_paragraph, *chunk);
    }

    // Results (only visible items)
    let mut items: Vec<ListItem> = Vec::new();
    for global_idx in start..end {
        let result = match palette.results.get(global_idx) {
            Some(r) => r,
            None => continue,
        };
        let style = if global_idx == palette.selected_index {
            Style::default()
                .fg(tc!(background))
                .bg(tc!(highlight))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(tc!(text))
        };

        let shortcut_text = result
            .shortcut
            .as_ref()
            .map(|s| format!(" [{}]", s))
            .unwrap_or_default();

        let command_text = format!(
            "{} - {}{}",
            result.command, result.description, shortcut_text
        );
        items.push(ListItem::new(command_text).style(style));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(tc!(border))),
        )
        .style(Style::default().fg(tc!(text)));
    if let Some(chunk) = chunks.get(2) {
        f.render_widget(list, *chunk);
    }
}
