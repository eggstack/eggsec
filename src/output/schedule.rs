#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledScan {
    pub id: String,
    pub target: String,
    pub scan_type: ScanType,
    pub scheduled_at: String,
    pub status: ScheduleStatus,
    pub priority: Priority,
    pub options: ScanOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScanType {
    Recon,
    PortScan,
    EndpointScan,
    Fingerprint,
    Fuzz,
    Waf,
    WafStress,
    Pipeline,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ScheduleStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

impl Priority {
    pub fn as_int(&self) -> i32 {
        match self {
            Priority::Low => 0,
            Priority::Normal => 1,
            Priority::High => 2,
            Priority::Critical => 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanOptions {
    pub ports: Option<String>,
    pub concurrency: Option<usize>,
    pub timeout: Option<u64>,
    pub wordlist: Option<String>,
}

pub struct ScanQueue {
    queue: VecDeque<ScheduledScan>,
    max_size: usize,
    running: Option<ScheduledScan>,
}

impl ScanQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: VecDeque::new(),
            max_size,
            running: None,
        }
    }

    pub fn enqueue(&mut self, scan: ScheduledScan) -> Result<(), String> {
        if self.queue.len() >= self.max_size {
            return Err("Queue is full".to_string());
        }

        let position = self
            .queue
            .iter()
            .position(|s| s.priority.as_int() < scan.priority.as_int())
            .unwrap_or(self.queue.len());

        self.queue.insert(position, scan);
        Ok(())
    }

    pub fn dequeue(&mut self) -> Option<ScheduledScan> {
        self.queue.pop_front()
    }

    pub fn cancel(&mut self, id: &str) -> bool {
        if let Some(pos) = self.queue.iter().position(|s| s.id == id) {
            self.queue.remove(pos);
            return true;
        }
        false
    }

    pub fn get_status(&self, id: &str) -> Option<&ScheduledScan> {
        self.queue.iter().find(|s| s.id == id)
    }

    pub fn list_pending(&self) -> Vec<&ScheduledScan> {
        self.queue.iter().collect()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty() && self.running.is_none()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn start_next(&mut self) -> Option<ScheduledScan> {
        if self.running.is_some() {
            return None;
        }

        if let Some(scan) = self.queue.pop_front() {
            self.running = Some(scan.clone());
            return Some(scan);
        }
        None
    }

    pub fn complete(&mut self, id: &str, _success: bool) {
        if let Some(running) = &self.running {
            if running.id == id {
                self.running = None;
            }
        }
    }

    pub fn get_running(&self) -> Option<&ScheduledScan> {
        self.running.as_ref()
    }
}

pub struct RateLimiter {
    requests_per_second: u32,
    burst_size: u32,
    tokens: u32,
    last_refill: Instant,
}

impl RateLimiter {
    pub fn new(requests_per_second: u32) -> Self {
        Self {
            requests_per_second,
            burst_size: requests_per_second * 2,
            tokens: requests_per_second * 2,
            last_refill: Instant::now(),
        }
    }

    pub fn try_acquire(&mut self) -> bool {
        self.refill();

        if self.tokens > 0 {
            self.tokens -= 1;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill);
        let tokens_to_add = (elapsed.as_secs_f64() * self.requests_per_second as f64) as u32;

        self.tokens = (self.tokens + tokens_to_add).min(self.burst_size);
        self.last_refill = now;
    }

    pub fn wait_time(&self) -> Duration {
        if self.tokens > 0 {
            Duration::ZERO
        } else {
            Duration::from_millis(100)
        }
    }
}

impl Default for ScanQueue {
    fn default() -> Self {
        Self::new(100)
    }
}
