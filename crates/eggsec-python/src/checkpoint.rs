use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::sync::Mutex;

use crate::pipeline::StepResult;

/// A checkpoint capturing pipeline execution state for resumption.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct Checkpoint {
    #[pyo3(get)]
    id: String,
    #[pyo3(get)]
    pipeline_name: String,
    pub(crate) completed_steps: Vec<String>,
    pub(crate) results: Vec<StepResult>,
    #[pyo3(get)]
    created_at_ms: u64,
}

#[pymethods]
impl Checkpoint {
    #[new]
    #[pyo3(signature = (id, pipeline_name, completed_steps=None, results=None, created_at_ms=0))]
    fn new(
        id: String,
        pipeline_name: String,
        completed_steps: Option<Vec<String>>,
        results: Option<Vec<StepResult>>,
        created_at_ms: u64,
    ) -> Self {
        Self {
            id,
            pipeline_name,
            completed_steps: completed_steps.unwrap_or_default(),
            results: results.unwrap_or_default(),
            created_at_ms,
        }
    }

    #[getter]
    fn results(&self) -> Vec<StepResult> {
        self.results.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", &self.id)?;
        dict.set_item("pipeline_name", &self.pipeline_name)?;
        dict.set_item("completed_steps", &self.completed_steps)?;

        let results_list = PyList::empty_bound(py);
        for r in &self.results {
            results_list.append(r.to_dict(py)?)?;
        }
        dict.set_item("results", results_list)?;
        dict.set_item("created_at_ms", self.created_at_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "Checkpoint(id={}, pipeline={}, completed={})",
            self.id,
            self.pipeline_name,
            self.completed_steps.len()
        )
    }
}

impl serde::Serialize for Checkpoint {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("Checkpoint", 5)?;
        s.serialize_field("id", &self.id)?;
        s.serialize_field("pipeline_name", &self.pipeline_name)?;
        s.serialize_field("completed_steps", &self.completed_steps)?;
        s.serialize_field("results", &self.results)?;
        s.serialize_field("created_at_ms", &self.created_at_ms)?;
        s.end()
    }
}

/// A thread-safe store for managing pipeline checkpoints.
#[pyclass]
pub struct CheckpointStore {
    checkpoints: Mutex<Vec<Checkpoint>>,
}

#[pymethods]
impl CheckpointStore {
    #[new]
    fn new() -> Self {
        Self {
            checkpoints: Mutex::new(Vec::new()),
        }
    }

    /// Save a new checkpoint and return it.
    fn save(
        &self,
        pipeline_name: String,
        completed_steps: Vec<String>,
        results: Vec<StepResult>,
    ) -> Checkpoint {
        let id = format!(
            "cp-{}-{}",
            completed_steps.len(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        );
        let created_at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let checkpoint = Checkpoint {
            id: id.clone(),
            pipeline_name,
            completed_steps,
            results,
            created_at_ms,
        };

        let mut store = self.checkpoints.lock().unwrap();
        store.push(checkpoint.clone());
        checkpoint
    }

    /// Load a checkpoint by ID.
    fn load(&self, checkpoint_id: &str) -> Option<Checkpoint> {
        let store = self.checkpoints.lock().unwrap();
        store.iter().find(|c| c.id == checkpoint_id).cloned()
    }

    /// List all stored checkpoints.
    fn list_checkpoints(&self) -> Vec<Checkpoint> {
        let store = self.checkpoints.lock().unwrap();
        store.clone()
    }

    /// Delete a checkpoint by ID. Returns true if found and deleted.
    fn delete(&self, checkpoint_id: &str) -> bool {
        let mut store = self.checkpoints.lock().unwrap();
        let len_before = store.len();
        store.retain(|c| c.id != checkpoint_id);
        store.len() < len_before
    }

    /// Number of stored checkpoints.
    fn len(&self) -> usize {
        let store = self.checkpoints.lock().unwrap();
        store.len()
    }

    fn __repr__(&self) -> String {
        let len = self.len();
        format!("CheckpointStore({} checkpoints)", len)
    }
}
