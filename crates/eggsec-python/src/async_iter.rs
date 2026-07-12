use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::event_protocol::EventEnvelope;

/// Async iterator for EventStream.
///
/// Wraps an EventStream snapshot to support `async for` in Python.
#[pyclass]
pub struct EventStreamAsyncIterator {
    events: Vec<EventEnvelope>,
    index: usize,
}

#[pymethods]
impl EventStreamAsyncIterator {
    #[new]
    fn new(events: Vec<EventEnvelope>) -> Self {
        Self { events, index: 0 }
    }

    fn __aiter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __anext__<'py>(
        mut slf: PyRefMut<'py, Self>,
        _py: Python<'py>,
    ) -> PyResult<Option<PyObject>> {
        if slf.index >= slf.events.len() {
            return Ok(None);
        }
        let env = slf.events[slf.index].clone();
        slf.index += 1;
        Python::with_gil(|py| {
            let dict = PyDict::new_bound(py);
            dict.set_item("schema_version", &env.schema_version)?;
            dict.set_item("event_id", &env.event_id)?;
            dict.set_item("timestamp_ms", env.timestamp_ms)?;
            dict.set_item("correlation_id", &env.correlation_id)?;
            dict.set_item("event_type", &env.event_type)?;
            dict.set_item("payload", &env.payload)?;
            Ok(Some(dict.into()))
        })
    }
}

/// Async iterator for a stream of findings.
///
/// Wraps a list of findings to support `async for` in Python.
#[pyclass]
pub struct FindingStreamAsyncIterator {
    findings: Vec<PyObject>,
    index: usize,
}

#[pymethods]
impl FindingStreamAsyncIterator {
    #[new]
    fn new(findings: Vec<PyObject>) -> Self {
        Self { findings, index: 0 }
    }

    fn __aiter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __anext__<'py>(mut slf: PyRefMut<'py, Self>, py: Python<'py>) -> PyResult<Option<PyObject>> {
        if slf.index >= slf.findings.len() {
            return Ok(None);
        }
        let finding = slf.findings[slf.index].clone_ref(py);
        slf.index += 1;
        Ok(Some(finding))
    }
}
