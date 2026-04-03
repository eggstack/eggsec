use chrono::{Datelike, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::str::FromStr;
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

pub struct CronScheduler {
    expressions: Vec<CronExpression>,
}

#[derive(Debug, Clone)]
pub struct CronExpression {
    pub second: u8,
    pub minute: u8,
    pub hour: u8,
    pub day_of_month: u8,
    pub month: u8,
    pub day_of_week: u8,
}

impl CronExpression {
    pub fn parse(expression: &str) -> Result<Self, String> {
        let parts: Vec<&str> = expression.split_whitespace().collect();
        if parts.len() != 5 && parts.len() != 6 {
            return Err("Cron expression must have 5 or 6 fields".to_string());
        }

        let second: u8;
        let minute_idx: usize;
        if parts.len() == 6 {
            second = parse_field(parts[0], 0, 59)?;
            minute_idx = 1;
        } else {
            second = 0;
            minute_idx = 0;
        };

        let minute = parse_field(parts[minute_idx], 0, 59)?;
        let hour = parse_field(parts[minute_idx + 1], 0, 23)?;
        let day_of_month = parse_field(parts[minute_idx + 2], 1, 31)?;
        let month = parse_field(parts[minute_idx + 3], 1, 12)?;
        let day_of_week = parse_field(parts[minute_idx + 4], 0, 6)?;

        Ok(CronExpression {
            second,
            minute,
            hour,
            day_of_month,
            month,
            day_of_week,
        })
    }

    pub fn matches(&self, t: &chrono::DateTime<chrono::Utc>) -> bool {
        let s = t.second() as u8;
        let m = t.minute() as u8;
        let h = t.hour() as u8;
        let d = t.day() as u8;
        let mo = t.month() as u8;
        let dow = t.weekday().num_days_from_sunday() as u8;

        field_matches(self.second, s)
            && field_matches(self.minute, m)
            && field_matches(self.hour, h)
            && field_matches(self.day_of_month, d)
            && field_matches(self.month, mo)
            && field_matches(self.day_of_week, dow)
    }
}

fn parse_field(field: &str, min: u8, max: u8) -> Result<u8, String> {
    if field == "*" {
        return Ok(min);
    }

    if let Ok(num) = field.parse::<u8>() {
        if num >= min && num <= max {
            return Ok(num);
        }
    }

    if field.contains('/') {
        let parts: Vec<&str> = field.split('/').collect();
        if parts.len() == 2 {
            let base = if parts[0] == "*" {
                min
            } else {
                parts[0].parse::<u8>().unwrap_or(min)
            };
            let step: u8 = parts[1].parse().unwrap_or(1);
            return Ok(base + step);
        }
    }

    Err(format!("Invalid cron field: {}", field))
}

fn field_matches(pattern: u8, value: u8) -> bool {
    pattern == 0 || pattern == value
}

impl CronScheduler {
    pub fn new() -> Self {
        Self {
            expressions: Vec::new(),
        }
    }

    pub fn add_schedule(&mut self, expression: &str) -> Result<(), String> {
        let cron = CronExpression::parse(expression)?;
        self.expressions.push(cron);
        Ok(())
    }

    pub fn should_run(&self, t: &chrono::DateTime<chrono::Utc>) -> bool {
        self.expressions.iter().any(|e| e.matches(t))
    }

    pub fn next_run(
        &self,
        after: &chrono::DateTime<chrono::Utc>,
    ) -> Option<chrono::DateTime<chrono::Utc>> {
        let mut next = *after + chrono::Duration::hours(1);

        for _ in 0..24 {
            if self.should_run(&next) {
                return Some(next);
            }
            next += chrono::Duration::hours(1);
        }

        None
    }
}

impl Default for CronScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl FromStr for CronExpression {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        CronExpression::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_parse() {
        let expr = CronExpression::parse("0 9 * * *").unwrap();
        assert_eq!(expr.minute, 0);
        assert_eq!(expr.hour, 9);
    }

    #[test]
    fn test_cron_with_seconds() {
        let expr = CronExpression::parse("30 0 9 * * *").unwrap();
        assert_eq!(expr.second, 30);
    }

    #[test]
    fn test_cron_invalid() {
        assert!(CronExpression::parse("invalid").is_err());
    }
}
