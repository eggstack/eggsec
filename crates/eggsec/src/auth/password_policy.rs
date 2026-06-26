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

        let response = self.client.get(target).send().await;

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
            if lower.contains("special")
                || lower.contains("symbol")
                || lower.contains("[@!#$%^&*()]")
            {
                result.requires_special = true;
            }

            let re = regex::Regex::new(r"(?:minimum|at least|must be|require)\s+(\d+)\s+characters?")
                    .expect("valid regex pattern");
            if let Some(caps) = re.captures(&lower) {
                if let Some(m) = caps.get(1) {
                    if let Ok(len) = m.as_str().parse::<usize>() {
                        result.min_length = Some(len);
                    }
                }
            }
        }

        let weak_passwords = vec!["password", "123456", "password123", "admin", "letmein"];

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
                    if !body.contains("invalid")
                        && !body.contains("error")
                        && !body.contains("weak")
                    {
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
        assert!(result.min_length.is_none());
        assert!(!result.accepts_weak_passwords);
        assert!(result.weak_passwords_tested.is_empty());
    }

    #[test]
    fn test_password_policy_result_with_policy() {
        let result = PasswordPolicyResult {
            target: "http://example.com".to_string(),
            policy_detected: true,
            min_length: Some(8),
            requires_uppercase: true,
            requires_lowercase: true,
            requires_digit: true,
            requires_special: true,
            accepts_weak_passwords: false,
            weak_passwords_tested: Vec::new(),
        };
        assert!(result.policy_detected);
        assert_eq!(result.min_length, Some(8));
        assert!(result.requires_uppercase);
        assert!(result.requires_lowercase);
        assert!(result.requires_digit);
        assert!(result.requires_special);
        assert!(!result.accepts_weak_passwords);
    }

    #[test]
    fn test_password_policy_result_accepts_weak() {
        let result = PasswordPolicyResult {
            target: "http://example.com".to_string(),
            policy_detected: true,
            min_length: None,
            requires_uppercase: false,
            requires_lowercase: false,
            requires_digit: false,
            requires_special: false,
            accepts_weak_passwords: true,
            weak_passwords_tested: vec![
                "password".to_string(),
                "123456".to_string(),
                "admin".to_string(),
            ],
        };
        assert!(result.accepts_weak_passwords);
        assert_eq!(result.weak_passwords_tested.len(), 3);
        assert!(result
            .weak_passwords_tested
            .contains(&"password".to_string()));
        assert!(result.weak_passwords_tested.contains(&"123456".to_string()));
        assert!(result.weak_passwords_tested.contains(&"admin".to_string()));
    }

    #[test]
    fn test_password_policy_result_serialization() {
        let result = PasswordPolicyResult {
            target: "http://example.com".to_string(),
            policy_detected: true,
            min_length: Some(12),
            requires_uppercase: true,
            requires_lowercase: false,
            requires_digit: true,
            requires_special: false,
            accepts_weak_passwords: true,
            weak_passwords_tested: vec!["test".to_string()],
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: PasswordPolicyResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.target, result.target);
        assert_eq!(deserialized.policy_detected, result.policy_detected);
        assert_eq!(deserialized.min_length, result.min_length);
        assert_eq!(deserialized.requires_uppercase, result.requires_uppercase);
        assert_eq!(deserialized.requires_lowercase, result.requires_lowercase);
        assert_eq!(deserialized.requires_digit, result.requires_digit);
        assert_eq!(deserialized.requires_special, result.requires_special);
        assert_eq!(
            deserialized.accepts_weak_passwords,
            result.accepts_weak_passwords
        );
        assert_eq!(
            deserialized.weak_passwords_tested,
            result.weak_passwords_tested
        );
    }

    #[test]
    fn test_password_policy_tester_creation() {
        let tester = PasswordPolicyTester::new(10);
        assert!(tester.is_ok());
    }

    #[test]
    fn test_password_policy_tester_creation_various_timeouts() {
        assert!(PasswordPolicyTester::new(1).is_ok());
        assert!(PasswordPolicyTester::new(30).is_ok());
        assert!(PasswordPolicyTester::new(300).is_ok());
    }
}
