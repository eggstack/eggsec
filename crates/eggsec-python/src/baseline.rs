use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde::{Deserialize, Serialize};

use super::finding_schema::VersionedFindingPy;

/// Correlation result for a finding pair.
#[pyclass(frozen, name = "FindingCorrelation")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingCorrelationPy {
    #[pyo3(get)]
    pub baseline_finding_id: String,
    #[pyo3(get)]
    pub current_finding_id: String,
    #[pyo3(get)]
    pub correlation_method: String,
    #[pyo3(get)]
    pub confidence: f64,
    #[pyo3(get)]
    pub changed_fields: Vec<String>,
}

#[pymethods]
impl FindingCorrelationPy {
    #[new]
    fn new(
        baseline_finding_id: String,
        current_finding_id: String,
        correlation_method: String,
        confidence: f64,
        changed_fields: Vec<String>,
    ) -> Self {
        Self {
            baseline_finding_id,
            current_finding_id,
            correlation_method,
            confidence,
            changed_fields,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("baseline_finding_id", &self.baseline_finding_id)?;
        dict.set_item("current_finding_id", &self.current_finding_id)?;
        dict.set_item("correlation_method", &self.correlation_method)?;
        dict.set_item("confidence", self.confidence)?;
        dict.set_item("changed_fields", &self.changed_fields)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    fn __repr__(&self) -> String {
        format!(
            "FindingCorrelation(baseline={}, current={}, method={}, confidence={:.2})",
            self.baseline_finding_id,
            self.current_finding_id,
            self.correlation_method,
            self.confidence,
        )
    }
}

/// Diff status for a finding between baseline and current.
#[pyclass(frozen, name = "FindingDiff")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FindingDiffPy {
    New,
    Resolved,
    Changed,
    Unchanged,
    Suppressed,
    Indeterminate,
}

#[pymethods]
impl FindingDiffPy {
    fn as_str(&self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Resolved => "resolved",
            Self::Changed => "changed",
            Self::Unchanged => "unchanged",
            Self::Suppressed => "suppressed",
            Self::Indeterminate => "indeterminate",
        }
    }

    fn __repr__(&self) -> String {
        format!("FindingDiff.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

/// Summary of differences between two assessments.
#[pyclass(frozen, name = "AssessmentDiff")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentDiffPy {
    #[pyo3(get)]
    pub baseline_id: String,
    #[pyo3(get)]
    pub current_id: String,
    #[pyo3(get)]
    pub compared_at: String,
    #[pyo3(get)]
    pub new_findings: u32,
    #[pyo3(get)]
    pub resolved_findings: u32,
    #[pyo3(get)]
    pub changed_findings: u32,
    #[pyo3(get)]
    pub unchanged_findings: u32,
    #[pyo3(get)]
    pub suppressed_findings: u32,
    #[pyo3(get)]
    pub is_regression: bool,
    #[pyo3(get)]
    pub is_improvement: bool,
    #[pyo3(get)]
    pub summary: String,
}

#[pymethods]
impl AssessmentDiffPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("baseline_id", &self.baseline_id)?;
        dict.set_item("current_id", &self.current_id)?;
        dict.set_item("compared_at", &self.compared_at)?;
        dict.set_item("new_findings", self.new_findings)?;
        dict.set_item("resolved_findings", self.resolved_findings)?;
        dict.set_item("changed_findings", self.changed_findings)?;
        dict.set_item("unchanged_findings", self.unchanged_findings)?;
        dict.set_item("suppressed_findings", self.suppressed_findings)?;
        dict.set_item("is_regression", self.is_regression)?;
        dict.set_item("is_improvement", self.is_improvement)?;
        dict.set_item("summary", &self.summary)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    fn __repr__(&self) -> String {
        format!(
            "AssessmentDiff(baseline={}, current={}, new={}, resolved={}, changed={})",
            self.baseline_id,
            self.current_id,
            self.new_findings,
            self.resolved_findings,
            self.changed_findings,
        )
    }
}

/// Computes the set of fields that differ between two findings.
fn compute_changed_fields(a: &VersionedFindingPy, b: &VersionedFindingPy) -> Vec<String> {
    let mut changed = Vec::new();
    if a.title != b.title {
        changed.push("title".to_string());
    }
    if a.severity != b.severity {
        changed.push("severity".to_string());
    }
    if a.description != b.description {
        changed.push("description".to_string());
    }
    if a.affected_asset.identifier != b.affected_asset.identifier {
        changed.push("affected_asset.identifier".to_string());
    }
    if a.affected_asset.host != b.affected_asset.host {
        changed.push("affected_asset.host".to_string());
    }
    if a.finding_type.as_str() != b.finding_type.as_str() {
        changed.push("finding_type".to_string());
    }
    if a.confidence.as_str() != b.confidence.as_str() {
        changed.push("confidence".to_string());
    }
    if a.cve != b.cve {
        changed.push("cve".to_string());
    }
    if a.cwe != b.cwe {
        changed.push("cwe".to_string());
    }
    if a.tags != b.tags {
        changed.push("tags".to_string());
    }
    if a.source_tool != b.source_tool {
        changed.push("source_tool".to_string());
    }
    if a.remediation != b.remediation {
        changed.push("remediation".to_string());
    }
    if a.location.url != b.location.url {
        changed.push("location.url".to_string());
    }
    if a.location.path != b.location.path {
        changed.push("location.path".to_string());
    }
    changed
}

/// Baseline comparator for diffing two sets of findings.
#[pyclass(name = "BaselineComparator")]
pub struct BaselineComparatorPy {
    correlation_rules: std::sync::RwLock<Vec<String>>,
}

#[pymethods]
impl BaselineComparatorPy {
    #[new]
    fn new() -> Self {
        Self {
            correlation_rules: std::sync::RwLock::new(vec![
                "fingerprint".to_string(),
                "title".to_string(),
                "location".to_string(),
            ]),
        }
    }

    /// Compare baseline and current findings. Returns an AssessmentDiff summary.
    fn compare(
        &self,
        baseline: Vec<VersionedFindingPy>,
        current: Vec<VersionedFindingPy>,
    ) -> AssessmentDiffPy {
        let correlations = self.correlate_inner(&baseline, &current);
        let mut new_count = 0u32;
        let mut resolved_count = 0u32;
        let mut changed_count = 0u32;
        let mut unchanged_count = 0u32;

        let mut matched_current: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut matched_baseline: std::collections::HashSet<String> =
            std::collections::HashSet::new();

        for corr in &correlations {
            matched_current.insert(corr.current_finding_id.clone());
            matched_baseline.insert(corr.baseline_finding_id.clone());

            if corr.changed_fields.is_empty() {
                unchanged_count += 1;
            } else {
                changed_count += 1;
            }
        }

        for f in &current {
            if !matched_current.contains(&f.id) {
                new_count += 1;
            }
        }

        for f in &baseline {
            if !matched_baseline.contains(&f.id) {
                resolved_count += 1;
            }
        }

        let is_regression = new_count > resolved_count;
        let is_improvement = resolved_count > new_count;

        let summary = format!(
            "Compared {} baseline findings with {} current findings: {} new, {} resolved, {} changed, {} unchanged",
            baseline.len(),
            current.len(),
            new_count,
            resolved_count,
            changed_count,
            unchanged_count,
        );

        AssessmentDiffPy {
            baseline_id: String::new(),
            current_id: String::new(),
            compared_at: chrono::Utc::now().to_rfc3339(),
            new_findings: new_count,
            resolved_findings: resolved_count,
            changed_findings: changed_count,
            unchanged_findings: unchanged_count,
            suppressed_findings: 0,
            is_regression,
            is_improvement,
            summary,
        }
    }

    /// Return detailed correlations between baseline and current findings.
    fn correlate(
        &self,
        baseline: Vec<VersionedFindingPy>,
        current: Vec<VersionedFindingPy>,
    ) -> Vec<FindingCorrelationPy> {
        self.correlate_inner(&baseline, &current)
    }

    /// Add a correlation rule: "fingerprint", "title", "location", "cve".
    fn add_correlation_rule(&self, rule: &str) {
        if let Ok(mut rules) = self.correlation_rules.write() {
            let rule_lower = rule.to_lowercase();
            if !rules.iter().any(|r| r == &rule_lower) {
                rules.push(rule_lower);
            }
        }
    }

    /// Return the current correlation rules.
    fn correlation_rules(&self) -> Vec<String> {
        self.correlation_rules
            .read()
            .map(|r| r.clone())
            .unwrap_or_default()
    }
}

impl BaselineComparatorPy {
    fn correlate_inner(
        &self,
        baseline: &[VersionedFindingPy],
        current: &[VersionedFindingPy],
    ) -> Vec<FindingCorrelationPy> {
        let rules = self
            .correlation_rules
            .read()
            .map(|r| r.clone())
            .unwrap_or_default();
        let mut correlations = Vec::new();
        let mut used_current: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Pass 1: Exact fingerprint match (highest confidence)
        if rules.contains(&"fingerprint".to_string()) {
            for bf in baseline {
                if bf.fingerprint.is_empty() {
                    continue;
                }
                for cf in current {
                    if used_current.contains(&cf.id) {
                        continue;
                    }
                    if bf.fingerprint == cf.fingerprint {
                        let changed = compute_changed_fields(bf, cf);
                        correlations.push(FindingCorrelationPy {
                            baseline_finding_id: bf.id.clone(),
                            current_finding_id: cf.id.clone(),
                            correlation_method: "fingerprint".to_string(),
                            confidence: 1.0,
                            changed_fields: changed,
                        });
                        used_current.insert(cf.id.clone());
                        break;
                    }
                }
            }
        }

        // Pass 2: Title + target (affected_asset.identifier) match
        if rules.contains(&"title".to_string()) {
            for bf in baseline {
                if correlations.iter().any(|c| c.baseline_finding_id == bf.id) {
                    continue;
                }
                for cf in current {
                    if used_current.contains(&cf.id) {
                        continue;
                    }
                    if bf.title == cf.title
                        && bf.affected_asset.identifier == cf.affected_asset.identifier
                    {
                        let changed = compute_changed_fields(bf, cf);
                        correlations.push(FindingCorrelationPy {
                            baseline_finding_id: bf.id.clone(),
                            current_finding_id: cf.id.clone(),
                            correlation_method: "title".to_string(),
                            confidence: 0.8,
                            changed_fields: changed,
                        });
                        used_current.insert(cf.id.clone());
                        break;
                    }
                }
            }
        }

        // Pass 3: Location match (target + finding_type)
        if rules.contains(&"location".to_string()) {
            for bf in baseline {
                if correlations.iter().any(|c| c.baseline_finding_id == bf.id) {
                    continue;
                }
                for cf in current {
                    if used_current.contains(&cf.id) {
                        continue;
                    }
                    if bf.affected_asset.identifier == cf.affected_asset.identifier
                        && bf.finding_type.as_str() == cf.finding_type.as_str()
                    {
                        let changed = compute_changed_fields(bf, cf);
                        correlations.push(FindingCorrelationPy {
                            baseline_finding_id: bf.id.clone(),
                            current_finding_id: cf.id.clone(),
                            correlation_method: "location".to_string(),
                            confidence: 0.6,
                            changed_fields: changed,
                        });
                        used_current.insert(cf.id.clone());
                        break;
                    }
                }
            }
        }

        // Pass 4: CVE match
        if rules.contains(&"cve".to_string()) {
            for bf in baseline {
                if correlations.iter().any(|c| c.baseline_finding_id == bf.id) {
                    continue;
                }
                let bf_cve = match &bf.cve {
                    Some(c) if !c.is_empty() => c.clone(),
                    _ => continue,
                };
                for cf in current {
                    if used_current.contains(&cf.id) {
                        continue;
                    }
                    match &cf.cve {
                        Some(c) if c == &bf_cve => {
                            let changed = compute_changed_fields(bf, cf);
                            correlations.push(FindingCorrelationPy {
                                baseline_finding_id: bf.id.clone(),
                                current_finding_id: cf.id.clone(),
                                correlation_method: "cve".to_string(),
                                confidence: 0.9,
                                changed_fields: changed,
                            });
                            used_current.insert(cf.id.clone());
                            break;
                        }
                        _ => continue,
                    }
                }
            }
        }

        correlations
    }
}
