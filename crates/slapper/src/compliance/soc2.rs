use crate::compliance::{ComplianceFinding, ComplianceReport, ComplianceStatus};
use crate::error::Result;
use crate::types::Severity;

pub async fn generate_report(target: &str, findings: &[Severity]) -> Result<ComplianceReport> {
    let mut report = ComplianceReport {
        framework: "SOC 2 Type II".to_string(),
        target: target.to_string(),
        overall_score: 78.0,
        total_requirements: 5,
        passed: 4,
        failed: 1,
        findings: Vec::new(),
    };

    let has_critical = findings.iter().any(|s| *s == Severity::Critical);
    let has_high = findings.iter().any(|s| *s == Severity::High);
    let has_medium = findings.iter().any(|s| *s == Severity::Medium);
    let has_low = findings.iter().any(|s| *s == Severity::Low);
    let is_https = target.starts_with("https://");

    report.findings.push(ComplianceFinding {
        requirement_id: "CC6.1 - Logical and Physical Access Controls".to_string(),
        description: if has_high {
            "Insufficient access control measures detected".to_string()
        } else {
            "Access controls appear adequate".to_string()
        },
        severity: if has_high { Severity::High } else { Severity::Low },
        status: if has_high { ComplianceStatus::Fail } else { ComplianceStatus::Pass },
        remediation: "Implement comprehensive access control policies".to_string(),
    });

    report.findings.push(ComplianceFinding {
        requirement_id: "CC7.1 - System Operations".to_string(),
        description: if has_medium {
            "System monitoring gaps detected".to_string()
        } else {
            "System monitoring appears adequate".to_string()
        },
        severity: if has_medium { Severity::Medium } else { Severity::Info },
        status: if has_medium { ComplianceStatus::NeedsReview } else { ComplianceStatus::Pass },
        remediation: "Enhance logging and monitoring coverage".to_string(),
    });

    report.findings.push(ComplianceFinding {
        requirement_id: "CC8.1 - Risk Assessment".to_string(),
        description: if has_critical {
            "Critical security risks identified".to_string()
        } else {
            "No critical risks identified".to_string()
        },
        severity: if has_critical { Severity::Critical } else { Severity::Low },
        status: if has_critical { ComplianceStatus::Fail } else { ComplianceStatus::Pass },
        remediation: "Conduct thorough risk assessment and remediation".to_string(),
    });

    report.findings.push(ComplianceFinding {
        requirement_id: "CC6.6 - Security Measures".to_string(),
        description: if !is_https {
            "Insufficient encryption for data in transit".to_string()
        } else {
            "Encryption measures appear adequate".to_string()
        },
        severity: if !is_https { Severity::High } else { Severity::Info },
        status: if !is_https { ComplianceStatus::Fail } else { ComplianceStatus::Pass },
        remediation: "Implement TLS 1.2+ for all communications".to_string(),
    });

    report.findings.push(ComplianceFinding {
        requirement_id: "CC5.3 - Control Policies".to_string(),
        description: if has_low {
            "Information disclosure in server headers".to_string()
        } else {
            "Control policies appear well-documented".to_string()
        },
        severity: if has_low { Severity::Medium } else { Severity::Info },
        status: if has_low { ComplianceStatus::NeedsReview } else { ComplianceStatus::Pass },
        remediation: "Review and update security policies".to_string(),
    });

    let failed_count = report.findings.iter().filter(|f| f.status == ComplianceStatus::Fail).count();
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
    async fn test_soc2_report_with_findings() {
        let findings = vec![Severity::High, Severity::Critical];
        let report = generate_report("http://example.com", &findings).await.unwrap();
        assert!(report.failed > 0);
    }

    #[tokio::test]
    async fn test_soc2_report_clean() {
        let findings = vec![Severity::Info];
        let report = generate_report("https://example.com", &findings).await.unwrap();
        assert_eq!(report.failed, 0);
    }
}
