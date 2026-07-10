use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use super::finding_schema::{ConfidencePy, FindingTypePy, VersionedFindingPy};

/// Compliance framework identifier.
#[pyclass(frozen, name = "ComplianceFramework", eq, eq_int)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComplianceFrameworkPy {
    OwaspTop10,
    NistCsf,
    PciDss,
    Hipaa,
    Soca2,
    Iso27001,
    CisBenchmarks,
    Custom,
}

#[pymethods]
impl ComplianceFrameworkPy {
    #[staticmethod]
    fn from_str(s: &str) -> PyResult<Self> {
        match s.to_lowercase().replace('-', "").replace('_', "").as_str() {
            "owasptop10" | "owasptop102021" => Ok(Self::OwaspTop10),
            "nistcsf" => Ok(Self::NistCsf),
            "pcidss" => Ok(Self::PciDss),
            "hipaa" => Ok(Self::Hipaa),
            "soc2" | "soca2" => Ok(Self::Soca2),
            "iso27001" => Ok(Self::Iso27001),
            "cisbenchmarks" => Ok(Self::CisBenchmarks),
            "custom" => Ok(Self::Custom),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Unknown compliance framework: {s}"
            ))),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::OwaspTop10 => "OWASP Top 10",
            Self::NistCsf => "NIST CSF",
            Self::PciDss => "PCI DSS",
            Self::Hipaa => "HIPAA",
            Self::Soca2 => "SOC 2",
            Self::Iso27001 => "ISO 27001",
            Self::CisBenchmarks => "CIS Benchmarks",
            Self::Custom => "Custom",
        }
    }

    fn __repr__(&self) -> String {
        format!("ComplianceFramework.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

/// A compliance control.
#[pyclass(frozen, name = "ComplianceControl")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceControlPy {
    #[pyo3(get)]
    pub framework: ComplianceFrameworkPy,
    #[pyo3(get)]
    pub control_id: String,
    #[pyo3(get)]
    pub title: String,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub version: String,
    #[pyo3(get)]
    pub parent_control: Option<String>,
}

#[pymethods]
impl ComplianceControlPy {
    #[new]
    #[pyo3(signature = (framework, control_id, title, description, *, version=None, parent_control=None))]
    fn py_new(
        framework: ComplianceFrameworkPy,
        control_id: String,
        title: String,
        description: String,
        version: Option<String>,
        parent_control: Option<String>,
    ) -> Self {
        Self {
            framework,
            control_id,
            title,
            description,
            version: version.unwrap_or_else(|| "1.0".to_string()),
            parent_control,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("framework", self.framework.as_str())?;
        dict.set_item("control_id", &self.control_id)?;
        dict.set_item("title", &self.title)?;
        dict.set_item("description", &self.description)?;
        dict.set_item("version", &self.version)?;
        dict.set_item("parent_control", &self.parent_control)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    fn __repr__(&self) -> String {
        format!(
            "ComplianceControl(framework={}, id={})",
            self.framework.as_str(),
            self.control_id
        )
    }
}

/// Mapping from a finding to a compliance control.
#[pyclass(frozen, name = "ComplianceMapping")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceMappingPy {
    #[pyo3(get)]
    pub finding_id: String,
    #[pyo3(get)]
    pub control: ComplianceControlPy,
    #[pyo3(get)]
    pub mapping_confidence: f64,
    #[pyo3(get)]
    pub rationale: String,
    #[pyo3(get)]
    pub mapped_by: String,
    #[pyo3(get)]
    pub mapped_at: String,
}

#[pymethods]
impl ComplianceMappingPy {
    #[new]
    #[pyo3(signature = (finding_id, control, mapping_confidence, rationale, *, mapped_by=None, mapped_at=None))]
    fn py_new(
        finding_id: String,
        control: ComplianceControlPy,
        mapping_confidence: f64,
        rationale: String,
        mapped_by: Option<String>,
        mapped_at: Option<String>,
    ) -> Self {
        Self {
            finding_id,
            control,
            mapping_confidence,
            rationale,
            mapped_by: mapped_by.unwrap_or_else(|| "automatic".to_string()),
            mapped_at: mapped_at.unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("finding_id", &self.finding_id)?;
        dict.set_item("control", self.control.to_dict(py)?)?;
        dict.set_item("mapping_confidence", self.mapping_confidence)?;
        dict.set_item("rationale", &self.rationale)?;
        dict.set_item("mapped_by", &self.mapped_by)?;
        dict.set_item("mapped_at", &self.mapped_at)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    fn __repr__(&self) -> String {
        format!(
            "ComplianceMapping(finding={}, control={})",
            self.finding_id, self.control.control_id
        )
    }
}

/// Assessment result for a compliance control.
#[pyclass(frozen, name = "ComplianceResult", eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComplianceResultPy {
    Pass,
    Fail,
    Partial,
    NotApplicable,
    NotAssessed,
}

#[pymethods]
impl ComplianceResultPy {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Partial => "partial",
            Self::NotApplicable => "not_applicable",
            Self::NotAssessed => "not_assessed",
        }
    }

    fn __repr__(&self) -> String {
        format!("ComplianceResult.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

/// Assessment of a single control.
#[pyclass(frozen, name = "ControlAssessment")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlAssessmentPy {
    #[pyo3(get)]
    pub control: ComplianceControlPy,
    #[pyo3(get)]
    pub result: ComplianceResultPy,
    #[pyo3(get)]
    pub finding_ids: Vec<String>,
    #[pyo3(get)]
    pub notes: Option<String>,
    #[pyo3(get)]
    pub assessed_at: String,
}

#[pymethods]
impl ControlAssessmentPy {
    #[new]
    #[pyo3(signature = (control, result, finding_ids, *, notes=None, assessed_at=None))]
    fn py_new(
        control: ComplianceControlPy,
        result: ComplianceResultPy,
        finding_ids: Vec<String>,
        notes: Option<String>,
        assessed_at: Option<String>,
    ) -> Self {
        Self {
            control,
            result,
            finding_ids,
            notes,
            assessed_at: assessed_at.unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("control", self.control.to_dict(py)?)?;
        dict.set_item("result", self.result.as_str())?;
        let finding_ids_list = PyList::empty_bound(py);
        for id in &self.finding_ids {
            finding_ids_list.append(id.as_str())?;
        }
        dict.set_item("finding_ids", &finding_ids_list)?;
        dict.set_item("notes", &self.notes)?;
        dict.set_item("assessed_at", &self.assessed_at)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    fn __repr__(&self) -> String {
        format!(
            "ControlAssessment(control={}, result={})",
            self.control.control_id,
            self.result.as_str()
        )
    }
}

/// Compliance report for a framework.
#[pyclass(frozen, name = "ComplianceReport")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReportPy {
    #[pyo3(get)]
    pub framework: ComplianceFrameworkPy,
    #[pyo3(get)]
    pub generated_at: String,
    #[pyo3(get)]
    pub total_controls: u32,
    #[pyo3(get)]
    pub passed: u32,
    #[pyo3(get)]
    pub failed: u32,
    #[pyo3(get)]
    pub partial: u32,
    #[pyo3(get)]
    pub not_applicable: u32,
    #[pyo3(get)]
    pub compliance_percentage: f64,
    #[pyo3(get)]
    pub assessments: Vec<ControlAssessmentPy>,
    #[pyo3(get)]
    pub disclaimer: String,
}

#[pymethods]
impl ComplianceReportPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("framework", self.framework.as_str())?;
        dict.set_item("generated_at", &self.generated_at)?;
        dict.set_item("total_controls", self.total_controls)?;
        dict.set_item("passed", self.passed)?;
        dict.set_item("failed", self.failed)?;
        dict.set_item("partial", self.partial)?;
        dict.set_item("not_applicable", self.not_applicable)?;
        dict.set_item("compliance_percentage", self.compliance_percentage)?;
        let assessments_list = PyList::empty_bound(py);
        for a in &self.assessments {
            assessments_list.append(a.to_dict(py)?)?;
        }
        dict.set_item("assessments", &assessments_list)?;
        dict.set_item("disclaimer", &self.disclaimer)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    fn __repr__(&self) -> String {
        format!(
            "ComplianceReport(framework={}, compliance={:.1}%)",
            self.framework.as_str(),
            self.compliance_percentage
        )
    }
}

const COMPLIANCE_DISCLAIMER: &str = "This report reflects technical control observations only and does not constitute a legal or regulatory compliance determination. Consult qualified professionals for formal compliance assessments.";

/// Severity rank for comparison (higher = more severe).
fn severity_rank(s: &str) -> u8 {
    match s.to_lowercase().as_str() {
        "critical" => 5,
        "high" => 4,
        "medium" => 3,
        "low" => 2,
        "info" | "informational" => 1,
        _ => 0,
    }
}

/// Mapper that correlates findings to compliance controls.
#[pyclass(name = "ComplianceMapper")]
pub struct ComplianceMapperPy {
    controls: std::sync::RwLock<Vec<ComplianceControlPy>>,
    mappings: std::sync::RwLock<Vec<ComplianceMappingPy>>,
}

#[pymethods]
impl ComplianceMapperPy {
    #[new]
    fn new() -> Self {
        Self {
            controls: std::sync::RwLock::new(Vec::new()),
            mappings: std::sync::RwLock::new(Vec::new()),
        }
    }

    /// Register a single compliance control.
    fn register_control(&self, control: ComplianceControlPy) -> PyResult<()> {
        let mut controls = self
            .controls
            .write()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        controls.push(control);
        Ok(())
    }

    /// Register multiple compliance controls.
    fn register_controls(&self, controls: Vec<ComplianceControlPy>) -> PyResult<()> {
        let mut store = self
            .controls
            .write()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        store.extend(controls);
        Ok(())
    }

    /// Manually map a finding to a compliance control.
    fn map_finding(
        &self,
        finding_id: &str,
        control_id: &str,
        framework: ComplianceFrameworkPy,
        confidence: f64,
        rationale: String,
    ) -> PyResult<ComplianceMappingPy> {
        let controls = self
            .controls
            .read()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let control = controls
            .iter()
            .find(|c| c.control_id == control_id && c.framework == framework)
            .ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "No control found with id '{control_id}' for framework {:?}",
                    framework.as_str()
                ))
            })?;
        let mapping = ComplianceMappingPy {
            finding_id: finding_id.to_string(),
            control: control.clone(),
            mapping_confidence: confidence,
            rationale,
            mapped_by: "manual".to_string(),
            mapped_at: chrono::Utc::now().to_rfc3339(),
        };
        let mut mappings = self
            .mappings
            .write()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        mappings.push(mapping.clone());
        Ok(mapping)
    }

    /// Auto-map findings based on CWE, OWASP tags, and finding type.
    fn auto_map_findings(
        &self,
        findings: Vec<VersionedFindingPy>,
    ) -> PyResult<Vec<ComplianceMappingPy>> {
        let controls = self
            .controls
            .read()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let mut new_mappings = Vec::new();

        for finding in &findings {
            let mut best_match: Option<(&ComplianceControlPy, f64, String)> = None;

            for control in controls.iter() {
                let mut confidence = 0.0_f64;
                let mut rationale_parts = Vec::new();

                let ctrl_text = format!(
                    "{} {} {}",
                    control.control_id, control.title, control.description
                )
                .to_lowercase();

                if let Some(ref cwe) = finding.cwe {
                    let cwe_lower = cwe.to_lowercase();
                    if ctrl_text.contains(&cwe_lower) {
                        confidence = 0.9;
                        rationale_parts.push(format!("CWE match: {cwe}"));
                    }
                }

                if let Some(ref cve) = finding.cve {
                    if ctrl_text.contains(&cve.to_lowercase()) {
                        confidence = confidence.max(0.95);
                        rationale_parts.push(format!("CVE match: {cve}"));
                    }
                }

                if let Some(ref owasp) = finding.owasp {
                    let owasp_lower = owasp.to_lowercase();
                    if ctrl_text.contains(&owasp_lower) {
                        confidence = confidence.max(0.85);
                        rationale_parts.push(format!("OWASP match: {owasp}"));
                    }
                }

                for tag in &finding.tags {
                    let tag_lower = tag.to_lowercase();
                    if ctrl_text.contains(&tag_lower) {
                        confidence = confidence.max(0.7);
                        rationale_parts.push(format!("Tag match: {tag}"));
                    }
                }

                let ft_str = finding.finding_type.as_str();
                if ft_str == "vulnerability" && ctrl_text.contains("vulnerability") {
                    confidence = confidence.max(0.6);
                    rationale_parts.push("Vulnerability finding type match".to_string());
                } else if ft_str == "misconfiguration" && ctrl_text.contains("configuration") {
                    confidence = confidence.max(0.7);
                    rationale_parts
                        .push("Misconfiguration type matches configuration control".to_string());
                }

                if confidence > 0.0 {
                    let rationale = if rationale_parts.is_empty() {
                        "Automated mapping based on finding metadata".to_string()
                    } else {
                        rationale_parts.join("; ")
                    };
                    match &best_match {
                        Some((_, best_conf, _)) if confidence <= *best_conf => {}
                        _ => {
                            best_match = Some((control, confidence, rationale));
                        }
                    }
                }
            }

            if let Some((control, confidence, rationale)) = best_match {
                let mapping = ComplianceMappingPy {
                    finding_id: finding.id.clone(),
                    control: control.clone(),
                    mapping_confidence: confidence,
                    rationale,
                    mapped_by: "automatic".to_string(),
                    mapped_at: chrono::Utc::now().to_rfc3339(),
                };
                new_mappings.push(mapping);
            }
        }

        let mut mappings = self
            .mappings
            .write()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        mappings.extend(new_mappings.clone());
        Ok(new_mappings)
    }

    /// Get all mappings for a specific finding.
    fn get_mappings_for_finding(&self, finding_id: &str) -> Vec<ComplianceMappingPy> {
        self.mappings
            .read()
            .map(|mappings| {
                mappings
                    .iter()
                    .filter(|m| m.finding_id == finding_id)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all mappings for a specific control.
    fn get_mappings_for_control(&self, control_id: &str) -> Vec<ComplianceMappingPy> {
        self.mappings
            .read()
            .map(|mappings| {
                mappings
                    .iter()
                    .filter(|m| m.control.control_id == control_id)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Assess a single control against a set of findings.
    fn assess_control(
        &self,
        control_id: &str,
        findings: Vec<VersionedFindingPy>,
    ) -> PyResult<ControlAssessmentPy> {
        let controls = self
            .controls
            .read()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let control = controls
            .iter()
            .find(|c| c.control_id == control_id)
            .ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "No control found with id '{control_id}'"
                ))
            })?;

        let mapped_finding_ids: Vec<String> = {
            let mappings = self
                .mappings
                .read()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            mappings
                .iter()
                .filter(|m| m.control.control_id == control_id)
                .map(|m| m.finding_id.clone())
                .collect()
        };

        let matched_findings: Vec<&VersionedFindingPy> = findings
            .iter()
            .filter(|f| mapped_finding_ids.contains(&f.id))
            .collect();

        let result = if matched_findings.is_empty() {
            ComplianceResultPy::NotAssessed
        } else {
            let has_critical = matched_findings
                .iter()
                .any(|f| f.severity.to_lowercase() == "critical");
            let has_high = matched_findings
                .iter()
                .any(|f| f.severity.to_lowercase() == "high");

            if has_critical {
                ComplianceResultPy::Fail
            } else if has_high {
                ComplianceResultPy::Partial
            } else {
                ComplianceResultPy::Pass
            }
        };

        Ok(ControlAssessmentPy {
            control: control.clone(),
            result,
            finding_ids: mapped_finding_ids,
            notes: None,
            assessed_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Generate a full compliance report for a framework.
    fn generate_report(
        &self,
        framework: ComplianceFrameworkPy,
        findings: Vec<VersionedFindingPy>,
    ) -> PyResult<ComplianceReportPy> {
        let framework_control_ids: Vec<String> = {
            let controls = self
                .controls
                .read()
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            controls
                .iter()
                .filter(|c| c.framework == framework)
                .map(|c| c.control_id.clone())
                .collect()
        };

        let mut assessments = Vec::new();
        let mut passed = 0u32;
        let mut failed = 0u32;
        let mut partial = 0u32;
        let mut not_applicable = 0u32;

        for control_id in &framework_control_ids {
            let assessment = self.assess_control(control_id, findings.clone())?;
            match assessment.result {
                ComplianceResultPy::Pass => passed += 1,
                ComplianceResultPy::Fail => failed += 1,
                ComplianceResultPy::Partial => partial += 1,
                ComplianceResultPy::NotApplicable => not_applicable += 1,
                ComplianceResultPy::NotAssessed => {}
            }
            assessments.push(assessment);
        }

        let total = framework_control_ids.len() as u32;
        let assessed = passed + failed + partial;
        let compliance_percentage = if assessed > 0 {
            (passed as f64 / assessed as f64) * 100.0
        } else {
            0.0
        };

        Ok(ComplianceReportPy {
            framework,
            generated_at: chrono::Utc::now().to_rfc3339(),
            total_controls: total,
            passed,
            failed,
            partial,
            not_applicable,
            compliance_percentage,
            assessments,
            disclaimer: COMPLIANCE_DISCLAIMER.to_string(),
        })
    }

    /// Return all registered controls.
    fn controls(&self) -> Vec<ComplianceControlPy> {
        self.controls.read().map(|r| r.clone()).unwrap_or_default()
    }

    /// Return all recorded mappings.
    fn mappings(&self) -> Vec<ComplianceMappingPy> {
        self.mappings.read().map(|r| r.clone()).unwrap_or_default()
    }
}
