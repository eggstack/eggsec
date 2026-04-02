use crate::constants::http;
use crate::types::SensitiveString;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_timeout() -> u64 {
    http::DEFAULT_TIMEOUT_SECS
}

fn default_retry_delay() -> u64 {
    1000
}

fn default_max_redirects() -> usize {
    http::DEFAULT_MAX_REDIRECTS as usize
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum OutputFormat {
    #[default]
    Pretty,
    Json,
    Csv,
    Sarif,
    Junit,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum Verbosity {
    #[default]
    Normal,
    Quiet,
    Verbose,
    Debug,
}
