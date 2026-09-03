use std::sync::OnceLock;

use pyo3::prelude::*;

use crate::error::ScanError;

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

fn get_runtime() -> PyResult<&'static tokio::runtime::Runtime> {
    if let Some(runtime) = RUNTIME.get() {
        return Ok(runtime);
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .thread_name("eggsec-python")
        .build()
        .or_else(|e| {
            // Fall back to a current-thread runtime so callers still
            // receive a usable handle instead of crashing the Python
            // interpreter on init failure.
            tracing::error!(
                error = %e,
                "failed to create multi-thread runtime; falling back to current-thread runtime"
            );
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
        })
        .map_err(|e| {
            tracing::error!(
                error = %e,
                "failed to construct fallback current-thread runtime"
            );
            ScanError::new_err(format!("failed to initialize runtime: {}", e))
        })?;
    // If another thread won the init race, drop ours and use theirs.
    let _ = RUNTIME.set(runtime);
    RUNTIME
        .get()
        .ok_or_else(|| ScanError::new_err("runtime unavailable after init"))
}

/// Execute an async future on the shared runtime, releasing the GIL during I/O.
pub(crate) fn block_on<F, T, E>(py: Python, future: F) -> PyResult<T>
where
    F: std::future::Future<Output = Result<T, E>> + Send + 'static,
    T: Send + 'static,
    E: std::fmt::Display + Send + 'static,
{
    let runtime = get_runtime()?;
    py.detach(move || runtime.block_on(future))
        .map_err(|e| ScanError::new_err(format!("Operation failed: {}", e)))
}
