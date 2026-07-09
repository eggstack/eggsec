use std::sync::OnceLock;

use pyo3::prelude::*;

use crate::error::ScanError;

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

fn get_runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("eggsec-python")
            .build()
            .expect("Failed to create tokio runtime for eggsec-python")
    })
}

/// Execute an async future on the shared runtime, releasing the GIL during I/O.
pub(crate) fn block_on<F, T, E>(py: Python, future: F) -> PyResult<T>
where
    F: std::future::Future<Output = Result<T, E>> + Send + 'static,
    T: Send + 'static,
    E: std::fmt::Display + Send + 'static,
{
    let runtime = get_runtime();
    py.allow_threads(move || runtime.block_on(future))
        .map_err(|e| ScanError::new_err(format!("Operation failed: {}", e)))
}
