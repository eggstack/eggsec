
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    failure_threshold: u64,
    success_threshold: u64,
    timeout: Duration,
    failure_count: Arc<AtomicU64>,
    success_count: Arc<AtomicU64>,
    state: Arc<Mutex<CircuitBreakerState>>,
    total_calls: Arc<AtomicUsize>,
    total_failures: Arc<AtomicUsize>,
}

struct CircuitBreakerState {
    state: CircuitState,
    last_failure: Option<tokio::time::Instant>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u64, success_threshold: u64, timeout: Duration) -> Self {
        Self {
            failure_threshold,
            success_threshold,
            timeout,
            failure_count: Arc::new(AtomicU64::new(0)),
            success_count: Arc::new(AtomicU64::new(0)),
            state: Arc::new(Mutex::new(CircuitBreakerState {
                state: CircuitState::Closed,
                last_failure: None,
            })),
            total_calls: Arc::new(AtomicUsize::new(0)),
            total_failures: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub async fn is_available(&self) -> bool {
        let mut state = self.state.lock().await;
        match state.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last) = state.last_failure {
                    if last.elapsed() >= self.timeout {
                        state.state = CircuitState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    pub async fn record_success(&self) {
        self.total_calls.fetch_add(1, Ordering::Relaxed);
        let successes = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;

        let mut state = self.state.lock().await;
        if state.state == CircuitState::HalfOpen {
            if successes >= self.success_threshold {
                state.state = CircuitState::Closed;
                self.failure_count.store(0, Ordering::Relaxed);
                self.success_count.store(0, Ordering::Relaxed);
            }
        } else {
            self.failure_count.store(0, Ordering::Relaxed);
        }
    }

    pub async fn record_failure(&self) {
        self.total_calls.fetch_add(1, Ordering::Relaxed);
        self.total_failures.fetch_add(1, Ordering::Relaxed);
        let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;

        let mut state = self.state.lock().await;
        state.last_failure = Some(tokio::time::Instant::now());

        if state.state == CircuitState::HalfOpen
            || (state.state == CircuitState::Closed && failures >= self.failure_threshold)
        {
            state.state = CircuitState::Open;
        }
    }

    pub async fn get_state(&self) -> CircuitState {
        self.state.lock().await.state
    }

    pub fn total_calls(&self) -> usize {
        self.total_calls.load(Ordering::Relaxed)
    }

    pub fn total_failures(&self) -> usize {
        self.total_failures.load(Ordering::Relaxed)
    }

    pub fn failure_rate(&self) -> f64 {
        let calls = self.total_calls();
        if calls == 0 {
            return 0.0;
        }
        self.total_failures() as f64 / calls as f64
    }
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new(5, 3, Duration::from_secs(30))
    }
}

pub struct CircuitBreakerRegistry {
    breakers: Arc<Mutex<std::collections::HashMap<String, CircuitBreaker>>>,
    default_failure_threshold: u64,
    default_success_threshold: u64,
    default_timeout: Duration,
}

impl CircuitBreakerRegistry {
    pub fn new(failure_threshold: u64, success_threshold: u64, timeout: Duration) -> Self {
        Self {
            breakers: Arc::new(Mutex::new(std::collections::HashMap::new())),
            default_failure_threshold: failure_threshold,
            default_success_threshold: success_threshold,
            default_timeout: timeout,
        }
    }

    pub fn default_registry() -> Self {
        Self::new(5, 3, Duration::from_secs(30))
    }

    pub async fn get_or_create(&self, name: &str) -> CircuitBreaker {
        let mut breakers = self.breakers.lock().await;
        if let Some(breaker) = breakers.get(name) {
            return breaker.clone();
        }
        let breaker = CircuitBreaker::new(
            self.default_failure_threshold,
            self.default_success_threshold,
            self.default_timeout,
        );
        breakers.insert(name.to_string(), breaker.clone());
        breaker
    }

    pub async fn get_state(&self, name: &str) -> Option<CircuitState> {
        let breakers = self.breakers.lock().await;
        let breaker = breakers.get(name)?;
        Some(breaker.get_state().await)
    }

    pub async fn stats(&self) -> Vec<(String, CircuitState, usize, usize, f64)> {
        let breakers = self.breakers.lock().await;
        let mut stats = Vec::new();
        for (name, breaker) in breakers.iter() {
            let state = breaker.get_state().await;
            let calls = breaker.total_calls();
            let failures = breaker.total_failures();
            let rate = breaker.failure_rate();
            stats.push((name.clone(), state, calls, failures, rate));
        }
        stats
    }
}

impl Clone for CircuitBreaker {
    fn clone(&self) -> Self {
        Self {
            failure_threshold: self.failure_threshold,
            success_threshold: self.success_threshold,
            timeout: self.timeout,
            failure_count: self.failure_count.clone(),
            success_count: self.success_count.clone(),
            state: self.state.clone(),
            total_calls: self.total_calls.clone(),
            total_failures: self.total_failures.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_closed() {
        let cb = CircuitBreaker::new(3, 2, Duration::from_secs(1));
        assert!(cb.is_available().await);
        assert_eq!(cb.get_state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() {
        let cb = CircuitBreaker::new(3, 2, Duration::from_millis(50));

        cb.record_failure().await;
        cb.record_failure().await;
        assert!(cb.is_available().await);

        cb.record_failure().await;
        assert!(!cb.is_available().await);
        assert_eq!(cb.get_state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open() {
        tokio::time::pause();
        let cb = CircuitBreaker::new(2, 2, Duration::from_millis(50));

        cb.record_failure().await;
        cb.record_failure().await;
        assert!(!cb.is_available().await);

        tokio::time::advance(Duration::from_millis(60)).await;
        assert!(cb.is_available().await);
        assert_eq!(cb.get_state().await, CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn test_circuit_breaker_closes_after_successes() {
        tokio::time::pause();
        let cb = CircuitBreaker::new(2, 2, Duration::from_millis(50));

        cb.record_failure().await;
        cb.record_failure().await;

        tokio::time::advance(Duration::from_millis(60)).await;
        assert!(cb.is_available().await);

        cb.record_success().await;
        cb.record_success().await;
        assert_eq!(cb.get_state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_reopens_on_failure_in_half_open() {
        tokio::time::pause();
        let cb = CircuitBreaker::new(2, 2, Duration::from_millis(50));

        cb.record_failure().await;
        cb.record_failure().await;

        tokio::time::advance(Duration::from_millis(60)).await;
        cb.record_failure().await;

        assert!(!cb.is_available().await);
        assert_eq!(cb.get_state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_concurrent_record() {
        tokio::time::pause();
        let breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(1)));
        let mut handles = vec![];
        for _ in 0..10 {
            let b = breaker.clone();
            handles.push(tokio::spawn(async move {
                b.record_failure().await;
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        assert_eq!(breaker.total_calls(), 10);
        assert_eq!(breaker.total_failures(), 10);
        assert_eq!(breaker.get_state().await, CircuitState::Open);
    }
}
