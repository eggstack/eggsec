use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

/// Distributed task type.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DistributedTaskTypePy {
    PortScan,
    ServiceFingerprint,
    EndpointDiscovery,
    Fuzz,
    WafTest,
    LoadTest,
    Recon,
}

#[pymethods]
impl DistributedTaskTypePy {
    fn __repr__(&self) -> String {
        format!("DistributedTaskType.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl DistributedTaskTypePy {
    fn as_str(&self) -> &str {
        match self {
            DistributedTaskTypePy::PortScan => "PortScan",
            DistributedTaskTypePy::ServiceFingerprint => "ServiceFingerprint",
            DistributedTaskTypePy::EndpointDiscovery => "EndpointDiscovery",
            DistributedTaskTypePy::Fuzz => "Fuzz",
            DistributedTaskTypePy::WafTest => "WafTest",
            DistributedTaskTypePy::LoadTest => "LoadTest",
            DistributedTaskTypePy::Recon => "Recon",
        }
    }

    fn from_engine(engine: eggsec::distributed::TaskType) -> Self {
        match engine {
            eggsec::distributed::TaskType::PortScan => DistributedTaskTypePy::PortScan,
            eggsec::distributed::TaskType::ServiceFingerprint => {
                DistributedTaskTypePy::ServiceFingerprint
            }
            eggsec::distributed::TaskType::EndpointDiscovery => {
                DistributedTaskTypePy::EndpointDiscovery
            }
            eggsec::distributed::TaskType::Fuzz => DistributedTaskTypePy::Fuzz,
            eggsec::distributed::TaskType::WafTest => DistributedTaskTypePy::WafTest,
            eggsec::distributed::TaskType::LoadTest => DistributedTaskTypePy::LoadTest,
            eggsec::distributed::TaskType::Recon => DistributedTaskTypePy::Recon,
        }
    }

    fn to_engine(&self) -> eggsec::distributed::TaskType {
        match self {
            DistributedTaskTypePy::PortScan => eggsec::distributed::TaskType::PortScan,
            DistributedTaskTypePy::ServiceFingerprint => {
                eggsec::distributed::TaskType::ServiceFingerprint
            }
            DistributedTaskTypePy::EndpointDiscovery => {
                eggsec::distributed::TaskType::EndpointDiscovery
            }
            DistributedTaskTypePy::Fuzz => eggsec::distributed::TaskType::Fuzz,
            DistributedTaskTypePy::WafTest => eggsec::distributed::TaskType::WafTest,
            DistributedTaskTypePy::LoadTest => eggsec::distributed::TaskType::LoadTest,
            DistributedTaskTypePy::Recon => eggsec::distributed::TaskType::Recon,
        }
    }
}

/// Worker status.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkerStatusPy {
    Idle,
    Busy,
    Disconnected,
}

#[pymethods]
impl WorkerStatusPy {
    fn __repr__(&self) -> String {
        format!("WorkerStatus.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl WorkerStatusPy {
    fn as_str(&self) -> &str {
        match self {
            WorkerStatusPy::Idle => "Idle",
            WorkerStatusPy::Busy => "Busy",
            WorkerStatusPy::Disconnected => "Disconnected",
        }
    }

    fn from_engine(engine: eggsec::distributed::WorkerStatus) -> Self {
        match engine {
            eggsec::distributed::WorkerStatus::Idle => WorkerStatusPy::Idle,
            eggsec::distributed::WorkerStatus::Busy => WorkerStatusPy::Busy,
            eggsec::distributed::WorkerStatus::Disconnected => WorkerStatusPy::Disconnected,
        }
    }
}

/// Worker registration information.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRegistrationPy {
    #[pyo3(get)]
    pub worker_id: String,
    #[pyo3(get)]
    pub hostname: String,
    capabilities: Vec<DistributedTaskTypePy>,
    #[pyo3(get)]
    pub max_concurrency: usize,
    #[pyo3(get)]
    pub status: WorkerStatusPy,
    #[pyo3(get)]
    pub last_heartbeat_secs: Option<i64>,
}

impl WorkerRegistrationPy {
    fn from_engine(engine: eggsec::distributed::WorkerRegistration) -> Self {
        Self {
            worker_id: engine.worker_id,
            hostname: engine.hostname,
            capabilities: engine
                .capabilities
                .into_iter()
                .map(DistributedTaskTypePy::from_engine)
                .collect(),
            max_concurrency: engine.max_concurrency,
            status: WorkerStatusPy::from_engine(engine.status),
            last_heartbeat_secs: engine.last_heartbeat_secs,
        }
    }
}

#[pymethods]
impl WorkerRegistrationPy {
    #[getter]
    fn capabilities(&self) -> Vec<DistributedTaskTypePy> {
        self.capabilities.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("worker_id", &self.worker_id)?;
        dict.set_item("hostname", &self.hostname)?;
        dict.set_item("max_concurrency", self.max_concurrency)?;
        dict.set_item("status", self.status.as_str())?;
        dict.set_item("last_heartbeat_secs", &self.last_heartbeat_secs)?;

        let caps_list = PyList::empty_bound(py);
        for c in &self.capabilities {
            caps_list.append(c.as_str())?;
        }
        dict.set_item("capabilities", caps_list)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "WorkerRegistration(id={}, host={})",
            self.worker_id, self.hostname
        )
    }
}

/// Worker heartbeat.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatPy {
    #[pyo3(get)]
    pub worker_id: String,
    #[pyo3(get)]
    pub status: WorkerStatusPy,
    #[pyo3(get)]
    pub current_jobs: usize,
    #[pyo3(get)]
    pub completed_jobs: usize,
    #[pyo3(get)]
    pub failed_jobs: usize,
    #[pyo3(get)]
    pub cpu_usage: f32,
    #[pyo3(get)]
    pub memory_usage: f32,
}

impl HeartbeatPy {
    fn from_engine(engine: eggsec::distributed::Heartbeat) -> Self {
        Self {
            worker_id: engine.worker_id,
            status: WorkerStatusPy::from_engine(engine.status),
            current_jobs: engine.current_jobs,
            completed_jobs: engine.completed_jobs,
            failed_jobs: engine.failed_jobs,
            cpu_usage: engine.cpu_usage,
            memory_usage: engine.memory_usage,
        }
    }
}

#[pymethods]
impl HeartbeatPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("worker_id", &self.worker_id)?;
        dict.set_item("status", self.status.as_str())?;
        dict.set_item("current_jobs", self.current_jobs)?;
        dict.set_item("completed_jobs", self.completed_jobs)?;
        dict.set_item("failed_jobs", self.failed_jobs)?;
        dict.set_item("cpu_usage", self.cpu_usage)?;
        dict.set_item("memory_usage", self.memory_usage)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "Heartbeat(id={}, jobs={})",
            self.worker_id, self.current_jobs
        )
    }
}

/// Distributed task definition.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedTaskPy {
    #[pyo3(get)]
    pub task_id: String,
    #[pyo3(get)]
    pub task_type: DistributedTaskTypePy,
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub parameters_json: String,
}

#[pymethods]
impl DistributedTaskPy {
    #[new]
    fn new(
        task_id: String,
        task_type: DistributedTaskTypePy,
        target: String,
        parameters_json: String,
    ) -> Self {
        Self {
            task_id,
            task_type,
            target,
            parameters_json,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("task_id", &self.task_id)?;
        dict.set_item("task_type", self.task_type.as_str())?;
        dict.set_item("target", &self.target)?;
        dict.set_item("parameters_json", &self.parameters_json)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "DistributedTask(id={}, type={})",
            self.task_id,
            self.task_type.as_str()
        )
    }
}

/// Distributed task result.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedTaskResultPy {
    #[pyo3(get)]
    pub task_id: String,
    #[pyo3(get)]
    pub worker_id: String,
    #[pyo3(get)]
    pub success: bool,
    #[pyo3(get)]
    pub output_json: String,
    #[pyo3(get)]
    pub duration_ms: u64,
}

#[pymethods]
impl DistributedTaskResultPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("task_id", &self.task_id)?;
        dict.set_item("worker_id", &self.worker_id)?;
        dict.set_item("success", self.success)?;
        dict.set_item("output_json", &self.output_json)?;
        dict.set_item("duration_ms", self.duration_ms)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "DistributedTaskResult(task={}, success={})",
            self.task_id, self.success
        )
    }
}

/// Get the list of supported distributed task types.
///
/// Returns:
///     List of task type strings.
#[pyfunction]
pub fn distributed_task_types() -> Vec<String> {
    eggsec::distributed::CAPABILITIES
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Generate a pre-shared key for distributed worker authentication.
///
/// Returns:
///     A hex-encoded PSK string.
#[pyfunction]
pub fn distributed_generate_psk() -> String {
    eggsec::distributed::generate_psk()
}
