use crate::compliance::{ComplianceFinding, ComplianceReport, ComplianceStatus};
use crate::error::Result;
use crate::types::Severity;

pub async fn generate_report(target: &str, findings: &[Severity]) -> Result<ComplianceReport> {
    let mut report = ComplianceReport {
        framework: "HIPAA Security Rule".to_string(),
        target: target.to_string(),
        findings: Vec::new(),
        ..Default::default()
    };

    let has_critical = findings.contains(&Severity::Critical);
    let has_high = findings.contains(&Severity::High);
    let has_medium = findings.contains(&Severity::Medium);
    let is_https = target.starts_with("https://");

    report.findings.push(ComplianceFinding {
        requirement_id: "§164.312(a)(1) - Access Control".to_string(),
        description: if has_critical {
            "Unauthorized access vulnerabilities detected".to_string()
        } else {
            "Access controls appear adequate".to_string()
        },
        severity: if has_critical {
            Severity::Critical
        } else {
            Severity::Low
        },
        status: if has_critical {
            ComplianceStatus::Fail
        } else {
            ComplianceStatus::Pass
        },
        remediation: "Implement role-based access control for all PHI systems".to_string(),
    });

    report.findings.push(ComplianceFinding {
        requirement_id: "§164.312(d) - Person or Entity Authentication".to_string(),
        description: if has_high {
            "Insufficient authentication controls detected".to_string()
        } else {
            "Authentication controls appear adequate".to_string()
        },
        severity: if has_high {
            Severity::High
        } else {
            Severity::Low
        },
        status: if has_high {
            ComplianceStatus::Fail
        } else {
            ComplianceStatus::Pass
        },
        remediation: "Implement multi-factor authentication".to_string(),
    });

    report.findings.push(ComplianceFinding {
        requirement_id: "§164.312(e)(1) - Transmission Security".to_string(),
        description: if !is_https {
            "PHI may be transmitted without adequate encryption".to_string()
        } else {
            "TLS encryption detected for transmissions".to_string()
        },
        severity: if !is_https {
            Severity::High
        } else {
            Severity::Low
        },
        status: if !is_https {
            ComplianceStatus::Fail
        } else {
            ComplianceStatus::Pass
        },
        remediation: "Implement TLS 1.2+ for all PHI transmissions".to_string(),
    });

    report.findings.push(ComplianceFinding {
        requirement_id: "§164.312(b) - Audit Controls".to_string(),
        description: if has_medium {
            "Audit logging gaps detected".to_string()
        } else {
            "Audit controls appear adequate".to_string()
        },
        severity: if has_medium {
            Severity::Medium
        } else {
            Severity::Info
        },
        status: if has_medium {
            ComplianceStatus::NeedsReview
        } else {
            ComplianceStatus::Pass
        },
        remediation: "Implement comprehensive audit logging for all PHI access".to_string(),
    });

    let failed_count = report
        .findings
        .iter()
        .filter(|f| f.status == ComplianceStatus::Fail)
        .count();
    let total = report.findings.len();
    report.total_requirements = total;
    report.passed = total - failed_count;
    report.failed = failed_count;
    report.overall_score = if total > 0 {
        ((total - failed_count) as f32 / total as f32) * 100.0
    } else {
        100.0
    };

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hipaa_report_with_findings() {
        let findings = vec![Severity::Critical, Severity::High];
        let report = generate_report("http://example.com", &findings)
            .await
            .unwrap();
        assert!(report.failed > 0);
    }

    #[tokio::test]
    async fn test_hipaa_report_clean() {
        let findings = vec![Severity::Info];
        let report = generate_report("https://example.com", &findings)
            .await
            .unwrap();
        assert_eq!(report.failed, 0);
    }
}
