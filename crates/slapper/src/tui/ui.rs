use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};

use super::App;
use crate::tui::components::{centered_rect, confirm_popup, help_popup_for_tab};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(f.area());

    draw_tabs(f, app, chunks[0]);
    draw_breadcrumb(f, app, chunks[1]);
    draw_content(f, app, chunks[2]);
    draw_status_bar(f, app, chunks[3]);

    if app.show_help {
        let help = help_popup_for_tab(app.current_tab);
        help.render(f, f.area());

        // Add context help below the popup
        let context_help = app.get_current_help();
        let context_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(help.height + 2), Constraint::Min(0)])
            .split(f.area());

        let context_paragraph = Paragraph::new(context_help).style(
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
        );
        f.render_widget(context_paragraph, context_chunks[1]);
    }

    if let Some(palette) = app.get_command_palette() {
        if palette.visible {
            draw_command_palette(f, app);
        }
    }

    if app.show_search {
        draw_search_popup(f, app);
    }

    if app.show_http_options {
        draw_http_options_popup(f, app);
    }

    if let Some(action) = app.pending_action {
        let (title, message) = action.message();
        let popup = confirm_popup(&title, &message);
        popup.render(f, f.area());
    }
}

fn draw_http_options_popup(f: &mut Frame, app: &App) {
    use ratatui::widgets::{Clear, Paragraph};

    let popup_width = 50;
    let popup_height = 18;

    let area = f.area();
    let popup_area = centered_rect(popup_width, popup_height, area);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title("Global HTTP Options (press h to close)")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let opts = &app.http_options;
    let content = vec![
        format!(
            "  --insecure: {}",
            if opts.insecure { "true" } else { "false" }
        ),
        format!(
            "  --proxy: {}",
            opts.proxy.as_deref().unwrap_or("(not set)")
        ),
        format!(
            "  --proxy-auth: {}",
            opts.proxy_auth.as_deref().unwrap_or("(not set)")
        ),
        format!("  --auth: {}", opts.auth.as_deref().unwrap_or("(not set)")),
        format!(
            "  --bearer: {}",
            opts.bearer.as_deref().unwrap_or("(not set)")
        ),
        format!(
            "  --cookie: {}",
            opts.cookie.as_deref().unwrap_or("(not set)")
        ),
        format!(
            "  --api-key: {}",
            opts.api_key.as_deref().unwrap_or("(not set)")
        ),
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

    let paragraph = Paragraph::new(content.join("\n")).style(Style::default().fg(Color::White));
    f.render_widget(paragraph, inner);
}

fn draw_command_palette(f: &mut Frame, app: &App) {
    use ratatui::widgets::{Clear, List, ListItem, Paragraph};

    let Some(palette) = app.get_command_palette() else {
        tracing::error!("Command palette unavailable despite being checked in caller");
        return;
    };
    let area = f.area();
    let popup_width = 60;
    let popup_height = 20;

    let popup_area = centered_rect(popup_width, popup_height, area);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title("Command Palette (Ctrl+P to close, Up/Down to navigate, Enter to select)")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

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

    // Query input
    let query_paragraph = Paragraph::new(format!("Query: {}", palette.query))
        .style(Style::default().fg(Color::White).bg(Color::DarkGray));
    f.render_widget(query_paragraph, chunks[0]);

    // Results
    let mut items: Vec<ListItem> = Vec::new();
    for (i, result) in palette.results.iter().enumerate() {
        let style = if i == palette.selected_index {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
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
                .border_style(Style::default().fg(Color::Gray)),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(list, chunks[2]);
}

fn draw_search_popup(f: &mut Frame, app: &App) {
    use ratatui::widgets::{Clear, Paragraph};

    let popup_width = 60;
    let popup_height = 5;

    let area = f.area();
    let popup_area = centered_rect(popup_width, popup_height, area);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title("Search (press Esc to close, Enter to search)")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let search_content = if app.search_query.is_empty() {
        "Type to search...".to_string()
    } else {
        format!("Searching: {}", app.search_query)
    };

    let paragraph = Paragraph::new(search_content).style(Style::default().fg(Color::White));
    f.render_widget(paragraph, inner);
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    use crate::tui::tabs::Tab;
    use ratatui::text::Line;

    let titles: Vec<Line> = Tab::all().iter().map(|t| Line::from(t.title())).collect();

    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("Slapper"))
        .select(app.current_tab as usize)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, area);
}

fn draw_breadcrumb(f: &mut Frame, app: &App, area: Rect) {
    use crate::tui::tabs::TabRender;
    use ratatui::text::Line;

    let breadcrumb_parts: Vec<Line> = match app.current_tab {
        crate::tui::tabs::Tab::Recon => {
            if let Some(parts) = app.recon.breadcrumb() {
                parts
            } else {
                vec!["Recon"]
            }
        }
        crate::tui::tabs::Tab::Load => vec!["Load"],
        crate::tui::tabs::Tab::ScanPorts => vec!["Scan Ports"],
        crate::tui::tabs::Tab::ScanEndpoints => vec!["Scan Endpoints"],
        crate::tui::tabs::Tab::Fingerprint => vec!["Fingerprint"],
        crate::tui::tabs::Tab::Fuzz => {
            if let Some(parts) = app.fuzz.breadcrumb() {
                parts
            } else {
                vec!["Fuzz"]
            }
        }
        crate::tui::tabs::Tab::Waf => {
            if let Some(parts) = app.waf.breadcrumb() {
                parts
            } else {
                vec!["WAF"]
            }
        }
        crate::tui::tabs::Tab::WafStress => vec!["WAF Stress"],
        crate::tui::tabs::Tab::Scan => vec!["Scan"],
        crate::tui::tabs::Tab::Resume => vec!["Resume"],
        crate::tui::tabs::Tab::Proxy => {
            if let Some(parts) = app.proxy.breadcrumb() {
                parts
            } else {
                vec!["Proxy"]
            }
        }
        crate::tui::tabs::Tab::Packet => {
            if let Some(parts) = app.packet.breadcrumb() {
                parts
            } else {
                vec!["Packet"]
            }
        }
        crate::tui::tabs::Tab::GraphQl => vec!["GraphQL Security"],
        crate::tui::tabs::Tab::OAuth => vec!["OAuth/OIDC Security"],
        crate::tui::tabs::Tab::Cluster => vec!["Cluster Management"],
        crate::tui::tabs::Tab::Stress => vec!["Stress Testing"],
        crate::tui::tabs::Tab::Report => vec!["Report"],
        crate::tui::tabs::Tab::Nse => vec!["NSE Scripts"],
        #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
        crate::tui::tabs::Tab::Plugin => vec!["Plugins"],
        #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
        crate::tui::tabs::Tab::Plugin => vec!["Plugins"],
        crate::tui::tabs::Tab::Settings => vec!["Settings"],
        crate::tui::tabs::Tab::History => vec!["History"],
        crate::tui::tabs::Tab::Dashboard => vec!["Dashboard"],
    }
    .iter()
    .enumerate()
    .map(|(i, part)| {
        if i == 0 {
            Line::from(Span::styled(
                *part,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ))
        } else {
            Line::from(vec![
                Span::raw(" > "),
                Span::styled(*part, Style::default().fg(Color::Cyan)),
            ])
        }
    })
    .collect();

    let breadcrumb_line: Line = breadcrumb_parts.into_iter().flatten().collect();

    let block = Block::default()
        .borders(Borders::NONE)
        .border_style(Style::default().fg(Color::DarkGray));

    let paragraph = Paragraph::new(breadcrumb_line)
        .block(block)
        .style(Style::default().fg(Color::White));

    f.render_widget(paragraph, area);
}

fn draw_content(f: &mut Frame, app: &App, area: Rect) {
    use crate::tui::tabs::TabRender;
    let insert_mode = app.mode == crate::tui::InputMode::Insert;

    match app.current_tab {
        crate::tui::tabs::Tab::Recon => {
            app.recon.render(f, area, insert_mode);
            app.recon.render_overlays(f, area);
        }
        crate::tui::tabs::Tab::Load => {
            app.load.render(f, area, insert_mode);
            app.load.render_overlays(f, area);
        }
        crate::tui::tabs::Tab::ScanPorts => {
            app.scan_ports.render(f, area, insert_mode);
            app.scan_ports.render_overlays(f, area);
        }
        crate::tui::tabs::Tab::ScanEndpoints => {
            app.scan_endpoints.render(f, area, insert_mode);
            app.scan_endpoints.render_overlays(f, area);
        }
        crate::tui::tabs::Tab::Fingerprint => {
            app.fingerprint.render(f, area, insert_mode);
            app.fingerprint.render_overlays(f, area);
        }
        crate::tui::tabs::Tab::Fuzz => {
            app.fuzz.render(f, area, insert_mode);
            app.fuzz.render_overlays(f, area);
        }
        crate::tui::tabs::Tab::Waf => {
            app.waf.render(f, area, insert_mode);
            app.waf.render_overlays(f, area);
        }
        crate::tui::tabs::Tab::WafStress => {
            app.waf_stress.render(f, area, insert_mode);
            app.waf_stress.render_overlays(f, area);
        }
        crate::tui::tabs::Tab::Scan => {
            app.scan.render(f, area, insert_mode);
            app.scan.render_overlays(f, area);
        }
        crate::tui::tabs::Tab::Resume => {
            app.resume.render(f, area, insert_mode);
            app.resume.render_overlays(f, area);
        }
        crate::tui::tabs::Tab::Proxy => {
            app.proxy.render(f, area, insert_mode);
            app.proxy.render_overlays(f, area);
        }
        crate::tui::tabs::Tab::Packet => {
            app.packet.render(f, area, insert_mode);
            app.packet.render_overlays(f, area);
        }
        crate::tui::tabs::Tab::GraphQl => {
            app.graphql.render(f, area, insert_mode);
        }
        crate::tui::tabs::Tab::OAuth => {
            app.oauth.render(f, area, insert_mode);
        }
        crate::tui::tabs::Tab::Cluster => {
            app.cluster.render(f, area, insert_mode);
        }
        crate::tui::tabs::Tab::Stress => {
            app.stress.render(f, area, insert_mode);
        }
        crate::tui::tabs::Tab::Report => {
            app.report.render(f, area, insert_mode);
        }
        #[cfg(feature = "nse")]
        crate::tui::tabs::Tab::Nse => {
            app.nse.render(f, area, insert_mode);
        }
        #[cfg(not(feature = "nse"))]
        crate::tui::tabs::Tab::Nse => {}
        #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
        crate::tui::tabs::Tab::Plugin => {
            app.plugin.render(f, area, insert_mode);
        }
        #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
        crate::tui::tabs::Tab::Plugin => {}
        crate::tui::tabs::Tab::Settings => {
            app.settings.render(f, area, insert_mode);
            app.settings.render_overlays(f, area);
        }
        crate::tui::tabs::Tab::History => {
            if let Ok(h) = app.history.lock() {
                h.render(f, area, insert_mode);
                h.render_overlays(f, area);
            }
        }
        crate::tui::tabs::Tab::Dashboard => {
            app.dashboard.render(f, area, insert_mode);
            app.dashboard.render_overlays(f, area);
        }
    }
}

fn get_tab_status(
    _tab: crate::tui::tabs::Tab,
    state: &crate::tui::tabs::AppState,
) -> (String, Color) {
    use crate::tui::tabs::AppState;
    match state {
        AppState::Idle => ("Ready - Press Enter to start".to_string(), Color::Gray),
        AppState::Running => ("Running - Ctrl+C to stop".to_string(), Color::Yellow),
        AppState::Completed => ("Completed".to_string(), Color::Green),
        AppState::Error(e) => (e.to_string(), Color::Red),
    }
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    use crate::tui::tabs::AppState;

    let (status_text, status_color) = match app.current_tab {
        crate::tui::tabs::Tab::Recon => get_tab_status(app.current_tab, &app.recon.state),
        crate::tui::tabs::Tab::Load => get_tab_status(app.current_tab, &app.load.state),
        crate::tui::tabs::Tab::ScanPorts => get_tab_status(app.current_tab, &app.scan_ports.state),
        crate::tui::tabs::Tab::ScanEndpoints => {
            get_tab_status(app.current_tab, &app.scan_endpoints.state)
        }
        crate::tui::tabs::Tab::Fingerprint => {
            get_tab_status(app.current_tab, &app.fingerprint.state)
        }
        crate::tui::tabs::Tab::Fuzz => get_tab_status(app.current_tab, &app.fuzz.state),
        crate::tui::tabs::Tab::Waf => get_tab_status(app.current_tab, &app.waf.state),
        crate::tui::tabs::Tab::WafStress => get_tab_status(app.current_tab, &app.waf_stress.state),
        crate::tui::tabs::Tab::Scan => get_tab_status(app.current_tab, &app.scan.state),
        crate::tui::tabs::Tab::Resume => match &app.resume.state {
            AppState::Idle => (
                "Ready - Enter session file and press Enter".to_string(),
                Color::Gray,
            ),
            AppState::Running => (
                "Loading session - Ctrl+C to stop".to_string(),
                Color::Yellow,
            ),
            AppState::Completed => ("Session loaded".to_string(), Color::Green),
            AppState::Error(e) => (e.to_string(), Color::Red),
        },
        crate::tui::tabs::Tab::Proxy => get_tab_status(app.current_tab, &app.proxy.state),
        crate::tui::tabs::Tab::Packet => get_tab_status(app.current_tab, &app.packet.state),
        crate::tui::tabs::Tab::Settings => (
            "Press 's' to save settings, 'r' to reset".to_string(),
            Color::Gray,
        ),
        crate::tui::tabs::Tab::History => (
            "↑↓ Navigate | 'd' Delete | 'r' Clear all".to_string(),
            Color::Gray,
        ),
        crate::tui::tabs::Tab::Dashboard => (
            "Dashboard - View scan results overview".to_string(),
            Color::Gray,
        ),
        crate::tui::tabs::Tab::GraphQl => get_tab_status(app.current_tab, &app.graphql.state),
        crate::tui::tabs::Tab::OAuth => get_tab_status(app.current_tab, &app.oauth.state),
        crate::tui::tabs::Tab::Cluster => get_tab_status(app.current_tab, &app.cluster.state),
        crate::tui::tabs::Tab::Stress => get_tab_status(app.current_tab, &app.stress.state),
        crate::tui::tabs::Tab::Report => get_tab_status(app.current_tab, &app.report.state),
        #[cfg(feature = "nse")]
        crate::tui::tabs::Tab::Nse => get_tab_status(app.current_tab, &app.nse.state),
        #[cfg(not(feature = "nse"))]
        crate::tui::tabs::Tab::Nse => ("NSE not available".to_string(), Color::Gray),
        #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
        crate::tui::tabs::Tab::Plugin => get_tab_status(app.current_tab, &app.plugin.state),
        #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
        crate::tui::tabs::Tab::Plugin => ("Plugins not available".to_string(), Color::Gray),
    };

    let _mode_style = match app.mode {
        super::InputMode::Normal => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
        super::InputMode::Insert => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    };

    let help_text = if app.is_help_visible() {
        " [Esc] Close | [Space] Help | [Enter] Confirm | [h/l] Tab | [j/k] Nav | [w/b] Word | [gg/Top] [G/Bot] | [n/p] Tab | [q] Quit ".to_string()
    } else {
        match app.mode {
            super::InputMode::Normal => {
                let mut help = " [h/l] Tab | [j/k] Nav | [w/b] Word | [1-9] Jump | [Space] Help | [i] Insert | [q] Quit | [Enter] Start | [e] Export ".to_string();
                if let Some(palette) = app.get_command_palette() {
                    if palette.visible {
                        help.push_str("[Ctrl+P] Close Palette ");
                    } else {
                        help.push_str("[Ctrl+P] Open Palette ");
                    }
                }
                help
            }
            super::InputMode::Insert => {
                " [Esc] Normal | Type to input | [Ctrl+C] Cancel ".to_string()
            }
        }
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(10), // [MODE][HELP]
            Constraint::Percentage(55),
            Constraint::Percentage(40),
        ])
        .split(area);

    // Mode indicator (NORMAL/INSERT)
    let mode_text = match app.mode {
        super::InputMode::Normal => "NORMAL",
        super::InputMode::Insert => "INSERT",
    };
    let mode_color = match app.mode {
        super::InputMode::Normal => Color::Green,
        super::InputMode::Insert => Color::Yellow,
    };
    let mode_indicator_widget = ratatui::widgets::Paragraph::new(format!(" {} ", mode_text)).style(
        Style::default()
            .fg(Color::Black)
            .bg(mode_color)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(mode_indicator_widget, chunks[0]);

    let status =
        ratatui::widgets::Paragraph::new(status_text).style(Style::default().fg(status_color));
    f.render_widget(status, chunks[1]);

    let help =
        ratatui::widgets::Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[2]);
}
