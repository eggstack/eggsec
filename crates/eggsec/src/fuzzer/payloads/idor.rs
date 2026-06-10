#![allow(clippy::vec_init_then_push)]

use crate::fuzzer::payloads::{Payload, PayloadType, Severity};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdorVulnerability {
    DirectReference,
    SequentialAccess,
    HorizontalPrivilegeEscalation,
    VerticalPrivilegeEscalation,
    ParameterTampering,
    HttpVerbTampering,
    MissingFunctionLevelAccessControl,
}

impl std::fmt::Display for IdorVulnerability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IdorVulnerability::DirectReference => write!(f, "Insecure Direct Object Reference"),
            IdorVulnerability::SequentialAccess => write!(f, "Sequential Object Enumeration"),
            IdorVulnerability::HorizontalPrivilegeEscalation => {
                write!(f, "Horizontal Privilege Escalation")
            }
            IdorVulnerability::VerticalPrivilegeEscalation => {
                write!(f, "Vertical Privilege Escalation")
            }
            IdorVulnerability::ParameterTampering => write!(f, "Parameter Tampering"),
            IdorVulnerability::HttpVerbTampering => write!(f, "HTTP Verb Tampering"),
            IdorVulnerability::MissingFunctionLevelAccessControl => {
                write!(f, "Missing Function Level Access Control")
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdorTestResult {
    pub vulnerability: IdorVulnerability,
    pub success: bool,
    pub endpoint: String,
    pub method: String,
    pub parameter: String,
    pub test_value: String,
    pub response_status: u16,
    pub description: String,
    pub severity: Severity,
}

pub struct IdorFuzzer {
    pub base_url: String,
    pub authenticated_cookies: Option<String>,
    pub user_ids: Vec<String>,
    pub resource_ids: Vec<String>,
    pub client: Option<Client>,
    pub base_user_id: Option<String>,
}

impl IdorFuzzer {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            authenticated_cookies: None,
            user_ids: vec![],
            resource_ids: vec![],
            client: None,
            base_user_id: None,
        }
    }

    pub fn with_client(mut self, client: Client) -> Self {
        self.client = Some(client);
        self
    }

    pub fn with_user_ids(mut self, ids: Vec<String>) -> Self {
        self.user_ids = ids;
        self
    }

    pub fn with_base_user_id(mut self, id: String) -> Self {
        self.base_user_id = Some(id);
        self
    }

    pub fn with_authentication(mut self, cookies: String) -> Self {
        self.authenticated_cookies = Some(cookies);
        self
    }

    pub async fn test_horizontal_escalation(&mut self) -> Vec<IdorTestResult> {
        let mut results = Vec::new();

        let client = match &self.client {
            Some(c) => c,
            None => return results,
        };

        let base_user_id = self.base_user_id.clone().unwrap_or_else(|| "1".to_string());

        let test_ids = if self.user_ids.is_empty() {
            vec![
                "2".to_string(),
                "3".to_string(),
                "4".to_string(),
                "5".to_string(),
            ]
        } else {
            self.user_ids.clone()
        };

        for user_id in test_ids {
            if user_id == base_user_id {
                continue;
            }

            let endpoint = self.base_url.replace(&base_user_id, &user_id);

            let mut request = client.get(&endpoint);
            if let Some(ref cookies) = self.authenticated_cookies {
                request = request.header("Cookie", cookies);
            }

            let response = request.send().await;

            match response {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    let is_vulnerable = status == 200 || status == 201;

                    results.push(IdorTestResult {
                        vulnerability: IdorVulnerability::HorizontalPrivilegeEscalation,
                        success: is_vulnerable,
                        endpoint: endpoint.clone(),
                        method: "GET".to_string(),
                        parameter: "id".to_string(),
                        test_value: user_id.clone(),
                        response_status: status,
                        description: if is_vulnerable {
                            format!("Could access user {}'s data!", user_id)
                        } else {
                            format!("Access denied to user {} (status: {})", user_id, status)
                        },
                        severity: if is_vulnerable {
                            Severity::High
                        } else {
                            Severity::Low
                        },
                    });
                }
                Err(_) => continue,
            }
        }

        results
    }

    pub async fn test_vertical_escalation(&mut self) -> Vec<IdorTestResult> {
        let mut results = Vec::new();

        let client = match &self.client {
            Some(c) => c,
            None => return results,
        };

        let escalation_values = vec![
            ("0", "Zero ID"),
            ("-1", "Negative ID"),
            ("admin", "Admin username"),
            ("administrator", "Administrator"),
            ("root", "Root"),
        ];

        for (value, desc) in escalation_values {
            let endpoint = format!("{}?id={}", self.base_url, value);

            let mut request = client.get(&endpoint);
            if let Some(ref cookies) = self.authenticated_cookies {
                request = request.header("Cookie", cookies);
            }

            let response = request.send().await;

            match response {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    let is_vulnerable = status == 200 || status == 201;

                    results.push(IdorTestResult {
                        vulnerability: IdorVulnerability::VerticalPrivilegeEscalation,
                        success: is_vulnerable,
                        endpoint: endpoint.clone(),
                        method: "GET".to_string(),
                        parameter: "id".to_string(),
                        test_value: value.to_string(),
                        response_status: status,
                        description: if is_vulnerable {
                            format!("Could access {} account!", desc)
                        } else {
                            format!("Access denied for {} (status: {})", desc, status)
                        },
                        severity: if is_vulnerable {
                            Severity::Critical
                        } else {
                            Severity::Low
                        },
                    });
                }
                Err(_) => continue,
            }
        }

        results
    }
}

pub fn get_payloads() -> Vec<Payload> {
    let mut payloads = Vec::new();

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=1".to_string(),
        description: "IDOR test - sequential ID parameter".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "authorization".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "user_id=1".to_string(),
        description: "IDOR test - alternate parameter name".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "parameter".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=admin".to_string(),
        description: "IDOR test - admin as ID".to_string(),
        severity: Severity::Critical,
        tags: vec!["idor".to_string(), "privilege_escalation".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=2".to_string(),
        description: "IDOR test - sequential increment ID".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "sequential".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=99999".to_string(),
        description: "IDOR test - sequential large ID".to_string(),
        severity: Severity::Medium,
        tags: vec!["idor".to_string(), "sequential".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=550e8400-e29b-41d4-a716-446655440000".to_string(),
        description: "IDOR test - UUID format ID".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "uuid".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "/api/users/2".to_string(),
        description: "IDOR test - path-based IDOR traversal".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "path_traversal".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=MQ==".to_string(),
        description: "IDOR test - base64 encoded ID".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "encoding".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: r#"{"id": 2}"#.to_string(),
        description: "IDOR test - JSON body parameter".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "json".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=-1".to_string(),
        description: "IDOR test - negative ID".to_string(),
        severity: Severity::Critical,
        tags: vec!["idor".to_string(), "negative".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=0".to_string(),
        description: "IDOR test - zero ID".to_string(),
        severity: Severity::Critical,
        tags: vec!["idor".to_string(), "zero".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id[]=1&id[]=2".to_string(),
        description: "IDOR test - array parameter injection".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "array".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "user[id]=2".to_string(),
        description: "IDOR test - nested object parameter".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "nested".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=encrypted_value_here".to_string(),
        description: "IDOR test - encrypted ID value".to_string(),
        severity: Severity::Medium,
        tags: vec!["idor".to_string(), "encrypted".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=00000000-0000-0000-0000-000000000000".to_string(),
        description: "IDOR test - null UUID".to_string(),
        severity: Severity::Critical,
        tags: vec!["idor".to_string(), "uuid".to_string(), "null".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=0x1".to_string(),
        description: "IDOR test - hex formatted ID".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "hex".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "uid=1".to_string(),
        description: "IDOR test - alternate uid parameter".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "parameter".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "account_id=1".to_string(),
        description: "IDOR test - alternate account_id parameter".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "parameter".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "profile_id=1".to_string(),
        description: "IDOR test - alternate profile_id parameter".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "parameter".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=1.0".to_string(),
        description: "IDOR test - decimal formatted ID".to_string(),
        severity: Severity::High,
        tags: vec!["idor".to_string(), "decimal".to_string()],
    });

    payloads.push(Payload {
        payload_type: PayloadType::Idor,
        payload: "id=test".to_string(),
        description: "IDOR test - string ID value".to_string(),
        severity: Severity::Medium,
        tags: vec!["idor".to_string(), "string".to_string()],
    });

    payloads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_payloads_returns_non_empty() {
        let payloads = get_payloads();
        assert!(!payloads.is_empty());
    }

    #[test]
    fn test_get_payloads_count_reasonable() {
        let payloads = get_payloads();
        assert!(payloads.len() >= 10);
        assert!(payloads.len() < 10000);
    }

    #[test]
    fn test_payloads_are_non_empty_strings() {
        let payloads = get_payloads();
        for p in &payloads {
            assert!(
                !p.payload.is_empty(),
                "Payload is empty: {:?}",
                p.description
            );
        }
    }

    #[test]
    fn test_payloads_contain_expected_patterns() {
        let payloads = get_payloads();
        let has_id_param = payloads.iter().any(|p| p.payload.contains("id="));
        let has_user_id = payloads.iter().any(|p| p.payload.contains("user_id="));
        let has_admin = payloads.iter().any(|p| p.payload.contains("admin"));
        let has_uuid = payloads.iter().any(|p| p.payload.contains("-e29b-"));
        let has_base64 = payloads.iter().any(|p| p.payload.contains("MQ=="));
        let has_json = payloads.iter().any(|p| p.payload.starts_with('{'));
        let has_negative = payloads.iter().any(|p| p.payload.contains("id=-1"));
        let has_array = payloads.iter().any(|p| p.payload.contains("[]"));
        let has_nested = payloads.iter().any(|p| p.payload.contains("[id]="));
        let has_hex = payloads.iter().any(|p| p.payload.contains("0x1"));
        let has_uid = payloads.iter().any(|p| p.payload.contains("uid="));
        let has_account_id = payloads.iter().any(|p| p.payload.contains("account_id="));
        let has_profile_id = payloads.iter().any(|p| p.payload.contains("profile_id="));
        let has_decimal = payloads.iter().any(|p| p.payload.contains("1.0"));
        let has_string = payloads.iter().any(|p| p.payload.contains("id=test"));
        let has_path = payloads.iter().any(|p| p.payload.contains("/api/users/"));
        assert!(has_id_param, "Missing id= parameter payload");
        assert!(has_user_id, "Missing user_id= parameter payload");
        assert!(has_admin, "Missing admin ID payload");
        assert!(has_uuid, "Missing UUID format payload");
        assert!(has_base64, "Missing base64 encoded payload");
        assert!(has_json, "Missing JSON body payload");
        assert!(has_negative, "Missing negative ID payload");
        assert!(has_array, "Missing array parameter payload");
        assert!(has_nested, "Missing nested object payload");
        assert!(has_hex, "Missing hex formatted payload");
        assert!(has_uid, "Missing uid alternate parameter payload");
        assert!(has_account_id, "Missing account_id parameter payload");
        assert!(has_profile_id, "Missing profile_id parameter payload");
        assert!(has_decimal, "Missing decimal formatted payload");
        assert!(has_string, "Missing string ID payload");
        assert!(has_path, "Missing path-based IDOR payload");
    }
}
