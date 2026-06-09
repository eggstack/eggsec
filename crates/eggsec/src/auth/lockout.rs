use crate::error::Result;
use crate::utils::create_insecure_http_client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockoutDetectionResult {
    pub target: String,
    pub lockout_threshold: Option<usize>,
    pub lockout_duration_seconds: Option<u64>,
    pub lockout_type: LockoutType,
    pub attempts_before_lockout: usize,
    pub indicators: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LockoutType {
    HardLockout,
    SoftLockout,
    ProgressiveDelay,
    Captcha,
    None,
}

pub struct LockoutDetector {
    client: reqwest::Client,
}

impl LockoutDetector {
    pub fn new(timeout_secs: u64) -> Result<Self> {
        let client = create_insecure_http_client(timeout_secs)?;
        Ok(Self { client })
    }

    pub async fn detect(
        &self,
        target: &str,
        username: &str,
        max_attempts: usize,
    ) -> Result<LockoutDetectionResult> {
        let mut result = LockoutDetectionResult {
            target: target.to_string(),
            lockout_threshold: None,
            lockout_duration_seconds: None,
            lockout_type: LockoutType::None,
            attempts_before_lockout: 0,
            indicators: Vec::new(),
        };

        let mut prev_status: Option<u16> = None;

        for i in 0..max_attempts {
            let response = self
                .client
                .post(target)
                .header(
                    reqwest::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(format!("username={}&password=wrongpassword{}", username, i))
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    let body = resp.text().await.unwrap_or_default();

                    if let Some(prev) = prev_status {
                        if status != prev || self.is_lockout_response(&body) {
                            result.lockout_threshold = Some(i);
                            result.attempts_before_lockout = i;

                            result.lockout_type = self.classify_lockout(status, &body);
                            result.indicators = self.extract_indicators(status, &body);

                            if result.lockout_type != LockoutType::None {
                                break;
                            }
                        }
                    }
                    prev_status = Some(status);
                }
                Err(_) => {
                    result.lockout_threshold = Some(i);
                    result.attempts_before_lockout = i;
                    result.lockout_type = LockoutType::HardLockout;
                    result
                        .indicators
                        .push("Connection failed after attempts".to_string());
                    break;
                }
            }
        }

        Ok(result)
    }

    fn is_lockout_response(&self, body: &str) -> bool {
        let lower = body.to_lowercase();
        lower.contains("too many attempts")
            || lower.contains("account locked")
            || lower.contains("try again later")
            || lower.contains("rate limit")
            || lower.contains("captcha")
    }

    fn classify_lockout(&self, status: u16, body: &str) -> LockoutType {
        let lower = body.to_lowercase();

        if status == crate::constants::STATUS_LOCKED {
            return LockoutType::HardLockout;
        }
        if status == crate::constants::STATUS_RATE_LIMITED {
            return LockoutType::SoftLockout;
        }
        if lower.contains("captcha") {
            return LockoutType::Captcha;
        }
        if lower.contains("try again") || lower.contains("wait") {
            return LockoutType::ProgressiveDelay;
        }
        if lower.contains("locked") {
            return LockoutType::HardLockout;
        }

        LockoutType::None
    }

    fn extract_indicators(&self, status: u16, body: &str) -> Vec<String> {
        let mut indicators = Vec::new();
        indicators.push(format!("HTTP status: {}", status));

        let lower = body.to_lowercase();
        if lower.contains("locked") {
            indicators.push("account locked message".to_string());
        }
        if lower.contains("captcha") {
            indicators.push("captcha challenge".to_string());
        }
        if lower.contains("wait") || lower.contains("delay") {
            indicators.push("delay message".to_string());
        }

        indicators
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lockout_detector_creation() {
        let detector = LockoutDetector::new(10);
        assert!(detector.is_ok());
    }

    #[test]
    fn test_lockout_type_variants() {
        assert_eq!(LockoutType::HardLockout, LockoutType::HardLockout);
        assert_ne!(LockoutType::HardLockout, LockoutType::SoftLockout);
    }

    #[test]
    fn test_is_lockout_response_positive() {
        let detector = LockoutDetector::new(10).unwrap();
        assert!(detector.is_lockout_response("Too many attempts. Please try again later."));
        assert!(detector.is_lockout_response("Account locked for security reasons."));
        assert!(detector.is_lockout_response("Please complete the captcha."));
    }

    #[test]
    fn test_is_lockout_response_negative() {
        let detector = LockoutDetector::new(10).unwrap();
        assert!(!detector.is_lockout_response("Invalid username or password."));
        assert!(!detector.is_lockout_response("Welcome to the dashboard."));
    }

    #[test]
    fn test_classify_lockout_hard() {
        let detector = LockoutDetector::new(10).unwrap();
        assert_eq!(detector.classify_lockout(423, ""), LockoutType::HardLockout);
        assert_eq!(
            detector.classify_lockout(200, "account locked"),
            LockoutType::HardLockout
        );
    }

    #[test]
    fn test_classify_lockout_soft() {
        let detector = LockoutDetector::new(10).unwrap();
        assert_eq!(detector.classify_lockout(429, ""), LockoutType::SoftLockout);
    }

    #[test]
    fn test_classify_lockout_captcha() {
        let detector = LockoutDetector::new(10).unwrap();
        assert_eq!(
            detector.classify_lockout(200, "please solve captcha"),
            LockoutType::Captcha
        );
    }

    #[test]
    fn test_lockout_result_creation() {
        let result = LockoutDetectionResult {
            target: "http://example.com/login".to_string(),
            lockout_threshold: Some(5),
            lockout_duration_seconds: None,
            lockout_type: LockoutType::SoftLockout,
            attempts_before_lockout: 5,
            indicators: vec!["HTTP status: 429".to_string()],
        };
        assert_eq!(result.lockout_threshold, Some(5));
        assert_eq!(result.lockout_type, LockoutType::SoftLockout);
    }
}
