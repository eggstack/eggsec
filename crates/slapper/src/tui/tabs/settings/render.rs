use super::SettingsSection;
use crate::tc;
use crate::tui::tabs::TabRender;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

impl TabRender for super::SettingsTab {
    fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        if let Some(ref err) = self.error {
            let error_text = Paragraph::new(format!("Error: {}", err.message()))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Settings - Error"),
                )
                .style(Style::default().fg(tc!(error)));
            f.render_widget(error_text, area);
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(20), Constraint::Min(0)])
            .split(area);

        let nav_area = chunks[0];
        let content_area = chunks[1];

        let nav_items = vec![
            ("HTTP Settings", SettingsSection::Http),
            ("Scan Settings", SettingsSection::Scan),
            ("Session Settings", SettingsSection::Session),
            ("Proxy Settings", SettingsSection::Proxy),
            ("Scope Settings", SettingsSection::Scope),
            ("Report", SettingsSection::Report),
            ("Schedule", SettingsSection::Schedule),
            ("Notifications", SettingsSection::Notifications),
            ("Theme", SettingsSection::Theme),
        ];

        let mut nav_lines = Vec::new();
        for (label, section) in &nav_items {
            let style = if *section == self.current_section {
                Style::default().fg(tc!(selected_text)).bg(tc!(selected))
            } else {
                Style::default().fg(tc!(border))
            };
            nav_lines.push(Line::from(Span::styled(format!("  {}", label), style)));
        }

        let nav = Paragraph::new(nav_lines)
            .block(Block::default().borders(Borders::ALL).title("Settings"));
        f.render_widget(nav, nav_area);

        let content_block =
            Block::default()
                .borders(Borders::ALL)
                .title(match self.current_section {
                    SettingsSection::Http => "HTTP Settings",
                    SettingsSection::Scan => "Scan Settings",
                    SettingsSection::Session => "Session Settings",
                    SettingsSection::Proxy => "Proxy Settings",
                    SettingsSection::Scope => "Scope Settings",
                    SettingsSection::Report => "Report Conversion",
                    SettingsSection::Schedule => "Schedule Management",
                    SettingsSection::Notifications => "Notification Settings",
                    SettingsSection::Theme => "Theme Settings",
                });
        let inner = content_block.inner(content_area);
        f.render_widget(content_block, content_area);

        use crate::tui::components::FormBuilder;

        match self.current_section {
            SettingsSection::Http => {
                let mut builder = FormBuilder::new("HTTP Settings").row_height(3);
                for field in &self.http_inputs.fields {
                    builder = builder.add_input(field.clone());
                }
                builder = builder.add_checkbox(self.follow_redirects.clone());
                builder = builder.add_checkbox(self.verify_tls.clone());
                builder.render(f, inner, insert_mode);
            }
            SettingsSection::Scan => {
                let mut builder = FormBuilder::new("Scan Settings").row_height(3);
                for field in &self.scan_inputs.fields {
                    builder = builder.add_input(field.clone());
                }
                builder = builder.add_checkbox(self.stealth_mode.clone());
                builder.render(f, inner, insert_mode);
            }
            SettingsSection::Session => {
                let mut builder = FormBuilder::new("Session Settings").row_height(3);
                for field in &self.session_inputs.fields {
                    builder = builder.add_input(field.clone());
                }
                builder.render(f, inner, insert_mode);
            }
            SettingsSection::Proxy => {
                let mut builder = FormBuilder::new("Proxy Settings").row_height(3);
                for field in &self.proxy_inputs.fields {
                    builder = builder.add_input(field.clone());
                }
                builder = builder.add_selector(self.proxy_rotation_selector.clone());
                builder.render(f, inner, insert_mode);
            }
            SettingsSection::Scope => {
                let mut builder = FormBuilder::new("Scope Settings").row_height(3);
                for field in &self.scope_inputs.fields {
                    builder = builder.add_input(field.clone());
                }
                builder.render(f, inner, insert_mode);
            }
            SettingsSection::Report => {
                let mut builder = FormBuilder::new("Report Conversion").row_height(3);
                for field in &self.report_inputs.fields {
                    builder = builder.add_input(field.clone());
                }
                builder.render(f, inner, insert_mode);
            }
            SettingsSection::Schedule => {
                let mut builder = FormBuilder::new("Schedule Management").row_height(3);
                for field in &self.schedule_inputs.fields {
                    builder = builder.add_input(field.clone());
                }
                builder.render(f, inner, insert_mode);
            }
            SettingsSection::Notifications => {
                let mut builder = FormBuilder::new("Notification Settings").row_height(3);
                for field in &self.notify_inputs.fields {
                    builder = builder.add_input(field.clone());
                }
                builder = builder.add_checkbox(self.notify_on_complete.clone());
                builder = builder.add_checkbox(self.notify_on_findings.clone());
                builder = builder.add_selector(self.severity_selector.clone());
                builder.render(f, inner, insert_mode);
            }
            SettingsSection::Theme => {
                let mut builder = FormBuilder::new("Theme Settings").row_height(3);
                builder = builder.add_checkbox(self.dark_mode.clone());
                builder = builder.add_selector(self.accent_color.clone());
                builder.render(f, inner, insert_mode);

                let theme_hint = Paragraph::new("Use Ctrl+T to toggle theme instantly");
                let hint_area = Rect {
                    y: inner.y + 6,
                    height: 1,
                    ..inner
                };
                f.render_widget(theme_hint, hint_area);
            }
        }

        if !self.status_message.is_empty() {
            let status = Paragraph::new(self.status_message.as_str())
                .style(Style::default().fg(tc!(success)));
            let status_area = Rect {
                x: inner.x,
                y: inner.y + inner.height.saturating_sub(2),
                width: inner.width,
                height: 1,
            };
            f.render_widget(status, status_area);
        }
    }
}
