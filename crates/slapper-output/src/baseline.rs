use crate::agent::AgentFinding;
use rustc_hash::FxHashSet;

#[derive(Debug, Clone)]
pub struct BaselineComparison {
    pub new_findings: Vec<AgentFinding>,
    pub resolved_findings: Vec<AgentFinding>,
    pub unchanged_findings: Vec<AgentFinding>,
}

impl BaselineComparison {
    pub fn compare(current: &[AgentFinding], baseline: &[AgentFinding]) -> Self {
        let baseline_ids: FxHashSet<_> = baseline.iter().map(|f| f.id.clone()).collect();
        let current_ids: FxHashSet<_> = current.iter().map(|f| f.id.clone()).collect();

        let new_findings: Vec<_> = current
            .iter()
            .filter(|f| !baseline_ids.contains(&f.id))
            .cloned()
            .collect();

        let resolved_findings: Vec<_> = baseline
            .iter()
            .filter(|f| !current_ids.contains(&f.id))
            .cloned()
            .collect();

        let unchanged_findings: Vec<_> = current
            .iter()
            .filter(|f| baseline_ids.contains(&f.id))
            .cloned()
            .collect();

        Self {
            new_findings,
            resolved_findings,
            unchanged_findings,
        }
    }

    pub fn has_new_findings(&self) -> bool {
        !self.new_findings.is_empty()
    }

    pub fn new_finding_count(&self) -> usize {
        self.new_findings.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AttackSurface;
    use crate::agent::{AgentFinding, Confidence, Evidence, FindingStatus, Remediation};
    use slapper_core::types::Severity;
    use chrono::Utc;

    fn make_finding(id: &str, title: &str) -> AgentFinding {
        AgentFinding {
            id: id.to_string(),
            tool_id: "test".to_string(),
            vulnerability_type: "test".to_string(),
            severity: Severity::High,
            title: title.to_string(),
            description: String::new(),
            evidence: Evidence::default(),
            remediation: Remediation::default(),
            confidence: Confidence::Confirmed,
            cvss: None,
            cwe_ids: vec![],
            target: "example.com".to_string(),
            endpoint: String::new(),
            parameter: None,
            timestamp: Utc::now(),
            attack_surface: AttackSurface::Web,
            status: FindingStatus::New,
        }
    }

    #[test]
    fn test_compare_no_changes() {
        let f1 = make_finding("1", "XSS");
        let baseline = vec![f1.clone()];
        let current = vec![f1];
        let result = BaselineComparison::compare(&current, &baseline);
        assert!(result.new_findings.is_empty());
        assert!(result.resolved_findings.is_empty());
        assert_eq!(result.unchanged_findings.len(), 1);
    }

    #[test]
    fn test_compare_new_finding() {
        let f1 = make_finding("1", "XSS");
        let f2 = make_finding("2", "SQLi");
        let baseline = vec![f1.clone()];
        let current = vec![f1, f2.clone()];
        let result = BaselineComparison::compare(&current, &baseline);
        assert_eq!(result.new_findings.len(), 1);
        assert_eq!(result.new_findings[0].id, "2");
        assert!(result.resolved_findings.is_empty());
    }

    #[test]
    fn test_compare_resolved_finding() {
        let f1 = make_finding("1", "XSS");
        let f2 = make_finding("2", "SQLi");
        let baseline = vec![f1.clone(), f2];
        let current = vec![f1];
        let result = BaselineComparison::compare(&current, &baseline);
        assert!(result.new_findings.is_empty());
        assert_eq!(result.resolved_findings.len(), 1);
        assert_eq!(result.resolved_findings[0].id, "2");
    }

    #[test]
    fn test_compare_mixed() {
        let f1 = make_finding("1", "XSS");
        let f2 = make_finding("2", "SQLi");
        let f3 = make_finding("3", "CSRF");
        let baseline = vec![f1.clone(), f2];
        let current = vec![f1, f3.clone()];
        let result = BaselineComparison::compare(&current, &baseline);
        assert_eq!(result.new_findings.len(), 1);
        assert_eq!(result.new_findings[0].id, "3");
        assert_eq!(result.resolved_findings.len(), 1);
        assert_eq!(result.resolved_findings[0].id, "2");
        assert_eq!(result.unchanged_findings.len(), 1);
    }

    #[test]
    fn test_compare_empty_baseline() {
        let f1 = make_finding("1", "XSS");
        let baseline: Vec<AgentFinding> = vec![];
        let current = vec![f1];
        let result = BaselineComparison::compare(&current, &baseline);
        assert_eq!(result.new_findings.len(), 1);
        assert!(result.resolved_findings.is_empty());
    }

    #[test]
    fn test_compare_empty_current() {
        let f1 = make_finding("1", "XSS");
        let baseline = vec![f1];
        let current: Vec<AgentFinding> = vec![];
        let result = BaselineComparison::compare(&current, &baseline);
        assert!(result.new_findings.is_empty());
        assert_eq!(result.resolved_findings.len(), 1);
    }

    #[test]
    fn test_compare_both_empty() {
        let baseline: Vec<AgentFinding> = vec![];
        let current: Vec<AgentFinding> = vec![];
        let result = BaselineComparison::compare(&current, &baseline);
        assert!(result.new_findings.is_empty());
        assert!(result.resolved_findings.is_empty());
        assert!(result.unchanged_findings.is_empty());
    }

    #[test]
    fn test_has_new_findings() {
        let f1 = make_finding("1", "XSS");
        let comp = BaselineComparison {
            new_findings: vec![f1],
            resolved_findings: vec![],
            unchanged_findings: vec![],
        };
        assert!(comp.has_new_findings());
    }

    #[test]
    fn test_has_no_new_findings() {
        let comp = BaselineComparison {
            new_findings: vec![],
            resolved_findings: vec![],
            unchanged_findings: vec![],
        };
        assert!(!comp.has_new_findings());
    }

    #[test]
    fn test_new_finding_count() {
        let f1 = make_finding("1", "XSS");
        let f2 = make_finding("2", "SQLi");
        let comp = BaselineComparison {
            new_findings: vec![f1, f2],
            resolved_findings: vec![],
            unchanged_findings: vec![],
        };
        assert_eq!(comp.new_finding_count(), 2);
    }
}
