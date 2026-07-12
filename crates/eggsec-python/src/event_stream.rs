use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::event_protocol::EventEnvelope;
use crate::handles::{EventLog, ExecutionEvent};

/// Push-based event stream with filtering and iteration support.
///
/// Wraps an `EventLog` and provides subscription-like filtering,
/// async iteration, and snapshot capabilities.
#[pyclass]
pub struct EventStream {
    events: Vec<EventEnvelope>,
    filter_type: Option<String>,
    filter_correlation: Option<String>,
}

#[pymethods]
impl EventStream {
    /// Create a new EventStream from an existing EventLog.
    /// All events are wrapped in EventEnvelope with version metadata.
    #[new]
    #[pyo3(signature = (event_log=None,))]
    fn new(py: Python<'_>, event_log: Option<&EventLog>) -> PyResult<Self> {
        let events = match event_log {
            Some(log) => {
                let mut envs = Vec::new();
                for ev in log.events() {
                    let env = EventEnvelope::from_legacy(py, ev)?;
                    envs.push(env);
                }
                envs
            }
            None => Vec::new(),
        };
        Ok(Self {
            events,
            filter_type: None,
            filter_correlation: None,
        })
    }

    /// Create a new empty EventStream.
    #[staticmethod]
    fn empty() -> Self {
        Self {
            events: Vec::new(),
            filter_type: None,
            filter_correlation: None,
        }
    }

    /// Push a versioned event onto the stream.
    fn push(&mut self, event: EventEnvelope) {
        self.events.push(event);
    }

    /// Get the number of events in the stream (unfiltered).
    fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if the stream is empty.
    fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Get an event by index from the unfiltered stream (returns dict).
    fn get(&self, py: Python<'_>, i: usize) -> PyResult<PyObject> {
        match self.events.get(i) {
            Some(e) => e.to_dict_impl(py),
            None => Err(pyo3::exceptions::PyIndexError::new_err(format!(
                "Index {} out of range",
                i
            ))),
        }
    }

    /// Return a new EventStream filtered by event type.
    fn filter_by_type(&self, event_type: &str) -> EventStream {
        let filtered: Vec<EventEnvelope> = self
            .events
            .iter()
            .filter(|e| e.event_type == event_type)
            .cloned()
            .collect();
        EventStream {
            events: filtered,
            filter_type: Some(event_type.to_string()),
            filter_correlation: self.filter_correlation.clone(),
        }
    }

    /// Return a new EventStream filtered by correlation ID.
    fn filter_by_correlation(&self, correlation_id: &str) -> EventStream {
        let filtered: Vec<EventEnvelope> = self
            .events
            .iter()
            .filter(|e| {
                e.correlation_id
                    .as_deref()
                    .map(|c| c == correlation_id)
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
        EventStream {
            events: filtered,
            filter_type: self.filter_type.clone(),
            filter_correlation: Some(correlation_id.to_string()),
        }
    }

    /// Convert the stream to a Python list of dicts.
    fn to_list(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty_bound(py);
        for event in &self.events {
            list.append(event.to_dict_impl(py)?)?;
        }
        Ok(list.into())
    }

    /// Convert the stream to a Python list of dicts (alias).
    fn to_dict_list(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::empty_bound(py);
        for event in &self.events {
            list.append(event.to_dict_impl(py)?)?;
        }
        Ok(list.into())
    }

    /// Get the latest (most recent) event as a dict, if any.
    fn latest(&self, py: Python) -> PyResult<PyObject> {
        match self.events.last() {
            Some(env) => Ok(env.to_dict_impl(py)?),
            None => Ok(py.None()),
        }
    }

    /// Get the number of events matching the current filters.
    fn count(&self) -> usize {
        self.events.len()
    }

    /// Return a snapshot dict of stream metadata.
    fn snapshot(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("total_events", self.events.len())?;
        dict.set_item("filter_type", &self.filter_type)?;
        dict.set_item("filter_correlation", &self.filter_correlation)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "EventStream(events={}, filter_type={:?}, filter_corr={:?})",
            self.events.len(),
            self.filter_type,
            self.filter_correlation,
        )
    }

    fn __len__(&self) -> usize {
        self.events.len()
    }

    /// Iterate over events (yields event dicts).
    fn __iter__<'py>(slf: PyRef<'py, Self>, py: Python<'py>) -> PyResult<PyObject> {
        let list = PyList::empty_bound(py);
        for event in slf.events.iter() {
            let dict = event.to_dict_impl(py)?;
            list.append(dict)?;
        }
        list.call_method0("__iter__").map(|o| o.into())
    }

    /// Check if an event with the given event_id exists in the stream.
    fn __contains__(&self, event_id: &str) -> bool {
        self.events.iter().any(|e| e.event_id == event_id)
    }

    /// Create an async iterator for this EventStream.
    fn __aiter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __anext__<'py>(slf: PyRef<'py, Self>, _py: Python<'py>) -> PyResult<Option<PyObject>> {
        // For a non-async context, return None immediately (empty async iterator)
        Ok(None)
    }

    /// Context manager __enter__.
    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    /// Context manager __exit__.
    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        false
    }
}

/// Create an EventStream from a list of ExecutionEvents (legacy).
#[pyfunction]
pub fn event_stream_from_legacy(
    py: Python<'_>,
    events: Vec<ExecutionEvent>,
) -> PyResult<EventStream> {
    let mut stream = EventStream::empty();
    for ev in events {
        let env = EventEnvelope::from_legacy(py, &ev)?;
        stream.push(env);
    }
    Ok(stream)
}
