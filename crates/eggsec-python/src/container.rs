use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use crate::error::EggsecResultExt;
use crate::finding::Severity;
use crate::runtime_async;
use crate::runtime_sync;

/// Container scan type.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContainerScanTypePy {
    Docker,
    Kubernetes,
    EscapeDetection,
    CisBenchmark,
    Full,
}

#[pymethods]
impl ContainerScanTypePy {
    fn __repr__(&self) -> String {
        format!("ContainerScanType.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl ContainerScanTypePy {
    fn as_str(&self) -> &str {
        match self {
            ContainerScanTypePy::Docker => "Docker",
            ContainerScanTypePy::Kubernetes => "Kubernetes",
            ContainerScanTypePy::EscapeDetection => "EscapeDetection",
            ContainerScanTypePy::CisBenchmark => "CisBenchmark",
            ContainerScanTypePy::Full => "Full",
        }
    }

    fn from_engine(engine: eggsec::container::ContainerScanType) -> Self {
        match engine {
            eggsec::container::ContainerScanType::Docker => ContainerScanTypePy::Docker,
            eggsec::container::ContainerScanType::Kubernetes => ContainerScanTypePy::Kubernetes,
            eggsec::container::ContainerScanType::EscapeDetection => {
                ContainerScanTypePy::EscapeDetection
            }
            eggsec::container::ContainerScanType::CisBenchmark => ContainerScanTypePy::CisBenchmark,
            eggsec::container::ContainerScanType::Full => ContainerScanTypePy::Full,
        }
    }
}

/// Escape risk level.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EscapeRiskLevelPy {
    None,
    Low,
    Medium,
    High,
    Critical,
}

#[pymethods]
impl EscapeRiskLevelPy {
    fn __repr__(&self) -> String {
        format!("EscapeRiskLevel.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl EscapeRiskLevelPy {
    fn as_str(&self) -> &str {
        match self {
            EscapeRiskLevelPy::None => "None",
            EscapeRiskLevelPy::Low => "Low",
            EscapeRiskLevelPy::Medium => "Medium",
            EscapeRiskLevelPy::High => "High",
            EscapeRiskLevelPy::Critical => "Critical",
        }
    }

    fn from_engine(engine: eggsec::container::escape::EscapeRiskLevel) -> Self {
        match engine {
            eggsec::container::escape::EscapeRiskLevel::None => EscapeRiskLevelPy::None,
            eggsec::container::escape::EscapeRiskLevel::Low => EscapeRiskLevelPy::Low,
            eggsec::container::escape::EscapeRiskLevel::Medium => EscapeRiskLevelPy::Medium,
            eggsec::container::escape::EscapeRiskLevel::High => EscapeRiskLevelPy::High,
            eggsec::container::escape::EscapeRiskLevel::Critical => EscapeRiskLevelPy::Critical,
        }
    }
}

/// CIS benchmark check status.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CisCheckStatusPy {
    Pass,
    Fail,
    Warn,
}

#[pymethods]
impl CisCheckStatusPy {
    fn __repr__(&self) -> String {
        format!("CisCheckStatus.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl CisCheckStatusPy {
    fn as_str(&self) -> &str {
        match self {
            CisCheckStatusPy::Pass => "Pass",
            CisCheckStatusPy::Fail => "Fail",
            CisCheckStatusPy::Warn => "Warn",
        }
    }

    fn from_engine(engine: eggsec::container::cis::CisCheckStatus) -> Self {
        match engine {
            eggsec::container::cis::CisCheckStatus::Pass => CisCheckStatusPy::Pass,
            eggsec::container::cis::CisCheckStatus::Fail => CisCheckStatusPy::Fail,
            eggsec::container::cis::CisCheckStatus::Warn => CisCheckStatusPy::Warn,
        }
    }
}

/// A single Docker image layer.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageLayerPy {
    #[pyo3(get)]
    pub layer_id: String,
    #[pyo3(get)]
    pub instruction: String,
    #[pyo3(get)]
    pub size_bytes: Option<u64>,
}

impl ImageLayerPy {
    fn from_engine(engine: eggsec::container::docker::ImageLayer) -> Self {
        Self {
            layer_id: engine.layer_id,
            instruction: engine.instruction,
            size_bytes: engine.size_bytes,
        }
    }
}

#[pymethods]
impl ImageLayerPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("layer_id", &self.layer_id)?;
        dict.set_item("instruction", &self.instruction)?;
        dict.set_item("size_bytes", self.size_bytes)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ImageLayer(instruction={}, size={:?})",
            self.instruction, self.size_bytes
        )
    }

    fn __str__(&self) -> String {
        match self.size_bytes {
            Some(size) => format!("{} ({} bytes)", self.instruction, size),
            None => self.instruction.clone(),
        }
    }
}

/// A Docker image misconfiguration.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerMisconfigPy {
    #[pyo3(get)]
    pub check: String,
    #[pyo3(get)]
    pub severity: Severity,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub recommendation: String,
}

impl DockerMisconfigPy {
    fn from_engine(engine: eggsec::container::docker::DockerMisconfiguration) -> Self {
        Self {
            check: engine.check,
            severity: Severity::from_engine(engine.severity),
            description: engine.description,
            recommendation: engine.recommendation,
        }
    }
}

#[pymethods]
impl DockerMisconfigPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("check", &self.check)?;
        dict.set_item("severity", self.severity.as_str())?;
        dict.set_item("description", &self.description)?;
        dict.set_item("recommendation", &self.recommendation)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "DockerMisconfig(check={}, severity={})",
            self.check,
            self.severity.as_str()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "[{}] {} - {}",
            self.severity.as_str(),
            self.check,
            self.description
        )
    }
}

/// Type alias for the Docker image report used by the operation registry.
pub type DockerImageReportPy = DockerScanResultPy;

/// Docker image scan result.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerScanResultPy {
    #[pyo3(get)]
    pub image_name: String,
    #[pyo3(get)]
    pub base_image: Option<String>,
    layers: Vec<ImageLayerPy>,
    misconfigurations: Vec<DockerMisconfigPy>,
    #[pyo3(get)]
    pub exposed_ports: Vec<u16>,
    #[pyo3(get)]
    pub running_as_root: bool,
    #[pyo3(get)]
    pub has_healthcheck: bool,
}

impl DockerScanResultPy {
    fn from_engine(engine: eggsec::container::docker::DockerScanResult) -> Self {
        Self {
            image_name: engine.image_name,
            base_image: engine.base_image,
            layers: engine
                .layers
                .into_iter()
                .map(ImageLayerPy::from_engine)
                .collect(),
            misconfigurations: engine
                .misconfigurations
                .into_iter()
                .map(DockerMisconfigPy::from_engine)
                .collect(),
            exposed_ports: engine.exposed_ports,
            running_as_root: engine.running_as_root,
            has_healthcheck: engine.has_healthcheck,
        }
    }
}

#[pymethods]
impl DockerScanResultPy {
    #[getter]
    fn layers(&self) -> Vec<ImageLayerPy> {
        self.layers.clone()
    }

    #[getter]
    fn misconfigurations(&self) -> Vec<DockerMisconfigPy> {
        self.misconfigurations.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("image_name", &self.image_name)?;
        dict.set_item("base_image", &self.base_image)?;
        let layers_list = PyList::empty_bound(py);
        for l in &self.layers {
            layers_list.append(l.to_dict(py)?)?;
        }
        dict.set_item("layers", layers_list)?;
        let misconfigs_list = PyList::empty_bound(py);
        for m in &self.misconfigurations {
            misconfigs_list.append(m.to_dict(py)?)?;
        }
        dict.set_item("misconfigurations", misconfigs_list)?;
        dict.set_item("exposed_ports", &self.exposed_ports)?;
        dict.set_item("running_as_root", self.running_as_root)?;
        dict.set_item("has_healthcheck", self.has_healthcheck)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "DockerScanResult(image={}, root={}, healthcheck={}, layers={}, misconfigs={})",
            self.image_name,
            self.running_as_root,
            self.has_healthcheck,
            self.layers.len(),
            self.misconfigurations.len()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Docker scan of '{}': {} layers, {} misconfigurations, root={}",
            self.image_name,
            self.layers.len(),
            self.misconfigurations.len(),
            self.running_as_root
        )
    }
}

/// Kubernetes cluster information.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterInfoPy {
    #[pyo3(get)]
    pub server_version: Option<String>,
    #[pyo3(get)]
    pub node_count: Option<usize>,
    #[pyo3(get)]
    pub namespace_count: Option<usize>,
}

impl ClusterInfoPy {
    fn from_engine(engine: eggsec::container::kubernetes::ClusterInfo) -> Self {
        Self {
            server_version: engine.server_version,
            node_count: engine.node_count,
            namespace_count: engine.namespace_count,
        }
    }
}

#[pymethods]
impl ClusterInfoPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("server_version", &self.server_version)?;
        dict.set_item("node_count", self.node_count)?;
        dict.set_item("namespace_count", self.namespace_count)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ClusterInfo(version={:?}, nodes={:?}, namespaces={:?})",
            self.server_version, self.node_count, self.namespace_count
        )
    }

    fn __str__(&self) -> String {
        format!(
            "K8s cluster v{:?}, {} nodes, {} namespaces",
            self.server_version,
            self.node_count.unwrap_or(0),
            self.namespace_count.unwrap_or(0)
        )
    }
}

/// A Kubernetes security finding.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sFindingPy {
    #[pyo3(get)]
    pub resource_type: String,
    #[pyo3(get)]
    pub resource_name: String,
    #[pyo3(get)]
    pub severity: Severity,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub recommendation: String,
}

impl K8sFindingPy {
    fn from_engine(engine: eggsec::container::kubernetes::K8sFinding) -> Self {
        Self {
            resource_type: engine.resource_type,
            resource_name: engine.resource_name,
            severity: Severity::from_engine(engine.severity),
            description: engine.description,
            recommendation: engine.recommendation,
        }
    }
}

#[pymethods]
impl K8sFindingPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("resource_type", &self.resource_type)?;
        dict.set_item("resource_name", &self.resource_name)?;
        dict.set_item("severity", self.severity.as_str())?;
        dict.set_item("description", &self.description)?;
        dict.set_item("recommendation", &self.recommendation)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "K8sFinding(type={}, name={}, severity={})",
            self.resource_type,
            self.resource_name,
            self.severity.as_str()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "[{}] {}/{} - {}",
            self.severity.as_str(),
            self.resource_type,
            self.resource_name,
            self.description
        )
    }
}

/// Type alias for the Kubernetes report used by the operation registry.
pub type KubernetesReportPy = KubernetesScanResultPy;

/// Kubernetes scan result.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KubernetesScanResultPy {
    cluster_info: Option<ClusterInfoPy>,
    rbac_issues: Vec<K8sFindingPy>,
    network_policy_issues: Vec<K8sFindingPy>,
    pod_security_issues: Vec<K8sFindingPy>,
    secret_exposure: Vec<K8sFindingPy>,
}

impl KubernetesScanResultPy {
    fn from_engine(engine: eggsec::container::kubernetes::KubernetesScanResult) -> Self {
        Self {
            cluster_info: engine.cluster_info.map(ClusterInfoPy::from_engine),
            rbac_issues: engine
                .rbac_issues
                .into_iter()
                .map(K8sFindingPy::from_engine)
                .collect(),
            network_policy_issues: engine
                .network_policy_issues
                .into_iter()
                .map(K8sFindingPy::from_engine)
                .collect(),
            pod_security_issues: engine
                .pod_security_issues
                .into_iter()
                .map(K8sFindingPy::from_engine)
                .collect(),
            secret_exposure: engine
                .secret_exposure
                .into_iter()
                .map(K8sFindingPy::from_engine)
                .collect(),
        }
    }
}

#[pymethods]
impl KubernetesScanResultPy {
    #[getter]
    fn cluster_info(&self) -> Option<ClusterInfoPy> {
        self.cluster_info.clone()
    }

    #[getter]
    fn rbac_issues(&self) -> Vec<K8sFindingPy> {
        self.rbac_issues.clone()
    }

    #[getter]
    fn network_policy_issues(&self) -> Vec<K8sFindingPy> {
        self.network_policy_issues.clone()
    }

    #[getter]
    fn pod_security_issues(&self) -> Vec<K8sFindingPy> {
        self.pod_security_issues.clone()
    }

    #[getter]
    fn secret_exposure(&self) -> Vec<K8sFindingPy> {
        self.secret_exposure.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        if let Some(ref info) = self.cluster_info {
            dict.set_item("cluster_info", info.to_dict(py)?)?;
        } else {
            dict.set_item("cluster_info", py.None())?;
        }
        let rbac_list = PyList::empty_bound(py);
        for f in &self.rbac_issues {
            rbac_list.append(f.to_dict(py)?)?;
        }
        dict.set_item("rbac_issues", rbac_list)?;
        let netpol_list = PyList::empty_bound(py);
        for f in &self.network_policy_issues {
            netpol_list.append(f.to_dict(py)?)?;
        }
        dict.set_item("network_policy_issues", netpol_list)?;
        let pod_list = PyList::empty_bound(py);
        for f in &self.pod_security_issues {
            pod_list.append(f.to_dict(py)?)?;
        }
        dict.set_item("pod_security_issues", pod_list)?;
        let secret_list = PyList::empty_bound(py);
        for f in &self.secret_exposure {
            secret_list.append(f.to_dict(py)?)?;
        }
        dict.set_item("secret_exposure", secret_list)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "KubernetesScanResult(rbac={}, netpol={}, pod={}, secrets={})",
            self.rbac_issues.len(),
            self.network_policy_issues.len(),
            self.pod_security_issues.len(),
            self.secret_exposure.len()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "K8s scan: {} RBAC, {} network policy, {} pod security, {} secret exposure findings",
            self.rbac_issues.len(),
            self.network_policy_issues.len(),
            self.pod_security_issues.len(),
            self.secret_exposure.len()
        )
    }
}

/// A container escape risk.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscapeRiskPy {
    #[pyo3(get)]
    pub risk_type: String,
    #[pyo3(get)]
    pub severity: Severity,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub recommendation: String,
}

impl EscapeRiskPy {
    fn from_engine(engine: eggsec::container::escape::EscapeRisk) -> Self {
        Self {
            risk_type: engine.risk_type,
            severity: Severity::from_engine(engine.severity),
            description: engine.description,
            recommendation: engine.recommendation,
        }
    }
}

#[pymethods]
impl EscapeRiskPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("risk_type", &self.risk_type)?;
        dict.set_item("severity", self.severity.as_str())?;
        dict.set_item("description", &self.description)?;
        dict.set_item("recommendation", &self.recommendation)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "EscapeRisk(type={}, severity={})",
            self.risk_type,
            self.severity.as_str()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "[{}] {} - {}",
            self.severity.as_str(),
            self.risk_type,
            self.description
        )
    }
}

/// Container escape detection result.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscapeDetectionResultPy {
    #[pyo3(get)]
    pub target: String,
    escape_risks: Vec<EscapeRiskPy>,
    #[pyo3(get)]
    pub risk_level: EscapeRiskLevelPy,
}

impl EscapeDetectionResultPy {
    fn from_engine(engine: eggsec::container::escape::EscapeDetectionResult) -> Self {
        Self {
            target: engine.target,
            escape_risks: engine
                .escape_risks
                .into_iter()
                .map(EscapeRiskPy::from_engine)
                .collect(),
            risk_level: EscapeRiskLevelPy::from_engine(engine.risk_level),
        }
    }
}

#[pymethods]
impl EscapeDetectionResultPy {
    #[getter]
    fn escape_risks(&self) -> Vec<EscapeRiskPy> {
        self.escape_risks.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        let risks_list = PyList::empty_bound(py);
        for r in &self.escape_risks {
            risks_list.append(r.to_dict(py)?)?;
        }
        dict.set_item("escape_risks", risks_list)?;
        dict.set_item("risk_level", self.risk_level.as_str())?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "EscapeDetectionResult(target={}, level={}, risks={})",
            self.target,
            self.risk_level.as_str(),
            self.escape_risks.len()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Escape analysis of '{}': {} risks, level={}",
            self.target,
            self.escape_risks.len(),
            self.risk_level.as_str()
        )
    }
}

/// A single CIS benchmark check.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CisCheckPy {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub severity: Severity,
    #[pyo3(get)]
    pub status: CisCheckStatusPy,
    #[pyo3(get)]
    pub recommendation: String,
}

impl CisCheckPy {
    fn from_engine(engine: eggsec::container::cis::CisCheck) -> Self {
        Self {
            id: engine.id,
            description: engine.description,
            severity: Severity::from_engine(engine.severity),
            status: CisCheckStatusPy::from_engine(engine.status),
            recommendation: engine.recommendation,
        }
    }
}

#[pymethods]
impl CisCheckPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.id)?;
        dict.set_item("description", &self.description)?;
        dict.set_item("severity", self.severity.as_str())?;
        dict.set_item("status", self.status.as_str())?;
        dict.set_item("recommendation", &self.recommendation)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "CisCheck(id={}, status={}, severity={})",
            self.id,
            self.status.as_str(),
            self.severity.as_str()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "[{}] {} {} - {}",
            self.status.as_str(),
            self.id,
            self.description,
            self.recommendation
        )
    }
}

/// CIS Docker benchmark result.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CisBenchmarkResultPy {
    #[pyo3(get)]
    pub benchmark_version: String,
    #[pyo3(get)]
    pub total_checks: usize,
    #[pyo3(get)]
    pub passed: usize,
    #[pyo3(get)]
    pub failed: usize,
    #[pyo3(get)]
    pub warnings: usize,
    checks: Vec<CisCheckPy>,
}

impl CisBenchmarkResultPy {
    fn from_engine(engine: eggsec::container::cis::CisBenchmarkResult) -> Self {
        Self {
            benchmark_version: engine.benchmark_version,
            total_checks: engine.total_checks,
            passed: engine.passed,
            failed: engine.failed,
            warnings: engine.warnings,
            checks: engine
                .checks
                .into_iter()
                .map(CisCheckPy::from_engine)
                .collect(),
        }
    }
}

#[pymethods]
impl CisBenchmarkResultPy {
    #[getter]
    fn checks(&self) -> Vec<CisCheckPy> {
        self.checks.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("benchmark_version", &self.benchmark_version)?;
        dict.set_item("total_checks", self.total_checks)?;
        dict.set_item("passed", self.passed)?;
        dict.set_item("failed", self.failed)?;
        dict.set_item("warnings", self.warnings)?;
        let checks_list = PyList::empty_bound(py);
        for c in &self.checks {
            checks_list.append(c.to_dict(py)?)?;
        }
        dict.set_item("checks", checks_list)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "CisBenchmarkResult(version={}, total={}, passed={}, failed={}, warnings={})",
            self.benchmark_version, self.total_checks, self.passed, self.failed, self.warnings
        )
    }

    fn __str__(&self) -> String {
        format!(
            "CIS {} - {}/{} passed, {} failed, {} warnings",
            self.benchmark_version, self.passed, self.total_checks, self.failed, self.warnings
        )
    }
}

/// A container finding from a scan report.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerFindingPy {
    #[pyo3(get)]
    pub category: String,
    #[pyo3(get)]
    pub severity: Severity,
    #[pyo3(get)]
    pub title: String,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub recommendation: String,
}

impl ContainerFindingPy {
    fn from_engine(engine: eggsec::container::ContainerFinding) -> Self {
        Self {
            category: engine.category,
            severity: Severity::from_engine(engine.severity),
            title: engine.title,
            description: engine.description,
            recommendation: engine.recommendation,
        }
    }
}

#[pymethods]
impl ContainerFindingPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("category", &self.category)?;
        dict.set_item("severity", self.severity.as_str())?;
        dict.set_item("title", &self.title)?;
        dict.set_item("description", &self.description)?;
        dict.set_item("recommendation", &self.recommendation)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ContainerFinding(category={}, severity={}, title={})",
            self.category,
            self.severity.as_str(),
            self.title
        )
    }

    fn __str__(&self) -> String {
        format!(
            "[{}] {} - {} ({})",
            self.severity.as_str(),
            self.title,
            self.category,
            self.description
        )
    }
}

/// Full container scan report.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerReportPy {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub scan_type: ContainerScanTypePy,
    docker: Option<DockerScanResultPy>,
    kubernetes: Option<KubernetesScanResultPy>,
    escape_risks: Option<EscapeDetectionResultPy>,
    cis_benchmarks: Option<CisBenchmarkResultPy>,
    findings: Vec<ContainerFindingPy>,
}

impl ContainerReportPy {
    fn from_engine(engine: eggsec::container::ContainerScanReport) -> Self {
        Self {
            target: engine.target,
            scan_type: ContainerScanTypePy::from_engine(engine.scan_type),
            docker: engine.docker.map(DockerScanResultPy::from_engine),
            kubernetes: engine.kubernetes.map(KubernetesScanResultPy::from_engine),
            escape_risks: engine
                .escape_risks
                .map(EscapeDetectionResultPy::from_engine),
            cis_benchmarks: engine.cis_benchmarks.map(CisBenchmarkResultPy::from_engine),
            findings: engine
                .findings
                .into_iter()
                .map(ContainerFindingPy::from_engine)
                .collect(),
        }
    }
}

#[pymethods]
impl ContainerReportPy {
    #[getter]
    fn docker(&self) -> Option<DockerScanResultPy> {
        self.docker.clone()
    }

    #[getter]
    fn kubernetes(&self) -> Option<KubernetesScanResultPy> {
        self.kubernetes.clone()
    }

    #[getter]
    fn escape_risks(&self) -> Option<EscapeDetectionResultPy> {
        self.escape_risks.clone()
    }

    #[getter]
    fn cis_benchmarks(&self) -> Option<CisBenchmarkResultPy> {
        self.cis_benchmarks.clone()
    }

    #[getter]
    fn findings(&self) -> Vec<ContainerFindingPy> {
        self.findings.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("scan_type", self.scan_type.as_str())?;
        if let Some(ref docker) = self.docker {
            dict.set_item("docker", docker.to_dict(py)?)?;
        } else {
            dict.set_item("docker", py.None())?;
        }
        if let Some(ref k8s) = self.kubernetes {
            dict.set_item("kubernetes", k8s.to_dict(py)?)?;
        } else {
            dict.set_item("kubernetes", py.None())?;
        }
        if let Some(ref escape) = self.escape_risks {
            dict.set_item("escape_risks", escape.to_dict(py)?)?;
        } else {
            dict.set_item("escape_risks", py.None())?;
        }
        if let Some(ref cis) = self.cis_benchmarks {
            dict.set_item("cis_benchmarks", cis.to_dict(py)?)?;
        } else {
            dict.set_item("cis_benchmarks", py.None())?;
        }
        let findings_list = PyList::empty_bound(py);
        for f in &self.findings {
            findings_list.append(f.to_dict(py)?)?;
        }
        dict.set_item("findings", findings_list)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ContainerReport(target={}, type={}, findings={})",
            self.target,
            self.scan_type.as_str(),
            self.findings.len()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Container scan of '{}': type={}, {} findings",
            self.target,
            self.scan_type.as_str(),
            self.findings.len()
        )
    }
}

/// Scan a Docker image for misconfigurations and security issues.
///
/// Args:
///     image_name: Docker image name (e.g. "nginx:latest").
///
/// Returns:
///     DockerScanResultPy: Scan result with layers, misconfigurations, and metadata.
///
/// Raises:
///     ScanError: If the scan fails.
#[pyfunction]
pub fn scan_docker_image(image_name: &str) -> PyResult<DockerScanResultPy> {
    let image_owned = image_name.to_string();
    Python::with_gil(|py| {
        let result = runtime_sync::block_on(py, async move {
            let scanner = eggsec::container::docker::DockerScanner::new();
            scanner.scan_image(&image_owned).await.map_pyerr()
        })?;
        Ok(DockerScanResultPy::from_engine(result))
    })
}

/// Scan a Docker image asynchronously.
///
/// Returns a PyFuture that resolves to DockerScanResultPy.
#[pyfunction]
pub fn async_scan_docker_image(image_name: &str) -> PyResult<crate::runtime_async::PyFuture> {
    let image_owned = image_name.to_string();

    runtime_async::spawn_async(async move {
        let scanner = eggsec::container::docker::DockerScanner::new();
        let result = scanner.scan_image(&image_owned).await.map_pyerr()?;
        Ok(DockerScanResultPy::from_engine(result))
    })
}

/// Scan a Kubernetes cluster for security issues.
///
/// Args:
///     api_server: Kubernetes API server URL (e.g. "https://k8s.example.com").
///     token: Optional bearer token for authentication.
///     timeout_secs: Request timeout in seconds.
///
/// Returns:
///     KubernetesScanResultPy: Scan result with RBAC, network policy, pod security, and secret findings.
///
/// Raises:
///     ConfigError: If the API server URL is invalid.
///     ScanError: If the scan fails.
#[pyfunction]
#[pyo3(signature = (api_server, token=None, timeout_secs=30))]
pub fn scan_kubernetes(
    api_server: &str,
    token: Option<&str>,
    timeout_secs: u64,
) -> PyResult<KubernetesScanResultPy> {
    let api_owned = api_server.to_string();
    let token_owned = token.map(|s| s.to_string());
    Python::with_gil(|py| {
        let result = runtime_sync::block_on(py, async move {
            let scanner = eggsec::container::kubernetes::KubernetesScanner::new(
                &api_owned,
                token_owned,
                timeout_secs,
            )
            .map_pyerr()?;
            scanner.scan().await.map_pyerr()
        })?;
        Ok(KubernetesScanResultPy::from_engine(result))
    })
}

/// Scan a Kubernetes cluster asynchronously.
///
/// Returns a PyFuture that resolves to KubernetesScanResultPy.
#[pyfunction]
#[pyo3(signature = (api_server, token=None, timeout_secs=30))]
pub fn async_scan_kubernetes(
    api_server: &str,
    token: Option<&str>,
    timeout_secs: u64,
) -> PyResult<crate::runtime_async::PyFuture> {
    let api_owned = api_server.to_string();
    let token_owned = token.map(|s| s.to_string());

    runtime_async::spawn_async(async move {
        let scanner = eggsec::container::kubernetes::KubernetesScanner::new(
            &api_owned,
            token_owned,
            timeout_secs,
        )
        .map_pyerr()?;
        let result = scanner.scan().await.map_pyerr()?;
        Ok(KubernetesScanResultPy::from_engine(result))
    })
}

/// Detect container escape risks from a Docker/Kubernetes configuration.
///
/// Args:
///     config: JSON or YAML configuration string to analyze.
///
/// Returns:
///     EscapeDetectionResultPy: Detected escape risks and overall risk level.
#[pyfunction]
pub fn detect_escape_risks(config: &str) -> PyResult<EscapeDetectionResultPy> {
    let detector = eggsec::container::escape::EscapeDetector::new();
    let result = detector.analyze_docker_config(config);
    Ok(EscapeDetectionResultPy::from_engine(result))
}

/// Check Docker configuration against CIS Docker Benchmark.
///
/// Args:
///     docker_info: Docker configuration or inspect output to check.
///
/// Returns:
///     CisBenchmarkResultPy: Benchmark result with individual check statuses.
#[pyfunction]
pub fn check_cis_docker_benchmark(docker_info: &str) -> PyResult<CisBenchmarkResultPy> {
    let checker = eggsec::container::cis::CisBenchmarkChecker::new();
    let result = checker.check_docker(docker_info);
    Ok(CisBenchmarkResultPy::from_engine(result))
}
