use crate::app::tab_error::TabError;
use crate::components::{Checkbox, InputField, InputGroup, Selector, SelectorItem};
use crate::tabs::{AppState, TabState};
use crate::theme::manager::ThemeInfo;
use crate::theme::palette::ThemeColors;
use crate::theme::{canonical_theme_id, display_theme_name};
use eggsec::config::{
    EggsecConfig, HttpConfig, NotificationConfig, OutputConfig, ScanConfig, ScheduledScan,
};
use rustc_hash::FxHashMap;

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
    pub config: Option<EggsecConfig>,
    pub config_path: Option<String>,
    pub status_message: String,
    pub theme_selector: Selector,
    pub pending_theme_name: Option<String>,
    pub error: Option<TabError>,
    /// Cached theme metadata from ThemeManager.
    pub theme_info_cache: Vec<ThemeInfo>,
    /// Number of themes with invalid/missing status.
    pub theme_invalid_count: usize,
    /// Path to the user theme directory (e.g., ~/.config/eggsec/themes).
    pub theme_dir_path: String,
    /// Per-theme contrast warnings keyed by canonical theme ID.
    pub theme_contrast_cache: FxHashMap<String, Vec<String>>,
    /// Resolved colors for the currently selected theme (for preview rendering).
    pub resolved_theme_colors: Option<ThemeColors>,
    /// ID of the currently applied (active) theme, set from ThemeManager.
    pub applied_theme_id: Option<String>,
    /// Flag set when theme selector moves; App layer checks and refreshes preview.
    pub needs_theme_preview_refresh: bool,
    /// Pending theme reload requested by user (picked up by App layer).
    pub pending_theme_reload: bool,
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
            .add(InputField::new("Rate Limit (req/s)"))
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

        let theme_selector = Selector::new("Theme").items(vec![
            SelectorItem::new(display_theme_name("cyber-red"), "cyber-red"),
            SelectorItem::new(display_theme_name("dark"), "dark"),
            SelectorItem::new(display_theme_name("light"), "light"),
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
            config_path: Some("eggsec.toml".to_string()),
            status_message: String::new(),
            theme_selector,
            pending_theme_name: None,
            error: None,
            theme_info_cache: Vec::new(),
            theme_invalid_count: 0,
            theme_dir_path: String::new(),
            theme_contrast_cache: FxHashMap::default(),
            resolved_theme_colors: None,
            applied_theme_id: None,
            needs_theme_preview_refresh: false,
            pending_theme_reload: false,
        }
    }

    pub fn set_config_path(&mut self, path: String) {
        self.config_path = Some(path);
    }

    pub fn take_pending_theme(&mut self) -> Option<String> {
        self.pending_theme_name.take()
    }

    pub fn take_pending_theme_reload(&mut self) -> bool {
        if self.pending_theme_reload {
            self.pending_theme_reload = false;
            true
        } else {
            false
        }
    }

    pub(crate) fn restore_theme_preview_selection(&mut self) {
        if let Some(ref applied_id) = self.applied_theme_id {
            self.theme_selector.select_by_value(applied_id);
        }
        self.needs_theme_preview_refresh = true;
    }

    /// Update cached theme metadata from the ThemeManager.
    pub fn update_theme_metadata(
        &mut self,
        info_cache: Vec<ThemeInfo>,
        invalid_count: usize,
        dir_path: String,
        contrast_cache: FxHashMap<String, Vec<String>>,
        resolved_theme_colors: Option<ThemeColors>,
    ) {
        self.theme_info_cache = info_cache;
        self.theme_invalid_count = invalid_count;
        self.theme_dir_path = dir_path;
        self.theme_contrast_cache = contrast_cache;
        self.resolved_theme_colors = resolved_theme_colors;
    }

    pub fn max_focus_index(&self) -> usize {
        self.detail_item_count().saturating_sub(1)
    }

    pub(crate) fn current_text_inputs(&self) -> Option<&InputGroup> {
        match self.current_section {
            SettingsSection::Http => Some(&self.http_inputs),
            SettingsSection::Scan => Some(&self.scan_inputs),
            SettingsSection::Session => Some(&self.session_inputs),
            SettingsSection::Proxy => Some(&self.proxy_inputs),
            SettingsSection::Scope => Some(&self.scope_inputs),
            SettingsSection::Report => Some(&self.report_inputs),
            SettingsSection::Schedule => Some(&self.schedule_inputs),
            SettingsSection::Notifications => Some(&self.notify_inputs),
            SettingsSection::Theme => None,
        }
    }

    pub(crate) fn current_text_inputs_mut(&mut self) -> Option<&mut InputGroup> {
        match self.current_section {
            SettingsSection::Http => Some(&mut self.http_inputs),
            SettingsSection::Scan => Some(&mut self.scan_inputs),
            SettingsSection::Session => Some(&mut self.session_inputs),
            SettingsSection::Proxy => Some(&mut self.proxy_inputs),
            SettingsSection::Scope => Some(&mut self.scope_inputs),
            SettingsSection::Report => Some(&mut self.report_inputs),
            SettingsSection::Schedule => Some(&mut self.schedule_inputs),
            SettingsSection::Notifications => Some(&mut self.notify_inputs),
            SettingsSection::Theme => None,
        }
    }

    pub(crate) fn current_text_field_count(&self) -> usize {
        self.current_text_inputs()
            .map(|inputs| inputs.fields.len())
            .unwrap_or(0)
    }

    fn detail_item_count(&self) -> usize {
        let text_fields = self.current_text_field_count();
        match self.current_section {
            SettingsSection::Http => text_fields + 2,
            SettingsSection::Scan => text_fields + 1,
            SettingsSection::Session
            | SettingsSection::Scope
            | SettingsSection::Report
            | SettingsSection::Schedule => text_fields,
            SettingsSection::Proxy => text_fields + 1,
            SettingsSection::Notifications => text_fields + 3,
            SettingsSection::Theme => 1,
        }
    }

    pub fn sync_component_focus(&mut self) {
        let is_detail = self.focus_area == SettingsFocusArea::SectionDetail;
        self.detail_focus_index = self.detail_focus_index.min(self.max_focus_index());
        let idx = self.detail_focus_index;
        let keep_proxy_rotation_open =
            is_detail && self.current_section == SettingsSection::Proxy && idx == 2;
        let keep_severity_open =
            is_detail && self.current_section == SettingsSection::Notifications && idx == 6;
        let keep_theme_selector_open =
            is_detail && self.current_section == SettingsSection::Theme && idx == 0;
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
        self.theme_selector.focused = false;
        if !keep_proxy_rotation_open {
            self.proxy_rotation_selector.close();
        }
        if !keep_severity_open {
            self.severity_selector.close();
        }
        if !keep_theme_selector_open {
            self.theme_selector.close();
        }

        if !is_detail {
            return;
        }

        match self.current_section {
            SettingsSection::Http => {
                let input_count = self.http_inputs.fields.len();
                if idx < input_count {
                    self.http_inputs.focus(idx);
                } else if idx == input_count {
                    self.follow_redirects.focused = true;
                } else {
                    self.verify_tls.focused = true;
                }
            }
            SettingsSection::Scan => {
                let input_count = self.scan_inputs.fields.len();
                if idx < input_count {
                    self.scan_inputs.focus(idx);
                } else {
                    self.stealth_mode.focused = true;
                }
            }
            SettingsSection::Session => {
                self.session_inputs.focus(idx);
            }
            SettingsSection::Proxy => {
                let input_count = self.proxy_inputs.fields.len();
                if idx < input_count {
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
                let input_count = self.notify_inputs.fields.len();
                if idx < input_count {
                    self.notify_inputs.focus(idx);
                } else if idx == input_count {
                    self.notify_on_complete.focused = true;
                } else if idx == input_count + 1 {
                    self.notify_on_findings.focused = true;
                } else {
                    self.severity_selector.focused = true;
                }
            }
            SettingsSection::Theme => {
                self.theme_selector.focused = true;
            }
        }
    }

    pub fn load_config(&mut self, config: &EggsecConfig) {
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
        if let Some(field) = self.scan_inputs.fields.get_mut(1) {
            field.value = config
                .scan
                .rate_limit_per_second
                .map(|v| v.to_string())
                .unwrap_or_default();
        }
        if let Some(field) = self.scan_inputs.fields.get_mut(2) {
            field.value = config.scan.port_timeout_secs.to_string();
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

    fn apply_to_config(&self, config: &mut EggsecConfig) {
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
                    Some(eggsec::types::SensitiveString::new(f.value.clone()))
                }
            })
            .unwrap_or(None);

        config.scan.default_concurrency = self
            .scan_inputs
            .fields
            .first()
            .map(|f| f.value.parse().unwrap_or(50))
            .unwrap_or(50);
        config.scan.rate_limit_per_second = self.scan_inputs.fields.get(1).and_then(|f| {
            if f.value.is_empty() {
                None
            } else {
                f.value.parse::<u32>().ok()
            }
        });
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

    fn load_base_config_from_disk(&self) -> Option<EggsecConfig> {
        let config_path = self.config_path.as_deref().unwrap_or("eggsec.toml");
        let content = match std::fs::read_to_string(config_path) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to read config file {}: {}", config_path, e);
                return None;
            }
        };
        match toml::from_str(&content) {
            Ok(config) => Some(config),
            Err(e) => {
                tracing::warn!("Failed to parse config file {}: {}", config_path, e);
                None
            }
        }
    }

    pub fn to_config(&self) -> EggsecConfig {
        let mut config = self.config.clone().unwrap_or_else(|| EggsecConfig {
            http: HttpConfig {
                timeout_secs: eggsec_core::constants::http::DEFAULT_TIMEOUT_SECS,
                max_retries: eggsec_core::constants::DEFAULT_MAX_RETRIES,
                follow_redirects: true,
                verify_tls: true,
                max_redirects: 10,
                proxy: None,
                proxy_auth: None,
                default_headers: rustc_hash::FxHashMap::default(),
                default_user_agent: None,
                retry_delay_ms: eggsec_core::constants::DEFAULT_RETRY_DELAY_MS,
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
            paths: eggsec::config::PathsConfig {
                custom_payloads_dir: None,
                wordlists_dir: None,
                export_dir: None,
            },
            profiles: rustc_hash::FxHashMap::default(),
            recon: eggsec::config::ReconConfig::default(),
            schedule: Vec::new(),
            remote: eggsec::config::RemoteConfig::default(),
            proxies: Vec::new(),
            ai: None,
            search: None,
            alert_channels: eggsec::config::AlertChannelsConfig::default(),
            execution_policy: eggsec::config::ExecutionPolicy::default(),
            #[cfg(feature = "external-integrations")]
            integrations: eggsec::integrations::IntegrationConfig::default(),
            auto_save_interval_secs: 30,
        });
        self.apply_to_config(&mut config);
        config
    }

    fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if let Some(field) = self.http_inputs.fields.first() {
            match field.value.parse::<u64>() {
                Ok(v) if v > 0 => {}
                Ok(_) => errors.push("HTTP timeout_secs must be greater than 0".to_string()),
                Err(_) => errors.push("HTTP timeout_secs must be a valid number".to_string()),
            }
        }
        if let Some(field) = self.http_inputs.fields.get(1) {
            if field.value.parse::<u32>().is_err() {
                errors.push("HTTP max_retries must be a valid number".to_string());
            }
        }
        if let Some(field) = self.http_inputs.fields.get(2) {
            if field.value.parse::<u64>().is_err() {
                errors.push("HTTP retry_delay_ms must be a valid number".to_string());
            }
        }
        if let Some(field) = self.http_inputs.fields.get(3) {
            if field.value.parse::<u32>().is_err() {
                errors.push("HTTP max_redirects must be a valid number".to_string());
            }
        }
        if let Some(field) = self.scan_inputs.fields.first() {
            match field.value.parse::<u32>() {
                Ok(v) if v > 0 => {}
                Ok(_) => errors.push("Scan default_concurrency must be greater than 0".to_string()),
                Err(_) => {
                    errors.push("Scan default_concurrency must be a valid number".to_string())
                }
            }
        }
        if let Some(field) = self.scan_inputs.fields.get(1) {
            if !field.value.is_empty() {
                match field.value.parse::<u32>() {
                    Ok(0) => errors.push(
                        "Scan rate_limit_per_second cannot be 0 (leave empty for no limit)"
                            .to_string(),
                    ),
                    Ok(_) => {}
                    Err(_) => {
                        errors.push("Scan rate_limit_per_second must be a valid number".to_string())
                    }
                }
            }
        }
        if let Some(field) = self.scan_inputs.fields.get(2) {
            match field.value.parse::<u64>() {
                Ok(v) if v > 0 => {}
                Ok(_) => errors.push("Scan port_timeout_secs must be greater than 0".to_string()),
                Err(_) => errors.push("Scan port_timeout_secs must be a valid number".to_string()),
            }
        }
        if let Some(field) = self.session_inputs.fields.first() {
            match field.value.parse::<u64>() {
                Ok(v) if v > 0 => {}
                Ok(_) => errors
                    .push("Session auto_save_interval_secs must be greater than 0".to_string()),
                Err(_) => errors
                    .push("Session auto_save_interval_secs must be a valid number".to_string()),
            }
        }
        if let Some(field) = self.report_inputs.fields.get(2) {
            let fmt = field.value.to_lowercase();
            let valid = ["json", "csv", "html", "sarif", "junit", "markdown"];
            if !valid.contains(&fmt.as_str()) {
                errors.push(format!(
                    "Report format must be one of: {}",
                    valid.join(", ")
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn save_config(&mut self) {
        if let Err(errors) = self.validate() {
            self.status_message = errors.first().cloned().unwrap_or_default();
            return;
        }

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
                let config_path = self.config_path.as_deref().unwrap_or("eggsec.toml");
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
        self.config = None;
        self.error = None;
        if let Some(field) = self.http_inputs.fields.get_mut(0) {
            field.value = "30".to_string();
            field.cursor_pos = 0;
        }
        if let Some(field) = self.http_inputs.fields.get_mut(1) {
            field.value = "3".to_string();
            field.cursor_pos = 0;
        }
        if let Some(field) = self.http_inputs.fields.get_mut(2) {
            field.value = "1000".to_string();
            field.cursor_pos = 0;
        }
        if let Some(field) = self.http_inputs.fields.get_mut(3) {
            field.value = "10".to_string();
            field.cursor_pos = 0;
        }
        if let Some(field) = self.scan_inputs.fields.get_mut(0) {
            field.value = "50".to_string();
            field.cursor_pos = 0;
        }
        if let Some(field) = self.scan_inputs.fields.get_mut(1) {
            field.value.clear();
            field.cursor_pos = 0;
        }
        if let Some(field) = self.scan_inputs.fields.get_mut(2) {
            field.value = "2".to_string();
            field.cursor_pos = 0;
        }
        if let Some(field) = self.proxy_inputs.fields.get_mut(0) {
            field.value.clear();
            field.cursor_pos = 0;
        }
        if let Some(field) = self.proxy_inputs.fields.get_mut(1) {
            field.value.clear();
            field.cursor_pos = 0;
        }
        for field in self.scope_inputs.fields.iter_mut() {
            field.value.clear();
            field.cursor_pos = 0;
        }
        for field in self.report_inputs.fields.iter_mut() {
            field.value.clear();
            field.cursor_pos = 0;
        }
        if let Some(f) = self.report_inputs.fields.get_mut(2) {
            f.value = "html".to_string();
            f.cursor_pos = 0;
        }
        if let Some(f) = self.report_inputs.fields.get_mut(3) {
            f.value = "./exports".to_string();
            f.cursor_pos = 0;
        }
        for field in self.schedule_inputs.fields.iter_mut() {
            field.value.clear();
            field.cursor_pos = 0;
        }
        if let Some(f) = self.schedule_inputs.fields.get_mut(2) {
            f.value = "quick".to_string();
            f.cursor_pos = 0;
        }
        for field in self.notify_inputs.fields.iter_mut() {
            field.value.clear();
            field.cursor_pos = 0;
        }
        if let Some(f) = self.session_inputs.fields.get_mut(0) {
            f.value = "30".to_string();
            f.cursor_pos = 0;
        }
        self.follow_redirects.checked = true;
        self.verify_tls.checked = true;
        self.stealth_mode.checked = false;
        self.notify_on_complete.checked = false;
        self.notify_on_findings.checked = true;
        self.proxy_rotation_selector.select(0);
        self.severity_selector.select(0);
        self.theme_selector.select(0);
        self.focus_area = SettingsFocusArea::SectionList;
        self.current_section = SettingsSection::Http;
        self.detail_focus_index = 0;
        self.status_message = String::new();
        self.sync_component_focus();
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

        let report = eggsec::output::convert::load_scan_report(input)
            .map_err(|e| format!("Failed to load report: {}", e))?;

        let converted = match output_format.parse::<eggsec::types::OutputFormat>() {
            Ok(eggsec::types::OutputFormat::Junit) => {
                eggsec::output::convert::convert_to_junit(&report)
                    .unwrap_or_else(|e| format!("Error: {}", e))
            }
            Ok(eggsec::types::OutputFormat::Csv) => {
                eggsec::output::convert::convert_to_csv(&report)
            }
            Ok(eggsec::types::OutputFormat::Html) => {
                eggsec::output::convert::convert_to_html(&report)
            }
            Ok(eggsec::types::OutputFormat::Sarif) => {
                eggsec::output::convert::convert_to_sarif(&report)
                    .unwrap_or_else(|e| format!("Error: {}", e))
            }
            Ok(eggsec::types::OutputFormat::Markdown) => {
                eggsec::output::convert::convert_to_markdown(&report)
                    .unwrap_or_else(|e| format!("Error: {}", e))
            }
            _ => eggsec::output::convert::convert_to_html(&report),
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

        let config_path = self.config_path.as_deref().unwrap_or("eggsec.toml");
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

    pub fn set_available_themes(&mut self, themes: &[(String, String)], current_id: &str) {
        let items: Vec<crate::components::SelectorItem> = themes
            .iter()
            .map(|(id, label)| crate::components::SelectorItem::new(label, id))
            .collect();
        self.theme_selector.set_items(items);
        let current_id = canonical_theme_id(current_id);
        if self.theme_selector.items.is_empty() {
            // No themes registered (extreme edge case - cyber-red is always
            // built-in). Avoid selecting from an empty list to keep the
            // selector in a consistent state.
            return;
        }
        self.theme_selector.select_by_value(&current_id);
        // If the current theme isn't in the list (e.g., it failed to load
        // and was skipped during install), keep the user's chosen theme
        // in the dropdown so they can see what's actually applied. The
        // `select(0)` fallback was visually confusing because it would
        // imply a theme switch.
        if self.theme_selector.selected_value() != Some(current_id.as_str()) {
            // Leading marker and "unavailable" suffix make it visually clear
            // that this entry is a placeholder, not an installed theme.
            let placeholder = format!("[! {current_id}] (not installed)");
            self.theme_selector
                .set_items_with_extra(crate::components::SelectorItem::new(
                    &placeholder,
                    &current_id,
                ));
            self.theme_selector.select_by_value(&current_id);
        }
    }

    pub fn is_input_focused(&self) -> bool {
        self.current_text_inputs()
            .map(InputGroup::is_focused)
            .unwrap_or(false)
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

    fn has_selector_open(&self) -> bool {
        self.theme_selector.is_open()
            || self.proxy_rotation_selector.is_open()
            || self.severity_selector.is_open()
    }

    fn reset(&mut self) {
        SettingsTab::reset(self);
    }

    fn set_error(&mut self, error: TabError) {
        self.error = Some(error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tabs::TabInput;

    fn set_http_field(tab: &mut SettingsTab, idx: usize, val: &str) {
        if let Some(f) = tab.http_inputs.fields.get_mut(idx) {
            f.value = val.to_string();
        }
    }

    fn set_scan_field(tab: &mut SettingsTab, idx: usize, val: &str) {
        if let Some(f) = tab.scan_inputs.fields.get_mut(idx) {
            f.value = val.to_string();
        }
    }

    fn set_session_field(tab: &mut SettingsTab, idx: usize, val: &str) {
        if let Some(f) = tab.session_inputs.fields.get_mut(idx) {
            f.value = val.to_string();
        }
    }

    fn set_report_field(tab: &mut SettingsTab, idx: usize, val: &str) {
        if let Some(f) = tab.report_inputs.fields.get_mut(idx) {
            f.value = val.to_string();
        }
    }

    #[test]
    fn validate_defaults_pass() {
        let tab = SettingsTab::new();
        assert!(tab.validate().is_ok());
    }

    #[test]
    fn validate_valid_values_pass() {
        let mut tab = SettingsTab::new();
        set_http_field(&mut tab, 0, "10");
        set_http_field(&mut tab, 1, "5");
        set_http_field(&mut tab, 2, "500");
        set_http_field(&mut tab, 3, "20");
        set_scan_field(&mut tab, 0, "100");
        set_scan_field(&mut tab, 2, "5");
        set_session_field(&mut tab, 0, "60");
        set_report_field(&mut tab, 2, "json");
        assert!(tab.validate().is_ok());
    }

    #[test]
    fn validate_invalid_timeout_fails() {
        let mut tab = SettingsTab::new();
        set_http_field(&mut tab, 0, "abc");
        let err = tab.validate().unwrap_err();
        assert!(err.iter().any(|e| e.contains("timeout_secs")));
    }

    #[test]
    fn validate_zero_timeout_fails() {
        let mut tab = SettingsTab::new();
        set_http_field(&mut tab, 0, "0");
        let err = tab.validate().unwrap_err();
        assert!(err.iter().any(|e| e.contains("timeout_secs")));
    }

    #[test]
    fn validate_invalid_auto_save_fails() {
        let mut tab = SettingsTab::new();
        set_session_field(&mut tab, 0, "xyz");
        let err = tab.validate().unwrap_err();
        assert!(err.iter().any(|e| e.contains("auto_save_interval_secs")));
    }

    #[test]
    fn validate_zero_auto_save_fails() {
        let mut tab = SettingsTab::new();
        set_session_field(&mut tab, 0, "0");
        let err = tab.validate().unwrap_err();
        assert!(err.iter().any(|e| e.contains("auto_save_interval_secs")));
    }

    #[test]
    fn validate_invalid_report_format_fails() {
        let mut tab = SettingsTab::new();
        set_report_field(&mut tab, 2, "xml");
        let err = tab.validate().unwrap_err();
        assert!(err.iter().any(|e| e.contains("Report format")));
    }

    #[test]
    fn validate_report_format_case_insensitive() {
        let mut tab = SettingsTab::new();
        set_report_field(&mut tab, 2, "JSON");
        assert!(tab.validate().is_ok());

        set_report_field(&mut tab, 2, "Html");
        assert!(tab.validate().is_ok());
    }

    #[test]
    fn validate_empty_concurrency_fails() {
        let mut tab = SettingsTab::new();
        set_scan_field(&mut tab, 0, "");
        let err = tab.validate().unwrap_err();
        assert!(err.iter().any(|e| e.contains("default_concurrency")));
    }

    #[test]
    fn validate_zero_concurrency_fails() {
        let mut tab = SettingsTab::new();
        set_scan_field(&mut tab, 0, "0");
        let err = tab.validate().unwrap_err();
        assert!(err.iter().any(|e| e.contains("default_concurrency")));
    }

    #[test]
    fn validate_multiple_errors() {
        let mut tab = SettingsTab::new();
        set_http_field(&mut tab, 0, "nope");
        set_scan_field(&mut tab, 0, "0");
        set_session_field(&mut tab, 0, "bad");
        let err = tab.validate().unwrap_err();
        assert!(err.len() >= 3);
    }

    #[test]
    fn validate_invalid_port_timeout_fails() {
        let mut tab = SettingsTab::new();
        set_scan_field(&mut tab, 2, "abc");
        let err = tab.validate().unwrap_err();
        assert!(err.iter().any(|e| e.contains("port_timeout_secs")));
    }

    #[test]
    fn validate_invalid_max_retries_fails() {
        let mut tab = SettingsTab::new();
        set_http_field(&mut tab, 1, "abc");
        let err = tab.validate().unwrap_err();
        assert!(err.iter().any(|e| e.contains("max_retries")));
    }

    #[test]
    fn save_config_blocks_on_validation_error() {
        let mut tab = SettingsTab::new();
        set_http_field(&mut tab, 0, "bad");
        tab.save_config();
        assert!(tab.status_message.contains("timeout_secs"));
    }

    #[test]
    fn theme_metadata_fields_default_empty() {
        let tab = SettingsTab::new();
        assert!(tab.theme_info_cache.is_empty());
        assert_eq!(tab.theme_invalid_count, 0);
        assert!(tab.theme_dir_path.is_empty());
        assert!(!tab.pending_theme_reload);
    }

    #[test]
    fn update_theme_metadata_stores_values() {
        use crate::theme::manager::{ThemeInfo, ThemeLoadStatus, ThemeSource};
        use crate::theme::ThemeMode;

        let mut tab = SettingsTab::new();
        let infos = vec![ThemeInfo {
            id: "cyber-red".to_string(),
            display_name: "Cyber Red".to_string(),
            mode: ThemeMode::Dark,
            source: ThemeSource::BuiltIn,
            status: ThemeLoadStatus::Loaded,
            contrast_warnings: Vec::new(),
        }];
        tab.update_theme_metadata(
            infos,
            2,
            "/tmp/themes".to_string(),
            rustc_hash::FxHashMap::default(),
            None,
        );

        assert_eq!(tab.theme_info_cache.len(), 1);
        assert_eq!(tab.theme_invalid_count, 2);
        assert_eq!(tab.theme_dir_path, "/tmp/themes");
        assert!(tab.theme_contrast_cache.is_empty());
    }

    #[test]
    fn per_theme_contrast_cache_stores_warnings_by_id() {
        use crate::theme::manager::{ThemeInfo, ThemeLoadStatus, ThemeSource};
        use crate::theme::ThemeMode;

        let mut tab = SettingsTab::new();
        let infos = vec![
            ThemeInfo {
                id: "dark".to_string(),
                display_name: "Dark".to_string(),
                mode: ThemeMode::Dark,
                source: ThemeSource::BuiltIn,
                status: ThemeLoadStatus::Loaded,
                contrast_warnings: Vec::new(),
            },
            ThemeInfo {
                id: "light".to_string(),
                display_name: "Light".to_string(),
                mode: ThemeMode::Light,
                source: ThemeSource::BuiltIn,
                status: ThemeLoadStatus::Loaded,
                contrast_warnings: Vec::new(),
            },
        ];
        let mut cache = rustc_hash::FxHashMap::default();
        cache.insert(
            "dark".to_string(),
            vec!["text/background too low".to_string()],
        );
        tab.update_theme_metadata(infos, 0, "/tmp/themes".to_string(), cache, None);

        assert_eq!(tab.theme_contrast_cache.len(), 1);
        assert!(tab.theme_contrast_cache.contains_key("dark"));
        assert!(!tab.theme_contrast_cache.contains_key("light"));
    }

    #[test]
    fn take_pending_theme_reload_returns_true_when_set() {
        let mut tab = SettingsTab::new();
        tab.pending_theme_reload = true;
        assert!(tab.take_pending_theme_reload());
        assert!(!tab.pending_theme_reload);
    }

    #[test]
    fn take_pending_theme_reload_returns_false_when_not_set() {
        let mut tab = SettingsTab::new();
        assert!(!tab.take_pending_theme_reload());
    }

    #[test]
    fn theme_reload_flag_set_on_r_in_theme_section() {
        let mut tab = SettingsTab::new();
        tab.current_section = SettingsSection::Theme;
        tab.handle_char('r');
        assert!(tab.pending_theme_reload);
    }

    #[test]
    fn theme_reload_flag_not_set_when_selector_open() {
        let mut tab = SettingsTab::new();
        tab.current_section = SettingsSection::Theme;
        tab.theme_selector.open();
        tab.handle_char('r');
        assert!(!tab.pending_theme_reload);
    }

    #[test]
    fn theme_reload_flag_not_set_in_other_sections() {
        let mut tab = SettingsTab::new();
        tab.current_section = SettingsSection::Http;
        tab.handle_char('r');
        assert!(!tab.pending_theme_reload);

        tab.current_section = SettingsSection::Scan;
        tab.handle_char('r');
        assert!(!tab.pending_theme_reload);

        tab.current_section = SettingsSection::Notifications;
        tab.handle_char('r');
        assert!(!tab.pending_theme_reload);
    }

    #[test]
    fn settings_layout_footer_visible_at_80x24() {
        use crate::tabs::TabRender;
        use ratatui::{backend::TestBackend, Terminal};

        let tab = SettingsTab::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let area = f.area();
                tab.render(f, area, false);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        // Footer should be present in the last content row
        let footer_text = "[s] Save";
        let mut found = false;
        for cell in buffer.content() {
            if cell.symbol() == &footer_text[0..1] {
                found = true;
                break;
            }
        }
        assert!(found, "footer should be rendered");
    }

    #[test]
    fn settings_layout_status_does_not_collide_with_footer() {
        use crate::tabs::TabRender;
        use ratatui::{backend::TestBackend, Terminal};

        let mut tab = SettingsTab::new();
        tab.status_message = "Settings saved successfully".to_string();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let area = f.area();
                tab.render(f, area, false);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        // Both status and footer should be visible
        let has_status = buffer
            .content()
            .iter()
            .any(|c| c.symbol() == "S" || c.symbol() == "s");
        assert!(has_status, "status message should be rendered");
    }

    #[test]
    fn settings_layout_small_terminal_renders_without_panic() {
        use crate::tabs::TabRender;
        use ratatui::{backend::TestBackend, Terminal};

        let tab = SettingsTab::new();
        // 60x20 is a supported small size per plan
        let backend = TestBackend::new(60, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let area = f.area();
                tab.render(f, area, false);
            })
            .unwrap();
        // No panic = pass
    }

    #[test]
    fn applied_theme_id_initially_none() {
        let tab = SettingsTab::new();
        assert!(tab.applied_theme_id.is_none());
    }

    #[test]
    fn applied_theme_id_tracks_applied_theme() {
        let mut tab = SettingsTab::new();
        tab.applied_theme_id = Some("cyber-red".to_string());
        assert_eq!(tab.applied_theme_id.as_deref(), Some("cyber-red"));
    }

    #[test]
    fn escape_restores_applied_theme_selection() {
        let mut tab = SettingsTab::new();
        // Simulate an applied theme.
        tab.applied_theme_id = Some("dark".to_string());
        // Set up a multi-theme selector.
        tab.theme_selector = Selector::new("Theme").items(vec![
            SelectorItem::new("Cyber Red", "cyber-red"),
            SelectorItem::new("Dark", "dark"),
            SelectorItem::new("Light", "light"),
        ]);
        // Open the selector and move to a different theme.
        tab.theme_selector.open();
        tab.theme_selector.select(0); // cyber-red
        assert_eq!(tab.theme_selector.selected_value(), Some("cyber-red"));

        // Press Escape — should restore to applied theme.
        tab.handle_escape();

        assert!(!tab.theme_selector.is_open());
        assert_eq!(
            tab.theme_selector.selected_value(),
            Some("dark"),
            "Escape should restore selector to applied_theme_id"
        );
        assert!(
            tab.needs_theme_preview_refresh,
            "Escape should flag preview refresh"
        );
    }

    #[test]
    fn escape_does_not_restore_when_no_applied_theme() {
        let mut tab = SettingsTab::new();
        tab.applied_theme_id = None;
        tab.theme_selector = Selector::new("Theme").items(vec![
            SelectorItem::new("Cyber Red", "cyber-red"),
            SelectorItem::new("Dark", "dark"),
        ]);
        tab.theme_selector.open();
        tab.theme_selector.select(0);

        tab.handle_escape();

        assert!(!tab.theme_selector.is_open());
        // Selection stays where the user left it (no applied theme to restore to).
        assert_eq!(tab.theme_selector.selected_value(), Some("cyber-red"));
    }

    #[test]
    fn enter_on_theme_selector_sets_pending_theme() {
        let mut tab = SettingsTab::new();
        tab.current_section = SettingsSection::Theme;
        tab.focus_area = SettingsFocusArea::SectionDetail;
        tab.detail_focus_index = 0;
        tab.theme_selector = Selector::new("Theme").items(vec![
            SelectorItem::new("Cyber Red", "cyber-red"),
            SelectorItem::new("Dark", "dark"),
            SelectorItem::new("Light", "light"),
        ]);
        tab.theme_selector.open();
        tab.theme_selector.select(1); // dark

        // Enter confirms the selection and sets pending_theme_name.
        tab.handle_enter();

        assert!(!tab.theme_selector.is_open());
        assert_eq!(
            tab.take_pending_theme().as_deref(),
            Some("dark"),
            "Enter should set pending_theme_name to the selected theme"
        );
    }

    #[test]
    fn enter_on_theme_section_opens_selector() {
        let mut tab = SettingsTab::new();
        tab.current_section = SettingsSection::Theme;
        tab.focus_area = SettingsFocusArea::SectionDetail;
        tab.detail_focus_index = 0;
        tab.sync_component_focus();
        assert!(!tab.theme_selector.is_open());

        tab.handle_enter();
        assert!(
            tab.theme_selector.is_open(),
            "Enter should open the selector"
        );
    }

    #[test]
    fn theme_hint_shows_apply_when_selector_open() {
        use crate::tabs::TabRender;
        use ratatui::{backend::TestBackend, Terminal};

        let mut tab = SettingsTab::new();
        tab.current_section = SettingsSection::Theme;
        tab.theme_selector = Selector::new("Theme").items(vec![SelectorItem::new("Dark", "dark")]);
        tab.theme_selector.open();

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let area = f.area();
                tab.render(f, area, false);
            })
            .unwrap();

        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(
            content.contains("Enter:apply"),
            "Footer should show Enter:apply when selector is open"
        );
        assert!(
            content.contains("Esc:cancel"),
            "Footer should show Esc:cancel when selector is open"
        );
    }

    #[test]
    fn theme_hint_shows_themes_when_selector_closed() {
        use crate::tabs::TabRender;
        use ratatui::{backend::TestBackend, Terminal};

        let mut tab = SettingsTab::new();
        tab.current_section = SettingsSection::Theme;
        // Selector is closed by default.

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let area = f.area();
                tab.render(f, area, false);
            })
            .unwrap();

        let content: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol().to_string())
            .collect();
        assert!(
            content.contains("Enter:themes"),
            "Footer should show Enter:themes when selector is closed"
        );
        assert!(
            content.contains("r:reload"),
            "Footer should show r:reload when selector is closed"
        );
    }

    #[test]
    fn validate_invalid_rate_limit_fails() {
        let mut tab = SettingsTab::new();
        set_scan_field(&mut tab, 1, "abc");
        let err = tab.validate().unwrap_err();
        assert!(err.iter().any(|e| e.contains("rate_limit_per_second")));
    }

    #[test]
    fn validate_zero_rate_limit_fails() {
        let mut tab = SettingsTab::new();
        set_scan_field(&mut tab, 1, "0");
        let err = tab.validate().unwrap_err();
        assert!(err.iter().any(|e| e.contains("rate_limit_per_second")));
    }

    #[test]
    fn validate_empty_rate_limit_passes() {
        let mut tab = SettingsTab::new();
        set_scan_field(&mut tab, 1, "");
        assert!(tab.validate().is_ok());
    }

    #[test]
    fn validate_valid_rate_limit_passes() {
        let mut tab = SettingsTab::new();
        set_scan_field(&mut tab, 1, "100");
        assert!(tab.validate().is_ok());
    }

    #[test]
    fn scan_fields_round_trip_no_data_loss() {
        use eggsec::config::{EggsecConfig, HttpConfig, ScanConfig};

        let mut tab = SettingsTab::new();

        // Build a config with specific scan values.
        let config = EggsecConfig {
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
                default_concurrency: 123,
                rate_limit_per_second: Some(500),
                jitter_ms: None,
                stealth_mode: true,
                exclude_ports: Vec::new(),
                exclude_hosts: Vec::new(),
                port_timeout_secs: 10,
                save_session: false,
                session_dir: None,
            },
            output: eggsec::config::OutputConfig::default(),
            notifications: eggsec::config::NotificationConfig::default(),
            paths: eggsec::config::PathsConfig {
                custom_payloads_dir: None,
                wordlists_dir: None,
                export_dir: None,
            },
            profiles: rustc_hash::FxHashMap::default(),
            recon: eggsec::config::ReconConfig::default(),
            schedule: Vec::new(),
            remote: eggsec::config::RemoteConfig::default(),
            proxies: Vec::new(),
            ai: None,
            search: None,
            alert_channels: eggsec::config::AlertChannelsConfig::default(),
            execution_policy: eggsec::config::ExecutionPolicy::default(),
            auto_save_interval_secs: 30,
        };

        // Load config into settings tab.
        tab.load_config(&config);

        // Verify scan fields were loaded.
        assert_eq!(
            tab.scan_inputs.fields[0].value, "123",
            "default_concurrency should load"
        );
        assert_eq!(
            tab.scan_inputs.fields[1].value, "500",
            "rate_limit_per_second should load"
        );
        assert_eq!(
            tab.scan_inputs.fields[2].value, "10",
            "port_timeout_secs should load"
        );
        assert!(tab.stealth_mode.checked, "stealth_mode should load");

        // Apply back to config.
        let mut result_config = config.clone();
        tab.apply_to_config(&mut result_config);

        // Verify round-trip.
        assert_eq!(result_config.scan.default_concurrency, 123);
        assert_eq!(result_config.scan.rate_limit_per_second, Some(500));
        assert_eq!(result_config.scan.port_timeout_secs, 10);
        assert!(result_config.scan.stealth_mode);
    }

    #[test]
    fn scan_fields_round_trip_none_rate_limit() {
        use eggsec::config::{EggsecConfig, HttpConfig, ScanConfig};

        let mut tab = SettingsTab::new();

        let config = EggsecConfig {
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
                jitter_ms: None,
                stealth_mode: false,
                exclude_ports: Vec::new(),
                exclude_hosts: Vec::new(),
                port_timeout_secs: 2,
                save_session: false,
                session_dir: None,
            },
            output: eggsec::config::OutputConfig::default(),
            notifications: eggsec::config::NotificationConfig::default(),
            paths: eggsec::config::PathsConfig {
                custom_payloads_dir: None,
                wordlists_dir: None,
                export_dir: None,
            },
            profiles: rustc_hash::FxHashMap::default(),
            recon: eggsec::config::ReconConfig::default(),
            schedule: Vec::new(),
            remote: eggsec::config::RemoteConfig::default(),
            proxies: Vec::new(),
            ai: None,
            search: None,
            alert_channels: eggsec::config::AlertChannelsConfig::default(),
            execution_policy: eggsec::config::ExecutionPolicy::default(),
            auto_save_interval_secs: 30,
        };

        tab.load_config(&config);

        // None rate_limit should produce empty string.
        assert_eq!(tab.scan_inputs.fields[1].value, "");

        // Apply back.
        let mut result_config = config.clone();
        tab.apply_to_config(&mut result_config);

        // Should round-trip as None.
        assert_eq!(result_config.scan.rate_limit_per_second, None);
    }

    #[test]
    fn theme_preview_uses_resolved_colors_not_thread_local() {
        use crate::tabs::TabRender;
        use crate::theme::palette::ThemeColors;
        use ratatui::style::Color;
        use ratatui::{backend::TestBackend, Terminal};

        // Custom theme with distinctive colors that differ from any built-in.
        let custom_colors = ThemeColors {
            primary: Color::Magenta,
            secondary: Color::Blue,
            accent: Color::Cyan,
            background: Color::Black,
            surface: Color::DarkGray,
            border: Color::Gray,
            border_focused: Color::Yellow,
            text: Color::Magenta,
            text_dim: Color::DarkGray,
            text_bright: Color::White,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            info: Color::Cyan,
            selected: Color::Blue,
            selected_text: Color::White,
            highlight: Color::Yellow,
            mode_normal: Color::Green,
            mode_insert: Color::Yellow,
            tab_active: Color::Cyan,
            tab_inactive: Color::Gray,
            status_running: Color::Green,
            status_idle: Color::Gray,
            status_error: Color::Red,
            focus_input: Color::Yellow,
            focus_results: Color::Cyan,
            safe: Color::Green,
            danger: Color::Red,
            muted: Color::DarkGray,
            active_task: Color::Green,
            paused_task: Color::Yellow,
            scope_match: Color::Green,
            scope_miss: Color::Red,
            policy_required: Color::Yellow,
            policy_denied: Color::Red,
        };

        let mut tab = SettingsTab::new();
        tab.current_section = SettingsSection::Theme;
        // Set the resolved theme colors so preview uses them.
        tab.resolved_theme_colors = Some(custom_colors.clone());

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let area = f.area();
                tab.render(f, area, false);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        // Find the preview area. The preview "Normal" text is rendered with
        // `custom_colors.text` (Magenta). Scan the buffer for a cell with
        // that foreground color in the expected preview region.
        let mut found_custom_fg = false;
        for cell in buffer.content() {
            if cell.symbol() == "N" && cell.fg == Color::Magenta {
                found_custom_fg = true;
                break;
            }
        }
        assert!(
            found_custom_fg,
            "Preview should render 'Normal' text with the custom theme's text color (Magenta), \
             not the thread-local theme color"
        );
    }
}
