use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::error::SlapperError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub concurrent_scans: u32,
    pub burst_size: u32,
    #[serde(default)]
    pub per_endpoint_limits: HashMap<String, EndpointLimit>,
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
            per_endpoint_limits: HashMap::new(),
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
            per_endpoint_limits: HashMap::new(),
            global_rate_limit: None,
            enable_ip_based_limiting: false,
        }
    }

    pub fn strict() -> Self {
        Self {
            requests_per_minute: 20,
            concurrent_scans: 2,
            burst_size: 5,
            per_endpoint_limits: HashMap::new(),
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
        let burst_size = value
            .get("burst_size")?
            .as_integer()?
            .try_into()
            .ok()?;

        let per_endpoint_limits = if let Some(ep) = value.get("per_endpoint_limits") {
            let mut map = HashMap::new();
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
            HashMap::new()
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

pub struct RateLimiter {
    config: RateLimitConfig,
    tokens: DashMap<String, TokenBucket>,
    concurrent_count: Arc<AtomicUsize>,
    concurrent_limit: usize,
    global_counter: Arc<AtomicUsize>,
    ip_tokens: DashMap<String, TokenBucket>,
}

#[derive(Clone)]
struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
    requests_this_minute: u32,
    minute_start: Instant,
}

impl TokenBucket {
    fn new(rate_limit: &RateLimitConfig) -> Self {
        Self {
            tokens: rate_limit.burst_size as f64,
            last_refill: Instant::now(),
            requests_this_minute: 0,
            minute_start: Instant::now(),
        }
    }

    fn new_with_burst(burst_size: u32) -> Self {
        Self {
            tokens: burst_size as f64,
            last_refill: Instant::now(),
            requests_this_minute: 0,
            minute_start: Instant::now(),
        }
    }

    fn refill(&mut self, burst_size: u32) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);

        let refill_amount = (elapsed.as_secs_f64() * burst_size as f64 / 60.0).floor();
        self.tokens = (self.tokens + refill_amount).min(burst_size as f64);
        self.last_refill = now;

        if now.duration_since(self.minute_start) >= Duration::from_secs(60) {
            self.requests_this_minute = 0;
            self.minute_start = now;
        }
    }

    fn try_consume(&mut self, tokens: f64, rpm: u32) -> bool {
        self.refill(rpm);

        if self.tokens >= tokens && self.requests_this_minute < rpm {
            self.tokens -= tokens;
            self.requests_this_minute += 1;
            true
        } else {
            false
        }
    }

    fn available_tokens(&self) -> f64 {
        self.tokens
    }

    fn reset(&mut self, config: &RateLimitConfig) {
        *self = Self::new(config);
    }
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config: config.clone(),
            tokens: DashMap::new(),
            concurrent_count: Arc::new(AtomicUsize::new(0)),
            concurrent_limit: config.concurrent_scans as usize,
            global_counter: Arc::new(AtomicUsize::new(0)),
            ip_tokens: DashMap::new(),
        }
    }

    pub fn with_config(mut self, config: RateLimitConfig) -> Self {
        self.config = config.clone();
        self.concurrent_limit = config.concurrent_scans as usize;
        self
    }

    fn get_or_create_bucket(&self, key: &str) -> TokenBucket {
        self.tokens
            .get(key)
            .map(|r| r.value().clone())
            .unwrap_or_else(|| TokenBucket::new(&self.config))
    }

    pub fn check_rate_limit(&self, key: &str) -> Result<(), SlapperError> {
        let mut bucket = self.tokens.entry(key.to_string()).or_insert_with(|| TokenBucket::new(&self.config));

        if bucket.try_consume(1.0, self.config.requests_per_minute) {
            Ok(())
        } else {
            Err(SlapperError::RateLimited(
                "Rate limit exceeded. Please wait before making more requests.".to_string(),
            ))
        }
    }

    pub fn check_rate_limit_endpoint(&self, key: &str, endpoint: &str, client_ip: Option<&str>) -> Result<(), SlapperError> {
        if let Some(global_limit) = self.config.global_rate_limit {
            let current = self.global_counter.fetch_add(1, Ordering::SeqCst);
            if current >= global_limit as usize {
                self.global_counter.fetch_sub(1, Ordering::SeqCst);
                return Err(SlapperError::RateLimited(
                    "Global rate limit exceeded. Please wait before making more requests.".to_string(),
                ));
            }
        }

        if self.config.enable_ip_based_limiting {
            if let Some(ip) = client_ip {
                let mut ip_bucket = self.ip_tokens.entry(ip.to_string()).or_insert_with(|| TokenBucket::new(&self.config));
                if !ip_bucket.try_consume(1.0, self.config.requests_per_minute) {
                    if self.config.global_rate_limit.is_some() {
                        self.global_counter.fetch_sub(1, Ordering::SeqCst);
                    }
                    return Err(SlapperError::RateLimited(
                        "Rate limit exceeded for your IP. Please wait before making more requests.".to_string(),
                    ));
                }
            }
        }

        let (rpm, burst) = if let Some(ep_limit) = self.config.per_endpoint_limits.get(endpoint) {
            (ep_limit.requests_per_minute, ep_limit.burst_size.unwrap_or(self.config.burst_size))
        } else {
            (self.config.requests_per_minute, self.config.burst_size)
        };

        let mut bucket = self.tokens.entry(key.to_string()).or_insert_with(|| TokenBucket::new_with_burst(burst));

        if bucket.try_consume(1.0, rpm) {
            Ok(())
        } else {
            if self.config.global_rate_limit.is_some() {
                self.global_counter.fetch_sub(1, Ordering::SeqCst);
            }
            Err(SlapperError::RateLimited(
                format!("Rate limit exceeded for endpoint {}. Please wait before making more requests.", endpoint),
            ))
        }
    }

    pub async fn try_acquire_concurrent(&self, key: &str) -> Result<ConcurrentPermit, SlapperError> {
        let current = self.concurrent_count.fetch_add(1, Ordering::SeqCst);

        if current >= self.concurrent_limit {
            self.concurrent_count.fetch_sub(1, Ordering::SeqCst);
            return Err(SlapperError::RateLimited(
                "Maximum concurrent scans reached. Please wait for an existing scan to complete."
                    .to_string(),
            ));
        }

        Ok(ConcurrentPermit {
            counter: self.concurrent_count.clone(),
            key: key.to_string(),
            global_counter: if self.config.global_rate_limit.is_some() {
                Some(self.global_counter.clone())
            } else {
                None
            },
        })
    }

    pub fn get_status(&self, key: &str) -> RateLimitStatus {
        if let Some(bucket) = self.tokens.get(key) {
            let bucket = bucket.value();
            RateLimitStatus {
                tokens_available: bucket.available_tokens(),
                requests_this_minute: bucket.requests_this_minute,
                requests_per_minute: self.config.requests_per_minute,
                concurrent_available: self.concurrent_limit.saturating_sub(self.concurrent_count.load(Ordering::SeqCst)),
                concurrent_limit: self.config.concurrent_scans,
                concurrent_in_use: self.concurrent_count.load(Ordering::SeqCst),
            }
        } else {
            RateLimitStatus {
                tokens_available: self.config.burst_size as f64,
                requests_this_minute: 0,
                requests_per_minute: self.config.requests_per_minute,
                concurrent_available: self.concurrent_limit.saturating_sub(self.concurrent_count.load(Ordering::SeqCst)),
                concurrent_limit: self.config.concurrent_scans,
                concurrent_in_use: self.concurrent_count.load(Ordering::SeqCst),
            }
        }
    }

    pub fn get_endpoint_status(&self, key: &str, endpoint: &str) -> RateLimitStatus {
        let (rpm, burst) = if let Some(ep_limit) = self.config.per_endpoint_limits.get(endpoint) {
            (ep_limit.requests_per_minute, ep_limit.burst_size.unwrap_or(self.config.burst_size))
        } else {
            (self.config.requests_per_minute, self.config.burst_size)
        };

        if let Some(bucket) = self.tokens.get(key) {
            let bucket = bucket.value();
            RateLimitStatus {
                tokens_available: bucket.available_tokens(),
                requests_this_minute: bucket.requests_this_minute,
                requests_per_minute: rpm,
                concurrent_available: self.concurrent_limit.saturating_sub(self.concurrent_count.load(Ordering::SeqCst)),
                concurrent_limit: self.config.concurrent_scans,
                concurrent_in_use: self.concurrent_count.load(Ordering::SeqCst),
            }
        } else {
            RateLimitStatus {
                tokens_available: burst as f64,
                requests_this_minute: 0,
                requests_per_minute: rpm,
                concurrent_available: self.concurrent_limit.saturating_sub(self.concurrent_count.load(Ordering::SeqCst)),
                concurrent_limit: self.config.concurrent_scans,
                concurrent_in_use: self.concurrent_count.load(Ordering::SeqCst),
            }
        }
    }

    pub fn get_global_status(&self) -> GlobalRateLimitStatus {
        GlobalRateLimitStatus {
            global_limit: self.config.global_rate_limit.unwrap_or(0),
            global_in_use: self.global_counter.load(Ordering::SeqCst),
        }
    }

    pub fn get_ip_status(&self, ip: &str) -> Option<RateLimitStatus> {
        self.ip_tokens.get(ip).map(|bucket| {
            let bucket = bucket.value();
            RateLimitStatus {
                tokens_available: bucket.available_tokens(),
                requests_this_minute: bucket.requests_this_minute,
                requests_per_minute: self.config.requests_per_minute,
                concurrent_available: 0,
                concurrent_limit: 0,
                concurrent_in_use: 0,
            }
        })
    }

    pub fn reset(&self, key: &str) {
        if let Some(mut bucket) = self.tokens.get_mut(key) {
            bucket.reset(&self.config);
        }
    }

    pub fn reset_all(&self) {
        self.tokens.clear();
        self.ip_tokens.clear();
        self.global_counter.store(0, Ordering::SeqCst);
    }
}

impl Clone for RateLimiter {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            tokens: DashMap::new(),
            concurrent_count: self.concurrent_count.clone(),
            concurrent_limit: self.concurrent_limit,
            global_counter: self.global_counter.clone(),
            ip_tokens: DashMap::new(),
        }
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

pub struct ConcurrentPermit {
    counter: Arc<AtomicUsize>,
    key: String,
    global_counter: Option<Arc<AtomicUsize>>,
}

impl ConcurrentPermit {
    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn release(&self) {
        self.counter.fetch_sub(1, Ordering::SeqCst);
        if let Some(ref global) = self.global_counter {
            global.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

impl Drop for ConcurrentPermit {
    fn drop(&mut self) {
        self.counter.fetch_sub(1, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiting() {
        let limiter = RateLimiter::new(RateLimitConfig::default());

        for _ in 0..10 {
            assert!(limiter.check_rate_limit("test").is_ok());
        }
    }

    #[tokio::test]
    async fn test_per_endpoint_limits() {
        let mut config = RateLimitConfig::default();
        config.per_endpoint_limits.insert(
            "/api/scan".to_string(),
            EndpointLimit {
                requests_per_minute: 10,
                burst_size: Some(2),
            },
        );
        let limiter = RateLimiter::new(config);

        for _ in 0..2 {
            assert!(limiter.check_rate_limit_endpoint("test", "/api/scan", None).is_ok());
        }

        let result = limiter.check_rate_limit_endpoint("test", "/api/scan", None);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_global_rate_limit() {
        let mut config = RateLimitConfig::default();
        config.global_rate_limit = Some(5);
        let limiter = RateLimiter::new(config);

        for _ in 0..5 {
            assert!(limiter.check_rate_limit_endpoint("test", "/api/scan", None).is_ok());
        }

        let result = limiter.check_rate_limit_endpoint("test", "/api/scan", None);
        assert!(result.is_err());

        let status = limiter.get_global_status();
        assert_eq!(status.global_in_use, 5);
    }

    #[tokio::test]
    async fn test_ip_based_limiting() {
        let mut config = RateLimitConfig::default();
        config.enable_ip_based_limiting = true;
        let limiter = RateLimiter::new(config);

        for _ in 0..10 {
            assert!(limiter.check_rate_limit_endpoint("test", "/api/scan", Some("192.168.1.1")).is_ok());
        }

        let result = limiter.check_rate_limit_endpoint("test", "/api/scan", Some("192.168.1.1"));
        assert!(result.is_err());

        let ip_status = limiter.get_ip_status("192.168.1.1");
        assert!(ip_status.is_some());
    }

    #[tokio::test]
    async fn test_concurrent_limit() {
        let config = RateLimitConfig {
            requests_per_minute: 100,
            concurrent_scans: 2,
            burst_size: 10,
            per_endpoint_limits: HashMap::new(),
            global_rate_limit: None,
            enable_ip_based_limiting: false,
        };
        let limiter = RateLimiter::new(config);

        let permit1 = limiter.try_acquire_concurrent("test").await.unwrap();
        assert_eq!(permit1.key(), "test");

        let permit2 = limiter.try_acquire_concurrent("test").await.unwrap();
        assert_eq!(permit2.key(), "test");

        let result = limiter.try_acquire_concurrent("test").await;
        assert!(result.is_err());

        drop(permit1);
        drop(permit2);

        let permit3 = limiter.try_acquire_concurrent("test").await.unwrap();
        drop(permit3);

        assert!(true);
    }

    #[tokio::test]
    async fn test_status() {
        let limiter = RateLimiter::new(RateLimitConfig::default());

        limiter.check_rate_limit("test").unwrap();

        let status = limiter.get_status("test");
        assert_eq!(status.requests_this_minute, 1);
    }

    #[tokio::test]
    async fn test_concurrent_status() {
        let config = RateLimitConfig {
            requests_per_minute: 100,
            concurrent_scans: 2,
            burst_size: 10,
            per_endpoint_limits: HashMap::new(),
            global_rate_limit: None,
            enable_ip_based_limiting: false,
        };
        let limiter = RateLimiter::new(config);

        let status = limiter.get_status("test");
        assert_eq!(status.concurrent_available, 2);
        assert_eq!(status.concurrent_in_use, 0);

        let _permit = limiter.try_acquire_concurrent("test").await.unwrap();

        let status = limiter.get_status("test");
        assert_eq!(status.concurrent_available, 1);
        assert_eq!(status.concurrent_in_use, 1);
    }
}