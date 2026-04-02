use crate::output::agent::AgentFinding;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct BaselineComparison {
    pub new_findings: Vec<AgentFinding>,
    pub resolved_findings: Vec<AgentFinding>,
    pub unchanged_findings: Vec<AgentFinding>,
}

impl BaselineComparison {
    pub fn compare(current: &[AgentFinding], baseline: &[AgentFinding]) -> Self {
        let baseline_ids: HashSet<_> = baseline.iter().map(|f| f.id.clone()).collect();
        let current_ids: HashSet<_> = current.iter().map(|f| f.id.clone()).collect();

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
