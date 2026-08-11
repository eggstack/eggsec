use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Mutex, OnceLock};

use crate::PyObject;
use pyo3::prelude::*;
use pyo3::PyTypeInfo;

use crate::error::ScanError;

/// Process-global shared Tokio runtime for all async operations.
///
/// Using a single shared runtime ensures that stateful async resources
/// (e.g. `AsyncTcpSession`, `AsyncUdpSocket`) survive across chained awaits.
/// Each `PyFuture` spawned via [`spawn_async`] runs on this runtime, so
/// resources created in one awaited call remain valid for subsequent calls.
static ASYNC_RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

fn get_async_runtime() -> &'static tokio::runtime::Runtime {
    ASYNC_RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("eggsec-async")
            .build()
            .expect("Failed to create shared async runtime for eggsec-python")
    })
}

/// A Python-awaitable wrapper around a Rust future running on the shared runtime.
///
/// The result is communicated back via a channel. Python polls via `__await__`.
#[pyclass]
pub struct PyFuture {
    rx: Option<Mutex<Receiver<PyResult<PyObject>>>>,
}

#[pymethods]
impl PyFuture {
    /// Python `__await__` protocol: returns self as an iterator.
    fn __await__(slf: PyRef<'_, Self>) -> PyResult<PyObject> {
        let py = slf.py();
        Ok(slf.into_pyobject(py)?.into_any().unbind())
    }

    /// Iterator protocol: returns Python `None` while pending and raises
    /// StopIteration with the result once the worker completes.
    fn __next__<'py>(mut slf: PyRefMut<'py, Self>, py: Python<'py>) -> PyResult<PyObject> {
        let recv_result = slf.rx.as_ref().map(|rx| rx.lock().unwrap().try_recv());
        match recv_result {
            Some(Ok(Ok(result))) => {
                slf.rx.take();
                // StopIteration's value must be a one-element args tuple.
                // Passing the PyObject directly produces an empty
                // StopIteration on current PyO3/Python combinations and
                // silently turns every async result into None.
                Err(PyErr::from_type(
                    pyo3::exceptions::PyStopIteration::type_object(py),
                    (result,),
                ))
            }
            Some(Ok(Err(e))) => {
                slf.rx.take();
                Err(e)
            }
            // `Option<PyObject>` is special-cased by PyO3's iterator
            // trampoline: `Ok(None)` becomes StopIteration.  Return an
            // actual Python None while the worker is still pending so
            // callers can poll this awaitable without losing its result.
            Some(Err(TryRecvError::Empty)) => Ok(py.None()),
            Some(Err(TryRecvError::Disconnected)) => {
                slf.rx.take();
                Err(ScanError::new_err("Async task failed unexpectedly"))
            }
            None => Ok(py.None()),
        }
    }
}

/// Spawn a Rust future on the shared Tokio runtime, returning a PyFuture
/// that Python can await.
///
/// All async operations share a single process-global runtime so that
/// stateful resources (TCP sessions, WebSocket connections, etc.) persist
/// across chained awaits.  The future must return `PyResult<T>` where
/// `T` can be converted to a Python object.
pub(crate) fn spawn_async<F, T>(future: F) -> PyResult<PyFuture>
where
    F: std::future::Future<Output = PyResult<T>> + Send + 'static,
    T: for<'py> IntoPyObject<'py> + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    let runtime = get_async_runtime();

    runtime.spawn(async move {
        let result = future.await;
        let converted = match result {
            Ok(val) => {
                let py_result = Python::attach(|py| {
                    use pyo3::conversion::IntoPyObjectExt;
                    val.into_py_any(py)
                });
                py_result
            }
            Err(e) => Err(e),
        };
        let _ = tx.send(converted);
    });

    Ok(PyFuture {
        rx: Some(Mutex::new(rx)),
    })
}
