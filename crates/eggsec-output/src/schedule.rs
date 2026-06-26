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

    pub fn complete(&mut self, id: &str, success: bool) {
        if let Some(ref mut running) = self.running {
            if running.id == id {
                running.status = if success {
                    ScheduleStatus::Completed
                } else {
                    ScheduleStatus::Failed("Scan failed".to_string())
                };
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
        let raw_tokens = elapsed.as_secs_f64() * self.requests_per_second as f64;
        let tokens_to_add = (raw_tokens.min(f64::from(u32::MAX)) as u32).min(self.burst_size);

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
    second_matcher: CronFieldMatcher,
    minute_matcher: CronFieldMatcher,
    hour_matcher: CronFieldMatcher,
    day_of_month_matcher: CronFieldMatcher,
    month_matcher: CronFieldMatcher,
    day_of_week_matcher: CronFieldMatcher,
}

impl CronExpression {
    pub fn parse(expression: &str) -> Result<Self, String> {
        let parts: Vec<&str> = expression.split_whitespace().collect();
        if parts.len() != 5 && parts.len() != 6 {
            return Err("Cron expression must have 5 or 6 fields".to_string());
        }

        let second: u8;
        let second_matcher: CronFieldMatcher;
        let minute_idx: usize;
        if parts.len() == 6 {
            let parsed = parse_field(parts[0], 0, 59)?;
            second = parsed.display_value();
            second_matcher = parsed;
            minute_idx = 1;
        } else {
            second = 0;
            second_matcher = CronFieldMatcher::exact(0);
            minute_idx = 0;
        };

        let minute_matcher = parse_field(parts[minute_idx], 0, 59)?;
        let hour_matcher = parse_field(parts[minute_idx + 1], 0, 23)?;
        let day_of_month_matcher = parse_field(parts[minute_idx + 2], 1, 31)?;
        let month_matcher = parse_field(parts[minute_idx + 3], 1, 12)?;
        let day_of_week_matcher = parse_field(parts[minute_idx + 4], 0, 6)?;

        Ok(CronExpression {
            second,
            minute: minute_matcher.display_value(),
            hour: hour_matcher.display_value(),
            day_of_month: day_of_month_matcher.display_value(),
            month: month_matcher.display_value(),
            day_of_week: day_of_week_matcher.display_value(),
            second_matcher,
            minute_matcher,
            hour_matcher,
            day_of_month_matcher,
            month_matcher,
            day_of_week_matcher,
        })
    }

    pub fn matches(&self, t: &chrono::DateTime<chrono::Utc>) -> bool {
        let s = t.second() as u8;
        let m = t.minute() as u8;
        let h = t.hour() as u8;
        let d = t.date_naive().day() as u8;
        let mo = t.date_naive().month() as u8;
        let dow = t.weekday().num_days_from_sunday() as u8;

        self.second_matcher.matches(s)
            && self.minute_matcher.matches(m)
            && self.hour_matcher.matches(h)
            && self.day_of_month_matcher.matches(d)
            && self.month_matcher.matches(mo)
            && self.day_of_week_matcher.matches(dow)
    }
}

#[derive(Debug, Clone, Copy)]
enum CronFieldMatcher {
    Any,
    Exact(u8),
    Step { start: u8, step: u8 },
}

impl CronFieldMatcher {
    fn exact(value: u8) -> Self {
        Self::Exact(value)
    }

    fn matches(&self, value: u8) -> bool {
        match self {
            Self::Any => true,
            Self::Exact(pattern) => *pattern == value,
            Self::Step { start, step } => value >= *start && ((value - *start) % *step == 0),
        }
    }

    fn display_value(&self) -> u8 {
        match self {
            Self::Any => 0,
            Self::Exact(value) => *value,
            Self::Step { start, .. } => *start,
        }
    }
}

fn parse_field(field: &str, min: u8, max: u8) -> Result<CronFieldMatcher, String> {
    if field == "*" {
        return Ok(CronFieldMatcher::Any);
    }

    if let Ok(num) = field.parse::<u8>() {
        if num >= min && num <= max {
            return Ok(CronFieldMatcher::Exact(num));
        }
    }

    if field.contains('/') {
        let parts: Vec<&str> = field.split('/').collect();
        if parts.len() == 2 {
            let base = if parts[0] == "*" {
                min
            } else {
                parts[0]
                    .parse::<u8>()
                    .map_err(|_| format!("Invalid cron field start: {}", field))?
            };
            let step = parts[1]
                .parse::<u8>()
                .map_err(|_| format!("Invalid cron field step: {}", field))?;
            if base < min || base > max {
                return Err(format!("Cron field start out of range: {}", field));
            }
            if step == 0 {
                return Err(format!("Cron field step cannot be zero: {}", field));
            }
            return Ok(CronFieldMatcher::Step { start: base, step });
        }
    }

    Err(format!("Invalid cron field: {}", field))
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
        let mut next = *after + chrono::Duration::seconds(1);
        let max_lookahead = 7 * 24 * 60 * 60;

        for _ in 0..max_lookahead {
            if self.should_run(&next) {
                return Some(next);
            }
            next += chrono::Duration::seconds(1);
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
    use chrono::TimeZone;

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

    #[test]
    fn test_cron_wildcard_matches_any_day_month() {
        let expr = CronExpression::parse("0 9 * * *").expect("parse should succeed");
        let t = chrono::Utc
            .with_ymd_and_hms(2026, 7, 21, 9, 0, 0)
            .single()
            .expect("valid datetime");
        assert!(expr.matches(&t));
    }

    #[test]
    fn test_cron_step_expression_matches_expected_values() {
        let expr = CronExpression::parse("*/15 * * * *").expect("parse should succeed");
        let t_match = chrono::Utc
            .with_ymd_and_hms(2026, 1, 1, 10, 30, 0)
            .single()
            .expect("valid datetime");
        let t_no_match = chrono::Utc
            .with_ymd_and_hms(2026, 1, 1, 10, 31, 0)
            .single()
            .expect("valid datetime");
        assert!(expr.matches(&t_match));
        assert!(!expr.matches(&t_no_match));
    }

    #[test]
    fn test_cron_next_run_respects_minute_precision() {
        let mut scheduler = CronScheduler::new();
        scheduler
            .add_schedule("5 9 * * *")
            .expect("schedule should parse");
        let after = chrono::Utc
            .with_ymd_and_hms(2026, 1, 1, 9, 4, 58)
            .single()
            .expect("valid datetime");
        let next = scheduler.next_run(&after).expect("next run should exist");
        let expected = chrono::Utc
            .with_ymd_and_hms(2026, 1, 1, 9, 5, 0)
            .single()
            .expect("valid datetime");
        assert_eq!(next, expected);
    }

    #[test]
    fn test_queue_complete_success() {
        let mut queue = ScanQueue::new(10);
        let scan = ScheduledScan {
            id: "s1".to_string(),
            target: "example.com".to_string(),
            scan_type: ScanType::PortScan,
            scheduled_at: "2026-01-01T00:00:00Z".to_string(),
            status: ScheduleStatus::Pending,
            priority: Priority::Normal,
            options: ScanOptions {
                ports: None,
                concurrency: None,
                timeout: None,
                wordlist: None,
            },
        };
        queue.enqueue(scan).expect("scan should be queued");
        let started = queue.start_next().expect("should start");
        assert_eq!(started.id, "s1");
        assert!(queue.get_running().is_some());

        queue.complete("s1", true);
        assert!(queue.get_running().is_none());
    }

    #[test]
    fn test_queue_complete_failure() {
        let mut queue = ScanQueue::new(10);
        let scan = ScheduledScan {
            id: "s2".to_string(),
            target: "example.com".to_string(),
            scan_type: ScanType::PortScan,
            scheduled_at: "2026-01-01T00:00:00Z".to_string(),
            status: ScheduleStatus::Pending,
            priority: Priority::Normal,
            options: ScanOptions {
                ports: None,
                concurrency: None,
                timeout: None,
                wordlist: None,
            },
        };
        queue.enqueue(scan).expect("scan should be queued");
        let _ = queue.start_next().expect("should start");

        queue.complete("s2", false);
        assert!(queue.get_running().is_none());
    }
}
