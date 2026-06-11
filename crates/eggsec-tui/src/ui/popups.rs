use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    widgets::{Block, BorderType, Borders},
    Frame,
};

use crate::components::centered_rect;
use crate::theme::Theme;
use crate::App;

pub fn draw_http_options_popup(f: &mut Frame, app: &App, theme: &Theme) {
    use ratatui::widgets::{Clear, Paragraph};

    let area = f.area();
    // Phase 9: clamp popup to viewport so it never overflows on small terminals (<60 etc).
    // centered_rect already clamps, but compute constrained min size for very small.
    let popup_width = if area.width < 50 {
        area.width.saturating_sub(4).max(20)
    } else {
        50
    };
    let popup_height = if area.height < 20 {
        area.height.saturating_sub(4).max(6)
    } else {
        18
    };

    let popup_area = centered_rect(popup_width, popup_height, area);

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
    use ratatui::widgets::{Clear, List, ListItem, Paragraph};

    let palette = match app.command_palette.as_mut() {
        Some(pal) if pal.visible => pal,
        _ => return,
    };
    let area = f.area();

    // Phase 9: clamp to viewport; on very small use short message path (avoid overflow).
    let constrained_w = area.width.saturating_sub(2).min(60).max(20);
    let constrained_h = area.height.saturating_sub(4).min(20).max(6);
    let popup_area = centered_rect(constrained_w, constrained_h, area);

    f.render_widget(Clear, popup_area);

    if area.width < 50 {
        // Phase 9: explicit short message on very small to avoid garble; still allows Enter/Esc via input.
        let short = Paragraph::new("Palette (small term)\n[Esc] close [↑↓] sel [Enter] run")
            .style(Style::default().fg(theme.colors.text));
        f.render_widget(short, popup_area);
        return;
    }

    let block = Block::default()
        .title("Command Palette (Ctrl+P to close, Up/Down to navigate, Enter to select)")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.colors.highlight));

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

    let content_height = chunks.get(2).copied().unwrap_or(inner).height;
    palette.update_content_height(content_height);

    let query_paragraph = Paragraph::new(format!("Query: {}", palette.query)).style(
        Style::default()
            .fg(theme.colors.text)
            .bg(theme.colors.surface),
    );
    f.render_widget(query_paragraph, chunks.first().copied().unwrap_or(inner));

    let visible_height = palette.visible_results_height();
    let total = palette.results.len();
    let start = palette.scroll_offset;
    let end = (start + visible_height).min(total);
    let status_text = if total > 0 {
        format!("{}/{}", end.min(total), total)
    } else {
        "0/0".to_string()
    };
    let status_paragraph =
        Paragraph::new(status_text.as_str()).style(Style::default().fg(theme.colors.text_dim));
    f.render_widget(status_paragraph, chunks.get(1).copied().unwrap_or(inner));

    let mut items: Vec<ListItem> = Vec::new();
    for global_idx in start..end {
        let result = &palette.results[global_idx];
        let style = if global_idx == palette.selected_index {
            Style::default()
                .fg(theme.colors.background)
                .bg(theme.colors.highlight)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.colors.text)
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
                .border_style(Style::default().fg(theme.colors.border)),
        )
        .style(Style::default().fg(theme.colors.text));
    f.render_widget(list, chunks.get(2).copied().unwrap_or(inner));
}

pub fn draw_search_popup(f: &mut Frame, app: &App, theme: &Theme) {
    use ratatui::widgets::{Clear, Paragraph};

    let area = f.area();
    // Phase 9: clamp to avoid overflow on small viewports.
    let popup_width = if area.width < 60 {
        area.width.saturating_sub(4).max(16)
    } else {
        60
    };
    let popup_height = if area.height < 8 {
        area.height.saturating_sub(2).max(3)
    } else {
        5
    };

    let popup_area = centered_rect(popup_width, popup_height, area);

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

    let paragraph = Paragraph::new(search_content).style(Style::default().fg(theme.colors.text));
    f.render_widget(paragraph, inner);
}

pub fn draw_quick_switch(f: &mut Frame, app: &mut App, theme: &Theme) {
    use ratatui::widgets::{Clear, List, ListItem, Paragraph};

    let area = f.area();
    // Phase 9: clamp to viewport; short path on very small.
    let popup_width = if area.width < 60 {
        area.width.saturating_sub(2).max(18)
    } else {
        60
    };
    let popup_height = if area.height < 20 {
        area.height.saturating_sub(4).max(6)
    } else {
        18
    };

    let popup_area = centered_rect(popup_width, popup_height, area);

    f.render_widget(Clear, popup_area);

    if area.width < 50 {
        // Phase 9 small viewport short message (input still works via overlay routing).
        let short = Paragraph::new("Quick switch (small)\n[^X close] [↑↓] [Enter]")
            .style(Style::default().fg(theme.colors.text));
        f.render_widget(short, popup_area);
        return;
    }

    let block = Block::default()
        .title("Tab Search (Ctrl+X to close, Enter to select, Up/Down to navigate)")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.colors.primary));

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

    let query_paragraph = Paragraph::new(format!("Filter: {}", app.quick_switch.query)).style(
        Style::default()
            .fg(theme.colors.text)
            .bg(theme.colors.surface),
    );
    f.render_widget(query_paragraph, chunks.first().copied().unwrap_or(inner));

    let results = app.get_quick_switch_results();
    let selected_display = if results.is_empty() {
        0
    } else {
        app.quick_switch.selected.min(results.len() - 1) + 1
    };
    let status_text = format!("{}/{}", selected_display, results.len());
    let status_paragraph =
        Paragraph::new(status_text.as_str()).style(Style::default().fg(theme.colors.text_dim));
    f.render_widget(status_paragraph, chunks.get(1).copied().unwrap_or(inner));

    let visible_rows = chunks
        .get(2)
        .copied()
        .unwrap_or(inner)
        .height
        .saturating_sub(2)
        .max(1) as usize;
    let selected = app
        .quick_switch
        .selected
        .min(results.len().saturating_sub(1));
    let start = selected.saturating_sub(visible_rows.saturating_sub(1));
    let end = (start + visible_rows).min(results.len());

    let mut items: Vec<ListItem> = Vec::new();
    for (offset, tab) in results[start..end].iter().enumerate() {
        let i = start + offset;
        let style = if i == selected {
            Style::default()
                .fg(theme.colors.background)
                .bg(theme.colors.highlight)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.colors.text)
        };
        let item_text = format!("{} - {}", tab.title(), tab.description());
        items.push(ListItem::new(item_text).style(style));
    }

    if items.is_empty() {
        items.push(
            ListItem::new("(No matching tabs found)")
                .style(Style::default().fg(theme.colors.text_dim)),
        );
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme.colors.border)),
        )
        .style(Style::default().fg(theme.colors.text));
    f.render_widget(list, chunks.get(2).copied().unwrap_or(inner));
}
