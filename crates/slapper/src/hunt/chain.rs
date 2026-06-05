use crate::error::Result;
use crate::hunt::{business, authz, session, HuntReport};
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
#[allow(dead_code)]
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

#[tracing::instrument(skip(report), fields(target = %report.target))]
pub async fn detect_attack_chains(report: &HuntReport) -> Result<Vec<AttackChain>> {
    tracing::info!("Detecting attack chains");
    let mut chains = Vec::new();

    chains.extend(detect_privilege_escalation_chain(report));
    chains.extend(detect_data_exfiltration_chain(report));
    chains.extend(detect_session_exploitation_chain(report));
    chains.extend(detect_rate_limit_chain(report));

    Ok(chains)
}

fn detect_privilege_escalation_chain(report: &HuntReport) -> Vec<AttackChain> {
    let mut chains = Vec::new();

    let has_idor = report.authz_bypasses.iter().any(|b| {
        matches!(
            b.bypass_type,
            authz::BypassType::Idor | authz::BypassType::ForceBrowsing
        )
    });

    let has_missing_authz = report
        .authz_bypasses
        .iter()
        .any(|b| matches!(b.bypass_type, authz::BypassType::MissingAuthorization));

    let has_session_issue = report.session_issues.iter().any(|i| {
        matches!(
            i.issue_type,
            session::SessionIssueType::SessionFixation
                | session::SessionIssueType::MissingHttpOnly
                | session::SessionIssueType::InsufficientEntropy
        )
    });

    if has_idor && has_missing_authz {
        let id = format!("pe-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let steps = report
            .authz_bypasses
            .iter()
            .filter(|b| {
                matches!(
                    b.bypass_type,
                    authz::BypassType::Idor | authz::BypassType::MissingAuthorization
                )
            })
            .enumerate()
            .map(|(i, b)| ChainStep {
                step_number: i + 1,
                vulnerability: format!("{:?}", b.bypass_type),
                prerequisite: "Low-privilege access".to_string(),
                impact: b.description.clone(),
                evidence: b.evidence.clone(),
                severity: b.severity,
            })
            .collect();

        chains.push(AttackChain {
            id,
            name: "Privilege Escalation Chain".to_string(),
            chain_type: ChainType::PrivilegeEscalation,
            steps,
            severity: Severity::Critical,
            description:
                "Multiple authorization bypasses can be chained for privilege escalation"
                    .to_string(),
            remediation: "Implement defense-in-depth authorization checks at every layer"
                .to_string(),
            cvss_score: Some(9.0),
        });
    } else if has_missing_authz && has_session_issue {
        let id = format!("pe-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let steps = vec![
            ChainStep {
                step_number: 1,
                vulnerability: "Session fixation or weak token".to_string(),
                prerequisite: "None".to_string(),
                impact: "Obtain valid session".to_string(),
                evidence: "Session issue detected".to_string(),
                severity: Severity::High,
            },
            ChainStep {
                step_number: 2,
                vulnerability: "Missing authorization on admin endpoint".to_string(),
                prerequisite: "Valid session".to_string(),
                impact: "Access admin functionality".to_string(),
                evidence: "Admin endpoint accessible".to_string(),
                severity: Severity::Critical,
            },
        ];

        chains.push(AttackChain {
            id,
            name: "Session to Admin Chain".to_string(),
            chain_type: ChainType::PrivilegeEscalation,
            steps,
            severity: Severity::Critical,
            description: "Session weakness combined with missing authz allows admin access"
                .to_string(),
            remediation: "Fix session security and implement authorization checks".to_string(),
            cvss_score: Some(8.5),
        });
    }

    chains
}

fn detect_data_exfiltration_chain(report: &HuntReport) -> Vec<AttackChain> {
    let mut chains = Vec::new();

    let has_sensitive_files = report.business_logic.iter().any(|f| {
        matches!(
            f.flaw_type,
            business::FlawType::TrustBoundaryViolation
        )
    });

    let has_idor = report.authz_bypasses.iter().any(|b| {
        matches!(b.bypass_type, authz::BypassType::Idor)
    });

    if has_sensitive_files && has_idor {
        let id = format!("de-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let steps = vec![
            ChainStep {
                step_number: 1,
                vulnerability: "IDOR on resource endpoints".to_string(),
                prerequisite: "Authenticated access".to_string(),
                impact: "Access other users' data".to_string(),
                evidence: "IDOR vulnerability found".to_string(),
                severity: Severity::High,
            },
            ChainStep {
                step_number: 2,
                vulnerability: "Sensitive files accessible".to_string(),
                prerequisite: "Server access".to_string(),
                impact: "Obtain credentials and configuration".to_string(),
                evidence: "Sensitive files found".to_string(),
                severity: Severity::Critical,
            },
        ];

        chains.push(AttackChain {
            id,
            name: "Data Exfiltration via IDOR + Config Leak".to_string(),
            chain_type: ChainType::DataExfiltration,
            steps,
            severity: Severity::Critical,
            description: "IDOR vulnerability combined with exposed configuration enables data exfiltration".to_string(),
            remediation: "Fix IDOR vulnerabilities and restrict access to sensitive files".to_string(),
            cvss_score: Some(9.5),
        });
    }

    chains
}

fn detect_session_exploitation_chain(report: &HuntReport) -> Vec<AttackChain> {
    let mut chains = Vec::new();

    let has_weak_session = report.session_issues.iter().any(|i| {
        matches!(
            i.issue_type,
            session::SessionIssueType::InsufficientEntropy
                | session::SessionIssueType::SessionFixation
                | session::SessionIssueType::MissingHttpOnly
                | session::SessionIssueType::MissingSecure
        )
    });

    let has_rate_limit_issue = report.business_logic.iter().any(|f| {
        matches!(f.flaw_type, business::FlawType::RateLimitBypass)
    });

    if has_weak_session && has_rate_limit_issue {
        let id = format!("se-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let steps = vec![
            ChainStep {
                step_number: 1,
                vulnerability: "Weak session management".to_string(),
                prerequisite: "Network position".to_string(),
                impact: "Session hijacking possible".to_string(),
                evidence: "Session security issue detected".to_string(),
                severity: Severity::High,
            },
            ChainStep {
                step_number: 2,
                vulnerability: "No rate limiting on authentication".to_string(),
                prerequisite: "Session weakness".to_string(),
                impact: "Brute force attacks possible".to_string(),
                evidence: "Rate limiting absent".to_string(),
                severity: Severity::Medium,
            },
        ];

        chains.push(AttackChain {
            id,
            name: "Session Hijacking + Brute Force".to_string(),
            chain_type: ChainType::LateralMovement,
            steps,
            severity: Severity::High,
            description: "Weak session security combined with no rate limiting enables account takeover"
                .to_string(),
            remediation: "Implement strong session management and rate limiting".to_string(),
            cvss_score: Some(7.5),
        });
    }

    chains
}

fn detect_rate_limit_chain(report: &HuntReport) -> Vec<AttackChain> {
    let mut chains = Vec::new();

    let has_no_rate_limit = report.business_logic.iter().any(|f| {
        matches!(f.flaw_type, business::FlawType::RateLimitBypass)
    });

    let has_admin_access = report.authz_bypasses.iter().any(|b| {
        matches!(b.bypass_type, authz::BypassType::MissingAuthorization)
    });

    if has_no_rate_limit && has_admin_access {
        let id = format!("rl-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        let steps = vec![
            ChainStep {
                step_number: 1,
                vulnerability: "No rate limiting".to_string(),
                prerequisite: "None".to_string(),
                impact: "Unlimited brute force attempts".to_string(),
                evidence: "Rate limiting absent".to_string(),
                severity: Severity::Medium,
            },
            ChainStep {
                step_number: 2,
                vulnerability: "Admin endpoint accessible".to_string(),
                prerequisite: "Admin credentials".to_string(),
                impact: "Full system compromise".to_string(),
                evidence: "Admin endpoint accessible".to_string(),
                severity: Severity::Critical,
            },
        ];

        chains.push(AttackChain {
            id,
            name: "Brute Force to Admin".to_string(),
            chain_type: ChainType::PrivilegeEscalation,
            steps,
            severity: Severity::Critical,
            description: "No rate limiting on login combined with accessible admin panel enables brute force attack".to_string(),
            remediation: "Implement rate limiting on authentication endpoints".to_string(),
            cvss_score: Some(8.0),
        });
    }

    chains
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
            steps: vec![ChainStep {
                step_number: 1,
                vulnerability: "IDOR".to_string(),
                prerequisite: "Valid user".to_string(),
                impact: "Access other user data".to_string(),
                evidence: "Evidence here".to_string(),
                severity: Severity::High,
            }],
            severity: Severity::High,
            description: "Test description".to_string(),
            remediation: "Fix it".to_string(),
            cvss_score: Some(7.5),
        };

        assert_eq!(chain.chain_type, ChainType::PrivilegeEscalation);
        assert_eq!(chain.steps.len(), 1);
    }

    #[tokio::test]
    async fn test_detect_attack_chains_empty_report() {
        let report = HuntReport::new("http://example.com");
        let chains = detect_attack_chains(&report).await.unwrap();
        assert!(chains.is_empty());
    }
}
