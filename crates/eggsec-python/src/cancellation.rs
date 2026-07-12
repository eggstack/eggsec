use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct CancellationTokenInner {
    cancelled: AtomicBool,
    reason: Mutex<Option<String>>,
}

/// A cancellation token for cooperative cancellation of operations.
///
/// Can be checked periodically by running operations to determine
/// if they should abort early.
#[pyclass]
#[derive(Debug, Clone)]
pub struct CancellationToken {
    inner: Arc<CancellationTokenInner>,
}

impl CancellationToken {
    pub(crate) fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::SeqCst)
    }

    pub(crate) fn reason(&self) -> Option<String> {
        self.inner.reason.lock().ok().and_then(|r| r.clone())
    }
}

#[pymethods]
impl CancellationToken {
    /// Create a new cancellation token.
    #[new]
    fn py_new() -> Self {
        Self {
            inner: Arc::new(CancellationTokenInner {
                cancelled: AtomicBool::new(false),
                reason: Mutex::new(None),
            }),
        }
    }

    /// Request cancellation with an optional reason.
    #[pyo3(signature = (reason=None))]
    fn cancel(&self, reason: Option<String>) {
        self.inner.cancelled.store(true, Ordering::SeqCst);
        if let Ok(mut r) = self.inner.reason.lock() {
            *r = reason;
        }
    }

    /// Check if cancellation has been requested.
    #[pyo3(name = "is_cancelled")]
    fn py_is_cancelled(&self) -> bool {
        self.is_cancelled()
    }

    /// Get the cancellation reason, if any.
    #[pyo3(name = "reason")]
    fn py_reason(&self) -> Option<String> {
        self.reason()
    }

    /// Get a Python object that can be checked for cancellation.
    ///
    /// Returns a dict with `is_cancelled` and `reason` fields.
    fn cancel_token(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("is_cancelled", self.is_cancelled())?;
        dict.set_item("reason", self.reason())?;
        Ok(dict.into())
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("cancelled", self.is_cancelled())?;
        dict.set_item("reason", self.reason())?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        #[derive(serde::Serialize)]
        struct CancelTokenJson {
            cancelled: bool,
            reason: Option<String>,
        }
        serde_json::to_string(&CancelTokenJson {
            cancelled: self.is_cancelled(),
            reason: self.reason(),
        })
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("CancellationToken(cancelled={})", self.is_cancelled())
    }
}
