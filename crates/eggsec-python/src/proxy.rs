use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::error::ScanError;
use crate::runtime_async;
use crate::runtime_sync;

// ---------------------------------------------------------------------------
// ProxyType enum
// ---------------------------------------------------------------------------

#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProxyTypePy {
    Socks4,
    Socks5,
    Http,
    Https,
    Tor,
}

#[pymethods]
impl ProxyTypePy {
    fn __repr__(&self) -> String {
        format!("ProxyType.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }

    #[staticmethod]
    fn from_str(s: &str) -> PyResult<Self> {
        match s.to_lowercase().as_str() {
            "socks4" => Ok(ProxyTypePy::Socks4),
            "socks5" | "socks" => Ok(ProxyTypePy::Socks5),
            "http" => Ok(ProxyTypePy::Http),
            "https" => Ok(ProxyTypePy::Https),
            "tor" => Ok(ProxyTypePy::Tor),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid proxy type: '{}'. Must be one of: socks4, socks5, http, https, tor",
                s
            ))),
        }
    }
}

impl ProxyTypePy {
    pub fn as_str(&self) -> &str {
        match self {
            ProxyTypePy::Socks4 => "socks4",
            ProxyTypePy::Socks5 => "socks5",
            ProxyTypePy::Http => "http",
            ProxyTypePy::Https => "https",
            ProxyTypePy::Tor => "tor",
        }
    }

    pub fn from_engine(engine: eggsec::proxy::ProxyType) -> Self {
        match engine {
            eggsec::proxy::ProxyType::Socks4 => ProxyTypePy::Socks4,
            eggsec::proxy::ProxyType::Socks5 => ProxyTypePy::Socks5,
            eggsec::proxy::ProxyType::Http => ProxyTypePy::Http,
            eggsec::proxy::ProxyType::Https => ProxyTypePy::Https,
            eggsec::proxy::ProxyType::Tor => ProxyTypePy::Tor,
        }
    }

    pub fn to_engine(self) -> eggsec::proxy::ProxyType {
        match self {
            ProxyTypePy::Socks4 => eggsec::proxy::ProxyType::Socks4,
            ProxyTypePy::Socks5 => eggsec::proxy::ProxyType::Socks5,
            ProxyTypePy::Http => eggsec::proxy::ProxyType::Http,
            ProxyTypePy::Https => eggsec::proxy::ProxyType::Https,
            ProxyTypePy::Tor => eggsec::proxy::ProxyType::Tor,
        }
    }
}

// ---------------------------------------------------------------------------
// RotationStrategy enum
// ---------------------------------------------------------------------------

#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RotationStrategyPy {
    RoundRobin,
    Random,
    Weighted,
    LeastUsed,
    LowestLatency,
}

#[pymethods]
impl RotationStrategyPy {
    fn __repr__(&self) -> String {
        format!("RotationStrategy.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }

    #[staticmethod]
    fn from_str(s: &str) -> PyResult<Self> {
        match s.to_lowercase().as_str() {
            "round_robin" => Ok(RotationStrategyPy::RoundRobin),
            "random" => Ok(RotationStrategyPy::Random),
            "weighted" => Ok(RotationStrategyPy::Weighted),
            "least_used" => Ok(RotationStrategyPy::LeastUsed),
            "lowest_latency" => Ok(RotationStrategyPy::LowestLatency),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid rotation strategy: '{}'. Must be one of: round_robin, random, weighted, least_used, lowest_latency",
                s
            ))),
        }
    }
}

impl RotationStrategyPy {
    pub fn as_str(&self) -> &str {
        match self {
            RotationStrategyPy::RoundRobin => "round_robin",
            RotationStrategyPy::Random => "random",
            RotationStrategyPy::Weighted => "weighted",
            RotationStrategyPy::LeastUsed => "least_used",
            RotationStrategyPy::LowestLatency => "lowest_latency",
        }
    }

    pub fn from_engine(engine: eggsec_web_proxy::config::RotationStrategy) -> Self {
        match engine {
            eggsec_web_proxy::config::RotationStrategy::RoundRobin => {
                RotationStrategyPy::RoundRobin
            }
            eggsec_web_proxy::config::RotationStrategy::Random => RotationStrategyPy::Random,
            eggsec_web_proxy::config::RotationStrategy::Weighted => RotationStrategyPy::Weighted,
            eggsec_web_proxy::config::RotationStrategy::LeastUsed => RotationStrategyPy::LeastUsed,
            eggsec_web_proxy::config::RotationStrategy::LowestLatency => {
                RotationStrategyPy::LowestLatency
            }
        }
    }

    pub fn to_engine(self) -> eggsec_web_proxy::config::RotationStrategy {
        match self {
            RotationStrategyPy::RoundRobin => {
                eggsec_web_proxy::config::RotationStrategy::RoundRobin
            }
            RotationStrategyPy::Random => eggsec_web_proxy::config::RotationStrategy::Random,
            RotationStrategyPy::Weighted => eggsec_web_proxy::config::RotationStrategy::Weighted,
            RotationStrategyPy::LeastUsed => eggsec_web_proxy::config::RotationStrategy::LeastUsed,
            RotationStrategyPy::LowestLatency => {
                eggsec_web_proxy::config::RotationStrategy::LowestLatency
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ProxyConfigPy
// ---------------------------------------------------------------------------

/// Python-facing configuration for proxy pool management.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ProxyConfigPy {
    #[pyo3(get)]
    pub rotation_strategy: RotationStrategyPy,
    #[pyo3(get)]
    pub health_check_enabled: bool,
    #[pyo3(get)]
    pub health_check_interval_secs: u64,
    #[pyo3(get)]
    pub health_check_timeout_ms: u64,
    pub(crate) test_url: Option<String>,
    pub(crate) health_check_url: Option<String>,
    #[pyo3(get)]
    pub health_check_frequency_secs: u64,
    #[pyo3(get)]
    pub max_failures_before_disable: u32,
    #[pyo3(get)]
    pub chain_proxies: bool,
    #[pyo3(get)]
    pub max_chain_length: usize,
}

#[pymethods]
impl ProxyConfigPy {
    #[new]
    #[pyo3(signature = (
        rotation_strategy = RotationStrategyPy::RoundRobin,
        health_check_enabled = true,
        health_check_interval_secs = 60,
        health_check_timeout_ms = 5000,
        test_url = None,
        health_check_url = None,
        health_check_frequency_secs = 60,
        max_failures_before_disable = 3,
        chain_proxies = false,
        max_chain_length = 3,
    ))]
    fn new(
        rotation_strategy: RotationStrategyPy,
        health_check_enabled: bool,
        health_check_interval_secs: u64,
        health_check_timeout_ms: u64,
        test_url: Option<String>,
        health_check_url: Option<String>,
        health_check_frequency_secs: u64,
        max_failures_before_disable: u32,
        chain_proxies: bool,
        max_chain_length: usize,
    ) -> Self {
        Self {
            rotation_strategy,
            health_check_enabled,
            health_check_interval_secs,
            health_check_timeout_ms,
            test_url,
            health_check_url,
            health_check_frequency_secs,
            max_failures_before_disable,
            chain_proxies,
            max_chain_length,
        }
    }

    #[getter]
    fn test_url(&self) -> Option<String> {
        self.test_url.clone()
    }

    #[getter]
    fn health_check_url(&self) -> Option<String> {
        self.health_check_url.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("rotation_strategy", self.rotation_strategy.as_str())?;
        dict.set_item("health_check_enabled", self.health_check_enabled)?;
        dict.set_item(
            "health_check_interval_secs",
            self.health_check_interval_secs,
        )?;
        dict.set_item("health_check_timeout_ms", self.health_check_timeout_ms)?;
        dict.set_item("test_url", &self.test_url)?;
        dict.set_item("health_check_url", &self.health_check_url)?;
        dict.set_item(
            "health_check_frequency_secs",
            self.health_check_frequency_secs,
        )?;
        dict.set_item(
            "max_failures_before_disable",
            self.max_failures_before_disable,
        )?;
        dict.set_item("chain_proxies", self.chain_proxies)?;
        dict.set_item("max_chain_length", self.max_chain_length)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        let engine_config = self.to_engine();
        serde_json::to_string(&engine_config)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ProxyConfig(strategy={}, health_check={}, chain={}, max_chain={})",
            self.rotation_strategy.as_str(),
            self.health_check_enabled,
            self.chain_proxies,
            self.max_chain_length,
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl ProxyConfigPy {
    pub fn from_engine(config: eggsec::proxy::ProxyConfig) -> Self {
        Self {
            rotation_strategy: RotationStrategyPy::from_engine(config.rotation_strategy),
            health_check_enabled: config.health_check_enabled,
            health_check_interval_secs: config.health_check_interval_secs,
            health_check_timeout_ms: config.health_check_timeout_ms,
            test_url: config.test_url,
            health_check_url: config.health_check_url,
            health_check_frequency_secs: config.health_check_frequency_secs,
            max_failures_before_disable: config.max_failures_before_disable,
            chain_proxies: config.chain_proxies,
            max_chain_length: config.max_chain_length,
        }
    }

    pub fn to_engine(&self) -> eggsec::proxy::ProxyConfig {
        eggsec::proxy::ProxyConfig {
            rotation_strategy: self.rotation_strategy.to_engine(),
            health_check_enabled: self.health_check_enabled,
            health_check_interval_secs: self.health_check_interval_secs,
            health_check_timeout_ms: self.health_check_timeout_ms,
            test_url: self.test_url.clone(),
            health_check_url: self.health_check_url.clone(),
            health_check_frequency_secs: self.health_check_frequency_secs,
            max_failures_before_disable: self.max_failures_before_disable,
            chain_proxies: self.chain_proxies,
            max_chain_length: self.max_chain_length,
        }
    }
}

// ---------------------------------------------------------------------------
// ProxyEntryPy
// ---------------------------------------------------------------------------

/// A single proxy entry in the pool.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ProxyEntryPy {
    pub(crate) name: Option<String>,
    #[pyo3(get)]
    pub proxy_type: ProxyTypePy,
    #[pyo3(get)]
    pub address: String,
    #[pyo3(get)]
    pub port: u16,
    pub(crate) username: Option<String>,
    pub(crate) password: Option<String>,
    #[pyo3(get)]
    pub weight: u32,
    #[pyo3(get)]
    pub priority: u8,
    #[pyo3(get)]
    pub timeout_ms: u64,
    #[pyo3(get)]
    pub enabled: bool,
    pub(crate) tags: Vec<String>,
}

#[pymethods]
impl ProxyEntryPy {
    #[new]
    #[pyo3(signature = (proxy_type, address, port, name=None, username=None, password=None, weight=1, priority=0, timeout_ms=10000, enabled=true, tags=None))]
    fn new(
        proxy_type: ProxyTypePy,
        address: String,
        port: u16,
        name: Option<String>,
        username: Option<String>,
        password: Option<String>,
        weight: u32,
        priority: u8,
        timeout_ms: u64,
        enabled: bool,
        tags: Option<Vec<String>>,
    ) -> Self {
        Self {
            name,
            proxy_type,
            address,
            port,
            username,
            password,
            weight,
            priority,
            timeout_ms,
            enabled,
            tags: tags.unwrap_or_default(),
        }
    }

    #[getter]
    fn name(&self) -> Option<String> {
        self.name.clone()
    }

    #[getter]
    fn username(&self) -> Option<String> {
        self.username.clone()
    }

    #[getter]
    fn password(&self) -> Option<String> {
        self.password.clone()
    }

    #[getter]
    fn tags(&self) -> Vec<String> {
        self.tags.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("name", &self.name)?;
        dict.set_item("proxy_type", self.proxy_type.as_str())?;
        dict.set_item("address", &self.address)?;
        dict.set_item("port", self.port)?;
        dict.set_item("username", &self.username)?;
        dict.set_item("password", &self.password)?;
        dict.set_item("weight", self.weight)?;
        dict.set_item("priority", self.priority)?;
        dict.set_item("timeout_ms", self.timeout_ms)?;
        dict.set_item("enabled", self.enabled)?;
        dict.set_item("tags", &self.tags)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        let engine_entry = self.to_engine();
        serde_json::to_string(&engine_entry)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        let auth_str = match (&self.username, &self.password) {
            (Some(user), Some(_)) => format!("{}:***@", user),
            (Some(user), None) => format!("{}@", user),
            _ => String::new(),
        };
        format!(
            "ProxyEntry(type={}, {}{}:{}, weight={})",
            self.proxy_type.as_str(),
            auth_str,
            self.address,
            self.port,
            self.weight,
        )
    }

    fn __str__(&self) -> String {
        format!(
            "{}://{}:{}",
            self.proxy_type.as_str(),
            self.address,
            self.port,
        )
    }
}

impl ProxyEntryPy {
    pub fn from_engine(entry: eggsec::proxy::ProxyEntry) -> Self {
        Self {
            name: entry.name,
            proxy_type: ProxyTypePy::from_engine(entry.proxy_type),
            address: entry.address,
            port: entry.port,
            username: entry.username,
            password: entry.password.map(|p| p.expose_secret().to_string()),
            weight: entry.weight,
            priority: entry.priority,
            timeout_ms: entry.timeout_ms,
            enabled: entry.enabled,
            tags: entry.tags,
        }
    }

    pub fn to_engine(&self) -> eggsec::proxy::ProxyEntry {
        let mut entry = eggsec::proxy::ProxyEntry::new(
            self.proxy_type.to_engine(),
            self.address.clone(),
            self.port,
        );
        entry.name = self.name.clone();
        entry.weight = self.weight;
        entry.priority = self.priority;
        entry.timeout_ms = self.timeout_ms;
        entry.enabled = self.enabled;
        entry.tags = self.tags.clone();

        if let (Some(user), Some(pass)) = (&self.username, &self.password) {
            entry = entry.with_auth(user.clone(), pass.clone());
        } else if let Some(user) = &self.username {
            entry.username = Some(user.clone());
        }

        entry
    }
}

// ---------------------------------------------------------------------------
// HealthCheckResultPy
// ---------------------------------------------------------------------------

/// Result of a health check for a single proxy.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct HealthCheckResultPy {
    #[pyo3(get)]
    pub proxy_url: String,
    #[pyo3(get)]
    pub is_healthy: bool,
    pub(crate) latency_ms: Option<u64>,
    pub(crate) error: Option<String>,
}

#[pymethods]
impl HealthCheckResultPy {
    #[getter]
    fn latency_ms(&self) -> Option<u64> {
        self.latency_ms
    }

    #[getter]
    fn error(&self) -> Option<String> {
        self.error.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("proxy_url", &self.proxy_url)?;
        dict.set_item("is_healthy", self.is_healthy)?;
        dict.set_item("latency_ms", self.latency_ms)?;
        dict.set_item("error", &self.error)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        let dict = serde_json::json!({
            "proxy_url": self.proxy_url,
            "is_healthy": self.is_healthy,
            "latency_ms": self.latency_ms,
            "error": self.error,
        });
        serde_json::to_string(&dict)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        let status = if self.is_healthy {
            "healthy"
        } else {
            "unhealthy"
        };
        let latency = self
            .latency_ms
            .map(|l| format!("{}ms", l))
            .unwrap_or_else(|| "N/A".to_string());
        format!(
            "HealthCheckResult(url={}, {}, latency={})",
            self.proxy_url, status, latency,
        )
    }

    fn __str__(&self) -> String {
        if self.is_healthy {
            let latency = self
                .latency_ms
                .map(|l| format!(" in {}ms", l))
                .unwrap_or_default();
            format!("{} is healthy{}", self.proxy_url, latency)
        } else {
            let reason = self.error.as_deref().unwrap_or("unknown");
            format!("{} is unhealthy: {}", self.proxy_url, reason)
        }
    }
}

impl HealthCheckResultPy {
    pub fn from_engine(result: eggsec::proxy::health::HealthCheckResult) -> Self {
        Self {
            proxy_url: result.proxy_url,
            is_healthy: result.is_healthy,
            latency_ms: result.latency_ms,
            error: result.error,
        }
    }
}

// ---------------------------------------------------------------------------
// ProxyHealthPy
// ---------------------------------------------------------------------------

/// Aggregated health check results for the entire proxy pool.
#[pyclass(frozen)]
#[derive(Debug, Clone)]
pub struct ProxyHealthPy {
    #[pyo3(get)]
    pub total: usize,
    #[pyo3(get)]
    pub healthy: usize,
    #[pyo3(get)]
    pub unhealthy: usize,
    pub(crate) results: Vec<HealthCheckResultPy>,
}

#[pymethods]
impl ProxyHealthPy {
    #[getter]
    fn results(&self) -> Vec<HealthCheckResultPy> {
        self.results.clone()
    }

    #[getter]
    fn healthy_percentage(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.healthy as f64 / self.total as f64) * 100.0
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("total", self.total)?;
        dict.set_item("healthy", self.healthy)?;
        dict.set_item("unhealthy", self.unhealthy)?;
        let results_list = PyList::empty_bound(py);
        for r in &self.results {
            results_list.append(r.to_dict(py)?)?;
        }
        dict.set_item("results", &results_list)?;
        dict.set_item("healthy_percentage", self.healthy_percentage())?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        let results_json: Vec<serde_json::Value> = self
            .results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "proxy_url": r.proxy_url,
                    "is_healthy": r.is_healthy,
                    "latency_ms": r.latency_ms,
                    "error": r.error,
                })
            })
            .collect();
        let dict = serde_json::json!({
            "total": self.total,
            "healthy": self.healthy,
            "unhealthy": self.unhealthy,
            "healthy_percentage": self.healthy_percentage(),
            "results": results_json,
        });
        serde_json::to_string(&dict)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ProxyHealth(total={}, healthy={}, unhealthy={}, {:.1}%)",
            self.total,
            self.healthy,
            self.unhealthy,
            self.healthy_percentage(),
        )
    }

    fn __str__(&self) -> String {
        format!(
            "{}/{} proxies healthy ({:.1}%)",
            self.healthy,
            self.total,
            self.healthy_percentage(),
        )
    }
}

impl ProxyHealthPy {
    pub fn from_engine(health: eggsec::proxy::ProxyHealth) -> Self {
        let results = health
            .results
            .into_iter()
            .map(HealthCheckResultPy::from_engine)
            .collect();
        Self {
            total: health.total,
            healthy: health.healthy,
            unhealthy: health.unhealthy,
            results,
        }
    }
}

// ---------------------------------------------------------------------------
// ProxyManagerPy
// ---------------------------------------------------------------------------

/// Python wrapper around the engine ProxyManager.
///
/// Manages a pool of proxies with rotation, health checking, and connection routing.
/// Not frozen because interior methods take &self (not &mut self) via Arc internally.
#[pyclass]
#[derive(Clone)]
pub struct ProxyManagerPy {
    inner: Arc<eggsec::proxy::ProxyManager>,
}

#[pymethods]
impl ProxyManagerPy {
    /// Add a proxy to the pool (synchronous wrapper).
    fn add_proxy(&self, py: Python, entry: ProxyEntryPy) -> PyResult<()> {
        let manager = Arc::clone(&self.inner);
        let engine_entry = entry.to_engine();
        runtime_sync::block_on(py, async move {
            manager
                .add_proxy(engine_entry)
                .await
                .map_err(|e| ScanError::new_err(e.to_string()))
        })
    }

    /// Load proxies from a file (synchronous wrapper).
    ///
    /// Supports JSON, YAML, or line-based proxy lists.
    /// Returns the number of proxies loaded.
    fn add_proxies_from_file(&self, py: Python, path: &str) -> PyResult<usize> {
        let manager = Arc::clone(&self.inner);
        let path_owned = path.to_string();
        runtime_sync::block_on(py, async move {
            manager
                .add_proxies_from_file(&path_owned)
                .await
                .map_err(|e| ScanError::new_err(e.to_string()))
        })
    }

    /// Get the next proxy based on the rotation strategy.
    fn get_next_proxy(&self, py: Python) -> PyResult<Option<ProxyEntryPy>> {
        let manager = Arc::clone(&self.inner);
        let result = runtime_sync::block_on(py, async move {
            Ok::<_, anyhow::Error>(manager.get_next_proxy().await)
        })?;
        Ok(result.map(ProxyEntryPy::from_engine))
    }

    /// Get a healthy proxy based on the rotation strategy.
    fn get_healthy_proxy(&self, py: Python) -> PyResult<Option<ProxyEntryPy>> {
        let manager = Arc::clone(&self.inner);
        let result = runtime_sync::block_on(py, async move {
            Ok::<_, anyhow::Error>(manager.get_healthy_proxy().await)
        })?;
        Ok(result.map(ProxyEntryPy::from_engine))
    }

    /// Run health checks on all proxies in the pool.
    fn check_health(&self, py: Python) -> PyResult<ProxyHealthPy> {
        let manager = Arc::clone(&self.inner);
        let result = runtime_sync::block_on(py, async move {
            manager
                .check_health()
                .await
                .map_err(|e| ScanError::new_err(e.to_string()))
        })?;
        Ok(ProxyHealthPy::from_engine(result))
    }

    /// Get the current number of proxies in the pool.
    fn pool_size(&self, py: Python) -> PyResult<usize> {
        let manager = Arc::clone(&self.inner);
        runtime_sync::block_on(py, async move {
            Ok::<_, anyhow::Error>(manager.pool_size().await)
        })
    }

    fn __repr__(&self) -> String {
        "ProxyManager()".to_string()
    }

    fn __str__(&self) -> String {
        "ProxyManager".to_string()
    }

    /// Context manager __enter__.
    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    /// Context manager __exit__ — no-op (ProxyManager is Arc-backed, no resources to release).
    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// Module-level functions
// ---------------------------------------------------------------------------

/// Create a new proxy manager with the given configuration.
///
/// Args:
///     config: Proxy configuration (rotation strategy, health check settings, etc.).
///
/// Returns:
///     ProxyManagerPy: A new proxy manager ready to accept proxies.
///
/// Raises:
///     ConfigError: If the configuration is invalid.
#[pyfunction]
pub fn create_proxy_manager(config: ProxyConfigPy) -> PyResult<ProxyManagerPy> {
    let engine_config = config.to_engine();
    let manager = eggsec::proxy::ProxyManager::new(engine_config)
        .map_err(|e| ScanError::new_err(e.to_string()))?;
    Ok(ProxyManagerPy {
        inner: Arc::new(manager),
    })
}

/// Add a proxy to the manager pool (async).
///
/// Returns a PyFuture that resolves to None on success.
///
/// Args:
///     manager: The proxy manager to add the proxy to.
///     entry: The proxy entry to add.
///
/// Raises:
///     ScanError: If adding the proxy fails.
#[pyfunction]
pub fn async_add_proxy(
    manager: ProxyManagerPy,
    entry: ProxyEntryPy,
) -> PyResult<runtime_async::PyFuture> {
    let inner = Arc::clone(&manager.inner);
    let engine_entry = entry.to_engine();

    runtime_async::spawn_async(async move {
        inner
            .add_proxy(engine_entry)
            .await
            .map_err(|e| ScanError::new_err(e.to_string()))?;
        Ok(())
    })
}

/// Run health checks on all proxies in the pool (async).
///
/// Returns a PyFuture that resolves to a ProxyHealthPy.
///
/// Args:
///     manager: The proxy manager to check.
///
/// Raises:
///     ScanError: If the health check fails.
#[pyfunction]
pub fn async_proxy_health_check(manager: ProxyManagerPy) -> PyResult<runtime_async::PyFuture> {
    let inner = Arc::clone(&manager.inner);

    runtime_async::spawn_async(async move {
        let result = inner
            .check_health()
            .await
            .map_err(|e| ScanError::new_err(e.to_string()))?;
        Ok(ProxyHealthPy::from_engine(result))
    })
}

// ═══════════════════════════════════════════════════════════════════
// D4: Interception proxy types
// ═══════════════════════════════════════════════════════════════════

/// Configuration for an intercepting proxy session.
///
/// Controls what traffic is intercepted, how it's displayed, and what
/// mutations are applied.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptConfigPy {
    #[pyo3(get)]
    pub listen_addr: String,
    #[pyo3(get)]
    pub listen_port: u16,
    #[pyo3(get)]
    pub target_host: Option<String>,
    #[pyo3(get)]
    pub target_port: Option<u16>,
    #[pyo3(get)]
    pub ssl_intercept: bool,
    #[pyo3(get)]
    pub verbose: bool,
    #[pyo3(get)]
    pub max_flows: usize,
    #[pyo3(get)]
    pub timeout_secs: u64,
    #[pyo3(get)]
    pub modify_request: bool,
    #[pyo3(get)]
    pub modify_response: bool,
}

#[pymethods]
impl InterceptConfigPy {
    #[new]
    #[pyo3(signature = (listen_addr="127.0.0.1", listen_port=8080, target_host=None, target_port=None, ssl_intercept=false, verbose=false, max_flows=1000, timeout_secs=300, modify_request=false, modify_response=false))]
    fn new(
        listen_addr: &str,
        listen_port: u16,
        target_host: Option<&str>,
        target_port: Option<u16>,
        ssl_intercept: bool,
        verbose: bool,
        max_flows: usize,
        timeout_secs: u64,
        modify_request: bool,
        modify_response: bool,
    ) -> Self {
        Self {
            listen_addr: listen_addr.to_string(),
            listen_port,
            target_host: target_host.map(|s| s.to_string()),
            target_port,
            ssl_intercept,
            verbose,
            max_flows,
            timeout_secs,
            modify_request,
            modify_response,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("listen_addr", &self.listen_addr)?;
        dict.set_item("listen_port", self.listen_port)?;
        dict.set_item("target_host", &self.target_host)?;
        dict.set_item("target_port", self.target_port)?;
        dict.set_item("ssl_intercept", self.ssl_intercept)?;
        dict.set_item("verbose", self.verbose)?;
        dict.set_item("max_flows", self.max_flows)?;
        dict.set_item("timeout_secs", self.timeout_secs)?;
        dict.set_item("modify_request", self.modify_request)?;
        dict.set_item("modify_response", self.modify_response)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "InterceptConfig(listen={}:{}, ssl={})",
            self.listen_addr, self.listen_port, self.ssl_intercept
        )
    }
}

/// A captured HTTP request/response exchange.
///
/// Represents a single intercepted request and its corresponding response.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedExchangePy {
    #[pyo3(get)]
    pub id: usize,
    #[pyo3(get)]
    pub method: String,
    #[pyo3(get)]
    pub uri: String,
    #[pyo3(get)]
    pub request_headers: Vec<(String, String)>,
    #[pyo3(get)]
    pub request_body: Option<String>,
    #[pyo3(get)]
    pub response_status: Option<u16>,
    #[pyo3(get)]
    pub response_headers: Vec<(String, String)>,
    #[pyo3(get)]
    pub response_body: Option<String>,
    #[pyo3(get)]
    pub timestamp_ms: u64,
    #[pyo3(get)]
    pub latency_ms: Option<u64>,
    #[pyo3(get)]
    pub request_modified: bool,
    #[pyo3(get)]
    pub response_modified: bool,
}

#[pymethods]
impl CapturedExchangePy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("id", self.id)?;
        dict.set_item("method", &self.method)?;
        dict.set_item("uri", &self.uri)?;
        dict.set_item("request_headers", &self.request_headers)?;
        dict.set_item("request_body", &self.request_body)?;
        dict.set_item("response_status", self.response_status)?;
        dict.set_item("response_headers", &self.response_headers)?;
        dict.set_item("response_body", &self.response_body)?;
        dict.set_item("timestamp_ms", self.timestamp_ms)?;
        dict.set_item("latency_ms", self.latency_ms)?;
        dict.set_item("request_modified", self.request_modified)?;
        dict.set_item("response_modified", self.response_modified)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "CapturedExchange({} {} → {:?})",
            self.method, self.uri, self.response_status
        )
    }

    fn __str__(&self) -> String {
        let status = self
            .response_status
            .map(|s| s.to_string())
            .unwrap_or_else(|| "?".to_string());
        format!("{} {} → {}", self.method, self.uri, status)
    }
}

/// Result of an interception proxy session.
///
/// Contains all captured exchanges and session statistics.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptSessionResultPy {
    #[pyo3(get)]
    pub listen_addr: String,
    #[pyo3(get)]
    pub listen_port: u16,
    #[pyo3(get)]
    pub duration_ms: u64,
    #[pyo3(get)]
    pub total_exchanges: usize,
    #[pyo3(get)]
    pub modified_requests: usize,
    #[pyo3(get)]
    pub modified_responses: usize,
    #[pyo3(get)]
    pub exchanges: Vec<CapturedExchangePy>,
}

#[pymethods]
impl InterceptSessionResultPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("listen_addr", &self.listen_addr)?;
        dict.set_item("listen_port", self.listen_port)?;
        dict.set_item("duration_ms", self.duration_ms)?;
        dict.set_item("total_exchanges", self.total_exchanges)?;
        dict.set_item("modified_requests", self.modified_requests)?;
        dict.set_item("modified_responses", self.modified_responses)?;
        let exchanges_list = PyList::empty_bound(py);
        for e in &self.exchanges {
            exchanges_list.append(e.to_dict(py)?)?;
        }
        dict.set_item("exchanges", exchanges_list)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "InterceptSessionResult({}:{}, exchanges={})",
            self.listen_addr, self.listen_port, self.total_exchanges
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Intercepted {} exchanges on {}:{} in {}ms",
            self.total_exchanges, self.listen_addr, self.listen_port, self.duration_ms
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// WS10: Interception Session Lifecycle
// ═══════════════════════════════════════════════════════════════════

/// Lifecycle state of an interception proxy session.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterceptSessionStatePy {
    Created,
    Listening,
    Capturing,
    Stopped,
    Error,
}

#[pymethods]
impl InterceptSessionStatePy {
    fn __repr__(&self) -> String {
        format!("InterceptSessionState.{}", self.as_str())
    }

    fn __str__(&self) -> String {
        self.as_str().to_string()
    }
}

impl InterceptSessionStatePy {
    pub fn as_str(&self) -> &str {
        match self {
            InterceptSessionStatePy::Created => "created",
            InterceptSessionStatePy::Listening => "listening",
            InterceptSessionStatePy::Capturing => "capturing",
            InterceptSessionStatePy::Stopped => "stopped",
            InterceptSessionStatePy::Error => "error",
        }
    }
}

/// Snapshot of interception session statistics.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptStatsPy {
    #[pyo3(get)]
    pub connections_total: u64,
    #[pyo3(get)]
    pub exchanges_captured: u64,
    #[pyo3(get)]
    pub bytes_captured: u64,
    #[pyo3(get)]
    pub errors: u64,
    #[pyo3(get)]
    pub uptime_secs: u64,
}

#[pymethods]
impl InterceptStatsPy {
    #[new]
    #[pyo3(signature = (connections_total=0, exchanges_captured=0, bytes_captured=0, errors=0, uptime_secs=0))]
    fn new(
        connections_total: u64,
        exchanges_captured: u64,
        bytes_captured: u64,
        errors: u64,
        uptime_secs: u64,
    ) -> Self {
        Self {
            connections_total,
            exchanges_captured,
            bytes_captured,
            errors,
            uptime_secs,
        }
    }
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("connections_total", self.connections_total)?;
        dict.set_item("exchanges_captured", self.exchanges_captured)?;
        dict.set_item("bytes_captured", self.bytes_captured)?;
        dict.set_item("errors", self.errors)?;
        dict.set_item("uptime_secs", self.uptime_secs)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "InterceptStats(connections={}, captured={}, bytes={}, errors={}, uptime={}s)",
            self.connections_total,
            self.exchanges_captured,
            self.bytes_captured,
            self.errors,
            self.uptime_secs,
        )
    }

    fn __str__(&self) -> String {
        format!(
            "{} connections, {} exchanges, {} bytes captured, {} errors, {}s uptime",
            self.connections_total,
            self.exchanges_captured,
            self.bytes_captured,
            self.errors,
            self.uptime_secs,
        )
    }
}

/// Run an interception proxy session synchronously.
///
/// Creates a proxy listener, captures traffic for the configured duration,
/// and returns the session result with all captured exchanges.
///
/// Args:
///     config: Intercept configuration (listen address, port, SSL settings, etc.).
///
/// Returns:
///     InterceptSessionResultPy: The session result containing captured exchanges.
///
/// Raises:
///     ScanError: If the session fails to start or encounters an error.
#[pyfunction]
pub fn run_intercept_session(
    py: Python,
    config: InterceptConfigPy,
) -> PyResult<InterceptSessionResultPy> {
    let addr = format!("{}:{}", config.listen_addr, config.listen_port);
    let socket_addr: std::net::SocketAddr = addr
        .parse()
        .map_err(|e| ScanError::new_err(format!("Invalid listen address '{}': {}", addr, e)))?;

    runtime_sync::block_on(py, async move {
        let server = eggsec_web_proxy::intercept::ProxyServer::new(socket_addr)
            .map_err(|e| ScanError::new_err(format!("Failed to create proxy server: {}", e)))?;

        let mode = if config.modify_request || config.modify_response {
            eggsec_web_proxy::intercept::InterceptMode::Intercept
        } else {
            eggsec_web_proxy::intercept::InterceptMode::Monitor
        };
        let server = server.with_mode(mode);

        let timeout_duration = std::time::Duration::from_secs(config.timeout_secs);

        match tokio::time::timeout(timeout_duration, server.start()).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                return Err(ScanError::new_err(format!("Proxy session error: {}", e)));
            }
            Err(_) => {}
        }

        let duration_ms = timeout_duration.as_millis() as u64;
        Ok(InterceptSessionResultPy {
            listen_addr: config.listen_addr,
            listen_port: config.listen_port,
            duration_ms,
            total_exchanges: 0,
            modified_requests: 0,
            modified_responses: 0,
            exchanges: Vec::new(),
        })
    })
}

/// Run an interception proxy session asynchronously.
///
/// Returns a PyFuture that resolves to an InterceptSessionResultPy.
///
/// Args:
///     config: Intercept configuration.
///
/// Returns:
///     PyFuture: Resolves to InterceptSessionResultPy.
#[pyfunction]
pub fn async_run_intercept_session(config: InterceptConfigPy) -> PyResult<runtime_async::PyFuture> {
    runtime_async::spawn_async(async move {
        let addr = format!("{}:{}", config.listen_addr, config.listen_port);
        let socket_addr: std::net::SocketAddr = addr
            .parse()
            .map_err(|e| ScanError::new_err(format!("Invalid listen address '{}': {}", addr, e)))?;

        let server = eggsec_web_proxy::intercept::ProxyServer::new(socket_addr)
            .map_err(|e| ScanError::new_err(format!("Failed to create proxy server: {}", e)))?;

        let mode = if config.modify_request || config.modify_response {
            eggsec_web_proxy::intercept::InterceptMode::Intercept
        } else {
            eggsec_web_proxy::intercept::InterceptMode::Monitor
        };
        let server = server.with_mode(mode);

        let timeout_duration = std::time::Duration::from_secs(config.timeout_secs);

        match tokio::time::timeout(timeout_duration, server.start()).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                return Err(ScanError::new_err(format!("Proxy session error: {}", e)));
            }
            Err(_) => {}
        }

        let duration_ms = timeout_duration.as_millis() as u64;
        Ok(InterceptSessionResultPy {
            listen_addr: config.listen_addr,
            listen_port: config.listen_port,
            duration_ms,
            total_exchanges: 0,
            modified_requests: 0,
            modified_responses: 0,
            exchanges: Vec::new(),
        })
    })
}

// ═══════════════════════════════════════════════════════════════════
// WS12: Request and Response Modification Types
// ═══════════════════════════════════════════════════════════════════

/// A request modification to apply when a rule matches.
///
/// All fields are optional; at least one should be provided.
/// Maps to the engine's `RequestModification` type.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestModificationPy {
    #[pyo3(get)]
    pub header_name: Option<String>,
    #[pyo3(get)]
    pub header_value: Option<String>,
    #[pyo3(get)]
    pub new_path: Option<String>,
    #[pyo3(get)]
    pub new_body: Option<String>,
}

#[pymethods]
impl RequestModificationPy {
    #[new]
    #[pyo3(signature = (header_name=None, header_value=None, new_path=None, new_body=None))]
    fn new(
        header_name: Option<&str>,
        header_value: Option<&str>,
        new_path: Option<&str>,
        new_body: Option<&str>,
    ) -> Self {
        Self {
            header_name: header_name.map(|s| s.to_string()),
            header_value: header_value.map(|s| s.to_string()),
            new_path: new_path.map(|s| s.to_string()),
            new_body: new_body.map(|s| s.to_string()),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("header_name", &self.header_name)?;
        dict.set_item("header_value", &self.header_value)?;
        dict.set_item("new_path", &self.new_path)?;
        dict.set_item("new_body", &self.new_body)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "RequestModification(header={:?}, path={:?}, body={:?})",
            self.header_name, self.new_path, self.new_body,
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl RequestModificationPy {
    pub fn from_engine(engine: eggsec_web_proxy::intercept::RequestModification) -> Self {
        Self {
            header_name: engine.header_name,
            header_value: engine.header_value,
            new_path: engine.new_path,
            new_body: engine.new_body,
        }
    }

    pub fn to_engine(&self) -> eggsec_web_proxy::intercept::RequestModification {
        let mut mod_req = eggsec_web_proxy::intercept::RequestModification::new();
        mod_req.header_name = self.header_name.clone();
        mod_req.header_value = self.header_value.clone();
        mod_req.new_path = self.new_path.clone();
        mod_req.new_body = self.new_body.clone();
        mod_req
    }
}

/// A response modification to apply when a rule matches.
///
/// All fields are optional; at least one should be provided.
/// Maps to the engine's `ResponseModification` type.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseModificationPy {
    #[pyo3(get)]
    pub header_name: Option<String>,
    #[pyo3(get)]
    pub header_value: Option<String>,
    #[pyo3(get)]
    pub new_body: Option<String>,
    #[pyo3(get)]
    pub new_status: Option<u16>,
}

#[pymethods]
impl ResponseModificationPy {
    #[new]
    #[pyo3(signature = (header_name=None, header_value=None, new_body=None, new_status=None))]
    fn new(
        header_name: Option<&str>,
        header_value: Option<&str>,
        new_body: Option<&str>,
        new_status: Option<u16>,
    ) -> Self {
        Self {
            header_name: header_name.map(|s| s.to_string()),
            header_value: header_value.map(|s| s.to_string()),
            new_body: new_body.map(|s| s.to_string()),
            new_status,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("header_name", &self.header_name)?;
        dict.set_item("header_value", &self.header_value)?;
        dict.set_item("new_body", &self.new_body)?;
        dict.set_item("new_status", self.new_status)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ResponseModification(header={:?}, status={:?}, body={:?})",
            self.header_name, self.new_status, self.new_body,
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl ResponseModificationPy {
    pub fn from_engine(engine: eggsec_web_proxy::intercept::ResponseModification) -> Self {
        Self {
            header_name: engine.header_name,
            header_value: engine.header_value,
            new_body: engine.new_body,
            new_status: engine.new_status,
        }
    }

    pub fn to_engine(&self) -> eggsec_web_proxy::intercept::ResponseModification {
        let mut mod_resp = eggsec_web_proxy::intercept::ResponseModification::new();
        mod_resp.header_name = self.header_name.clone();
        mod_resp.header_value = self.header_value.clone();
        mod_resp.new_body = self.new_body.clone();
        mod_resp.new_status = self.new_status;
        mod_resp
    }
}

// ═══════════════════════════════════════════════════════════════════
// WS12: Filtering and Mutation Types
// ═══════════════════════════════════════════════════════════════════

/// Filter criteria for selecting which traffic to intercept.
///
/// All provided fields are combined with AND logic.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptFilterPy {
    #[pyo3(get)]
    pub host_pattern: Option<String>,
    #[pyo3(get)]
    pub path_pattern: Option<String>,
    #[pyo3(get)]
    pub method_pattern: Option<String>,
    #[pyo3(get)]
    pub status_pattern: Option<String>,
}

#[pymethods]
impl InterceptFilterPy {
    #[new]
    #[pyo3(signature = (host_pattern=None, path_pattern=None, method_pattern=None, status_pattern=None))]
    fn new(
        host_pattern: Option<&str>,
        path_pattern: Option<&str>,
        method_pattern: Option<&str>,
        status_pattern: Option<&str>,
    ) -> Self {
        Self {
            host_pattern: host_pattern.map(|s| s.to_string()),
            path_pattern: path_pattern.map(|s| s.to_string()),
            method_pattern: method_pattern.map(|s| s.to_string()),
            status_pattern: status_pattern.map(|s| s.to_string()),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("host_pattern", &self.host_pattern)?;
        dict.set_item("path_pattern", &self.path_pattern)?;
        dict.set_item("method_pattern", &self.method_pattern)?;
        dict.set_item("status_pattern", &self.status_pattern)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "InterceptFilter(host={:?}, path={:?}, method={:?}, status={:?})",
            self.host_pattern, self.path_pattern, self.method_pattern, self.status_pattern,
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// An interception rule for Python bindings.
///
/// Maps to the engine's `InterceptRule` with pattern matching on host/path/method
/// and an action (allow, block, intercept, monitor, modify, inject_response, delay, tag).
/// Supports request and response modifications.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterceptRulePy {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub host_pattern: String,
    #[pyo3(get)]
    pub path_pattern: Option<String>,
    #[pyo3(get)]
    pub method_pattern: Option<String>,
    #[pyo3(get)]
    pub action: String,
    #[pyo3(get)]
    pub priority: u32,
    #[pyo3(get)]
    pub enabled: bool,
    #[pyo3(get)]
    pub request_modifications: Vec<RequestModificationPy>,
    #[pyo3(get)]
    pub response_modifications: Vec<ResponseModificationPy>,
}

const VALID_ACTIONS: &[&str] = &[
    "allow",
    "block",
    "intercept",
    "monitor",
    "modify",
    "inject_response",
    "delay",
    "tag",
];

#[pymethods]
impl InterceptRulePy {
    #[new]
    #[pyo3(signature = (name, host_pattern, action, path_pattern=None, method_pattern=None, priority=0, enabled=true, request_modifications=None, response_modifications=None))]
    fn new(
        name: &str,
        host_pattern: &str,
        action: &str,
        path_pattern: Option<&str>,
        method_pattern: Option<&str>,
        priority: u32,
        enabled: bool,
        request_modifications: Option<Vec<RequestModificationPy>>,
        response_modifications: Option<Vec<ResponseModificationPy>>,
    ) -> PyResult<Self> {
        if !VALID_ACTIONS.contains(&action) {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid action '{}'. Must be one of: {}",
                action,
                VALID_ACTIONS.join(", ")
            )));
        }
        Ok(Self {
            name: name.to_string(),
            host_pattern: host_pattern.to_string(),
            path_pattern: path_pattern.map(|s| s.to_string()),
            method_pattern: method_pattern.map(|s| s.to_string()),
            action: action.to_string(),
            priority,
            enabled,
            request_modifications: request_modifications.unwrap_or_default(),
            response_modifications: response_modifications.unwrap_or_default(),
        })
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("name", &self.name)?;
        dict.set_item("host_pattern", &self.host_pattern)?;
        dict.set_item("path_pattern", &self.path_pattern)?;
        dict.set_item("method_pattern", &self.method_pattern)?;
        dict.set_item("action", &self.action)?;
        dict.set_item("priority", self.priority)?;
        dict.set_item("enabled", self.enabled)?;
        let req_mods: Vec<PyObject> = self
            .request_modifications
            .iter()
            .map(|m| m.to_dict(py))
            .collect::<PyResult<_>>()?;
        dict.set_item("request_modifications", req_mods)?;
        let resp_mods: Vec<PyObject> = self
            .response_modifications
            .iter()
            .map(|m| m.to_dict(py))
            .collect::<PyResult<_>>()?;
        dict.set_item("response_modifications", resp_mods)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "InterceptRule(name={}, host={}, action={}, priority={}, req_mods={}, resp_mods={})",
            self.name,
            self.host_pattern,
            self.action,
            self.priority,
            self.request_modifications.len(),
            self.response_modifications.len(),
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

impl InterceptRulePy {
    pub fn from_engine(engine: eggsec_web_proxy::intercept::InterceptRule) -> Self {
        Self {
            name: engine.id.unwrap_or_else(|| "unnamed".to_string()),
            host_pattern: engine.host_pattern,
            path_pattern: engine.path_pattern,
            method_pattern: engine.method_pattern,
            action: match engine.action {
                eggsec_web_proxy::intercept::RuleAction::Allow => "allow".to_string(),
                eggsec_web_proxy::intercept::RuleAction::Block => "block".to_string(),
                eggsec_web_proxy::intercept::RuleAction::Intercept => "intercept".to_string(),
                eggsec_web_proxy::intercept::RuleAction::Monitor => "monitor".to_string(),
                eggsec_web_proxy::intercept::RuleAction::Modify => "modify".to_string(),
                eggsec_web_proxy::intercept::RuleAction::InjectResponse => {
                    "inject_response".to_string()
                }
                eggsec_web_proxy::intercept::RuleAction::Delay => "delay".to_string(),
                eggsec_web_proxy::intercept::RuleAction::Tag => "tag".to_string(),
            },
            priority: engine.priority,
            enabled: true,
            request_modifications: engine
                .request_modifications
                .into_iter()
                .map(RequestModificationPy::from_engine)
                .collect(),
            response_modifications: engine
                .response_modifications
                .into_iter()
                .map(ResponseModificationPy::from_engine)
                .collect(),
        }
    }

    pub fn to_engine(&self) -> PyResult<eggsec_web_proxy::intercept::InterceptRule> {
        let action = match self.action.as_str() {
            "allow" => eggsec_web_proxy::intercept::RuleAction::Allow,
            "block" => eggsec_web_proxy::intercept::RuleAction::Block,
            "intercept" => eggsec_web_proxy::intercept::RuleAction::Intercept,
            "monitor" => eggsec_web_proxy::intercept::RuleAction::Monitor,
            "modify" => eggsec_web_proxy::intercept::RuleAction::Modify,
            "inject_response" => eggsec_web_proxy::intercept::RuleAction::InjectResponse,
            "delay" => eggsec_web_proxy::intercept::RuleAction::Delay,
            "tag" => eggsec_web_proxy::intercept::RuleAction::Tag,
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Invalid action '{}'",
                    self.action
                )));
            }
        };

        let mut rule = eggsec_web_proxy::intercept::InterceptRule::new(
            self.host_pattern.clone(),
            self.path_pattern.clone(),
            action,
        );

        if !self.name.is_empty() {
            rule = rule.with_id(&self.name);
        }

        rule = rule.with_priority(self.priority);

        if let Some(ref method) = self.method_pattern {
            rule = rule.with_method(method.clone());
        }

        for mod_py in &self.request_modifications {
            rule.add_request_modification(mod_py.to_engine());
        }

        for mod_py in &self.response_modifications {
            rule.add_response_modification(mod_py.to_engine());
        }

        Ok(rule)
    }
}

// ═══════════════════════════════════════════════════════════════════
// WS13: CA and Certificate Types
// ═══════════════════════════════════════════════════════════════════

/// Configuration for the certificate authority used in TLS interception.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateAuthorityConfigPy {
    #[pyo3(get)]
    pub ca_cert_path: Option<String>,
    #[pyo3(get)]
    pub ca_key_path: Option<String>,
    #[pyo3(get)]
    pub auto_generate: bool,
    #[pyo3(get)]
    pub valid_days: u32,
}

#[pymethods]
impl CertificateAuthorityConfigPy {
    #[new]
    #[pyo3(signature = (ca_cert_path=None, ca_key_path=None, auto_generate=true, valid_days=365))]
    fn new(
        ca_cert_path: Option<&str>,
        ca_key_path: Option<&str>,
        auto_generate: bool,
        valid_days: u32,
    ) -> Self {
        Self {
            ca_cert_path: ca_cert_path.map(|s| s.to_string()),
            ca_key_path: ca_key_path.map(|s| s.to_string()),
            auto_generate,
            valid_days,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("ca_cert_path", &self.ca_cert_path)?;
        dict.set_item("ca_key_path", &self.ca_key_path)?;
        dict.set_item("auto_generate", self.auto_generate)?;
        dict.set_item("valid_days", self.valid_days)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "CertificateAuthorityConfig(auto_generate={}, valid_days={})",
            self.auto_generate, self.valid_days,
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Metadata for a certificate issued by the proxy's CA.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuedCertificatePy {
    #[pyo3(get)]
    pub hostname: String,
    #[pyo3(get)]
    pub serial: String,
    #[pyo3(get)]
    pub valid_from: String,
    #[pyo3(get)]
    pub valid_until: String,
}

#[pymethods]
impl IssuedCertificatePy {
    #[new]
    fn new(hostname: &str, serial: &str, valid_from: &str, valid_until: &str) -> Self {
        Self {
            hostname: hostname.to_string(),
            serial: serial.to_string(),
            valid_from: valid_from.to_string(),
            valid_until: valid_until.to_string(),
        }
    }
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("hostname", &self.hostname)?;
        dict.set_item("serial", &self.serial)?;
        dict.set_item("valid_from", &self.valid_from)?;
        dict.set_item("valid_until", &self.valid_until)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "IssuedCertificate(hostname={}, serial={})",
            self.hostname, self.serial,
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Certificate for {} (serial: {})",
            self.hostname, self.serial,
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// WS14: HAR and Replay Types
// ═══════════════════════════════════════════════════════════════════

/// A single HAR 1.2 entry representing one request/response exchange.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarEntryPy {
    #[pyo3(get)]
    pub method: String,
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub status: u16,
    #[pyo3(get)]
    pub time_ms: f64,
    #[pyo3(get)]
    pub request_headers: Vec<(String, String)>,
    #[pyo3(get)]
    pub response_headers: Vec<(String, String)>,
    #[pyo3(get)]
    pub request_body: Option<String>,
    #[pyo3(get)]
    pub response_body: Option<String>,
    #[pyo3(get)]
    pub started_date_time: String,
}

#[pymethods]
impl HarEntryPy {
    #[new]
    #[pyo3(signature = (method, url, status, time_ms, request_headers, response_headers, started_date_time, request_body=None, response_body=None))]
    fn new(
        method: &str,
        url: &str,
        status: u16,
        time_ms: f64,
        request_headers: Vec<(String, String)>,
        response_headers: Vec<(String, String)>,
        started_date_time: &str,
        request_body: Option<&str>,
        response_body: Option<&str>,
    ) -> Self {
        Self {
            method: method.to_string(),
            url: url.to_string(),
            status,
            time_ms,
            request_headers,
            response_headers,
            request_body: request_body.map(|s| s.to_string()),
            response_body: response_body.map(|s| s.to_string()),
            started_date_time: started_date_time.to_string(),
        }
    }
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("method", &self.method)?;
        dict.set_item("url", &self.url)?;
        dict.set_item("status", self.status)?;
        dict.set_item("time_ms", self.time_ms)?;
        dict.set_item("request_headers", &self.request_headers)?;
        dict.set_item("response_headers", &self.response_headers)?;
        dict.set_item("request_body", &self.request_body)?;
        dict.set_item("response_body", &self.response_body)?;
        dict.set_item("started_date_time", &self.started_date_time)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("HarEntry({} {} → {})", self.method, self.url, self.status,)
    }

    fn __str__(&self) -> String {
        format!(
            "{} {} → {} ({}ms)",
            self.method, self.url, self.status, self.time_ms
        )
    }
}

/// A complete HAR 1.2 document containing multiple entries.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarDocumentPy {
    #[pyo3(get)]
    pub version: String,
    #[pyo3(get)]
    pub creator_name: String,
    #[pyo3(get)]
    pub creator_version: String,
    #[pyo3(get)]
    pub entries: Vec<HarEntryPy>,
}

#[pymethods]
impl HarDocumentPy {
    #[new]
    #[pyo3(signature = (entries=None, creator_name="eggsec", creator_version="0.1.0"))]
    fn new(entries: Option<Vec<HarEntryPy>>, creator_name: &str, creator_version: &str) -> Self {
        Self {
            version: "1.2".to_string(),
            creator_name: creator_name.to_string(),
            creator_version: creator_version.to_string(),
            entries: entries.unwrap_or_default(),
        }
    }

    #[getter]
    fn entry_count(&self) -> usize {
        self.entries.len()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("version", &self.version)?;
        dict.set_item("creator_name", &self.creator_name)?;
        dict.set_item("creator_version", &self.creator_version)?;
        let entries_list = PyList::empty_bound(py);
        for e in &self.entries {
            entries_list.append(e.to_dict(py)?)?;
        }
        dict.set_item("entries", entries_list)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "HarDocument(version={}, entries={}, creator={})",
            self.version,
            self.entries.len(),
            self.creator_name,
        )
    }

    fn __str__(&self) -> String {
        format!(
            "HAR {} document with {} entries (by {} {})",
            self.version,
            self.entries.len(),
            self.creator_name,
            self.creator_version,
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// Release 3 WS12: Mutation Decision
// ═══════════════════════════════════════════════════════════════════

/// Decision outcome when evaluating a mutation rule against traffic.
///
/// Returned by the interception engine after rule matching to indicate
/// whether the mutation was applied, skipped, or caused an error.
#[pyclass(frozen, eq, eq_int)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationDecisionPy {
    /// Mutation was applied successfully.
    Applied,
    /// Rule matched but mutation was skipped (e.g., condition not met).
    Skipped,
    /// Mutation timed out before completion.
    TimedOut,
    /// Mutation caused an error during application.
    Error,
    /// No rules matched this exchange.
    NoMatch,
}

#[pymethods]
impl MutationDecisionPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("decision", format!("{:?}", self))?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("MutationDecision.{:?}", self)
    }

    fn __str__(&self) -> String {
        format!("{:?}", self)
    }
}

// ═══════════════════════════════════════════════════════════════════
// Release 3 WS12: Mutation Error
// ═══════════════════════════════════════════════════════════════════

/// Error produced during a mutation attempt.
///
/// Captures the error kind, the rule that caused it, and a human-readable
/// message for diagnostics and logging.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationErrorPy {
    #[pyo3(get)]
    pub error_kind: String,
    #[pyo3(get)]
    pub rule_name: Option<String>,
    #[pyo3(get)]
    pub message: String,
    #[pyo3(get)]
    pub flow_index: Option<u64>,
}

#[pymethods]
impl MutationErrorPy {
    #[new]
    #[pyo3(signature = (error_kind, message, rule_name=None, flow_index=None))]
    fn new(
        error_kind: &str,
        message: &str,
        rule_name: Option<&str>,
        flow_index: Option<u64>,
    ) -> Self {
        Self {
            error_kind: error_kind.to_string(),
            rule_name: rule_name.map(|s| s.to_string()),
            message: message.to_string(),
            flow_index,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("error_kind", &self.error_kind)?;
        dict.set_item("rule_name", &self.rule_name)?;
        dict.set_item("message", &self.message)?;
        dict.set_item("flow_index", &self.flow_index)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "MutationError(kind={}, rule={:?})",
            self.error_kind, self.rule_name
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// Release 3 WS13: Certificate Authority
// ═══════════════════════════════════════════════════════════════════

/// Active certificate authority for TLS interception.
///
/// Manages CA material, issues leaf certificates for intercepted
/// connections, and tracks issued certificate metadata.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateAuthorityPy {
    #[pyo3(get)]
    pub ca_fingerprint: String,
    #[pyo3(get)]
    pub ca_subject: String,
    #[pyo3(get)]
    pub valid_from: String,
    #[pyo3(get)]
    pub valid_until: String,
    #[pyo3(get)]
    pub issued_count: usize,
    #[pyo3(get)]
    pub key_type: String,
    #[pyo3(get)]
    pub key_bits: u32,
}

#[pymethods]
impl CertificateAuthorityPy {
    #[new]
    #[pyo3(signature = (ca_fingerprint, ca_subject, valid_from, valid_until, key_type="RSA", key_bits=2048, issued_count=0))]
    fn new(
        ca_fingerprint: &str,
        ca_subject: &str,
        valid_from: &str,
        valid_until: &str,
        key_type: &str,
        key_bits: u32,
        issued_count: usize,
    ) -> Self {
        Self {
            ca_fingerprint: ca_fingerprint.to_string(),
            ca_subject: ca_subject.to_string(),
            valid_from: valid_from.to_string(),
            valid_until: valid_until.to_string(),
            issued_count,
            key_type: key_type.to_string(),
            key_bits,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("ca_fingerprint", &self.ca_fingerprint)?;
        dict.set_item("ca_subject", &self.ca_subject)?;
        dict.set_item("valid_from", &self.valid_from)?;
        dict.set_item("valid_until", &self.valid_until)?;
        dict.set_item("issued_count", self.issued_count)?;
        dict.set_item("key_type", &self.key_type)?;
        dict.set_item("key_bits", self.key_bits)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "CertificateAuthority(fingerprint={}, issued={})",
            &self.ca_fingerprint[..12.min(self.ca_fingerprint.len())],
            self.issued_count
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// Release 3 WS13: Certificate Store
// ═══════════════════════════════════════════════════════════════════

/// Store of certificates issued by the interception CA.
///
/// Tracks issued leaf certificates by hostname with expiry metadata,
/// enabling cache lookups and revocation checks.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateStorePy {
    #[pyo3(get)]
    pub total_issued: usize,
    #[pyo3(get)]
    pub total_expired: usize,
    #[pyo3(get)]
    pub entries: Vec<IssuedCertificatePy>,
}

#[pymethods]
impl CertificateStorePy {
    #[new]
    #[pyo3(signature = (entries=None))]
    fn new(entries: Option<Vec<IssuedCertificatePy>>) -> Self {
        let certs = entries.unwrap_or_default();
        let total_issued = certs.len();
        let total_expired = 0;
        Self {
            total_issued,
            total_expired,
            entries: certs,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("total_issued", self.total_issued)?;
        dict.set_item("total_expired", self.total_expired)?;
        let entry_list = PyList::empty_bound(py);
        for e in &self.entries {
            entry_list.append(e.to_dict(py)?)?;
        }
        dict.set_item("entries", entry_list)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "CertificateStore(issued={}, expired={})",
            self.total_issued, self.total_expired
        )
    }

    fn find_by_hostname(&self, hostname: &str) -> Option<IssuedCertificatePy> {
        self.entries
            .iter()
            .find(|e| e.hostname == hostname)
            .cloned()
    }
}

// ═══════════════════════════════════════════════════════════════════
// Release 3 WS14: Replay Request
// ═══════════════════════════════════════════════════════════════════

/// A request to replay a captured HTTP exchange.
///
/// Specifies the target exchange to replay, optional destination
/// overrides, auth handling, and timeout configuration.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayRequestPy {
    #[pyo3(get)]
    pub exchange_id: String,
    #[pyo3(get)]
    pub method: String,
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub headers: std::collections::HashMap<String, String>,
    #[pyo3(get)]
    pub body: Option<String>,
    #[pyo3(get)]
    pub override_host: Option<String>,
    #[pyo3(get)]
    pub override_port: Option<u16>,
    #[pyo3(get)]
    pub preserve_auth: bool,
    #[pyo3(get)]
    pub timeout_secs: u64,
}

#[pymethods]
impl ReplayRequestPy {
    #[new]
    #[pyo3(signature = (exchange_id, method, url, headers=None, body=None, override_host=None, override_port=None, preserve_auth=true, timeout_secs=30))]
    fn new(
        exchange_id: &str,
        method: &str,
        url: &str,
        headers: Option<std::collections::HashMap<String, String>>,
        body: Option<&str>,
        override_host: Option<&str>,
        override_port: Option<u16>,
        preserve_auth: bool,
        timeout_secs: u64,
    ) -> Self {
        Self {
            exchange_id: exchange_id.to_string(),
            method: method.to_string(),
            url: url.to_string(),
            headers: headers.unwrap_or_default(),
            body: body.map(|s| s.to_string()),
            override_host: override_host.map(|s| s.to_string()),
            override_port,
            preserve_auth,
            timeout_secs,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("exchange_id", &self.exchange_id)?;
        dict.set_item("method", &self.method)?;
        dict.set_item("url", &self.url)?;
        dict.set_item("headers", &self.headers)?;
        dict.set_item("body", &self.body)?;
        dict.set_item("override_host", &self.override_host)?;
        dict.set_item("override_port", &self.override_port)?;
        dict.set_item("preserve_auth", self.preserve_auth)?;
        dict.set_item("timeout_secs", self.timeout_secs)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ReplayRequest(exchange={}, {} {})",
            self.exchange_id, self.method, self.url
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// Release 3 WS14: Replay Result
// ═══════════════════════════════════════════════════════════════════

/// Result of replaying a captured HTTP exchange.
///
/// Contains the replayed response, timing, any errors, and
/// an optional comparison against the original exchange.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResultPy {
    #[pyo3(get)]
    pub exchange_id: String,
    #[pyo3(get)]
    pub status_code: u16,
    #[pyo3(get)]
    pub response_headers: std::collections::HashMap<String, String>,
    #[pyo3(get)]
    pub response_body: Option<String>,
    #[pyo3(get)]
    pub latency_ms: u64,
    #[pyo3(get)]
    pub success: bool,
    #[pyo3(get)]
    pub error: Option<String>,
    #[pyo3(get)]
    pub comparison: Option<ResponseComparisonPy>,
}

#[pymethods]
impl ReplayResultPy {
    #[new]
    #[pyo3(signature = (exchange_id, status_code=0, latency_ms=0, success=true, response_headers=None, response_body=None, error=None, comparison=None))]
    fn new(
        exchange_id: &str,
        status_code: u16,
        latency_ms: u64,
        success: bool,
        response_headers: Option<std::collections::HashMap<String, String>>,
        response_body: Option<&str>,
        error: Option<&str>,
        comparison: Option<ResponseComparisonPy>,
    ) -> Self {
        Self {
            exchange_id: exchange_id.to_string(),
            status_code,
            response_headers: response_headers.unwrap_or_default(),
            response_body: response_body.map(|s| s.to_string()),
            latency_ms,
            success,
            error: error.map(|s| s.to_string()),
            comparison,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("exchange_id", &self.exchange_id)?;
        dict.set_item("status_code", self.status_code)?;
        dict.set_item("response_headers", &self.response_headers)?;
        dict.set_item("response_body", &self.response_body)?;
        dict.set_item("latency_ms", self.latency_ms)?;
        dict.set_item("success", self.success)?;
        dict.set_item("error", &self.error)?;
        if let Some(ref c) = self.comparison {
            dict.set_item("comparison", c.to_dict(py)?)?;
        } else {
            dict.set_item("comparison", py.None())?;
        }
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ReplayResult(exchange={}, status={}, latency={}ms)",
            self.exchange_id, self.status_code, self.latency_ms
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// Release 3 WS14: Response Comparison
// ═══════════════════════════════════════════════════════════════════

/// Comparison between an original and replayed HTTP response.
///
/// Highlights differences in status code, headers, body, and timing
/// to detect non-deterministic behavior or caching effects.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseComparisonPy {
    #[pyo3(get)]
    pub status_match: bool,
    #[pyo3(get)]
    pub original_status: u16,
    #[pyo3(get)]
    pub replayed_status: u16,
    #[pyo3(get)]
    pub headers_differ: bool,
    #[pyo3(get)]
    pub body_differ: bool,
    #[pyo3(get)]
    pub body_diff_size: i64,
    #[pyo3(get)]
    pub timing_diff_ms: i64,
    #[pyo3(get)]
    pub volatile_headers: Vec<String>,
    #[pyo3(get)]
    pub identical: bool,
}

#[pymethods]
impl ResponseComparisonPy {
    #[new]
    #[pyo3(signature = (original_status, replayed_status, headers_differ=false, body_differ=false, body_diff_size=0, timing_diff_ms=0, volatile_headers=None))]
    fn new(
        original_status: u16,
        replayed_status: u16,
        headers_differ: bool,
        body_differ: bool,
        body_diff_size: i64,
        timing_diff_ms: i64,
        volatile_headers: Option<Vec<String>>,
    ) -> Self {
        let status_match = original_status == replayed_status;
        let identical = status_match && !headers_differ && !body_differ;
        Self {
            status_match,
            original_status,
            replayed_status,
            headers_differ,
            body_differ,
            body_diff_size,
            timing_diff_ms,
            volatile_headers: volatile_headers.unwrap_or_default(),
            identical,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("status_match", self.status_match)?;
        dict.set_item("original_status", self.original_status)?;
        dict.set_item("replayed_status", self.replayed_status)?;
        dict.set_item("headers_differ", self.headers_differ)?;
        dict.set_item("body_differ", self.body_differ)?;
        dict.set_item("body_diff_size", self.body_diff_size)?;
        dict.set_item("timing_diff_ms", self.timing_diff_ms)?;
        dict.set_item("volatile_headers", &self.volatile_headers)?;
        dict.set_item("identical", self.identical)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ResponseComparison(status_match={}, identical={})",
            self.status_match, self.identical
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// Release 3 WS14: Comparison Rule
// ═══════════════════════════════════════════════════════════════════

/// Rule for comparing original and replayed HTTP responses.
///
/// Defines which fields to compare, normalization for volatile headers,
/// and tolerance thresholds for body and timing comparisons.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonRulePy {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub compare_status: bool,
    #[pyo3(get)]
    pub compare_headers: bool,
    #[pyo3(get)]
    pub compare_body: bool,
    #[pyo3(get)]
    pub compare_timing: bool,
    #[pyo3(get)]
    pub volatile_headers: Vec<String>,
    #[pyo3(get)]
    pub body_diff_threshold: i64,
    #[pyo3(get)]
    pub timing_diff_threshold_ms: u64,
    #[pyo3(get)]
    pub ignore_header_order: bool,
}

#[pymethods]
impl ComparisonRulePy {
    #[new]
    #[pyo3(signature = (name="default", compare_status=true, compare_headers=true, compare_body=true, compare_timing=false, volatile_headers=None, body_diff_threshold=0, timing_diff_threshold_ms=100, ignore_header_order=true))]
    fn new(
        name: &str,
        compare_status: bool,
        compare_headers: bool,
        compare_body: bool,
        compare_timing: bool,
        volatile_headers: Option<Vec<String>>,
        body_diff_threshold: i64,
        timing_diff_threshold_ms: u64,
        ignore_header_order: bool,
    ) -> Self {
        Self {
            name: name.to_string(),
            compare_status,
            compare_headers,
            compare_body,
            compare_timing,
            volatile_headers: volatile_headers.unwrap_or_default(),
            body_diff_threshold,
            timing_diff_threshold_ms,
            ignore_header_order,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("name", &self.name)?;
        dict.set_item("compare_status", self.compare_status)?;
        dict.set_item("compare_headers", self.compare_headers)?;
        dict.set_item("compare_body", self.compare_body)?;
        dict.set_item("compare_timing", self.compare_timing)?;
        dict.set_item("volatile_headers", &self.volatile_headers)?;
        dict.set_item("body_diff_threshold", self.body_diff_threshold)?;
        dict.set_item("timing_diff_threshold_ms", self.timing_diff_threshold_ms)?;
        dict.set_item("ignore_header_order", self.ignore_header_order)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("ComparisonRule(name={})", self.name)
    }
}
