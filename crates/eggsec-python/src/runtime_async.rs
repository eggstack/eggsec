use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::OnceLock;

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
    rx: Option<Receiver<PyResult<PyObject>>>,
}

#[pymethods]
impl PyFuture {
    /// Python `__await__` protocol: returns self as an iterator.
    fn __await__(slf: PyRef<'_, Self>) -> PyResult<PyObject> {
        let py = slf.py();
        use pyo3::conversion::IntoPy;
        Ok(slf.into_py(py))
    }

    /// Iterator protocol: returns Python `None` while pending and raises
    /// StopIteration with the result once the worker completes.
    fn __next__<'py>(mut slf: PyRefMut<'py, Self>, py: Python<'py>) -> PyResult<PyObject> {
        match slf.rx.as_ref() {
            Some(rx) => match rx.try_recv() {
                Ok(Ok(result)) => {
                    slf.rx.take();
                    // StopIteration's value must be a one-element args tuple.
                    // Passing the PyObject directly produces an empty
                    // StopIteration on current PyO3/Python combinations and
                    // silently turns every async result into None.
                    Err(PyErr::from_type_bound(
                        pyo3::exceptions::PyStopIteration::type_object_bound(py),
                        (result,),
                    ))
                }
                Ok(Err(e)) => {
                    slf.rx.take();
                    Err(e)
                }
                // `Option<PyObject>` is special-cased by PyO3's iterator
                // trampoline: `Ok(None)` becomes StopIteration.  Return an
                // actual Python None while the worker is still pending so
                // callers can poll this awaitable without losing its result.
                Err(TryRecvError::Empty) => Ok(py.None()),
                Err(TryRecvError::Disconnected) => {
                    slf.rx.take();
                    Err(ScanError::new_err("Async task failed unexpectedly"))
                }
            },
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
/// `T: IntoPy<PyObject>`.
pub(crate) fn spawn_async<F, T>(future: F) -> PyResult<PyFuture>
where
    F: std::future::Future<Output = PyResult<T>> + Send + 'static,
    T: IntoPy<PyObject> + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    let runtime = get_async_runtime();

    runtime.spawn(async move {
        let result = future.await;
        let converted = match result {
            Ok(val) => {
                let py_result = Python::with_gil(|py| Ok(val.into_py(py)));
                py_result
            }
            Err(e) => Err(e),
        };
        let _ = tx.send(converted);
    });

    Ok(PyFuture { rx: Some(rx) })
}
