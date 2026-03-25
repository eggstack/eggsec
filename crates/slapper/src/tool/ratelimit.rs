use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::error::SlapperError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub concurrent_scans: u32,
    pub burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            concurrent_scans: 5,
            burst_size: 10,
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
        }
    }

    pub fn strict() -> Self {
        Self {
            requests_per_minute: 20,
            concurrent_scans: 2,
            burst_size: 5,
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

        Some(Self {
            requests_per_minute,
            concurrent_scans,
            burst_size,
        })
    }
}

pub struct RateLimiter {
    config: RateLimitConfig,
    tokens: RwLock<HashMap<String, TokenBucket>>,
    concurrent_count: Arc<AtomicUsize>,
    concurrent_limit: usize,
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
            tokens: RwLock::new(HashMap::new()),
            concurrent_count: Arc::new(AtomicUsize::new(0)),
            concurrent_limit: config.concurrent_scans as usize,
        }
    }

    pub fn with_config(mut self, config: RateLimitConfig) -> Self {
        self.config = config.clone();
        self.concurrent_limit = config.concurrent_scans as usize;
        self
    }

    fn get_or_create_bucket(&self, key: &str) -> TokenBucket {
        let buckets = self.tokens.read();
        buckets
            .get(key)
            .cloned()
            .unwrap_or_else(|| TokenBucket::new(&self.config))
    }

    pub fn check_rate_limit(&self, key: &str) -> Result<(), SlapperError> {
        let mut buckets = self.tokens.write();
        
        let bucket = buckets
            .entry(key.to_string())
            .or_insert_with(|| TokenBucket::new(&self.config));

        if bucket.try_consume(1.0, self.config.requests_per_minute) {
            Ok(())
        } else {
            Err(SlapperError::RateLimited(
                "Rate limit exceeded. Please wait before making more requests.".to_string(),
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
        })
    }

    pub fn get_status(&self, key: &str) -> RateLimitStatus {
        let buckets = self.tokens.read();
        
        if let Some(bucket) = buckets.get(key) {
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

    pub fn reset(&self, key: &str) {
        let mut buckets = self.tokens.write();
        if let Some(bucket) = buckets.get_mut(key) {
            bucket.reset(&self.config);
        }
    }

    pub fn reset_all(&self) {
        let mut buckets = self.tokens.write();
        for bucket in buckets.values_mut() {
            bucket.reset(&self.config);
        }
    }
}

impl Clone for RateLimiter {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            tokens: RwLock::new(HashMap::new()),
            concurrent_count: self.concurrent_count.clone(),
            concurrent_limit: self.concurrent_limit,
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

pub struct ConcurrentPermit {
    counter: Arc<AtomicUsize>,
    key: String,
}

impl ConcurrentPermit {
    pub fn key(&self) -> &str {
        &self.key
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
    async fn test_concurrent_limit() {
        let config = RateLimitConfig {
            requests_per_minute: 100,
            concurrent_scans: 2,
            burst_size: 10,
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
