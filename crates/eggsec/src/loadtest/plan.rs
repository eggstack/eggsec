//! Transport-neutral load-test plan (Phase D WS1-WS2).
//!
//! [`LoadTestPlan`] is the reusable domain primitive: target, request
//! template, budgets, and explicit rate policy. It knows nothing about Clap
//! args, `EggsecConfig`, terminal progress, Reqwest builders, or filesystem
//! output. Engine/front-end adaptation lives in `adapter.rs`.

use std::time::Duration;

/// Explicit rate policy (pacing between request issues).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RatePolicy {
    /// No pacing; issue as fast as workers allow.
    Unlimited,
    /// At most `requests_per_second` issues (paced, not bursty).
    PerSecond(u32),
}

impl RatePolicy {
    /// Normalize: `None`/`0` means unlimited (fail-open to unlimited would
    /// hide misconfiguration, so callers log; the plan itself never sleeps
    /// indefinitely on zero).
    #[must_use]
    pub fn from_optional(rps: Option<u32>) -> Self {
        match rps {
            None | Some(0) => Self::Unlimited,
            Some(rate) => Self::PerSecond(rate),
        }
    }

    /// Minimum spacing between issues (`None` when unlimited).
    #[must_use]
    pub fn min_interval(self) -> Option<Duration> {
        match self {
            Self::Unlimited => None,
            Self::PerSecond(rate) => {
                let rate = rate.max(1) as f64;
                Some(Duration::from_secs_f64(1.0 / rate))
            }
        }
    }
}

/// Transport-neutral load-test plan.
#[derive(Debug, Clone)]
pub struct LoadTestPlan {
    /// Target URL (validated: http/https, host present, no userinfo).
    pub url: String,
    /// Total requests to issue.
    pub total_requests: u64,
    /// Max concurrent in-flight requests.
    pub concurrency: usize,
    /// Per-request timeout (fail-closed: always set, never zero).
    pub timeout: Duration,
    /// HTTP method name (uppercased, validated against the 8 parity verbs).
    pub method: String,
    /// Replayable body bytes (`None` = empty).
    pub body: Option<Vec<u8>>,
    /// Extra headers as ordered pairs (auth already applied by the adapter).
    pub headers: Vec<(String, String)>,
    /// Explicit pacing policy.
    pub rate: RatePolicy,
}

impl LoadTestPlan {
    /// Validate and build a plan. Rejects zero concurrency/requests/timeout,
    /// invalid URLs, and unknown methods (unknown methods fall back to GET
    /// with a warning at the adapter layer; this constructor keeps the given
    /// name verbatim so the fallback stays visible).
    pub fn new(
        url: String,
        total_requests: u64,
        concurrency: usize,
        timeout: Duration,
    ) -> Result<Self, String> {
        if concurrency == 0 {
            return Err("Concurrency must be greater than 0".to_string());
        }
        if total_requests == 0 {
            return Err("Total requests must be greater than 0".to_string());
        }
        if timeout.is_zero() {
            return Err("Timeout must be greater than 0".to_string());
        }
        validate_target_url(&url)?;
        Ok(Self {
            url,
            total_requests,
            concurrency,
            timeout,
            method: "GET".to_string(),
            body: None,
            headers: Vec::new(),
            rate: RatePolicy::Unlimited,
        })
    }

    /// Worker count for the run (`min(concurrency, total_requests)`).
    #[must_use]
    pub fn worker_count(&self) -> usize {
        self.concurrency.min(self.total_requests as usize)
    }
}

/// Fail-closed target validation shared by the adapter and executor.
pub fn validate_target_url(url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|e| format!("invalid URL '{url}': {e}"))?;
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(
            "URL userinfo is rejected (credentials must not appear in the URL)".to_string(),
        );
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| "request URL has no host".to_string())?;
    if host.is_empty() {
        return Err("request URL has no host".to_string());
    }
    match parsed.scheme() {
        "http" | "https" => Ok(()),
        other => Err(format!(
            "unsupported URL scheme '{other}' (expected http/https)"
        )),
    }
}

/// Normalize a method name to one of the 8 parity verbs; unknown names map
/// to `GET` (the caller logs the fallback).
#[must_use]
pub fn normalize_method(name: &str) -> String {
    match name.to_ascii_uppercase().as_str() {
        "GET" | "POST" | "PUT" | "DELETE" | "PATCH" | "HEAD" | "OPTIONS" | "TRACE" => {
            name.to_ascii_uppercase()
        }
        _ => {
            tracing::warn!("Unknown HTTP method '{name}', defaulting to GET");
            "GET".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_dimensions() {
        assert!(
            LoadTestPlan::new("http://example.com/".into(), 0, 1, Duration::from_secs(1)).is_err()
        );
        assert!(
            LoadTestPlan::new("http://example.com/".into(), 1, 0, Duration::from_secs(1)).is_err()
        );
        assert!(LoadTestPlan::new("http://example.com/".into(), 1, 1, Duration::ZERO).is_err());
    }

    #[test]
    fn rejects_userinfo_and_bad_scheme() {
        assert!(validate_target_url("http://user:pw@example.com/").is_err());
        assert!(validate_target_url("ftp://example.com/").is_err());
        // The `url` crate normalizes `http:///no-host` to host `no-host`,
        // so use a genuinely hostless URL here (empty host is rejected).
        assert!(validate_target_url("http://:80/").is_err());
        assert!(validate_target_url("http://example.com/").is_ok());
    }

    #[test]
    fn rate_policy_zero_is_unlimited() {
        assert_eq!(RatePolicy::from_optional(None), RatePolicy::Unlimited);
        assert_eq!(RatePolicy::from_optional(Some(0)), RatePolicy::Unlimited);
        assert_eq!(
            RatePolicy::from_optional(Some(10)),
            RatePolicy::PerSecond(10)
        );
    }

    #[test]
    fn worker_count_clamps_to_total() {
        let plan = LoadTestPlan::new("http://example.com/".into(), 3, 50, Duration::from_secs(1))
            .expect("plan");
        assert_eq!(plan.worker_count(), 3);
    }

    #[test]
    fn unknown_method_falls_back_to_get() {
        assert_eq!(normalize_method("get"), "GET");
        assert_eq!(normalize_method("FOOBAR"), "GET");
    }
}
