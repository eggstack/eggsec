use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

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
