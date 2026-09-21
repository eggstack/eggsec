//! Pure load-test metrics (Phase D WS5).
//!
//! [`Metrics`] is a single-threaded accumulator: the executor gives each
//! worker its own instance and merges them at the end, so completed requests
//! never serialize on a shared async mutex. [`LoadTestResults`] is the
//! serializable snapshot produced by [`Metrics::to_results`].

use crate::utils::preserve_all;
use hdrhistogram::Histogram;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Machine-readable transport-error category.
///
/// Classification is transport-neutral: the executor maps
/// [`eggsec_transport::TransportError`] (or backend-classified kinds) into
/// this enum without exposing concrete backend error types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadTestErrorKind {
    /// HTTP status outside 200..400 (counted in `status_codes`, not transport).
    HttpStatus,
    /// Fail-closed policy denial at any checkpoint.
    PolicyDenied,
    /// DNS resolution failure or invalid authority binding.
    Dns,
    /// Request timeout / deadline exhausted.
    Timeout,
    /// Connection failure (refused, reset, TLS handshake, ...).
    Connect,
    /// Request was invalid before dispatch (bad URL, header, ...).
    InvalidRequest,
    /// Backend failure that fits no narrower bucket.
    Backend,
    /// Run was cancelled before the request completed.
    Cancelled,
}

impl LoadTestErrorKind {
    /// Stable string key for [`LoadTestResults::error_kinds`].
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HttpStatus => "http_status",
            Self::PolicyDenied => "policy_denied",
            Self::Dns => "dns",
            Self::Timeout => "timeout",
            Self::Connect => "connect",
            Self::InvalidRequest => "invalid_request",
            Self::Backend => "backend",
            Self::Cancelled => "cancelled",
        }
    }

    /// Classify a transport-contract error without backend types.
    #[must_use]
    pub fn from_transport_error(err: &eggsec_transport::TransportError) -> Self {
        use eggsec_transport::TransportError;
        match err {
            TransportError::PolicyDenied { .. } => Self::PolicyDenied,
            TransportError::ResolutionFailed { .. } | TransportError::InvalidBinding { .. } => {
                Self::Dns
            }
            TransportError::InvalidRequest(_) => Self::InvalidRequest,
            TransportError::Backend(msg) => Self::classify_backend_message(msg),
        }
    }

    /// Classify a backend message string (used by the reqwest backend, which
    /// owns the only `reqwest::Error` contact point).
    #[must_use]
    pub fn classify_backend_message(msg: &str) -> Self {
        let lower = msg.to_ascii_lowercase();
        if lower.contains("timed out")
            || lower.contains("timeout")
            || lower.contains("deadline")
            || lower.contains("exhausted before hop")
        {
            Self::Timeout
        } else if lower.contains("dns")
            || lower.contains("resolve")
            || lower.contains("no addresses")
            || lower.contains("name or service not known")
        {
            Self::Dns
        } else if lower.contains("denied at")
            || lower.contains("not in allowed scope")
            || lower.contains("exclusion")
        {
            Self::PolicyDenied
        } else if lower.contains("connect")
            || lower.contains("connection")
            || lower.contains("refused")
            || lower.contains("reset")
            || lower.contains("tls")
            || lower.contains("certificate")
        {
            Self::Connect
        } else {
            Self::Backend
        }
    }

    /// Classify a `(checkpoint, reason)` denial without constructing an error.
    #[must_use]
    pub fn from_checkpoint(checkpoint: eggsec_transport::PolicyCheckpoint) -> Self {
        use eggsec_transport::PolicyCheckpoint;
        match checkpoint {
            PolicyCheckpoint::Dns | PolicyCheckpoint::Reresolution => Self::Dns,
            PolicyCheckpoint::InitialUrl | PolicyCheckpoint::TlsConsistency => Self::InvalidRequest,
            _ => Self::PolicyDenied,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub status_codes: FxHashMap<u16, u64>,
    pub errors: Vec<String>,
    /// Transport-error category counts (new in Phase D; defaults empty so
    /// pre-Phase-D serialized results still deserialize).
    #[serde(default)]
    pub error_kinds: FxHashMap<String, u64>,
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
            writeln!(f, "status codes")?;
            let mut sorted_codes: Vec<_> = self.status_codes.iter().collect();
            sorted_codes.sort_by_key(|(code, _)| *code);
            for (code, count) in &sorted_codes {
                writeln!(f, "\t{}: {}", code, count)?;
            }
        }

        if !self.error_kinds.is_empty() {
            writeln!(f, "error kinds")?;
            let mut sorted: Vec<_> = self.error_kinds.iter().collect();
            sorted.sort_by(|a, b| a.0.cmp(b.0));
            for (kind, count) in &sorted {
                writeln!(f, "\t{}: {}", kind, count)?;
            }
        }

        if !self.errors.is_empty() {
            writeln!(f, "errors (first 5)")?;
            for error in self.errors.iter().take(5) {
                writeln!(f, "\t{}", preserve_all(error, 60))?;
            }
        }

        Ok(())
    }
}

/// Single-threaded latency/status/error accumulator.
///
/// Not `Sync`: each executor worker owns one and the run merges them at the
/// end, so hot-path recording never touches a shared mutex.
///
/// Phase C: error-kind counters are keyed by [`LoadTestErrorKind`] internally
/// (no per-record `String` allocation); the stable `String`-keyed map is
/// materialized only in [`Metrics::to_results`], so the serialized
/// [`LoadTestResults`] shape is unchanged.
#[derive(Debug)]
pub struct Metrics {
    histogram: Histogram<u64>,
    successful: u64,
    failed: u64,
    status_codes: FxHashMap<u16, u64>,
    error_kind_counts: FxHashMap<LoadTestErrorKind, u64>,
    errors: Vec<String>,
    target_url: String,
}

impl Metrics {
    pub fn new(target_url: String) -> Self {
        Self {
            histogram: Histogram::new(3).expect("precision 3 is valid for hdrhistogram"),

            successful: 0,
            failed: 0,
            status_codes: FxHashMap::default(),
            error_kind_counts: FxHashMap::default(),
            errors: Vec::new(),
            target_url,
        }
    }

    /// Total recorded requests (saturating: success + failure cannot wrap).
    #[must_use]
    pub fn total(&self) -> u64 {
        self.successful.saturating_add(self.failed)
    }

    fn record_latency(&mut self, latency: Duration) {
        // `as_millis` saturates at u128::MAX; the histogram rejects values
        // above its max — both paths log and continue, never panic.
        let latency_ms = latency.as_millis().min(u64::MAX as u128) as u64;
        if let Err(e) = self.histogram.record(latency_ms) {
            tracing::warn!("Failed to record latency {}: {}", latency_ms, e);
        }
    }

    fn push_error(&mut self, message: String) {
        if self.errors.len() < 1000 {
            self.errors.push(message);
        }
    }

    fn bump_kind(&mut self, kind: LoadTestErrorKind) {
        let count = self.error_kind_counts.entry(kind).or_insert(0);
        *count = count.saturating_add(1);
    }

    pub fn record_http_response(&mut self, latency: Duration, status_code: u16) {
        self.record_latency(latency);
        let count = self.status_codes.entry(status_code).or_insert(0);
        *count = count.saturating_add(1);

        if (200..400).contains(&status_code) {
            self.successful = self.successful.saturating_add(1);
        } else {
            self.failed = self.failed.saturating_add(1);
            self.bump_kind(LoadTestErrorKind::HttpStatus);
            self.push_error(format!("HTTP {status_code}"));
        }
    }

    pub fn record_failure(&mut self, error: String, latency: Duration) {
        self.record_latency(latency);
        self.failed = self.failed.saturating_add(1);
        self.bump_kind(LoadTestErrorKind::classify_backend_message(&error));
        self.push_error(error);
    }

    /// Record a transport-contract failure with an explicit category.
    pub fn record_transport_error(
        &mut self,
        kind: LoadTestErrorKind,
        message: String,
        latency: Duration,
    ) {
        self.record_latency(latency);
        self.failed = self.failed.saturating_add(1);
        self.bump_kind(kind);
        self.push_error(message);
    }

    /// Record a cancelled request (partial-run accounting).
    pub fn record_cancelled(&mut self, latency: Duration) {
        self.record_latency(latency);
        self.failed = self.failed.saturating_add(1);
        self.bump_kind(LoadTestErrorKind::Cancelled);
        self.push_error("cancelled".to_string());
    }

    /// Merge another worker's accumulator into this one (saturating).
    pub fn merge(&mut self, other: &Metrics) {
        if let Err(e) = self.histogram.add(&other.histogram) {
            tracing::warn!("Failed to merge loadtest histogram: {}", e);
        }
        self.successful = self.successful.saturating_add(other.successful);
        self.failed = self.failed.saturating_add(other.failed);
        for (code, count) in &other.status_codes {
            let slot = self.status_codes.entry(*code).or_insert(0);
            *slot = slot.saturating_add(*count);
        }
        for (kind, count) in &other.error_kind_counts {
            let slot = self.error_kind_counts.entry(*kind).or_insert(0);
            *slot = slot.saturating_add(*count);
        }
        for err in &other.errors {
            if self.errors.len() >= 1000 {
                break;
            }
            self.errors.push(err.clone());
        }
    }

    pub fn to_results(&self, total_duration: Duration) -> LoadTestResults {
        let total = self.total();
        let duration_secs = total_duration.as_secs_f64();
        // Stable String-keyed view, materialized once per run (not per
        // request): the serialized shape is unchanged.
        let error_kinds: FxHashMap<String, u64> = self
            .error_kind_counts
            .iter()
            .map(|(kind, count)| (kind.as_str().to_string(), *count))
            .collect();

        LoadTestResults {
            target_url: self.target_url.clone(),
            total_requests: total,
            successful_requests: self.successful,
            failed_requests: self.failed,
            total_duration_ms: total_duration.as_millis().min(u64::MAX as u128) as u64,
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
            error_kinds,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn totals_account_success_and_failure() {
        let mut m = Metrics::new("http://example.com/".to_string());
        m.record_http_response(Duration::from_millis(5), 200);
        m.record_http_response(Duration::from_millis(6), 404);
        m.record_failure("boom".to_string(), Duration::from_millis(7));
        assert_eq!(m.total(), 3);
        let r = m.to_results(Duration::from_secs(1));
        assert_eq!(r.total_requests, 3);
        assert_eq!(r.successful_requests, 1);
        assert_eq!(r.failed_requests, 2);
    }

    #[test]
    fn status_distribution_counts_per_code() {
        let mut m = Metrics::new("http://example.com/".to_string());
        for _ in 0..3 {
            m.record_http_response(Duration::from_millis(1), 200);
        }
        for _ in 0..2 {
            m.record_http_response(Duration::from_millis(1), 500);
        }
        let r = m.to_results(Duration::from_secs(1));
        assert_eq!(r.status_codes.get(&200), Some(&3));
        assert_eq!(r.status_codes.get(&500), Some(&2));
        assert_eq!(r.error_kinds.get("http_status"), Some(&2));
    }

    #[test]
    fn transport_error_categorization() {
        use eggsec_transport::{PolicyCheckpoint, TransportError};
        assert_eq!(
            LoadTestErrorKind::from_transport_error(&TransportError::denied(
                PolicyCheckpoint::Host,
                "nope"
            )),
            LoadTestErrorKind::PolicyDenied
        );
        assert_eq!(
            LoadTestErrorKind::from_transport_error(&TransportError::ResolutionFailed {
                host: "h".to_string(),
                reason: "no addresses".to_string(),
            }),
            LoadTestErrorKind::Dns
        );
        assert_eq!(
            LoadTestErrorKind::from_transport_error(&TransportError::InvalidRequest(
                "bad".to_string()
            )),
            LoadTestErrorKind::InvalidRequest
        );
        assert_eq!(
            LoadTestErrorKind::classify_backend_message("request timed out after 5s"),
            LoadTestErrorKind::Timeout
        );
        assert_eq!(
            LoadTestErrorKind::classify_backend_message("connection refused"),
            LoadTestErrorKind::Connect
        );
        let mut m = Metrics::new("http://example.com/".to_string());
        m.record_transport_error(
            LoadTestErrorKind::PolicyDenied,
            "policy denied at host: nope".to_string(),
            Duration::from_millis(2),
        );
        let r = m.to_results(Duration::from_secs(1));
        assert_eq!(r.error_kinds.get("policy_denied"), Some(&1));
    }

    #[test]
    fn latency_percentiles_cover_histogram() {
        let mut m = Metrics::new("http://example.com/".to_string());
        for ms in [1u64, 2, 3, 4, 5, 10, 20, 50, 100] {
            m.record_http_response(Duration::from_millis(ms), 200);
        }
        let r = m.to_results(Duration::from_secs(1));
        assert!(r.latency_min_ms <= r.latency_p50_ms);
        assert!(r.latency_p50_ms <= r.latency_p95_ms);
        assert!(r.latency_p95_ms <= r.latency_p99_ms);
        assert!(r.latency_p99_ms <= r.latency_max_ms);
        assert!(r.latency_mean_ms > 0.0);
    }

    #[test]
    fn cancellation_and_partial_totals() {
        let mut m = Metrics::new("http://example.com/".to_string());
        m.record_http_response(Duration::from_millis(1), 200);
        m.record_cancelled(Duration::from_millis(1));
        let r = m.to_results(Duration::from_secs(1));
        assert_eq!(r.total_requests, 2);
        assert_eq!(r.successful_requests, 1);
        assert_eq!(r.failed_requests, 1);
        assert_eq!(r.error_kinds.get("cancelled"), Some(&1));
    }

    #[test]
    fn high_concurrency_merge_is_deterministic() {
        let mut base = Metrics::new("http://example.com/".to_string());
        let mut workers = Vec::new();
        for w in 0..8u16 {
            let mut m = Metrics::new("http://example.com/".to_string());
            for i in 0..25 {
                let code = if (w as usize + i) % 5 == 0 { 500 } else { 200 };
                m.record_http_response(Duration::from_millis(1 + (i as u64 % 5)), code);
            }
            workers.push(m);
        }
        for w in workers {
            base.merge(&w);
        }
        let r = base.to_results(Duration::from_secs(1));
        assert_eq!(r.total_requests, 200);
        assert_eq!(r.successful_requests + r.failed_requests, r.total_requests);
        let status_total: u64 = r.status_codes.values().copied().sum();
        assert_eq!(status_total, r.total_requests);
    }

    #[test]
    fn zero_and_one_request_boundaries() {
        let m = Metrics::new("http://example.com/".to_string());
        let r = m.to_results(Duration::from_secs(1));
        assert_eq!(r.total_requests, 0);
        assert_eq!(r.requests_per_second, 0.0);

        let mut one = Metrics::new("http://example.com/".to_string());
        one.record_http_response(Duration::from_millis(3), 200);
        let r = one.to_results(Duration::ZERO);
        assert_eq!(r.total_requests, 1);
        assert_eq!(r.requests_per_second, 0.0);

        let r = one.to_results(Duration::from_millis(500));
        assert!((r.requests_per_second - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn saturation_never_wraps() {
        let mut m = Metrics::new("http://example.com/".to_string());
        m.successful = u64::MAX;
        m.failed = 10;
        assert_eq!(m.total(), u64::MAX);
        m.record_http_response(Duration::from_millis(1), 200);
        assert_eq!(m.successful, u64::MAX);
        let r = m.to_results(Duration::from_secs(1));
        assert_eq!(r.total_requests, u64::MAX);
    }

    #[test]
    fn error_cap_and_kind_counts_stay_bounded() {
        let mut m = Metrics::new("http://example.com/".to_string());
        for i in 0..1500 {
            m.record_failure(format!("err-{i}"), Duration::from_millis(1));
        }
        assert_eq!(m.errors.len(), 1000);
        let r = m.to_results(Duration::from_secs(1));
        assert_eq!(r.failed_requests, 1500);
        let kinds: u64 = r.error_kinds.values().copied().sum();
        assert_eq!(kinds, 1500);
    }

    #[test]
    fn legacy_results_without_error_kinds_still_deserialize() {
        let json = r#"{"target_url":"http://example.com/","total_requests":1,"successful_requests":1,"failed_requests":0,"total_duration_ms":10,"requests_per_second":100.0,"latency_min_ms":1.0,"latency_max_ms":1.0,"latency_mean_ms":1.0,"latency_p50_ms":1.0,"latency_p90_ms":1.0,"latency_p95_ms":1.0,"latency_p99_ms":1.0,"status_codes":{"200":1},"errors":[]}"#;
        let r: LoadTestResults = serde_json::from_str(json).expect("legacy shape");
        assert!(r.error_kinds.is_empty());
    }
}
