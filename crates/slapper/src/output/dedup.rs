use crate::output::agent::AgentFinding;
use std::collections::HashMap;
use uuid::Uuid;

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
        match s.to_lowercase().as_str() {
            "strict" => Ok(DedupStrategy::Strict),
            "fuzzy" => Ok(DedupStrategy::Fuzzy),
            "disabled" => Ok(DedupStrategy::Disabled),
            _ => Err(format!("Unknown dedup strategy: {}", s)),
        }
    }
}

pub struct DedupEngine {
    strategy: DedupStrategy,
    seen: HashMap<String, Uuid>,
}

impl DedupEngine {
    pub fn new(strategy: DedupStrategy) -> Self {
        Self {
            strategy,
            seen: HashMap::new(),
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
                self.seen.insert(key, Uuid::new_v4()).is_none()
            })
            .cloned()
            .collect()
    }

    fn dedup_fuzzy(&mut self, findings: &[AgentFinding]) -> Vec<AgentFinding> {
        findings
            .iter()
            .filter(|f| {
                let key = format!("{}:{}", f.severity, f.title);
                self.seen.insert(key, Uuid::new_v4()).is_none()
            })
            .cloned()
            .collect()
    }
}
