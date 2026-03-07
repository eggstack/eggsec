#![allow(dead_code)]
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
            vec!["2".to_string(), "3".to_string(), "4".to_string(), "5".to_string()]
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
                        severity: if is_vulnerable { Severity::High } else { Severity::Low },
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
                        severity: if is_vulnerable { Severity::Critical } else { Severity::Low },
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

    payloads
}
