use crate::error::Result;
use crate::utils::create_insecure_http_client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingTestResult {
    pub target: String,
    pub timing_vulnerable: bool,
    pub measurements: Vec<TimingMeasurement>,
    pub analysis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingMeasurement {
    pub input: String,
    pub avg_response_time_ms: f64,
    pub samples: usize,
}

pub struct TimingTester {
    client: reqwest::Client,
}

impl TimingTester {
    pub fn new(timeout_secs: u64) -> Result<Self> {
        let client = create_insecure_http_client(timeout_secs)?;
        Ok(Self { client })
    }

    pub async fn test(&self, target: &str) -> Result<TimingTestResult> {
        let mut result = TimingTestResult {
            target: target.to_string(),
            timing_vulnerable: false,
            measurements: Vec::new(),
            analysis: String::new(),
        };

        let test_passwords = vec!["a", "aa", "aaa", "aaaa", "aaaaa", "wrong", "wrongpassword"];

        let mut measurements = Vec::new();
        for password in &test_passwords {
            let avg_time = self.measure_timing(target, "admin", password, 10).await;
            measurements.push(TimingMeasurement {
                input: password.to_string(),
                avg_response_time_ms: avg_time,
                samples: 10,
            });
        }

        if measurements.len() >= 2 {
            let times: Vec<f64> = measurements
                .iter()
                .map(|m| m.avg_response_time_ms)
                .collect();
            let max_time = times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let min_time = times.iter().cloned().fold(f64::INFINITY, f64::min);
            let diff = max_time - min_time;

            if diff > 50.0 {
                result.timing_vulnerable = true;
                result.analysis = format!(
                    "Significant timing difference detected: {:.2}ms (max: {:.2}ms, min: {:.2}ms)",
                    diff, max_time, min_time
                );
            } else {
                result.analysis =
                    format!("No significant timing difference: {:.2}ms variance", diff);
            }
        }

        result.measurements = measurements;

        Ok(result)
    }

    async fn measure_timing(
        &self,
        target: &str,
        username: &str,
        password: &str,
        samples: usize,
    ) -> f64 {
        let mut total_ms = 0.0;
        let mut successful = 0;

        for _ in 0..samples {
            let start = Instant::now();
            let result = self
                .client
                .post(target)
                .header(
                    reqwest::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(format!("username={}&password={}", username, password))
                .send()
                .await;
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;

            if result.is_ok() {
                total_ms += elapsed;
                successful += 1;
            }
        }

        if successful > 0 {
            total_ms / successful as f64
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timing_tester_creation() {
        let tester = TimingTester::new(10);
        assert!(tester.is_ok());
    }

    #[test]
    fn test_timing_result_default() {
        let result = TimingTestResult {
            target: "http://example.com/login".to_string(),
            timing_vulnerable: false,
            measurements: Vec::new(),
            analysis: String::new(),
        };
        assert!(!result.timing_vulnerable);
    }

    #[test]
    fn test_timing_measurement_creation() {
        let measurement = TimingMeasurement {
            input: "test".to_string(),
            avg_response_time_ms: 150.5,
            samples: 10,
        };
        assert_eq!(measurement.avg_response_time_ms, 150.5);
    }
}
