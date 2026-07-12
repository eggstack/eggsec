use pyo3::prelude::*;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::event_protocol::EventEnvelope;

/// Async callback wrapper — for `async def` handlers.
///
/// Wraps a Python async callable so it can be invoked from Rust
/// by scheduling it on the event loop. The callback is expected
/// to return an awaitable; errors are logged and swallowed.
///
/// Thread-safe: uses `Py<PyAny>` for GIL-safe access.
#[pyclass]
pub struct AsyncCallback {
    callback: Option<pyo3::Py<pyo3::types::PyAny>>,
    closed: bool,
}

#[pymethods]
impl AsyncCallback {
    /// Create a new AsyncCallback wrapping an async Python callable.
    ///
    /// Args:
    ///     callback: An async Python callable (coroutine function).
    #[new]
    fn new(callback: PyObject) -> Self {
        Self {
            callback: Some(callback.into()),
            closed: false,
        }
    }

    /// Invoke the async callback with an event envelope (passed as dict).
    fn invoke(&self, py: Python<'_>, event: &EventEnvelope) -> PyResult<PyObject> {
        if self.closed {
            return Ok(py.None());
        }
        if let Some(ref cb) = self.callback {
            let dict = event.to_dict_impl(py)?;
            match cb.call1(py, (dict,)) {
                Ok(result) => Ok(result),
                Err(e) => {
                    tracing::warn!("AsyncCallback invoke error: {}", e);
                    Ok(py.None())
                }
            }
        } else {
            Ok(py.None())
        }
    }

    /// Close the async callback, preventing further invocations.
    fn close(&mut self) {
        self.callback = None;
        self.closed = true;
    }

    /// Whether the async callback has been closed.
    #[getter]
    fn is_closed(&self) -> bool {
        self.closed
    }

    fn __repr__(&self) -> String {
        format!("AsyncCallback(closed={})", self.closed)
    }
}

/// Callback scheduler — queues callbacks for execution with backpressure.
///
/// Maintains an internal queue of pending callbacks and provides
/// methods to drain and execute them. Uses a bounded queue to
/// prevent unbounded memory growth under high event rates.
#[pyclass]
pub struct CallbackScheduler {
    queue: Arc<Mutex<VecDeque<CallbackTask>>>,
    capacity: usize,
    closed: bool,
}

struct CallbackTask {
    event: EventEnvelope,
}

#[pymethods]
impl CallbackScheduler {
    /// Create a new CallbackScheduler with a bounded capacity.
    ///
    /// Args:
    ///     capacity: Maximum number of queued callbacks (default 1000).
    #[new]
    #[pyo3(signature = (capacity=1000,))]
    fn new(capacity: usize) -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::with_capacity(capacity.min(4096)))),
            capacity,
            closed: false,
        }
    }

    /// Enqueue an event for delivery. Returns false if the queue is full.
    fn enqueue(&self, event: EventEnvelope) -> bool {
        if self.closed {
            return false;
        }
        let mut q = match self.queue.lock() {
            Ok(q) => q,
            Err(_) => return false,
        };
        if q.len() >= self.capacity {
            tracing::warn!("CallbackScheduler queue full, dropping event");
            return false;
        }
        q.push_back(CallbackTask { event });
        true
    }

    /// Drain all queued events and return them.
    fn drain(&self) -> Vec<EventEnvelope> {
        let mut q = match self.queue.lock() {
            Ok(q) => q,
            Err(_) => return Vec::new(),
        };
        q.drain(..).map(|t| t.event).collect()
    }

    /// Number of events currently queued.
    fn pending(&self) -> usize {
        self.queue.lock().map(|q| q.len()).unwrap_or(0)
    }

    /// Close the scheduler, preventing further enqueuing.
    fn close(&mut self) {
        self.closed = true;
    }

    /// Whether the scheduler has been closed.
    #[getter]
    fn is_closed(&self) -> bool {
        self.closed
    }

    fn __repr__(&self) -> String {
        let pending = self.pending();
        format!(
            "CallbackScheduler(pending={}, capacity={}, closed={})",
            pending, self.capacity, self.closed,
        )
    }
}
