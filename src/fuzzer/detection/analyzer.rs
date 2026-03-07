#![allow(dead_code)]

use std::time::Duration;

#[derive(Debug, Clone)]
pub struct TimingResult {
    pub response_time_ms: u64,
    pub is_anomaly: bool,
    pub is_redos_suspected: bool,
    pub anomaly_factor: f64,
}

#[derive(Debug, Clone)]
pub struct TimingAnalyzer {
    baseline_ms: Option<f64>,
    samples: Vec<f64>,
    spike_threshold: f64,
    redos_threshold_ms: u64,
    min_samples_for_baseline: usize,
}

impl Default for TimingAnalyzer {
    fn default() -> Self {
        Self {
            baseline_ms: None,
            samples: Vec::new(),
            spike_threshold: 3.0,
            redos_threshold_ms: 5000,
            min_samples_for_baseline: 20,
        }
    }
}

impl TimingAnalyzer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_thresholds(spike_threshold: f64, redos_threshold_ms: u64) -> Self {
        Self {
            baseline_ms: None,
            samples: Vec::new(),
            spike_threshold,
            redos_threshold_ms,
            min_samples_for_baseline: 20,
        }
    }

    pub fn record(&mut self, duration: Duration) -> TimingResult {
        let response_time_ms = duration.as_millis() as f64;
        self.samples.push(response_time_ms);

        if self.samples.len() >= self.min_samples_for_baseline {
            self.update_baseline();
        }

        let (is_anomaly, anomaly_factor) = self.check_anomaly(response_time_ms);
        let is_redos_suspected = response_time_ms as u64 >= self.redos_threshold_ms;

        TimingResult {
            response_time_ms: response_time_ms as u64,
            is_anomaly,
            is_redos_suspected,
            anomaly_factor,
        }
    }

    fn update_baseline(&mut self) {
        if self.samples.is_empty() {
            return;
        }

        let sorted_samples: Vec<f64> = {
            let mut s = self.samples.clone();
            s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            s
        };

        let len = sorted_samples.len();
        let start = len / 4;
        let end = len * 3 / 4;

        if start >= end {
            return;
        }

        let iqr_samples: Vec<f64> = sorted_samples[start..end].to_vec();
        let sum: f64 = iqr_samples.iter().sum();
        self.baseline_ms = Some(sum / iqr_samples.len() as f64);
    }

    fn check_anomaly(&self, response_time_ms: f64) -> (bool, f64) {
        match self.baseline_ms {
            Some(baseline) if baseline > 0.0 => {
                let factor = response_time_ms / baseline;
                let is_anomaly = factor >= self.spike_threshold;
                (is_anomaly, factor)
            }
            _ => (false, 1.0),
        }
    }

    pub fn get_baseline(&self) -> Option<f64> {
        self.baseline_ms
    }

    pub fn get_stats(&self) -> Option<TimingStats> {
        if self.samples.is_empty() {
            return None;
        }

        let mut sorted = self.samples.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let len = sorted.len();
        let sum: f64 = sorted.iter().sum();
        let mean = sum / len as f64;

        let median = if len.is_multiple_of(2) {
            (sorted[len / 2 - 1] + sorted[len / 2]) / 2.0
        } else {
            sorted[len / 2]
        };

        let p90_idx = (len as f64 * 0.90) as usize;
        let p99_idx = (len as f64 * 0.99) as usize;
        let max_val = sorted[len - 1];

        Some(TimingStats {
            min: sorted[0],
            max: max_val,
            mean,
            median,
            p90: sorted.get(p90_idx).copied().unwrap_or(max_val),
            p99: sorted.get(p99_idx).copied().unwrap_or(max_val),
            sample_count: len,
        })
    }
}

#[derive(Debug, Clone)]
pub struct TimingStats {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub median: f64,
    pub p90: f64,
    pub p99: f64,
    pub sample_count: usize,
}
