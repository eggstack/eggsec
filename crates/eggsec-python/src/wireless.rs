use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use crate::finding::Severity;
use crate::runtime_sync;

/// WiFi security type.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecurityTypePy {
    Open,
    WEP,
    WPA,
    WPA2,
    WPA3,
    Enterprise,
    Unknown,
}

#[pymethods]
impl SecurityTypePy {
    fn __repr__(&self) -> String {
        format!("SecurityType.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl SecurityTypePy {
    fn as_str(&self) -> &str {
        match self {
            SecurityTypePy::Open => "Open",
            SecurityTypePy::WEP => "WEP",
            SecurityTypePy::WPA => "WPA",
            SecurityTypePy::WPA2 => "WPA2",
            SecurityTypePy::WPA3 => "WPA3",
            SecurityTypePy::Enterprise => "Enterprise",
            SecurityTypePy::Unknown => "Unknown",
        }
    }

    fn from_engine(engine: eggsec::wireless::SecurityType) -> Self {
        match engine {
            eggsec::wireless::SecurityType::Open => SecurityTypePy::Open,
            eggsec::wireless::SecurityType::WEP => SecurityTypePy::WEP,
            eggsec::wireless::SecurityType::WPA => SecurityTypePy::WPA,
            eggsec::wireless::SecurityType::WPA2 => SecurityTypePy::WPA2,
            eggsec::wireless::SecurityType::WPA3 => SecurityTypePy::WPA3,
            eggsec::wireless::SecurityType::Enterprise => SecurityTypePy::Enterprise,
            eggsec::wireless::SecurityType::Unknown => SecurityTypePy::Unknown,
        }
    }
}

/// A discovered wireless network.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WirelessNetworkPy {
    #[pyo3(get)]
    pub ssid: String,
    #[pyo3(get)]
    pub bssid: String,
    #[pyo3(get)]
    pub channel: u8,
    #[pyo3(get)]
    pub security_type: SecurityTypePy,
    #[pyo3(get)]
    pub signal_strength: i32,
    #[pyo3(get)]
    pub last_seen: String,
    #[pyo3(get)]
    pub wps_enabled: bool,
    #[pyo3(get)]
    pub is_hidden: bool,
    #[pyo3(get)]
    pub transition_mode: bool,
}

impl WirelessNetworkPy {
    fn from_engine(engine: eggsec::wireless::WirelessNetwork) -> Self {
        Self {
            ssid: engine.ssid,
            bssid: engine.bssid,
            channel: engine.channel,
            security_type: SecurityTypePy::from_engine(engine.security_type),
            signal_strength: engine.signal_strength,
            last_seen: engine.last_seen,
            wps_enabled: engine.wps_enabled,
            is_hidden: engine.is_hidden,
            transition_mode: engine.transition_mode,
        }
    }
}

#[pymethods]
impl WirelessNetworkPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("ssid", &self.ssid)?;
        dict.set_item("bssid", &self.bssid)?;
        dict.set_item("channel", self.channel)?;
        dict.set_item("security_type", self.security_type.as_str())?;
        dict.set_item("signal_strength", self.signal_strength)?;
        dict.set_item("last_seen", &self.last_seen)?;
        dict.set_item("wps_enabled", self.wps_enabled)?;
        dict.set_item("is_hidden", self.is_hidden)?;
        dict.set_item("transition_mode", self.transition_mode)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!("WirelessNetwork(ssid={}, bssid={})", self.ssid, self.bssid)
    }
}

/// A wireless vulnerability.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WirelessVulnerabilityPy {
    #[pyo3(get)]
    pub ssid: String,
    #[pyo3(get)]
    pub bssid: String,
    #[pyo3(get)]
    pub vulnerability_type: String,
    #[pyo3(get)]
    pub severity: Severity,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub recommendation: String,
}

impl WirelessVulnerabilityPy {
    fn from_engine(engine: eggsec::wireless::WirelessVulnerability) -> Self {
        Self {
            ssid: engine.ssid,
            bssid: engine.bssid,
            vulnerability_type: engine.vulnerability_type,
            severity: Severity::from_engine(engine.severity),
            description: engine.description,
            recommendation: engine.recommendation,
        }
    }
}

#[pymethods]
impl WirelessVulnerabilityPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("ssid", &self.ssid)?;
        dict.set_item("bssid", &self.bssid)?;
        dict.set_item("vulnerability_type", &self.vulnerability_type)?;
        dict.set_item("severity", self.severity.as_str())?;
        dict.set_item("description", &self.description)?;
        dict.set_item("recommendation", &self.recommendation)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "WirelessVulnerability(ssid={}, type={})",
            self.ssid, self.vulnerability_type
        )
    }
}

/// Result of a wireless scan.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WirelessScanResultPy {
    #[pyo3(get)]
    pub interface: String,
    networks: Vec<WirelessNetworkPy>,
    #[pyo3(get)]
    pub scan_duration_secs: u64,
    recommendations: Vec<String>,
}

impl WirelessScanResultPy {
    fn from_engine(engine: eggsec::wireless::WirelessScanResult) -> Self {
        Self {
            interface: engine.interface,
            networks: engine
                .networks
                .into_iter()
                .map(WirelessNetworkPy::from_engine)
                .collect(),
            scan_duration_secs: engine.scan_duration_secs,
            recommendations: engine.recommendations,
        }
    }
}

#[pymethods]
impl WirelessScanResultPy {
    #[getter]
    fn networks(&self) -> Vec<WirelessNetworkPy> {
        self.networks.clone()
    }

    #[getter]
    fn recommendations(&self) -> Vec<String> {
        self.recommendations.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("interface", &self.interface)?;
        dict.set_item("scan_duration_secs", self.scan_duration_secs)?;

        let nets_list = PyList::empty_bound(py);
        for n in &self.networks {
            nets_list.append(n.to_dict(py)?)?;
        }
        dict.set_item("networks", nets_list)?;
        dict.set_item("recommendations", &self.recommendations)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "WirelessScanResult(interface={}, networks={})",
            self.interface,
            self.networks.len()
        )
    }
}

/// Configuration for a wireless scan.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WirelessScanConfigPy {
    #[pyo3(get)]
    pub interface: Option<String>,
    #[pyo3(get)]
    pub duration_secs: u64,
}

#[pymethods]
impl WirelessScanConfigPy {
    #[new]
    #[pyo3(signature = (interface=None, duration_secs=30))]
    fn new(interface: Option<String>, duration_secs: u64) -> Self {
        Self {
            interface,
            duration_secs,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "WirelessScanConfig(interface={:?}, duration={})",
            self.interface, self.duration_secs
        )
    }
}

/// Run a wireless network scan.
///
/// Scans for nearby wireless networks and analyzes them for vulnerabilities.
/// Requires root privileges for real scanning.
///
/// Args:
///     config: Wireless scan configuration (optional).
///
/// Returns:
///     WirelessScanResultPy: Scan results with discovered networks.
///
/// Raises:
///     FeatureUnavailableError: If wireless feature is not enabled.
///     NetworkError: If the scan fails.
#[pyfunction]
#[pyo3(signature = (config=None))]
pub fn wireless_scan(config: Option<WirelessScanConfigPy>) -> PyResult<WirelessScanResultPy> {
    let cfg = config.unwrap_or(WirelessScanConfigPy {
        interface: None,
        duration_secs: 30,
    });

    Python::with_gil(|py| {
        let result = runtime_sync::block_on(py, async move {
            let mut scanner = eggsec::wireless::WirelessScanner::new();
            if let Some(iface) = &cfg.interface {
                scanner = scanner.with_interface(iface.clone());
            }
            scanner.scan(cfg.duration_secs).await.map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!("Wireless scan failed: {}", e))
            })
        })?;

        Ok(WirelessScanResultPy::from_engine(result))
    })
}

/// Run a wireless network scan (async).
///
/// Returns a PyFuture that resolves to a WirelessScanResultPy.
#[pyfunction]
#[pyo3(signature = (config=None))]
pub fn async_wireless_scan(
    config: Option<WirelessScanConfigPy>,
) -> PyResult<crate::runtime_async::PyFuture> {
    let cfg = config.unwrap_or(WirelessScanConfigPy {
        interface: None,
        duration_secs: 30,
    });

    crate::runtime_async::spawn_async(async move {
        let mut scanner = eggsec::wireless::WirelessScanner::new();
        if let Some(iface) = &cfg.interface {
            scanner = scanner.with_interface(iface.clone());
        }
        let result = scanner.scan(cfg.duration_secs).await.map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Wireless scan failed: {}", e))
        })?;
        Ok(WirelessScanResultPy::from_engine(result))
    })
}

/// Analyze wireless networks for vulnerabilities.
///
/// Performs passive analysis of discovered networks for known weaknesses.
///
/// Args:
///     networks: List of WirelessNetworkPy to analyze.
///     known_good: Optional set of known-good BSSIDs to exclude.
///
/// Returns:
///     List of WirelessVulnerabilityPy findings.
#[pyfunction]
#[pyo3(signature = (networks, known_good=None))]
pub fn wireless_analyze_networks(
    networks: Vec<WirelessNetworkPy>,
    known_good: Option<Vec<String>>,
) -> PyResult<Vec<WirelessVulnerabilityPy>> {
    let engine_networks: Vec<eggsec::wireless::WirelessNetwork> = networks
        .into_iter()
        .map(|n| eggsec::wireless::WirelessNetwork {
            ssid: n.ssid,
            bssid: n.bssid,
            channel: n.channel,
            security_type: match n.security_type {
                SecurityTypePy::Open => eggsec::wireless::SecurityType::Open,
                SecurityTypePy::WEP => eggsec::wireless::SecurityType::WEP,
                SecurityTypePy::WPA => eggsec::wireless::SecurityType::WPA,
                SecurityTypePy::WPA2 => eggsec::wireless::SecurityType::WPA2,
                SecurityTypePy::WPA3 => eggsec::wireless::SecurityType::WPA3,
                SecurityTypePy::Enterprise => eggsec::wireless::SecurityType::Enterprise,
                SecurityTypePy::Unknown => eggsec::wireless::SecurityType::Unknown,
            },
            signal_strength: n.signal_strength,
            last_seen: n.last_seen,
            wps_enabled: n.wps_enabled,
            is_hidden: n.is_hidden,
            transition_mode: n.transition_mode,
        })
        .collect();

    let known_good_set: Option<std::collections::HashSet<String>> =
        known_good.map(|v| v.into_iter().collect());

    let vulns = eggsec::wireless::WirelessScanner::analyze_networks(
        &engine_networks,
        known_good_set.as_ref(),
    );

    Ok(vulns
        .into_iter()
        .map(WirelessVulnerabilityPy::from_engine)
        .collect())
}
