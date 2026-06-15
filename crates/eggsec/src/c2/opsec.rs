use super::{OpsecAssessment, OpsecCategory, OpsecFinding, OpsecSeverity};

pub fn simulate_opsec_assessment() -> OpsecAssessment {
    let findings = vec![
        OpsecFinding {
            category: OpsecCategory::ParentSpoofing,
            severity: OpsecSeverity::Medium,
            description: "Parent process spoofing not implemented".to_string(),
            recommendation: "Implement parent process ID spoofing for process creation".to_string(),
        },
        OpsecFinding {
            category: OpsecCategory::Timestomping,
            severity: OpsecSeverity::Low,
            description: "File timestamps not modified".to_string(),
            recommendation: "Apply timestomping to created/modified files".to_string(),
        },
        OpsecFinding {
            category: OpsecCategory::LogTampering,
            severity: OpsecSeverity::High,
            description: "Security event logs not cleared".to_string(),
            recommendation: "Clear or modify security event logs post-execution".to_string(),
        },
        OpsecFinding {
            category: OpsecCategory::ProcessMasquerading,
            severity: OpsecSeverity::Medium,
            description: "Process names are not disguised".to_string(),
            recommendation: "Masquerade process names as legitimate system processes".to_string(),
        },
        OpsecFinding {
            category: OpsecCategory::BurnMechanism,
            severity: OpsecSeverity::Info,
            description: "No self-destruct mechanism detected".to_string(),
            recommendation: "Implement agent self-destruct after mission completion".to_string(),
        },
        OpsecFinding {
            category: OpsecCategory::DecoyActivity,
            severity: OpsecSeverity::Low,
            description: "No decoy network traffic generated".to_string(),
            recommendation: "Generate decoy traffic to blend with legitimate activity".to_string(),
        },
    ];

    // Score: 100 - (findings weighted by severity)
    let mut score = 100u32;
    for finding in &findings {
        match finding.severity {
            OpsecSeverity::High => score = score.saturating_sub(20),
            OpsecSeverity::Medium => score = score.saturating_sub(10),
            OpsecSeverity::Low => score = score.saturating_sub(5),
            OpsecSeverity::Info => {}
        }
    }

    OpsecAssessment {
        score,
        max_score: 100,
        findings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulate_opsec_assessment() {
        let assessment = simulate_opsec_assessment();
        assert!(assessment.score <= assessment.max_score);
        assert!(!assessment.findings.is_empty());
        assert_eq!(assessment.findings.len(), 6);
    }

    #[test]
    fn test_opsec_score_calculation() {
        let assessment = simulate_opsec_assessment();
        // 6 findings: 1 High(-20), 2 Medium(-10 each = -20), 2 Low(-5 each = -10), 1 Info(0)
        // 100 - 20 - 20 - 10 = 50
        assert_eq!(assessment.score, 50);
    }

    #[test]
    fn test_opsec_findings_categories() {
        let assessment = simulate_opsec_assessment();
        let categories: Vec<_> = assessment.findings.iter().map(|f| f.category).collect();
        assert!(categories.contains(&OpsecCategory::ParentSpoofing));
        assert!(categories.contains(&OpsecCategory::Timestomping));
        assert!(categories.contains(&OpsecCategory::LogTampering));
        assert!(categories.contains(&OpsecCategory::ProcessMasquerading));
        assert!(categories.contains(&OpsecCategory::BurnMechanism));
        assert!(categories.contains(&OpsecCategory::DecoyActivity));
    }
}
