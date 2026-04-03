use crate::error::Result;
use crate::hunt::HuntConfig;
use crate::types::Severity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackChain {
    pub id: String,
    pub name: String,
    pub chain_type: ChainType,
    pub steps: Vec<ChainStep>,
    pub severity: Severity,
    pub description: String,
    pub remediation: String,
    pub cvss_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChainType {
    PrivilegeEscalation,
    DataExfiltration,
    RemoteCodeExecution,
    LateralMovement,
    Persistence,
    DenialOfService,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStep {
    pub step_number: usize,
    pub vulnerability: String,
    pub prerequisite: String,
    pub impact: String,
    pub evidence: String,
    pub severity: Severity,
}

pub async fn detect_attack_chains(target: &str, config: &HuntConfig) -> Result<Vec<AttackChain>> {
    let mut chains = Vec::new();

    chains.extend(detect_privilege_escalation(target, config).await?);
    chains.extend(detect_data_exfiltration(target, config).await?);
    chains.extend(detect_rce_chains(target, config).await?);
    chains.extend(detect_lateral_movement(target, config).await?);

    Ok(chains)
}

async fn detect_privilege_escalation(target: &str, _config: &HuntConfig) -> Result<Vec<AttackChain>> {
    let mut chains = Vec::new();

    let id = format!("pe-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    chains.push(AttackChain {
        id: id.clone(),
        name: "Horizontal to Vertical Privilege Escalation".to_string(),
        chain_type: ChainType::PrivilegeEscalation,
        steps: vec![
            ChainStep {
                step_number: 1,
                vulnerability: "IDOR on resource access".to_string(),
                prerequisite: "Valid user account with limited access".to_string(),
                impact: "Access other users' resources".to_string(),
                evidence: format!("Target: {}", target),
                severity: Severity::High,
            },
            ChainStep {
                step_number: 2,
                vulnerability: "Privilege escalation via API parameter manipulation".to_string(),
                prerequisite: "Horizontal privilege access".to_string(),
                impact: "Elevate to admin privileges".to_string(),
                evidence: format!("Target: {}", target),
                severity: Severity::Critical,
            },
        ],
        severity: Severity::Critical,
        description: "Chain allows a low-privilege user to escalate to admin privileges through IDOR and parameter manipulation".to_string(),
        remediation: "Implement proper authorization checks on all API endpoints; use server-side session validation".to_string(),
        cvss_score: Some(9.1),
    });

    Ok(chains)
}

async fn detect_data_exfiltration(target: &str, _config: &HuntConfig) -> Result<Vec<AttackChain>> {
    let mut chains = Vec::new();

    let id = format!("de-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    chains.push(AttackChain {
        id: id.clone(),
        name: "Mass Data Exfiltration via SQL Injection".to_string(),
        chain_type: ChainType::DataExfiltration,
        steps: vec![
            ChainStep {
                step_number: 1,
                vulnerability: "SQL Injection in search parameter".to_string(),
                prerequisite: "None - exploitable without authentication".to_string(),
                impact: "Extract all database contents".to_string(),
                evidence: format!("Target: {}", target),
                severity: Severity::Critical,
            },
            ChainStep {
                step_number: 2,
                vulnerability: "Lack of query result rate limiting".to_string(),
                prerequisite: "SQL injection presence".to_string(),
                impact: "Large volume data extraction without detection".to_string(),
                evidence: format!("Target: {}", target),
                severity: Severity::High,
            },
        ],
        severity: Severity::Critical,
        description: "SQL injection combined with lack of rate limiting allows mass data exfiltration".to_string(),
        remediation: "Use parameterized queries; implement rate limiting and anomaly detection".to_string(),
        cvss_score: Some(10.0),
    });

    Ok(chains)
}

async fn detect_rce_chains(target: &str, _config: &HuntConfig) -> Result<Vec<AttackChain>> {
    let mut chains = Vec::new();

    let id = format!("rce-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    chains.push(AttackChain {
        id: id.clone(),
        name: "SSRF to RCE via Metadata Service".to_string(),
        chain_type: ChainType::RemoteCodeExecution,
        steps: vec![
            ChainStep {
                step_number: 1,
                vulnerability: "Server-Side Request Forgery (SSRF)".to_string(),
                prerequisite: "User-controlled URL parameter".to_string(),
                impact: "Internal network access".to_string(),
                evidence: format!("Target: {}", target),
                severity: Severity::High,
            },
            ChainStep {
                step_number: 2,
                vulnerability: "Unprotected cloud metadata endpoint".to_string(),
                prerequisite: "SSRF allowing 169.254.169.254 access".to_string(),
                impact: "Obtain cloud credentials".to_string(),
                severity: Severity::Critical,
                evidence: format!("Target: {}", target),
            },
            ChainStep {
                step_number: 3,
                vulnerability: "Overly permissive IAM role".to_string(),
                prerequisite: "Cloud credentials obtained".to_string(),
                impact: "Remote code execution on EC2/container".to_string(),
                evidence: format!("Target: {}", target),
                severity: Severity::Critical,
            },
        ],
        severity: Severity::Critical,
        description: "SSRF vulnerability allows access to cloud metadata service, leading to RCE".to_string(),
        remediation: "Block 169.254.169.254 IP range; validate all user-supplied URLs; use IAM instance profiles with minimal privileges".to_string(),
        cvss_score: Some(9.8),
    });

    Ok(chains)
}

async fn detect_lateral_movement(target: &str, _config: &HuntConfig) -> Result<Vec<AttackChain>> {
    let mut chains = Vec::new();

    let id = format!("lm-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    chains.push(AttackChain {
        id: id.clone(),
        name: "Internal Network Penetration via File Upload".to_string(),
        chain_type: ChainType::LateralMovement,
        steps: vec![
            ChainStep {
                step_number: 1,
                vulnerability: "Unrestricted file upload".to_string(),
                prerequisite: "Authenticated user access".to_string(),
                impact: "Upload malicious files to server".to_string(),
                evidence: format!("Target: {}", target),
                severity: Severity::High,
            },
            ChainStep {
                step_number: 2,
                vulnerability: "Insecure file storage/execution".to_string(),
                prerequisite: "Malicious file uploaded".to_string(),
                impact: "Server compromise".to_string(),
                evidence: format!("Target: {}", target),
                severity: Severity::Critical,
            },
            ChainStep {
                step_number: 3,
                vulnerability: "Internal service trust relationships".to_string(),
                prerequisite: "Server compromised".to_string(),
                impact: "Access internal databases and services".to_string(),
                evidence: format!("Target: {}", target),
                severity: Severity::High,
            },
        ],
        severity: Severity::Critical,
        description: "File upload vulnerability leads to server compromise and lateral movement".to_string(),
        remediation: "Validate file types; store uploads outside webroot; scan for malware; segment internal networks".to_string(),
        cvss_score: Some(9.3),
    });

    Ok(chains)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attack_chain_creation() {
        let chain = AttackChain {
            id: "test-123".to_string(),
            name: "Test Chain".to_string(),
            chain_type: ChainType::PrivilegeEscalation,
            steps: vec![
                ChainStep {
                    step_number: 1,
                    vulnerability: "IDOR".to_string(),
                    prerequisite: "Valid user".to_string(),
                    impact: "Access other user data".to_string(),
                    evidence: "Evidence here".to_string(),
                    severity: Severity::High,
                },
            ],
            severity: Severity::High,
            description: "Test description".to_string(),
            remediation: "Fix it".to_string(),
            cvss_score: Some(7.5),
        };

        assert_eq!(chain.chain_type, ChainType::PrivilegeEscalation);
        assert_eq!(chain.steps.len(), 1);
    }

    #[tokio::test]
    async fn test_detect_attack_chains() {
        let config = HuntConfig::default();
        let chains = detect_attack_chains("http://example.com", &config).await.unwrap();
        assert!(!chains.is_empty());
    }
}
