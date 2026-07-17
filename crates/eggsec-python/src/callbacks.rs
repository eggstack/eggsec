use pyo3::prelude::*;

use crate::audit::EnforcementAuditEventPy;
use crate::event_protocol::EventEnvelope;

/// Audit sink — receives enforcement audit events.
///
/// Wraps a Python callable that is invoked for each audit event.
/// Errors in the callback are logged via tracing but never propagated
/// to avoid Rust panics from Python exceptions.
///
/// Thread-safe: uses `Py<PyAny>` for GIL-safe access from any thread.
/// GIL behavior: acquires the GIL only during callback invocation.
#[pyclass]
pub struct AuditSink {
    callback: Option<pyo3::Py<pyo3::types::PyAny>>,
    closed: bool,
}

#[pymethods]
impl AuditSink {
    /// Create a new AuditSink wrapping a Python callable.
    ///
    /// Args:
    ///     callback: A Python callable accepting a single audit event dict argument.
    #[new]
    fn new(callback: PyObject) -> Self {
        Self {
            callback: Some(callback.into()),
            closed: false,
        }
    }

    /// Invoke the callback with an audit event.
    ///
    /// The event is passed as a dict (from `to_dict`).
    /// If the callback raises an exception, it is logged and swallowed.
    fn send(&self, py: Python<'_>, event: &EnforcementAuditEventPy) -> PyResult<()> {
        if self.closed {
            return Ok(());
        }
        if let Some(ref cb) = self.callback {
            let dict = event.to_dict_impl(py)?;
            match cb.call1(py, (dict,)) {
                Ok(_) => Ok(()),
                Err(e) => {
                    tracing::warn!("AuditSink callback error: {}", e);
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }

    /// Close the sink, preventing further callbacks.
    fn close(&mut self) {
        self.callback = None;
        self.closed = true;
    }

    /// Whether the sink has been closed.
    #[getter]
    fn is_closed(&self) -> bool {
        self.closed
    }

    fn __repr__(&self) -> String {
        format!("AuditSink(closed={})", self.closed)
    }

    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __exit__(
        slf: Py<Self>,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        Python::with_gil(|py| -> PyResult<()> {
            slf.borrow_mut(py).close();
            Ok(())
        })
        .ok();
        false
    }
}

/// Finding sink — receives findings as they are discovered.
///
/// Wraps a Python callable that is invoked for each finding.
/// Errors in the callback are logged via tracing but never propagated.
#[pyclass]
pub struct FindingSink {
    callback: Option<pyo3::Py<pyo3::types::PyAny>>,
    closed: bool,
}

#[pymethods]
impl FindingSink {
    /// Create a new FindingSink wrapping a Python callable.
    ///
    /// Args:
    ///     callback: A Python callable accepting a single finding dict argument.
    #[new]
    fn new(callback: PyObject) -> Self {
        Self {
            callback: Some(callback.into()),
            closed: false,
        }
    }

    /// Invoke the callback with a finding (as a dict).
    fn send(&self, py: Python<'_>, finding: PyObject) -> PyResult<()> {
        if self.closed {
            return Ok(());
        }
        if let Some(ref cb) = self.callback {
            match cb.call1(py, (finding,)) {
                Ok(_) => Ok(()),
                Err(e) => {
                    tracing::warn!("FindingSink callback error: {}", e);
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }

    /// Close the sink, preventing further callbacks.
    fn close(&mut self) {
        self.callback = None;
        self.closed = true;
    }

    /// Whether the sink has been closed.
    #[getter]
    fn is_closed(&self) -> bool {
        self.closed
    }

    fn __repr__(&self) -> String {
        format!("FindingSink(closed={})", self.closed)
    }

    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __exit__(
        slf: Py<Self>,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        Python::with_gil(|py| -> PyResult<()> {
            slf.borrow_mut(py).close();
            Ok(())
        })
        .ok();
        false
    }
}

/// Artifact sink — receives artifacts (files, captures, reports).
///
/// Wraps a Python callable that is invoked for each artifact.
/// Errors in the callback are logged via tracing but never propagated.
#[pyclass]
pub struct ArtifactSink {
    callback: Option<pyo3::Py<pyo3::types::PyAny>>,
    closed: bool,
}

#[pymethods]
impl ArtifactSink {
    /// Create a new ArtifactSink wrapping a Python callable.
    ///
    /// Args:
    ///     callback: A Python callable accepting a single artifact dict argument.
    #[new]
    fn new(callback: PyObject) -> Self {
        Self {
            callback: Some(callback.into()),
            closed: false,
        }
    }

    /// Invoke the callback with an artifact (as a dict).
    fn send(&self, py: Python<'_>, artifact: PyObject) -> PyResult<()> {
        if self.closed {
            return Ok(());
        }
        if let Some(ref cb) = self.callback {
            match cb.call1(py, (artifact,)) {
                Ok(_) => Ok(()),
                Err(e) => {
                    tracing::warn!("ArtifactSink callback error: {}", e);
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }

    /// Close the sink, preventing further callbacks.
    fn close(&mut self) {
        self.callback = None;
        self.closed = true;
    }

    /// Whether the sink has been closed.
    #[getter]
    fn is_closed(&self) -> bool {
        self.closed
    }

    fn __repr__(&self) -> String {
        format!("ArtifactSink(closed={})", self.closed)
    }

    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __exit__(
        slf: Py<Self>,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        Python::with_gil(|py| -> PyResult<()> {
            slf.borrow_mut(py).close();
            Ok(())
        })
        .ok();
        false
    }
}

/// Progress sink — receives progress updates.
///
/// Wraps a Python callable that is invoked with percentage and message.
/// Errors in the callback are logged via tracing but never propagated.
#[pyclass]
pub struct ProgressSink {
    callback: Option<pyo3::Py<pyo3::types::PyAny>>,
    closed: bool,
}

#[pymethods]
impl ProgressSink {
    /// Create a new ProgressSink wrapping a Python callable.
    ///
    /// Args:
    ///     callback: A Python callable accepting (percentage: float, message: str).
    #[new]
    fn new(callback: PyObject) -> Self {
        Self {
            callback: Some(callback.into()),
            closed: false,
        }
    }

    /// Invoke the callback with a progress update.
    fn send(&self, py: Python<'_>, percentage: f64, message: &str) -> PyResult<()> {
        if self.closed {
            return Ok(());
        }
        if let Some(ref cb) = self.callback {
            match cb.call1(py, (percentage, message)) {
                Ok(_) => Ok(()),
                Err(e) => {
                    tracing::warn!("ProgressSink callback error: {}", e);
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }

    /// Close the sink, preventing further callbacks.
    fn close(&mut self) {
        self.callback = None;
        self.closed = true;
    }

    /// Whether the sink has been closed.
    #[getter]
    fn is_closed(&self) -> bool {
        self.closed
    }

    fn __repr__(&self) -> String {
        format!("ProgressSink(closed={})", self.closed)
    }

    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __exit__(
        slf: Py<Self>,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        Python::with_gil(|py| -> PyResult<()> {
            slf.borrow_mut(py).close();
            Ok(())
        })
        .ok();
        false
    }
}

/// Event consumer — receives versioned events.
///
/// Wraps a Python callable that is invoked for each EventEnvelope.
/// Errors in the callback are logged via tracing but never propagated.
#[pyclass]
pub struct EventConsumer {
    callback: Option<pyo3::Py<pyo3::types::PyAny>>,
    closed: bool,
}

#[pymethods]
impl EventConsumer {
    /// Create a new EventConsumer wrapping a Python callable.
    ///
    /// Args:
    ///     callback: A Python callable accepting a single event dict argument.
    #[new]
    fn new(callback: PyObject) -> Self {
        Self {
            callback: Some(callback.into()),
            closed: false,
        }
    }

    /// Invoke the callback with an event envelope (as a dict).
    fn send(&self, py: Python<'_>, event: &EventEnvelope) -> PyResult<()> {
        if self.closed {
            return Ok(());
        }
        if let Some(ref cb) = self.callback {
            let dict = event.to_dict_impl(py)?;
            match cb.call1(py, (dict,)) {
                Ok(_) => Ok(()),
                Err(e) => {
                    tracing::warn!("EventConsumer callback error: {}", e);
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }

    /// Close the consumer, preventing further callbacks.
    fn close(&mut self) {
        self.callback = None;
        self.closed = true;
    }

    /// Whether the consumer has been closed.
    #[getter]
    fn is_closed(&self) -> bool {
        self.closed
    }

    fn __repr__(&self) -> String {
        format!("EventConsumer(closed={})", self.closed)
    }

    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __exit__(
        slf: Py<Self>,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        Python::with_gil(|py| -> PyResult<()> {
            slf.borrow_mut(py).close();
            Ok(())
        })
        .ok();
        false
    }
}
