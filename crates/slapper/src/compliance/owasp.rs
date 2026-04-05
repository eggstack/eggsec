use crate::compliance::{ComplianceFinding, ComplianceReport, ComplianceStatus};
use crate::error::Result;
use crate::types::Severity;

pub async fn generate_report(target: &str, findings: &[Severity]) -> Result<ComplianceReport> {
    let mut report = ComplianceReport {
        framework: "OWASP Top 10".to_string(),
        target: target.to_string(),
        overall_score: 75.0,
        total_requirements: 10,
        passed: 7,
        failed: 3,
        findings: Vec::new(),
    };

    let has_critical = findings.iter().any(|s| *s == Severity::Critical);
    let has_high = findings.iter().any(|s| *s == Severity::High);
    let has_medium = findings.iter().any(|s| *s == Severity::Medium);
    let has_low = findings.iter().any(|s| *s == Severity::Low);
    let is_https = target.starts_with("https://");

    report.findings.push(ComplianceFinding {
        requirement_id: "A01:2021 - Broken Access Control".to_string(),
        description: if has_high {
            "Access control vulnerabilities detected".to_string()
        } else {
            "No critical access control issues found".to_string()
        },
        severity: if has_high { Severity::Critical } else { Severity::Low },
        status: if has_high { ComplianceStatus::Fail } else { ComplianceStatus::Pass },
        remediation: "Implement proper authorization on all endpoints".to_string(),
    });

    report.findings.push(ComplianceFinding {
        requirement_id: "A02:2021 - Cryptographic Failures".to_string(),
        description: if !is_https {
            "Sensitive data exposed via insecure transmission".to_string()
        } else {
            "TLS encryption in use".to_string()
        },
        severity: if !is_https { Severity::High } else { Severity::Low },
        status: if !is_https { ComplianceStatus::Fail } else { ComplianceStatus::Pass },
        remediation: "Use TLS 1.3 for all connections; encrypt sensitive data at rest".to_string(),
    });

    report.findings.push(ComplianceFinding {
        requirement_id: "A03:2021 - Injection".to_string(),
        description: if has_critical {
            "Injection vulnerabilities detected".to_string()
        } else {
            "No injection issues detected".to_string()
        },
        severity: if has_critical { Severity::Critical } else { Severity::Info },
        status: if has_critical { ComplianceStatus::Fail } else { ComplianceStatus::Pass },
        remediation: "Use parameterized queries; validate all inputs".to_string(),
    });

    report.findings.push(ComplianceFinding {
        requirement_id: "A05:2021 - Security Misconfiguration".to_string(),
        description: if has_medium {
            "Security headers missing or misconfigured".to_string()
        } else {
            "Security configuration appears adequate".to_string()
        },
        severity: if has_medium { Severity::Medium } else { Severity::Low },
        status: if has_medium { ComplianceStatus::Fail } else { ComplianceStatus::Pass },
        remediation: "Review security headers; disable unnecessary features".to_string(),
    });

    report.findings.push(ComplianceFinding {
        requirement_id: "A09:2021 - Security Logging & Monitoring".to_string(),
        description: if has_low {
            "Information disclosure detected in headers".to_string()
        } else {
            "No information disclosure issues found".to_string()
        },
        severity: if has_low { Severity::Medium } else { Severity::Info },
        status: if has_low { ComplianceStatus::NeedsReview } else { ComplianceStatus::Pass },
        remediation: "Remove server/version headers; implement proper logging".to_string(),
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
    async fn test_owasp_report_with_findings() {
        let findings = vec![Severity::High, Severity::Critical];
        let report = generate_report("http://example.com", &findings).await.unwrap();
        assert!(!report.findings.is_empty());
        assert!(report.failed > 0);
    }

    #[tokio::test]
    async fn test_owasp_report_clean() {
        let findings = vec![Severity::Info];
        let report = generate_report("https://example.com", &findings).await.unwrap();
        assert_eq!(report.failed, 0);
    }
}
