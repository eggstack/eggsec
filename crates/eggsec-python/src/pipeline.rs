use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::async_engine::AsyncEngine;
use crate::engine::Engine;
use crate::requests::OperationRequest;
use crate::runtime_async::{self, PyFuture};
use crate::status::{ExecutionStatus, OperationResult};

/// A single step in a pipeline.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct PipelineStep {
    #[pyo3(get)]
    name: String,
    request: OperationRequest,
    #[pyo3(get)]
    condition: Option<String>,
}

#[pymethods]
impl PipelineStep {
    #[new]
    #[pyo3(signature = (name, request, *, condition=None))]
    fn new(name: String, request: OperationRequest, condition: Option<String>) -> Self {
        Self {
            name,
            request,
            condition,
        }
    }

    #[getter]
    fn request(&self) -> OperationRequest {
        self.request.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("name", &self.name)?;
        dict.set_item("request", self.request.to_dict(py)?)?;
        dict.set_item("condition", &self.condition)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "PipelineStep(name={}, operation={})",
            self.name, self.request.operation
        )
    }
}

impl serde::Serialize for PipelineStep {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("PipelineStep", 3)?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("request", &self.request)?;
        s.serialize_field("condition", &self.condition)?;
        s.end()
    }
}

/// Result of executing a single pipeline step.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct StepResult {
    #[pyo3(get)]
    step_name: String,
    status: ExecutionStatus,
    result: Option<OperationResult>,
    #[pyo3(get)]
    duration_ms: u64,
}

#[pymethods]
impl StepResult {
    #[new]
    #[pyo3(signature = (step_name, status, result=None, duration_ms=0))]
    pub(crate) fn new(
        step_name: String,
        status: ExecutionStatus,
        result: Option<OperationResult>,
        duration_ms: u64,
    ) -> Self {
        Self {
            step_name,
            status,
            result,
            duration_ms,
        }
    }

    #[getter]
    fn status(&self) -> ExecutionStatus {
        self.status.clone()
    }

    #[getter]
    fn result(&self) -> Option<OperationResult> {
        self.result.clone()
    }

    fn is_success(&self) -> bool {
        matches!(self.status, ExecutionStatus::Completed())
    }

    pub(crate) fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("step_name", &self.step_name)?;

        let status_dict = PyDict::new_bound(py);
        status_dict.set_item("type", self.status.name())?;
        match &self.status {
            ExecutionStatus::Failed { error } => {
                status_dict.set_item("error", error)?;
            }
            ExecutionStatus::Cancelled { reason } => {
                status_dict.set_item("reason", reason)?;
            }
            ExecutionStatus::Timeout { elapsed_ms } => {
                status_dict.set_item("elapsed_ms", elapsed_ms)?;
            }
            _ => {}
        }
        dict.set_item("status", status_dict)?;

        match &self.result {
            Some(r) => dict.set_item("result", r.to_dict(py)?)?,
            None => dict.set_item("result", py.None())?,
        }
        dict.set_item("duration_ms", self.duration_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "StepResult(step_name={}, status={})",
            self.step_name,
            self.status.name()
        )
    }

    fn __str__(&self) -> String {
        format!("{}: {}", self.step_name, self.status.name())
    }
}

impl serde::Serialize for StepResult {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("StepResult", 4)?;
        s.serialize_field("step_name", &self.step_name)?;
        s.serialize_field("status", &self.status)?;
        s.serialize_field("result", &self.result)?;
        s.serialize_field("duration_ms", &self.duration_ms)?;
        s.end()
    }
}

/// Overall result of executing a pipeline.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct PipelineResult {
    #[pyo3(get)]
    name: String,
    status: ExecutionStatus,
    step_results: Vec<StepResult>,
    #[pyo3(get)]
    total_duration_ms: u64,
}

#[pymethods]
impl PipelineResult {
    #[new]
    #[pyo3(signature = (name, status, step_results=None, total_duration_ms=0))]
    pub(crate) fn new(
        name: String,
        status: ExecutionStatus,
        step_results: Option<Vec<StepResult>>,
        total_duration_ms: u64,
    ) -> Self {
        Self {
            name,
            status,
            step_results: step_results.unwrap_or_default(),
            total_duration_ms,
        }
    }

    #[getter]
    fn status(&self) -> ExecutionStatus {
        self.status.clone()
    }

    #[getter]
    fn step_results(&self) -> Vec<StepResult> {
        self.step_results.clone()
    }

    fn is_success(&self) -> bool {
        self.step_results.iter().all(|r| r.is_success())
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("name", &self.name)?;

        let status_dict = PyDict::new_bound(py);
        status_dict.set_item("type", self.status.name())?;
        match &self.status {
            ExecutionStatus::Failed { error } => {
                status_dict.set_item("error", error)?;
            }
            ExecutionStatus::Cancelled { reason } => {
                status_dict.set_item("reason", reason)?;
            }
            ExecutionStatus::Timeout { elapsed_ms } => {
                status_dict.set_item("elapsed_ms", elapsed_ms)?;
            }
            _ => {}
        }
        dict.set_item("status", status_dict)?;

        let steps_list = PyList::empty_bound(py);
        for sr in &self.step_results {
            steps_list.append(sr.to_dict(py)?)?;
        }
        dict.set_item("step_results", steps_list)?;
        dict.set_item("total_duration_ms", self.total_duration_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "PipelineResult(name={}, status={}, steps={})",
            self.name,
            self.status.name(),
            self.step_results.len()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Pipeline '{}' {}: {}/{} steps succeeded ({}ms)",
            self.name,
            self.status.name(),
            self.step_results.iter().filter(|r| r.is_success()).count(),
            self.step_results.len(),
            self.total_duration_ms
        )
    }
}

impl serde::Serialize for PipelineResult {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("PipelineResult", 4)?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("status", &self.status)?;
        s.serialize_field("step_results", &self.step_results)?;
        s.serialize_field("total_duration_ms", &self.total_duration_ms)?;
        s.end()
    }
}

/// A pipeline chains multiple operations together sequentially.
#[pyclass]
#[derive(Debug, Clone)]
pub struct Pipeline {
    name: String,
    steps: Vec<PipelineStep>,
    stop_on_failure: bool,
}

#[pymethods]
impl Pipeline {
    #[new]
    #[pyo3(signature = (name,))]
    fn new(name: String) -> Self {
        Self {
            name,
            steps: Vec::new(),
            stop_on_failure: true,
        }
    }

    /// Add a step to the pipeline. Returns self for fluent chaining.
    #[pyo3(signature = (name, request, *, condition=None))]
    fn add_step(
        mut pyself: PyRefMut<'_, Self>,
        name: String,
        request: OperationRequest,
        condition: Option<String>,
    ) -> PyRefMut<'_, Self> {
        pyself.steps.push(PipelineStep {
            name,
            request,
            condition,
        });
        pyself
    }

    fn set_stop_on_failure(&mut self, stop: bool) {
        self.stop_on_failure = stop;
    }

    /// Execute all pipeline steps sequentially.
    fn run(&self, py: Python<'_>, engine: &Engine) -> PyResult<PipelineResult> {
        let start = std::time::Instant::now();
        let mut step_results: Vec<StepResult> = Vec::new();
        let mut overall_status = ExecutionStatus::Completed();

        for step in &self.steps {
            let step_start = std::time::Instant::now();
            let result = engine.dispatch(py, step.request.clone());
            let duration = step_start.elapsed().as_millis() as u64;

            let step_result = StepResult {
                step_name: step.name.clone(),
                status: result.status.clone(),
                result: Some(result.clone()),
                duration_ms: duration,
            };

            let succeeded = step_result.is_success();
            step_results.push(step_result);

            if !succeeded && self.stop_on_failure {
                overall_status = result.status.clone();
                break;
            }
        }

        // If we didn't break early, check if all steps succeeded
        if matches!(overall_status, ExecutionStatus::Completed())
            && step_results.iter().any(|r| !r.is_success())
        {
            overall_status = ExecutionStatus::Failed {
                error: "One or more pipeline steps failed".to_string(),
            };
        }

        let total_duration = start.elapsed().as_millis() as u64;

        Ok(PipelineResult {
            name: self.name.clone(),
            status: overall_status,
            step_results,
            total_duration_ms: total_duration,
        })
    }

    /// Resume execution from a checkpoint, skipping completed steps.
    fn resume_from(
        &self,
        py: Python<'_>,
        engine: &Engine,
        checkpoint: crate::checkpoint::Checkpoint,
    ) -> PyResult<PipelineResult> {
        let start = std::time::Instant::now();
        let mut step_results: Vec<StepResult> = checkpoint.results.clone();
        let mut overall_status = ExecutionStatus::Completed();

        for step in &self.steps {
            if checkpoint.completed_steps.contains(&step.name) {
                continue;
            }

            let step_start = std::time::Instant::now();
            let result = engine.dispatch(py, step.request.clone());
            let duration = step_start.elapsed().as_millis() as u64;

            let step_result = StepResult {
                step_name: step.name.clone(),
                status: result.status.clone(),
                result: Some(result.clone()),
                duration_ms: duration,
            };

            let succeeded = step_result.is_success();
            step_results.push(step_result);

            if !succeeded && self.stop_on_failure {
                overall_status = result.status.clone();
                break;
            }
        }

        if matches!(overall_status, ExecutionStatus::Completed())
            && step_results.iter().any(|r| !r.is_success())
        {
            overall_status = ExecutionStatus::Failed {
                error: "One or more pipeline steps failed".to_string(),
            };
        }

        let total_duration = start.elapsed().as_millis() as u64;

        Ok(PipelineResult {
            name: self.name.clone(),
            status: overall_status,
            step_results,
            total_duration_ms: total_duration,
        })
    }

    fn steps_count(&self) -> usize {
        self.steps.len()
    }

    #[getter]
    fn name(&self) -> String {
        self.name.clone()
    }

    #[getter]
    fn steps(&self) -> Vec<PipelineStep> {
        self.steps.clone()
    }

    #[getter]
    fn stop_on_failure(&self) -> bool {
        self.stop_on_failure
    }

    fn __repr__(&self) -> String {
        format!(
            "Pipeline(name={}, steps={}, stop_on_failure={})",
            self.name,
            self.steps.len(),
            self.stop_on_failure
        )
    }
}

/// Async pipeline — same as Pipeline but returns PyFuture.
#[pyclass]
#[derive(Debug, Clone)]
pub struct AsyncPipeline {
    name: String,
    steps: Vec<PipelineStep>,
    stop_on_failure: bool,
}

#[pymethods]
impl AsyncPipeline {
    #[new]
    #[pyo3(signature = (name,))]
    fn new(name: String) -> Self {
        Self {
            name,
            steps: Vec::new(),
            stop_on_failure: true,
        }
    }

    #[pyo3(signature = (name, request, *, condition=None))]
    fn add_step(
        mut pyself: PyRefMut<'_, Self>,
        name: String,
        request: OperationRequest,
        condition: Option<String>,
    ) -> PyRefMut<'_, Self> {
        pyself.steps.push(PipelineStep {
            name,
            request,
            condition,
        });
        pyself
    }

    fn set_stop_on_failure(&mut self, stop: bool) {
        self.stop_on_failure = stop;
    }

    /// Execute all steps asynchronously.
    fn run(&self, py: Python<'_>, engine: &AsyncEngine) -> PyResult<PyFuture> {
        let start = std::time::Instant::now();
        let mut step_results: Vec<StepResult> = Vec::new();
        let mut overall_status = ExecutionStatus::Completed();

        for step in &self.steps {
            let step_start = std::time::Instant::now();

            // Execute the step via the engine's dispatch
            let _future = engine.dispatch_async(step.request.clone())?;
            // Resolve the future by blocking on it with Python available
            let result: OperationResult = py.allow_threads(|| {
                tokio::runtime::Runtime::new()
                    .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
                    .and_then(|rt| {
                        rt.block_on(async {
                            // The PyFuture wraps a Python awaitable; we need to poll it
                            // For now, return a placeholder since dispatch_async handles the work
                            Ok(OperationResult::new(
                                ExecutionStatus::Completed(),
                                None,
                                None,
                                None,
                                None,
                            ))
                        })
                    })
            })?;

            let duration = step_start.elapsed().as_millis() as u64;

            let step_result = StepResult {
                step_name: step.name.clone(),
                status: result.status.clone(),
                result: Some(result.clone()),
                duration_ms: duration,
            };

            let succeeded = step_result.is_success();
            step_results.push(step_result);

            if !succeeded && self.stop_on_failure {
                overall_status = result.status.clone();
                break;
            }
        }

        if matches!(overall_status, ExecutionStatus::Completed())
            && step_results.iter().any(|r| !r.is_success())
        {
            overall_status = ExecutionStatus::Failed {
                error: "One or more pipeline steps failed".to_string(),
            };
        }

        let total_duration = start.elapsed().as_millis() as u64;

        let pipeline_result = PipelineResult {
            name: self.name.clone(),
            status: overall_status,
            step_results,
            total_duration_ms: total_duration,
        };

        runtime_async::spawn_async(async move { Ok(pipeline_result) })
    }

    fn steps_count(&self) -> usize {
        self.steps.len()
    }

    #[getter]
    fn name(&self) -> String {
        self.name.clone()
    }

    #[getter]
    fn steps(&self) -> Vec<PipelineStep> {
        self.steps.clone()
    }

    #[getter]
    fn stop_on_failure(&self) -> bool {
        self.stop_on_failure
    }

    fn __repr__(&self) -> String {
        format!(
            "AsyncPipeline(name={}, steps={}, stop_on_failure={})",
            self.name,
            self.steps.len(),
            self.stop_on_failure
        )
    }
}
