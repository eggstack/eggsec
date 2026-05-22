use crate::config::{
    HttpConfig, NotificationConfig, OutputConfig, ScanConfig, ScheduledScan, SlapperConfig,
};
use crate::tui::app::tab_error::TabError;
use crate::tui::components::{Checkbox, InputField, InputGroup, Selector, SelectorItem};
use crate::tui::tabs::{AppState, TabState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsFocusArea {
    SectionList,
    SectionDetail,
}

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
    pub focus_area: SettingsFocusArea,
    pub detail_focus_index: usize,
    pub config: Option<SlapperConfig>,
    pub config_path: Option<String>,
    pub status_message: String,
    pub dark_mode: Checkbox,
    pub accent_color: Selector,
    pub error: Option<TabError>,
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
    Theme,
}

impl SettingsTab {
    pub fn new() -> Self {
        let http_inputs = InputGroup::new()
            .add(InputField::new("Timeout (s)").with_value("30"))
            .add(InputField::new("Max Retries").with_value("3"))
            .add(InputField::new("Retry Delay (ms)").with_value("1000"))
            .add(InputField::new("Max Redirects").with_value("10"));

        let scan_inputs = InputGroup::new()
            .add(InputField::new("Default Concurrency").with_value("50"))
            .add(InputField::new("Rate Limit (req/s)").with_value("0"))
            .add(InputField::new("Port Timeout (s)").with_value("2"));

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

        let accent_color = Selector::new("Accent Color").items(vec![
            SelectorItem::new("Cyan", "cyan"),
            SelectorItem::new("Blue", "blue"),
            SelectorItem::new("Green", "green"),
            SelectorItem::new("Yellow", "yellow"),
            SelectorItem::new("Red", "red"),
            SelectorItem::new("Magenta", "magenta"),
            SelectorItem::new("White", "white"),
            SelectorItem::new("Black", "black"),
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
            focus_area: SettingsFocusArea::SectionList,
            detail_focus_index: 0,
            config: None,
            config_path: Some("slapper.toml".to_string()),
            status_message: String::new(),
            dark_mode: Checkbox::new("Dark Mode").checked(true),
            accent_color,
            error: None,
        }
    }

    pub fn set_config_path(&mut self, path: String) {
        self.config_path = Some(path);
    }

    pub fn max_focus_index(&self) -> usize {
        match self.current_section {
            SettingsSection::Http => 5,
            SettingsSection::Scan => 3,
            SettingsSection::Proxy => 2,
            SettingsSection::Scope => 1,
            SettingsSection::Report => 3,
            SettingsSection::Schedule => 3,
            SettingsSection::Notifications => 6,
            SettingsSection::Theme => 1,
        }
    }

    pub fn sync_component_focus(&mut self) {
        let is_detail = self.focus_area == SettingsFocusArea::SectionDetail;
        let idx = self.detail_focus_index;
        let keep_proxy_rotation_open =
            is_detail && self.current_section == SettingsSection::Proxy && idx == 2;
        let keep_severity_open =
            is_detail && self.current_section == SettingsSection::Notifications && idx == 6;
        let keep_accent_open =
            is_detail && self.current_section == SettingsSection::Theme && idx == 1;

        // Reset all
        self.http_inputs.blur();
        self.scan_inputs.blur();
        self.proxy_inputs.blur();
        self.scope_inputs.blur();
        self.report_inputs.blur();
        self.schedule_inputs.blur();
        self.notify_inputs.blur();
        self.follow_redirects.focused = false;
        self.verify_tls.focused = false;
        self.stealth_mode.focused = false;
        self.notify_on_complete.focused = false;
        self.notify_on_findings.focused = false;
        self.proxy_rotation_selector.focused = false;
        self.severity_selector.focused = false;
        self.dark_mode.focused = false;
        self.accent_color.focused = false;
        if !keep_proxy_rotation_open {
            self.proxy_rotation_selector.close();
        }
        if !keep_severity_open {
            self.severity_selector.close();
        }
        if !keep_accent_open {
            self.accent_color.close();
        }

        if !is_detail {
            return;
        }

        match self.current_section {
            SettingsSection::Http => {
                if idx < 4 {
                    self.http_inputs.focus(idx);
                } else if idx == 4 {
                    self.follow_redirects.focused = true;
                } else {
                    self.verify_tls.focused = true;
                }
            }
            SettingsSection::Scan => {
                if idx < 3 {
                    self.scan_inputs.focus(idx);
                } else {
                    self.stealth_mode.focused = true;
                }
            }
            SettingsSection::Proxy => {
                if idx < 2 {
                    self.proxy_inputs.focus(idx);
                } else {
                    self.proxy_rotation_selector.focused = true;
                }
            }
            SettingsSection::Scope => {
                self.scope_inputs.focus(idx);
            }
            SettingsSection::Report => {
                self.report_inputs.focus(idx);
            }
            SettingsSection::Schedule => {
                self.schedule_inputs.focus(idx);
            }
            SettingsSection::Notifications => {
                if idx < 4 {
                    self.notify_inputs.focus(idx);
                } else if idx == 4 {
                    self.notify_on_complete.focused = true;
                } else if idx == 5 {
                    self.notify_on_findings.focused = true;
                } else {
                    self.severity_selector.focused = true;
                }
            }
            SettingsSection::Theme => {
                if idx == 0 {
                    self.dark_mode.focused = true;
                } else {
                    self.accent_color.focused = true;
                }
            }
        }
    }

    pub fn load_config(&mut self, config: &SlapperConfig) {
        self.http_inputs.fields[0].value = config.http.timeout_secs.to_string();
        self.http_inputs.fields[1].value = config.http.max_retries.to_string();
        self.http_inputs.fields[2].value = config.http.retry_delay_ms.to_string();
        self.http_inputs.fields[3].value = config.http.max_redirects.to_string();
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
        let retry_delay_ms = self.http_inputs.fields[2].value.parse().unwrap_or(1000);
        let max_redirects = self.http_inputs.fields[3].value.parse().unwrap_or(10);
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
                default_headers: rustc_hash::FxHashMap::default(),
                default_user_agent: None,
                retry_delay_ms,
            },
            scan: ScanConfig {
                default_concurrency,
                rate_limit_per_second: self.scan_inputs.fields[1].value.parse().ok(),
                stealth_mode: self.stealth_mode.checked,
                jitter_ms: None,
                exclude_ports: Vec::new(),
                exclude_hosts: Vec::new(),
                port_timeout_secs: self.scan_inputs.fields[2].value.parse().unwrap_or(2),
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
            profiles: rustc_hash::FxHashMap::default(),
            recon: crate::config::ReconConfig::default(),
            schedule: Vec::new(),
            remote: crate::config::RemoteConfig::default(),
            proxies: Vec::new(),
            ai: None,
            search: None,
            alert_channels: crate::config::AlertChannelsConfig::default(),
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

    pub fn sync_with_theme(&mut self, theme: &crate::tui::theme::Theme) {
        self.dark_mode.checked = theme.mode == crate::tui::theme::ThemeMode::Dark;
        let color_name = match theme.colors.accent {
            ratatui::style::Color::Cyan => "Cyan",
            ratatui::style::Color::Blue => "Blue",
            ratatui::style::Color::Green => "Green",
            ratatui::style::Color::Yellow => "Yellow",
            ratatui::style::Color::Red => "Red",
            ratatui::style::Color::Magenta => "Magenta",
            ratatui::style::Color::White => "White",
            ratatui::style::Color::Black => "Black",
            _ => "Cyan",
        };
        if let Some(idx) = self
            .accent_color
            .items
            .iter()
            .position(|it| it.label == color_name)
        {
            self.accent_color.select(idx);
        }
    }

    pub fn reset(&mut self) {
        self.http_inputs.fields[0].value = "30".to_string();
        self.http_inputs.fields[1].value = "3".to_string();
        self.http_inputs.fields[2].value = "10".to_string();
        self.scan_inputs.fields[0].value = "50".to_string();
        self.scan_inputs.fields[1].value = "0".to_string();
        self.scan_inputs.fields[2].value = "2".to_string();
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
            "sarif" => crate::output::convert::convert_to_sarif(&report)
                .unwrap_or_else(|e| format!("Error: {}", e)),
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

    pub fn is_input_focused(&self) -> bool {
        match self.current_section {
            SettingsSection::Http => self.http_inputs.is_focused(),
            SettingsSection::Scan => self.scan_inputs.is_focused(),
            SettingsSection::Proxy => self.proxy_inputs.is_focused(),
            SettingsSection::Scope => self.scope_inputs.is_focused(),
            SettingsSection::Report => self.report_inputs.is_focused(),
            SettingsSection::Schedule => self.schedule_inputs.is_focused(),
            SettingsSection::Notifications => self.notify_inputs.is_focused(),
            SettingsSection::Theme => false,
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
        self.error = None;
        SettingsTab::reset(self);
    }

    fn set_error(&mut self, error: TabError) {
        self.error = Some(error);
    }
}
