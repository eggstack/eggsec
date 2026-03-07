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
            if tokens > 0 {
                if self.tokens.compare_exchange(
                    tokens,
                    tokens - 1,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ).is_ok() {
                    return;
                }
            }
            
            let sleep_ns = self.interval_ns.min(1_000_000);
            tokio::time::sleep(Duration::from_nanos(sleep_ns)).await;
        }
    }
    
    pub fn refill(&self) {
        let mut last = self.last_refill.lock().unwrap();
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

impl PacketBatch {
    pub fn new(capacity: usize) -> Self {
        Self {
            packets: Vec::with_capacity(capacity),
            total_size: 0,
        }
    }
    
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
