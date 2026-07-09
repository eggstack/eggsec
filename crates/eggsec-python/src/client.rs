use pyo3::prelude::*;

use crate::dto::PortScanResult;
use crate::runtime_sync;
use crate::scope::Scope;

/// Client for performing scoped security scans through the Eggsec engine.
///
/// Wraps the Rust engine directly (does not shell out to the CLI).
/// The GIL is released during network I/O operations.
#[pyclass]
pub struct Client {
    scope: Scope,
    mode: String,
    concurrency: usize,
    timeout_ms: u64,
}

#[pymethods]
impl Client {
    /// Create a new scan client.
    ///
    /// Args:
    ///     scope: Scope defining authorized targets and ports.
    ///     mode: Execution mode ("manual" or "automation").
    ///     concurrency: Max concurrent connections (default: 100).
    ///     timeout_ms: Connection timeout in milliseconds (default: 5000).
    ///
    /// Raises:
    ///     ValueError: If mode is not "manual" or "automation".
    #[new]
    #[pyo3(signature = (scope, *, mode="manual", concurrency=100, timeout_ms=5000))]
    fn new(scope: Scope, mode: &str, concurrency: usize, timeout_ms: u64) -> PyResult<Self> {
        if mode != "manual" && mode != "automation" {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid mode '{}'. Must be 'manual' or 'automation'.",
                mode
            )));
        }
        Ok(Self {
            scope,
            mode: mode.to_string(),
            concurrency,
            timeout_ms,
        })
    }

    /// Perform a TCP port scan.
    ///
    /// Args:
    ///     target: Hostname or IP to scan.
    ///     ports: List of port numbers to scan.
    ///     concurrency: Max concurrent connections (overrides client default).
    ///     timeout_ms: Connection timeout in ms (overrides client default).
    ///
    /// Returns:
    ///     PortScanResult: Structured scan results.
    ///
    /// Raises:
    ///     EnforcementError: If the target is outside the allowed scope.
    ///     ScanError: If the scan fails.
    #[pyo3(signature = (target, ports, *, concurrency=None, timeout_ms=None))]
    fn scan_ports(
        &self,
        target: &str,
        ports: Vec<u16>,
        concurrency: Option<usize>,
        timeout_ms: Option<u64>,
    ) -> PyResult<PortScanResult> {
        self.scope.enforce_target(target)?;

        let effective_concurrency = concurrency.unwrap_or(self.concurrency);
        let effective_timeout_ms = timeout_ms.unwrap_or(self.timeout_ms);

        for &port in &ports {
            self.scope.enforce_port(port)?;
        }

        let config = eggsec::scanner::PortScanConfig {
            ports,
            concurrency: effective_concurrency,
            timeout_duration: std::time::Duration::from_millis(effective_timeout_ms),
            tui_mode: false,
            spoof_config: eggsec::scanner::SpoofConfig::default(),
            progress_tx: None,
            max_results: None,
        };

        let target_owned = target.to_string();

        Python::with_gil(|py| {
            let result = runtime_sync::block_on(py, async move {
                eggsec::scanner::scan_ports(&target_owned, config).await
            })?;

            Ok(PortScanResult::from_engine(result))
        })
    }

    /// Get the client's scope.
    #[getter]
    fn scope(&self) -> Scope {
        self.scope.clone()
    }

    /// Get the client's mode.
    #[getter]
    fn mode(&self) -> String {
        self.mode.clone()
    }

    fn __repr__(&self) -> String {
        format!("Client(mode={})", self.mode)
    }
}
