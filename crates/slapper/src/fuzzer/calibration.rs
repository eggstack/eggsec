//! Auto-calibration system for smart response filtering
//!
//! This module provides ffuf-style calibration that samples baseline responses
//! before fuzzing to automatically configure filters. This helps eliminate
//! false positives by learning what "normal" responses look like.

use crate::cli::FuzzArgs;
use crate::error::Result;
use crate::fuzzer::filters::FilterChain;
use crate::fuzzer::engine::FuzzResult;
use reqwest::Client;
use std::time::{Duration, Instant};

/// Result of the calibration process
#[derive(Debug, Clone)]
pub struct CalibrationResult {
    /// Filters derived from calibration samples
    pub filter_chain: FilterChain,
    /// Baseline response statistics
    pub baseline_stats: BaselineStats,
    /// Number of calibration samples taken
    pub samples_taken: usize,
}

/// Statistical summary of baseline responses
#[derive(Debug, Clone)]
pub struct BaselineStats {
    /// Set of status codes observed in baseline
    pub status_codes: Vec<u16>,
    /// Average response size in bytes
    pub avg_size: u64,
    /// Min response size observed
    pub min_size: u64,
    /// Max response size observed
    pub max_size: u64,
    /// Average word count
    pub avg_words: u64,
    /// Average line count
    pub avg_lines: u64,
    /// Average response time in ms
    pub avg_time_ms: u64,
    /// Min response time in ms
    pub min_time_ms: u64,
    /// Max response time in ms
    pub max_time_ms: u64,
}

impl BaselineStats {
    fn new() -> Self {
        BaselineStats {
            status_codes: Vec::new(),
            avg_size: 0,
            min_size: u64::MAX,
            max_size: 0,
            avg_words: 0,
            avg_lines: 0,
            avg_time_ms: 0,
            min_time_ms: u64::MAX,
            max_time_ms: 0,
        }
    }
}

/// Auto-calibration system that samples baseline responses
pub struct Calibrator {
    client: Client,
    args: FuzzArgs,
    samples: Vec<FuzzResult>,
}

impl Calibrator {
    /// Create a new Calibrator with the given HTTP client and arguments
    pub fn new(client: Client, args: FuzzArgs) -> Self {
        Calibrator {
            client,
            args,
            samples: Vec::new(),
        }
    }

    /// Run calibration by sampling baseline responses
    ///
    /// Sends requests with random/dummy payloads to establish baseline behavior.
    /// The responses are analyzed to create filters that exclude similar responses.
    pub async fn calibrate(&mut self) -> Result<CalibrationResult> {
        let url = self.args.url.clone();
        let sample_count = 10; // Number of calibration samples

        eprintln!("Running auto-calibration with {} samples...", sample_count);

        let mut stats = BaselineStats::new();
        let mut filter_chain = FilterChain::new();

        for i in 0..sample_count {
            let sample_payload = format!("FUZZ{}", i);
            let test_url = self.inject_payload(&url, &sample_payload);

            let start = Instant::now();

            match self.send_request(&test_url).await {
                Ok(response) => {
                    let elapsed = start.elapsed();
                    let status = response.status().as_u16();
                    let body = response.text().await.unwrap_or_default();

                    let words = body.split_whitespace().count() as u64;
                    let lines = body.lines().count() as u64;
                    let size = body.len() as u64;
                    let time_ms = elapsed.as_millis() as u64;

                    // Update stats
                    if !stats.status_codes.contains(&status) {
                        stats.status_codes.push(status);
                    }
                    stats.min_size = stats.min_size.min(size);
                    stats.max_size = stats.max_size.max(size);
                    stats.min_time_ms = stats.min_time_ms.min(time_ms);
                    stats.max_time_ms = stats.max_time_ms.max(time_ms);

                    // Create a FuzzResult for filtering
                    let result = FuzzResult {
                        payload: crate::fuzzer::payloads::Payload {
                            payload: sample_payload,
                            payload_type: crate::fuzzer::payloads::PayloadType::Sqli,
                            description: "Calibration sample".to_string(),
                            severity: crate::waf::types::Severity::Info,
                            tags: Vec::new(),
                        },
                        status_code: status,
                        response_time_ms: time_ms,
                        response_length: Some(size),
                        is_waf_blocked: false,
                        is_anomaly: false,
                        is_redos_suspected: false,
                        leaks_found: Vec::new(),
                        error: None,
                        owasp_category: None,
                        detected_severity: crate::waf::types::Severity::Info,
                    };

                    self.samples.push(result.clone());
                    stats.avg_size = if i == 0 { size } else { (stats.avg_size * i as u64 + size) / (i + 1) as u64 };
                    stats.avg_words = if i == 0 { words } else { (stats.avg_words * i as u64 + words) / (i + 1) as u64 };
                    stats.avg_lines = if i == 0 { lines } else { (stats.avg_lines * i as u64 + lines) / (i + 1) as u64 };
                    stats.avg_time_ms = if i == 0 { time_ms } else { (stats.avg_time_ms * i as u64 + time_ms) / (i + 1) as u64 };
                }
                Err(e) => {
                    eprintln!("Calibration sample {} failed: {}", i, e);
                }
            }

            // Small delay between samples
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Build filters based on observed baseline
        // Exclude baseline status codes
        if !stats.status_codes.is_empty() {
            filter_chain.add_status_filter(stats.status_codes.clone());
        }

        eprintln!(
            "Calibration complete: status_codes={:?}, avg_size={}, avg_time={}ms",
            stats.status_codes, stats.avg_size, stats.avg_time_ms
        );

        Ok(CalibrationResult {
            filter_chain,
            baseline_stats: stats,
            samples_taken: self.samples.len(),
        })
    }

    /// Inject payload into URL (replace FUZZ keyword or append as parameter)
    fn inject_payload(&self, url: &str, payload: &str) -> String {
        if url.contains("FUZZ") {
            url.replace("FUZZ", payload)
        } else if url.contains('?') {
            format!("{}&FUZZ={}", url, urlencoding::encode(payload))
        } else {
            format!("{}?FUZZ={}", url, urlencoding::encode(payload))
        }
    }

    /// Send HTTP request and return response
    async fn send_request(&self, url: &str) -> Result<reqwest::Response> {
        let method = self.args.method.parse::<reqwest::Method>().unwrap_or(reqwest::Method::GET);
        let request = self.client.request(method, url);
        Ok(request.send().await?)
    }
}

impl CalibrationResult {
    /// Check if a fuzz result should be filtered out (is similar to baseline)
    pub fn should_filter(&self, result: &FuzzResult) -> bool {
        self.filter_chain.should_filter(result)
    }
}
