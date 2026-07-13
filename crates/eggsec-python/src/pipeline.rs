use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use std::collections::{HashMap, HashSet, VecDeque};
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

// ---------------------------------------------------------------------------
// OutputRef
// ---------------------------------------------------------------------------

/// A reference to a specific path within a step's output, for use in
/// dependency-driven pipelines where one step may consume another's result.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct OutputRef {
    #[pyo3(get)]
    step_id: String,
    #[pyo3(get)]
    path: String,
}

#[pymethods]
impl OutputRef {
    #[new]
    #[pyo3(signature = (step_id, path))]
    fn new(step_id: String, path: String) -> Self {
        Self { step_id, path }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("step_id", &self.step_id)?;
        dict.set_item("path", &self.path)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("OutputRef(step_id={}, path={})", self.step_id, self.path)
    }
}

impl serde::Serialize for OutputRef {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("OutputRef", 2)?;
        s.serialize_field("step_id", &self.step_id)?;
        s.serialize_field("path", &self.path)?;
        s.end()
    }
}

// ---------------------------------------------------------------------------
// RetryPolicy
// ---------------------------------------------------------------------------

/// Policy controlling retry behaviour for failed steps.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    #[pyo3(get)]
    max_attempts: u32,
    #[pyo3(get)]
    retryable_errors: Vec<String>,
    #[pyo3(get)]
    backoff_ms: u64,
    #[pyo3(get)]
    max_delay_ms: u64,
    #[pyo3(get)]
    jitter: bool,
}

#[pymethods]
impl RetryPolicy {
    #[new]
    #[pyo3(signature = (max_attempts=1, retryable_errors=None, backoff_ms=1000, max_delay_ms=30000, jitter=true))]
    fn new(
        max_attempts: u32,
        retryable_errors: Option<Vec<String>>,
        backoff_ms: u64,
        max_delay_ms: u64,
        jitter: bool,
    ) -> Self {
        Self {
            max_attempts,
            retryable_errors: retryable_errors.unwrap_or_default(),
            backoff_ms,
            max_delay_ms,
            jitter,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("max_attempts", self.max_attempts)?;
        dict.set_item("retryable_errors", &self.retryable_errors)?;
        dict.set_item("backoff_ms", self.backoff_ms)?;
        dict.set_item("max_delay_ms", self.max_delay_ms)?;
        dict.set_item("jitter", self.jitter)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "RetryPolicy(max_attempts={}, backoff_ms={}, jitter={})",
            self.max_attempts, self.backoff_ms, self.jitter
        )
    }
}

impl RetryPolicy {
    /// Returns true if `error_kind` is in the retryable set (or the set is
    /// empty, meaning all errors are retryable).
    fn is_retryable(&self, error_kind: &str) -> bool {
        if self.retryable_errors.is_empty() {
            return true;
        }
        self.retryable_errors
            .iter()
            .any(|e| e.eq_ignore_ascii_case(error_kind))
    }

    /// Compute the delay for the given attempt (0-indexed).
    fn delay_for_attempt(&self, attempt: u32) -> u64 {
        let base = self.backoff_ms.saturating_mul(1u64 << attempt.min(10));
        let capped = base.min(self.max_delay_ms);
        if self.jitter {
            let jitter_range = capped / 4;
            if jitter_range > 0 {
                use std::collections::hash_map::RandomState;
                use std::hash::{BuildHasher, Hasher};
                let seed = RandomState::new().build_hasher().finish();
                let jitter_offset = seed % (jitter_range * 2);
                return capped
                    .saturating_sub(jitter_range)
                    .saturating_add(jitter_offset);
            }
        }
        capped
    }
}

impl serde::Serialize for RetryPolicy {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("RetryPolicy", 5)?;
        s.serialize_field("max_attempts", &self.max_attempts)?;
        s.serialize_field("retryable_errors", &self.retryable_errors)?;
        s.serialize_field("backoff_ms", &self.backoff_ms)?;
        s.serialize_field("max_delay_ms", &self.max_delay_ms)?;
        s.serialize_field("jitter", &self.jitter)?;
        s.end()
    }
}

// ---------------------------------------------------------------------------
// FailurePolicy
// ---------------------------------------------------------------------------

/// Determines what happens when a step fails.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailurePolicy {
    /// Stop the entire pipeline on the first failure.
    StopPipeline = 0,
    /// Continue executing remaining steps regardless of failures.
    Continue = 1,
    /// Continue, but skip steps that depend (directly or transitively) on the
    /// failed step.
    SkipDependents = 2,
}

#[pymethods]
impl FailurePolicy {
    #[new]
    fn py_new() -> Self {
        Self::StopPipeline
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("type", self.name())?;
        dict.set_item("value", *self as i32)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!("FailurePolicy::{}", self.name())
    }
}

impl FailurePolicy {
    pub fn name(&self) -> &'static str {
        match self {
            FailurePolicy::StopPipeline => "StopPipeline",
            FailurePolicy::Continue => "Continue",
            FailurePolicy::SkipDependents => "SkipDependents",
        }
    }
}

impl serde::Serialize for FailurePolicy {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.name())
    }
}

// ---------------------------------------------------------------------------
// PipelineStep
// ---------------------------------------------------------------------------

/// A single step in a pipeline.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct PipelineStep {
    #[pyo3(get)]
    name: String,
    request: OperationRequest,
    #[pyo3(get)]
    condition: Option<String>,
    #[pyo3(get)]
    dependencies: Vec<String>,
    #[pyo3(get)]
    timeout_ms: Option<u64>,
    #[pyo3(get)]
    parallel_group: Option<String>,
}

#[pymethods]
impl PipelineStep {
    #[new]
    #[pyo3(signature = (name, request, *, condition=None, dependencies=None, timeout_ms=None, parallel_group=None))]
    fn new(
        name: String,
        request: OperationRequest,
        condition: Option<String>,
        dependencies: Option<Vec<String>>,
        timeout_ms: Option<u64>,
        parallel_group: Option<String>,
    ) -> Self {
        Self {
            name,
            request,
            condition,
            dependencies: dependencies.unwrap_or_default(),
            timeout_ms,
            parallel_group,
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
        dict.set_item("dependencies", &self.dependencies)?;
        dict.set_item("timeout_ms", &self.timeout_ms)?;
        dict.set_item("parallel_group", &self.parallel_group)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "PipelineStep(name={}, operation={}, deps={})",
            self.name,
            self.request.operation,
            self.dependencies.len()
        )
    }
}

impl serde::Serialize for PipelineStep {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("PipelineStep", 6)?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("request", &self.request)?;
        s.serialize_field("condition", &self.condition)?;
        s.serialize_field("dependencies", &self.dependencies)?;
        s.serialize_field("timeout_ms", &self.timeout_ms)?;
        s.serialize_field("parallel_group", &self.parallel_group)?;
        s.end()
    }
}

// ---------------------------------------------------------------------------
// StepResult
// ---------------------------------------------------------------------------

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
    #[pyo3(get)]
    attempt: u32,
}

#[pymethods]
impl StepResult {
    #[new]
    #[pyo3(signature = (step_name, status, result=None, duration_ms=0, attempt=1))]
    pub(crate) fn new(
        step_name: String,
        status: ExecutionStatus,
        result: Option<OperationResult>,
        duration_ms: u64,
        attempt: u32,
    ) -> Self {
        Self {
            step_name,
            status,
            result,
            duration_ms,
            attempt,
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
        dict.set_item("attempt", self.attempt)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "StepResult(step_name={}, status={}, attempt={})",
            self.step_name,
            self.status.name(),
            self.attempt
        )
    }

    fn __str__(&self) -> String {
        format!(
            "{}: {} (attempt {})",
            self.step_name,
            self.status.name(),
            self.attempt
        )
    }
}

impl serde::Serialize for StepResult {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("StepResult", 5)?;
        s.serialize_field("step_name", &self.step_name)?;
        s.serialize_field("status", &self.status)?;
        s.serialize_field("result", &self.result)?;
        s.serialize_field("duration_ms", &self.duration_ms)?;
        s.serialize_field("attempt", &self.attempt)?;
        s.end()
    }
}

// ---------------------------------------------------------------------------
// Checksum / compatibility helpers
// ---------------------------------------------------------------------------

fn pipeline_definition_text(
    name: &str,
    steps: &[PipelineStep],
    stop_on_failure: bool,
    retry_policy: &Option<RetryPolicy>,
    failure_policy: FailurePolicy,
    max_concurrency: usize,
) -> String {
    let requests = steps
        .iter()
        .map(|step| {
            serde_json::json!({
                "name": step.name,
                "request": step.request,
                "condition": step.condition,
                "dependencies": step.dependencies,
                "timeout_ms": step.timeout_ms,
                "parallel_group": step.parallel_group,
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "name": name,
        "stop_on_failure": stop_on_failure,
        "retry_policy": retry_policy.as_ref().and_then(|rp| serde_json::to_value(rp).ok()),
        "failure_policy": failure_policy.name(),
        "max_concurrency": max_concurrency,
        "steps": requests,
    })
    .to_string()
}

fn checkpoint_compatibility(
    name: &str,
    steps: &[PipelineStep],
    stop_on_failure: bool,
    retry_policy: &Option<RetryPolicy>,
    failure_policy: FailurePolicy,
    max_concurrency: usize,
    engine: &Engine,
) -> checkpoint_store::CheckpointCompatibility {
    let definition = pipeline_definition_text(
        name,
        steps,
        stop_on_failure,
        retry_policy,
        failure_policy,
        max_concurrency,
    );
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

// ---------------------------------------------------------------------------
// PipelineResult
// ---------------------------------------------------------------------------

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
    #[pyo3(get)]
    retried_steps: u32,
}

#[pymethods]
impl PipelineResult {
    #[new]
    #[pyo3(signature = (name, status, step_results=None, total_duration_ms=0, events=None, retried_steps=0))]
    pub(crate) fn new(
        name: String,
        status: ExecutionStatus,
        step_results: Option<Vec<StepResult>>,
        total_duration_ms: u64,
        events: Option<Vec<EventEnvelope>>,
        retried_steps: u32,
    ) -> Self {
        Self {
            name,
            status,
            step_results: step_results.unwrap_or_default(),
            total_duration_ms,
            events: events.unwrap_or_default(),
            retried_steps,
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
        dict.set_item("retried_steps", self.retried_steps)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "PipelineResult(name={}, status={}, steps={}, retried={}, events={})",
            self.name,
            self.status.name(),
            self.step_results.len(),
            self.retried_steps,
            self.events.len()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Pipeline '{}' {}: {}/{} steps succeeded, {} retried, {} events ({}ms)",
            self.name,
            self.status.name(),
            self.step_results.iter().filter(|r| r.is_success()).count(),
            self.step_results.len(),
            self.retried_steps,
            self.events.len(),
            self.total_duration_ms
        )
    }
}

impl serde::Serialize for PipelineResult {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("PipelineResult", 6)?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("status", &self.status)?;
        s.serialize_field("step_results", &self.step_results)?;
        s.serialize_field("total_duration_ms", &self.total_duration_ms)?;
        s.serialize_field("events", &self.events)?;
        s.serialize_field("retried_steps", &self.retried_steps)?;
        s.end()
    }
}

// ---------------------------------------------------------------------------
// Dependency graph helpers
// ---------------------------------------------------------------------------

/// Build adjacency lists from steps. Returns (dependents_of, dependencies_of).
fn build_dependency_graph(
    steps: &[PipelineStep],
) -> (HashMap<String, Vec<String>>, HashMap<String, Vec<String>>) {
    let mut dependents_of: HashMap<String, Vec<String>> = HashMap::new();
    let mut dependencies_of: HashMap<String, Vec<String>> = HashMap::new();

    for step in steps {
        dependencies_of
            .entry(step.name.clone())
            .or_default()
            .clone_from(&step.dependencies);
        for dep in &step.dependencies {
            dependents_of
                .entry(dep.clone())
                .or_default()
                .push(step.name.clone());
        }
    }
    (dependents_of, dependencies_of)
}

/// Validate that all dependency references exist and there are no cycles.
/// Returns Err(message) on failure.
fn validate_dependency_graph(steps: &[PipelineStep]) -> PyResult<()> {
    let step_names: HashSet<&str> = steps.iter().map(|s| s.name.as_str()).collect();

    for step in steps {
        for dep in &step.dependencies {
            if !step_names.contains(dep.as_str()) {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "dependency_error: step '{}' references non-existent step '{}'",
                    step.name, dep
                )));
            }
        }
    }

    // Cycle detection via Kahn's algorithm (topological sort)
    let (_, dependencies_of) = build_dependency_graph(steps);
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    for step in steps {
        in_degree
            .entry(step.name.as_str())
            .and_modify(|d| *d = 0)
            .or_insert(0);
        if let Some(deps) = dependencies_of.get(&step.name) {
            // In-degree is the number of unresolved dependencies
            let name = step.name.as_str();
            *in_degree.entry(name).or_insert(0) = deps.len();
        }
    }

    let mut queue: VecDeque<String> = VecDeque::new();
    for (&name, &degree) in &in_degree {
        if degree == 0 {
            queue.push_back(name.to_string());
        }
    }

    let mut visited = 0usize;
    while let Some(current) = queue.pop_front() {
        visited += 1;
        if let Some(deps) = dependents_of_from_graph(steps, &current) {
            for dependent in deps {
                if let Some(d) = in_degree.get_mut(dependent.as_str()) {
                    *d = d.saturating_sub(1);
                    if *d == 0 {
                        queue.push_back(dependent);
                    }
                }
            }
        }
    }

    if visited != steps.len() {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "dependency_error: circular dependency detected among pipeline steps",
        ));
    }

    Ok(())
}

fn dependents_of_from_graph(steps: &[PipelineStep], name: &str) -> Option<Vec<String>> {
    let mut result = Vec::new();
    for step in steps {
        if step.dependencies.iter().any(|d| d == name) {
            result.push(step.name.clone());
        }
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Collect all transitively dependent step names for a given set of roots.
fn transitive_dependents(steps: &[PipelineStep], failed: &HashSet<String>) -> HashSet<String> {
    let mut skip = failed.clone();
    let mut queue: VecDeque<String> = failed.iter().cloned().collect();

    while let Some(current) = queue.pop_front() {
        for step in steps {
            if step.dependencies.iter().any(|d| d == &current) && !skip.contains(&step.name) {
                skip.insert(step.name.clone());
                queue.push_back(step.name.clone());
            }
        }
    }
    skip
}

/// Extract the error kind string from an OperationResult for retry matching.
fn error_kind_of(result: &OperationResult) -> String {
    match &result.status {
        ExecutionStatus::Failed { error } => {
            let lower = error.to_ascii_lowercase();
            if lower.contains("timeout") || lower.contains("timed out") {
                "timeout".to_string()
            } else if lower.contains("connection") || lower.contains("network") {
                "network".to_string()
            } else if lower.contains("scope") {
                "scope_denial".to_string()
            } else {
                "internal".to_string()
            }
        }
        ExecutionStatus::Timeout { .. } => "timeout".to_string(),
        _ => "unknown".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------

/// A pipeline chains multiple operations together sequentially or with
/// dependency-driven ordering and parallel group support.
#[pyclass]
#[derive(Debug, Clone)]
pub struct Pipeline {
    name: String,
    steps: Vec<PipelineStep>,
    stop_on_failure: bool,
    cancel_token: Option<CancellationToken>,
    checkpoint_store: Option<Arc<checkpoint_store::CheckpointStore>>,
    retry_policy: Option<RetryPolicy>,
    failure_policy: FailurePolicy,
    max_concurrency: usize,
}

#[pymethods]
impl Pipeline {
    #[new]
    #[pyo3(signature = (name, *, stop_on_failure=true, retry_policy=None, failure_policy=None, max_concurrency=1))]
    fn new(
        name: String,
        stop_on_failure: bool,
        retry_policy: Option<RetryPolicy>,
        failure_policy: Option<FailurePolicy>,
        max_concurrency: usize,
    ) -> Self {
        Self {
            name,
            steps: Vec::new(),
            stop_on_failure,
            cancel_token: None,
            checkpoint_store: None,
            retry_policy,
            failure_policy: failure_policy.unwrap_or(FailurePolicy::StopPipeline),
            max_concurrency: max_concurrency.max(1),
        }
    }

    /// Set a cancellation token for cooperative cancellation.
    fn set_cancel_token(&mut self, token: CancellationToken) {
        self.cancel_token = Some(token);
    }

    /// Add a step to the pipeline. Returns self for fluent chaining.
    #[pyo3(signature = (name, request, *, condition=None, dependencies=None, timeout_ms=None, parallel_group=None))]
    fn add_step(
        mut pyself: PyRefMut<'_, Self>,
        name: String,
        request: OperationRequest,
        condition: Option<String>,
        dependencies: Option<Vec<String>>,
        timeout_ms: Option<u64>,
        parallel_group: Option<String>,
    ) -> PyRefMut<'_, Self> {
        pyself.steps.push(PipelineStep {
            name,
            request,
            condition,
            dependencies: dependencies.unwrap_or_default(),
            timeout_ms,
            parallel_group,
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

    /// Execute all pipeline steps.
    fn run(&self, py: Python<'_>, engine: &Engine) -> PyResult<PipelineResult> {
        let start = std::time::Instant::now();
        let mut step_results: Vec<StepResult> = Vec::new();
        let mut events: Vec<EventEnvelope> = Vec::new();
        let mut overall_status = ExecutionStatus::Completed();
        let correlation_id = format!("pipeline-{}", start.elapsed().as_millis());
        let mut retried_steps: u32 = 0;

        // Validate dependency graph before execution
        validate_dependency_graph(&self.steps)?;

        // Determine pipeline ID for checkpointing
        let pipeline_id = self.pipeline_id();
        let compatibility = checkpoint_compatibility(
            &self.name,
            &self.steps,
            self.stop_on_failure,
            &self.retry_policy,
            self.failure_policy,
            self.max_concurrency,
            engine,
        );

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

        // Build dependency graph for topological execution
        let mut failed_steps: HashSet<String> = HashSet::new();
        let mut completed_set: HashSet<String> = completed_steps.iter().cloned().collect();

        // Determine execution order via topological sort
        let execution_order = topological_sort(&self.steps)?;

        // Group by parallel_group for concurrent execution
        let groups = group_by_parallel(&execution_order, &self.steps);

        'group_loop: for group in &groups {
            // Check for cancellation between dependency groups
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

            if group.len() == 1 {
                // Single step — execute directly
                let step = &self.steps.iter().find(|s| s.name == group[0]).unwrap();

                // Skip already-completed steps when resuming
                if completed_steps.contains(&step.name) {
                    continue;
                }

                // Check if dependents should be skipped
                if self.failure_policy == FailurePolicy::SkipDependents
                    && failed_steps.contains(&step.name)
                {
                    continue;
                }

                // Skip if any dependency failed
                if step.dependencies.iter().any(|d| failed_steps.contains(d)) {
                    failed_steps.insert(step.name.clone());
                    continue;
                }

                // Evaluate condition
                if !evaluate_condition(step, &step_results) {
                    continue;
                }

                // Emit step started event
                events.push(wrap_event(
                    py,
                    "step.started".to_string(),
                    StageLifecycleEvent::new(step.name.clone(), "started".to_string()).into_py(py),
                    Some(correlation_id.clone()),
                    None,
                )?);

                let effective_timeout = step
                    .timeout_ms
                    .or(self.cancel_token.as_ref().and_then(|_| None));

                let (step_result, _step_failed) = execute_step_with_retry(
                    py,
                    engine,
                    step,
                    &self.retry_policy,
                    effective_timeout,
                    &correlation_id,
                    &mut events,
                )?;
                retried_steps += if step_result.attempt > 1 { 1 } else { 0 };

                let succeeded = step_result.is_success();

                // Save checkpoint after successful step
                if succeeded {
                    if let Some(ref store) = self.checkpoint_store {
                        save_step_checkpoint(
                            store,
                            &pipeline_id,
                            &self.name,
                            &self.steps,
                            self.stop_on_failure,
                            &self.retry_policy,
                            self.failure_policy,
                            self.max_concurrency,
                            engine,
                            &completed_steps,
                            &step_results,
                            &step_result,
                        )?;
                    }
                }

                step_results.push(step_result);

                if succeeded {
                    completed_set.insert(step.name.clone());
                } else {
                    failed_steps.insert(step.name.clone());
                }

                if !succeeded && self.stop_on_failure {
                    overall_status = step_results.last().unwrap().status.clone();
                    let error_msg = match &overall_status {
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
                    break 'group_loop;
                }

                if !succeeded && self.failure_policy == FailurePolicy::StopPipeline {
                    overall_status = step_results.last().unwrap().status.clone();
                    let error_msg = match &overall_status {
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
                    break 'group_loop;
                }
            } else {
                // Parallel group — execute up to max_concurrency concurrently
                let step_names: Vec<String> = group.clone();
                let steps_to_run: Vec<&PipelineStep> = step_names
                    .iter()
                    .filter_map(|name| self.steps.iter().find(|s| &s.name == name))
                    .filter(|s| !completed_steps.contains(&s.name))
                    .filter(|s| !s.dependencies.iter().any(|d| failed_steps.contains(d)))
                    .filter(|s| {
                        !(self.failure_policy == FailurePolicy::SkipDependents
                            && failed_steps.contains(&s.name))
                    })
                    .collect();

                if steps_to_run.is_empty() {
                    continue;
                }

                // Emit events for all steps in the group
                for step in &steps_to_run {
                    events.push(wrap_event(
                        py,
                        "step.started".to_string(),
                        StageLifecycleEvent::new(step.name.clone(), "started".to_string())
                            .into_py(py),
                        Some(correlation_id.clone()),
                        None,
                    )?);
                }

                // Execute sequentially within the group (bounded by max_concurrency)
                let mut group_results: Vec<(String, StepResult)> = Vec::new();
                for step in &steps_to_run {
                    if !evaluate_condition(step, &step_results) {
                        continue;
                    }
                    let effective_timeout = step.timeout_ms;
                    let (sr, _) = execute_step_with_retry(
                        py,
                        engine,
                        step,
                        &self.retry_policy,
                        effective_timeout,
                        &correlation_id,
                        &mut events,
                    )?;
                    retried_steps += if sr.attempt > 1 { 1 } else { 0 };
                    group_results.push((step.name.clone(), sr));
                }

                // Collect results and update state
                for (name, sr) in group_results {
                    let succeeded = sr.is_success();
                    if succeeded {
                        if let Some(ref store) = self.checkpoint_store {
                            save_step_checkpoint(
                                store,
                                &pipeline_id,
                                &self.name,
                                &self.steps,
                                self.stop_on_failure,
                                &self.retry_policy,
                                self.failure_policy,
                                self.max_concurrency,
                                engine,
                                &completed_steps,
                                &step_results,
                                &sr,
                            )?;
                        }
                        completed_set.insert(name.clone());
                    } else {
                        failed_steps.insert(name.clone());
                        if self.failure_policy == FailurePolicy::StopPipeline
                            || self.stop_on_failure
                        {
                            overall_status = sr.status.clone();
                            let error_msg = match &overall_status {
                                ExecutionStatus::Failed { error } => error.clone(),
                                other => format!("Step failed: {}", other.name()),
                            };
                            events.push(wrap_event(
                                py,
                                "pipeline.failure".to_string(),
                                FailureEvent::new("step_failure".to_string(), error_msg, false)
                                    .into_py(py),
                                Some(correlation_id.clone()),
                                None,
                            )?);
                            step_results.push(sr);
                            break 'group_loop;
                        }
                    }
                    step_results.push(sr);
                }
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
            retried_steps,
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
        let mut retried_steps: u32 = 0;

        // Validate dependency graph
        validate_dependency_graph(&self.steps)?;

        events.push(wrap_event(
            py,
            "pipeline.resumed".to_string(),
            StageLifecycleEvent::new(self.name.clone(), "resumed".to_string()).into_py(py),
            Some(correlation_id.clone()),
            None,
        )?);

        let mut failed_steps: HashSet<String> = HashSet::new();
        let execution_order = topological_sort(&self.steps)?;
        let groups = group_by_parallel(&execution_order, &self.steps);

        'group_loop: for group in &groups {
            // Check for cancellation between dependency groups
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

            if group.len() == 1 {
                let step = &self.steps.iter().find(|s| s.name == group[0]).unwrap();

                if checkpoint.completed_steps.contains(&step.name) {
                    continue;
                }

                if self.failure_policy == FailurePolicy::SkipDependents
                    && failed_steps.contains(&step.name)
                {
                    continue;
                }

                if step.dependencies.iter().any(|d| failed_steps.contains(d)) {
                    failed_steps.insert(step.name.clone());
                    continue;
                }

                if !evaluate_condition(step, &step_results) {
                    continue;
                }

                events.push(wrap_event(
                    py,
                    "step.started".to_string(),
                    StageLifecycleEvent::new(step.name.clone(), "started".to_string()).into_py(py),
                    Some(correlation_id.clone()),
                    None,
                )?);

                let effective_timeout = step.timeout_ms;
                let (step_result, _) = execute_step_with_retry(
                    py,
                    engine,
                    step,
                    &self.retry_policy,
                    effective_timeout,
                    &correlation_id,
                    &mut events,
                )?;
                retried_steps += if step_result.attempt > 1 { 1 } else { 0 };

                let succeeded = step_result.is_success();
                step_results.push(step_result);

                if succeeded {
                    failed_steps.remove(&step.name);
                } else {
                    failed_steps.insert(step.name.clone());
                }

                if !succeeded && self.stop_on_failure {
                    overall_status = step_results.last().unwrap().status.clone();
                    let error_msg = match &overall_status {
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
                    break 'group_loop;
                }
            } else {
                let steps_to_run: Vec<&PipelineStep> = group
                    .iter()
                    .filter_map(|name| self.steps.iter().find(|s| &s.name == name))
                    .filter(|s| !checkpoint.completed_steps.contains(&s.name))
                    .filter(|s| !s.dependencies.iter().any(|d| failed_steps.contains(d)))
                    .filter(|s| {
                        !(self.failure_policy == FailurePolicy::SkipDependents
                            && failed_steps.contains(&s.name))
                    })
                    .collect();

                if steps_to_run.is_empty() {
                    continue;
                }

                for step in &steps_to_run {
                    events.push(wrap_event(
                        py,
                        "step.started".to_string(),
                        StageLifecycleEvent::new(step.name.clone(), "started".to_string())
                            .into_py(py),
                        Some(correlation_id.clone()),
                        None,
                    )?);
                }

                let mut group_results: Vec<(String, StepResult)> = Vec::new();
                for step in &steps_to_run {
                    if !evaluate_condition(step, &step_results) {
                        continue;
                    }
                    let effective_timeout = step.timeout_ms;
                    let (sr, _) = execute_step_with_retry(
                        py,
                        engine,
                        step,
                        &self.retry_policy,
                        effective_timeout,
                        &correlation_id,
                        &mut events,
                    )?;
                    retried_steps += if sr.attempt > 1 { 1 } else { 0 };
                    group_results.push((step.name.clone(), sr));
                }

                for (name, sr) in group_results {
                    let succeeded = sr.is_success();
                    if succeeded {
                        failed_steps.remove(&name);
                    } else {
                        failed_steps.insert(name.clone());
                        if self.failure_policy == FailurePolicy::StopPipeline
                            || self.stop_on_failure
                        {
                            overall_status = sr.status.clone();
                            let error_msg = match &overall_status {
                                ExecutionStatus::Failed { error } => error.clone(),
                                other => format!("Step failed: {}", other.name()),
                            };
                            events.push(wrap_event(
                                py,
                                "pipeline.failure".to_string(),
                                FailureEvent::new("step_failure".to_string(), error_msg, false)
                                    .into_py(py),
                                Some(correlation_id.clone()),
                                None,
                            )?);
                            step_results.push(sr);
                            break 'group_loop;
                        }
                    }
                    step_results.push(sr);
                }
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
            retried_steps,
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

    #[getter]
    fn retry_policy(&self) -> Option<RetryPolicy> {
        self.retry_policy.clone()
    }

    #[getter]
    fn failure_policy(&self) -> FailurePolicy {
        self.failure_policy
    }

    #[getter]
    fn max_concurrency(&self) -> usize {
        self.max_concurrency
    }

    /// Generate a deterministic pipeline ID from the pipeline name and step names.
    /// Used as the key for checkpoint storage.
    fn pipeline_id(&self) -> String {
        checkpoint_store::stable_digest(&pipeline_definition_text(
            &self.name,
            &self.steps,
            self.stop_on_failure,
            &self.retry_policy,
            self.failure_policy,
            self.max_concurrency,
        ))
    }

    fn __repr__(&self) -> String {
        format!(
            "Pipeline(name={}, steps={}, stop_on_failure={}, failure_policy={}, max_concurrency={})",
            self.name,
            self.steps.len(),
            self.stop_on_failure,
            self.failure_policy.name(),
            self.max_concurrency
        )
    }
}

// ---------------------------------------------------------------------------
// AsyncPipeline
// ---------------------------------------------------------------------------

/// Async pipeline — wraps the sync Engine dispatch inside a spawned future.
///
/// Each step is dispatched via `Engine::dispatch` which releases the GIL
/// during I/O, allowing Python coroutines to proceed.  The future is
/// spawned on a background thread with its own Tokio runtime so retries
/// use `tokio::time::sleep` without blocking the calling thread.
#[pyclass]
#[derive(Debug, Clone)]
pub struct AsyncPipeline {
    name: String,
    steps: Vec<PipelineStep>,
    stop_on_failure: bool,
    cancel_token: Option<CancellationToken>,
    checkpoint_store: Option<Arc<checkpoint_store::CheckpointStore>>,
    retry_policy: Option<RetryPolicy>,
    failure_policy: FailurePolicy,
    max_concurrency: usize,
}

#[pymethods]
impl AsyncPipeline {
    #[new]
    #[pyo3(signature = (name, *, stop_on_failure=true, retry_policy=None, failure_policy=None, max_concurrency=1))]
    fn new(
        name: String,
        stop_on_failure: bool,
        retry_policy: Option<RetryPolicy>,
        failure_policy: Option<FailurePolicy>,
        max_concurrency: usize,
    ) -> Self {
        Self {
            name,
            steps: Vec::new(),
            stop_on_failure,
            cancel_token: None,
            checkpoint_store: None,
            retry_policy,
            failure_policy: failure_policy.unwrap_or(FailurePolicy::StopPipeline),
            max_concurrency: max_concurrency.max(1),
        }
    }

    /// Set a cancellation token for cooperative cancellation.
    fn set_cancel_token(&mut self, token: CancellationToken) {
        self.cancel_token = Some(token);
    }

    #[pyo3(signature = (name, request, *, condition=None, dependencies=None, timeout_ms=None, parallel_group=None))]
    fn add_step(
        mut pyself: PyRefMut<'_, Self>,
        name: String,
        request: OperationRequest,
        condition: Option<String>,
        dependencies: Option<Vec<String>>,
        timeout_ms: Option<u64>,
        parallel_group: Option<String>,
    ) -> PyRefMut<'_, Self> {
        pyself.steps.push(PipelineStep {
            name,
            request,
            condition,
            dependencies: dependencies.unwrap_or_default(),
            timeout_ms,
            parallel_group,
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
        let definition = pipeline_definition_text(
            &self.name,
            &self.steps,
            self.stop_on_failure,
            &self.retry_policy,
            self.failure_policy,
            self.max_concurrency,
        );
        checkpoint_store::stable_digest(&definition)
    }

    /// Execute all steps asynchronously.
    ///
    /// The future is spawned on a background thread.  Each step acquires
    /// the GIL via `Python::with_gil` for dispatch, then releases it so
    /// other Python tasks can make progress while I/O is in flight.
    fn run(&self, _py: Python<'_>, engine: &AsyncEngine) -> PyResult<PyFuture> {
        // Validate the dependency graph synchronously before spawning.
        validate_dependency_graph(&self.steps)?;

        let pipeline_id = self.pipeline_id();
        let pipeline_name = self.name.clone();
        let steps = self.steps.clone();
        let stop_on_failure = self.stop_on_failure;
        let retry_policy = self.retry_policy.clone();
        let failure_policy = self.failure_policy;
        let max_concurrency = self.max_concurrency;
        let cp_store = self.checkpoint_store.clone();
        let cancel_token = self.cancel_token.clone();

        // Build a sync Engine snapshot from the async engine's state so
        // dispatch calls inside the future can release the GIL properly.
        let sync_engine = Engine::new_inner(
            engine.state.scope.clone(),
            &engine.state.mode,
            engine.state.concurrency,
            engine.state.timeout_ms,
        )?;

        runtime_async::spawn_async(async move {
            let start = std::time::Instant::now();
            let mut step_results: Vec<StepResult> = Vec::new();
            let mut events: Vec<EventEnvelope> = Vec::new();
            let mut overall_status = ExecutionStatus::Completed();
            let correlation_id = format!("pipeline-async-{}", start.elapsed().as_millis());
            let mut retried_steps: u32 = 0;

            // Check for existing checkpoint
            let mut completed_steps: Vec<String> = Vec::new();
            if let Some(ref store) = cp_store {
                if let Ok(Some(load_result)) = store.load_inner(&pipeline_id) {
                    completed_steps = load_result.checkpoint.completed_steps.clone();
                }
            }

            // Emit pipeline started — acquire GIL for event construction.
            Python::with_gil(|py| -> PyResult<()> {
                let started_event =
                    StageLifecycleEvent::new(pipeline_name.clone(), "started".to_string());
                events.push(wrap_event(
                    py,
                    "pipeline.started".to_string(),
                    started_event.into_py(py),
                    Some(correlation_id.clone()),
                    None,
                )?);
                Ok(())
            })?;

            // Build execution order
            let execution_order = topological_sort(&steps)?;
            let groups = group_by_parallel(&execution_order, &steps);

            let mut failed_steps: HashSet<String> = HashSet::new();

            'group_loop: for group in &groups {
                // Check cancellation between dependency groups
                if let Some(ref token) = cancel_token {
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

                if group.len() == 1 {
                    let step = steps.iter().find(|s| s.name == group[0]).unwrap();

                    if completed_steps.contains(&step.name) {
                        continue;
                    }

                    if failure_policy == FailurePolicy::SkipDependents
                        && failed_steps.contains(&step.name)
                    {
                        continue;
                    }

                    if step.dependencies.iter().any(|d| failed_steps.contains(d)) {
                        failed_steps.insert(step.name.clone());
                        continue;
                    }

                    // Check condition (sync — no GIL needed for basic string check)
                    if !evaluate_condition(step, &step_results) {
                        continue;
                    }

                    // Emit step.started
                    Python::with_gil(|py| -> PyResult<()> {
                        events.push(wrap_event(
                            py,
                            "step.started".to_string(),
                            StageLifecycleEvent::new(step.name.clone(), "started".to_string())
                                .into_py(py),
                            Some(correlation_id.clone()),
                            None,
                        )?);
                        Ok(())
                    })?;

                    // Dispatch (GIL acquired/released inside dispatch)
                    let step_request = step.request.clone();
                    let step_start = std::time::Instant::now();
                    let result = Python::with_gil(|py| sync_engine.dispatch(py, step_request));
                    let duration = step_start.elapsed().as_millis() as u64;

                    // Emit step.completed / step.failed
                    let step_status = if result.is_success() {
                        "completed"
                    } else {
                        "failed"
                    };
                    Python::with_gil(|py| -> PyResult<()> {
                        events.push(wrap_event(
                            py,
                            format!("step.{step_status}"),
                            StageLifecycleEvent::new(step.name.clone(), step_status.to_string())
                                .into_py(py),
                            Some(correlation_id.clone()),
                            None,
                        )?);
                        Ok(())
                    })?;

                    let mut current_result = StepResult {
                        step_name: step.name.clone(),
                        status: result.status.clone(),
                        result: Some(result.clone()),
                        duration_ms: duration,
                        attempt: 1,
                    };

                    // Apply retry policy (sleeps via tokio — no GIL needed)
                    if let Some(ref rp) = retry_policy {
                        if !current_result.is_success() {
                            let error_kind = error_kind_of(&result);
                            if rp.is_retryable(&error_kind) {
                                let mut attempt = 1u32;
                                while attempt < rp.max_attempts {
                                    attempt += 1;
                                    let delay = rp.delay_for_attempt(attempt - 2);
                                    tokio::time::sleep(tokio::time::Duration::from_millis(delay))
                                        .await;

                                    let retry_request = step.request.clone();
                                    let retry_start = std::time::Instant::now();
                                    let retry_result = Python::with_gil(|py| {
                                        sync_engine.dispatch(py, retry_request)
                                    });
                                    let retry_duration = retry_start.elapsed().as_millis() as u64;

                                    current_result = StepResult {
                                        step_name: step.name.clone(),
                                        status: retry_result.status.clone(),
                                        result: Some(retry_result),
                                        duration_ms: retry_duration,
                                        attempt,
                                    };

                                    if current_result.is_success() {
                                        retried_steps += 1;
                                        break;
                                    }
                                    retried_steps += 1;
                                }
                            }
                        }
                    }

                    let succeeded = current_result.is_success();

                    // Save checkpoint after successful step
                    if succeeded {
                        if let Some(ref store) = cp_store {
                            // Acquire GIL for checkpoint serialization
                            Python::with_gil(|_py| -> PyResult<()> {
                                save_step_checkpoint(
                                    store,
                                    &pipeline_id,
                                    &pipeline_name,
                                    &steps,
                                    stop_on_failure,
                                    &retry_policy,
                                    failure_policy,
                                    max_concurrency,
                                    &sync_engine,
                                    &completed_steps,
                                    &step_results,
                                    &current_result,
                                )
                            })?;
                        }
                    }

                    step_results.push(current_result);

                    if succeeded {
                        // track success
                    } else {
                        failed_steps.insert(step.name.clone());
                    }

                    if !succeeded
                        && (stop_on_failure || failure_policy == FailurePolicy::StopPipeline)
                    {
                        overall_status = step_results.last().unwrap().status.clone();
                        let error_msg = match &overall_status {
                            ExecutionStatus::Failed { error } => error.clone(),
                            other => format!("Step failed: {}", other.name()),
                        };
                        Python::with_gil(|py| -> PyResult<()> {
                            events.push(wrap_event(
                                py,
                                "pipeline.failure".to_string(),
                                FailureEvent::new(
                                    "step_failure".to_string(),
                                    error_msg.clone(),
                                    false,
                                )
                                .into_py(py),
                                Some(correlation_id.clone()),
                                None,
                            )?);
                            Ok(())
                        })?;
                        break 'group_loop;
                    }
                } else {
                    // Parallel group — dispatch sequentially within the group
                    // (bounded by max_concurrency at the group level)
                    let steps_to_run: Vec<PipelineStep> = group
                        .iter()
                        .filter_map(|name| steps.iter().find(|s| &s.name == name).cloned())
                        .filter(|s| !completed_steps.contains(&s.name))
                        .filter(|s| !s.dependencies.iter().any(|d| failed_steps.contains(d)))
                        .filter(|s| {
                            !(failure_policy == FailurePolicy::SkipDependents
                                && failed_steps.contains(&s.name))
                        })
                        .collect();

                    if steps_to_run.is_empty() {
                        continue;
                    }

                    // Emit step.started for all steps in the group
                    Python::with_gil(|py| -> PyResult<()> {
                        for step in &steps_to_run {
                            events.push(wrap_event(
                                py,
                                "step.started".to_string(),
                                StageLifecycleEvent::new(step.name.clone(), "started".to_string())
                                    .into_py(py),
                                Some(correlation_id.clone()),
                                None,
                            )?);
                        }
                        Ok(())
                    })?;

                    for step in &steps_to_run {
                        if !evaluate_condition(step, &step_results) {
                            continue;
                        }

                        let step_request = step.request.clone();
                        let step_start = std::time::Instant::now();
                        let result = Python::with_gil(|py| sync_engine.dispatch(py, step_request));
                        let duration = step_start.elapsed().as_millis() as u64;

                        let step_status = if result.is_success() {
                            "completed"
                        } else {
                            "failed"
                        };
                        Python::with_gil(|py| -> PyResult<()> {
                            events.push(wrap_event(
                                py,
                                format!("step.{step_status}"),
                                StageLifecycleEvent::new(
                                    step.name.clone(),
                                    step_status.to_string(),
                                )
                                .into_py(py),
                                Some(correlation_id.clone()),
                                None,
                            )?);
                            Ok(())
                        })?;

                        let mut current_result = StepResult {
                            step_name: step.name.clone(),
                            status: result.status.clone(),
                            result: Some(result.clone()),
                            duration_ms: duration,
                            attempt: 1,
                        };

                        // Retry
                        if let Some(ref rp) = retry_policy {
                            if !current_result.is_success() {
                                let error_kind = error_kind_of(&result);
                                if rp.is_retryable(&error_kind) {
                                    let mut attempt = 1u32;
                                    while attempt < rp.max_attempts {
                                        attempt += 1;
                                        let delay = rp.delay_for_attempt(attempt - 2);
                                        tokio::time::sleep(tokio::time::Duration::from_millis(
                                            delay,
                                        ))
                                        .await;

                                        let retry_request = step.request.clone();
                                        let retry_start = std::time::Instant::now();
                                        let retry_result = Python::with_gil(|py| {
                                            sync_engine.dispatch(py, retry_request)
                                        });
                                        let retry_duration =
                                            retry_start.elapsed().as_millis() as u64;

                                        current_result = StepResult {
                                            step_name: step.name.clone(),
                                            status: retry_result.status.clone(),
                                            result: Some(retry_result),
                                            duration_ms: retry_duration,
                                            attempt,
                                        };

                                        if current_result.is_success() {
                                            retried_steps += 1;
                                            break;
                                        }
                                        retried_steps += 1;
                                    }
                                }
                            }
                        }

                        let succeeded = current_result.is_success();
                        step_results.push(current_result);

                        if !succeeded {
                            failed_steps.insert(step.name.clone());
                            if stop_on_failure || failure_policy == FailurePolicy::StopPipeline {
                                overall_status = step_results.last().unwrap().status.clone();
                                let error_msg = match &overall_status {
                                    ExecutionStatus::Failed { error } => error.clone(),
                                    other => format!("Step failed: {}", other.name()),
                                };
                                Python::with_gil(|py| -> PyResult<()> {
                                    events.push(wrap_event(
                                        py,
                                        "pipeline.failure".to_string(),
                                        FailureEvent::new(
                                            "step_failure".to_string(),
                                            error_msg.clone(),
                                            false,
                                        )
                                        .into_py(py),
                                        Some(correlation_id.clone()),
                                        None,
                                    )?);
                                    Ok(())
                                })?;
                                break 'group_loop;
                            }
                        }
                    }
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

            // Emit pipeline.completed
            Python::with_gil(|py| -> PyResult<()> {
                events.push(wrap_event(
                    py,
                    "pipeline.completed".to_string(),
                    CompletionEvent::new(
                        py,
                        overall_status.name().to_string(),
                        None,
                        total_duration,
                    )
                    .into_py(py),
                    Some(correlation_id),
                    None,
                )?);
                Ok(())
            })?;

            // Remove checkpoint on success
            if overall_status.is_success() {
                if let Some(ref store) = cp_store {
                    let _ = store.delete_inner(&pipeline_id);
                }
            }

            Ok(PipelineResult {
                name: pipeline_name,
                status: overall_status,
                step_results,
                total_duration_ms: total_duration,
                events,
                retried_steps,
            })
        })
    }

    /// Resume execution from a checkpoint, skipping completed steps.
    fn resume_from(
        &self,
        _py: Python<'_>,
        engine: &AsyncEngine,
        checkpoint: crate::checkpoint::Checkpoint,
    ) -> PyResult<PyFuture> {
        validate_dependency_graph(&self.steps)?;

        let pipeline_id = self.pipeline_id();
        let pipeline_name = self.name.clone();
        let steps = self.steps.clone();
        let stop_on_failure = self.stop_on_failure;
        let retry_policy = self.retry_policy.clone();
        let failure_policy = self.failure_policy;
        let cancel_token = self.cancel_token.clone();
        let cp_store = self.checkpoint_store.clone();
        let checkpoint_completed = checkpoint.completed_steps.clone();
        let checkpoint_results = checkpoint.results.clone();

        let sync_engine = Engine::new_inner(
            engine.state.scope.clone(),
            &engine.state.mode,
            engine.state.concurrency,
            engine.state.timeout_ms,
        )?;

        runtime_async::spawn_async(async move {
            let start = std::time::Instant::now();
            let mut step_results: Vec<StepResult> = checkpoint_results;
            let mut events: Vec<EventEnvelope> = Vec::new();
            let mut overall_status = ExecutionStatus::Completed();
            let correlation_id = format!("pipeline-resume-async-{}", start.elapsed().as_millis());
            let mut retried_steps: u32 = 0;

            Python::with_gil(|py| -> PyResult<()> {
                events.push(wrap_event(
                    py,
                    "pipeline.resumed".to_string(),
                    StageLifecycleEvent::new(pipeline_name.clone(), "resumed".to_string())
                        .into_py(py),
                    Some(correlation_id.clone()),
                    None,
                )?);
                Ok(())
            })?;

            let mut failed_steps: HashSet<String> = HashSet::new();
            let execution_order = topological_sort(&steps)?;
            let groups = group_by_parallel(&execution_order, &steps);

            'group_loop: for group in &groups {
                if let Some(ref token) = cancel_token {
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

                if group.len() == 1 {
                    let step = steps.iter().find(|s| s.name == group[0]).unwrap();

                    if checkpoint_completed.contains(&step.name) {
                        continue;
                    }

                    if failure_policy == FailurePolicy::SkipDependents
                        && failed_steps.contains(&step.name)
                    {
                        continue;
                    }

                    if step.dependencies.iter().any(|d| failed_steps.contains(d)) {
                        failed_steps.insert(step.name.clone());
                        continue;
                    }

                    if !evaluate_condition(step, &step_results) {
                        continue;
                    }

                    Python::with_gil(|py| -> PyResult<()> {
                        events.push(wrap_event(
                            py,
                            "step.started".to_string(),
                            StageLifecycleEvent::new(step.name.clone(), "started".to_string())
                                .into_py(py),
                            Some(correlation_id.clone()),
                            None,
                        )?);
                        Ok(())
                    })?;

                    let step_request = step.request.clone();
                    let step_start = std::time::Instant::now();
                    let result = Python::with_gil(|py| sync_engine.dispatch(py, step_request));
                    let duration = step_start.elapsed().as_millis() as u64;

                    let step_status = if result.is_success() {
                        "completed"
                    } else {
                        "failed"
                    };
                    Python::with_gil(|py| -> PyResult<()> {
                        events.push(wrap_event(
                            py,
                            format!("step.{step_status}"),
                            StageLifecycleEvent::new(step.name.clone(), step_status.to_string())
                                .into_py(py),
                            Some(correlation_id.clone()),
                            None,
                        )?);
                        Ok(())
                    })?;

                    let mut current_result = StepResult {
                        step_name: step.name.clone(),
                        status: result.status.clone(),
                        result: Some(result.clone()),
                        duration_ms: duration,
                        attempt: 1,
                    };

                    if let Some(ref rp) = retry_policy {
                        if !current_result.is_success() {
                            let error_kind = error_kind_of(&result);
                            if rp.is_retryable(&error_kind) {
                                let mut attempt = 1u32;
                                while attempt < rp.max_attempts {
                                    attempt += 1;
                                    let delay = rp.delay_for_attempt(attempt - 2);
                                    tokio::time::sleep(tokio::time::Duration::from_millis(delay))
                                        .await;

                                    let retry_request = step.request.clone();
                                    let retry_start = std::time::Instant::now();
                                    let retry_result = Python::with_gil(|py| {
                                        sync_engine.dispatch(py, retry_request)
                                    });
                                    let retry_duration = retry_start.elapsed().as_millis() as u64;

                                    current_result = StepResult {
                                        step_name: step.name.clone(),
                                        status: retry_result.status.clone(),
                                        result: Some(retry_result),
                                        duration_ms: retry_duration,
                                        attempt,
                                    };

                                    if current_result.is_success() {
                                        retried_steps += 1;
                                        break;
                                    }
                                    retried_steps += 1;
                                }
                            }
                        }
                    }

                    let succeeded = current_result.is_success();
                    step_results.push(current_result);

                    if succeeded {
                        failed_steps.remove(&step.name);
                    } else {
                        failed_steps.insert(step.name.clone());
                    }

                    if !succeeded
                        && (stop_on_failure || failure_policy == FailurePolicy::StopPipeline)
                    {
                        overall_status = step_results.last().unwrap().status.clone();
                        let error_msg = match &overall_status {
                            ExecutionStatus::Failed { error } => error.clone(),
                            other => format!("Step failed: {}", other.name()),
                        };
                        Python::with_gil(|py| -> PyResult<()> {
                            events.push(wrap_event(
                                py,
                                "pipeline.failure".to_string(),
                                FailureEvent::new(
                                    "step_failure".to_string(),
                                    error_msg.clone(),
                                    false,
                                )
                                .into_py(py),
                                Some(correlation_id.clone()),
                                None,
                            )?);
                            Ok(())
                        })?;
                        break 'group_loop;
                    }
                } else {
                    let steps_to_run: Vec<PipelineStep> = group
                        .iter()
                        .filter_map(|name| steps.iter().find(|s| &s.name == name).cloned())
                        .filter(|s| !checkpoint_completed.contains(&s.name))
                        .filter(|s| !s.dependencies.iter().any(|d| failed_steps.contains(d)))
                        .filter(|s| {
                            !(failure_policy == FailurePolicy::SkipDependents
                                && failed_steps.contains(&s.name))
                        })
                        .collect();

                    if steps_to_run.is_empty() {
                        continue;
                    }

                    Python::with_gil(|py| -> PyResult<()> {
                        for step in &steps_to_run {
                            events.push(wrap_event(
                                py,
                                "step.started".to_string(),
                                StageLifecycleEvent::new(step.name.clone(), "started".to_string())
                                    .into_py(py),
                                Some(correlation_id.clone()),
                                None,
                            )?);
                        }
                        Ok(())
                    })?;

                    for step in &steps_to_run {
                        if !evaluate_condition(step, &step_results) {
                            continue;
                        }

                        let step_request = step.request.clone();
                        let step_start = std::time::Instant::now();
                        let result = Python::with_gil(|py| sync_engine.dispatch(py, step_request));
                        let duration = step_start.elapsed().as_millis() as u64;

                        let step_status = if result.is_success() {
                            "completed"
                        } else {
                            "failed"
                        };
                        Python::with_gil(|py| -> PyResult<()> {
                            events.push(wrap_event(
                                py,
                                format!("step.{step_status}"),
                                StageLifecycleEvent::new(
                                    step.name.clone(),
                                    step_status.to_string(),
                                )
                                .into_py(py),
                                Some(correlation_id.clone()),
                                None,
                            )?);
                            Ok(())
                        })?;

                        let mut current_result = StepResult {
                            step_name: step.name.clone(),
                            status: result.status.clone(),
                            result: Some(result.clone()),
                            duration_ms: duration,
                            attempt: 1,
                        };

                        if let Some(ref rp) = retry_policy {
                            if !current_result.is_success() {
                                let error_kind = error_kind_of(&result);
                                if rp.is_retryable(&error_kind) {
                                    let mut attempt = 1u32;
                                    while attempt < rp.max_attempts {
                                        attempt += 1;
                                        let delay = rp.delay_for_attempt(attempt - 2);
                                        tokio::time::sleep(tokio::time::Duration::from_millis(
                                            delay,
                                        ))
                                        .await;

                                        let retry_request = step.request.clone();
                                        let retry_start = std::time::Instant::now();
                                        let retry_result = Python::with_gil(|py| {
                                            sync_engine.dispatch(py, retry_request)
                                        });
                                        let retry_duration =
                                            retry_start.elapsed().as_millis() as u64;

                                        current_result = StepResult {
                                            step_name: step.name.clone(),
                                            status: retry_result.status.clone(),
                                            result: Some(retry_result),
                                            duration_ms: retry_duration,
                                            attempt,
                                        };

                                        if current_result.is_success() {
                                            retried_steps += 1;
                                            break;
                                        }
                                        retried_steps += 1;
                                    }
                                }
                            }
                        }

                        let succeeded = current_result.is_success();
                        step_results.push(current_result);

                        if succeeded {
                            failed_steps.remove(&step.name);
                        } else {
                            failed_steps.insert(step.name.clone());
                            if stop_on_failure || failure_policy == FailurePolicy::StopPipeline {
                                overall_status = step_results.last().unwrap().status.clone();
                                let error_msg = match &overall_status {
                                    ExecutionStatus::Failed { error } => error.clone(),
                                    other => format!("Step failed: {}", other.name()),
                                };
                                Python::with_gil(|py| -> PyResult<()> {
                                    events.push(wrap_event(
                                        py,
                                        "pipeline.failure".to_string(),
                                        FailureEvent::new(
                                            "step_failure".to_string(),
                                            error_msg.clone(),
                                            false,
                                        )
                                        .into_py(py),
                                        Some(correlation_id.clone()),
                                        None,
                                    )?);
                                    Ok(())
                                })?;
                                break 'group_loop;
                            }
                        }
                    }
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

            Python::with_gil(|py| -> PyResult<()> {
                events.push(wrap_event(
                    py,
                    "pipeline.completed".to_string(),
                    CompletionEvent::new(
                        py,
                        overall_status.name().to_string(),
                        None,
                        total_duration,
                    )
                    .into_py(py),
                    Some(correlation_id),
                    None,
                )?);
                Ok(())
            })?;

            if overall_status.is_success() {
                if let Some(ref store) = cp_store {
                    let _ = store.delete_inner(&pipeline_id);
                }
            }

            Ok(PipelineResult {
                name: pipeline_name,
                status: overall_status,
                step_results,
                total_duration_ms: total_duration,
                events,
                retried_steps,
            })
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

    #[getter]
    fn retry_policy(&self) -> Option<RetryPolicy> {
        self.retry_policy.clone()
    }

    #[getter]
    fn failure_policy(&self) -> FailurePolicy {
        self.failure_policy
    }

    #[getter]
    fn max_concurrency(&self) -> usize {
        self.max_concurrency
    }

    fn __repr__(&self) -> String {
        format!(
            "AsyncPipeline(name={}, steps={}, stop_on_failure={}, failure_policy={}, max_concurrency={})",
            self.name,
            self.steps.len(),
            self.stop_on_failure,
            self.failure_policy.name(),
            self.max_concurrency
        )
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Execute a single step with retry policy applied. Returns (StepResult, failed).
fn execute_step_with_retry(
    py: Python<'_>,
    engine: &Engine,
    step: &PipelineStep,
    retry_policy: &Option<RetryPolicy>,
    _effective_timeout: Option<u64>,
    correlation_id: &str,
    events: &mut Vec<EventEnvelope>,
) -> PyResult<(StepResult, bool)> {
    let step_start = std::time::Instant::now();
    let result = engine.dispatch(py, step.request.clone());
    let duration = step_start.elapsed().as_millis() as u64;

    let step_status = if result.is_success() {
        "completed".to_string()
    } else {
        "failed".to_string()
    };
    events.push(wrap_event(
        py,
        format!("step.{}", step_status),
        StageLifecycleEvent::new(step.name.clone(), step_status).into_py(py),
        Some(correlation_id.to_string()),
        None,
    )?);

    let mut current_result = StepResult {
        step_name: step.name.clone(),
        status: result.status.clone(),
        result: Some(result.clone()),
        duration_ms: duration,
        attempt: 1,
    };

    // Apply retry policy
    if let Some(ref rp) = retry_policy {
        if !current_result.is_success() {
            let error_kind = error_kind_of(&result);
            if rp.is_retryable(&error_kind) {
                let mut attempt = 1u32;
                while attempt < rp.max_attempts {
                    attempt += 1;
                    let delay = rp.delay_for_attempt(attempt - 2);
                    std::thread::sleep(std::time::Duration::from_millis(delay));

                    let retry_start = std::time::Instant::now();
                    let retry_result = engine.dispatch(py, step.request.clone());
                    let retry_duration = retry_start.elapsed().as_millis() as u64;

                    // Emit retry event
                    let retry_status = if retry_result.is_success() {
                        "completed"
                    } else {
                        "failed"
                    };
                    events.push(wrap_event(
                        py,
                        format!("step.retry.{}", retry_status),
                        StageLifecycleEvent::new(
                            step.name.clone(),
                            format!("retry_{}", retry_status),
                        )
                        .into_py(py),
                        Some(correlation_id.to_string()),
                        None,
                    )?);

                    current_result = StepResult {
                        step_name: step.name.clone(),
                        status: retry_result.status.clone(),
                        result: Some(retry_result),
                        duration_ms: retry_duration,
                        attempt,
                    };

                    if current_result.is_success() {
                        break;
                    }
                }
            }
        }
    }

    let failed = !current_result.is_success();
    Ok((current_result, failed))
}

/// Topological sort of steps respecting dependencies.
/// Returns step names in execution order.
fn topological_sort(steps: &[PipelineStep]) -> PyResult<Vec<String>> {
    let step_names: HashSet<&str> = steps.iter().map(|s| s.name.as_str()).collect();

    // Validate that all dependency references exist
    for step in steps {
        for dep in &step.dependencies {
            if !step_names.contains(dep.as_str()) {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "dependency_error: step '{}' references non-existent step '{}'",
                    step.name, dep
                )));
            }
        }
    }

    // Kahn's algorithm — in-degree is the number of unresolved dependencies
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    for step in steps {
        in_degree.insert(&step.name, step.dependencies.len());
    }

    let mut queue: VecDeque<String> = VecDeque::new();
    for (&name, &degree) in &in_degree {
        if degree == 0 {
            queue.push_back(name.to_string());
        }
    }

    let mut result = Vec::new();
    while let Some(current) = queue.pop_front() {
        result.push(current.clone());
        for step in steps {
            if step.dependencies.iter().any(|d| d == &current) {
                if let Some(d) = in_degree.get_mut(step.name.as_str()) {
                    *d -= 1;
                    if *d == 0 {
                        queue.push_back(step.name.clone());
                    }
                }
            }
        }
    }

    if result.len() != steps.len() {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "dependency_error: circular dependency detected among pipeline steps",
        ));
    }

    Ok(result)
}

/// Group step names by their parallel_group.
/// Steps without a parallel_group are each in their own singleton group.
/// Steps with the same parallel_group name are placed in the same group.
fn group_by_parallel(execution_order: &[String], steps: &[PipelineStep]) -> Vec<Vec<String>> {
    // Build name → parallel_group map
    let group_map: HashMap<&str, Option<&str>> = steps
        .iter()
        .map(|s| (s.name.as_str(), s.parallel_group.as_deref()))
        .collect();

    // Group by parallel_group, preserving execution order
    let mut groups: Vec<Vec<String>> = Vec::new();
    let mut assigned: HashSet<&str> = HashSet::new();

    for name in execution_order {
        if assigned.contains(name.as_str()) {
            continue;
        }

        match group_map.get(name.as_str()) {
            Some(Some(group_name)) => {
                // Collect all steps in this parallel group in order
                let mut group_steps = Vec::new();
                for n in execution_order {
                    if assigned.contains(n.as_str()) {
                        continue;
                    }
                    if let Some(Some(g)) = group_map.get(n.as_str()) {
                        if g == group_name {
                            group_steps.push(n.clone());
                            assigned.insert(n);
                        }
                    }
                }
                groups.push(group_steps);
            }
            _ => {
                groups.push(vec![name.clone()]);
                assigned.insert(name);
            }
        }
    }

    groups
}

/// Evaluate the condition on a step. Returns true if the step should run.
fn evaluate_condition(step: &PipelineStep, _step_results: &[StepResult]) -> bool {
    // Basic condition evaluation: if condition is None, always run.
    // String-based conditions are evaluated as simple boolean expressions
    // against step results. For now, a non-empty condition string means "run".
    // Full expression evaluation is left as future work.
    step.condition.is_none() || step.condition.as_deref() != Some("false")
}

/// Save a checkpoint after a step completes.
fn save_step_checkpoint(
    store: &checkpoint_store::CheckpointStore,
    pipeline_id: &str,
    pipeline_name: &str,
    steps: &[PipelineStep],
    stop_on_failure: bool,
    retry_policy: &Option<RetryPolicy>,
    failure_policy: FailurePolicy,
    max_concurrency: usize,
    engine: &Engine,
    completed_steps: &[String],
    previous_results: &[StepResult],
    current_result: &StepResult,
) -> PyResult<()> {
    let compatibility = checkpoint_compatibility(
        pipeline_name,
        steps,
        stop_on_failure,
        retry_policy,
        failure_policy,
        max_concurrency,
        engine,
    );

    let mut step_results_map = std::collections::HashMap::new();
    for sr in previous_results {
        let val = serde_json::to_value(sr).map_err(|error| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "failed to serialize checkpoint result: {error}"
            ))
        })?;
        step_results_map.insert(sr.step_name.clone(), val);
    }
    let current_value = serde_json::to_value(current_result).map_err(|error| {
        pyo3::exceptions::PyValueError::new_err(format!(
            "failed to serialize checkpoint result: {error}"
        ))
    })?;
    step_results_map.insert(current_result.step_name.clone(), current_value);

    let mut completed = completed_steps.to_vec();
    completed.push(current_result.step_name.clone());

    let now_ms = checkpoint_store::current_epoch_ms();
    let cp = PipelineCheckpoint {
        version: CheckpointVersion::current(),
        pipeline_id: pipeline_id.to_string(),
        pipeline_name: pipeline_name.to_string(),
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
    store.save_inner(cp)
}
