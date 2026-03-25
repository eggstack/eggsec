
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::net::SocketAddr;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProxyType {
    #[serde(rename = "socks4")]
    Socks4,
    #[serde(rename = "socks5")]
    Socks5,
    #[serde(rename = "http")]
    Http,
    #[serde(rename = "https")]
    Https,
    #[serde(rename = "tor")]
    Tor,
}

impl Default for ProxyType {
    fn default() -> Self {
        Self::Socks5
    }
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

    fn from_str(s: &str) -> Result<Self, Self::Err> {
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
    pub password: Option<String>,

    #[serde(default)]
    pub weight: u32,

    #[serde(default)]
    pub priority: u8,

    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,

    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_timeout() -> u64 {
    10000
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
        self.password = Some(password);
        self
    }

    pub fn with_weight(mut self, weight: u32) -> Self {
        self.weight = weight;
        self
    }

    pub fn socket_addr(&self) -> Result<SocketAddr> {
        format!("{}:{}", self.address, self.port)
            .parse()
            .with_context(|| format!("Invalid proxy address: {}:{}", self.address, self.port))
    }

    pub fn to_url(&self) -> String {
        let scheme = self.proxy_type.to_string();
        match (&self.username, &self.password) {
            (Some(user), Some(pass)) => {
                format!(
                    "{}://{}:{}@{}:{}",
                    scheme, user, pass, self.address, self.port
                )
            }
            _ => format!("{}://{}:{}", scheme, self.address, self.port),
        }
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<Self>> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read proxy file: {:?}", path.as_ref()))?;

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
            serde_yaml::from_str(&content)?
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
                parts[0]
                    .parse::<ProxyType>()
                    .map_err(|e| anyhow::anyhow!(e))?,
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
                (Some(parts[0].to_string()), Some(parts[1].to_string()))
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
            anyhow::bail!("Invalid proxy format: {}", line);
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
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
