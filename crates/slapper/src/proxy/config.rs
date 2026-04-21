use crate::error::{Result, SlapperError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::SocketAddr;
use std::path::Path;

use crate::types::SensitiveString;

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

    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,

    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_timeout() -> u64 {
    10000
}

fn default_true() -> bool {
    true
}

impl ProxyEntry {
    pub fn new(proxy_type: ProxyType, address: String, port: u16) -> Self {
        Self {
            name: None,
            proxy_type,
            address,
            port,
            username: None,
            password: None,
            weight: 1,
            priority: 0,
            timeout_ms: default_timeout(),
            enabled: true,
            tags: Vec::new(),
        }
    }

    pub fn with_auth(mut self, username: String, password: String) -> Self {
        self.username = Some(username);
        self.password = Some(SensitiveString::new(password));
        self
    }

    pub fn with_weight(mut self, weight: u32) -> Self {
        self.weight = weight;
        self
    }

    pub fn socket_addr(&self) -> Result<SocketAddr> {
        format!("{}:{}", self.address, self.port)
            .parse()
            .map_err(|e| {
                SlapperError::Proxy(format!(
                    "Invalid proxy address: {}:{}: {}",
                    self.address, self.port, e
                ))
            })
    }

    pub fn to_url(&self) -> String {
        let scheme = self.proxy_type.to_string();
        match (&self.username, &self.password) {
            (Some(user), Some(pass)) => {
                format!(
                    "{}://{}:{}@{}:{}",
                    scheme,
                    user,
                    pass.expose_secret(),
                    self.address,
                    self.port
                )
            }
            _ => format!("{}://{}:{}", scheme, self.address, self.port),
        }
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<Self>> {
        let content = fs::read_to_string(&path).map_err(|e| {
            SlapperError::Proxy(format!(
                "Failed to read proxy file: {:?}: {}",
                path.as_ref(),
                e
            ))
        })?;

        let proxies = if path
            .as_ref()
            .extension()
            .map(|e| e == "json")
            .unwrap_or(false)
        {
            serde_json::from_str(&content)?
        } else if path
            .as_ref()
            .extension()
            .map(|e| e == "yaml" || e == "yml")
            .unwrap_or(false)
        {
            serde_yaml_neo::from_str(&content)?
        } else {
            Self::parse_proxy_list(&content)?
        };

        Ok(proxies)
    }

    fn parse_proxy_list(content: &str) -> Result<Vec<Self>> {
        let mut proxies = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Ok(proxy) = Self::parse_line(line) {
                proxies.push(proxy);
            }
        }

        Ok(proxies)
    }

    fn parse_line(line: &str) -> Result<Self> {
        let parts: Vec<&str> = line.splitn(2, "://").collect();
        let (proxy_type, remainder) = if parts.len() == 2 {
            (
                parts[0].parse::<ProxyType>().map_err(SlapperError::Proxy)?,
                parts[1],
            )
        } else {
            (ProxyType::Socks5, parts[0])
        };

        let (auth, host_port) = if remainder.contains('@') {
            let parts: Vec<&str> = remainder.splitn(2, '@').collect();
            (Some(parts[0]), parts[1])
        } else {
            (None, remainder)
        };

        let (username, password) = if let Some(auth_str) = auth {
            let parts: Vec<&str> = auth_str.splitn(2, ':').collect();
            if parts.len() == 2 {
                (
                    Some(parts[0].to_string()),
                    Some(SensitiveString::new(parts[1].to_string())),
                )
            } else {
                (Some(parts[0].to_string()), None)
            }
        } else {
            (None, None)
        };

        let parts: Vec<&str> = host_port.rsplitn(2, ':').collect();
        let (address, port) = if parts.len() == 2 {
            (parts[1].to_string(), parts[0].parse()?)
        } else {
            return Err(SlapperError::Proxy(format!(
                "Invalid proxy format: {}",
                line
            )));
        };

        Ok(Self {
            name: None,
            proxy_type,
            address,
            port,
            username,
            password,
            weight: 1,
            priority: 0,
            timeout_ms: default_timeout(),
            enabled: true,
            tags: Vec::new(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    #[serde(default)]
    pub rotation_strategy: RotationStrategy,

    #[serde(default = "default_health_check")]
    pub health_check_enabled: bool,

    #[serde(default = "default_health_interval")]
    pub health_check_interval_secs: u64,

    #[serde(default = "default_health_timeout")]
    pub health_check_timeout_ms: u64,

    #[serde(default)]
    pub test_url: Option<String>,

    #[serde(default = "default_max_failures")]
    pub max_failures_before_disable: u32,

    #[serde(default)]
    pub chain_proxies: bool,

    #[serde(default)]
    pub max_chain_length: usize,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            rotation_strategy: RotationStrategy::default(),
            health_check_enabled: default_health_check(),
            health_check_interval_secs: default_health_interval(),
            health_check_timeout_ms: default_health_timeout(),
            test_url: Some("https://api.ipify.org?format=json".to_string()),
            max_failures_before_disable: default_max_failures(),
            chain_proxies: false,
            max_chain_length: 3,
        }
    }
}

fn default_health_check() -> bool {
    true
}
fn default_health_interval() -> u64 {
    60
}
fn default_health_timeout() -> u64 {
    5000
}
fn default_max_failures() -> u32 {
    3
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq)]
pub enum RotationStrategy {
    #[default]
    #[serde(rename = "round_robin")]
    RoundRobin,
    #[serde(rename = "random")]
    Random,
    #[serde(rename = "weighted")]
    Weighted,
    #[serde(rename = "least_used")]
    LeastUsed,
    #[serde(rename = "lowest_latency")]
    LowestLatency,
}

impl std::fmt::Display for RotationStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RotationStrategy::RoundRobin => write!(f, "round_robin"),
            RotationStrategy::Random => write!(f, "random"),
            RotationStrategy::Weighted => write!(f, "weighted"),
            RotationStrategy::LeastUsed => write!(f, "least_used"),
            RotationStrategy::LowestLatency => write!(f, "lowest_latency"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub enabled: bool,
    pub interval_secs: u64,
    pub timeout_ms: u64,
    pub test_url: String,
    pub max_failures: u32,
}

impl From<&ProxyConfig> for HealthCheckConfig {
    fn from(config: &ProxyConfig) -> Self {
        Self {
            enabled: config.health_check_enabled,
            interval_secs: config.health_check_interval_secs,
            timeout_ms: config.health_check_timeout_ms,
            test_url: config
                .test_url
                .clone()
                .unwrap_or_else(|| "https://api.ipify.org".to_string()),
            max_failures: config.max_failures_before_disable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_type_display() {
        assert_eq!(ProxyType::Socks4.to_string(), "socks4");
        assert_eq!(ProxyType::Socks5.to_string(), "socks5");
        assert_eq!(ProxyType::Http.to_string(), "http");
        assert_eq!(ProxyType::Https.to_string(), "https");
        assert_eq!(ProxyType::Tor.to_string(), "tor");
    }

    #[test]
    fn test_proxy_type_from_str() {
        assert_eq!("socks4".parse::<ProxyType>().unwrap(), ProxyType::Socks4);
        assert_eq!("socks4a".parse::<ProxyType>().unwrap(), ProxyType::Socks4);
        assert_eq!("socks5".parse::<ProxyType>().unwrap(), ProxyType::Socks5);
        assert_eq!("socks".parse::<ProxyType>().unwrap(), ProxyType::Socks5);
        assert_eq!("http".parse::<ProxyType>().unwrap(), ProxyType::Http);
        assert_eq!("https".parse::<ProxyType>().unwrap(), ProxyType::Https);
        assert_eq!("tor".parse::<ProxyType>().unwrap(), ProxyType::Tor);
        assert!("unknown".parse::<ProxyType>().is_err());
    }

    #[test]
    fn test_proxy_type_from_str_case_insensitive() {
        assert_eq!("SOCKS5".parse::<ProxyType>().unwrap(), ProxyType::Socks5);
        assert_eq!("Http".parse::<ProxyType>().unwrap(), ProxyType::Http);
    }

    #[test]
    fn test_proxy_entry_new() {
        let entry = ProxyEntry::new(ProxyType::Socks5, "127.0.0.1".to_string(), 1080);
        assert_eq!(entry.proxy_type, ProxyType::Socks5);
        assert_eq!(entry.address, "127.0.0.1");
        assert_eq!(entry.port, 1080);
        assert!(entry.enabled);
        assert_eq!(entry.weight, 1);
        assert_eq!(entry.priority, 0);
        assert_eq!(entry.timeout_ms, 10000);
        assert!(entry.username.is_none());
        assert!(entry.password.is_none());
        assert!(entry.tags.is_empty());
    }

    #[test]
    fn test_proxy_entry_with_auth() {
        let entry = ProxyEntry::new(ProxyType::Socks5, "127.0.0.1".to_string(), 1080)
            .with_auth("user".to_string(), "pass".to_string());
        assert_eq!(entry.username, Some("user".to_string()));
        assert!(entry.password.is_some());
    }

    #[test]
    fn test_proxy_entry_with_weight() {
        let entry =
            ProxyEntry::new(ProxyType::Socks5, "127.0.0.1".to_string(), 1080).with_weight(10);
        assert_eq!(entry.weight, 10);
    }

    #[test]
    fn test_proxy_entry_socket_addr_valid() {
        let entry = ProxyEntry::new(ProxyType::Socks5, "127.0.0.1".to_string(), 1080);
        let addr = entry.socket_addr().unwrap();
        assert_eq!(addr.to_string(), "127.0.0.1:1080");
    }

    #[test]
    fn test_proxy_entry_socket_addr_invalid() {
        let entry = ProxyEntry::new(ProxyType::Socks5, "not-an-ip".to_string(), 1080);
        assert!(entry.socket_addr().is_err());
    }

    #[test]
    fn test_proxy_entry_to_url_no_auth() {
        let entry = ProxyEntry::new(ProxyType::Http, "proxy.example.com".to_string(), 8080);
        assert_eq!(entry.to_url(), "http://proxy.example.com:8080");
    }

    #[test]
    fn test_proxy_entry_to_url_with_auth() {
        let entry = ProxyEntry::new(ProxyType::Socks5, "10.0.0.1".to_string(), 9050)
            .with_auth("user".to_string(), "secret".to_string());
        assert_eq!(entry.to_url(), "socks5://user:secret@10.0.0.1:9050");
    }

    #[test]
    fn test_proxy_entry_parse_line_basic() {
        let entry = ProxyEntry::parse_line("socks5://192.168.1.1:1080").unwrap();
        assert_eq!(entry.proxy_type, ProxyType::Socks5);
        assert_eq!(entry.address, "192.168.1.1");
        assert_eq!(entry.port, 1080);
    }

    #[test]
    fn test_proxy_entry_parse_line_with_auth() {
        let entry = ProxyEntry::parse_line("http://user:pass@proxy.com:3128").unwrap();
        assert_eq!(entry.proxy_type, ProxyType::Http);
        assert_eq!(entry.address, "proxy.com");
        assert_eq!(entry.port, 3128);
        assert_eq!(entry.username, Some("user".to_string()));
        assert!(entry.password.is_some());
    }

    #[test]
    fn test_proxy_entry_parse_line_no_scheme() {
        let entry = ProxyEntry::parse_line("10.0.0.1:9050").unwrap();
        assert_eq!(entry.proxy_type, ProxyType::Socks5);
        assert_eq!(entry.address, "10.0.0.1");
        assert_eq!(entry.port, 9050);
    }

    #[test]
    fn test_proxy_entry_parse_line_invalid() {
        assert!(ProxyEntry::parse_line("not-valid").is_err());
    }

    #[test]
    fn test_proxy_entry_parse_proxy_list() {
        let content = "# comment\nsocks5://1.1.1.1:1080\n\nhttp://2.2.2.2:8080\n";
        let proxies = ProxyEntry::parse_proxy_list(content).unwrap();
        assert_eq!(proxies.len(), 2);
        assert_eq!(proxies[0].address, "1.1.1.1");
        assert_eq!(proxies[1].address, "2.2.2.2");
    }

    #[test]
    fn test_proxy_config_default() {
        let config = ProxyConfig::default();
        assert_eq!(config.health_check_enabled, true);
        assert_eq!(config.health_check_interval_secs, 60);
        assert_eq!(config.health_check_timeout_ms, 5000);
        assert_eq!(config.max_failures_before_disable, 3);
        assert_eq!(config.max_chain_length, 3);
        assert!(!config.chain_proxies);
    }

    #[test]
    fn test_rotation_strategy_display() {
        assert_eq!(RotationStrategy::RoundRobin.to_string(), "round_robin");
        assert_eq!(RotationStrategy::Random.to_string(), "random");
        assert_eq!(RotationStrategy::Weighted.to_string(), "weighted");
        assert_eq!(RotationStrategy::LeastUsed.to_string(), "least_used");
        assert_eq!(
            RotationStrategy::LowestLatency.to_string(),
            "lowest_latency"
        );
    }

    #[test]
    fn test_health_check_config_from_proxy_config() {
        let config = ProxyConfig::default();
        let hc: HealthCheckConfig = (&config).into();
        assert!(hc.enabled);
        assert_eq!(hc.interval_secs, 60);
        assert_eq!(hc.timeout_ms, 5000);
        assert_eq!(hc.max_failures, 3);
    }

    #[test]
    fn test_health_check_config_from_proxy_config_no_test_url() {
        let mut config = ProxyConfig::default();
        config.test_url = None;
        let hc: HealthCheckConfig = (&config).into();
        assert_eq!(hc.test_url, "https://api.ipify.org");
    }

    #[test]
    fn test_proxy_config_json_roundtrip() {
        let config = ProxyConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: ProxyConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.rotation_strategy, parsed.rotation_strategy);
        assert_eq!(config.health_check_enabled, parsed.health_check_enabled);
    }

    #[test]
    fn test_proxy_entry_json_roundtrip() {
        let entry = ProxyEntry::new(ProxyType::Http, "proxy.com".to_string(), 3128).with_weight(5);
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: ProxyEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.address, "proxy.com");
        assert_eq!(parsed.port, 3128);
        assert_eq!(parsed.weight, 5);
    }

    #[test]
    fn test_proxy_entry_yaml_roundtrip() {
        let entry = ProxyEntry::new(ProxyType::Socks5, "10.0.0.1".to_string(), 9050);
        let yaml = serde_yaml_neo::to_string(&entry).unwrap();
        let parsed: ProxyEntry = serde_yaml_neo::from_str(&yaml).unwrap();
        assert_eq!(parsed.address, "10.0.0.1");
        assert_eq!(parsed.port, 9050);
    }
}
