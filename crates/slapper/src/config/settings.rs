pub use super::api::ApiConfig;
pub use super::http::HttpConfig;
pub use super::scan::{NotificationConfig, OutputConfig, ScanConfig, ScanProfile};

use crate::constants::cache as cache_constants;
use crate::constants::http;
use crate::proxy::ProxyType;
use crate::types::SensitiveString;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

use crate::constants;

fn default_ttl() -> u64 {
    cache_constants::DEFAULT_TTL_SECS
}

fn default_remote_port() -> u16 {
    constants::DEFAULT_REMOTE_PORT
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
    pub api_key: SensitiveString,
    pub base_url: Option<String>,
    pub max_tokens: Option<usize>,
}

impl Default for ReconConfig {
    fn default() -> Self {
        Self {
            dns_concurrency: default_concurrency(),
            apis: ApiConfig::default(),
        }
    }
}

impl SlapperConfig {
    pub fn load(path: &PathBuf) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path).map_err(|e| ConfigError::Io(e.to_string()))?;
        let config: SlapperConfig =
            toml::from_str(&contents).map_err(|e| ConfigError::Parse(e.to_string()))?;
        Ok(config)
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), ConfigError> {
        let contents =
            toml::to_string_pretty(self).map_err(|e| ConfigError::Serialize(e.to_string()))?;
        std::fs::write(path, contents).map_err(|e| ConfigError::Io(e.to_string()))?;
        Ok(())
    }

    pub fn default_path() -> Option<PathBuf> {
        ProjectDirs::from("tools", "slapper", "slapper")
            .map(|p: ProjectDirs| p.config_dir().join("config.toml"))
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.http.timeout_secs > 300 {
            return Err(ConfigError::Validation(
                "timeout_secs cannot exceed 300".to_string(),
            ));
        }
        if self.http.max_retries > 10 {
            return Err(ConfigError::Validation(
                "max_retries cannot exceed 10".to_string(),
            ));
        }
        if self.http.proxy_auth.is_some() && self.http.proxy.is_none() {
            return Err(ConfigError::Validation(
                "proxy_auth requires proxy to be set".to_string(),
            ));
        }
        if self.scan.default_concurrency > 1000 {
            return Err(ConfigError::Validation(
                "default_concurrency cannot exceed 1000".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(String),

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
