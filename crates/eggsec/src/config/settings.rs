pub use super::api::ApiConfig;
pub use super::http::HttpConfig;
pub use super::scan::{NotificationConfig, OutputConfig, ScanConfig, ScanProfile};

use crate::config::policy::ExecutionPolicy;
use crate::constants::cache as cache_constants;
use crate::constants::http;
pub use crate::constants::DEFAULT_REMOTE_PORT;
use crate::proxy::ProxyType;
use crate::types::SensitiveString;
use directories::ProjectDirs;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Alert channel configuration for EggsecConfig
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AlertChannelsConfig {
    #[serde(default)]
    pub channels: FxHashMap<String, AlertChannelConfigEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AlertChannelConfigEntry {
    Webhook(WebhookConfigEntry),
    Email(EmailConfigEntry),
    Slack(SlackConfigEntry),
    PagerDuty(PagerDutyConfigEntry),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfigEntry {
    pub url: String,
    pub secret: Option<SensitiveString>,
    #[serde(default)]
    pub headers: FxHashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfigEntry {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub from: String,
    pub to: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfigEntry {
    pub webhook_url: String,
    pub channel: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagerDutyConfigEntry {
    pub routing_key: SensitiveString,
    pub severity: String,
}

fn default_ttl() -> u64 {
    cache_constants::DEFAULT_TTL_SECS
}

fn default_remote_port() -> u16 {
    DEFAULT_REMOTE_PORT
}

fn default_concurrency() -> usize {
    http::DEFAULT_CONCURRENCY
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    #[serde(default = "default_ttl")]
    pub ttl_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PathsConfig {
    #[serde(default)]
    pub custom_payloads_dir: Option<PathBuf>,

    #[serde(default)]
    pub wordlists_dir: Option<PathBuf>,

    #[serde(default)]
    pub export_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EggsecConfig {
    #[serde(default)]
    pub http: HttpConfig,

    #[serde(default)]
    pub scan: ScanConfig,

    #[serde(default)]
    pub output: OutputConfig,

    #[serde(default)]
    pub notifications: NotificationConfig,

    #[serde(default)]
    pub profiles: FxHashMap<String, ScanProfile>,

    #[serde(default, flatten)]
    pub paths: PathsConfig,

    #[serde(default)]
    pub recon: ReconConfig,

    #[serde(default)]
    pub schedule: Vec<ScheduledScan>,

    #[serde(default)]
    pub remote: RemoteConfig,

    #[serde(default)]
    pub proxies: Vec<ProxyConfigEntry>,

    #[serde(default)]
    pub ai: Option<AiConfig>,

    #[serde(default)]
    pub search: Option<SearchConfig>,

    #[serde(default)]
    pub alert_channels: AlertChannelsConfig,

    #[serde(default)]
    pub execution_policy: ExecutionPolicy,

    #[cfg(feature = "external-integrations")]
    #[serde(default)]
    pub integrations: crate::integrations::IntegrationConfig,

    #[serde(default = "default_auto_save_interval")]
    pub auto_save_interval_secs: u64,
}

fn default_auto_save_interval() -> u64 {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RemoteConfig {
    #[serde(default)]
    pub psk: Option<SensitiveString>,

    #[serde(default = "default_remote_port")]
    pub default_port: u16,

    #[serde(default)]
    pub allowed_workers: Vec<AllowedWorker>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowedWorker {
    pub host: String,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledScan {
    pub schedule: String,
    pub target: String,
    pub scan_type: String,
    pub output: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfigEntry {
    pub proxy_type: ProxyType,
    pub address: String,
    pub port: u16,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<SensitiveString>,
    #[serde(default)]
    pub local_addr: Option<String>,
    #[serde(default)]
    pub weight: Option<u32>,
    #[serde(default)]
    pub priority: Option<u32>,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconConfig {
    #[serde(default = "default_concurrency")]
    pub dns_concurrency: usize,

    #[serde(default)]
    pub apis: ApiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: String,
    pub model: Option<String>,
    #[serde(default)]
    pub api_key: Option<SensitiveString>,
    pub base_url: Option<String>,
    pub max_tokens: Option<usize>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default = "default_max_payloads")]
    pub max_payloads: usize,
    #[serde(default = "default_max_bypasses")]
    pub max_bypasses: usize,
}

fn default_max_payloads() -> usize {
    50
}

fn default_max_bypasses() -> usize {
    10
}

fn default_search_cache_ttl() -> u64 {
    3600
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub searxng_url: Option<String>,

    #[serde(default)]
    pub engines: Vec<String>,

    #[serde(default = "default_search_cache_ttl")]
    pub cache_ttl_seconds: u64,
}

impl SearchConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if let Some(ref url) = self.searxng_url {
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(ConfigError::Validation(
                    "search.searxng_url must start with http:// or https://".to_string(),
                ));
            }
        }
        if self.cache_ttl_seconds == 0 {
            return Err(ConfigError::Validation(
                "search.cache_ttl_seconds cannot be 0".to_string(),
            ));
        }
        if self.cache_ttl_seconds > 86400 {
            return Err(ConfigError::Validation(
                "search.cache_ttl_seconds cannot exceed 86400 (24 hours)".to_string(),
            ));
        }
        Ok(())
    }
}

impl ScheduledScan {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.schedule.is_empty() {
            return Err(ConfigError::Validation(
                "schedule cron expression cannot be empty".to_string(),
            ));
        }
        if self.target.is_empty() {
            return Err(ConfigError::Validation(
                "schedule.target cannot be empty".to_string(),
            ));
        }
        if self.scan_type.is_empty() {
            return Err(ConfigError::Validation(
                "schedule.scan_type cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}

impl ProxyConfigEntry {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.address.is_empty() {
            return Err(ConfigError::Validation(
                "proxy.address cannot be empty".to_string(),
            ));
        }
        if self.port == 0 {
            return Err(ConfigError::Validation(format!(
                "proxy.port {} is invalid",
                self.port
            )));
        }
        if let Some(ref username) = self.username {
            if username.is_empty() {
                return Err(ConfigError::Validation(
                    "proxy.username cannot be empty when specified".to_string(),
                ));
            }
            if self.password.is_none() {
                return Err(ConfigError::Validation(
                    "proxy.password is required when proxy.username is set".to_string(),
                ));
            }
        }
        if let Some(ref local_addr) = self.local_addr {
            if local_addr.is_empty() {
                return Err(ConfigError::Validation(
                    "proxy.local_addr cannot be empty when specified".to_string(),
                ));
            }
            if let Err(e) = local_addr.parse::<IpAddr>() {
                return Err(ConfigError::Validation(format!(
                    "proxy.local_addr '{}' is not a valid IP address: {}",
                    local_addr, e
                )));
            }
        }
        if let Some(weight) = self.weight {
            if weight == 0 {
                return Err(ConfigError::Validation(
                    "proxy.weight cannot be 0".to_string(),
                ));
            }
        }
        if let Some(priority) = self.priority {
            if priority == 0 {
                return Err(ConfigError::Validation(
                    "proxy.priority cannot be 0".to_string(),
                ));
            }
        }
        Ok(())
    }
}

impl HttpConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.timeout_secs == 0 {
            return Err(ConfigError::Validation(
                "http.timeout_secs cannot be 0".to_string(),
            ));
        }
        if self.timeout_secs > 300 {
            return Err(ConfigError::Validation(
                "http.timeout_secs cannot exceed 300".to_string(),
            ));
        }
        if self.max_retries > 10 {
            return Err(ConfigError::Validation(
                "http.max_retries cannot exceed 10".to_string(),
            ));
        }
        if self.retry_delay_ms == 0 {
            return Err(ConfigError::Validation(
                "http.retry_delay_ms cannot be 0".to_string(),
            ));
        }
        if self.max_redirects > 50 {
            return Err(ConfigError::Validation(
                "http.max_redirects cannot exceed 50".to_string(),
            ));
        }
        if self.proxy_auth.is_some() && self.proxy.is_none() {
            return Err(ConfigError::Validation(
                "http.proxy_auth requires http.proxy to be set".to_string(),
            ));
        }
        if let Some(ref proxy) = self.proxy {
            if !proxy.starts_with("http://")
                && !proxy.starts_with("https://")
                && !proxy.starts_with("socks5://")
                && !proxy.starts_with("socks4://")
            {
                return Err(ConfigError::Validation(
                    "http.proxy must start with http://, https://, socks5://, or socks4://"
                        .to_string(),
                ));
            }
        }
        Ok(())
    }
}

impl ScanConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.default_concurrency == 0 {
            return Err(ConfigError::Validation(
                "scan.default_concurrency cannot be 0".to_string(),
            ));
        }
        if self.default_concurrency > 1000 {
            return Err(ConfigError::Validation(
                "scan.default_concurrency cannot exceed 1000".to_string(),
            ));
        }
        if self.port_timeout_secs == 0 {
            return Err(ConfigError::Validation(
                "scan.port_timeout_secs cannot be 0".to_string(),
            ));
        }
        if self.port_timeout_secs > 60 {
            return Err(ConfigError::Validation(
                "scan.port_timeout_secs cannot exceed 60".to_string(),
            ));
        }
        if let Some(rate_limit) = self.rate_limit_per_second {
            if rate_limit == 0 {
                return Err(ConfigError::Validation(
                    "scan.rate_limit_per_second cannot be 0 when set".to_string(),
                ));
            }
            if rate_limit > 100000 {
                return Err(ConfigError::Validation(
                    "scan.rate_limit_per_second cannot exceed 100000".to_string(),
                ));
            }
        }
        for port in &self.exclude_ports {
            if *port == 0 {
                return Err(ConfigError::Validation(format!(
                    "scan.exclude_ports contains invalid port {}",
                    port
                )));
            }
        }
        for host in &self.exclude_hosts {
            if host.is_empty() {
                return Err(ConfigError::Validation(
                    "scan.exclude_hosts cannot contain empty strings".to_string(),
                ));
            }
        }
        Ok(())
    }
}

fn validate_dir_path(path: &Path, field_name: &str) -> Result<(), ConfigError> {
    if !path.exists() {
        return Err(ConfigError::Validation(format!(
            "{field_name} path '{}' does not exist",
            path.display()
        )));
    }
    if !path.is_dir() {
        return Err(ConfigError::Validation(format!(
            "{field_name} path '{}' is not a directory",
            path.display()
        )));
    }
    Ok(())
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            model: Some("gpt-4".to_string()),
            api_key: None,
            base_url: None,
            max_tokens: Some(4096),
            temperature: Some(0.7),
            max_payloads: 50,
            max_bypasses: 10,
        }
    }
}

impl AiConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.provider.is_empty() {
            return Err(ConfigError::Validation(
                "ai.provider cannot be empty".to_string(),
            ));
        }
        if let Some(url) = &self.base_url {
            if url.is_empty() {
                return Err(ConfigError::Validation(
                    "ai.base_url cannot be empty when specified".to_string(),
                ));
            }
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(ConfigError::Validation(
                    "ai.base_url must start with http:// or https://".to_string(),
                ));
            }
        }
        if let Some(tokens) = self.max_tokens {
            if tokens == 0 {
                return Err(ConfigError::Validation(
                    "ai.max_tokens cannot be 0 when specified".to_string(),
                ));
            }
            if tokens > 128000 {
                return Err(ConfigError::Validation(
                    "ai.max_tokens cannot exceed 128000".to_string(),
                ));
            }
        }
        if let Some(temp) = self.temperature {
            if !(0.0..=2.0).contains(&temp) {
                return Err(ConfigError::Validation(
                    "ai.temperature must be between 0.0 and 2.0".to_string(),
                ));
            }
        }
        Ok(())
    }
}

impl Default for ReconConfig {
    fn default() -> Self {
        Self {
            dns_concurrency: default_concurrency(),
            apis: ApiConfig::default(),
        }
    }
}

impl EggsecConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path).map_err(ConfigError::Io)?;
        let config: EggsecConfig =
            toml::from_str(&contents).map_err(|e| ConfigError::Parse(e.to_string()))?;
        Ok(config)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ConfigError> {
        let contents =
            toml::to_string_pretty(self).map_err(|e| ConfigError::Serialize(e.to_string()))?;
        std::fs::write(path, contents).map_err(ConfigError::Io)?;
        Ok(())
    }

    pub fn default_path() -> Option<PathBuf> {
        ProjectDirs::from(
            crate::constants::PROJECT_QUALIFIER,
            "",
            crate::constants::PROJECT_NAME,
        )
        .map(|p: ProjectDirs| p.config_dir().join(super::loader::DEFAULT_CONFIG_NAME))
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        self.http.validate()?;
        self.scan.validate()?;
        if self.recon.dns_concurrency == 0 {
            return Err(ConfigError::Validation(
                "dns_concurrency cannot be 0".to_string(),
            ));
        }
        if self.recon.dns_concurrency > 100 {
            return Err(ConfigError::Validation(
                "dns_concurrency cannot exceed 100".to_string(),
            ));
        }
        for scan in &self.schedule {
            scan.validate()?;
        }
        for proxy in &self.proxies {
            proxy.validate()?;
        }
        if let Some(ref ai) = self.ai {
            ai.validate()?;
        }
        if let Some(ref search) = self.search {
            search.validate()?;
        }
        for webhook in &self.notifications.webhooks {
            webhook.validate()?;
        }
        for (name, profile) in &self.profiles {
            if name.is_empty() {
                return Err(ConfigError::Validation(
                    "profile names cannot be empty".to_string(),
                ));
            }
            if let Some(ref http) = profile.http {
                http.validate()?;
            }
            if let Some(ref scan) = profile.scan {
                scan.validate()?;
            }
            if let Some(ref fuzz) = profile.fuzz {
                fuzz.validate()?;
            }
        }
        if let Some(ref paths) = self.paths.custom_payloads_dir {
            validate_dir_path(paths, "custom_payloads_dir")?;
        }
        if let Some(ref paths) = self.paths.wordlists_dir {
            validate_dir_path(paths, "wordlists_dir")?;
        }
        if let Some(ref remote) = self.remote.psk {
            let psk = remote.expose_secret();
            if psk.len() < 16 {
                return Err(ConfigError::Validation(
                    "remote.psk must be at least 16 characters".to_string(),
                ));
            }
        }
        for worker in &self.remote.allowed_workers {
            if worker.host.is_empty() {
                return Err(ConfigError::Validation(
                    "allowed_workers.host cannot be empty".to_string(),
                ));
            }
            if let Some(port) = worker.port {
                if port == 0 {
                    return Err(ConfigError::Validation(format!(
                        "allowed_workers.port {} is invalid",
                        port
                    )));
                }
            }
        }
        if self.remote.default_port == 0 && self.remote.psk.is_some() {
            return Err(ConfigError::Validation(
                "remote.default_port cannot be 0 when PSK is configured".to_string(),
            ));
        }
        if !self.alert_channels.channels.is_empty() {
            for (name, channel) in &self.alert_channels.channels {
                match channel {
                    AlertChannelConfigEntry::Webhook(webhook) => {
                        if webhook.url.is_empty() {
                            return Err(ConfigError::Validation(format!(
                                "alert_channels.'{}'.url cannot be empty",
                                name
                            )));
                        }
                        if !webhook.url.starts_with("http://")
                            && !webhook.url.starts_with("https://")
                        {
                            return Err(ConfigError::Validation(format!(
                                "alert_channels.'{}'.url must start with http:// or https://",
                                name
                            )));
                        }
                    }
                    AlertChannelConfigEntry::Email(email) => {
                        if email.smtp_host.is_empty() {
                            return Err(ConfigError::Validation(format!(
                                "alert_channels.'{}'.smtp_host cannot be empty",
                                name
                            )));
                        }
                        if email.smtp_port == 0 {
                            return Err(ConfigError::Validation(format!(
                                "alert_channels.'{}'.smtp_port cannot be 0",
                                name
                            )));
                        }
                        if email.from.is_empty() {
                            return Err(ConfigError::Validation(format!(
                                "alert_channels.'{}'.from cannot be empty",
                                name
                            )));
                        }
                        if email.to.is_empty() {
                            return Err(ConfigError::Validation(format!(
                                "alert_channels.'{}'.to cannot be empty",
                                name
                            )));
                        }
                    }
                    AlertChannelConfigEntry::Slack(slack) => {
                        if slack.webhook_url.is_empty() {
                            return Err(ConfigError::Validation(format!(
                                "alert_channels.'{}'.webhook_url cannot be empty",
                                name
                            )));
                        }
                        if !slack.webhook_url.starts_with("http://")
                            && !slack.webhook_url.starts_with("https://")
                        {
                            return Err(ConfigError::Validation(format!(
                                "alert_channels.'{}'.webhook_url must start with http:// or https://",
                                name
                            )));
                        }
                    }
                    AlertChannelConfigEntry::PagerDuty(pd) => {
                        if pd.routing_key.expose_secret().is_empty() {
                            return Err(ConfigError::Validation(format!(
                                "alert_channels.'{}'.routing_key cannot be empty",
                                name
                            )));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[source] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Serialization error: {0}")]
    Serialize(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

impl From<toml::de::Error> for ConfigError {
    fn from(e: toml::de::Error) -> Self {
        ConfigError::Parse(e.to_string())
    }
}

impl From<toml::ser::Error> for ConfigError {
    fn from(e: toml::ser::Error) -> Self {
        ConfigError::Serialize(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::EggsecConfig;
    use crate::config::loader::DEFAULT_CONFIG_NAME;

    #[test]
    fn test_eggsec_config_default_path_uses_default_config_name() {
        let path = EggsecConfig::default_path().expect("default path should be available");
        assert!(path.to_string_lossy().ends_with(DEFAULT_CONFIG_NAME));
    }
}
