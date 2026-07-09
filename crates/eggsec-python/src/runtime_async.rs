use std::sync::mpsc::{self, Receiver, TryRecvError};

use pyo3::prelude::*;

use crate::error::ScanError;

/// A Python-awaitable wrapper around a Rust future running on a background thread.
///
/// Each async operation spawns a dedicated thread with its own Tokio runtime.
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

    /// Iterator protocol: returns `Some(result)` on first call after completion,
    /// `None` thereafter. Raises StopIteration with the result on completion.
    fn __next__<'py>(mut slf: PyRefMut<'py, Self>, _py: Python<'py>) -> PyResult<Option<PyObject>> {
        match slf.rx.as_ref() {
            Some(rx) => match rx.try_recv() {
                Ok(Ok(result)) => {
                    slf.rx.take();
                    Err(pyo3::exceptions::PyStopIteration::new_err(result))
                }
                Ok(Err(e)) => {
                    slf.rx.take();
                    Err(e)
                }
                Err(TryRecvError::Empty) => Ok(None),
                Err(TryRecvError::Disconnected) => {
                    slf.rx.take();
                    Err(ScanError::new_err("Async task failed unexpectedly"))
                }
            },
            None => Ok(None),
        }
    }
}

/// Spawn a Rust future on a background thread with its own Tokio runtime,
/// returning a PyFuture that Python can await.
///
/// The future must return `PyResult<T>` where `T: IntoPy<PyObject>`.
/// The conversion to PyObject happens inside the future (requires GIL access
/// via `Python::with_gil`), so the channel only carries `PyObject`.
pub(crate) fn spawn_async<F, T>(future: F) -> PyResult<PyFuture>
where
    F: std::future::Future<Output = PyResult<T>> + Send + 'static,
    T: IntoPy<PyObject> + Send + 'static,
{
    let (tx, rx) = mpsc::channel();

    std::thread::Builder::new()
        .name("eggsec-async".to_string())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .thread_name("eggsec-async-worker")
                .build();

            match rt {
                Ok(runtime) => {
                    let result = runtime.block_on(future);
                    // Convert T -> PyObject inside GIL before sending
                    let converted = match result {
                        Ok(val) => {
                            let py_result = Python::with_gil(|py| Ok(val.into_py(py)));
                            py_result
                        }
                        Err(e) => Err(e),
                    };
                    let _ = tx.send(converted);
                }
                Err(e) => {
                    let _ = tx.send(Err(ScanError::new_err(format!(
                        "Failed to create async runtime: {}",
                        e
                    ))));
                }
            }
        })
        .map_err(|e| ScanError::new_err(format!("Failed to spawn async task: {}", e)))?;

    Ok(PyFuture { rx: Some(rx) })
}
