use crate::constants::{PROJECT_NAME, PROJECT_QUALIFIER};
use crate::types::SensitiveString;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub fn default_data_dir() -> PathBuf {
    ProjectDirs::from(PROJECT_QUALIFIER, "", PROJECT_NAME)
        .map(|p: ProjectDirs| p.config_dir().join("maxmind"))
        .unwrap_or_else(|| PathBuf::from(".").join("maxmind"))
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiConfig {
    pub virustotal: ApiKeyConfig,
    pub alienvault: ApiKeyConfig,
    pub shodan: ApiKeyConfig,
    pub ipapi: IpApiConfig,
    pub maxmind: MaxMindConfig,
    pub wayback_machine: WaybackConfig,
    pub nvd: NvdConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NvdConfig {
    #[serde(default)]
    pub api_key: Option<SensitiveString>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IpApiConfig {
    #[serde(default)]
    pub enabled: bool,

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

    #[serde(default)]
    pub edition_ids: Vec<String>,

    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,

    #[serde(default)]
    pub auto_update: bool,
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
