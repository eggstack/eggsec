use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub struct RateLimiter {
    permits_per_second: u32,
    interval: Duration,
    available: f64,
    max_permits: f64,
    last_update: Instant,
}

impl RateLimiter {
    pub fn new(requests_per_second: u32) -> Self {
        Self {
            permits_per_second: requests_per_second,
            interval: Duration::from_millis(100),
            available: requests_per_second as f64,
            max_permits: requests_per_second as f64,
            last_update: Instant::now(),
        }
    }

    pub async fn acquire(&mut self) {
        self.refill();

        while self.available < 1.0 {
            tokio::time::sleep(self.interval).await;
            self.refill();
        }

        self.available -= 1.0;
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update);
        self.last_update = now;

        let replenished = elapsed.as_secs_f64() * self.permits_per_second as f64;
        self.available = (self.available + replenished).min(self.max_permits);
    }

    pub fn available(&self) -> f64 {
        self.available
    }
}

pub struct AdaptiveRateLimiter {
    base_rate: u32,
    current_rate: f64,
    min_rate: u32,
    max_rate: u32,
    response_times: VecDeque<Duration>,
    error_count: u32,
    success_count: u32,
    cooldown_until: Option<Instant>,
    window_size: usize,
}

impl AdaptiveRateLimiter {
    pub fn new(base_rate: u32) -> Self {
        Self {
            base_rate,
            current_rate: base_rate as f64,
            min_rate: 1,
            max_rate: base_rate * 10,
            response_times: VecDeque::new(),
            error_count: 0,
            success_count: 0,
            cooldown_until: None,
            window_size: 100,
        }
    }

    pub fn with_limits(mut self, min_rate: u32, max_rate: u32) -> Self {
        self.min_rate = min_rate;
        self.max_rate = max_rate;
        self
    }

    pub async fn acquire(&mut self) {
        if let Some(until) = self.cooldown_until {
            if Instant::now() < until {
                let remaining = until.duration_since(Instant::now());
                tokio::time::sleep(remaining).await;
            }
            self.cooldown_until = None;
        }

        let delay = Duration::from_secs_f64(1.0 / self.current_rate);
        tokio::time::sleep(delay).await;
    }

    pub fn record_response(&mut self, duration: Duration, success: bool) {
        if self.response_times.len() >= self.window_size {
            self.response_times.pop_front();
        }
        self.response_times.push_back(duration);

        if success {
            self.success_count += 1;
            self.adjust_rate_up();
        } else {
            self.error_count += 1;
            self.adjust_rate_down();
        }
    }

    fn adjust_rate_up(&mut self) {
        if self.success_count >= 10 && self.current_rate < self.max_rate as f64 {
            let avg_response = self.calculate_avg_response_time();

            if avg_response < Duration::from_millis(500) {
                self.current_rate = (self.current_rate * 1.2).min(self.max_rate as f64);
            } else if avg_response < Duration::from_secs(1) {
                self.current_rate = (self.current_rate * 1.1).min(self.max_rate as f64);
            }

            self.success_count = 0;
        }
    }

    fn adjust_rate_down(&mut self) {
        let error_rate = if self.success_count + self.error_count > 0 {
            self.error_count as f64 / (self.success_count + self.error_count) as f64
        } else {
            1.0
        };

        if error_rate > 0.1 || self.response_times.back() > Some(&Duration::from_secs(5)) {
            self.current_rate = (self.current_rate * 0.5).max(self.min_rate as f64);
            self.cooldown_until = Some(Instant::now() + Duration::from_secs(5));
        }

        self.error_count = 0;
    }

    fn calculate_avg_response_time(&self) -> Duration {
        if self.response_times.is_empty() {
            return Duration::from_secs(0);
        }

        let total: Duration = self.response_times.iter().sum();
        total / self.response_times.len() as u32
    }

    pub fn get_current_rate(&self) -> u32 {
        self.current_rate as u32
    }

    pub fn get_error_rate(&self) -> f64 {
        let total = self.success_count + self.error_count;
        if total == 0 {
            return 0.0;
        }
        self.error_count as f64 / total as f64
    }

    pub fn reset(&mut self) {
        self.current_rate = self.base_rate as f64;
        self.response_times.clear();
        self.error_count = 0;
        self.success_count = 0;
        self.cooldown_until = None;
    }
}

pub struct PerTargetRateLimiter {
    limiters: Arc<Mutex<std::collections::HashMap<String, AdaptiveRateLimiter>>>,
    default_rate: u32,
}

impl PerTargetRateLimiter {
    pub fn new(default_rate: u32) -> Self {
        Self {
            limiters: Arc::new(Mutex::new(std::collections::HashMap::new())),
            default_rate,
        }
    }

    pub async fn acquire(&self, target: &str) {
        let mut limiters = self.limiters.lock().await;
        let limiter = limiters
            .entry(target.to_string())
            .or_insert_with(|| AdaptiveRateLimiter::new(self.default_rate));
        limiter.acquire().await;
    }

    pub async fn record_response(&self, target: &str, duration: Duration, success: bool) {
        let mut limiters = self.limiters.lock().await;
        if let Some(limiter) = limiters.get_mut(target) {
            limiter.record_response(duration, success);
        }
    }
}

pub struct JitterConfig {
    pub min_ms: u64,
    pub max_ms: u64,
}

impl JitterConfig {
    pub fn new(min_ms: u64, max_ms: u64) -> Self {
        Self { min_ms, max_ms }
    }

    pub fn from_spec(spec: &str) -> Option<Self> {
        let parts: Vec<&str> = spec.split('-').collect();
        match parts.len() {
            1 => {
                let ms: u64 = parts[0].parse().ok()?;
                Some(Self::new(ms, ms))
            }
            2 => {
                let min: u64 = parts[0].parse().ok()?;
                let max: u64 = parts[1].parse().ok()?;
                if min > max {
                    return None;
                }
                Some(Self::new(min, max))
            }
            _ => None,
        }
    }

    pub fn random_delay(&self) -> Duration {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let ms = rng.gen_range(self.min_ms..=self.max_ms);
        Duration::from_millis(ms)
    }
}

#[derive(Clone)]
pub struct SharedRateLimiter {
    inner: Arc<Mutex<RateLimiter>>,
}

impl SharedRateLimiter {
    pub fn new(requests_per_second: u32) -> Self {
        Self {
            inner: Arc::new(Mutex::new(RateLimiter::new(requests_per_second))),
        }
    }

    pub async fn acquire(&self) {
        let mut limiter = self.inner.lock().await;
        limiter.acquire().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_new() {
        let limiter = RateLimiter::new(10);
        assert_eq!(limiter.permits_per_second, 10);
    }

    #[test]
    fn test_jitter_config_new() {
        let jitter = JitterConfig::new(100, 500);
        assert_eq!(jitter.min_ms, 100);
        assert_eq!(jitter.max_ms, 500);
    }

    #[test]
    fn test_jitter_config_from_spec_single() {
        let jitter = JitterConfig::from_spec("100").unwrap();
        assert_eq!(jitter.min_ms, 100);
        assert_eq!(jitter.max_ms, 100);
    }

    #[test]
    fn test_jitter_config_from_spec_range() {
        let jitter = JitterConfig::from_spec("100-500").unwrap();
        assert_eq!(jitter.min_ms, 100);
        assert_eq!(jitter.max_ms, 500);
    }

    #[test]
    fn test_jitter_config_from_spec_invalid() {
        assert!(JitterConfig::from_spec("500-100").is_none());
        assert!(JitterConfig::from_spec("abc").is_none());
    }

    #[tokio::test]
    async fn test_rate_limiter_acquire() {
        let mut limiter = RateLimiter::new(100);
        limiter.acquire().await;
        assert!(limiter.available() < 100.0);
    }
}
