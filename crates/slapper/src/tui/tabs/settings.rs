use crate::config::{
    HttpConfig, NotificationConfig, OutputConfig, ScanConfig, ScheduledScan, SlapperConfig,
};
use crate::tui::components::{Checkbox, InputField, InputGroup, Selector, SelectorItem};
use crate::tui::tabs::{AppState, TabInput, TabRender, TabState};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct SettingsTab {
    pub http_inputs: InputGroup,
    pub scan_inputs: InputGroup,
    pub proxy_inputs: InputGroup,
    pub scope_inputs: InputGroup,
    pub report_inputs: InputGroup,
    pub schedule_inputs: InputGroup,
    pub notify_inputs: InputGroup,
    pub follow_redirects: Checkbox,
    pub verify_tls: Checkbox,
    pub stealth_mode: Checkbox,
    pub notify_on_complete: Checkbox,
    pub notify_on_findings: Checkbox,
    pub proxy_rotation_selector: Selector,
    pub severity_selector: Selector,
    pub current_section: SettingsSection,
    pub config: Option<SlapperConfig>,
    pub config_path: Option<String>,
    pub status_message: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SettingsSection {
    Http,
    Scan,
    Proxy,
    Scope,
    Report,
    Schedule,
    Notifications,
}

impl SettingsTab {
    pub fn new() -> Self {
        let http_inputs = InputGroup::new()
            .add(InputField::new("Timeout (s)").with_value("30"))
            .add(InputField::new("Max Retries").with_value("3"))
            .add(InputField::new("Max Redirects").with_value("10"));

        let scan_inputs = InputGroup::new()
            .add(InputField::new("Default Concurrency").with_value("50"))
            .add(InputField::new("Rate Limit (req/s)").with_value("0"))
            .add(InputField::new("Port Timeout (s)").with_value("300"));

        let proxy_inputs = InputGroup::new()
            .add(InputField::new("Proxy URL"))
            .add(InputField::new("Proxy Auth"));

        let scope_inputs = InputGroup::new()
            .add(InputField::new("Allowed Targets (comma-sep)"))
            .add(InputField::new("Excluded Targets (comma-sep)"));

        let report_inputs = InputGroup::new()
            .add(InputField::new("Input File"))
            .add(InputField::new("Output File"))
            .add(InputField::new("Format (json/csv/html/sarif/junit)").with_value("html"))
            .add(InputField::new("Export Directory").with_value("./exports"));

        let schedule_inputs = InputGroup::new()
            .add(InputField::new("Cron Expression (e.g., 0 */6 * * *)"))
            .add(InputField::new("Target URL"))
            .add(InputField::new("Scan Type").with_value("quick"))
            .add(InputField::new("Output File (optional)"));

        let notify_inputs = InputGroup::new()
            .add(InputField::new("Slack Webhook URL"))
            .add(InputField::new("Discord Webhook URL"))
            .add(InputField::new("Teams Webhook URL"))
            .add(InputField::new("Custom Webhook URL"));

        let proxy_rotation_selector = Selector::new("Proxy Rotation").items(vec![
            SelectorItem::new("None", "none"),
            SelectorItem::new("Round Robin", "round-robin"),
            SelectorItem::new("Random", "random"),
            SelectorItem::new("Least Connections", "least-conn"),
        ]);

        let severity_selector = Selector::new("Min Severity").items(vec![
            SelectorItem::new("Info", "info"),
            SelectorItem::new("Low", "low"),
            SelectorItem::new("Medium", "medium"),
            SelectorItem::new("High", "high"),
            SelectorItem::new("Critical", "critical"),
        ]);

        Self {
            http_inputs,
            scan_inputs,
            proxy_inputs,
            scope_inputs,
            report_inputs,
            schedule_inputs,
            notify_inputs,
            follow_redirects: Checkbox::new("Follow Redirects").checked(true),
            verify_tls: Checkbox::new("Verify TLS").checked(true),
            stealth_mode: Checkbox::new("Stealth Mode").checked(false),
            notify_on_complete: Checkbox::new("Notify on Complete").checked(false),
            notify_on_findings: Checkbox::new("Notify on Findings").checked(true),
            proxy_rotation_selector,
            severity_selector,
            current_section: SettingsSection::Http,
            config: None,
            config_path: Some("slapper.toml".to_string()),
            status_message: String::new(),
        }
    }

    pub fn set_config_path(&mut self, path: String) {
        self.config_path = Some(path);
    }

    pub fn load_config(&mut self, config: &SlapperConfig) {
        self.http_inputs.fields[0].value = config.http.timeout_secs.to_string();
        self.http_inputs.fields[1].value = config.http.max_retries.to_string();
        self.http_inputs.fields[2].value = config.http.max_redirects.to_string();
        self.follow_redirects.checked = config.http.follow_redirects;
        self.verify_tls.checked = config.http.verify_tls;

        self.scan_inputs.fields[0].value = config.scan.default_concurrency.to_string();
        self.stealth_mode.checked = config.scan.stealth_mode;

        if let Some(ref proxy_url) = config.http.proxy {
            self.proxy_inputs.fields[0].value = proxy_url.clone();
        }

        if let Some(ref export_dir) = config.paths.export_dir {
            self.report_inputs.fields[3].value = export_dir.clone();
        }

        self.config = Some(config.clone());
    }

    pub fn to_config(&self) -> SlapperConfig {
        let timeout_secs = self.http_inputs.fields[0].value.parse().unwrap_or(30);
        let max_retries = self.http_inputs.fields[1].value.parse().unwrap_or(3);
        let max_redirects = self.http_inputs.fields[2].value.parse().unwrap_or(10);
        let default_concurrency = self.scan_inputs.fields[0].value.parse().unwrap_or(50);

        SlapperConfig {
            http: HttpConfig {
                timeout_secs,
                max_retries,
                follow_redirects: self.follow_redirects.checked,
                verify_tls: self.verify_tls.checked,
                max_redirects,
                proxy: if self.proxy_inputs.fields[0].value.is_empty() {
                    None
                } else {
                    Some(self.proxy_inputs.fields[0].value.clone())
                },
                proxy_auth: if self.proxy_inputs.fields[1].value.is_empty() {
                    None
                } else {
                    Some(crate::types::SensitiveString::new(
                        self.proxy_inputs.fields[1].value.clone(),
                    ))
                },
                default_headers: std::collections::HashMap::new(),
                default_user_agent: None,
                retry_delay_ms: 100,
            },
            scan: ScanConfig {
                default_concurrency,
                rate_limit_per_second: self.scan_inputs.fields[1].value.parse().ok(),
                stealth_mode: self.stealth_mode.checked,
                jitter_ms: None,
                exclude_ports: Vec::new(),
                exclude_hosts: Vec::new(),
                port_timeout_secs: self.scan_inputs.fields[2].value.parse().unwrap_or(300),
                save_session: false,
                session_dir: None,
            },
            output: OutputConfig::default(),
            notifications: NotificationConfig::default(),
            paths: crate::config::PathsConfig {
                custom_payloads_dir: None,
                plugins_dir: None,
                wordlists_dir: None,
                export_dir: if self.report_inputs.fields[3].value.is_empty()
                    || self.report_inputs.fields[3].value == "./exports"
                {
                    None
                } else {
                    Some(self.report_inputs.fields[3].value.clone())
                },
            },
            profiles: std::collections::HashMap::new(),
            recon: crate::config::ReconConfig::default(),
            schedule: Vec::new(),
            remote: crate::config::RemoteConfig::default(),
            proxies: Vec::new(),
            ai: None,
        }
    }

    pub fn save_config(&mut self) {
        let config = self.to_config();

        let toml = toml::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize config: {}", e));

        match toml {
            Ok(content) => {
                let config_path = self.config_path.as_deref().unwrap_or("slapper.toml");
                if let Err(e) = std::fs::write(config_path, &content) {
                    self.status_message = format!("Error saving config: {}", e);
                } else {
                    self.status_message = format!("Configuration saved to {}", config_path);
                }
            }
            Err(e) => {
                self.status_message = format!("Error serializing config: {}", e);
            }
        }

        self.config = Some(config);
    }

    pub fn reset(&mut self) {
        self.http_inputs.fields[0].value = "30".to_string();
        self.http_inputs.fields[1].value = "3".to_string();
        self.http_inputs.fields[2].value = "10".to_string();
        self.scan_inputs.fields[0].value = "50".to_string();
        self.scan_inputs.fields[1].value = "0".to_string();
        self.scan_inputs.fields[2].value = "300".to_string();
        self.proxy_inputs.fields[0].value.clear();
        self.proxy_inputs.fields[1].value.clear();
        self.scope_inputs.fields[0].value.clear();
        self.scope_inputs.fields[1].value.clear();
        self.follow_redirects.checked = true;
        self.verify_tls.checked = true;
        self.stealth_mode.checked = false;
        self.status_message = "Settings reset to defaults".to_string();
    }

    pub fn input_file(&self) -> &str {
        self.report_inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn output_file(&self) -> &str {
        self.report_inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn format(&self) -> &str {
        self.report_inputs
            .fields
            .get(2)
            .map(|f| f.value.as_str())
            .unwrap_or("html")
    }

    pub fn convert_report(&mut self) -> Result<String, String> {
        let input = self.input_file();
        if input.is_empty() {
            return Err("Input file is required".to_string());
        }

        let output_format = self.format();
        let output = self.output_file();

        let report = crate::output::convert::load_scan_report(input)
            .map_err(|e| format!("Failed to load report: {}", e))?;

        let converted = match output_format {
            "junit" => crate::output::convert::convert_to_junit(&report),
            "csv" => crate::output::convert::convert_to_csv(&report),
            "html" => crate::output::convert::convert_to_html(&report),
            "sarif" => crate::output::convert::convert_to_sarif(&report),
            "markdown" => crate::output::convert::convert_to_markdown(&report),
            _ => crate::output::convert::convert_to_html(&report),
        };

        if !output.is_empty() {
            std::fs::write(output, &converted)
                .map_err(|e| format!("Failed to write output: {}", e))?;
            self.status_message = format!("Report converted and saved to {}", output);
        }

        Ok(converted)
    }

    pub fn schedule_cron(&self) -> &str {
        self.schedule_inputs
            .fields
            .first()
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn schedule_target(&self) -> &str {
        self.schedule_inputs
            .fields
            .get(1)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    }

    pub fn schedule_scan_type(&self) -> &str {
        self.schedule_inputs
            .fields
            .get(2)
            .map(|f| f.value.as_str())
            .unwrap_or("quick")
    }

    pub fn schedule_output(&self) -> Option<&str> {
        let v = self
            .schedule_inputs
            .fields
            .get(3)
            .map(|f| f.value.as_str())
            .unwrap_or("");
        if v.is_empty() {
            None
        } else {
            Some(v)
        }
    }

    pub fn add_schedule(&mut self) -> Result<(), String> {
        let cron = self.schedule_cron();
        if cron.is_empty() {
            return Err("Cron expression is required".to_string());
        }

        let target = self.schedule_target();
        if target.is_empty() {
            return Err("Target URL is required".to_string());
        }

        let new_schedule = ScheduledScan {
            schedule: cron.to_string(),
            target: target.to_string(),
            scan_type: self.schedule_scan_type().to_string(),
            output: self.schedule_output().map(|s| s.to_string()),
            enabled: true,
        };

        let mut config = self.config.clone().unwrap_or_default();
        config.schedule.push(new_schedule);

        let toml = toml::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        let config_path = self.config_path.as_deref().unwrap_or("slapper.toml");
        std::fs::write(config_path, &toml).map_err(|e| format!("Failed to write config: {}", e))?;

        self.status_message = "Schedule added successfully".to_string();
        self.config = Some(config);

        Ok(())
    }

    pub fn list_schedules(&self) -> String {
        match &self.config {
            Some(c) if !c.schedule.is_empty() => c
                .schedule
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    format!(
                        "[{}] {} -> {} ({})",
                        i + 1,
                        s.schedule,
                        s.target,
                        s.scan_type
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
            _ => "No schedules configured".to_string(),
        }
    }
}

impl Default for SettingsTab {
    fn default() -> Self {
        Self::new()
    }
}

impl TabState for SettingsTab {
    fn state(&self) -> AppState {
        AppState::Idle
    }

    fn progress(&self) -> f64 {
        0.0
    }

    fn reset(&mut self) {
        // Settings tab just resets to current state, no clear needed
        // The tab displays configuration, not scan results
    }
}

impl TabRender for SettingsTab {
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
        ];

        let mut nav_lines = Vec::new();
        for (label, section) in &nav_items {
            let style = if *section == self.current_section {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else {
                Style::default().fg(Color::Gray)
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
        }

        if !self.status_message.is_empty() {
            let status = Paragraph::new(self.status_message.as_str())
                .style(Style::default().fg(Color::Green));
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

impl TabInput for SettingsTab {
    fn handle_focus_next(&mut self) {
        match self.current_section {
            SettingsSection::Http => self.http_inputs.focus_next(),
            SettingsSection::Scan => self.scan_inputs.focus_next(),
            SettingsSection::Proxy => self.proxy_inputs.focus_next(),
            SettingsSection::Scope => self.scope_inputs.focus_next(),
            SettingsSection::Report => self.report_inputs.focus_next(),
            SettingsSection::Schedule => self.schedule_inputs.focus_next(),
            SettingsSection::Notifications => self.notify_inputs.focus_next(),
        }
    }

    fn handle_focus_prev(&mut self) {
        match self.current_section {
            SettingsSection::Http => self.http_inputs.focus_prev(),
            SettingsSection::Scan => self.scan_inputs.focus_prev(),
            SettingsSection::Proxy => self.proxy_inputs.focus_prev(),
            SettingsSection::Scope => self.scope_inputs.focus_prev(),
            SettingsSection::Report => self.report_inputs.focus_prev(),
            SettingsSection::Schedule => self.schedule_inputs.focus_prev(),
            SettingsSection::Notifications => self.notify_inputs.focus_prev(),
        }
    }

    fn handle_char(&mut self, c: char) {
        match self.current_section {
            SettingsSection::Http => self.http_inputs.insert(c),
            SettingsSection::Scan => self.scan_inputs.insert(c),
            SettingsSection::Proxy => self.proxy_inputs.insert(c),
            SettingsSection::Scope => self.scope_inputs.insert(c),
            SettingsSection::Report => self.report_inputs.insert(c),
            SettingsSection::Schedule => self.schedule_inputs.insert(c),
            SettingsSection::Notifications => self.notify_inputs.insert(c),
        }
    }

    fn handle_backspace(&mut self) {
        match self.current_section {
            SettingsSection::Http => self.http_inputs.backspace(),
            SettingsSection::Scan => self.scan_inputs.backspace(),
            SettingsSection::Proxy => self.proxy_inputs.backspace(),
            SettingsSection::Scope => self.scope_inputs.backspace(),
            SettingsSection::Report => self.report_inputs.backspace(),
            SettingsSection::Schedule => self.schedule_inputs.backspace(),
            SettingsSection::Notifications => self.notify_inputs.backspace(),
        }
    }

    fn handle_enter(&mut self) {
        match self.current_section {
            SettingsSection::Http => {
                if self.http_inputs.is_focused() {
                    self.http_inputs.blur();
                } else if self.follow_redirects.focused {
                    self.follow_redirects.toggle();
                } else if self.verify_tls.focused {
                    self.verify_tls.toggle();
                }
            }
            SettingsSection::Scan => {
                if self.scan_inputs.is_focused() {
                    self.scan_inputs.blur();
                } else if self.stealth_mode.focused {
                    self.stealth_mode.toggle();
                }
            }
            SettingsSection::Proxy => {
                if self.proxy_inputs.is_focused() {
                    self.proxy_inputs.blur();
                } else if self.proxy_rotation_selector.focused {
                    self.proxy_rotation_selector.toggle();
                }
            }
            SettingsSection::Scope => {
                if self.scope_inputs.is_focused() {
                    self.scope_inputs.blur();
                }
            }
            SettingsSection::Report => {
                if self.report_inputs.is_focused() {
                    self.report_inputs.blur();
                }
            }
            SettingsSection::Schedule => {
                if self.schedule_inputs.is_focused() {
                    self.schedule_inputs.blur();
                }
            }
            SettingsSection::Notifications => {
                if self.notify_inputs.is_focused() {
                    self.notify_inputs.blur();
                } else if self.notify_on_complete.focused {
                    self.notify_on_complete.toggle();
                } else if self.notify_on_findings.focused {
                    self.notify_on_findings.toggle();
                } else if self.severity_selector.focused {
                    self.severity_selector.toggle();
                }
            }
        }
    }

    fn handle_escape(&mut self) {
        match self.current_section {
            SettingsSection::Http => self.http_inputs.blur(),
            SettingsSection::Scan => self.scan_inputs.blur(),
            SettingsSection::Proxy => self.proxy_inputs.blur(),
            SettingsSection::Scope => self.scope_inputs.blur(),
            SettingsSection::Report => self.report_inputs.blur(),
            SettingsSection::Schedule => self.schedule_inputs.blur(),
            SettingsSection::Notifications => self.notify_inputs.blur(),
        }
    }

    fn handle_up(&mut self) {
        let sections = [
            SettingsSection::Http,
            SettingsSection::Scan,
            SettingsSection::Proxy,
            SettingsSection::Scope,
            SettingsSection::Report,
            SettingsSection::Schedule,
            SettingsSection::Notifications,
        ];
        if let Some(idx) = sections.iter().position(|s| *s == self.current_section) {
            if idx > 0 {
                self.current_section = sections[idx - 1];
            } else {
                self.current_section = sections[sections.len() - 1];
            }
        }
    }

    fn handle_down(&mut self) {
        let sections = [
            SettingsSection::Http,
            SettingsSection::Scan,
            SettingsSection::Proxy,
            SettingsSection::Scope,
            SettingsSection::Report,
            SettingsSection::Schedule,
            SettingsSection::Notifications,
        ];
        if let Some(idx) = sections.iter().position(|s| *s == self.current_section) {
            if idx < sections.len() - 1 {
                self.current_section = sections[idx + 1];
            } else {
                self.current_section = sections[0];
            }
        }
    }

    fn handle_left(&mut self) -> bool {
        match self.current_section {
            SettingsSection::Http => self.http_inputs.move_left(),
            SettingsSection::Scan => self.scan_inputs.move_left(),
            SettingsSection::Proxy => self.proxy_inputs.move_left(),
            SettingsSection::Scope => self.scope_inputs.move_left(),
            SettingsSection::Report => self.report_inputs.move_left(),
            SettingsSection::Schedule => self.schedule_inputs.move_left(),
            SettingsSection::Notifications => self.notify_inputs.move_left(),
        }
    }

    fn handle_right(&mut self) -> bool {
        match self.current_section {
            SettingsSection::Http => self.http_inputs.move_right(),
            SettingsSection::Scan => self.scan_inputs.move_right(),
            SettingsSection::Proxy => self.proxy_inputs.move_right(),
            SettingsSection::Scope => self.scope_inputs.move_right(),
            SettingsSection::Report => self.report_inputs.move_right(),
            SettingsSection::Schedule => self.schedule_inputs.move_right(),
            SettingsSection::Notifications => self.notify_inputs.move_right(),
        }
    }

    fn is_input_focused(&self) -> bool {
        match self.current_section {
            SettingsSection::Http => self.http_inputs.is_focused(),
            SettingsSection::Scan => self.scan_inputs.is_focused(),
            SettingsSection::Proxy => self.proxy_inputs.is_focused(),
            SettingsSection::Scope => self.scope_inputs.is_focused(),
            SettingsSection::Report => self.report_inputs.is_focused(),
            SettingsSection::Schedule => self.schedule_inputs.is_focused(),
            SettingsSection::Notifications => self.notify_inputs.is_focused(),
        }
    }
}
