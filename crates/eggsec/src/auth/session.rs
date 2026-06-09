use crate::error::Result;
use crate::utils::create_insecure_http_client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTestResult {
    pub target: String,
    pub session_fixation_possible: bool,
    pub session_cookie_issues: Vec<String>,
    pub token_predictable: bool,
    pub findings: Vec<String>,
}

pub struct SessionTester {
    client: reqwest::Client,
}

impl SessionTester {
    pub fn new(timeout_secs: u64) -> Result<Self> {
        let client = create_insecure_http_client(timeout_secs)?;
        Ok(Self { client })
    }

    pub async fn test(&self, target: &str) -> Result<SessionTestResult> {
        let mut result = SessionTestResult {
            target: target.to_string(),
            session_fixation_possible: false,
            session_cookie_issues: Vec::new(),
            token_predictable: false,
            findings: Vec::new(),
        };

        let response1 = self.client.get(target).send().await;
        let response2 = self.client.get(target).send().await;

        if let (Ok(r1), Ok(r2)) = (response1, response2) {
            let set_cookie1 = r1.headers().get(reqwest::header::SET_COOKIE);
            let set_cookie2 = r2.headers().get(reqwest::header::SET_COOKIE);

            if let (Some(c1), Some(c2)) = (set_cookie1, set_cookie2) {
                if c1 == c2 {
                    result.session_fixation_possible = true;
                    result
                        .findings
                        .push("Session tokens reused across requests".to_string());
                }
            }

            if let Some(cookie_header) = set_cookie1 {
                if let Ok(cookie_str) = cookie_header.to_str() {
                    let mut issues = Vec::new();
                    if !cookie_str.contains("HttpOnly") {
                        issues.push("HttpOnly not set".to_string());
                    }
                    if !cookie_str.contains("Secure") {
                        issues.push("Secure flag not set".to_string());
                    }
                    if !cookie_str.contains("SameSite") {
                        issues.push("SameSite not set".to_string());
                    }
                    if !issues.is_empty() {
                        result
                            .session_cookie_issues
                            .push(format!("Session cookie: {}", issues.join(", ")));
                    }
                }
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_tester_creation() {
        let tester = SessionTester::new(10);
        assert!(tester.is_ok());
    }

    #[test]
    fn test_session_result_default() {
        let result = SessionTestResult {
            target: "http://example.com".to_string(),
            session_fixation_possible: false,
            session_cookie_issues: Vec::new(),
            token_predictable: false,
            findings: Vec::new(),
        };
        assert!(!result.session_fixation_possible);
    }
}
