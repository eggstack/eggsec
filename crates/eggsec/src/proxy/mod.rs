// Adapter layer: re-exports from eggsec-web-proxy domain crate.
// All domain logic lives in the domain crate. This module provides
// backward-compatible paths for TUI, CLI, and other consumers.

use serde::{Deserialize, Serialize};

#[cfg(feature = "web-proxy")]
pub use eggsec_web_proxy::intercept;
#[cfg(feature = "web-proxy")]
pub use eggsec_web_proxy::*;
#[cfg(feature = "web-proxy")]
pub use eggsec_web_proxy::{
    HealthCheckConfig, HealthChecker, ProxiedConnection, ProxyConfig, ProxyEntry, ProxyHealth,
    ProxyManager, ProxyPool, ProxyRotator, ProxyType,
};

// Stub types when web-proxy feature is disabled
#[cfg(not(feature = "web-proxy"))]
pub mod intercept {
    pub mod types {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, Default, Serialize, Deserialize)]
        pub struct WebProxySessionReport {
            pub listen_addr: String,
            pub dry_run: bool,
            pub flows: Vec<ProxyFlow>,
            pub budget: BudgetUsage,
        }
        impl WebProxySessionReport {
            pub fn new(_listen: &str, _dry_run: bool) -> Self {
                Self::default()
            }
        }

        #[derive(Debug, Clone, Default, Serialize, Deserialize)]
        pub struct ProxyFlow {
            pub direction: ProxyFlowDirection,
            pub method: String,
            pub uri: String,
            pub status_code: u16,
        }

        #[derive(Debug, Clone, Default, Serialize, Deserialize)]
        pub enum ProxyFlowDirection {
            #[default]
            Request,
            Response,
        }

        #[derive(Debug, Clone, Default, Serialize, Deserialize)]
        pub struct BudgetUsage {
            pub flows_captured: u64,
            pub bytes_captured: u64,
        }
    }

    pub mod correlation {
        pub struct CorrelationId;
    }
    pub mod protocols {
        pub struct ProtocolDetection;
    }

    pub fn to_scan_report_data_proxy(_report: &types::WebProxySessionReport) -> serde_json::Value {
        serde_json::json!({})
    }
}

#[cfg(not(feature = "web-proxy"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProxyType {
    #[serde(rename = "socks4")]
    Socks4,
    #[serde(rename = "socks5")]
    #[default]
    Socks5,
    #[serde(rename = "http")]
    Http,
    #[serde(rename = "https")]
    Https,
    #[serde(rename = "tor")]
    Tor,
}

#[cfg(not(feature = "web-proxy"))]
impl std::fmt::Display for ProxyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProxyType::Socks4 => write!(f, "socks4"),
            ProxyType::Socks5 => write!(f, "socks5"),
            ProxyType::Http => write!(f, "http"),
            ProxyType::Https => write!(f, "https"),
            ProxyType::Tor => write!(f, "tor"),
        }
    }
}

#[cfg(not(feature = "web-proxy"))]
impl std::str::FromStr for ProxyType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "socks4" | "socks4a" => Ok(ProxyType::Socks4),
            "socks5" | "socks" => Ok(ProxyType::Socks5),
            "http" => Ok(ProxyType::Http),
            "https" => Ok(ProxyType::Https),
            "tor" => Ok(ProxyType::Tor),
            _ => Err(format!("Unknown proxy type: {}", s)),
        }
    }
}

#[cfg(not(feature = "web-proxy"))]
use eggsec_core::types::SensitiveString;

#[cfg(not(feature = "web-proxy"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyEntry {
    #[serde(default)]
    pub name: Option<String>,
    pub proxy_type: ProxyType,
    pub address: String,
    #[serde(default)]
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<SensitiveString>,
    #[serde(default)]
    pub weight: u32,
    #[serde(default)]
    pub priority: u8,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[cfg(not(feature = "web-proxy"))]
fn default_timeout_ms() -> u64 {
    crate::constants::DEFAULT_PROXY_TIMEOUT_MS
}

#[cfg(not(feature = "web-proxy"))]
fn default_true() -> bool {
    true
}

#[cfg(not(feature = "web-proxy"))]
impl ProxyEntry {
    pub fn load_from_file<P: AsRef<std::path::Path>>(_path: P) -> Result<Vec<Self>, anyhow::Error> {
        Ok(Vec::new())
    }
}

#[cfg(not(feature = "web-proxy"))]
pub struct ProxyManager;

#[cfg(not(feature = "web-proxy"))]
impl ProxyManager {
    pub fn new(_config: serde_json::Value) -> Result<Self, anyhow::Error> {
        Ok(Self)
    }
    pub async fn add_proxies_from_file(&self, _path: &str) -> Result<(), anyhow::Error> {
        Ok(())
    }
    pub fn get_random_proxy(&self) -> Option<ProxyEntry> {
        None
    }
}

#[cfg(not(feature = "web-proxy"))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_health_interval")]
    pub interval_secs: u64,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub test_url: String,
    #[serde(default = "default_max_failures")]
    pub max_failures: u32,
}

#[cfg(not(feature = "web-proxy"))]
fn default_health_interval() -> u64 {
    60
}

#[cfg(not(feature = "web-proxy"))]
fn default_max_failures() -> u32 {
    3
}

#[cfg(not(feature = "web-proxy"))]
pub struct HealthChecker;

#[cfg(not(feature = "web-proxy"))]
impl HealthChecker {
    pub fn new(_config: HealthCheckConfig) -> Result<Self, anyhow::Error> {
        Ok(Self)
    }
    pub async fn check(&self, _proxy: &ProxyEntry) -> HealthCheckResult {
        HealthCheckResult {
            proxy_url: String::new(),
            is_healthy: false,
            latency_ms: None,
            error: Some("health check not available without web-proxy feature".into()),
        }
    }
    pub async fn check_all(&self, _proxies: &[ProxyEntry]) -> Result<ProxyHealth, anyhow::Error> {
        Ok(ProxyHealth {
            total: 0,
            healthy: 0,
            unhealthy: 0,
            results: Vec::new(),
        })
    }
}

#[cfg(not(feature = "web-proxy"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub proxy_url: String,
    pub is_healthy: bool,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

#[cfg(not(feature = "web-proxy"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyHealth {
    pub total: usize,
    pub healthy: usize,
    pub unhealthy: usize,
    pub results: Vec<HealthCheckResult>,
}

#[cfg(not(feature = "web-proxy"))]
pub struct ProxiedConnection;

#[cfg(not(feature = "web-proxy"))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProxyConfig {
    #[serde(default)]
    pub enabled: bool,
}

#[cfg(not(feature = "web-proxy"))]
pub struct ProxyPool;

#[cfg(not(feature = "web-proxy"))]
pub struct ProxyRotator;
