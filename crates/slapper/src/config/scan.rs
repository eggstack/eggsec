use crate::config::settings::ConfigError;
use crate::constants::http;
use crate::types::SensitiveString;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn default_concurrency() -> usize {
    http::DEFAULT_CONCURRENCY
}

fn default_port_timeout() -> u64 {
    2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    #[serde(default = "default_concurrency")]
    pub default_concurrency: usize,

    #[serde(default)]
    pub rate_limit_per_second: Option<u32>,

    #[serde(default)]
    pub jitter_ms: Option<(u64, u64)>,

    #[serde(default)]
    pub stealth_mode: bool,

    #[serde(default)]
    pub exclude_ports: Vec<u16>,

    #[serde(default)]
    pub exclude_hosts: Vec<String>,

    #[serde(default = "default_port_timeout")]
    pub port_timeout_secs: u64,

    #[serde(default)]
    pub save_session: bool,

    #[serde(default)]
    pub session_dir: Option<PathBuf>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            default_concurrency: default_concurrency(),
            rate_limit_per_second: None,
            jitter_ms: None,
            stealth_mode: false,
            exclude_ports: vec![],
            exclude_hosts: vec![],
            port_timeout_secs: default_port_timeout(),
            save_session: false,
            session_dir: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    #[serde(default)]
    pub format: crate::types::OutputFormat,

    #[serde(default)]
    pub verbosity: super::http::Verbosity,

    #[serde(default)]
    pub color: bool,

    #[serde(default)]
    pub progress_bars: bool,

    #[serde(default)]
    pub save_results: bool,

    #[serde(default)]
    pub results_dir: Option<PathBuf>,

    #[serde(default)]
    pub include_timestamp: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            format: crate::types::OutputFormat::Pretty,
            verbosity: super::http::Verbosity::Normal,
            color: true,
            progress_bars: true,
            save_results: false,
            results_dir: None,
            include_timestamp: true,
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    #[serde(default)]
    pub webhooks: Vec<WebhookConfig>,

    #[serde(default)]
    pub slack_webhook: Option<String>,

    #[serde(default)]
    pub discord_webhook: Option<String>,

    #[serde(default)]
    pub teams_webhook: Option<String>,

    #[serde(default = "default_true")]
    pub notify_on_complete: bool,

    #[serde(default = "default_true")]
    pub notify_on_findings: bool,

    #[serde(default = "default_true")]
    pub notify_on_error: bool,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            webhooks: Vec::new(),
            slack_webhook: None,
            discord_webhook: None,
            teams_webhook: None,
            notify_on_complete: true,
            notify_on_findings: true,
            notify_on_error: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub url: String,

    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub headers: FxHashMap<String, String>,

    #[serde(default)]
    pub events: Vec<WebhookEvent>,

    #[serde(default)]
    pub secret: Option<SensitiveString>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEvent {
    #[serde(alias = "ScanStart")]
    ScanStarted,
    #[serde(alias = "ScanComplete")]
    ScanComplete,
    #[serde(alias = "Finding")]
    FindingDetected,
    #[serde(alias = "Error")]
    ScanError,
}

impl WebhookConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.url.is_empty() {
            return Err(ConfigError::Validation(
                "webhook.url cannot be empty".to_string(),
            ));
        }
        if !self.url.starts_with("http://") && !self.url.starts_with("https://") {
            return Err(ConfigError::Validation(
                "webhook.url must start with http:// or https://".to_string(),
            ));
        }
        for header_key in self.headers.keys() {
            if header_key.is_empty() {
                return Err(ConfigError::Validation(
                    "webhook.headers keys cannot be empty".to_string(),
                ));
            }
        }
        if let Some(ref name) = self.name {
            if name.is_empty() {
                return Err(ConfigError::Validation(
                    "webhook.name cannot be empty when specified".to_string(),
                ));
            }
        }
        Ok(())
    }
}

/// Config-driven scan profile. Users can define custom profiles in `SlapperConfig.profiles`.
///
/// TODO(reframe-pass3): Planned defense-lab config profiles:
/// - `defense-lab`: local/private-scope controlled probe suite
/// - `synvoid-local`: localhost/container/private lab defaults
/// - `waf-regression`: WAF payload and evasion-resistance regression
/// - `protocol-edge`: malformed protocol, TCP/TLS/HTTP edge behavior
/// - `nse-safe`: sandboxed safe/default/version/discovery NSE scripts only
///
/// Defense-lab profiles should validate scope at load time and reject public targets.
/// Stress and packet features must require explicit feature gates.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanProfile {
    pub name: String,
    pub http: Option<super::http::HttpConfig>,
    pub scan: Option<ScanConfig>,
    pub fuzz: Option<FuzzProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FuzzProfile {
    pub payload_types: Vec<String>,
    pub concurrency: Option<usize>,
    pub timeout_ms: Option<u64>,
}

impl FuzzProfile {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if let Some(concurrency) = self.concurrency {
            if concurrency == 0 {
                return Err(ConfigError::Validation(
                    "fuzz.concurrency cannot be 0 when set".to_string(),
                ));
            }
            if concurrency > 1000 {
                return Err(ConfigError::Validation(
                    "fuzz.concurrency cannot exceed 1000".to_string(),
                ));
            }
        }
        if let Some(timeout) = self.timeout_ms {
            if timeout == 0 {
                return Err(ConfigError::Validation(
                    "fuzz.timeout_ms cannot be 0 when set".to_string(),
                ));
            }
        }
        for pt in &self.payload_types {
            if pt.is_empty() {
                return Err(ConfigError::Validation(
                    "fuzz.payload_types cannot contain empty strings".to_string(),
                ));
            }
        }
        Ok(())
    }
}
