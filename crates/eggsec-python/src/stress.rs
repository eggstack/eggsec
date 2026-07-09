use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use crate::error::EggsecResultExt;
use crate::runtime_async;
use crate::runtime_sync;

/// Stress test type enum.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StressTypePy {
    Syn,
    Udp,
    Http,
    Tcp,
    Icmp,
}

#[pymethods]
impl StressTypePy {
    fn __repr__(&self) -> String {
        format!("StressType.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl StressTypePy {
    fn as_str(&self) -> &str {
        match self {
            StressTypePy::Syn => "SYN flood",
            StressTypePy::Udp => "UDP flood",
            StressTypePy::Http => "HTTP flood",
            StressTypePy::Tcp => "TCP flood",
            StressTypePy::Icmp => "ICMP flood",
        }
    }

    fn from_engine(engine: eggsec::stress::StressType) -> Self {
        match engine {
            eggsec::stress::StressType::Syn => StressTypePy::Syn,
            eggsec::stress::StressType::Udp => StressTypePy::Udp,
            eggsec::stress::StressType::Http => StressTypePy::Http,
            eggsec::stress::StressType::Tcp => StressTypePy::Tcp,
            eggsec::stress::StressType::Icmp => StressTypePy::Icmp,
        }
    }

    fn to_engine(self) -> eggsec::stress::StressType {
        match self {
            StressTypePy::Syn => eggsec::stress::StressType::Syn,
            StressTypePy::Udp => eggsec::stress::StressType::Udp,
            StressTypePy::Http => eggsec::stress::StressType::Http,
            StressTypePy::Tcp => eggsec::stress::StressType::Tcp,
            StressTypePy::Icmp => eggsec::stress::StressType::Icmp,
        }
    }
}

/// Python-facing configuration for stress testing.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct StressConfigPy {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub port: u16,
    #[pyo3(get)]
    pub stress_type: StressTypePy,
    #[pyo3(get)]
    pub rate_pps: u64,
    #[pyo3(get)]
    pub duration_secs: u64,
    #[pyo3(get)]
    pub concurrency: usize,
    #[pyo3(get)]
    pub spoof_source: bool,
    #[pyo3(get)]
    pub spoof_range: Option<String>,
    #[pyo3(get)]
    pub random_source_port: bool,
    #[pyo3(get)]
    pub payload_size: usize,
    #[pyo3(get)]
    pub use_proxies: bool,
    #[pyo3(get)]
    pub proxy_pool: Option<String>,
}

#[pymethods]
impl StressConfigPy {
    #[new]
    #[pyo3(signature = (target, port, stress_type, rate_pps, duration_secs, *, concurrency=10, spoof_source=false, spoof_range=None, random_source_port=true, payload_size=64, use_proxies=false, proxy_pool=None))]
    fn new(
        target: String,
        port: u16,
        stress_type: StressTypePy,
        rate_pps: u64,
        duration_secs: u64,
        concurrency: usize,
        spoof_source: bool,
        spoof_range: Option<String>,
        random_source_port: bool,
        payload_size: usize,
        use_proxies: bool,
        proxy_pool: Option<String>,
    ) -> Self {
        Self {
            target,
            port,
            stress_type,
            rate_pps,
            duration_secs,
            concurrency,
            spoof_source,
            spoof_range,
            random_source_port,
            payload_size,
            use_proxies,
            proxy_pool,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("port", self.port)?;
        dict.set_item("stress_type", self.stress_type.as_str())?;
        dict.set_item("rate_pps", self.rate_pps)?;
        dict.set_item("duration_secs", self.duration_secs)?;
        dict.set_item("concurrency", self.concurrency)?;
        dict.set_item("spoof_source", self.spoof_source)?;
        dict.set_item("spoof_range", &self.spoof_range)?;
        dict.set_item("random_source_port", self.random_source_port)?;
        dict.set_item("payload_size", self.payload_size)?;
        dict.set_item("use_proxies", self.use_proxies)?;
        dict.set_item("proxy_pool", &self.proxy_pool)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        let engine = self.to_engine();
        serde_json::to_string(&engine)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "StressConfig(target={}, port={}, type={}, rate_pps={}, duration_secs={})",
            self.target,
            self.port,
            self.stress_type.as_str(),
            self.rate_pps,
            self.duration_secs
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl StressConfigPy {
    fn to_engine(&self) -> eggsec::stress::StressConfig {
        eggsec::stress::StressConfig {
            target: self.target.clone(),
            port: self.port,
            stress_type: self.stress_type.to_engine(),
            rate_pps: self.rate_pps,
            duration_secs: self.duration_secs,
            concurrency: self.concurrency,
            spoof_source: self.spoof_source,
            spoof_range: self.spoof_range.clone(),
            random_source_port: self.random_source_port,
            payload_size: self.payload_size,
            use_proxies: self.use_proxies,
            proxy_pool: self.proxy_pool.clone(),
        }
    }
}

/// Stress test statistics.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressStatsPy {
    #[pyo3(get)]
    pub duration_ms: u64,
    #[pyo3(get)]
    pub packets_sent: u64,
    #[pyo3(get)]
    pub bytes_sent: u64,
    #[pyo3(get)]
    pub errors: u64,
}

#[pymethods]
impl StressStatsPy {
    /// Average packets per second.
    #[getter]
    fn avg_rate_pps(&self) -> u64 {
        if self.duration_ms == 0 {
            return 0;
        }
        (self.packets_sent * 1000) / self.duration_ms
    }

    /// Average bandwidth in Mbps.
    #[getter]
    fn avg_bandwidth_mbps(&self) -> f64 {
        if self.duration_ms == 0 {
            return 0.0;
        }
        let bits = self.bytes_sent * 8;
        let seconds = self.duration_ms as f64 / 1000.0;
        bits as f64 / seconds / 1_000_000.0
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("duration_ms", self.duration_ms)?;
        dict.set_item("packets_sent", self.packets_sent)?;
        dict.set_item("bytes_sent", self.bytes_sent)?;
        dict.set_item("errors", self.errors)?;
        dict.set_item("avg_rate_pps", self.avg_rate_pps())?;
        dict.set_item("avg_bandwidth_mbps", self.avg_bandwidth_mbps())?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "StressStats(duration_ms={}, packets={}, bytes={}, errors={})",
            self.duration_ms, self.packets_sent, self.bytes_sent, self.errors
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Sent {} packets ({} bytes) in {}ms, {} errors, avg {:.0} pps",
            self.packets_sent,
            self.bytes_sent,
            self.duration_ms,
            self.errors,
            self.avg_rate_pps()
        )
    }
}

impl StressStatsPy {
    fn from_engine(engine: eggsec::stress::StressStats) -> Self {
        Self {
            duration_ms: engine.duration_ms,
            packets_sent: engine.packets_sent,
            bytes_sent: engine.bytes_sent,
            errors: engine.errors,
        }
    }
}

/// Stress test config summary.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressConfigSummaryPy {
    #[pyo3(get)]
    pub rate_pps: u64,
    #[pyo3(get)]
    pub duration_secs: u64,
    #[pyo3(get)]
    pub spoof_source: bool,
    #[pyo3(get)]
    pub used_proxies: bool,
}

#[pymethods]
impl StressConfigSummaryPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("rate_pps", self.rate_pps)?;
        dict.set_item("duration_secs", self.duration_secs)?;
        dict.set_item("spoof_source", self.spoof_source)?;
        dict.set_item("used_proxies", self.used_proxies)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "StressConfigSummary(rate_pps={}, duration_secs={}, spoof_source={}, used_proxies={})",
            self.rate_pps, self.duration_secs, self.spoof_source, self.used_proxies
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl StressConfigSummaryPy {
    fn from_engine(engine: eggsec::stress::StressConfigSummary) -> Self {
        Self {
            rate_pps: engine.rate_pps,
            duration_secs: engine.duration_secs,
            spoof_source: engine.spoof_source,
            used_proxies: engine.used_proxies,
        }
    }
}

/// Complete stress test result with stats, config summary, and warnings.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressResultPy {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub stress_type: StressTypePy,
    #[pyo3(get)]
    pub stats: StressStatsPy,
    #[pyo3(get)]
    pub config_used: StressConfigSummaryPy,
    pub(crate) warnings: Vec<String>,
}

#[pymethods]
impl StressResultPy {
    #[getter]
    fn warnings(&self, py: Python) -> PyResult<PyObject> {
        let list = PyList::new_bound(py, &self.warnings);
        Ok(list.into())
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("stress_type", self.stress_type.as_str())?;
        dict.set_item("stats", self.stats.to_dict(py)?)?;
        dict.set_item("config_used", self.config_used.to_dict(py)?)?;
        let warn_list = PyList::new_bound(py, &self.warnings);
        dict.set_item("warnings", &warn_list)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "StressResult(target={}, type={}, packets={}, errors={})",
            self.target,
            self.stress_type.as_str(),
            self.stats.packets_sent,
            self.stats.errors
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Stress test {} to {} sent {} packets in {}ms with {} errors",
            self.stress_type.as_str(),
            self.target,
            self.stats.packets_sent,
            self.stats.duration_ms,
            self.stats.errors
        )
    }
}

/// Parse a stress type string into StressTypePy.
fn parse_stress_type(s: &str) -> PyResult<StressTypePy> {
    match s.to_lowercase().as_str() {
        "syn" | "syn flood" => Ok(StressTypePy::Syn),
        "udp" | "udp flood" => Ok(StressTypePy::Udp),
        "http" | "http flood" => Ok(StressTypePy::Http),
        "tcp" | "tcp flood" => Ok(StressTypePy::Tcp),
        "icmp" | "icmp flood" => Ok(StressTypePy::Icmp),
        _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Unknown stress type: '{}'. Expected one of: syn, udp, http, tcp, icmp",
            s
        ))),
    }
}

/// Run a stress test (synchronous, non-interactive).
///
/// WARNING: Stress testing sends high volumes of traffic to test system resilience.
/// Only use against targets you have explicit written authorization to test.
/// Unauthorized use may violate laws.
///
/// This function uses non-interactive mode (no stdin confirmation) since Python
/// bindings are non-interactive. A warning is logged via tracing before execution.
///
/// Args:
///     target: Target IP or hostname.
///     port: Target port.
///     stress_type: Flood type - "syn", "udp", "http", "tcp", or "icmp".
///     rate_pps: Target packets per second.
///     duration_secs: Test duration in seconds.
///     concurrency: Number of concurrent workers (default 10).
///     spoof_source: Enable IP source spoofing (requires root, default False).
///
/// Returns:
///     StressResultPy: Test results including stats, config summary, and warnings.
///
/// Raises:
///     ValueError: If stress_type is invalid.
///     ScanError: If the stress test fails.
///     ScopeError: If target is not in authorized scope.
#[pyfunction]
#[pyo3(signature = (target, port, stress_type, rate_pps, duration_secs, *, concurrency=10, spoof_source=false))]
pub fn stress_test(
    py: Python<'_>,
    target: &str,
    port: u16,
    stress_type: &str,
    rate_pps: u64,
    duration_secs: u64,
    concurrency: usize,
    spoof_source: bool,
) -> PyResult<StressResultPy> {
    let stype = parse_stress_type(stress_type)?;
    let target_owned = target.to_string();

    tracing::warn!(
        target = %target,
        port = port,
        stress_type = %stype.as_str(),
        rate_pps = rate_pps,
        duration_secs = duration_secs,
        "Python stress test invoked — non-interactive mode"
    );

    let result = runtime_sync::block_on(py, async move {
        let config = eggsec::stress::StressConfig {
            target: target_owned,
            port,
            stress_type: stype.to_engine(),
            rate_pps,
            duration_secs,
            concurrency,
            spoof_source,
            spoof_range: None,
            random_source_port: true,
            payload_size: 64,
            use_proxies: false,
            proxy_pool: None,
        };

        let test = eggsec::stress::StressTest::new(config).map_pyerr()?;
        let stats = test.run_non_interactive().await.map_pyerr()?;

        Ok::<_, pyo3::PyErr>(stats)
    })?;

    Ok(StressResultPy {
        target: target.to_string(),
        stress_type: stype,
        stats: StressStatsPy::from_engine(result),
        config_used: StressConfigSummaryPy {
            rate_pps,
            duration_secs,
            spoof_source,
            used_proxies: false,
        },
        warnings: vec![
            "Stress testing sends high-volume traffic. Ensure you have authorization.".into(),
        ],
    })
}

/// Run a stress test (asynchronous, non-interactive).
///
/// WARNING: Stress testing sends high volumes of traffic to test system resilience.
/// Only use against targets you have explicit written authorization to test.
/// Unauthorized use may violate laws.
///
/// Returns a PyFuture that resolves to StressResultPy.
///
/// Args:
///     target: Target IP or hostname.
///     port: Target port.
///     stress_type: Flood type - "syn", "udp", "http", "tcp", or "icmp".
///     rate_pps: Target packets per second.
///     duration_secs: Test duration in seconds.
///     concurrency: Number of concurrent workers (default 10).
///     spoof_source: Enable IP source spoofing (requires root, default False).
///
/// Returns:
///     PyFuture: Awaitable that resolves to StressResultPy.
#[pyfunction]
#[pyo3(signature = (target, port, stress_type, rate_pps, duration_secs, *, concurrency=10, spoof_source=false))]
pub fn async_stress_test(
    target: &str,
    port: u16,
    stress_type: &str,
    rate_pps: u64,
    duration_secs: u64,
    concurrency: usize,
    spoof_source: bool,
) -> PyResult<runtime_async::PyFuture> {
    let stype = parse_stress_type(stress_type)?;
    let target_owned = target.to_string();
    let target_for_result = target_owned.clone();

    tracing::warn!(
        target = %target,
        port = port,
        stress_type = %stype.as_str(),
        rate_pps = rate_pps,
        duration_secs = duration_secs,
        "Python async stress test invoked — non-interactive mode"
    );

    runtime_async::spawn_async(async move {
        let config = eggsec::stress::StressConfig {
            target: target_owned,
            port,
            stress_type: stype.to_engine(),
            rate_pps,
            duration_secs,
            concurrency,
            spoof_source,
            spoof_range: None,
            random_source_port: true,
            payload_size: 64,
            use_proxies: false,
            proxy_pool: None,
        };

        let test = eggsec::stress::StressTest::new(config).map_pyerr()?;
        let stats = test.run_non_interactive().await.map_pyerr()?;

        Ok(StressResultPy {
            target: target_for_result,
            stress_type: stype,
            stats: StressStatsPy::from_engine(stats),
            config_used: StressConfigSummaryPy {
                rate_pps,
                duration_secs,
                spoof_source,
                used_proxies: false,
            },
            warnings: vec![
                "Stress testing sends high-volume traffic. Ensure you have authorization.".into(),
            ],
        })
    })
}
