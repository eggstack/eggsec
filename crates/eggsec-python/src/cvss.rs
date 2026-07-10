use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde::{Deserialize, Serialize};

/// Structured CVSS score data.
#[pyclass(frozen, name = "CvssScore")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CvssScorePy {
    #[pyo3(get)]
    pub version: String,
    #[pyo3(get)]
    pub vector: String,
    #[pyo3(get)]
    pub base_score: f64,
    #[pyo3(get)]
    pub severity: String,
    #[pyo3(get)]
    pub exploitability: Option<f64>,
    #[pyo3(get)]
    pub impact: Option<f64>,
}

#[pymethods]
impl CvssScorePy {
    #[new]
    #[pyo3(signature = (version, vector, base_score, *, severity=None, exploitability=None, impact=None))]
    fn new(
        version: String,
        vector: String,
        base_score: f64,
        severity: Option<String>,
        exploitability: Option<f64>,
        impact: Option<f64>,
    ) -> Self {
        let severity = severity.unwrap_or_else(|| {
            if base_score >= 9.0 {
                "Critical".to_string()
            } else if base_score >= 7.0 {
                "High".to_string()
            } else if base_score >= 4.0 {
                "Medium".to_string()
            } else if base_score > 0.0 {
                "Low".to_string()
            } else {
                "Info".to_string()
            }
        });
        Self {
            version,
            vector,
            base_score,
            severity,
            exploitability,
            impact,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("version", &self.version)?;
        dict.set_item("vector", &self.vector)?;
        dict.set_item("base_score", self.base_score)?;
        dict.set_item("severity", &self.severity)?;
        dict.set_item("exploitability", &self.exploitability)?;
        dict.set_item("impact", &self.impact)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Parse a CVSS vector string and extract version and metadata.
    ///
    /// Accepts vectors like "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H".
    /// The base_score defaults to 0.0; use a calculator for accurate scoring.
    #[staticmethod]
    fn parse(vector: &str) -> PyResult<Self> {
        let version = if vector.starts_with("CVSS:4.0/") {
            "4.0"
        } else if vector.starts_with("CVSS:3.1/") {
            "3.1"
        } else if vector.starts_with("CVSS:3.0/") {
            "3.0"
        } else {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Invalid CVSS vector format. Expected prefix CVSS:3.0/, CVSS:3.1/, or CVSS:4.0/",
            ));
        };

        Ok(Self {
            version: version.to_string(),
            vector: vector.to_string(),
            base_score: 0.0,
            severity: "Info".to_string(),
            exploitability: None,
            impact: None,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "CvssScore(version={}, base_score={}, severity={})",
            self.version, self.base_score, self.severity
        )
    }
}

/// Structured vulnerability record with CVSS and reference data.
#[pyclass(frozen, name = "VulnerabilityRecord")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilityRecordPy {
    #[pyo3(get)]
    pub cve_id: Option<String>,
    #[pyo3(get)]
    pub cwe_id: Option<String>,
    #[pyo3(get)]
    pub title: String,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub cvss: Option<CvssScorePy>,
    #[pyo3(get)]
    pub severity: String,
    #[pyo3(get)]
    pub confidence: String,
    #[pyo3(get)]
    pub affected_assets: Vec<String>,
    #[pyo3(get)]
    pub references: Vec<String>,
    #[pyo3(get)]
    pub published_at: Option<String>,
    #[pyo3(get)]
    pub exploit_available: bool,
    #[pyo3(get)]
    pub risk_accepted: bool,
}

#[pymethods]
impl VulnerabilityRecordPy {
    #[new]
    #[pyo3(signature = (title, description, severity, *, cve_id=None, cwe_id=None, cvss=None, confidence=None, affected_assets=None, references=None, published_at=None, exploit_available=None, risk_accepted=None))]
    fn new(
        title: String,
        description: String,
        severity: String,
        cve_id: Option<String>,
        cwe_id: Option<String>,
        cvss: Option<CvssScorePy>,
        confidence: Option<String>,
        affected_assets: Option<Vec<String>>,
        references: Option<Vec<String>>,
        published_at: Option<String>,
        exploit_available: Option<bool>,
        risk_accepted: Option<bool>,
    ) -> Self {
        Self {
            cve_id,
            cwe_id,
            title,
            description,
            cvss,
            severity,
            confidence: confidence.unwrap_or_else(|| "medium".to_string()),
            affected_assets: affected_assets.unwrap_or_default(),
            references: references.unwrap_or_default(),
            published_at,
            exploit_available: exploit_available.unwrap_or(false),
            risk_accepted: risk_accepted.unwrap_or(false),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("cve_id", &self.cve_id)?;
        dict.set_item("cwe_id", &self.cwe_id)?;
        dict.set_item("title", &self.title)?;
        dict.set_item("description", &self.description)?;
        match &self.cvss {
            Some(cvss) => dict.set_item("cvss", cvss.to_dict(py)?)?,
            None => dict.set_item("cvss", py.None())?,
        }
        dict.set_item("severity", &self.severity)?;
        dict.set_item("confidence", &self.confidence)?;
        dict.set_item("affected_assets", &self.affected_assets)?;
        dict.set_item("references", &self.references)?;
        dict.set_item("published_at", &self.published_at)?;
        dict.set_item("exploit_available", self.exploit_available)?;
        dict.set_item("risk_accepted", self.risk_accepted)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        let cve = self.cve_id.as_deref().unwrap_or("N/A");
        format!(
            "VulnerabilityRecord(cve={}, severity={}, title={})",
            cve, self.severity, self.title
        )
    }
}

/// Tracks remediation status for a finding.
#[pyclass(frozen, name = "RemediationRecord")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationRecordPy {
    #[pyo3(get)]
    pub finding_id: String,
    #[pyo3(get)]
    pub status: String,
    #[pyo3(get)]
    pub assigned_to: Option<String>,
    #[pyo3(get)]
    pub notes: Vec<String>,
    #[pyo3(get)]
    pub created_at: String,
    #[pyo3(get)]
    pub updated_at: String,
    #[pyo3(get)]
    pub estimated_effort: Option<String>,
}

#[pymethods]
impl RemediationRecordPy {
    #[new]
    #[pyo3(signature = (finding_id, *, status=None, assigned_to=None, notes=None, created_at=None, updated_at=None, estimated_effort=None))]
    fn new(
        finding_id: String,
        status: Option<String>,
        assigned_to: Option<String>,
        notes: Option<Vec<String>>,
        created_at: Option<String>,
        updated_at: Option<String>,
        estimated_effort: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let created_at = created_at.unwrap_or_else(|| now.clone());
        let updated_at = updated_at.unwrap_or(now);
        Self {
            finding_id,
            status: status.unwrap_or_else(|| "pending".to_string()),
            assigned_to,
            notes: notes.unwrap_or_default(),
            created_at,
            updated_at,
            estimated_effort,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("finding_id", &self.finding_id)?;
        dict.set_item("status", &self.status)?;
        dict.set_item("assigned_to", &self.assigned_to)?;
        dict.set_item("notes", &self.notes)?;
        dict.set_item("created_at", &self.created_at)?;
        dict.set_item("updated_at", &self.updated_at)?;
        dict.set_item("estimated_effort", &self.estimated_effort)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "RemediationRecord(finding_id={}, status={})",
            self.finding_id, self.status
        )
    }
}
