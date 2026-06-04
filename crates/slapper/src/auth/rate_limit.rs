use crate::error::Result;
use crate::utils::create_insecure_http_client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitResult {
    pub target: String,
    pub rate_limited: bool,
    pub rate_limit_header: Option<String>,
    pub rate_limit_value: Option<String>,
    pub requests_until_limited: usize,
    pub bypass_techniques: Vec<RateLimitBypassResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitBypassResult {
    pub technique: String,
    pub successful: bool,
    pub details: String,
}

pub struct RateLimitTester {
    client: reqwest::Client,
}

impl RateLimitTester {
    pub fn new(timeout_secs: u64) -> Result<Self> {
        let client = create_insecure_http_client(timeout_secs)?;
        Ok(Self { client })
    }

    pub async fn test(&self, target: &str) -> Result<RateLimitResult> {
        let mut result = RateLimitResult {
            target: target.to_string(),
            rate_limited: false,
            rate_limit_header: None,
            rate_limit_value: None,
            requests_until_limited: 0,
            bypass_techniques: Vec::new(),
        };

        for i in 0..50 {
            let response = self
                .client
                .post(target)
                .header(
                    reqwest::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(format!("username=admin&password=wrong{}", i))
                .send()
                .await;

            match response {
                Ok(resp) => {
                    if resp.status() == crate::constants::STATUS_RATE_LIMITED {
                        result.rate_limited = true;
                        result.requests_until_limited = i + 1;

                        if let Some(limit) = resp.headers().get("X-RateLimit-Limit") {
                            result.rate_limit_header = Some("X-RateLimit-Limit".to_string());
                            result.rate_limit_value =
                                Some(limit.to_str().unwrap_or("").to_string());
                        }
                        if let Some(limit) = resp.headers().get("RateLimit-Limit") {
                            result.rate_limit_header = Some("RateLimit-Limit".to_string());
                            result.rate_limit_value =
                                Some(limit.to_str().unwrap_or("").to_string());
                        }
                        break;
                    }
                }
                Err(_) => {
                    result.rate_limited = true;
                    result.requests_until_limited = i + 1;
                    break;
                }
            }
        }

        result.bypass_techniques = self.test_bypass_techniques(target).await;

        Ok(result)
    }

    async fn test_bypass_techniques(&self, target: &str) -> Vec<RateLimitBypassResult> {
        let mut results = Vec::new();

        let xff_headers = [
            ("X-Forwarded-For", "1.1.1.1"),
            ("X-Real-IP", "1.1.1.1"),
            ("X-Client-IP", "1.1.1.1"),
            ("X-Originating-IP", "1.1.1.1"),
            ("X-Remote-IP", "1.1.1.1"),
        ];

        for (header, value) in xff_headers {
            let response = self
                .client
                .post(target)
                .header(
                    reqwest::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .header(header, value)
                .body("username=admin&password=wrong")
                .send()
                .await;

            let success = match response {
                Ok(resp) => resp.status() != 429,
                Err(_) => false,
            };

            results.push(RateLimitBypassResult {
                technique: format!("{} header", header),
                successful: success,
                details: if success {
                    format!("Bypassed rate limit using {} header", header)
                } else {
                    format!("Rate limit still enforced with {} header", header)
                },
            });
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_result_default() {
        let result = RateLimitResult {
            target: "http://example.com/login".to_string(),
            rate_limited: false,
            rate_limit_header: None,
            rate_limit_value: None,
            requests_until_limited: 0,
            bypass_techniques: Vec::new(),
        };
        assert!(!result.rate_limited);
    }

    #[test]
    fn test_bypass_result_creation() {
        let bypass = RateLimitBypassResult {
            technique: "X-Forwarded-For".to_string(),
            successful: true,
            details: "Bypassed".to_string(),
        };
        assert!(bypass.successful);
    }

    #[test]
    fn test_rate_limit_tester_creation() {
        let tester = RateLimitTester::new(10);
        assert!(tester.is_ok());
    }
}
