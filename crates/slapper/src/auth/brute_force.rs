use crate::auth::AuthEngine;
use crate::error::Result;
use crate::utils::create_insecure_http_client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BruteForceResult {
    pub target: String,
    pub attempts_made: usize,
    pub successful_logins: usize,
    pub weak_credentials: Vec<WeakCredential>,
    pub rate_limited: bool,
    pub lockout_detected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeakCredential {
    pub username: String,
    pub password: String,
    pub response_status: u16,
    pub response_indicators: Vec<String>,
}

pub struct BruteForceTester {
    engine: AuthEngine,
    client: reqwest::Client,
}

impl BruteForceTester {
    pub fn new(max_attempts: usize, concurrency: usize, timeout_secs: u64) -> Result<Self> {
        let engine = AuthEngine::new(max_attempts, concurrency, timeout_secs, true)?;
        let client = create_insecure_http_client(timeout_secs)?;
        Ok(Self { engine, client })
    }

    pub async fn test(
        &self,
        target: &str,
        username: &str,
        passwords: &[String],
    ) -> Result<BruteForceResult> {
        let mut result = BruteForceResult {
            target: target.to_string(),
            attempts_made: 0,
            successful_logins: 0,
            weak_credentials: Vec::new(),
            rate_limited: false,
            lockout_detected: false,
        };

        for password in passwords {
            if !self.engine.increment_attempts() || self.engine.should_stop() {
                break;
            }

            let response = self
                .client
                .post(target)
                .header(
                    reqwest::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(format!("username={}&password={}", username, password))
                .send()
                .await;

            result.attempts_made += 1;

            match response {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    let body = resp.text().await.unwrap_or_default();

                    if (status == 302 || status == 200)
                        && !body.contains("invalid")
                        && !body.contains("error")
                        && !body.contains("failed")
                    {
                        result.successful_logins += 1;
                        result.weak_credentials.push(WeakCredential {
                            username: username.to_string(),
                            password: password.clone(),
                            response_status: status,
                            response_indicators: self.analyze_response(&body),
                        });
                    }

                    if status == 429 {
                        result.rate_limited = true;
                    }
                    if status == 423
                        || body.contains("locked")
                        || body.contains("too many attempts")
                    {
                        result.lockout_detected = true;
                        if self.engine.stop_on_lockout {
                            self.engine.stop();
                        }
                    }
                }
                Err(_) => {
                    result.lockout_detected = true;
                }
            }
        }

        Ok(result)
    }

    fn analyze_response(&self, body: &str) -> Vec<String> {
        let mut indicators = Vec::new();
        let lower = body.to_lowercase();

        if lower.contains("welcome") || lower.contains("dashboard") {
            indicators.push("welcome message found".to_string());
        }
        if lower.contains("set-cookie") || lower.contains("session") {
            indicators.push("session indicator found".to_string());
        }
        if lower.contains("token") || lower.contains("jwt") {
            indicators.push("token found in response".to_string());
        }

        indicators
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brute_force_result_default() {
        let result = BruteForceResult {
            target: "http://example.com/login".to_string(),
            attempts_made: 0,
            successful_logins: 0,
            weak_credentials: Vec::new(),
            rate_limited: false,
            lockout_detected: false,
        };
        assert_eq!(result.attempts_made, 0);
        assert!(!result.rate_limited);
    }

    #[test]
    fn test_weak_credential_creation() {
        let cred = WeakCredential {
            username: "admin".to_string(),
            password: "password123".to_string(),
            response_status: 200,
            response_indicators: vec!["welcome message found".to_string()],
        };
        assert_eq!(cred.username, "admin");
        assert_eq!(cred.password, "password123");
    }

    #[test]
    fn test_analyze_response_indicators() {
        let tester = BruteForceTester::new(100, 10, 10).unwrap();
        let body = "Welcome to the dashboard! Your session token: abc123";
        let indicators = tester.analyze_response(body);
        assert!(!indicators.is_empty());
        assert!(indicators.iter().any(|i| i.contains("welcome")));
    }

    #[test]
    fn test_analyze_response_no_indicators() {
        let tester = BruteForceTester::new(100, 10, 10).unwrap();
        let body = "Invalid credentials. Please try again.";
        let indicators = tester.analyze_response(body);
        assert!(indicators.is_empty());
    }
}
