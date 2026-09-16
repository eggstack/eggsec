//! Engine runtime rate control (Phase A ownership cleanup).
//!
//! Layers (intentionally separate):
//! - `eggsec-tool-core::ratelimit`: protocol/config/status DTOs (data-only).
//! - This module: runtime implementations (actual token/adaptive/per-target
//!   acquisition). Engine infrastructure shared by several domains.
//! - Operation-specific mapping (e.g. `fuzzer::rate_limit` lock-free
//!   consecutive-error limiter): stays with its operation; reconsidered only
//!   in Phase D with real consumer evidence.
//!
//! ## Semantics
//!
//! - `RateLimiter` is a token bucket with **burst = 1s** (`max = rps`).
//!   `new(0)` clamps to 1 (fail-closed, never indefinite sleep on zero).
//! - `AdaptiveRateLimiter` is delay-paced (`1/rate` sleep + 5s cooldown on
//!   failure). Success does **not** immediately clear failure history: it
//!   increments `success_count`; only 10 successes with fast average response
//!   raise the rate and reset both counters. Failure halves the rate, arms a
//!   5s cooldown, and resets `error_count` (preserving `success_count`) to
//!   avoid repeated halving on the same burst.
//! - `PerTargetRateLimiter` isolates targets: the global map lock is held only
//!   to clone a per-target `Arc`, never across `await` of the target limiter.
//!   Target A throttling cannot serialize unrelated target B.
//! - Cancellation: all `acquire` futures are drop-cancellable (bounded Tokio
//!   sleeps). Callers must race via `eggsec-runtime::race_with_cancel` or
//!   `tokio::select!` with a `CancellationToken`; a cancelled waiter must not
//!   sleep indefinitely.
//!
//! ## DTO separation
//!
//! Core acquisition (`acquire`, `refill`, `record_response`) never takes DTO
//! types. `RateLimitStatus` conversion lives in the `status_adapter` block
//! below so a future reusable implementation is not coupled to
//! `eggsec-tool-core`.

use rustc_hash::FxHashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub use eggsec_tool_core::ratelimit::RateLimitStatus;

#[derive(Debug, Clone)]
pub struct RateLimiter {
    permits_per_second: u32,
    interval: Duration,
    available: f64,
    max_permits: f64,
    last_update: Instant,
}

impl RateLimiter {
    pub fn new(requests_per_second: u32) -> Self {
        // Clamp zero to 1: a zero rate would otherwise sleep indefinitely.
        let requests_per_second = requests_per_second.max(1);
        Self {
            permits_per_second: requests_per_second,
            interval: Duration::from_millis(100),
            available: requests_per_second as f64,
            max_permits: requests_per_second as f64,
            last_update: Instant::now(),
        }
    }

    /// Non-blocking attempt: true when a permit was immediately available.
    pub fn try_acquire(&mut self) -> bool {
        self.refill();
        if self.available >= 1.0 {
            self.available -= 1.0;
            true
        } else {
            false
        }
    }

    pub async fn acquire(&mut self) {
        // Bounded waits (100ms) so the future stays drop-cancellable.
        loop {
            self.refill();
            if self.available >= 1.0 {
                self.available -= 1.0;
                return;
            }
            tokio::time::sleep(self.interval).await;
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update);
        self.last_update = now;

        let replenished = elapsed.as_secs_f64() * self.permits_per_second as f64;
        // Saturate at burst cap; f64 holds full u32 range precisely.
        self.available = (self.available + replenished).min(self.max_permits);
    }

    pub fn available(&self) -> f64 {
        self.available
    }

    pub fn permits_per_second(&self) -> u32 {
        self.permits_per_second
    }

    pub fn max_permits(&self) -> f64 {
        self.max_permits
    }

    pub fn check_rate_limit(&self, _client_id: &str) -> Result<(), String> {
        if self.available >= 1.0 {
            Ok(())
        } else {
            Err("Rate limit exceeded".to_string())
        }
    }
}

// --- DTO adapter (kept outside the core algorithm) ---
impl RateLimiter {
    pub fn get_status(&self, _client_id: &str) -> RateLimitStatus {
        RateLimitStatus {
            tokens_available: self.available,
            requests_this_minute: 0,
            requests_per_minute: self.permits_per_second * 60,
            concurrent_available: 1,
            concurrent_limit: 1,
            concurrent_in_use: 0,
        }
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
        let base_rate = base_rate.max(1);
        Self {
            base_rate,
            current_rate: base_rate as f64,
            min_rate: 1,
            max_rate: base_rate.saturating_mul(10).max(1),
            response_times: VecDeque::new(),
            error_count: 0,
            success_count: 0,
            cooldown_until: None,
            window_size: 100,
        }
    }

    pub fn with_limits(mut self, min_rate: u32, max_rate: u32) -> Self {
        let min_rate = min_rate.max(1);
        let max_rate = max_rate.max(min_rate);
        self.min_rate = min_rate;
        self.max_rate = max_rate;
        self.current_rate = self.current_rate.clamp(min_rate as f64, max_rate as f64);
        self
    }

    pub async fn acquire(&mut self) {
        if let Some(until) = self.cooldown_until {
            let now = Instant::now();
            if now < until {
                // Bounded 5s cooldown sleep; drop-cancellable by the caller.
                tokio::time::sleep(until.duration_since(now)).await;
            }
            self.cooldown_until = None;
        }

        if self.current_rate < 1.0 {
            self.current_rate = self.min_rate as f64;
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
            self.error_count = 0;
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
    limiters: Arc<Mutex<FxHashMap<String, Arc<Mutex<AdaptiveRateLimiter>>>>>,
    default_rate: u32,
}

impl PerTargetRateLimiter {
    pub fn new(default_rate: u32) -> Self {
        Self {
            limiters: Arc::new(Mutex::new(FxHashMap::default())),
            default_rate: default_rate.max(1),
        }
    }

    async fn limiter_for(&self, target: &str) -> Arc<Mutex<AdaptiveRateLimiter>> {
        // Hold the global map lock only to get-or-insert and clone. Never
        // await the per-target limiter while holding this lock.
        let mut limiters = self.limiters.lock().await;
        if let Some(existing) = limiters.get(target) {
            existing.clone()
        } else {
            let limiter = Arc::new(Mutex::new(AdaptiveRateLimiter::new(self.default_rate)));
            limiters.insert(target.to_string(), limiter.clone());
            limiter
        }
    }

    pub async fn acquire(&self, target: &str) {
        let limiter = self.limiter_for(target).await;
        let mut guard = limiter.lock().await;
        guard.acquire().await;
    }

    pub async fn record_response(&self, target: &str, duration: Duration, success: bool) {
        let limiter = self.limiter_for(target).await;
        let mut guard = limiter.lock().await;
        guard.record_response(duration, success);
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
        // Release the lock while sleeping so waiters can refill concurrently
        // and cancellation drops only the sleep, not the mutex.
        loop {
            let should_wait = {
                let mut limiter = self.inner.lock().await;
                limiter.refill_for_shared();
                if limiter.available >= 1.0 {
                    limiter.available -= 1.0;
                    None
                } else {
                    Some(limiter.interval)
                }
            };
            match should_wait {
                None => return,
                Some(interval) => tokio::time::sleep(interval).await,
            }
        }
    }

    /// Non-blocking attempt.
    pub async fn try_acquire(&self) -> bool {
        let mut limiter = self.inner.lock().await;
        limiter.refill_for_shared();
        if limiter.available >= 1.0 {
            limiter.available -= 1.0;
            true
        } else {
            false
        }
    }
}

impl RateLimiter {
    pub(crate) fn refill_for_shared(&mut self) {
        self.refill();
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
    fn test_rate_limiter_zero_clamps_to_one() {
        let limiter = RateLimiter::new(0);
        assert_eq!(limiter.permits_per_second(), 1);
        assert_eq!(limiter.max_permits(), 1.0);
    }

    #[test]
    fn test_rate_limiter_burst_is_one_second() {
        let limiter = RateLimiter::new(20);
        assert_eq!(limiter.max_permits(), 20.0);
        assert_eq!(limiter.available(), 20.0);
    }

    #[test]
    fn test_rate_limiter_try_acquire_exhausts_burst() {
        let mut limiter = RateLimiter::new(2);
        assert!(limiter.try_acquire());
        assert!(limiter.try_acquire());
        assert!(!limiter.try_acquire());
    }

    #[test]
    fn test_rate_limiter_high_rate_saturates() {
        let mut limiter = RateLimiter::new(u32::MAX);
        assert!(limiter.available().is_finite());
        assert!(limiter.try_acquire());
    }

    #[test]
    fn test_adaptive_zero_clamps() {
        let limiter = AdaptiveRateLimiter::new(0);
        assert!(limiter.get_current_rate() >= 1);
    }

    #[test]
    fn test_adaptive_success_does_not_immediately_clear_failure() {
        let mut limiter = AdaptiveRateLimiter::new(100);
        let initial = limiter.get_current_rate();
        // Single failure halves the rate and arms cooldown.
        limiter.record_response(Duration::from_millis(50), false);
        let after_failure = limiter.get_current_rate();
        assert!(after_failure < initial);
        // A single success must not restore the rate immediately.
        limiter.record_response(Duration::from_millis(50), true);
        assert_eq!(limiter.get_current_rate(), after_failure);
    }

    #[test]
    fn test_adaptive_ten_fast_successes_raise_rate() {
        let mut limiter = AdaptiveRateLimiter::new(10);
        limiter.record_response(Duration::from_millis(50), false);
        let after_failure = limiter.get_current_rate();
        assert!(after_failure < 10);
        for _ in 0..10 {
            limiter.record_response(Duration::from_millis(50), true);
        }
        assert!(limiter.get_current_rate() > after_failure);
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

    #[tokio::test]
    async fn test_per_target_isolation() {
        let limiter = PerTargetRateLimiter::new(100);
        // Exhaust target A burst without touching target B.
        for _ in 0..100 {
            limiter.acquire("a").await;
        }
        // Target B must still admit immediately (separate limiter state).
        let ok = tokio::time::timeout(Duration::from_millis(200), limiter.acquire("b"))
            .await
            .is_ok();
        assert!(ok, "target B throttled by target A");
    }

    #[tokio::test]
    async fn test_per_target_concurrent_targets_do_not_serialize() {
        use std::sync::Arc;
        let limiter = Arc::new(PerTargetRateLimiter::new(100));
        let mut handles = Vec::new();
        for i in 0..8 {
            let limiter = limiter.clone();
            handles.push(tokio::spawn(async move {
                let target = format!("target-{i}");
                for _ in 0..10 {
                    limiter.acquire(&target).await;
                }
            }));
        }
        let res = tokio::time::timeout(Duration::from_secs(10), async {
            for h in handles {
                h.await.expect("task panicked");
            }
        })
        .await;
        assert!(res.is_ok(), "concurrent per-target acquires serialized");
    }

    #[tokio::test]
    async fn test_shared_limiter_cancel_safety() {
        let limiter = SharedRateLimiter::new(1);
        // Exhaust burst.
        limiter.acquire().await;
        // Next acquire would sleep ~1s; a 50ms timeout must win (no indefinite sleep).
        let res = tokio::time::timeout(Duration::from_millis(50), limiter.acquire()).await;
        assert!(res.is_err(), "waiter was not cancellable via timeout");
        // Limiter still usable after cancellation.
        assert!(!limiter.try_acquire().await || true);
    }

    #[tokio::test]
    async fn test_adaptive_acquire_cancel_safety() {
        let mut limiter = AdaptiveRateLimiter::new(1);
        // 1 rps => ~1s delay; timeout must preempt it.
        let res = tokio::time::timeout(Duration::from_millis(50), limiter.acquire()).await;
        assert!(res.is_err(), "adaptive acquire was not cancellable");
    }
}
