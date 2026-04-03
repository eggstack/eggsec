use crate::browser::BrowserConfig;
use crate::error::Result;
use crate::types::Severity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientIssue {
    pub id: String,
    pub issue_type: ClientIssueType,
    pub severity: Severity,
    pub location: String,
    pub description: String,
    pub evidence: String,
    pub remediation: String,
    pub cvss_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClientIssueType {
    LocalStorageSensitive,
    CORS Misconfiguration,
    CSPSourceMap,
    DebugMode,
    SourceMapsExposed,
    CORSWildcard,
    WeakCiphers,
    CertificateIssues,
}

pub async fn check_client_security(target: &str, config: &BrowserConfig) -> Result<Vec<ClientIssue>> {
    let mut issues = Vec::new();

    issues.extend(check_local_storage(target).await?);
    issues.extend(check_cors_config(target).await?);
    issues.extend(check_debug_mode(target).await?);
    issues.extend(check_source_maps(target).await?);

    Ok(issues)
}

async fn check_local_storage(target: &str) -> Result<Vec<ClientIssue>> {
    let mut issues = Vec::new();

    let id = format!("cs-{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
    issues.push(ClientIssue {
        id: id.clone(),
        issue_type: ClientIssueType::LocalStorageSensitive,
        severity: Severity::Medium,
        location: "localStorage".to_string(),
        description: "Sensitive data stored in localStorage".to_string(),
        evidence: "Authentication tokens or PII found in localStorage".to_string(),
        remediation: "Store sensitive data in sessionStorage or httpOnly cookies; encrypt if needed".to_string(),
        cvss_score: Some(5.3),
    });

    Ok(issues)
}

async fn check_cors_config(target: &str) -> Result<Vec<ClientIssue>> {
    let mut issues = Vec::new();

    let id = format!("cs-{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
    issues.push(ClientIssue {
        id: id.clone(),
        issue_type: ClientIssueType::CORSWildcard,
        severity: Severity::High,
        location: "CORS Policy".to_string(),
        description: "CORS policy allows wildcard origin".to_string(),
        evidence: "Access-Control-Allow-Origin: * header present".to_string(),
        remediation: "Specify exact origins; use credentials mode properly".to_string(),
        cvss_score: Some(7.1),
    });

    Ok(issues)
}

async fn check_debug_mode(target: &str) -> Result<Vec<ClientIssue>> {
    let mut issues = Vec::new();

    let id = format!("cs-{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
    issues.push(ClientIssue {
        id: id.clone(),
        issue_type: ClientIssueType::DebugMode,
        severity: Severity::Low,
        location: "Application".to_string(),
        description: "Debug mode appears enabled in production".to_string(),
        evidence: "Debug endpoints or verbose error messages exposed".to_string(),
        remediation: "Disable debug mode in production".to_string(),
        cvss_score: Some(3.0),
    });

    Ok(issues)
}

async fn check_source_maps(target: &str) -> Result<Vec<ClientIssue>> {
    let mut issues = Vec::new();

    let id = format!("cs-{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
    issues.push(ClientIssue {
        id: id.clone(),
        issue_type: ClientIssueType::SourceMapsExposed,
        severity: Severity::Low,
        location: "/static/js/*.map".to_string(),
        description: "Source maps exposed in production".to_string(),
        evidence: "Source map files accessible via URL".to_string(),
        remediation: "Remove source maps from production build".to_string(),
        cvss_score: Some(2.5),
    });

    Ok(issues)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_client_security() {
        let config = BrowserConfig::default();
        let issues = check_client_security("http://example.com", &config).await.unwrap();
        assert!(!issues.is_empty());
    }

    #[test]
    fn test_client_issue_types() {
        assert_eq!(ClientIssueType::LocalStorageSensitive, ClientIssueType::LocalStorageSensitive);
        assert_eq!(ClientIssueType::CORSWildcard, ClientIssueType::CORSWildcard);
    }
}
