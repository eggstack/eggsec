use crate::utils::preserve_all;
use hdrhistogram::Histogram;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
pub struct LoadTestResults {
    pub target_url: String,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_duration_ms: u64,
    pub requests_per_second: f64,
    pub latency_min_ms: f64,
    pub latency_max_ms: f64,
    pub latency_mean_ms: f64,
    pub latency_p50_ms: f64,
    pub latency_p90_ms: f64,
    pub latency_p95_ms: f64,
    pub latency_p99_ms: f64,
    pub status_codes: std::collections::HashMap<u16, u64>,
    pub errors: Vec<String>,
}

impl std::fmt::Display for LoadTestResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Load Test Results")?;
        writeln!(f, "target: {}", preserve_all(&self.target_url, 60))?;
        writeln!(
            f,
            "requests: {} total, {} successful, {} failed",
            self.total_requests, self.successful_requests, self.failed_requests
        )?;
        writeln!(f, "rps: {:.2}", self.requests_per_second)?;
        writeln!(
            f,
            "duration: {:.2}s",
            self.total_duration_ms as f64 / 1000.0
        )?;
        writeln!(f, "latency: min={:.2}ms mean={:.2}ms p50={:.2}ms p90={:.2}ms p95={:.2}ms p99={:.2}ms max={:.2}ms",
            self.latency_min_ms, self.latency_mean_ms, self.latency_p50_ms,
            self.latency_p90_ms, self.latency_p95_ms, self.latency_p99_ms, self.latency_max_ms)?;

        if !self.status_codes.is_empty() {
            let _ = writeln!(f, "status codes");
            for (code, count) in &self.status_codes {
                writeln!(f, "\t{}: {}", code, count)?;
            }
        }

        if !self.errors.is_empty() {
            let _ = writeln!(f, "errors (first 5)");
            for error in self.errors.iter().take(5) {
                writeln!(f, "\t{}", preserve_all(error, 60))?;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Metrics {
    histogram: Histogram<u64>,
    pub successful: u64,
    pub failed: u64,
    pub status_codes: std::collections::HashMap<u16, u64>,
    pub errors: Vec<String>,
    target_url: String,
}

impl Metrics {
    pub fn new(target_url: String) -> Self {
        Self {
            histogram: Histogram::new(3).expect("Failed to create histogram"),
            successful: 0,
            failed: 0,
            status_codes: std::collections::HashMap::new(),
            errors: Vec::new(),
            target_url,
        }
    }

    pub fn record_success(&mut self, latency: Duration, status_code: u16) {
        let latency_ms = latency.as_millis() as u64;
        self.histogram.record(latency_ms).ok();
        self.successful += 1;
        *self.status_codes.entry(status_code).or_insert(0) += 1;
    }

    pub fn record_http_response(&mut self, latency: Duration, status_code: u16) {
        let latency_ms = latency.as_millis() as u64;
        self.histogram.record(latency_ms).ok();
        *self.status_codes.entry(status_code).or_insert(0) += 1;

        if (200..400).contains(&status_code) {
            self.successful += 1;
        } else {
            self.failed += 1;
            if self.errors.len() < 100 {
                self.errors.push(format!("HTTP {}", status_code));
            }
        }
    }

    pub fn record_failure(&mut self, error: String) {
        self.failed += 1;
        if self.errors.len() < 100 {
            self.errors.push(error);
        }
    }

    pub fn to_results(&self, total_duration: Duration) -> LoadTestResults {
        let total = self.successful + self.failed;
        let duration_secs = total_duration.as_secs_f64();

        LoadTestResults {
            target_url: self.target_url.clone(),
            total_requests: total,
            successful_requests: self.successful,
            failed_requests: self.failed,
            total_duration_ms: total_duration.as_millis() as u64,
            requests_per_second: if duration_secs > 0.0 {
                total as f64 / duration_secs
            } else {
                0.0
            },
            latency_min_ms: self.histogram.min() as f64,
            latency_max_ms: self.histogram.max() as f64,
            latency_mean_ms: self.histogram.mean(),
            latency_p50_ms: self.histogram.value_at_percentile(50.0) as f64,
            latency_p90_ms: self.histogram.value_at_percentile(90.0) as f64,
            latency_p95_ms: self.histogram.value_at_percentile(95.0) as f64,
            latency_p99_ms: self.histogram.value_at_percentile(99.0) as f64,
            status_codes: self.status_codes.clone(),
            errors: self.errors.clone(),
        }
    }
}
