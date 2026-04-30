use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};

use crate::tc;

use super::App;
use crate::tui::components::{centered_rect, confirm_popup, help_popup_for_tab};

/// Layout constants — shared with `runner.rs` for mouse hit-testing.
const LAYOUT_MARGIN: u16 = 1;
const TAB_BAR_HEIGHT: u16 = 3;

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(LAYOUT_MARGIN)
        .constraints([
            Constraint::Length(TAB_BAR_HEIGHT),
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
                .fg(tc!(text_dim))
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

    if app.show_search && !app.search_query.is_empty() {
        if let Some(ref search) = app.global_search {
            if !search.is_empty() {
                crate::tui::search::draw_search_results(f, app);
            }
        }
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
        .border_style(Style::default().fg(tc!(primary)));

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

    let paragraph = Paragraph::new(content.join("\n")).style(Style::default().fg(tc!(text)));
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

    // Query input
    let query_paragraph = Paragraph::new(format!("Query: {}", palette.query))
        .style(Style::default().fg(tc!(text)).bg(tc!(surface)));
    f.render_widget(query_paragraph, chunks[0]);

    // Pagination
    let visible_height = 14usize;
    let total = palette.results.len();
    let start = palette.scroll_offset;
    let end = (start + visible_height).min(total);
    let status_text = if total > 0 {
        format!("{}/{}", end.min(total), total)
    } else {
        "0/0".to_string()
    };
    let status_paragraph =
        Paragraph::new(status_text.as_str()).style(Style::default().fg(tc!(text_dim)));
    f.render_widget(status_paragraph, chunks[1]);

    // Results (only visible items)
    let mut items: Vec<ListItem> = Vec::new();
    for global_idx in start..end {
        let result = &palette.results[global_idx];
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
                .border_style(Style::default().fg(tc!(border))),
        )
        .style(Style::default().fg(tc!(text)));
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
        .border_style(Style::default().fg(tc!(accent)));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let search_content = if app.search_query.is_empty() {
        "Type to search...".to_string()
    } else {
        format!("Searching: {}", app.search_query)
    };

    let paragraph = Paragraph::new(search_content).style(Style::default().fg(tc!(text)));
    f.render_widget(paragraph, inner);
}

fn draw_tabs(f: &mut Frame, app: &App, area: Rect) {
    use crate::tui::tabs::{Tab, TabWindow};
    use ratatui::text::Line;

    let window = TabWindow::for_width(area.width, app.current_tab, app.tab_scroll_offset);

    use std::sync::LazyLock;
    static TAB_TITLES: LazyLock<Vec<Line>> = LazyLock::new(|| {
        Tab::all().iter().map(|t| Line::from(t.title())).collect()
    });

    let visible_titles: Vec<Line> = TAB_TITLES[window.start..window.end]
        .iter()
        .cloned()
        .collect();

    let tabs = Tabs::new(visible_titles)
        .block(Block::default().borders(Borders::ALL).title(format!("Slapper{}", window.range_text())))
        .select(window.selected_visible)
        .style(Style::default().fg(tc!(tab_active)))
        .highlight_style(
            Style::default()
                .fg(tc!(highlight))
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, area);
}

fn draw_breadcrumb(f: &mut Frame, app: &App, area: Rect) {
    use ratatui::text::Line;

    let breadcrumb_parts: Vec<Line> = app
        .current_tab
        .as_tab_render(app)
        .breadcrumb()
        .unwrap_or_else(|| app.current_tab.default_breadcrumb())
    .iter()
    .enumerate()
    .map(|(i, part)| {
        if i == 0 {
            Line::from(Span::styled(
                *part,
                Style::default()
                    .fg(tc!(text))
                    .add_modifier(Modifier::BOLD),
            ))
        } else {
            Line::from(vec![
                Span::raw(" > "),
                Span::styled(*part, Style::default().fg(tc!(primary))),
            ])
        }
    })
    .collect();

    let breadcrumb_line: Line = breadcrumb_parts.into_iter().flatten().collect();

    let block = Block::default()
        .borders(Borders::NONE)
        .border_style(Style::default().fg(tc!(border)));

    let paragraph = Paragraph::new(breadcrumb_line)
        .block(block)
        .style(Style::default().fg(tc!(text)));

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
            app.graphql.render_overlays(f, area);
        }
        crate::tui::tabs::Tab::OAuth => {
            app.oauth.render(f, area, insert_mode);
            app.oauth.render_overlays(f, area);
        }
        crate::tui::tabs::Tab::Cluster => {
            app.cluster.render(f, area, insert_mode);
            app.cluster.render_overlays(f, area);
        }
        crate::tui::tabs::Tab::Stress => {
            app.stress.render(f, area, insert_mode);
            app.stress.render_overlays(f, area);
        }
        crate::tui::tabs::Tab::Report => {
            app.report.render(f, area, insert_mode);
            app.report.render_overlays(f, area);
        }
        #[cfg(feature = "nse")]
        crate::tui::tabs::Tab::Nse => {
            app.nse.render(f, area, insert_mode);
            app.nse.render_overlays(f, area);
        }
        #[cfg(not(feature = "nse"))]
        crate::tui::tabs::Tab::Nse => {}
        #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
        crate::tui::tabs::Tab::Plugin => {
            app.plugin.render(f, area, insert_mode);
            app.plugin.render_overlays(f, area);
        }
        #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
        crate::tui::tabs::Tab::Plugin => {}
        crate::tui::tabs::Tab::Settings => {
            app.settings.render(f, area, insert_mode);
            app.settings.render_overlays(f, area);
        }
        crate::tui::tabs::Tab::History => {
            let h = app.history.lock();
            h.render(f, area, insert_mode);
            h.render_overlays(f, area);
        }
        crate::tui::tabs::Tab::Dashboard => {
            app.dashboard.render(f, area, insert_mode);
            app.dashboard.render_overlays(f, area);
        }
        #[cfg(feature = "advanced-hunting")]
        crate::tui::tabs::Tab::Hunt => {
            app.hunt.render(f, area, insert_mode);
            app.hunt.render_overlays(f, area);
        }
        #[cfg(not(feature = "advanced-hunting"))]
        crate::tui::tabs::Tab::Hunt => {}
        #[cfg(feature = "headless-browser")]
        crate::tui::tabs::Tab::Browser => {
            app.browser.render(f, area, insert_mode);
            app.browser.render_overlays(f, area);
        }
        #[cfg(not(feature = "headless-browser"))]
        crate::tui::tabs::Tab::Browser => {}
        #[cfg(feature = "compliance")]
        crate::tui::tabs::Tab::Compliance => {
            app.compliance.render(f, area, insert_mode);
            app.compliance.render_overlays(f, area);
        }
        #[cfg(not(feature = "compliance"))]
        crate::tui::tabs::Tab::Compliance => {}
        #[cfg(feature = "database")]
        crate::tui::tabs::Tab::Storage => {
            app.storage.render(f, area, insert_mode);
            app.storage.render_overlays(f, area);
        }
        #[cfg(not(feature = "database"))]
        crate::tui::tabs::Tab::Storage => {}
        #[cfg(feature = "external-integrations")]
        crate::tui::tabs::Tab::Integrations => {
            app.integrations.render(f, area, insert_mode);
            app.integrations.render_overlays(f, area);
        }
        #[cfg(not(feature = "external-integrations"))]
        crate::tui::tabs::Tab::Integrations => {}
        #[cfg(feature = "finding-workflow")]
        crate::tui::tabs::Tab::Workflow => {
            app.workflow.render(f, area, insert_mode);
            app.workflow.render_overlays(f, area);
        }
        #[cfg(not(feature = "finding-workflow"))]
        crate::tui::tabs::Tab::Workflow => {}
        #[cfg(feature = "vuln-management")]
        crate::tui::tabs::Tab::Vuln => {
            app.vuln.render(f, area, insert_mode);
            app.vuln.render_overlays(f, area);
        }
        #[cfg(not(feature = "vuln-management"))]
        crate::tui::tabs::Tab::Vuln => {}
    }
}

fn get_tab_status(state: &crate::tui::tabs::AppState) -> (String, ratatui::style::Color) {
    use crate::tui::tabs::AppState;
    match state {
        AppState::Idle => ("Ready - Press Enter to start".to_string(), tc!(status_idle)),
        AppState::Running => ("Running - Ctrl+C to stop".to_string(), tc!(status_running)),
        AppState::Completed => ("Completed".to_string(), tc!(success)),
        AppState::Error(e) => (e.to_string(), tc!(error)),
    }
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let (status_text, status_color) = match app.current_tab {
        crate::tui::tabs::Tab::Recon => get_tab_status(&app.recon.state),
        crate::tui::tabs::Tab::Load => get_tab_status(&app.load.state),
        crate::tui::tabs::Tab::ScanPorts => get_tab_status(&app.scan_ports.state),
        crate::tui::tabs::Tab::ScanEndpoints => get_tab_status(&app.scan_endpoints.state),
        crate::tui::tabs::Tab::Fingerprint => get_tab_status(&app.fingerprint.state),
        crate::tui::tabs::Tab::Fuzz => get_tab_status(&app.fuzz.state),
        crate::tui::tabs::Tab::Waf => get_tab_status(&app.waf.state),
        crate::tui::tabs::Tab::WafStress => get_tab_status(&app.waf_stress.state),
        crate::tui::tabs::Tab::Scan => get_tab_status(&app.scan.state),
        crate::tui::tabs::Tab::Resume => get_tab_status(&app.resume.state),
        crate::tui::tabs::Tab::Proxy => get_tab_status(&app.proxy.state),
        crate::tui::tabs::Tab::Packet => get_tab_status(&app.packet.state),
        crate::tui::tabs::Tab::Settings => (
            "Press 's' to save settings, 'r' to reset".to_string(),
            tc!(status_idle)
        ),
        crate::tui::tabs::Tab::History => (
            "↑↓ Navigate | 'd' Delete | 'r' Clear all".to_string(),
            tc!(status_idle)
        ),
        crate::tui::tabs::Tab::Dashboard => (
            "Dashboard - View scan results overview".to_string(),
            tc!(status_idle)
        ),
        crate::tui::tabs::Tab::GraphQl => get_tab_status(&app.graphql.state),
        crate::tui::tabs::Tab::OAuth => get_tab_status(&app.oauth.state),
        crate::tui::tabs::Tab::Cluster => get_tab_status(&app.cluster.state),
        crate::tui::tabs::Tab::Stress => get_tab_status(&app.stress.state),
        crate::tui::tabs::Tab::Report => get_tab_status(&app.report.state),
        #[cfg(feature = "nse")]
        crate::tui::tabs::Tab::Nse => get_tab_status(&app.nse.state),
        #[cfg(not(feature = "nse"))]
        crate::tui::tabs::Tab::Nse => ("NSE not available".to_string(), tc!(status_idle)),
        #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
        crate::tui::tabs::Tab::Plugin => get_tab_status(&app.plugin.state),
        #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
        crate::tui::tabs::Tab::Plugin => ("Plugins not available".to_string(), tc!(status_idle)),
        #[cfg(feature = "advanced-hunting")]
        crate::tui::tabs::Tab::Hunt => get_tab_status(&app.hunt.state),
        #[cfg(not(feature = "advanced-hunting"))]
        crate::tui::tabs::Tab::Hunt => ("Hunting feature not enabled".to_string(), tc!(status_idle)),
        #[cfg(feature = "headless-browser")]
        crate::tui::tabs::Tab::Browser => get_tab_status(&app.browser.state),
        #[cfg(not(feature = "headless-browser"))]
        crate::tui::tabs::Tab::Browser => ("Browser feature not enabled".to_string(), tc!(status_idle)),
        #[cfg(feature = "compliance")]
        crate::tui::tabs::Tab::Compliance => get_tab_status(&app.compliance.state),
        #[cfg(not(feature = "compliance"))]
        crate::tui::tabs::Tab::Compliance => {
            ("Compliance feature not enabled".to_string(), tc!(status_idle))
        }
        #[cfg(feature = "database")]
        crate::tui::tabs::Tab::Storage => get_tab_status(&app.storage.state),
        #[cfg(not(feature = "database"))]
        crate::tui::tabs::Tab::Storage => ("Storage feature not enabled".to_string(), tc!(status_idle)),
        #[cfg(feature = "external-integrations")]
        crate::tui::tabs::Tab::Integrations => get_tab_status(&app.integrations.state),
        #[cfg(not(feature = "external-integrations"))]
        crate::tui::tabs::Tab::Integrations => {
            ("Integrations feature not enabled".to_string(), tc!(status_idle))
        }
        #[cfg(feature = "finding-workflow")]
        crate::tui::tabs::Tab::Workflow => get_tab_status(&app.workflow.state),
        #[cfg(not(feature = "finding-workflow"))]
        crate::tui::tabs::Tab::Workflow => {
            ("Workflow feature not enabled".to_string(), tc!(status_idle))
        }
        #[cfg(feature = "vuln-management")]
        crate::tui::tabs::Tab::Vuln => get_tab_status(&app.vuln.state),
        #[cfg(not(feature = "vuln-management"))]
        crate::tui::tabs::Tab::Vuln => ("Vuln management not enabled".to_string(), tc!(status_idle)),
    };

    let help_text = if app.is_help_visible() {
        " [Esc] Close | [Space] Help | [Enter] Confirm | [j/k] Nav | [w/b] Word | [gg/G] Top/Bot | [n/p] Tab | [h/l] Input | [q] Quit ".to_string()
    } else {
        match app.mode {
            super::InputMode::Normal => {
                let mut help = if area.width < 80 {
                    format!(
                        " [n/p]Tab [j/k]Nav [Ctrl+F]Search [Ctrl+Z]{} [i]Insert [q]Quit",
                        if app.is_paused() { "[Paused]" } else { "" }
                    )
                } else {
                    format!(
                        " [n/p] Tab | [j/k] Nav | [/] Search | [Ctrl+F] Global | [Ctrl+Z]{} | [1-9] Jump | [Space] Help | [i] Insert | [q] Quit | [b] Bookmark | [Ctrl+T] Theme",
                        if app.is_paused() { " Resume" } else { " Pause" }
                    )
                };
                if let Some(palette) = app.get_command_palette() {
                    if palette.visible {
                        help.push_str(" [Ctrl+P] Close");
                    } else {
                        help.push_str(" [Ctrl+P] Palette");
                    }
                }
                if !app.get_bookmarked_tabs().is_empty() {
                    help.push_str(&format!(" [{}]", app.get_bookmarked_tabs().len()));
                }
                help
            }
            super::InputMode::Insert => {
                if area.width < 60 {
                    " [Esc] Normal | Type | [Ctrl+C] Cancel | [Ctrl+V] Paste".to_string()
                } else {
                    " [Esc] Normal | Type to input | [Ctrl+C] Cancel | [Ctrl+V] Paste".to_string()
                }
            }
        }
    };

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

    // Mode indicator (NORMAL/INSERT)
    let mode_text = match app.mode {
        super::InputMode::Normal => "NORMAL",
        super::InputMode::Insert => "INSERT",
    };
    let mode_color = match app.mode {
super::InputMode::Normal => tc!(mode_normal),

        super::InputMode::Insert => tc!(mode_insert),

    };
    let mode_indicator_widget = ratatui::widgets::Paragraph::new(format!(" {} ", mode_text)).style(
        Style::default()
            .fg(tc!(background))
            .bg(mode_color)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(mode_indicator_widget, chunks[0]);

    let status =
        ratatui::widgets::Paragraph::new(status_text).style(Style::default().fg(status_color));
    f.render_widget(status, chunks[1]);

    let help =
        ratatui::widgets::Paragraph::new(help_text).style(Style::default().fg(tc!(text_dim)));
    f.render_widget(help, chunks[2]);
}
