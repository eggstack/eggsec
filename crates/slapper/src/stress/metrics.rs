use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
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
}
