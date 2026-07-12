use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use crate::finding::Severity;
use crate::runtime_sync;

/// Post-exploitation category.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PostexCategoryPy {
    Lotl,
    Persistence,
    LateralMovement,
    CredentialAccess,
}

#[pymethods]
impl PostexCategoryPy {
    fn __repr__(&self) -> String {
        format!("PostexCategory.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl PostexCategoryPy {
    fn as_str(&self) -> &str {
        match self {
            PostexCategoryPy::Lotl => "Lotl",
            PostexCategoryPy::Persistence => "Persistence",
            PostexCategoryPy::LateralMovement => "LateralMovement",
            PostexCategoryPy::CredentialAccess => "CredentialAccess",
        }
    }

    fn from_engine(engine: eggsec::postex::PostexCategory) -> Self {
        match engine {
            eggsec::postex::PostexCategory::Lotl => PostexCategoryPy::Lotl,
            eggsec::postex::PostexCategory::Persistence => PostexCategoryPy::Persistence,
            eggsec::postex::PostexCategory::LateralMovement => PostexCategoryPy::LateralMovement,
            eggsec::postex::PostexCategory::CredentialAccess => {
                PostexCategoryPy::CredentialAccess
            }
        }
    }
}

/// Post-exploitation risk level.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PostexRiskPy {
    Low,
    Medium,
    High,
    Critical,
}

#[pymethods]
impl PostexRiskPy {
    fn __repr__(&self) -> String {
        format!("PostexRisk.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl PostexRiskPy {
    fn as_str(&self) -> &str {
        match self {
            PostexRiskPy::Low => "Low",
            PostexRiskPy::Medium => "Medium",
            PostexRiskPy::High => "High",
            PostexRiskPy::Critical => "Critical",
        }
    }

    fn from_engine(engine: eggsec::postex::PostexRisk) -> Self {
        match engine {
            eggsec::postex::PostexRisk::Low => PostexRiskPy::Low,
            eggsec::postex::PostexRisk::Medium => PostexRiskPy::Medium,
            eggsec::postex::PostexRisk::High => PostexRiskPy::High,
            eggsec::postex::PostexRisk::Critical => PostexRiskPy::Critical,
        }
    }

    #[allow(dead_code)]
    fn to_severity(&self) -> Severity {
        match self {
            PostexRiskPy::Low => Severity::Low,
            PostexRiskPy::Medium => Severity::Medium,
            PostexRiskPy::High => Severity::High,
            PostexRiskPy::Critical => Severity::Critical,
        }
    }
}

/// Post-exploitation profile.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PostexProfilePy {
    Minimal,
    Standard,
    Aggressive,
}

#[pymethods]
impl PostexProfilePy {
    fn __repr__(&self) -> String {
        format!("PostexProfile.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl PostexProfilePy {
    fn as_str(&self) -> &str {
        match self {
            PostexProfilePy::Minimal => "Minimal",
            PostexProfilePy::Standard => "Standard",
            PostexProfilePy::Aggressive => "Aggressive",
        }
    }

    fn from_engine(engine: eggsec::postex::PostexProfile) -> Self {
        match engine {
            eggsec::postex::PostexProfile::Minimal => PostexProfilePy::Minimal,
            eggsec::postex::PostexProfile::Standard => PostexProfilePy::Standard,
            eggsec::postex::PostexProfile::Aggressive => PostexProfilePy::Aggressive,
        }
    }

    fn to_engine(&self) -> eggsec::postex::PostexProfile {
        match self {
            PostexProfilePy::Minimal => eggsec::postex::PostexProfile::Minimal,
            PostexProfilePy::Standard => eggsec::postex::PostexProfile::Standard,
            PostexProfilePy::Aggressive => eggsec::postex::PostexProfile::Aggressive,
        }
    }
}

/// A post-exploitation technique definition.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostexTechniquePy {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub mitre_id: String,
    #[pyo3(get)]
    pub category: PostexCategoryPy,
    #[pyo3(get)]
    pub risk: PostexRiskPy,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub reversible: bool,
}

impl PostexTechniquePy {
    fn from_engine(engine: eggsec::postex::PostexTechnique) -> Self {
        Self {
            id: engine.id,
            name: engine.name,
            mitre_id: engine.mitre_id,
            category: PostexCategoryPy::from_engine(engine.category),
            risk: PostexRiskPy::from_engine(engine.risk),
            description: engine.description,
            reversible: engine.reversible,
        }
    }
}

#[pymethods]
impl PostexTechniquePy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.id)?;
        dict.set_item("name", &self.name)?;
        dict.set_item("mitre_id", &self.mitre_id)?;
        dict.set_item("category", self.category.as_str())?;
        dict.set_item("risk", self.risk.as_str())?;
        dict.set_item("description", &self.description)?;
        dict.set_item("reversible", self.reversible)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!("PostexTechnique(id={}, name={})", self.id, self.name)
    }
}

/// A post-exploitation detection result.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostexDetectionPy {
    #[pyo3(get)]
    pub technique: PostexTechniquePy,
    #[pyo3(get)]
    pub simulated: bool,
    #[pyo3(get)]
    pub confidence: f64,
    #[pyo3(get)]
    pub evidence: String,
    recommendations: Vec<String>,
}

impl PostexDetectionPy {
    fn from_engine(engine: eggsec::postex::PostexDetection) -> Self {
        Self {
            technique: PostexTechniquePy::from_engine(engine.technique),
            simulated: engine.simulated,
            confidence: engine.confidence,
            evidence: engine.evidence,
            recommendations: engine.recommendations,
        }
    }
}

#[pymethods]
impl PostexDetectionPy {
    #[getter]
    fn recommendations(&self) -> Vec<String> {
        self.recommendations.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("technique", self.technique.to_dict(py)?)?;
        dict.set_item("simulated", self.simulated)?;
        dict.set_item("confidence", self.confidence)?;
        dict.set_item("evidence", &self.evidence)?;
        dict.set_item("recommendations", &self.recommendations)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "PostexDetection(technique={}, simulated={})",
            self.technique.name, self.simulated
        )
    }
}

/// Post-exploitation scan summary.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostexSummaryPy {
    #[pyo3(get)]
    pub total: usize,
    #[pyo3(get)]
    pub simulated: usize,
    #[pyo3(get)]
    pub not_simulated: usize,
    categories: std::collections::HashMap<String, usize>,
}

impl PostexSummaryPy {
    fn from_engine(engine: eggsec::postex::PostexSummary) -> Self {
        Self {
            total: engine.total,
            simulated: engine.simulated,
            not_simulated: engine.not_simulated,
            categories: engine.categories,
        }
    }
}

#[pymethods]
impl PostexSummaryPy {
    #[getter]
    fn categories(&self) -> std::collections::HashMap<String, usize> {
        self.categories.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "PostexSummary(total={}, simulated={})",
            self.total, self.simulated
        )
    }
}

/// Full post-exploitation scan report.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostexReportPy {
    #[pyo3(get)]
    pub target: String,
    detections: Vec<PostexDetectionPy>,
    #[pyo3(get)]
    pub summary: PostexSummaryPy,
    #[pyo3(get)]
    pub timestamp: String,
    #[pyo3(get)]
    pub dry_run: bool,
    actions_performed: Vec<String>,
}

impl PostexReportPy {
    fn from_engine(engine: eggsec::postex::PostexReport) -> Self {
        Self {
            target: engine.target,
            detections: engine
                .detections
                .into_iter()
                .map(PostexDetectionPy::from_engine)
                .collect(),
            summary: PostexSummaryPy::from_engine(engine.summary),
            timestamp: engine.timestamp,
            dry_run: engine.dry_run,
            actions_performed: engine.actions_performed,
        }
    }
}

#[pymethods]
impl PostexReportPy {
    #[getter]
    fn detections(&self) -> Vec<PostexDetectionPy> {
        self.detections.clone()
    }

    #[getter]
    fn actions_performed(&self) -> Vec<String> {
        self.actions_performed.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("timestamp", &self.timestamp)?;
        dict.set_item("dry_run", self.dry_run)?;
        dict.set_item("actions_performed", &self.actions_performed)?;

        let det_list = PyList::empty_bound(py);
        for d in &self.detections {
            det_list.append(d.to_dict(py)?)?;
        }
        dict.set_item("detections", det_list)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "PostexReport(target={}, detections={})",
            self.target,
            self.detections.len()
        )
    }
}

/// Configuration for a post-exploitation scan.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostexScanConfigPy {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub profile: PostexProfilePy,
    #[pyo3(get)]
    pub dry_run: bool,
}

#[pymethods]
impl PostexScanConfigPy {
    #[new]
    #[pyo3(signature = (target, profile=PostexProfilePy::Standard, dry_run=true))]
    fn new(target: String, profile: PostexProfilePy, dry_run: bool) -> Self {
        Self {
            target,
            profile,
            dry_run,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "PostexScanConfig(target={}, profile={}, dry_run={})",
            self.target,
            self.profile.as_str(),
            self.dry_run
        )
    }
}

/// Run a post-exploitation simulation scan.
///
/// Simulates post-exploitation techniques against a target to test
/// defensive visibility and response capabilities.
///
/// Args:
///     config: Post-exploitation scan configuration.
///
/// Returns:
///     PostexReportPy: Full post-exploitation report.
///
/// Raises:
///     FeatureUnavailableError: If postex feature is not enabled.
///     ScanError: If the scan fails.
#[pyfunction]
pub fn postex_scan(config: PostexScanConfigPy) -> PyResult<PostexReportPy> {
    Python::with_gil(|py| {
        let result = runtime_sync::block_on(py, async move {
            let scanner =
                eggsec::postex::PostexScanner::new(config.dry_run, config.profile.to_engine());
            scanner
                .scan(&config.target)
                .await
                .map_err(|e| {
                    pyo3::exceptions::PyRuntimeError::new_err(format!(
                        "Postex scan failed: {}",
                        e
                    ))
                })
        })?;

        Ok(PostexReportPy::from_engine(result))
    })
}

/// Run a post-exploitation simulation scan (async).
///
/// Returns a PyFuture that resolves to a PostexReportPy.
#[pyfunction]
pub fn async_postex_scan(
    config: PostexScanConfigPy,
) -> PyResult<crate::runtime_async::PyFuture> {
    crate::runtime_async::spawn_async(async move {
        let scanner =
            eggsec::postex::PostexScanner::new(config.dry_run, config.profile.to_engine());
        let report = scanner
            .scan(&config.target)
            .await
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "Postex scan failed: {}",
                    e
                ))
            })?;
        Ok(PostexReportPy::from_engine(report))
    })
}

/// List all available post-exploitation techniques.
///
/// Returns:
///     List of PostexTechniquePy definitions.
#[pyfunction]
pub fn postex_list_techniques() -> Vec<PostexTechniquePy> {
    let scanner = eggsec::postex::PostexScanner::new(true, eggsec::postex::PostexProfile::Standard);
    scanner
        .techniques()
        .iter()
        .cloned()
        .map(PostexTechniquePy::from_engine)
        .collect()
}
