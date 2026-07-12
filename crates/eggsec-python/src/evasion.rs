use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use crate::finding::Severity;
use crate::runtime_sync;

/// Evasion target type.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvasionTargetTypePy {
    Process,
    File,
    Network,
    Registry,
    Memory,
}

#[pymethods]
impl EvasionTargetTypePy {
    fn __repr__(&self) -> String {
        format!("EvasionTargetType.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl EvasionTargetTypePy {
    fn as_str(&self) -> &str {
        match self {
            EvasionTargetTypePy::Process => "Process",
            EvasionTargetTypePy::File => "File",
            EvasionTargetTypePy::Network => "Network",
            EvasionTargetTypePy::Registry => "Registry",
            EvasionTargetTypePy::Memory => "Memory",
        }
    }

    fn from_engine(engine: eggsec::evasion::EvasionTargetType) -> Self {
        match engine {
            eggsec::evasion::EvasionTargetType::Process => EvasionTargetTypePy::Process,
            eggsec::evasion::EvasionTargetType::File => EvasionTargetTypePy::File,
            eggsec::evasion::EvasionTargetType::Network => EvasionTargetTypePy::Network,
            eggsec::evasion::EvasionTargetType::Registry => EvasionTargetTypePy::Registry,
            eggsec::evasion::EvasionTargetType::Memory => EvasionTargetTypePy::Memory,
        }
    }
}

/// Evasion technique category.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvasionCategoryPy {
    Syscall,
    HookBypass,
    Obfuscation,
    Injection,
    AntiAnalysis,
    TrafficObfuscation,
}

#[pymethods]
impl EvasionCategoryPy {
    fn __repr__(&self) -> String {
        format!("EvasionCategory.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl EvasionCategoryPy {
    fn as_str(&self) -> &str {
        match self {
            EvasionCategoryPy::Syscall => "Syscall",
            EvasionCategoryPy::HookBypass => "HookBypass",
            EvasionCategoryPy::Obfuscation => "Obfuscation",
            EvasionCategoryPy::Injection => "Injection",
            EvasionCategoryPy::AntiAnalysis => "AntiAnalysis",
            EvasionCategoryPy::TrafficObfuscation => "TrafficObfuscation",
        }
    }

    fn from_engine(engine: eggsec::evasion::EvasionCategory) -> Self {
        match engine {
            eggsec::evasion::EvasionCategory::Syscall => EvasionCategoryPy::Syscall,
            eggsec::evasion::EvasionCategory::HookBypass => EvasionCategoryPy::HookBypass,
            eggsec::evasion::EvasionCategory::Obfuscation => EvasionCategoryPy::Obfuscation,
            eggsec::evasion::EvasionCategory::Injection => EvasionCategoryPy::Injection,
            eggsec::evasion::EvasionCategory::AntiAnalysis => EvasionCategoryPy::AntiAnalysis,
            eggsec::evasion::EvasionCategory::TrafficObfuscation => {
                EvasionCategoryPy::TrafficObfuscation
            }
        }
    }
}

/// Evasion risk level.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvasionRiskPy {
    Low,
    Medium,
    High,
    Critical,
}

#[pymethods]
impl EvasionRiskPy {
    fn __repr__(&self) -> String {
        format!("EvasionRisk.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl EvasionRiskPy {
    fn as_str(&self) -> &str {
        match self {
            EvasionRiskPy::Low => "Low",
            EvasionRiskPy::Medium => "Medium",
            EvasionRiskPy::High => "High",
            EvasionRiskPy::Critical => "Critical",
        }
    }

    fn from_engine(engine: eggsec::evasion::EvasionRisk) -> Self {
        match engine {
            eggsec::evasion::EvasionRisk::Low => EvasionRiskPy::Low,
            eggsec::evasion::EvasionRisk::Medium => EvasionRiskPy::Medium,
            eggsec::evasion::EvasionRisk::High => EvasionRiskPy::High,
            eggsec::evasion::EvasionRisk::Critical => EvasionRiskPy::Critical,
        }
    }

    #[allow(dead_code)]
    fn to_severity(&self) -> Severity {
        match self {
            EvasionRiskPy::Low => Severity::Low,
            EvasionRiskPy::Medium => Severity::Medium,
            EvasionRiskPy::High => Severity::High,
            EvasionRiskPy::Critical => Severity::Critical,
        }
    }
}

/// An evasion technique definition.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvasionTechniquePy {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub mitre_id: Option<String>,
    #[pyo3(get)]
    pub category: EvasionCategoryPy,
    #[pyo3(get)]
    pub risk_level: EvasionRiskPy,
    #[pyo3(get)]
    pub description: String,
}

impl EvasionTechniquePy {
    fn from_engine(engine: eggsec::evasion::EvasionTechnique) -> Self {
        Self {
            id: engine.id,
            name: engine.name,
            mitre_id: engine.mitre_id,
            category: EvasionCategoryPy::from_engine(engine.category),
            risk_level: EvasionRiskPy::from_engine(engine.risk_level),
            description: engine.description,
        }
    }
}

#[pymethods]
impl EvasionTechniquePy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.id)?;
        dict.set_item("name", &self.name)?;
        dict.set_item("mitre_id", &self.mitre_id)?;
        dict.set_item("category", self.category.as_str())?;
        dict.set_item("risk_level", self.risk_level.as_str())?;
        dict.set_item("description", &self.description)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!("EvasionTechnique(id={}, name={})", self.id, self.name)
    }
}

/// An evasion detection result.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvasionDetectionPy {
    #[pyo3(get)]
    pub technique: EvasionTechniquePy,
    #[pyo3(get)]
    pub detected: bool,
    #[pyo3(get)]
    pub confidence: f64,
    #[pyo3(get)]
    pub evidence: Option<String>,
    recommendations: Vec<String>,
}

impl EvasionDetectionPy {
    fn from_engine(engine: eggsec::evasion::EvasionDetection) -> Self {
        Self {
            technique: EvasionTechniquePy::from_engine(engine.technique),
            detected: engine.detected,
            confidence: engine.confidence,
            evidence: engine.evidence,
            recommendations: engine.recommendations,
        }
    }
}

#[pymethods]
impl EvasionDetectionPy {
    #[getter]
    fn recommendations(&self) -> Vec<String> {
        self.recommendations.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("technique", self.technique.to_dict(py)?)?;
        dict.set_item("detected", self.detected)?;
        dict.set_item("confidence", self.confidence)?;
        dict.set_item("evidence", &self.evidence)?;
        dict.set_item("recommendations", &self.recommendations)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "EvasionDetection(technique={}, detected={})",
            self.technique.name, self.detected
        )
    }
}

/// Evasion scan summary.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvasionSummaryPy {
    #[pyo3(get)]
    pub total_techniques: usize,
    #[pyo3(get)]
    pub detected: usize,
    #[pyo3(get)]
    pub not_detected: usize,
    #[pyo3(get)]
    pub detection_rate: f64,
}

impl EvasionSummaryPy {
    fn from_engine(engine: eggsec::evasion::EvasionSummary) -> Self {
        Self {
            total_techniques: engine.total_techniques,
            detected: engine.detected,
            not_detected: engine.not_detected,
            detection_rate: engine.detection_rate,
        }
    }
}

#[pymethods]
impl EvasionSummaryPy {
    fn __repr__(&self) -> String {
        format!(
            "EvasionSummary(total={}, detected={}, rate={:.1}%)",
            self.total_techniques,
            self.detected,
            self.detection_rate * 100.0
        )
    }
}

/// Full evasion scan report.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvasionReportPy {
    #[pyo3(get)]
    pub target: String,
    detections: Vec<EvasionDetectionPy>,
    #[pyo3(get)]
    pub summary: EvasionSummaryPy,
    #[pyo3(get)]
    pub timestamp: String,
    #[pyo3(get)]
    pub dry_run: bool,
}

impl EvasionReportPy {
    fn from_engine(engine: eggsec::evasion::EvasionReport) -> Self {
        Self {
            target: engine.target,
            detections: engine
                .detections
                .into_iter()
                .map(EvasionDetectionPy::from_engine)
                .collect(),
            summary: EvasionSummaryPy::from_engine(engine.summary),
            timestamp: engine.timestamp,
            dry_run: engine.dry_run,
        }
    }
}

#[pymethods]
impl EvasionReportPy {
    #[getter]
    fn detections(&self) -> Vec<EvasionDetectionPy> {
        self.detections.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("timestamp", &self.timestamp)?;
        dict.set_item("dry_run", self.dry_run)?;

        let det_list = PyList::empty_bound(py);
        for d in &self.detections {
            det_list.append(d.to_dict(py)?)?;
        }
        dict.set_item("detections", det_list)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "EvasionReport(target={}, detections={})",
            self.target,
            self.detections.len()
        )
    }
}

/// Configuration for an evasion scan.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvasionScanConfigPy {
    #[pyo3(get)]
    pub target_type: EvasionTargetTypePy,
    #[pyo3(get)]
    pub path: Option<String>,
    #[pyo3(get)]
    pub pid: Option<u32>,
    #[pyo3(get)]
    pub dry_run: bool,
}

#[pymethods]
impl EvasionScanConfigPy {
    #[new]
    #[pyo3(signature = (target_type, path=None, pid=None, dry_run=true))]
    fn new(
        target_type: EvasionTargetTypePy,
        path: Option<String>,
        pid: Option<u32>,
        dry_run: bool,
    ) -> Self {
        Self {
            target_type,
            path,
            pid,
            dry_run,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "EvasionScanConfig(type={}, dry_run={})",
            self.target_type.as_str(),
            self.dry_run
        )
    }
}

/// Run an evasion technique scan.
///
/// Scans a target for common evasion techniques (syscall hooking, process
/// injection, traffic obfuscation, etc.) using MITRE ATT&CK mapping.
///
/// Args:
///     config: Evasion scan configuration.
///
/// Returns:
///     EvasionReportPy: Full evasion report with detections.
///
/// Raises:
///     FeatureUnavailableError: If evasion feature is not enabled.
///     ScanError: If the scan fails.
#[pyfunction]
pub fn evasion_scan(config: EvasionScanConfigPy) -> PyResult<EvasionReportPy> {
    Python::with_gil(|py| {
        let result = runtime_sync::block_on(py, async move {
            let scanner = eggsec::evasion::EvasionScanner::new(config.dry_run);
            let target = eggsec::evasion::EvasionTarget {
                target_type: match config.target_type {
                    EvasionTargetTypePy::Process => eggsec::evasion::EvasionTargetType::Process,
                    EvasionTargetTypePy::File => eggsec::evasion::EvasionTargetType::File,
                    EvasionTargetTypePy::Network => eggsec::evasion::EvasionTargetType::Network,
                    EvasionTargetTypePy::Registry => eggsec::evasion::EvasionTargetType::Registry,
                    EvasionTargetTypePy::Memory => eggsec::evasion::EvasionTargetType::Memory,
                },
                path: config.path,
                pid: config.pid,
            };
            scanner
                .scan(&target)
                .await
                .map_err(|e| {
                    pyo3::exceptions::PyRuntimeError::new_err(format!(
                        "Evasion scan failed: {}",
                        e
                    ))
                })
        })?;

        Ok(EvasionReportPy::from_engine(result))
    })
}

/// Run an evasion technique scan (async).
///
/// Returns a PyFuture that resolves to an EvasionReportPy.
#[pyfunction]
pub fn async_evasion_scan(
    config: EvasionScanConfigPy,
) -> PyResult<crate::runtime_async::PyFuture> {
    crate::runtime_async::spawn_async(async move {
        let scanner = eggsec::evasion::EvasionScanner::new(config.dry_run);
        let target = eggsec::evasion::EvasionTarget {
            target_type: match config.target_type {
                EvasionTargetTypePy::Process => eggsec::evasion::EvasionTargetType::Process,
                EvasionTargetTypePy::File => eggsec::evasion::EvasionTargetType::File,
                EvasionTargetTypePy::Network => eggsec::evasion::EvasionTargetType::Network,
                EvasionTargetTypePy::Registry => eggsec::evasion::EvasionTargetType::Registry,
                EvasionTargetTypePy::Memory => eggsec::evasion::EvasionTargetType::Memory,
            },
            path: config.path,
            pid: config.pid,
        };
        let report = scanner
            .scan(&target)
            .await
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "Evasion scan failed: {}",
                    e
                ))
            })?;
        Ok(EvasionReportPy::from_engine(report))
    })
}

/// List all available evasion techniques.
///
/// Returns:
///     List of EvasionTechniquePy definitions.
#[pyfunction]
pub fn evasion_list_techniques() -> Vec<EvasionTechniquePy> {
    let scanner = eggsec::evasion::EvasionScanner::new(true);
    scanner
        .techniques()
        .iter()
        .cloned()
        .map(EvasionTechniquePy::from_engine)
        .collect()
}
