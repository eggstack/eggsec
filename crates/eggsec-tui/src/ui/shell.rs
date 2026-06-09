use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};

use crate::app::NotificationSeverity;
use crate::theme::Theme;
use crate::App;
use crate::InputMode;

pub fn draw_tabs(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    use crate::tabs::{Tab, TabWindow};
    use ratatui::text::Line;

    let window = TabWindow::for_width(area.width, app.current_tab, app.tab_scroll_offset);

    let all_tabs: Vec<Line> = Tab::all().iter().map(|t| Line::from(t.title())).collect();
    let visible_titles: Vec<Line> = all_tabs[window.start..window.end].to_vec();

    let tabs = Tabs::new(visible_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Eggsec{}", window.range_text())),
        )
        .select(window.selected_visible)
        .style(Style::default().fg(theme.colors.tab_active))
        .highlight_style(
            Style::default()
                .fg(theme.colors.highlight)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, area);
}

pub fn draw_breadcrumb(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    use ratatui::text::{Line, Span};

    let parts = app
        .current_tab
        .as_tab_render(app)
        .breadcrumb()
        .unwrap_or_else(|| app.current_tab.default_breadcrumb());

    let mut spans = Vec::new();
    let total_parts = parts.len();

    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(
                " > ",
                Style::default().fg(theme.colors.text_dim),
            ));
        }

        let is_last = i == total_parts - 1;
        let style = if is_last {
            Style::default()
                .fg(theme.colors.accent)
                .add_modifier(Modifier::BOLD)
        } else if i == 0 {
            Style::default()
                .fg(theme.colors.text)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.colors.primary)
        };

        spans.push(Span::styled(*part, style));
    }

    let block = Block::default()
        .borders(Borders::NONE)
        .border_style(Style::default().fg(theme.colors.border));

    let paragraph = Paragraph::new(Line::from(spans))
        .block(block)
        .style(Style::default().fg(theme.colors.text));

    f.render_widget(paragraph, area);
}

pub fn draw_content(f: &mut Frame, app: &App, area: Rect) {
    use crate::tabs::TabRender;
    let insert_mode = app.mode == crate::InputMode::Insert;

    if app.current_tab == crate::tabs::Tab::History {
        let h = app.history.lock();
        h.render(f, area, insert_mode);
        h.render_overlays(f, area);
        return;
    }

    let tab_render = app.current_tab.as_tab_render(app);
    tab_render.render(f, area, insert_mode);
    tab_render.render_overlays(f, area);
}

pub fn draw_status_bar(f: &mut Frame, app: &App, theme: &Theme, area: Rect) {
    let (status_text, status_color) = if let Some(notif) = &app.overlay.notification {
        if !notif.is_expired() {
            let color = match notif.severity {
                NotificationSeverity::Info => theme.colors.status_idle,
                NotificationSeverity::Success => theme.colors.success,
                NotificationSeverity::Warning => theme.colors.warning,
                NotificationSeverity::Error => theme.colors.error,
            };
            (notif.message.clone(), color)
        } else {
            get_normal_status(app, theme)
        }
    } else {
        get_normal_status(app, theme)
    };

    let help_text = get_help_text(app, area);

    let use_compact = area.width < 100;
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(if use_compact {
            [
                Constraint::Length(8),
                Constraint::Percentage(60),
                Constraint::Percentage(40),
            ]
        } else {
            [
                Constraint::Length(10),
                Constraint::Percentage(55),
                Constraint::Percentage(40),
            ]
        })
        .split(area);

    let mode_text = match app.mode {
        InputMode::Normal => "NORMAL",
        InputMode::Insert => "INSERT",
    };
    let mode_color = match app.mode {
        InputMode::Normal => theme.colors.mode_normal,
        InputMode::Insert => theme.colors.mode_insert,
    };
    let mode_indicator_widget = ratatui::widgets::Paragraph::new(format!(" {} ", mode_text)).style(
        Style::default()
            .fg(theme.colors.background)
            .bg(mode_color)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(
        mode_indicator_widget,
        chunks.first().copied().unwrap_or(area),
    );

    let status =
        ratatui::widgets::Paragraph::new(status_text).style(Style::default().fg(status_color));
    f.render_widget(status, chunks.get(1).copied().unwrap_or(area));

    let help = ratatui::widgets::Paragraph::new(help_text)
        .style(Style::default().fg(theme.colors.text_dim));
    f.render_widget(help, chunks.get(2).copied().unwrap_or(area));
}

pub fn get_tab_status(
    state: &crate::tabs::AppState,
    theme: &Theme,
) -> (String, ratatui::style::Color) {
    use crate::tabs::AppState;
    match state {
        AppState::Idle => (
            "Ready - Press Enter to start".to_string(),
            theme.colors.status_idle,
        ),
        AppState::Running => (
            "Running - Ctrl+C to stop".to_string(),
            theme.colors.status_running,
        ),
        AppState::Completed => ("Completed".to_string(), theme.colors.success),
        AppState::Error(e) => (e.to_string(), theme.colors.error),
    }
}

pub fn get_normal_status(app: &App, theme: &Theme) -> (String, ratatui::style::Color) {
    match app.current_tab {
        crate::tabs::Tab::Settings => (
            "Press 's' to save settings, 'r' to reset".to_string(),
            theme.colors.status_idle,
        ),
        crate::tabs::Tab::History => (
            "↑↓ Navigate | 'd' Delete | 'r' Clear all".to_string(),
            theme.colors.status_idle,
        ),
        crate::tabs::Tab::Dashboard => (
            "Dashboard - View scan results overview".to_string(),
            theme.colors.status_idle,
        ),
        _ => {
            let state = app.current_tab.as_tab_state(app).state();
            get_tab_status(&state, theme)
        }
    }
}

pub fn get_help_text(app: &App, area: Rect) -> String {
    let is_narrow = area.width < 80;

    if app.overlay.pending_action.is_some() {
        return "[Enter] Confirm [Esc] Cancel".to_string();
    }

    if app
        .get_command_palette()
        .map(|p| p.visible)
        .unwrap_or(false)
    {
        return if is_narrow {
            "[Enter] Run [↑↓] Sel [Esc] Close".to_string()
        } else {
            "[Enter] Run [Up/Down] Select [Esc] Close".to_string()
        };
    }

    if app.overlay.show_search {
        return if is_narrow {
            "[Enter] Search [Bksp] Edit [Esc] Close".to_string()
        } else {
            "[Enter] Search [Backspace] Edit [Esc] Close".to_string()
        };
    }

    if app.overlay.show_help {
        return if is_narrow {
            "[Esc] Close | [h/l] Pane Nav".to_string()
        } else {
            "[Esc] Close Help | [h/l] Pane Navigation".to_string()
        };
    }

    match app.mode {
        InputMode::Normal => {
            if is_narrow {
                format!(
                    "[n/p] Tabs [hjkl] Move [/] Search [^X] Quick{} [q] Quit",
                    if app.is_paused() { " [P]" } else { "" }
                )
            } else {
                format!(
                    "[n/p] Tabs [hjkl] Move [/] Search [Ctrl+X] Quick Switch [Space] Help [q] Quit{}",
                    if app.is_paused() { " [Ctrl+Y] Resume" } else { "" }
                )
            }
        }
        InputMode::Insert => {
            if is_narrow {
                "[Esc] Normal [Tab] Next [Arw] Move [^V] Paste".to_string()
            } else {
                "[Esc] Normal Mode | [Tab/S-Tab] Focus | [Arrows] Move | [Ctrl+V] Paste".to_string()
            }
        }
    }
}
