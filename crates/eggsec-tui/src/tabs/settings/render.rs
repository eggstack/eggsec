use super::SettingsSection;
use crate::tabs::TabRender;
use crate::tc;
use crate::theme::ThemeMode;
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

        let nav_area = chunks.first().copied().unwrap_or(area);
        let content_area = chunks.get(1).copied().unwrap_or(area);

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

        use crate::components::FormBuilder;

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
                // --- Theme details pane ---
                let current_name = self
                    .theme_selector
                    .selected_label()
                    .unwrap_or("Unknown");
                let current_id = self
                    .theme_selector
                    .selected_value()
                    .unwrap_or("unknown");

                // Determine source and mode from cached theme info.
                let (source_label, mode_label) = self
                    .theme_info_cache
                    .iter()
                    .find(|info| info.id == current_id)
                    .map(|info| {
                        let src = match info.source {
                            crate::theme::manager::ThemeSource::BuiltIn => "Built-in",
                            crate::theme::manager::ThemeSource::Packaged => "Packaged",
                            crate::theme::manager::ThemeSource::Custom => "Custom",
                        };
                        let mode = match info.mode {
                            ThemeMode::Dark => "Dark",
                            ThemeMode::Light => "Light",
                        };
                        (src, mode)
                    })
                    .unwrap_or(("Built-in", "Dark"));

                // Contrast validation (from actual validation, not just invalid count).
                let contrast_warnings = if !self.theme_contrast_warnings.is_empty() {
                    format!("{} warning(s)", self.theme_contrast_warnings.len())
                } else if self.theme_invalid_count > 0 {
                    format!("{} invalid", self.theme_invalid_count)
                } else {
                    "OK".to_string()
                };

                let theme_count = self.theme_info_cache.len();
                let invalid = self.theme_invalid_count;
                let dir = &self.theme_dir_path;

                // Build metadata lines above the selector.
                let mut meta_lines = Vec::new();

                // Line 1: display name and source/mode badge.
                meta_lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {} ", current_name),
                        Style::default()
                            .fg(tc!(text))
                            .add_modifier(ratatui::style::Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{} · {}", source_label, mode_label),
                        Style::default().fg(tc!(text_dim)),
                    ),
                ]));

                // Line 2: theme count, invalid count, contrast, dir.
                let mut stats_spans = vec![
                    Span::styled(
                        format!("  {} themes", theme_count),
                        Style::default().fg(tc!(text_dim)),
                    ),
                ];
                if invalid > 0 {
                    stats_spans.push(Span::styled(
                        format!("  {} invalid", invalid),
                        Style::default().fg(tc!(warning)),
                    ));
                }
                stats_spans.push(Span::styled(
                    format!("  Contrast: {}", contrast_warnings),
                    Style::default().fg(if contrast_warnings == "OK" {
                        tc!(safe)
                    } else {
                        tc!(warning)
                    }),
                ));
                meta_lines.push(Line::from(stats_spans));

                // Line 3: theme directory path.
                meta_lines.push(Line::from(Span::styled(
                    format!("  Dir: {}", dir),
                    Style::default().fg(tc!(text_dim)),
                )));

                let meta_height = meta_lines.len() as u16;
                let meta = Paragraph::new(meta_lines);
                let meta_area = Rect {
                    x: inner.x,
                    y: inner.y,
                    width: inner.width,
                    height: meta_height,
                };
                f.render_widget(meta, meta_area);

                // Render the selector below the metadata.
                let selector_y = inner.y + meta_height;
                let selector_area = Rect {
                    y: selector_y,
                    height: 3,
                    ..inner
                };
                self.theme_selector.render(f, selector_area);

                // --- Preview row below the selector ---
                let preview_y = selector_y + 3;
                let preview_height: u16 = 3;

                if preview_y + preview_height <= inner.y + inner.height {
                    let preview_area = Rect {
                        y: preview_y,
                        height: preview_height,
                        ..inner
                    };

                    let mut preview_lines = Vec::new();

                    // Line 1: Normal, Selected, Success, Warning, Error, Info text samples.
                    preview_lines.push(Line::from(vec![
                        Span::styled("  Normal ", Style::default().fg(tc!(text))),
                        Span::styled("Selected ", Style::default().fg(tc!(selected_text)).bg(tc!(selected))),
                        Span::styled("Success ", Style::default().fg(tc!(success))),
                        Span::styled("Warning ", Style::default().fg(tc!(warning))),
                        Span::styled("Error ", Style::default().fg(tc!(error))),
                        Span::styled("Info", Style::default().fg(tc!(info))),
                    ]));

                    // Line 2: Safe, Danger, Muted, Policy Required, Policy Denied.
                    preview_lines.push(Line::from(vec![
                        Span::styled("  Safe ", Style::default().fg(tc!(safe))),
                        Span::styled("Danger ", Style::default().fg(tc!(danger))),
                        Span::styled("Muted ", Style::default().fg(tc!(muted))),
                        Span::styled("Policy Required ", Style::default().fg(tc!(policy_required))),
                        Span::styled("Policy Denied", Style::default().fg(tc!(policy_denied))),
                    ]));

                    // Line 3: Active task, Paused task, Scope match, Scope miss.
                    preview_lines.push(Line::from(vec![
                        Span::styled("  Active ", Style::default().fg(tc!(active_task))),
                        Span::styled("Paused ", Style::default().fg(tc!(paused_task))),
                        Span::styled("Scope Match ", Style::default().fg(tc!(scope_match))),
                        Span::styled("Scope Miss", Style::default().fg(tc!(scope_miss))),
                    ]));

                    let preview_block = Block::default()
                        .borders(Borders::TOP)
                        .title("Preview");
                    let preview_inner = preview_block.inner(preview_area);
                    f.render_widget(preview_block, preview_area);
                    f.render_widget(Paragraph::new(preview_lines), preview_inner);
                }

                // Hint text below the preview.
                let hint_y = preview_y + preview_height;
                if hint_y < inner.y + inner.height {
                    let hint = Paragraph::new(
                        "Press [r] to reload themes   [Ctrl+T] to cycle",
                    )
                    .style(Style::default().fg(tc!(text_dim)));
                    let hint_area = Rect {
                        y: hint_y,
                        height: 1,
                        ..inner
                    };
                    f.render_widget(hint, hint_area);
                }
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

        // Persistent footer hint: tells the user about the save shortcut and
        // section navigation. Rendered on the very last line of the section
        // (1 line above the status bar) so it is always visible.
        if inner.height >= 2 {
            let hint = Paragraph::new(
                "[s] Save   [Esc] Back   [Tab] Next field   [\u{2191}\u{2193}] Section",
            )
            .style(Style::default().fg(tc!(text_dim)));
            let hint_area = Rect {
                x: inner.x,
                y: inner.y + inner.height.saturating_sub(1),
                width: inner.width,
                height: 1,
            };
            f.render_widget(hint, hint_area);
        }
    }
}
