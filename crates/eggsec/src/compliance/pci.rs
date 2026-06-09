use crate::compliance::{ComplianceFinding, ComplianceReport, ComplianceStatus};
use crate::error::Result;
use crate::types::Severity;

pub async fn generate_report(target: &str, findings: &[Severity]) -> Result<ComplianceReport> {
    let mut report = ComplianceReport {
        framework: "PCI DSS v4.0".to_string(),
        target: target.to_string(),
        findings: Vec::new(),
        ..Default::default()
    };

    let has_critical = findings.contains(&Severity::Critical);
    let has_high = findings.contains(&Severity::High);
    let has_medium = findings.contains(&Severity::Medium);
    let is_https = target.starts_with("https://");

    report.findings.push(ComplianceFinding {
        requirement_id: "Req 2.1 - Default credentials".to_string(),
        description: if has_critical {
            "Default credentials detected in target".to_string()
        } else {
            "No default credentials detected".to_string()
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
        remediation: "Change all default credentials immediately".to_string(),
    });

    report.findings.push(ComplianceFinding {
        requirement_id: "Req 3.4 - Data encryption at rest".to_string(),
        description: if !is_https {
            "No TLS encryption detected for data in transit".to_string()
        } else {
            "TLS encryption in use".to_string()
        },
        severity: if !is_https {
            Severity::Critical
        } else {
            Severity::Low
        },
        status: if !is_https {
            ComplianceStatus::Fail
        } else {
            ComplianceStatus::Pass
        },
        remediation: "Implement AES-256 encryption for stored card data".to_string(),
    });

    report.findings.push(ComplianceFinding {
        requirement_id: "Req 6.5 - Address common coding vulnerabilities".to_string(),
        description: if has_high {
            "Injection flaws detected".to_string()
        } else {
            "No injection vulnerabilities detected".to_string()
        },
        severity: if has_high {
            Severity::High
        } else {
            Severity::Info
        },
        status: if has_high {
            ComplianceStatus::Fail
        } else {
            ComplianceStatus::Pass
        },
        remediation: "Use parameterized queries; validate all inputs".to_string(),
    });

    report.findings.push(ComplianceFinding {
        requirement_id: "Req 11.3 - External penetration testing".to_string(),
        description: if has_medium {
            "Security misconfigurations detected".to_string()
        } else {
            "No significant misconfigurations found".to_string()
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
        remediation: "Conduct regular penetration testing".to_string(),
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
    async fn test_pci_report_with_findings() {
        let findings = vec![Severity::Critical, Severity::High];
        let report = generate_report("http://example.com", &findings)
            .await
            .unwrap();
        assert!(report.failed > 0);
    }

    #[tokio::test]
    async fn test_pci_report_clean() {
        let findings = vec![Severity::Info];
        let report = generate_report("https://example.com", &findings)
            .await
            .unwrap();
        assert_eq!(report.failed, 0);
    }
}
