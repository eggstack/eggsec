use crate::agent::AgentFinding;
use rustc_hash::FxHashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DedupStrategy {
    Strict,
    Fuzzy,
    #[default]
    Disabled,
}

impl std::str::FromStr for DedupStrategy {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            s if s.eq_ignore_ascii_case("strict") => Ok(DedupStrategy::Strict),
            s if s.eq_ignore_ascii_case("fuzzy") => Ok(DedupStrategy::Fuzzy),
            s if s.eq_ignore_ascii_case("disabled") => Ok(DedupStrategy::Disabled),
            _ => Err(format!("Unknown dedup strategy: {}", s)),
        }
    }
}

pub struct DedupEngine {
    strategy: DedupStrategy,
    seen: FxHashSet<String>,
}

impl DedupEngine {
    pub fn new(strategy: DedupStrategy) -> Self {
        Self {
            strategy,
            seen: FxHashSet::default(),
        }
    }

    pub fn deduplicate(&mut self, findings: &[AgentFinding]) -> Vec<AgentFinding> {
        match self.strategy {
            DedupStrategy::Disabled => findings.to_vec(),
            DedupStrategy::Strict => self.dedup_strict(findings),
            DedupStrategy::Fuzzy => self.dedup_fuzzy(findings),
        }
    }

    fn dedup_strict(&mut self, findings: &[AgentFinding]) -> Vec<AgentFinding> {
        findings
            .iter()
            .filter(|f| {
                let key = format!("{}:{}:{}", f.severity, f.title, f.target);
                self.seen.insert(key)
            })
            .cloned()
            .collect()
    }

    fn dedup_fuzzy(&mut self, findings: &[AgentFinding]) -> Vec<AgentFinding> {
        findings
            .iter()
            .filter(|f| {
                let key = format!("{}:{}", f.severity, f.title);
                self.seen.insert(key)
            })
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{AgentFinding, Confidence, Evidence, FindingStatus, Remediation};
    use chrono::Utc;
    use slapper_core::types::Severity;

    fn make_finding(id: &str, severity: Severity, title: &str, target: &str) -> AgentFinding {
        AgentFinding {
            id: id.to_string(),
            tool_id: "test".to_string(),
            vulnerability_type: "test".to_string(),
            severity,
            title: title.to_string(),
            description: String::new(),
            evidence: Evidence::default(),
            remediation: Remediation::new("fix it"),
            confidence: Confidence::Confirmed,
            cvss: None,
            cwe_ids: vec![],
            target: target.to_string(),
            endpoint: String::new(),
            parameter: None,
            timestamp: Utc::now(),
            attack_surface: crate::agent::AttackSurface::Web,
            status: FindingStatus::New,
        }
    }

    #[test]
    fn test_dedup_strategy_from_str() {
        assert_eq!(
            "strict".parse::<DedupStrategy>().unwrap(),
            DedupStrategy::Strict
        );
        assert_eq!(
            "STRICT".parse::<DedupStrategy>().unwrap(),
            DedupStrategy::Strict
        );
        assert_eq!(
            "fuzzy".parse::<DedupStrategy>().unwrap(),
            DedupStrategy::Fuzzy
        );
        assert_eq!(
            "disabled".parse::<DedupStrategy>().unwrap(),
            DedupStrategy::Disabled
        );
        assert!("unknown".parse::<DedupStrategy>().is_err());
    }

    #[test]
    fn test_dedup_strategy_default() {
        assert_eq!(DedupStrategy::default(), DedupStrategy::Disabled);
    }

    #[test]
    fn test_disabled_returns_all() {
        let f1 = make_finding("1", Severity::High, "XSS", "example.com");
        let f2 = make_finding("1", Severity::High, "XSS", "example.com");
        let findings = vec![f1, f2];
        let mut engine = DedupEngine::new(DedupStrategy::Disabled);
        let result = engine.deduplicate(&findings);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_strict_dedup_by_severity_title_target() {
        let f1 = make_finding("1", Severity::High, "XSS", "example.com");
        let f2 = make_finding("2", Severity::High, "XSS", "example.com");
        let f3 = make_finding("3", Severity::High, "XSS", "other.com");
        let f4 = make_finding("4", Severity::Critical, "XSS", "example.com");
        let findings = vec![f1, f2, f3, f4];
        let mut engine = DedupEngine::new(DedupStrategy::Strict);
        let result = engine.deduplicate(&findings);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_fuzzy_dedup_by_severity_title() {
        let f1 = make_finding("1", Severity::High, "XSS", "example.com");
        let f2 = make_finding("2", Severity::High, "XSS", "other.com");
        let f3 = make_finding("3", Severity::Critical, "XSS", "example.com");
        let findings = vec![f1, f2, f3];
        let mut engine = DedupEngine::new(DedupStrategy::Fuzzy);
        let result = engine.deduplicate(&findings);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_empty_findings() {
        let findings: Vec<AgentFinding> = vec![];
        let mut engine = DedupEngine::new(DedupStrategy::Strict);
        let result = engine.deduplicate(&findings);
        assert!(result.is_empty());
    }
}
