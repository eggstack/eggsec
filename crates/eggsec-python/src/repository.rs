use std::collections::HashMap;

use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde::{Deserialize, Serialize};

use super::finding_schema::VersionedFindingPy;

/// Severity order for comparison (higher = more severe).
fn severity_rank(severity: &str) -> u8 {
    match severity.to_lowercase().as_str() {
        "critical" => 5,
        "high" => 4,
        "medium" => 3,
        "low" => 2,
        "info" | "informational" => 1,
        _ => 0,
    }
}

/// Parse an RFC 3339 / ISO 8601 datetime string to a timestamp in seconds since epoch.
/// Returns 0 on parse failure.
fn parse_timestamp(s: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.timestamp())
        .unwrap_or(0)
}

/// In-memory finding repository for Python bindings.
/// Provides query/filter/find capabilities without requiring SQLite.
#[pyclass(name = "FindingRepository")]
pub struct FindingRepositoryPy {
    findings: std::sync::RwLock<Vec<VersionedFindingPy>>,
}

#[pymethods]
impl FindingRepositoryPy {
    #[new]
    fn new() -> Self {
        Self {
            findings: std::sync::RwLock::new(Vec::new()),
        }
    }

    /// Add a single finding. Rejects if a finding with the same ID already exists.
    fn add_finding(&self, finding: VersionedFindingPy) -> PyResult<()> {
        let mut findings = self
            .findings
            .write()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        if findings.iter().any(|f| f.id == finding.id) {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Finding with ID '{}' already exists",
                finding.id
            )));
        }
        findings.push(finding);
        Ok(())
    }

    /// Add multiple findings. Returns the count of findings actually added
    /// (skips duplicates silently).
    fn add_findings(&self, findings: Vec<VersionedFindingPy>) -> PyResult<u32> {
        let mut store = self
            .findings
            .write()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let mut added = 0u32;
        for finding in findings {
            if !store.iter().any(|f| f.id == finding.id) {
                store.push(finding);
                added += 1;
            }
        }
        Ok(added)
    }

    /// Get a finding by its ID.
    fn get_finding(&self, finding_id: &str) -> Option<VersionedFindingPy> {
        let findings = self.findings.read().ok()?;
        findings.iter().find(|f| f.id == finding_id).cloned()
    }

    /// Remove a finding by ID. Returns true if found and removed.
    fn remove_finding(&self, finding_id: &str) -> bool {
        let Ok(mut findings) = self.findings.write() else {
            return false;
        };
        let before = findings.len();
        findings.retain(|f| f.id != finding_id);
        findings.len() < before
    }

    /// Return the number of findings in the repository.
    fn count(&self) -> usize {
        self.findings.read().map(|f| f.len()).unwrap_or(0)
    }

    /// Return true if the repository contains no findings.
    fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// Remove all findings from the repository.
    fn clear(&self) {
        if let Ok(mut findings) = self.findings.write() {
            findings.clear();
        }
    }

    /// List all finding IDs.
    fn list_ids(&self) -> Vec<String> {
        self.findings
            .read()
            .map(|f| f.iter().map(|f| f.id.clone()).collect())
            .unwrap_or_default()
    }

    /// Return all findings.
    fn all_findings(&self) -> Vec<VersionedFindingPy> {
        self.findings.read().map(|f| f.to_vec()).unwrap_or_default()
    }

    /// Filter findings by severity (case-insensitive).
    fn by_severity(&self, severity: &str) -> Vec<VersionedFindingPy> {
        let target = severity.to_lowercase();
        self.findings
            .read()
            .map(|f| {
                f.iter()
                    .filter(|f| f.severity.to_lowercase() == target)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Filter findings by confidence level (case-insensitive).
    fn by_confidence(&self, confidence: &str) -> Vec<VersionedFindingPy> {
        let target = confidence.to_lowercase();
        self.findings
            .read()
            .map(|f| {
                f.iter()
                    .filter(|f| f.confidence.as_str().to_lowercase() == target)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Filter findings by finding type (case-insensitive).
    fn by_finding_type(&self, finding_type: &str) -> Vec<VersionedFindingPy> {
        let target = finding_type.to_lowercase();
        self.findings
            .read()
            .map(|f| {
                f.iter()
                    .filter(|f| f.finding_type.as_str().to_lowercase() == target)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Filter findings by affected asset identifier (exact match).
    fn by_target(&self, target: &str) -> Vec<VersionedFindingPy> {
        self.findings
            .read()
            .map(|f| {
                f.iter()
                    .filter(|f| f.affected_asset.identifier == target)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Filter findings by CVE ID (case-insensitive).
    fn by_cve(&self, cve: &str) -> Vec<VersionedFindingPy> {
        let target = cve.to_lowercase();
        self.findings
            .read()
            .map(|f| {
                f.iter()
                    .filter(|f| {
                        f.cve
                            .as_ref()
                            .map(|c| c.to_lowercase() == target)
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Filter findings by CWE ID (case-insensitive).
    fn by_cwe(&self, cwe: &str) -> Vec<VersionedFindingPy> {
        let target = cwe.to_lowercase();
        self.findings
            .read()
            .map(|f| {
                f.iter()
                    .filter(|f| {
                        f.cwe
                            .as_ref()
                            .map(|c| c.to_lowercase() == target)
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Filter findings by tag (exact match).
    fn by_tag(&self, tag: &str) -> Vec<VersionedFindingPy> {
        self.findings
            .read()
            .map(|f| {
                f.iter()
                    .filter(|f| f.tags.iter().any(|t| t == tag))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Filter findings by source tool name (case-insensitive).
    fn by_tool(&self, tool: &str) -> Vec<VersionedFindingPy> {
        let target = tool.to_lowercase();
        self.findings
            .read()
            .map(|f| {
                f.iter()
                    .filter(|f| f.source_tool.to_lowercase() == target)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Filter findings by date range (RFC 3339 strings, inclusive).
    fn by_date_range(&self, start: &str, end: &str) -> PyResult<Vec<VersionedFindingPy>> {
        let start_ts = parse_timestamp(start);
        let end_ts = parse_timestamp(end);
        if start_ts == 0 || end_ts == 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Invalid date format. Use RFC 3339 (e.g., 2024-01-01T00:00:00Z)",
            ));
        }
        Ok(self
            .findings
            .read()
            .map(|f| {
                f.iter()
                    .filter(|f| {
                        let ts = parse_timestamp(&f.discovered_at);
                        ts >= start_ts && ts <= end_ts
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default())
    }

    /// Combined filter: all provided criteria are AND-ed together.
    /// min_severity filters out findings below the given severity level.
    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (severity=None, confidence=None, finding_type=None, target=None, cve=None, cwe=None, tag=None, tool=None, min_severity=None))]
    fn filter(
        &self,
        severity: Option<&str>,
        confidence: Option<&str>,
        finding_type: Option<&str>,
        target: Option<&str>,
        cve: Option<&str>,
        cwe: Option<&str>,
        tag: Option<&str>,
        tool: Option<&str>,
        min_severity: Option<&str>,
    ) -> Vec<VersionedFindingPy> {
        let min_sev_rank = min_severity.map(|s| severity_rank(s)).unwrap_or(0);
        self.findings
            .read()
            .map(|f| {
                f.iter()
                    .filter(|f| {
                        if let Some(s) = severity {
                            if f.severity.to_lowercase() != s.to_lowercase() {
                                return false;
                            }
                        }
                        if let Some(c) = confidence {
                            if f.confidence.as_str().to_lowercase() != c.to_lowercase() {
                                return false;
                            }
                        }
                        if let Some(ft) = finding_type {
                            if f.finding_type.as_str().to_lowercase() != ft.to_lowercase() {
                                return false;
                            }
                        }
                        if let Some(t) = target {
                            if f.affected_asset.identifier != t {
                                return false;
                            }
                        }
                        if let Some(cve_id) = cve {
                            match &f.cve {
                                Some(c) if c.to_lowercase() == cve_id.to_lowercase() => {}
                                _ => return false,
                            }
                        }
                        if let Some(cwe_id) = cwe {
                            match &f.cwe {
                                Some(c) if c.to_lowercase() == cwe_id.to_lowercase() => {}
                                _ => return false,
                            }
                        }
                        if let Some(tg) = tag {
                            if !f.tags.iter().any(|t| t == tg) {
                                return false;
                            }
                        }
                        if let Some(tool_name) = tool {
                            if f.source_tool.to_lowercase() != tool_name.to_lowercase() {
                                return false;
                            }
                        }
                        if min_sev_rank > 0 && severity_rank(&f.severity) < min_sev_rank {
                            return false;
                        }
                        true
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Remove duplicate findings based on fingerprint. Keeps the first occurrence.
    /// Returns the number of duplicates removed.
    fn deduplicate(&self) -> u32 {
        let Ok(mut findings) = self.findings.write() else {
            return 0;
        };
        let before = findings.len();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        findings.retain(|f| {
            let key = if f.fingerprint.is_empty() {
                format!(
                    "{}|{}|{}",
                    f.title,
                    f.affected_asset.identifier,
                    f.finding_type.as_str()
                )
            } else {
                f.fingerprint.clone()
            };
            seen.insert(key)
        });
        (before - findings.len()) as u32
    }

    /// Serialize the repository to a Python dictionary.
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let findings = self.findings.read().map(|f| f.to_vec()).unwrap_or_default();
        let dict = PyDict::new_bound(py);
        let list = pyo3::types::PyList::empty_bound(py);
        for f in &findings {
            let item_dict = PyDict::new_bound(py);
            item_dict.set_item("id", &f.id)?;
            item_dict.set_item("title", &f.title)?;
            item_dict.set_item("severity", &f.severity)?;
            item_dict.set_item("description", &f.description)?;
            item_dict.set_item("fingerprint", &f.fingerprint)?;
            item_dict.set_item("confidence", f.confidence.as_str())?;
            item_dict.set_item("finding_type", f.finding_type.as_str())?;
            item_dict.set_item("cve", &f.cve)?;
            item_dict.set_item("cwe", &f.cwe)?;
            item_dict.set_item("tags", &f.tags)?;
            item_dict.set_item("source_tool", &f.source_tool)?;
            item_dict.set_item("source_module", &f.source_module)?;
            item_dict.set_item("discovered_at", &f.discovered_at)?;
            list.append(item_dict)?;
        }
        dict.set_item("findings", list)?;
        dict.set_item("count", findings.len())?;
        Ok(dict.into())
    }

    /// Serialize the repository to a JSON string.
    fn to_json(&self) -> String {
        let findings = self.findings.read().map(|f| f.to_vec()).unwrap_or_default();
        serde_json::to_string_pretty(&findings).unwrap_or_else(|_| "[]".to_string())
    }
}

/// Assessment represents a point-in-time scan result.
#[pyclass(frozen, name = "Assessment")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentPy {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub created_at: String,
    #[pyo3(get)]
    pub finding_count: u32,
    #[pyo3(get)]
    pub metadata: String,
}

#[pymethods]
impl AssessmentPy {
    #[new]
    #[pyo3(signature = (id, name, target, *, created_at=None, finding_count=None, metadata=None))]
    fn new(
        id: String,
        name: String,
        target: String,
        created_at: Option<String>,
        finding_count: Option<u32>,
        metadata: Option<String>,
    ) -> Self {
        Self {
            id,
            name,
            target,
            created_at: created_at.unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
            finding_count: finding_count.unwrap_or(0),
            metadata: metadata.unwrap_or_else(|| "{}".to_string()),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.id)?;
        dict.set_item("name", &self.name)?;
        dict.set_item("target", &self.target)?;
        dict.set_item("created_at", &self.created_at)?;
        dict.set_item("finding_count", self.finding_count)?;
        dict.set_item("metadata", &self.metadata)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    fn __repr__(&self) -> String {
        format!(
            "Assessment(id={}, name={}, target={}, findings={})",
            self.id, self.name, self.target, self.finding_count
        )
    }
}

/// In-memory assessment repository.
#[pyclass(name = "AssessmentRepository")]
pub struct AssessmentRepositoryPy {
    assessments: std::sync::RwLock<HashMap<String, AssessmentPy>>,
}

#[pymethods]
impl AssessmentRepositoryPy {
    #[new]
    fn new() -> Self {
        Self {
            assessments: std::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Save or overwrite an assessment.
    fn save(&self, assessment: AssessmentPy) -> PyResult<()> {
        let Ok(mut assessments) = self.assessments.write() else {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(
                "Failed to acquire lock",
            ));
        };
        assessments.insert(assessment.id.clone(), assessment);
        Ok(())
    }

    /// Get an assessment by ID.
    fn get(&self, assessment_id: &str) -> Option<AssessmentPy> {
        self.assessments.read().ok()?.get(assessment_id).cloned()
    }

    /// List all assessments.
    fn list(&self) -> Vec<AssessmentPy> {
        self.assessments
            .read()
            .map(|a| a.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Remove an assessment by ID. Returns true if found and removed.
    fn remove(&self, assessment_id: &str) -> bool {
        self.assessments
            .write()
            .ok()
            .and_then(|mut a| a.remove(assessment_id))
            .is_some()
    }

    /// Return the number of assessments.
    fn count(&self) -> usize {
        self.assessments.read().map(|a| a.len()).unwrap_or(0)
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let assessments = self
            .assessments
            .read()
            .map(|a| a.values().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        let dict = PyDict::new_bound(py);
        let list = pyo3::types::PyList::empty_bound(py);
        for a in &assessments {
            let item_dict = PyDict::new_bound(py);
            item_dict.set_item("id", &a.id)?;
            item_dict.set_item("name", &a.name)?;
            item_dict.set_item("target", &a.target)?;
            item_dict.set_item("created_at", &a.created_at)?;
            item_dict.set_item("finding_count", a.finding_count)?;
            item_dict.set_item("metadata", &a.metadata)?;
            list.append(item_dict)?;
        }
        dict.set_item("assessments", list)?;
        dict.set_item("count", assessments.len())?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        let assessments = self
            .assessments
            .read()
            .map(|a| a.values().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        serde_json::to_string_pretty(&assessments).unwrap_or_else(|_| "[]".to_string())
    }
}
