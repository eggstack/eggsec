use pyo3::prelude::*;

use crate::dto::PortScanResult;
use crate::endpoint::EndpointScanResult;
use crate::error::EggsecResultExt;
use crate::fingerprint::FingerprintScanResult;
use crate::runtime_async;
use crate::runtime_sync;
use crate::scope::Scope;

/// Perform a scoped TCP port scan.
///
/// This is a convenience function that creates an ephemeral Client internally.
/// For repeated scans, prefer creating a Client to reuse scope configuration.
///
/// Args:
///     target: Hostname or IP to scan.
///     ports: List of port numbers to scan.
///     scope: Scope defining authorized targets.
///     concurrency: Max concurrent connections (default: 100).
///     timeout_ms: Connection timeout in milliseconds (default: 5000).
///
/// Returns:
///     PortScanResult: Structured scan results.
///
/// Raises:
///     EnforcementError: If the target is outside the allowed scope.
///     ScanError: If the scan fails.
///
/// Example:
///     >>> import eggsec
///     >>> result = eggsec.scan_ports(
///     ...     target="127.0.0.1",
///     ...     ports=[22, 80, 443],
///     ...     scope=eggsec.Scope.allow_hosts(["127.0.0.1"]),
///     ... )
///     >>> print(result.open_ports)
#[pyfunction]
#[pyo3(signature = (target, ports, scope, *, concurrency=100, timeout_ms=5000))]
pub fn scan_ports(
    target: &str,
    ports: Vec<u16>,
    scope: Scope,
    concurrency: usize,
    timeout_ms: u64,
) -> PyResult<PortScanResult> {
    scope.enforce_target(target)?;

    for &port in &ports {
        scope.enforce_port(port)?;
    }

    let config = eggsec::scanner::PortScanConfig {
        ports,
        concurrency,
        timeout_duration: std::time::Duration::from_millis(timeout_ms),
        tui_mode: false,
        spoof_config: eggsec::scanner::SpoofConfig::default(),
        progress_tx: None,
        max_results: None,
    };

    let target_owned = target.to_string();

    Python::with_gil(|py| {
        let result = runtime_sync::block_on(py, async move {
            eggsec::scanner::scan_ports(&target_owned, config)
                .await
                .map_pyerr()
        })?;

        Ok(PortScanResult::from_engine(result))
    })
}

/// Perform an async scoped TCP port scan.
///
/// Returns a PyFuture that can be awaited in Python.
#[pyfunction]
#[pyo3(signature = (target, ports, scope, *, concurrency=100, timeout_ms=5000))]
pub fn async_scan_ports(
    target: &str,
    ports: Vec<u16>,
    scope: Scope,
    concurrency: usize,
    timeout_ms: u64,
) -> PyResult<crate::runtime_async::PyFuture> {
    scope.enforce_target(target)?;

    for &port in &ports {
        scope.enforce_port(port)?;
    }

    let target_owned = target.to_string();
    let ports_owned = ports;

    runtime_async::spawn_async(async move {
        let config = eggsec::scanner::PortScanConfig {
            ports: ports_owned,
            concurrency,
            timeout_duration: std::time::Duration::from_millis(timeout_ms),
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

/// Perform endpoint discovery against a web server.
///
/// Args:
///     base_url: Base URL to scan (e.g. "https://example.com").
///     endpoints: List of paths to probe (e.g. ["admin", "login"]).
///     scope: Scope defining authorized targets.
///     concurrency: Max concurrent requests (default: 20).
///     timeout_ms: Request timeout in milliseconds (default: 30000).
///     include_404: Include 404 responses (default: False).
///     verify_tls: Verify TLS certificates (default: True).
///
/// Returns:
///     EndpointScanResult: Structured endpoint discovery results.
#[pyfunction]
#[pyo3(signature = (base_url, endpoints, scope, *, concurrency=20, timeout_ms=30000, include_404=false, verify_tls=true))]
pub fn scan_endpoints(
    base_url: &str,
    endpoints: Vec<String>,
    scope: Scope,
    concurrency: usize,
    timeout_ms: u64,
    include_404: bool,
    verify_tls: bool,
) -> PyResult<EndpointScanResult> {
    let host = extract_host_from_url(base_url)?;
    scope.enforce_target(&host)?;

    let config = eggsec::scanner::EndpointScanConfig {
        base_url: base_url.to_string(),
        endpoints,
        concurrency,
        timeout_duration: std::time::Duration::from_millis(timeout_ms),
        include_404,
        tui_mode: false,
        spoof_config: std::sync::Arc::new(eggsec::scanner::SpoofConfig::default()),
        verify_tls,
        progress_tx: None,
        max_results: None,
    };

    Python::with_gil(|py| {
        let result = runtime_sync::block_on(py, async move {
            eggsec::scanner::scan_endpoints(config)
                .await
                .map_pyerr()
        })?;

        Ok(EndpointScanResult::from_engine(result))
    })
}

/// Perform async endpoint discovery.
#[pyfunction]
#[pyo3(signature = (base_url, endpoints, scope, *, concurrency=20, timeout_ms=30000, include_404=false, verify_tls=true))]
pub fn async_scan_endpoints(
    base_url: &str,
    endpoints: Vec<String>,
    scope: Scope,
    concurrency: usize,
    timeout_ms: u64,
    include_404: bool,
    verify_tls: bool,
) -> PyResult<crate::runtime_async::PyFuture> {
    let host = extract_host_from_url(base_url)?;
    scope.enforce_target(&host)?;

    let base_url_owned = base_url.to_string();

    runtime_async::spawn_async(async move {
        let config = eggsec::scanner::EndpointScanConfig {
            base_url: base_url_owned,
            endpoints,
            concurrency,
            timeout_duration: std::time::Duration::from_millis(timeout_ms),
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

/// Perform service fingerprinting on target ports.
///
/// Args:
///     target: Hostname or IP to fingerprint.
///     ports: List of port numbers to fingerprint.
///     scope: Scope defining authorized targets.
///     concurrency: Max concurrent connections (default: 100).
///     timeout_ms: Connection timeout in milliseconds (default: 2000).
///
/// Returns:
///     FingerprintScanResult: Structured fingerprinting results.
#[pyfunction]
#[pyo3(signature = (target, ports, scope, *, concurrency=100, timeout_ms=2000))]
pub fn fingerprint_services(
    target: &str,
    ports: Vec<u16>,
    scope: Scope,
    concurrency: usize,
    timeout_ms: u64,
) -> PyResult<FingerprintScanResult> {
    scope.enforce_target(target)?;

    for &port in &ports {
        scope.enforce_port(port)?;
    }

    let target_owned = target.to_string();

    Python::with_gil(|py| {
        let result = runtime_sync::block_on(py, async move {
            eggsec::scanner::fingerprint_services(
                &target_owned,
                ports,
                std::time::Duration::from_millis(timeout_ms),
                false,
                concurrency,
                None,
                None,
            )
            .await
            .map_pyerr()
        })?;

        Ok(FingerprintScanResult::from_engine(result))
    })
}

/// Perform async service fingerprinting.
#[pyfunction]
#[pyo3(signature = (target, ports, scope, *, concurrency=100, timeout_ms=2000))]
pub fn async_fingerprint_services(
    target: &str,
    ports: Vec<u16>,
    scope: Scope,
    concurrency: usize,
    timeout_ms: u64,
) -> PyResult<crate::runtime_async::PyFuture> {
    scope.enforce_target(target)?;

    for &port in &ports {
        scope.enforce_port(port)?;
    }

    let target_owned = target.to_string();
    let ports_owned = ports;

    runtime_async::spawn_async(async move {
        let result = eggsec::scanner::fingerprint_services(
            &target_owned,
            ports_owned,
            std::time::Duration::from_millis(timeout_ms),
            false,
            concurrency,
            None,
            None,
        )
        .await
        .map_pyerr()?;
        Ok(FingerprintScanResult::from_engine(result))
    })
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
