use ratatui::layout::Rect;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::components::centered_rect;
use crate::theme::Theme;
use crate::App;

/// Computes a popup area constrained to the viewport, clamping width and height
/// to avoid overflow on small terminals. `margin` is the total horizontal/vertical
/// space reserved outside the popup (e.g., 4 for 2px on each side).
fn constrained_popup_area(area: Rect, width: u16, height: u16, margin: u16) -> Rect {
    let w = area.width.saturating_sub(margin).min(width).max(16);
    let h = area.height.saturating_sub(margin).min(height).max(4);
    centered_rect(w, h, area)
}

/// Shared rendering for selectable-list popups (command palette, quick switch).
///
/// Handles the common shell: popup area, clear, small terminal fallback, block
/// with title/border, vertical split (query + status + list), empty state.
/// The caller provides pre-built `items` and metadata.
struct SelectableListPopup {
    title: &'static str,
    border_color: ratatui::style::Color,
    query_text: String,
    status_text: String,
    items: Vec<ListItem<'static>>,
    empty_message: &'static str,
    area: Rect,
}

impl SelectableListPopup {
    fn render(self, f: &mut Frame, theme: &Theme) {
        let popup_area = constrained_popup_area(self.area, 60, 20, 2);
        f.render_widget(ratatui::widgets::Clear, popup_area);

        if self.area.width < 50 {
            let short = Paragraph::new(format!(
                "{} (small)\n[Esc close] [Up/Down] [Enter]",
                self.title.split('(').next().unwrap_or(self.title).trim()
            ))
            .style(Style::default().fg(theme.colors.text));
            f.render_widget(short, popup_area);
            return;
        }

        let block = Block::default()
            .title(self.title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(self.border_color));

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

        let query_paragraph = Paragraph::new(self.query_text).style(
            Style::default()
                .fg(theme.colors.text)
                .bg(theme.colors.surface),
        );
        f.render_widget(query_paragraph, chunks.first().copied().unwrap_or(inner));

        let status_paragraph = Paragraph::new(self.status_text.as_str())
            .style(Style::default().fg(theme.colors.text_dim));
        f.render_widget(status_paragraph, chunks.get(1).copied().unwrap_or(inner));

        let list_area = chunks.get(2).copied().unwrap_or(inner);
        if self.items.is_empty() {
            let empty = Paragraph::new(self.empty_message)
                .style(Style::default().fg(theme.colors.text_dim));
            f.render_widget(empty, list_area);
            return;
        }

        let list = List::new(self.items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme.colors.border)),
            )
            .style(Style::default().fg(theme.colors.text));
        f.render_widget(list, list_area);
    }
}

/// Creates a `ListItem` with the standard selected/unselected styling.
fn styled_list_item(
    text: impl Into<ratatui::text::Text<'static>>,
    is_selected: bool,
    theme: &Theme,
) -> ListItem<'static> {
    let style = if is_selected {
        Style::default()
            .fg(theme.colors.background)
            .bg(theme.colors.highlight)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.colors.text)
    };
    ListItem::new(text).style(style)
}

pub fn draw_http_options_popup(f: &mut Frame, app: &App, theme: &Theme) {
    use ratatui::widgets::{Clear, Paragraph};

    let area = f.area();
    let popup_area = constrained_popup_area(area, 50, 18, 4);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title("Global HTTP Options (press h to close)")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.colors.primary));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let opts = &app.http_options;
    let redacted = |v: Option<&str>| {
        if v.is_some() {
            "********".to_string()
        } else {
            "(not set)".to_string()
        }
    };
    let content = vec![
        format!(
            "  --insecure: {}",
            if opts.insecure { "true" } else { "false" }
        ),
        format!(
            "  --proxy: {}",
            opts.proxy.as_deref().unwrap_or("(not set)")
        ),
        format!("  --proxy-auth: {}", redacted(opts.proxy_auth.as_deref())),
        format!("  --auth: {}", redacted(opts.auth.as_deref())),
        format!("  --bearer: {}", redacted(opts.bearer.as_deref())),
        format!("  --cookie: {}", redacted(opts.cookie.as_deref())),
        format!("  --api-key: {}", redacted(opts.api_key.as_deref())),
        format!(
            "  --user-agent: {}",
            opts.user_agent.as_deref().unwrap_or("(not set)")
        ),
        format!(
            "  --stealth: {}",
            if opts.stealth { "true" } else { "false" }
        ),
        format!(
            "  --rate-limit: {}",
            opts.rate_limit
                .map(|r| r.to_string())
                .unwrap_or("(not set)".to_string())
        ),
        format!(
            "  --jitter: {}",
            opts.jitter.as_deref().unwrap_or("(not set)")
        ),
    ];

    let paragraph =
        Paragraph::new(content.join("\n")).style(Style::default().fg(theme.colors.text));
    f.render_widget(paragraph, inner);
}

pub fn draw_command_palette(f: &mut Frame, app: &mut App, theme: &Theme) {
    let palette = match app.command_palette.as_mut() {
        Some(pal) if pal.visible => pal,
        _ => return,
    };
    let area = f.area();

    let visible_height = palette.visible_results_height();
    let total = palette.results.len();
    let start = palette.scroll_offset;
    let end = (start + visible_height).min(total);

    let content_height = constrained_popup_area(area, 60, 20, 2)
        .height
        .saturating_sub(4) // borders + margin
        .saturating_sub(3); // query + status
    palette.update_content_height(content_height);

    let status_text = if total > 0 {
        format!("{}/{}", end.min(total), total)
    } else {
        "0/0".to_string()
    };

    let empty_message = if palette.query.is_empty() {
        "No commands available"
    } else {
        "No matching commands"
    };

    let items: Vec<ListItem> = (start..end)
        .map(|global_idx| {
            let result = &palette.results[global_idx];
            let shortcut_text = result
                .shortcut
                .as_ref()
                .map(|s| format!(" [{}]", s))
                .unwrap_or_default();
            let text = format!(
                "{} - {}{}",
                result.command, result.description, shortcut_text
            );
            styled_list_item(text, global_idx == palette.selected_index, theme)
        })
        .collect();

    SelectableListPopup {
        title: "Command Palette (Ctrl+P to close, Up/Down to navigate, Enter to select)",
        border_color: theme.colors.highlight,
        query_text: format!("Query: {}", palette.query),
        status_text,
        items,
        empty_message,
        area,
    }
    .render(f, theme);
}

pub fn draw_search_popup(f: &mut Frame, app: &App, theme: &Theme) {
    use ratatui::widgets::{Clear, Paragraph};

    let area = f.area();
    let popup_area = constrained_popup_area(area, 60, 5, 4);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title("Search (press Esc to close, Enter to search)")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.colors.accent));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let search_content = if app.search.query.is_empty() {
        "Type to search...".to_string()
    } else {
        format!("Searching: {}", app.search.query)
    };

    let style = if app.search.query.is_empty() {
        Style::default().fg(theme.colors.text_dim)
    } else {
        Style::default().fg(theme.colors.text)
    };
    let paragraph = Paragraph::new(search_content).style(style);
    f.render_widget(paragraph, inner);
}

pub fn draw_quick_switch(f: &mut Frame, app: &mut App, theme: &Theme) {
    let area = f.area();
    let results = app.get_quick_switch_results();
    let selected = app
        .quick_switch
        .selected
        .min(results.len().saturating_sub(1));

    let selected_display = if results.is_empty() {
        0
    } else {
        selected + 1
    };
    let status_text = format!("{}/{}", selected_display, results.len());

    let empty_message = if app.quick_switch.query.is_empty() {
        "No tabs available"
    } else {
        "No matching tabs"
    };

    // Compute visible window around selected item
    let popup_area = constrained_popup_area(area, 60, 18, 2);
    let visible_rows = popup_area.height.saturating_sub(6).max(1) as usize; // borders(2) + margin(2) + query(2) + status(1)
    let start = selected.saturating_sub(visible_rows.saturating_sub(1));
    let end = (start + visible_rows).min(results.len());

    let items: Vec<ListItem> = results[start..end]
        .iter()
        .enumerate()
        .map(|(offset, tab)| {
            let i = start + offset;
            let text = format!("{} - {}", tab.title(), tab.description());
            styled_list_item(text, i == selected, theme)
        })
        .collect();

    SelectableListPopup {
        title: "Tab Search (Ctrl+X to close, Enter to select, Up/Down to navigate)",
        border_color: theme.colors.primary,
        query_text: format!("Filter: {}", app.quick_switch.query),
        status_text,
        items,
        empty_message,
        area,
    }
    .render(f, theme);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn constrained_popup_area_clamps_to_viewport() {
        let area = Rect::new(0, 0, 80, 24);
        let popup = constrained_popup_area(area, 200, 100, 4);
        assert!(popup.width <= 80);
        assert!(popup.height <= 24);
    }

    #[test]
    fn constrained_popup_area_enforces_minimum_size() {
        let area = Rect::new(0, 0, 40, 20);
        let popup = constrained_popup_area(area, 60, 20, 4);
        assert!(popup.width >= 16, "width {} should be >= 16", popup.width);
        assert!(popup.height >= 4, "height {} should be >= 4", popup.height);
    }

    #[test]
    fn constrained_popup_area_exact_fit() {
        let area = Rect::new(0, 0, 80, 24);
        let popup = constrained_popup_area(area, 50, 18, 4);
        assert_eq!(popup.width, 50);
        assert_eq!(popup.height, 18);
    }

    #[test]
    fn constrained_popup_area_small_terminal() {
        let area = Rect::new(0, 0, 40, 12);
        let popup = constrained_popup_area(area, 60, 20, 4);
        assert!(popup.width <= 36);
        assert!(popup.height <= 8);
        assert!(popup.width >= 16);
        assert!(popup.height >= 4);
    }
}
