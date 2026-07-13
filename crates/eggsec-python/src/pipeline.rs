use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use std::sync::Arc;

use crate::async_engine::AsyncEngine;
use crate::cancellation::CancellationToken;
use crate::checkpoint_store::{self, CheckpointVersion, PipelineCheckpoint};
use crate::engine::Engine;
use crate::event_protocol::{
    wrap_event, CompletionEvent, EventEnvelope, FailureEvent, StageLifecycleEvent,
};
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
#[derive(Debug, Clone, serde::Deserialize)]
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

fn pipeline_definition_text(name: &str, steps: &[PipelineStep], stop_on_failure: bool) -> String {
    let requests = steps
        .iter()
        .map(|step| {
            serde_json::json!({
                "name": step.name,
                "request": step.request,
                "condition": step.condition,
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "name": name,
        "stop_on_failure": stop_on_failure,
        "steps": requests,
    })
    .to_string()
}

fn checkpoint_compatibility(
    name: &str,
    steps: &[PipelineStep],
    stop_on_failure: bool,
    engine: &Engine,
) -> checkpoint_store::CheckpointCompatibility {
    let definition = pipeline_definition_text(name, steps, stop_on_failure);
    let mut targets = steps
        .iter()
        .map(|step| step.request.target.clone())
        .collect::<Vec<_>>();
    targets.sort();
    let feature_set = {
        let mut features = crate::features::features().into_iter().collect::<Vec<_>>();
        features.sort_by(|left, right| left.0.cmp(&right.0));
        serde_json::to_string(&features).expect("feature map is JSON serializable")
    };
    let scope_definition =
        serde_json::to_string(&engine.state.scope.inner).expect("scope is JSON serializable");
    checkpoint_store::CheckpointCompatibility {
        operation_schema_version: checkpoint_store::OPERATION_SCHEMA_VERSION.to_string(),
        target_set_hash: checkpoint_store::stable_digest(&targets.join("\n")),
        scope_hash: checkpoint_store::stable_digest(&scope_definition),
        execution_profile: engine.state.enforcement.execution_profile.to_string(),
        enabled_features_hash: checkpoint_store::stable_digest(&feature_set),
        pipeline_definition_hash: checkpoint_store::stable_digest(&definition),
        artifact_store_id: None,
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
    #[pyo3(get)]
    events: Vec<EventEnvelope>,
}

#[pymethods]
impl PipelineResult {
    #[new]
    #[pyo3(signature = (name, status, step_results=None, total_duration_ms=0, events=None))]
    pub(crate) fn new(
        name: String,
        status: ExecutionStatus,
        step_results: Option<Vec<StepResult>>,
        total_duration_ms: u64,
        events: Option<Vec<EventEnvelope>>,
    ) -> Self {
        Self {
            name,
            status,
            step_results: step_results.unwrap_or_default(),
            total_duration_ms,
            events: events.unwrap_or_default(),
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

        let events_list = PyList::empty_bound(py);
        for ev in &self.events {
            let obj: PyObject = ev.clone().into_py(py);
            events_list.append(obj)?;
        }
        dict.set_item("events", events_list)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "PipelineResult(name={}, status={}, steps={}, events={})",
            self.name,
            self.status.name(),
            self.step_results.len(),
            self.events.len()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Pipeline '{}' {}: {}/{} steps succeeded, {} events ({}ms)",
            self.name,
            self.status.name(),
            self.step_results.iter().filter(|r| r.is_success()).count(),
            self.step_results.len(),
            self.events.len(),
            self.total_duration_ms
        )
    }
}

impl serde::Serialize for PipelineResult {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("PipelineResult", 5)?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("status", &self.status)?;
        s.serialize_field("step_results", &self.step_results)?;
        s.serialize_field("total_duration_ms", &self.total_duration_ms)?;
        s.serialize_field("events", &self.events)?;
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
    cancel_token: Option<CancellationToken>,
    checkpoint_store: Option<Arc<checkpoint_store::CheckpointStore>>,
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
            cancel_token: None,
            checkpoint_store: None,
        }
    }

    /// Set a cancellation token for cooperative cancellation.
    fn set_cancel_token(&mut self, token: CancellationToken) {
        self.cancel_token = Some(token);
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

    /// Attach a checkpoint store for automatic save/resume.
    fn set_checkpoint_store(&mut self, store: checkpoint_store::CheckpointStore) {
        self.checkpoint_store = Some(Arc::new(store));
    }

    /// Execute all pipeline steps sequentially.
    fn run(&self, py: Python<'_>, engine: &Engine) -> PyResult<PipelineResult> {
        let start = std::time::Instant::now();
        let mut step_results: Vec<StepResult> = Vec::new();
        let mut events: Vec<EventEnvelope> = Vec::new();
        let mut overall_status = ExecutionStatus::Completed();
        let correlation_id = format!("pipeline-{}", start.elapsed().as_millis());

        // Determine pipeline ID for checkpointing
        let pipeline_id = self.pipeline_id();
        let compatibility =
            checkpoint_compatibility(&self.name, &self.steps, self.stop_on_failure, engine);

        // Check for existing checkpoint to resume from
        let mut completed_steps: Vec<String> = Vec::new();
        if let Some(ref store) = self.checkpoint_store {
            if let Some(load_result) = store.load_inner(&pipeline_id)? {
                compatibility.validate(&load_result.checkpoint)?;
                completed_steps = load_result.checkpoint.completed_steps.clone();
                for step_name in &completed_steps {
                    let value = load_result
                        .checkpoint
                        .step_results
                        .get(step_name)
                        .ok_or_else(|| {
                            pyo3::exceptions::PyValueError::new_err(format!(
                                "checkpoint_incompatible: missing result for completed step '{step_name}'"
                            ))
                        })?;
                    step_results.push(serde_json::from_value(value.clone()).map_err(|error| {
                        pyo3::exceptions::PyValueError::new_err(format!(
                            "checkpoint_incompatible: invalid result for completed step '{step_name}': {error}"
                        ))
                    })?);
                }
                events.push(wrap_event(
                    py,
                    "pipeline.resumed_from_checkpoint".to_string(),
                    StageLifecycleEvent::new(self.name.clone(), "resumed".to_string()).into_py(py),
                    Some(correlation_id.clone()),
                    None,
                )?);
            }
        }

        // Emit pipeline started event
        events.push(wrap_event(
            py,
            "pipeline.started".to_string(),
            StageLifecycleEvent::new(self.name.clone(), "started".to_string()).into_py(py),
            Some(correlation_id.clone()),
            None,
        )?);

        for step in &self.steps {
            // Skip already-completed steps when resuming
            if completed_steps.contains(&step.name) {
                continue;
            }

            // Check for cancellation before starting the step
            if let Some(ref token) = self.cancel_token {
                if token.is_cancelled() {
                    let reason = token
                        .reason()
                        .unwrap_or_else(|| "Pipeline cancelled".to_string());
                    overall_status = ExecutionStatus::Cancelled {
                        reason: Some(reason),
                    };
                    break;
                }
            }

            // Emit step started event
            events.push(wrap_event(
                py,
                "step.started".to_string(),
                StageLifecycleEvent::new(step.name.clone(), "started".to_string()).into_py(py),
                Some(correlation_id.clone()),
                None,
            )?);

            let step_start = std::time::Instant::now();
            let result = engine.dispatch(py, step.request.clone());
            let duration = step_start.elapsed().as_millis() as u64;

            // Emit step completed/failed event
            let step_status = if result.is_success() {
                "completed".to_string()
            } else {
                "failed".to_string()
            };
            events.push(wrap_event(
                py,
                format!("step.{}", step_status),
                StageLifecycleEvent::new(step.name.clone(), step_status).into_py(py),
                Some(correlation_id.clone()),
                None,
            )?);

            let step_result = StepResult {
                step_name: step.name.clone(),
                status: result.status.clone(),
                result: Some(result.clone()),
                duration_ms: duration,
            };

            let succeeded = step_result.is_success();

            // Save checkpoint after successful step
            if succeeded {
                if let Some(ref store) = self.checkpoint_store {
                    let mut step_results_map = std::collections::HashMap::new();
                    for sr in &step_results {
                        let val = serde_json::to_value(sr).map_err(|error| {
                            pyo3::exceptions::PyValueError::new_err(format!(
                                "failed to serialize checkpoint result: {error}"
                            ))
                        })?;
                        step_results_map.insert(sr.step_name.clone(), val);
                    }
                    let current_value = serde_json::to_value(&step_result).map_err(|error| {
                        pyo3::exceptions::PyValueError::new_err(format!(
                            "failed to serialize checkpoint result: {error}"
                        ))
                    })?;
                    step_results_map.insert(step_result.step_name.clone(), current_value);

                    let mut completed = completed_steps.clone();
                    completed.push(step.name.clone());

                    let now_ms = crate::checkpoint_store::current_epoch_ms();
                    let cp = PipelineCheckpoint {
                        version: CheckpointVersion::current(),
                        pipeline_id: pipeline_id.clone(),
                        pipeline_name: self.name.clone(),
                        completed_steps: completed,
                        current_step: None,
                        step_results: step_results_map,
                        created_at_ms: now_ms,
                        updated_at_ms: now_ms,
                        operation_schema_version: compatibility.operation_schema_version.clone(),
                        target_set_hash: compatibility.target_set_hash.clone(),
                        scope_hash: compatibility.scope_hash.clone(),
                        execution_profile: compatibility.execution_profile.clone(),
                        enabled_features_hash: compatibility.enabled_features_hash.clone(),
                        pipeline_definition_hash: compatibility.pipeline_definition_hash.clone(),
                        artifact_store_id: compatibility.artifact_store_id.clone(),
                    };
                    store.save_inner(cp)?;
                }
            }

            step_results.push(step_result);

            if !succeeded && self.stop_on_failure {
                overall_status = result.status.clone();

                // Emit failure event
                let error_msg = match &result.status {
                    ExecutionStatus::Failed { error } => error.clone(),
                    other => format!("Step failed: {}", other.name()),
                };
                events.push(wrap_event(
                    py,
                    "pipeline.failure".to_string(),
                    FailureEvent::new("step_failure".to_string(), error_msg, false).into_py(py),
                    Some(correlation_id.clone()),
                    None,
                )?);
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

        // Emit pipeline completed event
        events.push(wrap_event(
            py,
            "pipeline.completed".to_string(),
            CompletionEvent::new(py, overall_status.name().to_string(), None, total_duration)
                .into_py(py),
            Some(correlation_id),
            None,
        )?);

        // Remove checkpoint on successful completion
        if overall_status.is_success() {
            if let Some(ref store) = self.checkpoint_store {
                let _ = store.delete_inner(&pipeline_id);
            }
        }

        Ok(PipelineResult {
            name: self.name.clone(),
            status: overall_status,
            step_results,
            total_duration_ms: total_duration,
            events,
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
        let mut events: Vec<EventEnvelope> = Vec::new();
        let mut overall_status = ExecutionStatus::Completed();
        let correlation_id = format!("pipeline-resume-{}", start.elapsed().as_millis());

        events.push(wrap_event(
            py,
            "pipeline.resumed".to_string(),
            StageLifecycleEvent::new(self.name.clone(), "resumed".to_string()).into_py(py),
            Some(correlation_id.clone()),
            None,
        )?);

        for step in &self.steps {
            if checkpoint.completed_steps.contains(&step.name) {
                continue;
            }

            // Check for cancellation before starting the step
            if let Some(ref token) = self.cancel_token {
                if token.is_cancelled() {
                    let reason = token
                        .reason()
                        .unwrap_or_else(|| "Pipeline cancelled".to_string());
                    overall_status = ExecutionStatus::Cancelled {
                        reason: Some(reason),
                    };
                    break;
                }
            }

            // Emit step started event
            events.push(wrap_event(
                py,
                "step.started".to_string(),
                StageLifecycleEvent::new(step.name.clone(), "started".to_string()).into_py(py),
                Some(correlation_id.clone()),
                None,
            )?);

            let step_start = std::time::Instant::now();
            let result = engine.dispatch(py, step.request.clone());
            let duration = step_start.elapsed().as_millis() as u64;

            // Emit step completed/failed event
            let step_status = if result.is_success() {
                "completed".to_string()
            } else {
                "failed".to_string()
            };
            events.push(wrap_event(
                py,
                format!("step.{}", step_status),
                StageLifecycleEvent::new(step.name.clone(), step_status).into_py(py),
                Some(correlation_id.clone()),
                None,
            )?);

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

                let error_msg = match &result.status {
                    ExecutionStatus::Failed { error } => error.clone(),
                    other => format!("Step failed: {}", other.name()),
                };
                events.push(wrap_event(
                    py,
                    "pipeline.failure".to_string(),
                    FailureEvent::new("step_failure".to_string(), error_msg, false).into_py(py),
                    Some(correlation_id.clone()),
                    None,
                )?);
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

        // Emit pipeline completed event
        events.push(wrap_event(
            py,
            "pipeline.completed".to_string(),
            CompletionEvent::new(py, overall_status.name().to_string(), None, total_duration)
                .into_py(py),
            Some(correlation_id),
            None,
        )?);

        Ok(PipelineResult {
            name: self.name.clone(),
            status: overall_status,
            step_results,
            total_duration_ms: total_duration,
            events,
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

    /// Generate a deterministic pipeline ID from the pipeline name and step names.
    /// Used as the key for checkpoint storage.
    fn pipeline_id(&self) -> String {
        checkpoint_store::stable_digest(&pipeline_definition_text(
            &self.name,
            &self.steps,
            self.stop_on_failure,
        ))
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
    cancel_token: Option<CancellationToken>,
    checkpoint_store: Option<Arc<checkpoint_store::CheckpointStore>>,
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
            cancel_token: None,
            checkpoint_store: None,
        }
    }

    /// Set a cancellation token for cooperative cancellation.
    fn set_cancel_token(&mut self, token: CancellationToken) {
        self.cancel_token = Some(token);
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

    /// Attach a checkpoint store for automatic save/resume.
    fn set_checkpoint_store(&mut self, store: checkpoint_store::CheckpointStore) {
        self.checkpoint_store = Some(Arc::new(store));
    }

    /// Generate a deterministic pipeline ID from the pipeline name and step names.
    fn pipeline_id(&self) -> String {
        let step_names: Vec<&str> = self.steps.iter().map(|s| s.name.as_str()).collect();
        format!("{}:{}", self.name, step_names.join(","))
    }

    /// Execute all steps asynchronously.
    ///
    /// Delegates to the sync Pipeline execution (which releases the GIL during
    /// I/O) and wraps the result in a PyFuture for async API compatibility.
    fn run(&self, py: Python<'_>, engine: &AsyncEngine) -> PyResult<PyFuture> {
        let sync_engine = Engine::new_inner(
            engine.state.scope.clone(),
            &engine.state.mode,
            engine.state.concurrency,
            engine.state.timeout_ms,
        )?;

        let mut pipeline = Pipeline {
            name: self.name.clone(),
            steps: self.steps.clone(),
            stop_on_failure: self.stop_on_failure,
            cancel_token: self.cancel_token.clone(),
            checkpoint_store: self.checkpoint_store.clone(),
        };

        let pipeline_result = pipeline.run(py, &sync_engine)?;
        runtime_async::spawn_async(async move { Ok(pipeline_result) })
    }

    /// Resume execution from a checkpoint, skipping completed steps.
    fn resume_from(
        &self,
        py: Python<'_>,
        engine: &AsyncEngine,
        checkpoint: crate::checkpoint::Checkpoint,
    ) -> PyResult<PyFuture> {
        let sync_engine = Engine::new_inner(
            engine.state.scope.clone(),
            &engine.state.mode,
            engine.state.concurrency,
            engine.state.timeout_ms,
        )?;

        let pipeline = Pipeline {
            name: self.name.clone(),
            steps: self.steps.clone(),
            stop_on_failure: self.stop_on_failure,
            cancel_token: self.cancel_token.clone(),
            checkpoint_store: self.checkpoint_store.clone(),
        };

        let pipeline_result = pipeline.resume_from(py, &sync_engine, checkpoint)?;
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
