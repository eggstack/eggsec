use pyo3::prelude::*;

use crate::dto::PortScanResult;
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
            eggsec::scanner::scan_ports(&target_owned, config).await
        })?;

        Ok(PortScanResult::from_engine(result))
    })
}
