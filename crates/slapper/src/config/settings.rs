use crate::constants::{self, cache};
use crate::proxy::ProxyType;
use crate::types::SensitiveString;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

/// Directory paths for payloads, plugins, and wordlists.
///
/// Uses `#[serde(flatten)]` to maintain backward compatibility with
/// existing config files that have these fields at the top level.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PathsConfig {
    #[serde(default)]
    pub custom_payloads_dir: Option<PathBuf>,

    #[serde(default)]
    pub plugins_dir: Option<PathBuf>,

    #[serde(default)]
    pub wordlists_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SlapperConfig {
    #[serde(default)]
    pub http: HttpConfig,

    #[serde(default)]
    pub scan: ScanConfig,

    #[serde(default)]
    pub output: OutputConfig,

    #[serde(default)]
    pub notifications: NotificationConfig,

    #[serde(default)]
    pub profiles: HashMap<String, ScanProfile>,

    /// Directory paths configuration.
    /// Flattened for backward compatibility with existing config files.
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

fn default_remote_port() -> u16 {
    constants::DEFAULT_REMOTE_PORT
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
    #[serde(default = "default_proxy_weight")]
    pub weight: u32,
    #[serde(default = "default_proxy_priority")]
    pub priority: u8,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_proxy_weight() -> u32 {
    1
}
fn default_proxy_priority() -> u8 {
    0
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReconConfig {
    #[serde(default)]
    pub subdomain_wordlist: Option<PathBuf>,

    #[serde(default = "default_dns_concurrency")]
    pub dns_concurrency: usize,

    #[serde(default)]
    pub screenshot_output_dir: Option<PathBuf>,

    #[serde(default)]
    pub apis: ApiConfig,

    #[serde(default)]
    pub cache: CacheConfig,
}

fn default_dns_concurrency() -> usize {
    20
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheConfig {
    #[serde(default = "default_cache_enabled")]
    pub enabled: bool,

    #[serde(default = "default_cache_ttl_secs")]
    pub ttl_secs: u64,

    #[serde(default)]
    pub cache_dir: Option<PathBuf>,

    #[serde(default = "default_max_cache_entries")]
    pub max_entries: usize,
}

fn default_cache_enabled() -> bool {
    true
}

fn default_cache_ttl_secs() -> u64 {
    cache::DEFAULT_TTL_SECS
}

fn default_max_cache_entries() -> usize {
    cache::DEFAULT_MAX_ENTRIES
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiConfig {
    #[serde(default)]
    pub threatstream: ApiKeyConfig,

    #[serde(default)]
    pub virustotal: ApiKeyConfig,

    #[serde(default)]
    pub alienvault: ApiKeyConfig,

    #[serde(default)]
    pub crtsh: ApiKeyConfig,

    #[serde(default)]
    pub securitytrails: ApiKeyConfig,

    #[serde(default)]
    pub shodan: ApiKeyConfig,

    #[serde(default)]
    pub passivetotal: ApiKeyConfig,

    #[serde(default, rename = "WaybackMachine")]
    pub wayback_machine: WaybackConfig,

    #[serde(default)]
    pub nvd: NvdConfig,

    #[serde(default)]
    pub ipapi: IpApiConfig,

    #[serde(default)]
    pub maxmind: MaxMindConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NvdConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub api_key: Option<SensitiveString>,

    #[serde(default = "default_nvd_rate_limit")]
    pub rate_limit_delay_ms: u64,
}

fn default_nvd_rate_limit() -> u64 {
    6000
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IpApiConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub use_premium: bool,

    #[serde(default)]
    pub api_key: Option<SensitiveString>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MaxMindConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub account_id: Option<u32>,

    #[serde(default)]
    pub license_key: Option<SensitiveString>,

    #[serde(default = "default_maxmind_edition_ids")]
    pub edition_ids: Vec<String>,

    #[serde(default)]
    pub auto_update: bool,

    #[serde(default = "default_maxmind_data_dir")]
    pub data_dir: String,

    #[serde(default)]
    pub use_geoipupdate_binary: bool,
}

fn default_maxmind_edition_ids() -> Vec<String> {
    vec!["GeoLite2-City".to_string(), "GeoLite2-Country".to_string()]
}

fn default_maxmind_data_dir() -> String {
    "~/.slapper/geoip".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiKeyConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub api_key: Option<SensitiveString>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WaybackConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub api_key: Option<SensitiveString>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    #[serde(default)]
    pub max_retries: u32,

    #[serde(default = "default_retry_delay")]
    pub retry_delay_ms: u64,

    #[serde(default)]
    pub verify_tls: bool,

    #[serde(default)]
    pub follow_redirects: bool,

    #[serde(default = "default_max_redirects")]
    pub max_redirects: usize,

    #[serde(default)]
    pub default_headers: HashMap<String, String>,

    #[serde(default)]
    pub default_user_agent: Option<String>,

    #[serde(default)]
    pub proxy: Option<String>,

    #[serde(default)]
    pub proxy_auth: Option<SensitiveString>,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            timeout_secs: default_timeout(),
            max_retries: 3,
            retry_delay_ms: default_retry_delay(),
            verify_tls: true,
            follow_redirects: true,
            max_redirects: default_max_redirects(),
            default_headers: HashMap::new(),
            default_user_agent: None,
            proxy: None,
            proxy_auth: None,
        }
    }
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
    pub format: OutputFormat,

    #[serde(default)]
    pub verbosity: Verbosity,

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
            format: OutputFormat::Pretty,
            verbosity: Verbosity::Normal,
            color: true,
            progress_bars: true,
            save_results: false,
            results_dir: None,
            include_timestamp: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NotificationConfig {
    #[serde(default)]
    pub webhooks: Vec<WebhookConfig>,

    #[serde(default)]
    pub slack_webhook: Option<String>,

    #[serde(default)]
    pub discord_webhook: Option<String>,

    #[serde(default)]
    pub teams_webhook: Option<String>,

    #[serde(default)]
    pub notify_on_complete: bool,

    #[serde(default)]
    pub notify_on_findings: bool,

    #[serde(default)]
    pub min_severity_for_notify: Severity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub secret: Option<SensitiveString>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub events: Vec<WebhookEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WebhookEvent {
    ScanStart,
    ScanComplete,
    Finding,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProfile {
    pub name: String,
    pub stages: Vec<String>,
    #[serde(default)]
    pub concurrency: Option<usize>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub payload_types: Vec<String>,
}

use crate::cli::OutputFormat;
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum Verbosity {
    Quiet,
    #[default]
    Normal,
    Verbose,
    Debug,
}

pub use crate::types::Severity;

#[derive(Debug, Error)]
pub enum ConfigValidationError {
    #[error("Invalid log level '{0}': must be one of trace, debug, info, warn, error")]
    InvalidLogLevel(String),
    #[error("Invalid proxy URL '{0}': must be a valid URL (http, https, socks5)")]
    InvalidProxyUrl(String),
    #[error("Invalid timeout value {0}: must be positive")]
    InvalidTimeout(u64),
    #[error("Invalid concurrency value {0}: must be at least 1")]
    InvalidConcurrency(usize),
    #[error("Invalid rate limit value {0}: must be at least 1")]
    InvalidRateLimit(u32),
}

impl SlapperConfig {
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        if let Some(ref proxy) = self.http.proxy {
            if !proxy.starts_with("http://")
                && !proxy.starts_with("https://")
                && !proxy.starts_with("socks5://")
            {
                return Err(ConfigValidationError::InvalidProxyUrl(proxy.clone()));
            }
        }

        if self.http.timeout_secs == 0 {
            return Err(ConfigValidationError::InvalidTimeout(
                self.http.timeout_secs,
            ));
        }
        if self.http.max_retries > 10 {
            return Err(ConfigValidationError::InvalidTimeout(
                self.http.max_retries as u64,
            ));
        }

        if self.scan.default_concurrency == 0 {
            return Err(ConfigValidationError::InvalidConcurrency(
                self.scan.default_concurrency,
            ));
        }

        if let Some(rate_limit) = self.scan.rate_limit_per_second {
            if rate_limit == 0 {
                return Err(ConfigValidationError::InvalidRateLimit(rate_limit));
            }
        }

        for proxy in &self.proxies {
            if proxy.weight == 0 {
                return Err(ConfigValidationError::InvalidConcurrency(
                    proxy.weight as usize,
                ));
            }
        }

        Ok(())
    }
}

fn default_timeout() -> u64 {
    30
}
fn default_retry_delay() -> u64 {
    1000
}
fn default_max_redirects() -> usize {
    10
}
fn default_concurrency() -> usize {
    10
}
fn default_port_timeout() -> u64 {
    2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub api_url: Option<String>,
    pub api_key: Option<SensitiveString>,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            api_url: None,
            api_key: None,
            model: Some("gpt-4".to_string()),
            max_tokens: Some(4096),
            temperature: Some(0.7),
        }
    }
}
