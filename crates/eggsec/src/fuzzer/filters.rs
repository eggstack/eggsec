//! Response filtering system for fuzzing results
//!
//! This module provides a flexible filtering system that can exclude responses
//! based on various criteria like status code, response size, word count,
//! line count, response time, and regex patterns. This is similar to ffuf's
//! filtering capabilities.

use crate::fuzzer::engine::FuzzResult;
use regex::Regex;

/// A filter that can be applied to fuzz results
#[derive(Debug, Clone)]
pub enum PayloadFilter {
    /// Filter by HTTP status code (exclude if matches)
    StatusCode(Vec<u16>),
    /// Filter by response size (exclude if size matches any in list)
    ResponseSize(Vec<u64>),
    /// Filter by response size range (exclude if within range)
    ResponseSizeRange { min: u64, max: u64 },
    /// Filter by word count (exclude if word count matches)
    WordCount(Vec<u64>),
    /// Filter by word count range
    WordCountRange { min: u64, max: u64 },
    /// Filter by line count (exclude if line count matches)
    LineCount(Vec<u64>),
    /// Filter by line count range
    LineCountRange { min: u64, max: u64 },
    /// Filter by response time in ms (exclude if <= threshold)
    ResponseTimeMax(u64),
    /// Filter by response time range in ms
    ResponseTimeRange { min: u64, max: u64 },
    /// Filter by regex pattern on response body
    Regex(Regex),
    /// Filter by response size greater than threshold
    SizeGreaterThan(u64),
    /// Filter by response size less than threshold
    SizeLessThan(u64),
}

/// Chain of filters applied in sequence
/// If any filter matches, the result is filtered out
#[derive(Debug, Clone, Default)]
pub struct FilterChain {
    filters: Vec<PayloadFilter>,
}

impl FilterChain {
    /// Create a new empty filter chain
    pub fn new() -> Self {
        FilterChain {
            filters: Vec::new(),
        }
    }

    /// Add a status code filter
    pub fn add_status_filter(&mut self, codes: Vec<u16>) {
        if !codes.is_empty() {
            self.filters.push(PayloadFilter::StatusCode(codes));
        }
    }

    /// Add a response size filter
    pub fn add_size_filter(&mut self, sizes: Vec<u64>) {
        if !sizes.is_empty() {
            self.filters.push(PayloadFilter::ResponseSize(sizes));
        }
    }

    /// Add a word count filter
    pub fn add_word_filter(&mut self, words: Vec<u64>) {
        if !words.is_empty() {
            self.filters.push(PayloadFilter::WordCount(words));
        }
    }

    /// Add a line count filter
    pub fn add_line_filter(&mut self, lines: Vec<u64>) {
        if !lines.is_empty() {
            self.filters.push(PayloadFilter::LineCount(lines));
        }
    }

    /// Add a response time max filter (exclude if <= threshold)
    pub fn add_time_filter(&mut self, time_ms: u64) {
        self.filters.push(PayloadFilter::ResponseTimeMax(time_ms));
    }

    /// Add a regex filter
    pub fn add_regex_filter(&mut self, pattern: String) {
        if let Ok(regex) = Regex::new(&pattern) {
            self.filters.push(PayloadFilter::Regex(regex));
        }
    }

    /// Add size greater than filter
    pub fn add_size_greater_than(&mut self, threshold: u64) {
        self.filters.push(PayloadFilter::SizeGreaterThan(threshold));
    }

    /// Add size less than filter
    pub fn add_size_less_than(&mut self, threshold: u64) {
        self.filters.push(PayloadFilter::SizeLessThan(threshold));
    }

    /// Check if a fuzz result should be filtered out
    /// Returns true if the result should be EXCLUDED
    pub fn should_filter(&self, result: &FuzzResult) -> bool {
        for filter in &self.filters {
            if self.filter_matches(filter, result) {
                return true;
            }
        }
        false
    }

    /// Check if any filter matches the result
    fn filter_matches(&self, filter: &PayloadFilter, result: &FuzzResult) -> bool {
        match filter {
            PayloadFilter::StatusCode(codes) => codes.contains(&result.status_code),
            PayloadFilter::ResponseSize(sizes) => {
                if let Some(size) = result.response_length {
                    sizes.contains(&size)
                } else {
                    false
                }
            }
            PayloadFilter::ResponseSizeRange { min, max } => {
                if let Some(size) = result.response_length {
                    size >= *min && size <= *max
                } else {
                    false
                }
            }
            PayloadFilter::WordCount(counts) => {
                let words = if let Some(ref body) = result.response_body {
                    body.split_whitespace().count() as u64
                } else {
                    result.response_length.unwrap_or(1).saturating_sub(1) / 5
                };
                counts.contains(&words)
            }
            PayloadFilter::WordCountRange { min, max } => {
                let words = if let Some(ref body) = result.response_body {
                    body.split_whitespace().count() as u64
                } else {
                    result.response_length.unwrap_or(1).saturating_sub(1) / 5
                };
                words >= *min && words <= *max
            }
            PayloadFilter::LineCount(counts) => {
                let lines = if let Some(ref body) = result.response_body {
                    body.lines().count() as u64
                } else {
                    result.response_length.unwrap_or(1).saturating_sub(1) / 30
                };
                counts.contains(&lines)
            }
            PayloadFilter::LineCountRange { min, max } => {
                let lines = if let Some(ref body) = result.response_body {
                    body.lines().count() as u64
                } else {
                    result.response_length.unwrap_or(1).saturating_sub(1) / 30
                };
                lines >= *min && lines <= *max
            }
            PayloadFilter::ResponseTimeMax(time) => result.response_time_ms <= *time,
            PayloadFilter::ResponseTimeRange { min, max } => {
                result.response_time_ms >= *min && result.response_time_ms <= *max
            }
            PayloadFilter::Regex(regex) => {
                // Match regex against response body if available
                if let Some(ref body) = result.response_body {
                    regex.is_match(body)
                } else {
                    false
                }
            }
            PayloadFilter::SizeGreaterThan(threshold) => {
                if let Some(size) = result.response_length {
                    size > *threshold
                } else {
                    false
                }
            }
            PayloadFilter::SizeLessThan(threshold) => {
                if let Some(size) = result.response_length {
                    size < *threshold
                } else {
                    false
                }
            }
        }
    }

    /// Get the number of filters in the chain
    pub fn len(&self) -> usize {
        self.filters.len()
    }

    /// Check if the filter chain is empty
    pub fn is_empty(&self) -> bool {
        self.filters.is_empty()
    }
}

/// Helper to parse comma-separated values into a Vec
pub fn parse_comma_separated<T: std::str::FromStr>(input: &str) -> Vec<T> {
    input
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect()
}

/// Parse a range string like "100-200" into (min, max)
pub fn parse_range(input: &str) -> Option<(u64, u64)> {
    let parts: Vec<&str> = input.split('-').collect();
    if parts.len() == 2 {
        let min = parts[0].trim().parse().ok()?;
        let max = parts[1].trim().parse().ok()?;
        Some((min, max))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fuzzer::payloads::{Payload, PayloadType};
    use crate::waf::types::Severity;

    fn make_result(status: u16, size: Option<u64>, time_ms: u64) -> FuzzResult {
        FuzzResult {
            payload: Payload {
                payload_type: PayloadType::Sqli,
                payload: "test".to_string(),
                description: "test".to_string(),
                severity: Severity::Info,
                tags: vec!["test".to_string()],
            },
            status_code: status,
            response_time_ms: time_ms,
            response_length: size,
            response_body: None,
            is_waf_blocked: false,
            is_anomaly: false,
            is_redos_suspected: false,
            leaks_found: Vec::new(),
            error: None,
            owasp_category: None,
            detected_severity: Severity::Info,
        }
    }

    fn make_result_with_body(status: u16, body: &str, time_ms: u64) -> FuzzResult {
        FuzzResult {
            payload: Payload {
                payload_type: PayloadType::Sqli,
                payload: "test".to_string(),
                description: "test".to_string(),
                severity: Severity::Info,
                tags: vec!["test".to_string()],
            },
            status_code: status,
            response_time_ms: time_ms,
            response_length: Some(body.len() as u64),
            response_body: Some(body.to_string()),
            is_waf_blocked: false,
            is_anomaly: false,
            is_redos_suspected: false,
            leaks_found: Vec::new(),
            error: None,
            owasp_category: None,
            detected_severity: Severity::Info,
        }
    }

    #[test]
    fn test_status_filter() {
        let mut chain = FilterChain::new();
        chain.add_status_filter(vec![404, 500]);

        let result_404 = make_result(404, Some(100), 10);
        let result_200 = make_result(200, Some(100), 10);

        assert!(chain.should_filter(&result_404));
        assert!(!chain.should_filter(&result_200));
    }

    #[test]
    fn test_size_filter() {
        let mut chain = FilterChain::new();
        chain.add_size_filter(vec![100, 200]);

        let result_100 = make_result(200, Some(100), 10);
        let result_300 = make_result(200, Some(300), 10);

        assert!(chain.should_filter(&result_100));
        assert!(!chain.should_filter(&result_300));
    }

    #[test]
    fn test_size_greater_than() {
        let mut chain = FilterChain::new();
        chain.add_size_greater_than(1000);

        let result_small = make_result(200, Some(500), 10);
        let result_large = make_result(200, Some(1500), 10);

        assert!(!chain.should_filter(&result_small));
        assert!(chain.should_filter(&result_large));
    }

    #[test]
    fn test_parse_comma_separated() {
        let result: Vec<u16> = parse_comma_separated("200,404,500");
        assert_eq!(result, vec![200, 404, 500]);
    }

    #[test]
    fn test_parse_range() {
        let result = parse_range("100-200");
        assert_eq!(result, Some((100, 200)));
    }

    #[test]
    fn test_word_filter_uses_response_body_when_available() {
        let mut chain = FilterChain::new();
        chain.add_word_filter(vec![4]);
        let result = make_result_with_body(200, "one two three four", 10);
        assert!(chain.should_filter(&result));
    }

    #[test]
    fn test_line_filter_uses_response_body_when_available() {
        let mut chain = FilterChain::new();
        chain.add_line_filter(vec![3]);
        let result = make_result_with_body(200, "line1\nline2\nline3", 10);
        assert!(chain.should_filter(&result));
    }

    #[test]
    fn test_time_filter_excludes_responses_at_or_below_threshold() {
        let mut chain = FilterChain::new();
        chain.add_time_filter(100);

        let result_fast = make_result(200, Some(100), 50);
        let result_equal = make_result(200, Some(100), 100);
        let result_slow = make_result(200, Some(100), 101);

        assert!(chain.should_filter(&result_fast));
        assert!(chain.should_filter(&result_equal));
        assert!(!chain.should_filter(&result_slow));
    }
}
