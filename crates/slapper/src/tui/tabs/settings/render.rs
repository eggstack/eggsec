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
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(20), Constraint::Min(0)])
            .split(area);

        let nav_area = chunks[0];
        let content_area = chunks[1];

        let nav_items = vec![
            ("HTTP Settings", SettingsSection::Http),
            ("Scan Settings", SettingsSection::Scan),
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
                    SettingsSection::Proxy => "Proxy Settings",
                    SettingsSection::Scope => "Scope Settings",
                    SettingsSection::Report => "Report Conversion",
                    SettingsSection::Schedule => "Schedule Management",
                    SettingsSection::Notifications => "Notification Settings",
                    SettingsSection::Theme => "Theme Settings",
                });
        let inner = content_block.inner(content_area);
        f.render_widget(content_block, content_area);

        match self.current_section {
            SettingsSection::Http => {
                let input_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Length(2),
                        Constraint::Length(2),
                    ])
                    .split(inner);

                for (i, field) in self.http_inputs.fields.iter().enumerate() {
                    field.render(f, input_chunks[i], insert_mode);
                }

                let fr = self.follow_redirects.clone();
                let mut fr = fr;
                fr.focused = self.http_inputs.is_focused();
                fr.render(f, input_chunks[3]);

                let vt = self.verify_tls.clone();
                let mut vt = vt;
                vt.focused = self.http_inputs.is_focused();
                vt.render(f, input_chunks[4]);
            }
            SettingsSection::Scan => {
                let input_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Length(2),
                    ])
                    .split(inner);

                for (i, field) in self.scan_inputs.fields.iter().enumerate() {
                    field.render(f, input_chunks[i], insert_mode);
                }

                let sm = self.stealth_mode.clone();
                sm.render(f, input_chunks[3]);
            }
            SettingsSection::Proxy => {
                let input_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Length(3),
                    ])
                    .split(inner);

                for (i, field) in self.proxy_inputs.fields.iter().enumerate() {
                    field.render(f, input_chunks[i], insert_mode);
                }

                let sel = self.proxy_rotation_selector.clone();
                sel.render(f, input_chunks[2]);
            }
            SettingsSection::Scope => {
                let input_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Length(3)])
                    .split(inner);

                for (i, field) in self.scope_inputs.fields.iter().enumerate() {
                    field.render(f, input_chunks[i], insert_mode);
                }
            }
            SettingsSection::Report => {
                let input_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Length(3),
                    ])
                    .split(inner);

                for (i, field) in self.report_inputs.fields.iter().enumerate() {
                    field.render(f, input_chunks[i], insert_mode);
                }
            }
            SettingsSection::Schedule => {
                let input_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Length(3),
                    ])
                    .split(inner);

                for (i, field) in self.schedule_inputs.fields.iter().enumerate() {
                    field.render(f, input_chunks[i], insert_mode);
                }
            }
            SettingsSection::Notifications => {
                let input_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Length(2),
                        Constraint::Length(2),
                        Constraint::Length(3),
                    ])
                    .split(inner);

                for (i, field) in self.notify_inputs.fields.iter().enumerate() {
                    field.render(f, input_chunks[i], insert_mode);
                }
                self.notify_on_complete.render(f, input_chunks[4]);
                self.notify_on_findings.render(f, input_chunks[5]);

                let mut severity_sel = self.severity_selector.clone();
                severity_sel.focused = self.is_input_focused();
                severity_sel.render(f, input_chunks[6]);
            }
            SettingsSection::Theme => {
                let input_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Length(3),
                        Constraint::Length(3),
                    ])
                    .split(inner);

                let dm = self.dark_mode.clone();
                let mut dm = dm;
                dm.focused = self.is_input_focused();
                dm.render(f, input_chunks[0]);

                let mut accent_sel = self.accent_color.clone();
                accent_sel.focused = self.is_input_focused();
                accent_sel.render(f, input_chunks[1]);

                let theme_hint = Paragraph::new("Use Ctrl+T to toggle theme instantly");
                f.render_widget(theme_hint, input_chunks[2]);
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
