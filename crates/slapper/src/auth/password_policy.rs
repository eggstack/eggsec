use crate::error::Result;
use crate::utils::create_insecure_http_client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordPolicyResult {
    pub target: String,
    pub policy_detected: bool,
    pub min_length: Option<usize>,
    pub requires_uppercase: bool,
    pub requires_lowercase: bool,
    pub requires_digit: bool,
    pub requires_special: bool,
    pub accepts_weak_passwords: bool,
    pub weak_passwords_tested: Vec<String>,
}

pub struct PasswordPolicyTester {
    client: reqwest::Client,
}

impl PasswordPolicyTester {
    pub fn new(timeout_secs: u64) -> Result<Self> {
        let client = create_insecure_http_client(timeout_secs)?;
        Ok(Self { client })
    }

    pub async fn test(&self, target: &str) -> Result<PasswordPolicyResult> {
        let mut result = PasswordPolicyResult {
            target: target.to_string(),
            policy_detected: false,
            min_length: None,
            requires_uppercase: false,
            requires_lowercase: false,
            requires_digit: false,
            requires_special: false,
            accepts_weak_passwords: false,
            weak_passwords_tested: Vec::new(),
        };

        let response = self
            .client
            .get(target)
            .send()
            .await;

        if let Ok(resp) = response {
            let body = resp.text().await.unwrap_or_default();
            let lower = body.to_lowercase();

            if lower.contains("password") && lower.contains("policy") {
                result.policy_detected = true;
            }

            if lower.contains("uppercase") || lower.contains("upper case") {
                result.requires_uppercase = true;
            }
            if lower.contains("lowercase") || lower.contains("lower case") {
                result.requires_lowercase = true;
            }
            if lower.contains("digit") || lower.contains("number") || lower.contains("0-9") {
                result.requires_digit = true;
            }
            if lower.contains("special") || lower.contains("symbol") || lower.contains("[@!#$%^&*()]") {
                result.requires_special = true;
            }

            if let Some(caps) = lower.match_indices("character").next() {
                let after_caps = &lower[caps.0..];
                for (i, c) in after_caps.chars().take(20).enumerate() {
                    if c.is_ascii_digit() {
                        if let Ok(len) = after_caps[..i].chars().filter(|&ch| ch == ' ').count().to_string().parse::<usize>() {
                            result.min_length = Some(len);
                            break;
                        }
                    }
                }
            }
        }

        let weak_passwords = vec![
            "password",
            "123456",
            "password123",
            "admin",
            "letmein",
        ];

        for pwd in &weak_passwords {
            let test_response = self
                .client
                .post(target)
                .header(
                    reqwest::header::CONTENT_TYPE,
                    "application/x-www-form-urlencoded",
                )
                .body(format!("username=test&password={}", pwd))
                .send()
                .await;

            if let Ok(resp) = test_response {
                let status = resp.status().as_u16();
                if status == 302 || status == 200 {
                    let body = resp.text().await.unwrap_or_default();
                    if !body.contains("invalid") && !body.contains("error") && !body.contains("weak") {
                        result.accepts_weak_passwords = true;
                        result.weak_passwords_tested.push(pwd.to_string());
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
    fn test_password_policy_result_default() {
        let result = PasswordPolicyResult {
            target: "http://example.com".to_string(),
            policy_detected: false,
            min_length: None,
            requires_uppercase: false,
            requires_lowercase: false,
            requires_digit: false,
            requires_special: false,
            accepts_weak_passwords: false,
            weak_passwords_tested: Vec::new(),
        };
        assert!(!result.policy_detected);
    }
}