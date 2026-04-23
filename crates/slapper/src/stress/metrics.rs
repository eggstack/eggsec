
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub struct StressMetrics {
    packets_sent: AtomicU64,
    bytes_sent: AtomicU64,
    errors: AtomicU64,
    start_time: std::sync::OnceLock<Instant>,
}

impl StressMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&self) {
        let _ = self.start_time.set(Instant::now());
    }

    pub fn record_packet(&self, size: u64) {
        self.packets_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(size, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn packets_sent(&self) -> u64 {
        self.packets_sent.load(Ordering::Relaxed)
    }

    pub fn bytes_sent(&self) -> u64 {
        self.bytes_sent.load(Ordering::Relaxed)
    }

    pub fn errors(&self) -> u64 {
        self.errors.load(Ordering::Relaxed)
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time
            .get()
            .map(|t| t.elapsed())
            .unwrap_or_default()
    }

    pub fn to_stats(&self) -> StressStats {
        StressStats {
            duration_ms: self.elapsed().as_millis() as u64,
            packets_sent: self.packets_sent(),
            bytes_sent: self.bytes_sent(),
            errors: self.errors(),
        }
    }
}

impl Clone for StressMetrics {
    fn clone(&self) -> Self {
        Self {
            packets_sent: AtomicU64::new(self.packets_sent.load(Ordering::Relaxed)),
            bytes_sent: AtomicU64::new(self.bytes_sent.load(Ordering::Relaxed)),
            errors: AtomicU64::new(self.errors.load(Ordering::Relaxed)),
            start_time: std::sync::OnceLock::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressStats {
    pub duration_ms: u64,
    pub packets_sent: u64,
    pub bytes_sent: u64,
    pub errors: u64,
}

impl StressStats {
    pub fn avg_rate_pps(&self) -> u64 {
        if self.duration_ms == 0 {
            return 0;
        }
        (self.packets_sent * 1000) / self.duration_ms
    }

    pub fn avg_bandwidth_mbps(&self) -> f64 {
        if self.duration_ms == 0 {
            return 0.0;
        }
        let bits = self.bytes_sent * 8;
        let seconds = self.duration_ms as f64 / 1000.0;
        (bits as f64) / seconds / 1_000_000.0
    }

    pub fn merge(&mut self, other: &StressStats) {
        self.duration_ms = self.duration_ms.max(other.duration_ms);
        self.packets_sent += other.packets_sent;
        self.bytes_sent += other.bytes_sent;
        self.errors += other.errors;
    }
}

pub struct RateLimiter {
    target_pps: u64,
    interval_ns: u64,
    tokens: Arc<AtomicU64>,
    last_refill: Arc<std::sync::Mutex<Instant>>,
}

impl RateLimiter {
    pub fn new(target_pps: u64) -> Self {
        let interval_ns = if target_pps > 0 {
            1_000_000_000 / target_pps
        } else {
            0
        };

        Self {
            target_pps,
            interval_ns,
            tokens: Arc::new(AtomicU64::new(target_pps)),
            last_refill: Arc::new(std::sync::Mutex::new(Instant::now())),
        }
    }

    pub async fn wait_for_token(&self) {
        if self.target_pps == 0 {
            return;
        }

        loop {
            let tokens = self.tokens.load(Ordering::Relaxed);
            if tokens > 0
                && self
                    .tokens
                    .compare_exchange(tokens, tokens - 1, Ordering::Relaxed, Ordering::Relaxed)
                    .is_ok()
                {
                    return;
                }

            let sleep_ns = self.interval_ns.min(1_000_000);
            tokio::time::sleep(Duration::from_nanos(sleep_ns)).await;
        }
    }

    pub fn refill(&self) {
        let mut last = match self.last_refill.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::warn!("Rate limiter mutex was poisoned, recovering");
                poisoned.into_inner()
            }
        };
        let now = Instant::now();
        let elapsed_ns = now.duration_since(*last).as_nanos() as u64;

        if elapsed_ns >= 1_000_000_000 {
            let new_tokens = (elapsed_ns / 1_000_000_000) * self.target_pps;
            self.tokens.fetch_add(new_tokens, Ordering::Relaxed);
            *last = now;
        }
    }
}

#[derive(Debug, Clone)]
pub struct PacketBatch {
    pub packets: Vec<Vec<u8>>,
    pub total_size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stress_metrics_new() {
        let metrics = StressMetrics::new();
        assert_eq!(metrics.packets_sent(), 0);
        assert_eq!(metrics.bytes_sent(), 0);
        assert_eq!(metrics.errors(), 0);
    }

    #[test]
    fn test_stress_metrics_record_packet() {
        let metrics = StressMetrics::new();
        metrics.record_packet(64);
        assert_eq!(metrics.packets_sent(), 1);
        assert_eq!(metrics.bytes_sent(), 64);
    }

    #[test]
    fn test_stress_metrics_record_multiple() {
        let metrics = StressMetrics::new();
        metrics.record_packet(64);
        metrics.record_packet(128);
        metrics.record_packet(256);
        assert_eq!(metrics.packets_sent(), 3);
        assert_eq!(metrics.bytes_sent(), 448);
    }

    #[test]
    fn test_stress_metrics_record_error() {
        let metrics = StressMetrics::new();
        metrics.record_error();
        assert_eq!(metrics.errors(), 1);
    }

    #[test]
    fn test_stress_metrics_to_stats() {
        let metrics = StressMetrics::new();
        metrics.record_packet(100);
        metrics.record_packet(200);
        let stats = metrics.to_stats();
        assert_eq!(stats.packets_sent, 2);
        assert_eq!(stats.bytes_sent, 300);
    }

    #[test]
    fn test_stress_metrics_clone() {
        let metrics = StressMetrics::new();
        metrics.record_packet(100);
        let cloned = metrics.clone();
        cloned.record_packet(200);
        assert_eq!(metrics.packets_sent(), 1);
        assert_eq!(cloned.packets_sent(), 1);
    }

    #[test]
    fn test_stress_stats_avg_rate_pps() {
        let stats = StressStats {
            duration_ms: 1000,
            packets_sent: 1000,
            bytes_sent: 64000,
            errors: 0,
        };
        assert_eq!(stats.avg_rate_pps(), 1000);
    }

    #[test]
    fn test_stress_stats_avg_rate_pps_zero_duration() {
        let stats = StressStats {
            duration_ms: 0,
            packets_sent: 100,
            bytes_sent: 6400,
            errors: 0,
        };
        assert_eq!(stats.avg_rate_pps(), 0);
    }

    #[test]
    fn test_stress_stats_avg_bandwidth_mbps() {
        let stats = StressStats {
            duration_ms: 1000,
            packets_sent: 1000,
            bytes_sent: 125000, // 1 Mb
            errors: 0,
        };
        assert_eq!(stats.avg_bandwidth_mbps(), 1.0);
    }

    #[test]
    fn test_stress_stats_merge() {
        let mut stats1 = StressStats {
            duration_ms: 1000,
            packets_sent: 100,
            bytes_sent: 6400,
            errors: 5,
        };
        let stats2 = StressStats {
            duration_ms: 2000,
            packets_sent: 200,
            bytes_sent: 12800,
            errors: 3,
        };
        stats1.merge(&stats2);
        assert_eq!(stats1.duration_ms, 2000);
        assert_eq!(stats1.packets_sent, 300);
        assert_eq!(stats1.bytes_sent, 19200);
        assert_eq!(stats1.errors, 8);
    }

    #[test]
    fn test_packet_batch_new() {
        let batch = PacketBatch::new(10);
        assert!(batch.is_empty());
        assert_eq!(batch.len(), 0);
    }

    #[test]
    fn test_packet_batch_add() {
        let mut batch = PacketBatch::new(10);
        batch.add(vec![1, 2, 3, 4]);
        assert!(!batch.is_empty());
        assert_eq!(batch.len(), 1);
        assert_eq!(batch.total_size, 4);
    }

    #[test]
    fn test_packet_batch_clear() {
        let mut batch = PacketBatch::new(10);
        batch.add(vec![1, 2]);
        batch.add(vec![3, 4]);
        batch.clear();
        assert!(batch.is_empty());
        assert_eq!(batch.total_size, 0);
    }

    #[test]
    fn test_rate_limiter_new() {
        let limiter = RateLimiter::new(100);
        assert_eq!(limiter.interval_ns, 10_000_000);
    }

    #[test]
    fn test_rate_limiter_zero_pps() {
        let limiter = RateLimiter::new(0);
        assert_eq!(limiter.interval_ns, 0);
    }
}

impl PacketBatch {
    pub fn add(&mut self, packet: Vec<u8>) {
        self.total_size += packet.len() as u64;
        self.packets.push(packet);
    }

    pub fn len(&self) -> usize {
        self.packets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.packets.is_empty()
    }

    pub fn clear(&mut self) {
        self.packets.clear();
        self.total_size = 0;
    }
}
