use super::SettingsSection;
use crate::tabs::TabRender;
use crate::tc;
use crate::theme::palette::{ThemeColors, ThemeMode};
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
                        .border_style(Style::default().fg(tc!(error)))
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
            .block(Block::default().borders(Borders::ALL).title("Settings").border_style(Style::default().fg(tc!(border))));
        f.render_widget(nav, nav_area);

        let content_block =
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(tc!(border)))
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

        // Reserve footer (1 row) and optional status (1 row) so they never
        // overlap with section content.
        let status_rows: u16 = if self.status_message.is_empty() { 0 } else { 1 };
        let footer_rows: u16 = if inner.height >= 2 { 1 } else { 0 };
        let reserved = status_rows + footer_rows;
        let body_height = inner.height.saturating_sub(reserved);
        let body = Rect {
            height: body_height,
            ..inner
        };
        let status_area = if status_rows > 0 {
            Some(Rect {
                y: inner.y + body_height,
                height: 1,
                ..inner
            })
        } else {
            None
        };
        let footer_area = if footer_rows > 0 {
            Some(Rect {
                y: inner.y + body_height + status_rows,
                height: 1,
                ..inner
            })
        } else {
            None
        };

        use crate::components::FormBuilder;

        match self.current_section {
            SettingsSection::Http => {
                let mut builder = FormBuilder::new("HTTP Settings").row_height(3);
                for field in &self.http_inputs.fields {
                    builder = builder.add_input(field.clone());
                }
                builder = builder.add_checkbox(self.follow_redirects.clone());
                builder = builder.add_checkbox(self.verify_tls.clone());
                builder.render(f, body, insert_mode);
            }
            SettingsSection::Scan => {
                let mut builder = FormBuilder::new("Scan Settings").row_height(3);
                for field in &self.scan_inputs.fields {
                    builder = builder.add_input(field.clone());
                }
                builder = builder.add_checkbox(self.stealth_mode.clone());
                builder.render(f, body, insert_mode);
            }
            SettingsSection::Session => {
                let mut builder = FormBuilder::new("Session Settings").row_height(3);
                for field in &self.session_inputs.fields {
                    builder = builder.add_input(field.clone());
                }
                builder.render(f, body, insert_mode);
            }
            SettingsSection::Proxy => {
                let mut builder = FormBuilder::new("Proxy Settings").row_height(3);
                for field in &self.proxy_inputs.fields {
                    builder = builder.add_input(field.clone());
                }
                builder = builder.add_selector(self.proxy_rotation_selector.clone());
                builder.render(f, body, insert_mode);
                for dropdown in builder.collect_dropdowns(body, area.height) {
                    dropdown.render(f);
                }
            }
            SettingsSection::Scope => {
                let mut builder = FormBuilder::new("Scope Settings").row_height(3);
                for field in &self.scope_inputs.fields {
                    builder = builder.add_input(field.clone());
                }
                builder.render(f, body, insert_mode);
            }
            SettingsSection::Report => {
                let mut builder = FormBuilder::new("Report Conversion").row_height(3);
                for field in &self.report_inputs.fields {
                    builder = builder.add_input(field.clone());
                }
                builder.render(f, body, insert_mode);
            }
            SettingsSection::Schedule => {
                let mut builder = FormBuilder::new("Schedule Management").row_height(3);
                for field in &self.schedule_inputs.fields {
                    builder = builder.add_input(field.clone());
                }
                builder.render(f, body, insert_mode);
            }
            SettingsSection::Notifications => {
                let mut builder = FormBuilder::new("Notification Settings").row_height(3);
                for field in &self.notify_inputs.fields {
                    builder = builder.add_input(field.clone());
                }
                builder = builder.add_checkbox(self.notify_on_complete.clone());
                builder = builder.add_checkbox(self.notify_on_findings.clone());
                builder = builder.add_selector(self.severity_selector.clone());
                builder.render(f, body, insert_mode);
                for dropdown in builder.collect_dropdowns(body, area.height) {
                    dropdown.render(f);
                }
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

                // Determine source, mode, and status from cached theme info.
                let (source_label, mode_label, status_label) = self
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
                        let status = match info.status {
                            crate::theme::manager::ThemeLoadStatus::Loaded => "",
                            crate::theme::manager::ThemeLoadStatus::FallbackAdjusted => " (adjusted)",
                            crate::theme::manager::ThemeLoadStatus::Invalid(_) => " (invalid)",
                            crate::theme::manager::ThemeLoadStatus::Missing => " (missing)",
                        };
                        (src, mode, status)
                    })
                    .unwrap_or(("Built-in", "Dark", ""));

                // Contrast validation (per-theme from cache).
                let selected_contrast = self
                    .theme_contrast_cache
                    .get(current_id)
                    .map(|w| w.len())
                    .unwrap_or(0);
                let contrast_warnings = if selected_contrast > 0 {
                    format!("{} warning(s)", selected_contrast)
                } else if self.theme_invalid_count > 0 {
                    format!("{} invalid", self.theme_invalid_count)
                } else {
                    "OK".to_string()
                };

                let theme_count = self.theme_info_cache.len();
                let invalid = self.theme_invalid_count;
                let dir = &self.theme_dir_path;

                // Count fallback-adjusted themes.
                let fallback_count = self
                    .theme_info_cache
                    .iter()
                    .filter(|i| i.status == crate::theme::manager::ThemeLoadStatus::FallbackAdjusted)
                    .count();

                // Determine applied theme name for Selected vs Applied display.
                let selected_id = current_id;
                let applied_matches = self
                    .applied_theme_id
                    .as_deref()
                    .map(|aid| aid == selected_id)
                    .unwrap_or(true);
                let show_applied = self.theme_selector.is_open() || !applied_matches;

                // Build metadata lines above the selector.
                let mut meta_lines = Vec::new();

                // Line 1: Selected/Applied label, display name, source/mode badge, and status.
                let label = if show_applied {
                    if applied_matches {
                        "  Selected/Applied "
                    } else {
                        "  Selected "
                    }
                } else {
                    "  "
                };
                meta_lines.push(Line::from(vec![
                    Span::styled(
                        format!("{}{} ", label, current_name),
                        Style::default()
                            .fg(tc!(text))
                            .add_modifier(ratatui::style::Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{} · {}{}", source_label, mode_label, status_label),
                        Style::default().fg(tc!(text_dim)),
                    ),
                ]));

                // Line 2: theme counts (loaded, invalid, fallback), contrast, dir.
                let loaded_count = theme_count.saturating_sub(invalid);
                let mut stats_spans = vec![
                    Span::styled(
                        format!("  {} loaded", loaded_count),
                        Style::default().fg(tc!(text_dim)),
                    ),
                ];
                if invalid > 0 {
                    stats_spans.push(Span::styled(
                        format!("  {} invalid", invalid),
                        Style::default().fg(tc!(warning)),
                    ));
                }
                if fallback_count > 0 {
                    stats_spans.push(Span::styled(
                        format!("  {} adjusted", fallback_count),
                        Style::default().fg(tc!(info)),
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

                // Line 3: Applied theme name when selected differs from applied.
                if !applied_matches {
                    if let Some(ref applied_id) = self.applied_theme_id {
                        let applied_label = self
                            .theme_info_cache
                            .iter()
                            .find(|info| info.id == *applied_id)
                            .map(|info| info.display_name.as_str())
                            .unwrap_or("Unknown");
                        meta_lines.push(Line::from(Span::styled(
                            format!("  Applied: {}", applied_label),
                            Style::default().fg(tc!(text_dim)),
                        )));
                    }
                }

                // Line 4: theme directory path.
                meta_lines.push(Line::from(Span::styled(
                    format!("  Dir: {}", dir),
                    Style::default().fg(tc!(text_dim)),
                )));

                let meta_height = meta_lines.len() as u16;
                let meta = Paragraph::new(meta_lines);
                let meta_area = Rect {
                    x: body.x,
                    y: body.y,
                    width: body.width,
                    height: meta_height,
                };
                f.render_widget(meta, meta_area);

                // Render the selector below the metadata.
                let selector_y = body.y + meta_height;
                let selector_area = Rect {
                    y: selector_y,
                    height: 3,
                    ..body
                };
                self.theme_selector.render(f, selector_area);

                // --- Preview row below the selector ---
                let preview_y = selector_y + 3;
                let preview_height: u16 = 3;

                if preview_y + preview_height <= body.y + body.height {
                    let preview_area = Rect {
                        y: preview_y,
                        height: preview_height,
                        ..body
                    };

                    let mut preview_lines = Vec::new();

                    // Use resolved theme colors for preview when available,
                    // fall back to tc!() thread-local theme.
                    let c = self.resolved_theme_colors.as_ref();
                    let fg = |get: fn(&ThemeColors) -> ratatui::style::Color| {
                        c.map(get).unwrap_or_else(|| tc!(text))
                    };

                    // Line 1: Normal, Selected, Success, Warning, Error, Info text samples.
                    preview_lines.push(Line::from(vec![
                        Span::styled("  Normal ", Style::default().fg(fg(|c| c.text))),
                        Span::styled("Selected ", Style::default().fg(fg(|c| c.selected_text)).bg(fg(|c| c.selected))),
                        Span::styled("Success ", Style::default().fg(fg(|c| c.success))),
                        Span::styled("Warning ", Style::default().fg(fg(|c| c.warning))),
                        Span::styled("Error ", Style::default().fg(fg(|c| c.error))),
                        Span::styled("Info", Style::default().fg(fg(|c| c.info))),
                    ]));

                    // Line 2: Safe, Danger, Muted, Policy Required, Policy Denied.
                    preview_lines.push(Line::from(vec![
                        Span::styled("  Safe ", Style::default().fg(fg(|c| c.safe))),
                        Span::styled("Danger ", Style::default().fg(fg(|c| c.danger))),
                        Span::styled("Muted ", Style::default().fg(fg(|c| c.muted))),
                        Span::styled("Policy Required ", Style::default().fg(fg(|c| c.policy_required))),
                        Span::styled("Policy Denied", Style::default().fg(fg(|c| c.policy_denied))),
                    ]));

                    // Line 3: Active task, Paused task, Scope match, Scope miss.
                    preview_lines.push(Line::from(vec![
                        Span::styled("  Active ", Style::default().fg(fg(|c| c.active_task))),
                        Span::styled("Paused ", Style::default().fg(fg(|c| c.paused_task))),
                        Span::styled("Scope Match ", Style::default().fg(fg(|c| c.scope_match))),
                        Span::styled("Scope Miss", Style::default().fg(fg(|c| c.scope_miss))),
                    ]));

                    let preview_block = Block::default()
                        .borders(Borders::TOP)
                        .title("Preview");
                    let preview_inner = preview_block.inner(preview_area);
                    f.render_widget(preview_block, preview_area);
                    f.render_widget(Paragraph::new(preview_lines), preview_inner);
                }

                // Hint text below the preview — context-aware for selector state.
                let hint_y = preview_y + preview_height;
                if hint_y < body.y + body.height {
                    let hint_text = if self.theme_selector.is_open() {
                        "Enter:apply  Esc:cancel  Up/Down or j/k:preview"
                    } else {
                        "Enter:themes  r:reload  Ctrl+T:cycle"
                    };
                    let hint = Paragraph::new(hint_text)
                        .style(Style::default().fg(tc!(text_dim)));
                    let hint_area = Rect {
                        y: hint_y,
                        height: 1,
                        ..body
                    };
                    f.render_widget(hint, hint_area);
                }

                // Render theme selector dropdown overlay last so it overlays other content.
                if let Some(dropdown) = self.theme_selector.dropdown_info(selector_area, area.height) {
                    dropdown.render(f);
                }
            }
        }

        // Status message in reserved area.
        if let Some(status_area) = status_area {
            let status_style = if self.status_message.contains("error")
                || self.status_message.contains("Error")
                || self.status_message.contains("failed")
            {
                Style::default().fg(tc!(error))
            } else if self.status_message.contains("warning")
                || self.status_message.contains("Warning")
            {
                Style::default().fg(tc!(warning))
            } else {
                Style::default().fg(tc!(success))
            };
            let status = Paragraph::new(self.status_message.as_str()).style(status_style);
            f.render_widget(status, status_area);
        }

        // Persistent footer hint in reserved area.
        if let Some(footer_area) = footer_area {
            let hint = Paragraph::new(
                "[s] Save   [Esc] Back   [Tab] Next field   [\u{2191}\u{2193}] Section",
            )
            .style(Style::default().fg(tc!(text_dim)));
            f.render_widget(hint, footer_area);
        }
    }
}
