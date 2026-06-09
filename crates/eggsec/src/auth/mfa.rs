use crate::error::Result;
use crate::types::Severity;
use crate::utils::create_insecure_http_client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaTestResult {
    pub target: String,
    pub mfa_enabled: bool,
    pub mfa_bypass_possible: bool,
    pub bypass_methods: Vec<MfaBypassMethod>,
    pub findings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaBypassMethod {
    pub method: String,
    pub description: String,
    pub severity: Severity,
}

pub struct MfaTester {
    client: reqwest::Client,
}

impl MfaTester {
    pub fn new(timeout_secs: u64) -> Result<Self> {
        let client = create_insecure_http_client(timeout_secs)?;
        Ok(Self { client })
    }

    pub async fn test(&self, target: &str) -> Result<MfaTestResult> {
        let mut result = MfaTestResult {
            target: target.to_string(),
            mfa_enabled: false,
            mfa_bypass_possible: false,
            bypass_methods: Vec::new(),
            findings: Vec::new(),
        };

        let mfa_indicators = self.check_mfa_enabled(target).await;
        result.mfa_enabled = !mfa_indicators.is_empty();
        result.findings = mfa_indicators;

        if result.mfa_enabled {
            result.bypass_methods = self.test_bypass_methods(target).await;
            result.mfa_bypass_possible = !result.bypass_methods.is_empty();
        }

        Ok(result)
    }

    async fn check_mfa_enabled(&self, target: &str) -> Vec<String> {
        let mut findings = Vec::new();

        let response = self
            .client
            .post(target)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body("username=admin&password=admin")
            .send()
            .await;

        if let Ok(resp) = response {
            let body = resp.text().await.unwrap_or_default();
            let lower = body.to_lowercase();

            if lower.contains("two-factor")
                || lower.contains("mfa")
                || lower.contains("verification code")
                || lower.contains("authenticator")
                || lower.contains("totp")
                || lower.contains("2fa")
            {
                findings.push("MFA/2FA detected in login flow".to_string());
            }
            if lower.contains("enter code") || lower.contains("enter token") {
                findings.push("Code/token input found".to_string());
            }
        }

        findings
    }

    async fn test_bypass_methods(&self, target: &str) -> Vec<MfaBypassMethod> {
        let mut methods = Vec::new();

        let response = self
            .client
            .post(target)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body("username=admin&password=admin&mfa_code=000000")
            .send()
            .await;

        if let Ok(resp) = response {
            let status = resp.status().as_u16();
            if status == 302 || status == 200 {
                methods.push(MfaBypassMethod {
                    method: "Weak MFA Code".to_string(),
                    description: "MFA accepted weak code '000000'".to_string(),
                    severity: Severity::Critical,
                });
            }
        }

        let response = self
            .client
            .post(target)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body("username=admin&password=admin&mfa_skip=true")
            .send()
            .await;

        if let Ok(resp) = response {
            let status = resp.status().as_u16();
            if status == 302 || status == 200 {
                methods.push(MfaBypassMethod {
                    method: "MFA Skip Parameter".to_string(),
                    description: "MFA can be bypassed with skip parameter".to_string(),
                    severity: Severity::Critical,
                });
            }
        }

        methods
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mfa_tester_creation() {
        let tester = MfaTester::new(10);
        assert!(tester.is_ok());
    }

    #[test]
    fn test_mfa_result_default() {
        let result = MfaTestResult {
            target: "http://example.com/login".to_string(),
            mfa_enabled: false,
            mfa_bypass_possible: false,
            bypass_methods: Vec::new(),
            findings: Vec::new(),
        };
        assert!(!result.mfa_enabled);
    }

    #[test]
    fn test_mfa_bypass_method_creation() {
        let method = MfaBypassMethod {
            method: "Test".to_string(),
            description: "Test bypass".to_string(),
            severity: Severity::Critical,
        };
        assert_eq!(method.severity, Severity::Critical);
    }
}
