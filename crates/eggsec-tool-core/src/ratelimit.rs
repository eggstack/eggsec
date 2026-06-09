use rustc_hash::FxHashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub concurrent_scans: u32,
    pub burst_size: u32,
    #[serde(default)]
    pub per_endpoint_limits: FxHashMap<String, EndpointLimit>,
    #[serde(default)]
    pub global_rate_limit: Option<u32>,
    #[serde(default)]
    pub enable_ip_based_limiting: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointLimit {
    pub requests_per_minute: u32,
    pub burst_size: Option<u32>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            concurrent_scans: 5,
            burst_size: 10,
            per_endpoint_limits: FxHashMap::default(),
            global_rate_limit: None,
            enable_ip_based_limiting: false,
        }
    }
}

impl RateLimitConfig {
    pub fn standard() -> Self {
        Self::default()
    }

    pub fn relaxed() -> Self {
        Self {
            requests_per_minute: 300,
            concurrent_scans: 10,
            burst_size: 25,
            per_endpoint_limits: FxHashMap::default(),
            global_rate_limit: None,
            enable_ip_based_limiting: false,
        }
    }

    pub fn strict() -> Self {
        Self {
            requests_per_minute: 20,
            concurrent_scans: 2,
            burst_size: 5,
            per_endpoint_limits: FxHashMap::default(),
            global_rate_limit: None,
            enable_ip_based_limiting: false,
        }
    }

    pub fn from_toml(value: &toml::Value) -> Option<Self> {
        let requests_per_minute = value
            .get("requests_per_minute")?
            .as_integer()?
            .try_into()
            .ok()?;
        let concurrent_scans = value
            .get("concurrent_scans")?
            .as_integer()?
            .try_into()
            .ok()?;
        let burst_size = value.get("burst_size")?.as_integer()?.try_into().ok()?;

        let per_endpoint_limits = if let Some(ep) = value.get("per_endpoint_limits") {
            let mut map = FxHashMap::default();
            if let Some(table) = ep.as_table() {
                for (key, val) in table {
                    if let Some(ep_val) = val.as_table() {
                        let rpm = ep_val
                            .get("requests_per_minute")
                            .and_then(|v| v.as_integer())
                            .map(|v| v as u32)
                            .unwrap_or(requests_per_minute);
                        let bs = ep_val
                            .get("burst_size")
                            .and_then(|v| v.as_integer())
                            .map(|v| v as u32);
                        map.insert(
                            key.clone(),
                            EndpointLimit {
                                requests_per_minute: rpm,
                                burst_size: bs,
                            },
                        );
                    }
                }
            }
            map
        } else {
            FxHashMap::default()
        };

        let global_rate_limit = value
            .get("global_rate_limit")
            .and_then(|v| v.as_integer())
            .map(|v| v as u32);

        let enable_ip_based_limiting = value
            .get("enable_ip_based_limiting")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Some(Self {
            requests_per_minute,
            concurrent_scans,
            burst_size,
            per_endpoint_limits,
            global_rate_limit,
            enable_ip_based_limiting,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitStatus {
    pub tokens_available: f64,
    pub requests_this_minute: u32,
    pub requests_per_minute: u32,
    pub concurrent_available: usize,
    pub concurrent_limit: u32,
    pub concurrent_in_use: usize,
}

#[derive(Debug, Clone)]
pub struct GlobalRateLimitStatus {
    pub global_limit: u32,
    pub global_in_use: usize,
}
