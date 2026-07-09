use pyo3::prelude::*;

use crate::dto::PortScanResult;
use crate::endpoint::EndpointScanResult;
use crate::error::EggsecResultExt;
use crate::fingerprint::FingerprintScanResult;
use crate::runtime_async;
use crate::scope::Scope;

/// Async client for performing scoped security scans through the Eggsec engine.
///
/// Provides the same operations as Client but returns Python awaitables.
/// Each async operation spawns a background thread with its own Tokio runtime.
/// The GIL is released during network I/O.
#[pyclass]
pub struct AsyncClient {
    scope: Scope,
    mode: String,
    concurrency: usize,
    timeout_ms: u64,
}

#[pymethods]
impl AsyncClient {
    /// Create a new async scan client.
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

    /// Perform an async TCP port scan.
    ///
    /// Returns a PyFuture that can be awaited in Python.
    #[pyo3(signature = (target, ports, *, concurrency=None, timeout_ms=None))]
    fn scan_ports(
        &self,
        target: &str,
        ports: Vec<u16>,
        concurrency: Option<usize>,
        timeout_ms: Option<u64>,
    ) -> PyResult<crate::runtime_async::PyFuture> {
        self.scope.enforce_target(target)?;

        let effective_concurrency = concurrency.unwrap_or(self.concurrency);
        let effective_timeout_ms = timeout_ms.unwrap_or(self.timeout_ms);

        for &port in &ports {
            self.scope.enforce_port(port)?;
        }

        let target_owned = target.to_string();
        let ports_owned = ports;

        runtime_async::spawn_async(async move {
            let config = eggsec::scanner::PortScanConfig {
                ports: ports_owned,
                concurrency: effective_concurrency,
                timeout_duration: std::time::Duration::from_millis(effective_timeout_ms),
                tui_mode: false,
                spoof_config: eggsec::scanner::SpoofConfig::default(),
                progress_tx: None,
                max_results: None,
            };

            let result = eggsec::scanner::scan_ports(&target_owned, config)
                .await
                .map_pyerr()?;
            Ok(PortScanResult::from_engine(result))
        })
    }

    /// Perform async endpoint discovery against a web server.
    ///
    /// Returns a PyFuture that can be awaited in Python.
    #[pyo3(signature = (base_url, endpoints, *, concurrency=None, timeout_ms=None, include_404=false, verify_tls=true))]
    fn scan_endpoints(
        &self,
        base_url: &str,
        endpoints: Vec<String>,
        concurrency: Option<usize>,
        timeout_ms: Option<u64>,
        include_404: bool,
        verify_tls: bool,
    ) -> PyResult<crate::runtime_async::PyFuture> {
        let host = extract_host_from_url(base_url)?;
        self.scope.enforce_target(&host)?;

        let effective_concurrency = concurrency.unwrap_or(self.concurrency);
        let effective_timeout_ms = timeout_ms.unwrap_or(self.timeout_ms);

        let base_url_owned = base_url.to_string();

        runtime_async::spawn_async(async move {
            let config = eggsec::scanner::EndpointScanConfig {
                base_url: base_url_owned,
                endpoints,
                concurrency: effective_concurrency,
                timeout_duration: std::time::Duration::from_millis(effective_timeout_ms),
                include_404,
                tui_mode: false,
                spoof_config: std::sync::Arc::new(eggsec::scanner::SpoofConfig::default()),
                verify_tls,
                progress_tx: None,
                max_results: None,
            };

            let result = eggsec::scanner::scan_endpoints(config)
                .await
                .map_pyerr()?;
            Ok(EndpointScanResult::from_engine(result))
        })
    }

    /// Perform async service fingerprinting on target ports.
    ///
    /// Returns a PyFuture that can be awaited in Python.
    #[pyo3(signature = (target, ports, *, concurrency=None, timeout_ms=None))]
    fn fingerprint_services(
        &self,
        target: &str,
        ports: Vec<u16>,
        concurrency: Option<usize>,
        timeout_ms: Option<u64>,
    ) -> PyResult<crate::runtime_async::PyFuture> {
        self.scope.enforce_target(target)?;

        let effective_concurrency = concurrency.unwrap_or(self.concurrency);
        let effective_timeout_ms = timeout_ms.unwrap_or(self.timeout_ms);

        for &port in &ports {
            self.scope.enforce_port(port)?;
        }

        let target_owned = target.to_string();
        let ports_owned = ports;

        runtime_async::spawn_async(async move {
            let result = eggsec::scanner::fingerprint_services(
                &target_owned,
                ports_owned,
                std::time::Duration::from_millis(effective_timeout_ms),
                false,
                effective_concurrency,
                None,
                None,
            )
            .await
            .map_pyerr()?;
            Ok(FingerprintScanResult::from_engine(result))
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

    /// Close the client (no-op, exists for API consistency).
    fn close(&self) {}

    /// Context manager __aenter__ (returns self for use with `async with`).
    fn __aenter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    /// Context manager __aexit__.
    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __aexit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        false // Don't suppress exceptions
    }

    fn __repr__(&self) -> String {
        format!("AsyncClient(mode={})", self.mode)
    }
}

/// Extract hostname from a URL for scope enforcement.
fn extract_host_from_url(url: &str) -> PyResult<String> {
    let parsed = url::Url::parse(url)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid URL: {}", e)))?;

    parsed
        .host_str()
        .map(|h| h.to_string())
        .ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err("URL does not contain a valid host")
        })
}
