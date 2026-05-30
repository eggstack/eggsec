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
    pub session_inputs: InputGroup,
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
    Session,
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

        let session_inputs =
            InputGroup::new().add(InputField::new("Auto-save Interval (seconds)").with_value("30"));

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
            session_inputs,
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
            SettingsSection::Session => 0,
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
        self.session_inputs.blur();
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
            SettingsSection::Session => {
                self.session_inputs.focus(idx);
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
        if let Some(field) = self.http_inputs.fields.get_mut(0) {
            field.value = config.http.timeout_secs.to_string();
        }
        if let Some(field) = self.http_inputs.fields.get_mut(1) {
            field.value = config.http.max_retries.to_string();
        }
        if let Some(field) = self.http_inputs.fields.get_mut(2) {
            field.value = config.http.retry_delay_ms.to_string();
        }
        if let Some(field) = self.http_inputs.fields.get_mut(3) {
            field.value = config.http.max_redirects.to_string();
        }
        self.follow_redirects.checked = config.http.follow_redirects;
        self.verify_tls.checked = config.http.verify_tls;

        if let Some(field) = self.scan_inputs.fields.get_mut(0) {
            field.value = config.scan.default_concurrency.to_string();
        }
        self.stealth_mode.checked = config.scan.stealth_mode;

        if let Some(field) = self.session_inputs.fields.get_mut(0) {
            field.value = config.auto_save_interval_secs.to_string();
        }

        if let Some(ref proxy_url) = config.http.proxy {
            if let Some(field) = self.proxy_inputs.fields.get_mut(0) {
                field.value = proxy_url.clone();
            }
        }

        if let Some(ref export_dir) = config.paths.export_dir {
            if let Some(field) = self.report_inputs.fields.get_mut(3) {
                field.value = export_dir.clone();
            }
        }

        self.notify_on_complete.checked = config.notifications.notify_on_complete;
        self.notify_on_findings.checked = config.notifications.notify_on_findings;

        self.config = Some(config.clone());
    }

    fn apply_to_config(&self, config: &mut SlapperConfig) {
        config.http.timeout_secs = self
            .http_inputs
            .fields
            .first()
            .map(|f| f.value.parse().unwrap_or(30))
            .unwrap_or(30);
        config.http.max_retries = self
            .http_inputs
            .fields
            .get(1)
            .map(|f| f.value.parse().unwrap_or(3))
            .unwrap_or(3);
        config.http.retry_delay_ms = self
            .http_inputs
            .fields
            .get(2)
            .map(|f| f.value.parse().unwrap_or(1000))
            .unwrap_or(1000);
        config.http.max_redirects = self
            .http_inputs
            .fields
            .get(3)
            .map(|f| f.value.parse().unwrap_or(10))
            .unwrap_or(10);
        config.http.follow_redirects = self.follow_redirects.checked;
        config.http.verify_tls = self.verify_tls.checked;
        config.http.proxy = self
            .proxy_inputs
            .fields
            .first()
            .map(|f| {
                if f.value.is_empty() {
                    None
                } else {
                    Some(f.value.clone())
                }
            })
            .unwrap_or(None);
        config.http.proxy_auth = self
            .proxy_inputs
            .fields
            .get(1)
            .map(|f| {
                if f.value.is_empty() {
                    None
                } else {
                    Some(crate::types::SensitiveString::new(f.value.clone()))
                }
            })
            .unwrap_or(None);

        config.scan.default_concurrency = self
            .scan_inputs
            .fields
            .first()
            .map(|f| f.value.parse().unwrap_or(50))
            .unwrap_or(50);
        config.scan.rate_limit_per_second = self
            .scan_inputs
            .fields
            .get(1)
            .and_then(|f| f.value.parse().ok());
        config.scan.port_timeout_secs = self
            .scan_inputs
            .fields
            .get(2)
            .map(|f| f.value.parse().unwrap_or(2))
            .unwrap_or(2);
        config.scan.stealth_mode = self.stealth_mode.checked;

        config.paths.export_dir = self.report_inputs.fields.get(3).and_then(|f| {
            let val = f.value.clone();
            if val.is_empty() || val == "./exports" {
                None
            } else {
                Some(val)
            }
        });

        config.auto_save_interval_secs = self
            .session_inputs
            .fields
            .first()
            .map(|f| f.value.parse().unwrap_or(30))
            .unwrap_or(30);

        config.notifications.notify_on_complete = self.notify_on_complete.checked;
        config.notifications.notify_on_findings = self.notify_on_findings.checked;
    }

    fn load_base_config_from_disk(&self) -> Option<SlapperConfig> {
        let config_path = self.config_path.as_deref().unwrap_or("slapper.toml");
        let content = std::fs::read_to_string(config_path).ok()?;
        toml::from_str(&content).ok()
    }

    pub fn to_config(&self) -> SlapperConfig {
        let mut config = self.config.clone().unwrap_or_else(|| SlapperConfig {
            http: HttpConfig {
                timeout_secs: 30,
                max_retries: 3,
                follow_redirects: true,
                verify_tls: true,
                max_redirects: 10,
                proxy: None,
                proxy_auth: None,
                default_headers: rustc_hash::FxHashMap::default(),
                default_user_agent: None,
                retry_delay_ms: 1000,
            },
            scan: ScanConfig {
                default_concurrency: 50,
                rate_limit_per_second: None,
                stealth_mode: false,
                jitter_ms: None,
                exclude_ports: Vec::new(),
                exclude_hosts: Vec::new(),
                port_timeout_secs: 2,
                save_session: false,
                session_dir: None,
            },
            output: OutputConfig::default(),
            notifications: NotificationConfig::default(),
            paths: crate::config::PathsConfig {
                custom_payloads_dir: None,
                wordlists_dir: None,
                export_dir: None,
            },
            profiles: rustc_hash::FxHashMap::default(),
            recon: crate::config::ReconConfig::default(),
            schedule: Vec::new(),
            remote: crate::config::RemoteConfig::default(),
            proxies: Vec::new(),
            ai: None,
            search: None,
            alert_channels: crate::config::AlertChannelsConfig::default(),
            execution_policy: crate::config::ExecutionPolicy::default(),
            auto_save_interval_secs: 30,
        });
        self.apply_to_config(&mut config);
        config
    }

    pub fn save_config(&mut self) {
        let mut config = self
            .config
            .clone()
            .or_else(|| self.load_base_config_from_disk())
            .unwrap_or_else(|| self.to_config());
        self.apply_to_config(&mut config);

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
        self.config = None;
        self.error = None;
        if let Some(field) = self.http_inputs.fields.get_mut(0) {
            field.value = "30".to_string();
        }
        if let Some(field) = self.http_inputs.fields.get_mut(1) {
            field.value = "3".to_string();
        }
        if let Some(field) = self.http_inputs.fields.get_mut(2) {
            field.value = "1000".to_string();
        }
        if let Some(field) = self.http_inputs.fields.get_mut(3) {
            field.value = "10".to_string();
        }
        if let Some(field) = self.scan_inputs.fields.get_mut(0) {
            field.value = "50".to_string();
        }
        if let Some(field) = self.scan_inputs.fields.get_mut(1) {
            field.value = "0".to_string();
        }
        if let Some(field) = self.scan_inputs.fields.get_mut(2) {
            field.value = "2".to_string();
        }
        if let Some(field) = self.proxy_inputs.fields.get_mut(0) {
            field.value.clear();
        }
        if let Some(field) = self.proxy_inputs.fields.get_mut(1) {
            field.value.clear();
        }
        for field in self.scope_inputs.fields.iter_mut() {
            field.value.clear();
        }
        for field in self.report_inputs.fields.iter_mut() {
            field.value.clear();
        }
        for field in self.schedule_inputs.fields.iter_mut() {
            field.value.clear();
        }
        for field in self.notify_inputs.fields.iter_mut() {
            field.value.clear();
        }
        for field in self.session_inputs.fields.iter_mut() {
            field.value.clear();
        }
        self.follow_redirects.checked = true;
        self.verify_tls.checked = true;
        self.stealth_mode.checked = false;
        self.notify_on_complete.checked = false;
        self.notify_on_findings.checked = false;
        self.proxy_rotation_selector.select(0);
        self.severity_selector.select(0);
        self.dark_mode.checked = false;
        self.accent_color.select(0);
        self.focus_area = SettingsFocusArea::SectionList;
        self.current_section = SettingsSection::Http;
        self.detail_focus_index = 0;
        self.status_message = String::new();
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
            "junit" => crate::output::convert::convert_to_junit(&report)
                .unwrap_or_else(|e| format!("Error: {}", e)),
            "csv" => crate::output::convert::convert_to_csv(&report),
            "html" => crate::output::convert::convert_to_html(&report),
            "sarif" => crate::output::convert::convert_to_sarif(&report)
                .unwrap_or_else(|e| format!("Error: {}", e)),
            "markdown" => crate::output::convert::convert_to_markdown(&report)
                .unwrap_or_else(|e| format!("Error: {}", e)),
            _ => crate::output::convert::convert_to_html(&report),
        };

        if !output.is_empty() {
            if let Err(e) = std::fs::write(output, &converted) {
                self.status_message = format!("Error: Failed to write output: {}", e);
            } else {
                self.status_message = format!("Report converted and saved to {}", output);
            }
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
        if let Err(e) = std::fs::write(config_path, &toml) {
            tracing::warn!("Failed to write config file: {}", e);
            return Err(format!("Failed to write config: {}", e));
        }

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
            SettingsSection::Session => self.session_inputs.is_focused(),
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
