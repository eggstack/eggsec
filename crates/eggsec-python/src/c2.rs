use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use crate::finding::Severity;
use crate::runtime_sync;

/// C2 beacon protocol.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BeaconProtocolPy {
    Http,
    Https,
    Dns,
    Tcp,
    Custom,
}

#[pymethods]
impl BeaconProtocolPy {
    fn __repr__(&self) -> String {
        format!("BeaconProtocol.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl BeaconProtocolPy {
    fn as_str(&self) -> &str {
        match self {
            BeaconProtocolPy::Http => "Http",
            BeaconProtocolPy::Https => "Https",
            BeaconProtocolPy::Dns => "Dns",
            BeaconProtocolPy::Tcp => "Tcp",
            BeaconProtocolPy::Custom => "Custom",
        }
    }

    fn from_engine(engine: eggsec::c2::BeaconProtocol) -> Self {
        match engine {
            eggsec::c2::BeaconProtocol::Http => BeaconProtocolPy::Http,
            eggsec::c2::BeaconProtocol::Https => BeaconProtocolPy::Https,
            eggsec::c2::BeaconProtocol::Dns => BeaconProtocolPy::Dns,
            eggsec::c2::BeaconProtocol::Tcp => BeaconProtocolPy::Tcp,
            eggsec::c2::BeaconProtocol::Custom => BeaconProtocolPy::Custom,
        }
    }
}

/// C2 task type.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskTypePy {
    Recon,
    Execute,
    Exfil,
    Persist,
    Lateral,
    Evade,
    SelfDestruct,
}

#[pymethods]
impl TaskTypePy {
    fn __repr__(&self) -> String {
        format!("TaskType.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl TaskTypePy {
    fn as_str(&self) -> &str {
        match self {
            TaskTypePy::Recon => "Recon",
            TaskTypePy::Execute => "Execute",
            TaskTypePy::Exfil => "Exfil",
            TaskTypePy::Persist => "Persist",
            TaskTypePy::Lateral => "Lateral",
            TaskTypePy::Evade => "Evade",
            TaskTypePy::SelfDestruct => "SelfDestruct",
        }
    }

    fn from_engine(engine: eggsec::c2::TaskType) -> Self {
        match engine {
            eggsec::c2::TaskType::Recon => TaskTypePy::Recon,
            eggsec::c2::TaskType::Execute => TaskTypePy::Execute,
            eggsec::c2::TaskType::Exfil => TaskTypePy::Exfil,
            eggsec::c2::TaskType::Persist => TaskTypePy::Persist,
            eggsec::c2::TaskType::Lateral => TaskTypePy::Lateral,
            eggsec::c2::TaskType::Evade => TaskTypePy::Evade,
            eggsec::c2::TaskType::SelfDestruct => TaskTypePy::SelfDestruct,
        }
    }
}

/// C2 task status.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskStatusPy {
    Completed,
    Failed,
    Simulated,
    Denied,
}

#[pymethods]
impl TaskStatusPy {
    fn __repr__(&self) -> String {
        format!("TaskStatus.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl TaskStatusPy {
    fn as_str(&self) -> &str {
        match self {
            TaskStatusPy::Completed => "Completed",
            TaskStatusPy::Failed => "Failed",
            TaskStatusPy::Simulated => "Simulated",
            TaskStatusPy::Denied => "Denied",
        }
    }

    fn from_engine(engine: eggsec::c2::TaskStatus) -> Self {
        match engine {
            eggsec::c2::TaskStatus::Completed => TaskStatusPy::Completed,
            eggsec::c2::TaskStatus::Failed => TaskStatusPy::Failed,
            eggsec::c2::TaskStatus::Simulated => TaskStatusPy::Simulated,
            eggsec::c2::TaskStatus::Denied => TaskStatusPy::Denied,
        }
    }
}

/// OPSEC assessment category.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OpsecCategoryPy {
    ParentSpoofing,
    Timestomping,
    LogTampering,
    ProcessMasquerading,
    BurnMechanism,
    DecoyActivity,
}

#[pymethods]
impl OpsecCategoryPy {
    fn __repr__(&self) -> String {
        format!("OpsecCategory.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl OpsecCategoryPy {
    fn as_str(&self) -> &str {
        match self {
            OpsecCategoryPy::ParentSpoofing => "ParentSpoofing",
            OpsecCategoryPy::Timestomping => "Timestomping",
            OpsecCategoryPy::LogTampering => "LogTampering",
            OpsecCategoryPy::ProcessMasquerading => "ProcessMasquerading",
            OpsecCategoryPy::BurnMechanism => "BurnMechanism",
            OpsecCategoryPy::DecoyActivity => "DecoyActivity",
        }
    }

    fn from_engine(engine: eggsec::c2::OpsecCategory) -> Self {
        match engine {
            eggsec::c2::OpsecCategory::ParentSpoofing => OpsecCategoryPy::ParentSpoofing,
            eggsec::c2::OpsecCategory::Timestomping => OpsecCategoryPy::Timestomping,
            eggsec::c2::OpsecCategory::LogTampering => OpsecCategoryPy::LogTampering,
            eggsec::c2::OpsecCategory::ProcessMasquerading => OpsecCategoryPy::ProcessMasquerading,
            eggsec::c2::OpsecCategory::BurnMechanism => OpsecCategoryPy::BurnMechanism,
            eggsec::c2::OpsecCategory::DecoyActivity => OpsecCategoryPy::DecoyActivity,
        }
    }
}

/// OPSEC severity level.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OpsecSeverityPy {
    Info,
    Low,
    Medium,
    High,
}

#[pymethods]
impl OpsecSeverityPy {
    fn __repr__(&self) -> String {
        format!("OpsecSeverity.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl OpsecSeverityPy {
    fn as_str(&self) -> &str {
        match self {
            OpsecSeverityPy::Info => "Info",
            OpsecSeverityPy::Low => "Low",
            OpsecSeverityPy::Medium => "Medium",
            OpsecSeverityPy::High => "High",
        }
    }

    fn from_engine(engine: eggsec::c2::OpsecSeverity) -> Self {
        match engine {
            eggsec::c2::OpsecSeverity::Info => OpsecSeverityPy::Info,
            eggsec::c2::OpsecSeverity::Low => OpsecSeverityPy::Low,
            eggsec::c2::OpsecSeverity::Medium => OpsecSeverityPy::Medium,
            eggsec::c2::OpsecSeverity::High => OpsecSeverityPy::High,
        }
    }

    #[allow(dead_code)]
    fn to_severity(&self) -> Severity {
        match self {
            OpsecSeverityPy::Info => Severity::Info,
            OpsecSeverityPy::Low => Severity::Low,
            OpsecSeverityPy::Medium => Severity::Medium,
            OpsecSeverityPy::High => Severity::High,
        }
    }
}

/// A C2 campaign phase.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignPhasePy {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub description: String,
    mitre_techniques: Vec<String>,
    #[pyo3(get)]
    pub order: u32,
}

impl CampaignPhasePy {
    fn from_engine(engine: eggsec::c2::CampaignPhase) -> Self {
        Self {
            id: engine.id,
            name: engine.name,
            description: engine.description,
            mitre_techniques: engine.mitre_techniques,
            order: engine.order,
        }
    }
}

#[pymethods]
impl CampaignPhasePy {
    #[getter]
    fn mitre_techniques(&self) -> Vec<String> {
        self.mitre_techniques.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.id)?;
        dict.set_item("name", &self.name)?;
        dict.set_item("description", &self.description)?;
        dict.set_item("mitre_techniques", &self.mitre_techniques)?;
        dict.set_item("order", self.order)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!("CampaignPhase(id={}, name={})", self.id, self.name)
    }
}

/// A C2 campaign definition.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2CampaignPy {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub mitre_profile: String,
    phases: Vec<CampaignPhasePy>,
}

impl C2CampaignPy {
    fn from_engine(engine: eggsec::c2::C2Campaign) -> Self {
        Self {
            id: engine.id,
            name: engine.name,
            description: engine.description,
            mitre_profile: engine.mitre_profile,
            phases: engine
                .phases
                .into_iter()
                .map(CampaignPhasePy::from_engine)
                .collect(),
        }
    }
}

#[pymethods]
impl C2CampaignPy {
    #[getter]
    fn phases(&self) -> Vec<CampaignPhasePy> {
        self.phases.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.id)?;
        dict.set_item("name", &self.name)?;
        dict.set_item("description", &self.description)?;
        dict.set_item("mitre_profile", &self.mitre_profile)?;

        let phases_list = PyList::empty_bound(py);
        for p in &self.phases {
            phases_list.append(p.to_dict(py)?)?;
        }
        dict.set_item("phases", phases_list)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!("C2Campaign(id={}, name={})", self.id, self.name)
    }
}

/// A C2 beacon test result.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeaconResultPy {
    #[pyo3(get)]
    pub protocol: BeaconProtocolPy,
    #[pyo3(get)]
    pub interval_ms: u64,
    #[pyo3(get)]
    pub jitter_percent: u32,
    #[pyo3(get)]
    pub success: bool,
    #[pyo3(get)]
    pub evidence: Option<String>,
}

impl BeaconResultPy {
    fn from_engine(engine: eggsec::c2::BeaconResult) -> Self {
        Self {
            protocol: BeaconProtocolPy::from_engine(engine.protocol),
            interval_ms: engine.interval_ms,
            jitter_percent: engine.jitter_percent,
            success: engine.success,
            evidence: engine.evidence,
        }
    }
}

#[pymethods]
impl BeaconResultPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("protocol", self.protocol.as_str())?;
        dict.set_item("interval_ms", self.interval_ms)?;
        dict.set_item("jitter_percent", self.jitter_percent)?;
        dict.set_item("success", self.success)?;
        dict.set_item("evidence", &self.evidence)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "BeaconResult(protocol={}, success={})",
            self.protocol.as_str(),
            self.success
        )
    }
}

/// A C2 task execution result.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2TaskResultPy {
    #[pyo3(get)]
    pub task_type: TaskTypePy,
    #[pyo3(get)]
    pub status: TaskStatusPy,
    #[pyo3(get)]
    pub output: Option<String>,
    #[pyo3(get)]
    pub mitre_technique: Option<String>,
}

impl C2TaskResultPy {
    fn from_engine(engine: eggsec::c2::TaskResult) -> Self {
        Self {
            task_type: TaskTypePy::from_engine(engine.task_type),
            status: TaskStatusPy::from_engine(engine.status),
            output: engine.output,
            mitre_technique: engine.mitre_technique,
        }
    }
}

#[pymethods]
impl C2TaskResultPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("task_type", self.task_type.as_str())?;
        dict.set_item("status", self.status.as_str())?;
        dict.set_item("output", &self.output)?;
        dict.set_item("mitre_technique", &self.mitre_technique)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "C2TaskResult(type={}, status={})",
            self.task_type.as_str(),
            self.status.as_str()
        )
    }
}

/// An OPSEC finding.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsecFindingPy {
    #[pyo3(get)]
    pub category: OpsecCategoryPy,
    #[pyo3(get)]
    pub severity: OpsecSeverityPy,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub recommendation: String,
}

impl OpsecFindingPy {
    fn from_engine(engine: eggsec::c2::OpsecFinding) -> Self {
        Self {
            category: OpsecCategoryPy::from_engine(engine.category),
            severity: OpsecSeverityPy::from_engine(engine.severity),
            description: engine.description,
            recommendation: engine.recommendation,
        }
    }
}

#[pymethods]
impl OpsecFindingPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("category", self.category.as_str())?;
        dict.set_item("severity", self.severity.as_str())?;
        dict.set_item("description", &self.description)?;
        dict.set_item("recommendation", &self.recommendation)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "OpsecFinding(category={}, severity={})",
            self.category.as_str(),
            self.severity.as_str()
        )
    }
}

/// OPSEC assessment result.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsecAssessmentPy {
    #[pyo3(get)]
    pub score: u32,
    #[pyo3(get)]
    pub max_score: u32,
    findings: Vec<OpsecFindingPy>,
}

impl OpsecAssessmentPy {
    fn from_engine(engine: eggsec::c2::OpsecAssessment) -> Self {
        Self {
            score: engine.score,
            max_score: engine.max_score,
            findings: engine
                .findings
                .into_iter()
                .map(OpsecFindingPy::from_engine)
                .collect(),
        }
    }
}

#[pymethods]
impl OpsecAssessmentPy {
    #[getter]
    fn findings(&self) -> Vec<OpsecFindingPy> {
        self.findings.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("score", self.score)?;
        dict.set_item("max_score", self.max_score)?;

        let findings_list = PyList::empty_bound(py);
        for f in &self.findings {
            findings_list.append(f.to_dict(py)?)?;
        }
        dict.set_item("findings", findings_list)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!("OpsecAssessment(score={}/{})", self.score, self.max_score)
    }
}

/// C2 scan summary.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2SummaryPy {
    #[pyo3(get)]
    pub total_beacons: usize,
    #[pyo3(get)]
    pub successful_beacons: usize,
    #[pyo3(get)]
    pub total_tasks: usize,
    #[pyo3(get)]
    pub completed_tasks: usize,
    #[pyo3(get)]
    pub opsec_score: u32,
    #[pyo3(get)]
    pub opsec_max: u32,
}

impl C2SummaryPy {
    fn from_engine(engine: eggsec::c2::C2Summary) -> Self {
        Self {
            total_beacons: engine.total_beacons,
            successful_beacons: engine.successful_beacons,
            total_tasks: engine.total_tasks,
            completed_tasks: engine.completed_tasks,
            opsec_score: engine.opsec_score,
            opsec_max: engine.opsec_max,
        }
    }
}

#[pymethods]
impl C2SummaryPy {
    fn __repr__(&self) -> String {
        format!(
            "C2Summary(beacons={}/{}, tasks={}/{}, opsec={}/{})",
            self.successful_beacons,
            self.total_beacons,
            self.completed_tasks,
            self.total_tasks,
            self.opsec_score,
            self.opsec_max
        )
    }
}

/// Full C2 simulation report.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2ReportPy {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub campaign: C2CampaignPy,
    beacon_results: Vec<BeaconResultPy>,
    task_results: Vec<C2TaskResultPy>,
    #[pyo3(get)]
    pub opsec_assessment: OpsecAssessmentPy,
    #[pyo3(get)]
    pub summary: C2SummaryPy,
    #[pyo3(get)]
    pub timestamp: String,
    #[pyo3(get)]
    pub dry_run: bool,
}

impl C2ReportPy {
    fn from_engine(engine: eggsec::c2::C2Report) -> Self {
        Self {
            target: engine.target,
            campaign: C2CampaignPy::from_engine(engine.campaign),
            beacon_results: engine
                .beacon_results
                .into_iter()
                .map(BeaconResultPy::from_engine)
                .collect(),
            task_results: engine
                .task_results
                .into_iter()
                .map(C2TaskResultPy::from_engine)
                .collect(),
            opsec_assessment: OpsecAssessmentPy::from_engine(engine.opsec_assessment),
            summary: C2SummaryPy::from_engine(engine.summary),
            timestamp: engine.timestamp,
            dry_run: engine.dry_run,
        }
    }
}

#[pymethods]
impl C2ReportPy {
    #[getter]
    fn beacon_results(&self) -> Vec<BeaconResultPy> {
        self.beacon_results.clone()
    }

    #[getter]
    fn task_results(&self) -> Vec<C2TaskResultPy> {
        self.task_results.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("campaign", self.campaign.to_dict(py)?)?;
        dict.set_item("timestamp", &self.timestamp)?;
        dict.set_item("dry_run", self.dry_run)?;
        dict.set_item("opsec_assessment", self.opsec_assessment.to_dict(py)?)?;

        let beacons_list = PyList::empty_bound(py);
        for b in &self.beacon_results {
            beacons_list.append(b.to_dict(py)?)?;
        }
        dict.set_item("beacon_results", beacons_list)?;

        let tasks_list = PyList::empty_bound(py);
        for t in &self.task_results {
            tasks_list.append(t.to_dict(py)?)?;
        }
        dict.set_item("task_results", tasks_list)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "C2Report(target={}, campaign={})",
            self.target, self.campaign.name
        )
    }
}

/// Configuration for a C2 simulation scan.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2ScanConfigPy {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub campaign_profile: String,
    #[pyo3(get)]
    pub dry_run: bool,
}

#[pymethods]
impl C2ScanConfigPy {
    #[new]
    #[pyo3(signature = (target, campaign_profile="default".to_string(), dry_run=true))]
    fn new(target: String, campaign_profile: String, dry_run: bool) -> Self {
        Self {
            target,
            campaign_profile,
            dry_run,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "C2ScanConfig(target={}, profile={}, dry_run={})",
            self.target, self.campaign_profile, self.dry_run
        )
    }
}

/// Run a C2 simulation scan.
///
/// Simulates command-and-control communication patterns, beaconing,
/// task execution, and OPSEC assessment against a target.
///
/// Args:
///     config: C2 scan configuration.
///
/// Returns:
///     C2ReportPy: Full C2 simulation report.
///
/// Raises:
///     FeatureUnavailableError: If c2 feature is not enabled.
///     ScanError: If the scan fails.
#[pyfunction]
pub fn c2_scan(config: C2ScanConfigPy) -> PyResult<C2ReportPy> {
    Python::with_gil(|py| {
        let result = runtime_sync::block_on(py, async move {
            let scanner = eggsec::c2::C2Scanner::new(config.dry_run, &config.campaign_profile);
            scanner.scan(&config.target).await.map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("C2 scan failed: {}", e))
            })
        })?;

        Ok(C2ReportPy::from_engine(result))
    })
}

/// Run a C2 simulation scan (async).
///
/// Returns a PyFuture that resolves to a C2ReportPy.
#[pyfunction]
pub fn async_c2_scan(config: C2ScanConfigPy) -> PyResult<crate::runtime_async::PyFuture> {
    crate::runtime_async::spawn_async(async move {
        let scanner = eggsec::c2::C2Scanner::new(config.dry_run, &config.campaign_profile);
        let report = scanner.scan(&config.target).await.map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("C2 scan failed: {}", e))
        })?;
        Ok(C2ReportPy::from_engine(report))
    })
}

/// Get the current C2 campaign definition.
///
/// Returns:
///     C2CampaignPy: The active campaign with its phases.
#[pyfunction]
pub fn c2_get_campaign() -> PyResult<C2CampaignPy> {
    let scanner = eggsec::c2::C2Scanner::new(true, "default");
    Ok(C2CampaignPy::from_engine(scanner.campaign().clone()))
}
