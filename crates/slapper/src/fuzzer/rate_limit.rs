use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Instant;

const DEFAULT_RATE_LIMIT_THRESHOLD: usize = 3;
const DEFAULT_BACKOFF_MULTIPLIER: f64 = 0.5;
const DEFAULT_RECOVERY_MULTIPLIER: f64 = 1.25;
const DEFAULT_SUCCESS_THRESHOLD_FOR_INCREASE: usize = 10;

#[derive(Debug, Clone)]
pub struct AdaptiveRateLimiter {
    current_rate: Arc<AtomicU64>,
    max_rate: u64,
    min_rate: u64,
    window_ms: u64,

    consecutive_errors: Arc<AtomicUsize>,
    consecutive_successes: Arc<AtomicUsize>,

    rate_limit_threshold: usize,
    backoff_multiplier: f64,
    recovery_multiplier: f64,
}

impl Default for AdaptiveRateLimiter {
    fn default() -> Self {
        Self::new(100, 1, 1000)
    }
}

impl AdaptiveRateLimiter {
    pub fn new(max_rate: u64, min_rate: u64, window_ms: u64) -> Self {
        Self {
            current_rate: Arc::new(AtomicU64::new(max_rate)),
            max_rate,
            min_rate,
            window_ms,
            consecutive_errors: Arc::new(AtomicUsize::new(0)),
            consecutive_successes: Arc::new(AtomicUsize::new(0)),
            rate_limit_threshold: DEFAULT_RATE_LIMIT_THRESHOLD,
            backoff_multiplier: DEFAULT_BACKOFF_MULTIPLIER,
            recovery_multiplier: DEFAULT_RECOVERY_MULTIPLIER,
        }
    }

    pub fn with_thresholds(
        max_rate: u64,
        min_rate: u64,
        window_ms: u64,
        rate_limit_threshold: usize,
        backoff_multiplier: f64,
        recovery_multiplier: f64,
    ) -> Self {
        Self {
            current_rate: Arc::new(AtomicU64::new(max_rate)),
            max_rate,
            min_rate,
            window_ms,
            consecutive_errors: Arc::new(AtomicUsize::new(0)),
            consecutive_successes: Arc::new(AtomicUsize::new(0)),
            rate_limit_threshold,
            backoff_multiplier,
            recovery_multiplier,
        }
    }

    pub fn get_rate(&self) -> u64 {
        self.current_rate.load(Ordering::SeqCst)
    }

    pub fn record_success(&self) {
        self.consecutive_errors.store(0, Ordering::SeqCst);
        let current = self.consecutive_successes.fetch_add(1, Ordering::SeqCst);

        if current >= DEFAULT_SUCCESS_THRESHOLD_FOR_INCREASE {
            self.try_increase_rate();
            self.consecutive_successes.store(0, Ordering::SeqCst);
        }
    }

    pub fn record_error(&self, status_code: Option<u16>) {
        self.consecutive_successes.store(0, Ordering::SeqCst);

        let should_backoff = status_code
            .map(|code| code == crate::constants::STATUS_RATE_LIMITED || code == crate::constants::STATUS_SERVER_ERROR || code >= 500)
            .unwrap_or(true);

        if should_backoff {
            let current = self.consecutive_errors.fetch_add(1, Ordering::SeqCst);
            if current >= self.rate_limit_threshold {
                self.backoff();
            }
        }
    }

    pub fn record_timeout(&self) {
        self.consecutive_errors.fetch_add(1, Ordering::SeqCst);
        let errors = self.consecutive_errors.load(Ordering::SeqCst);

        if errors >= self.rate_limit_threshold {
            self.backoff();
        }
    }

    fn backoff(&self) {
        let current = self.current_rate.load(Ordering::SeqCst);
        let new_rate = ((current as f64) * self.backoff_multiplier) as u64;
        let new_rate = new_rate.max(self.min_rate);

        self.current_rate.store(new_rate, Ordering::SeqCst);
        tracing::info!("Rate limiter: backing off to {} req/s", new_rate);
    }

    fn try_increase_rate(&self) {
        let current = self.current_rate.load(Ordering::SeqCst);

        if current >= self.max_rate {
            return;
        }

        let new_rate = ((current as f64) * self.recovery_multiplier) as u64;
        let new_rate = new_rate.min(self.max_rate);

        self.current_rate.store(new_rate, Ordering::SeqCst);
    }

    pub fn get_concurrency(&self) -> usize {
        let rate = self.get_rate();
        let window_secs = self.window_ms as f64 / 1000.0;
        (rate as f64 * window_secs).max(1.0) as usize
    }

    pub fn reset(&self) {
        self.current_rate.store(self.max_rate, Ordering::SeqCst);
        self.consecutive_errors.store(0, Ordering::SeqCst);
        self.consecutive_successes.store(0, Ordering::SeqCst);
    }
}

pub struct RateLimiterTokenBucket {
    capacity: u64,
    tokens: Arc<AtomicU64>,
    refill_rate: f64,
    last_refill: Arc<Mutex<Instant>>,
}

impl RateLimiterTokenBucket {
    pub fn new(capacity: u64, refill_rate: f64) -> Self {
        Self {
            capacity,
            tokens: Arc::new(AtomicU64::new(capacity)),
            refill_rate,
            last_refill: Arc::new(Mutex::new(Instant::now())),
        }
    }

    pub async fn acquire(&self, tokens: u64) -> bool {
        self.refill().await;

        loop {
            let current = self.tokens.load(Ordering::SeqCst);
            if current < tokens {
                return false;
            }

            let new = current - tokens;
            if self
                .tokens
                .compare_exchange(current, new, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return true;
            }
        }
    }

    async fn refill(&self) {
        let mut last = self.last_refill.lock().await;
        let now = Instant::now();
        let elapsed = now.duration_since(*last).as_secs_f64();

        let to_add = (elapsed * self.refill_rate) as u64;
        if to_add > 0 {
            let current = self.tokens.load(Ordering::SeqCst);
            let new = (current + to_add).min(self.capacity);
            self.tokens.store(new, Ordering::SeqCst);
            *last = now;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_rate_limiter() {
        let limiter = AdaptiveRateLimiter::new(100, 1, 1000);
        assert_eq!(limiter.get_rate(), 100);

        limiter.record_error(Some(429));
        limiter.record_error(Some(429));
        limiter.record_error(Some(429));
        limiter.record_error(Some(429));
        assert!(limiter.get_rate() < 100);
    }

    #[tokio::test]
    async fn test_token_bucket() {
        let bucket = RateLimiterTokenBucket::new(10, 1.0);
        assert!(bucket.acquire(5).await);
    }
}
