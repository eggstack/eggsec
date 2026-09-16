//! Engine-local circuit breaker (Phase A ownership cleanup).
//!
//! Small and potentially reusable, but kept engine-local until Phase D finds
//! real independent consumers. Semantics are explicit:
//!
//! - **Consecutive failures while Closed**: a Closed-state success resets
//!   `failure_count` to 0. The breaker opens after `failure_threshold`
//!   consecutive failures, not cumulative history.
//! - **Half-open probe limit = 1**: timeout expiry admits a single probe.
//!   Concurrent `is_available()` calls while a probe is in flight are rejected
//!   so expiry cannot release an unbounded herd.
//! - **Rejected calls are not counted**: only admitted calls (those that
//!   passed `is_available()` and then called `record_success`/`record_failure`)
//!   increment `total_calls`/`total_failures`. Check-then-record is the
//!   caller contract; `is_available() == false` must not be followed by a
//!   record call.
//! - **HalfOpen accounting**: success increments `success_count`; reaching
//!   `success_threshold` closes the breaker (resetting both counters).
//!   Any HalfOpen failure re-opens immediately and resets `success_count`.
//! - Time uses `std::time::Instant` only; tests use short real timeouts, never
//!   Tokio `test-util`.

use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Clone)]
pub struct CircuitBreaker {
    failure_threshold: u64,
    success_threshold: u64,
    timeout: Duration,
    failure_count: Arc<AtomicU64>,
    success_count: Arc<AtomicU64>,
    state: Arc<Mutex<CircuitBreakerState>>,
    half_open_in_flight: Arc<AtomicBool>,
    total_calls: Arc<AtomicUsize>,
    total_failures: Arc<AtomicUsize>,
}

struct CircuitBreakerState {
    state: CircuitState,
    last_failure: Option<Instant>,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u64, success_threshold: u64, timeout: Duration) -> Self {
        Self {
            failure_threshold: failure_threshold.max(1),
            success_threshold: success_threshold.max(1),
            timeout,
            failure_count: Arc::new(AtomicU64::new(0)),
            success_count: Arc::new(AtomicU64::new(0)),
            state: Arc::new(Mutex::new(CircuitBreakerState {
                state: CircuitState::Closed,
                last_failure: None,
            })),
            half_open_in_flight: Arc::new(AtomicBool::new(false)),
            total_calls: Arc::new(AtomicUsize::new(0)),
            total_failures: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Check-then-act gate. Returns true when the caller may proceed and must
    /// then call exactly one of `record_success`/`record_failure`.
    pub fn is_available(&self) -> bool {
        let mut state = self.state.lock();
        match state.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                let expired = state
                    .last_failure
                    .is_some_and(|last| last.elapsed() >= self.timeout);
                if !expired {
                    return false;
                }
                // Transition to HalfOpen and claim the single probe permit.
                state.state = CircuitState::HalfOpen;
                self.success_count.store(0, Ordering::Relaxed);
                // Claim must succeed here (no probe was in flight while Open).
                self.half_open_in_flight.store(true, Ordering::SeqCst);
                true
            }
            CircuitState::HalfOpen => {
                // Only one concurrent probe: CAS false->true admits, true stays rejected.
                self.half_open_in_flight
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_ok()
            }
        }
    }

    pub fn record_success(&self) {
        let mut state = self.state.lock();
        self.total_calls.fetch_add(1, Ordering::Relaxed);

        match state.state {
            CircuitState::Closed => {
                // Consecutive semantics: success clears failure history.
                self.failure_count.store(0, Ordering::Relaxed);
            }
            CircuitState::HalfOpen => {
                let successes = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                if successes >= self.success_threshold {
                    state.state = CircuitState::Closed;
                    self.failure_count.store(0, Ordering::Relaxed);
                    self.success_count.store(0, Ordering::Relaxed);
                    self.half_open_in_flight.store(false, Ordering::SeqCst);
                } else {
                    // Permit released so the next probe can proceed sequentially.
                    self.half_open_in_flight.store(false, Ordering::SeqCst);
                }
            }
            CircuitState::Open => {
                // Should not happen under check-then-act (Open rejects), but
                // release defensively and stay Open.
                self.half_open_in_flight.store(false, Ordering::SeqCst);
            }
        }
    }

    pub fn record_failure(&self) {
        let mut state = self.state.lock();
        self.total_calls.fetch_add(1, Ordering::Relaxed);
        self.total_failures.fetch_add(1, Ordering::Relaxed);

        state.last_failure = Some(Instant::now());

        match state.state {
            CircuitState::Closed => {
                let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                if failures >= self.failure_threshold {
                    state.state = CircuitState::Open;
                }
            }
            CircuitState::HalfOpen => {
                // Any HalfOpen failure re-opens immediately.
                state.state = CircuitState::Open;
                self.success_count.store(0, Ordering::Relaxed);
                self.half_open_in_flight.store(false, Ordering::SeqCst);
            }
            CircuitState::Open => {
                // Already open; refresh last_failure for continued backoff.
            }
        }
    }

    pub fn get_state(&self) -> CircuitState {
        self.state.lock().state
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_closed() {
        let cb = CircuitBreaker::new(3, 2, Duration::from_secs(1));
        assert!(cb.is_available());
        assert_eq!(cb.get_state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_opens_on_consecutive_failures() {
        let cb = CircuitBreaker::new(3, 2, Duration::from_secs(1));

        cb.record_failure();
        cb.record_failure();
        assert!(cb.is_available());

        cb.record_failure();
        assert!(!cb.is_available());
        assert_eq!(cb.get_state(), CircuitState::Open);
    }

    #[test]
    fn test_closed_success_resets_consecutive_failures() {
        let cb = CircuitBreaker::new(3, 2, Duration::from_secs(1));

        cb.record_failure();
        cb.record_failure();
        cb.record_success();
        // Failure history cleared: two more failures must not open.
        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Closed);
        assert!(cb.is_available());
        // Third consecutive failure opens.
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_reopens() {
        let cb = CircuitBreaker::new(2, 2, Duration::from_secs(1));

        cb.record_failure();
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Open);

        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Open);
        assert!(!cb.is_available());
    }

    #[test]
    fn test_rejected_calls_are_not_counted() {
        let cb = CircuitBreaker::new(1, 1, Duration::from_secs(60));
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Open);
        assert_eq!(cb.total_calls(), 1);
        assert_eq!(cb.total_failures(), 1);
        // Rejected checks must not increment counters (no record call).
        assert!(!cb.is_available());
        assert!(!cb.is_available());
        assert_eq!(cb.total_calls(), 1);
        assert_eq!(cb.total_failures(), 1);
    }

    #[test]
    fn test_half_open_admits_single_probe() {
        let cb = CircuitBreaker::new(1, 1, Duration::from_millis(20));
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Open);
        std::thread::sleep(Duration::from_millis(40));
        assert!(cb.is_available());
        assert_eq!(cb.get_state(), CircuitState::HalfOpen);
        // Second concurrent check while probe in flight is rejected.
        assert!(!cb.is_available());
        // Probe success closes (threshold 1).
        cb.record_success();
        assert_eq!(cb.get_state(), CircuitState::Closed);
        assert!(cb.is_available());
    }

    #[test]
    fn test_half_open_failure_reopens() {
        let cb = CircuitBreaker::new(1, 2, Duration::from_millis(20));
        cb.record_failure();
        std::thread::sleep(Duration::from_millis(40));
        assert!(cb.is_available());
        assert_eq!(cb.get_state(), CircuitState::HalfOpen);
        cb.record_failure();
        assert_eq!(cb.get_state(), CircuitState::Open);
        assert!(!cb.is_available());
    }

    #[test]
    fn test_open_half_open_open_cycle() {
        let cb = CircuitBreaker::new(1, 1, Duration::from_millis(20));
        for _ in 0..3 {
            cb.record_failure();
            assert_eq!(cb.get_state(), CircuitState::Open);
            std::thread::sleep(Duration::from_millis(40));
            assert!(cb.is_available());
            assert_eq!(cb.get_state(), CircuitState::HalfOpen);
            cb.record_failure();
            assert_eq!(cb.get_state(), CircuitState::Open);
        }
    }

    #[test]
    fn test_half_open_needs_threshold_successes_to_close() {
        let cb = CircuitBreaker::new(1, 2, Duration::from_millis(20));
        cb.record_failure();
        std::thread::sleep(Duration::from_millis(40));
        assert!(cb.is_available());
        cb.record_success();
        // One success < threshold 2: stays HalfOpen, permit released.
        assert_eq!(cb.get_state(), CircuitState::HalfOpen);
        assert!(cb.is_available());
        cb.record_success();
        assert_eq!(cb.get_state(), CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_concurrent_record() {
        let breaker = Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(1)));
        let mut handles = vec![];
        for _ in 0..10 {
            let b = breaker.clone();
            handles.push(tokio::spawn(async move {
                b.record_failure();
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        assert_eq!(breaker.total_calls(), 10);
        assert_eq!(breaker.total_failures(), 10);
        assert_eq!(breaker.get_state(), CircuitState::Open);
    }

    #[tokio::test]
    async fn test_concurrent_half_open_single_probe() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let breaker = Arc::new(CircuitBreaker::new(1, 1, Duration::from_millis(20)));
        breaker.record_failure();
        tokio::time::sleep(Duration::from_millis(40)).await;
        let admitted = Arc::new(AtomicUsize::new(0));
        let mut handles = vec![];
        for _ in 0..10 {
            let b = breaker.clone();
            let admitted = admitted.clone();
            handles.push(tokio::spawn(async move {
                if b.is_available() {
                    admitted.fetch_add(1, Ordering::SeqCst);
                    // Hold the probe briefly so overlap is exercised.
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    b.record_success();
                }
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        assert_eq!(admitted.load(Ordering::SeqCst), 1);
        assert_eq!(breaker.get_state(), CircuitState::Closed);
    }
}
