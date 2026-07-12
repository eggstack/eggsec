use pyo3::prelude::*;
use std::collections::HashMap;

/// Python wrapper for `SensitiveString`.
///
/// A zeroizing string for secrets. Use `new()` to create and `expose_secret()` to read.
/// `__repr__` and `__str__` are always redacted.
#[pyclass(name = "SensitiveString", frozen)]
#[derive(Clone)]
pub(crate) struct PySensitiveString {
    inner: eggsec_core::types::SensitiveString,
}

#[pymethods]
impl PySensitiveString {
    #[new]
    fn new(secret: &str) -> Self {
        Self {
            inner: eggsec_core::types::SensitiveString::new(secret),
        }
    }

    fn expose_secret(&self) -> String {
        self.inner.expose_secret().to_string()
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    fn __repr__(&self) -> String {
        "SensitiveString([REDACTED])".to_string()
    }

    fn __str__(&self) -> String {
        "[REDACTED]".to_string()
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.inner.expose_secret().hash(&mut hasher);
        hasher.finish()
    }
}

/// Python wrapper for `HttpConfig`.
///
/// HTTP client settings (timeouts, retries, TLS, proxy, headers).
#[pyclass(name = "HttpConfig", frozen)]
#[derive(Clone)]
pub(crate) struct PyHttpConfig {
    timeout_secs: u64,
    max_retries: u32,
    retry_delay_ms: u64,
    verify_tls: bool,
    follow_redirects: bool,
    max_redirects: usize,
    default_headers: HashMap<String, String>,
    default_user_agent: Option<String>,
    proxy: Option<String>,
    proxy_auth: Option<PySensitiveString>,
}

#[pymethods]
impl PyHttpConfig {
    #[new]
    #[pyo3(signature = (
        timeout_secs=30,
        max_retries=3,
        retry_delay_ms=1000,
        verify_tls=true,
        follow_redirects=true,
        max_redirects=10,
        default_headers=None,
        default_user_agent=None,
        proxy=None,
        proxy_auth=None,
    ))]
    fn new(
        timeout_secs: u64,
        max_retries: u32,
        retry_delay_ms: u64,
        verify_tls: bool,
        follow_redirects: bool,
        max_redirects: usize,
        default_headers: Option<HashMap<String, String>>,
        default_user_agent: Option<String>,
        proxy: Option<String>,
        proxy_auth: Option<String>,
    ) -> Self {
        Self {
            timeout_secs,
            max_retries,
            retry_delay_ms,
            verify_tls,
            follow_redirects,
            max_redirects,
            default_headers: default_headers.unwrap_or_default(),
            default_user_agent,
            proxy,
            proxy_auth: proxy_auth.map(|s| PySensitiveString::new(&s)),
        }
    }

    #[getter]
    fn timeout_secs(&self) -> u64 {
        self.timeout_secs
    }

    #[getter]
    fn max_retries(&self) -> u32 {
        self.max_retries
    }

    #[getter]
    fn retry_delay_ms(&self) -> u64 {
        self.retry_delay_ms
    }

    #[getter]
    fn verify_tls(&self) -> bool {
        self.verify_tls
    }

    #[getter]
    fn follow_redirects(&self) -> bool {
        self.follow_redirects
    }

    #[getter]
    fn max_redirects(&self) -> usize {
        self.max_redirects
    }

    #[getter]
    fn default_headers(&self) -> HashMap<String, String> {
        self.default_headers.clone()
    }

    #[getter]
    fn default_user_agent(&self) -> Option<String> {
        self.default_user_agent.clone()
    }

    #[getter]
    fn proxy(&self) -> Option<String> {
        self.proxy.clone()
    }

    #[getter]
    fn has_proxy_auth(&self) -> bool {
        self.proxy_auth.is_some()
    }

    #[getter]
    fn proxy_auth(&self, py: Python) -> PyResult<PyObject> {
        match &self.proxy_auth {
            Some(s) => Ok(Py::new(py, s.clone())?.into_any()),
            None => Ok(py.None()),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "HttpConfig(timeout_secs={}, max_retries={}, verify_tls={})",
            self.timeout_secs, self.max_retries, self.verify_tls
        )
    }
}

impl PyHttpConfig {
    pub(crate) fn from_inner(config: &eggsec::config::HttpConfig) -> Self {
        Self {
            timeout_secs: config.timeout_secs,
            max_retries: config.max_retries,
            retry_delay_ms: config.retry_delay_ms,
            verify_tls: config.verify_tls,
            follow_redirects: config.follow_redirects,
            max_redirects: config.max_redirects,
            default_headers: config
                .default_headers
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            default_user_agent: config.default_user_agent.clone(),
            proxy: config.proxy.clone(),
            proxy_auth: config
                .proxy_auth
                .as_ref()
                .map(|s| PySensitiveString::new(s.expose_secret())),
        }
    }
}

/// Python wrapper for `ScanConfig`.
///
/// Port scanning, endpoint discovery, and fuzzing settings.
#[pyclass(name = "ScanConfig", frozen)]
#[derive(Clone)]
pub(crate) struct PyScanConfig {
    default_concurrency: usize,
    rate_limit_per_second: Option<u32>,
    jitter_ms: Option<(u64, u64)>,
    stealth_mode: bool,
    exclude_ports: Vec<u16>,
    exclude_hosts: Vec<String>,
    port_timeout_secs: u64,
    save_session: bool,
}

#[pymethods]
impl PyScanConfig {
    #[new]
    #[pyo3(signature = (
        default_concurrency=10,
        rate_limit_per_second=None,
        jitter_ms=None,
        stealth_mode=false,
        exclude_ports=None,
        exclude_hosts=None,
        port_timeout_secs=2,
        save_session=false,
    ))]
    fn new(
        default_concurrency: usize,
        rate_limit_per_second: Option<u32>,
        jitter_ms: Option<(u64, u64)>,
        stealth_mode: bool,
        exclude_ports: Option<Vec<u16>>,
        exclude_hosts: Option<Vec<String>>,
        port_timeout_secs: u64,
        save_session: bool,
    ) -> Self {
        Self {
            default_concurrency,
            rate_limit_per_second,
            jitter_ms,
            stealth_mode,
            exclude_ports: exclude_ports.unwrap_or_default(),
            exclude_hosts: exclude_hosts.unwrap_or_default(),
            port_timeout_secs,
            save_session,
        }
    }

    #[getter]
    fn default_concurrency(&self) -> usize {
        self.default_concurrency
    }

    #[getter]
    fn rate_limit_per_second(&self) -> Option<u32> {
        self.rate_limit_per_second
    }

    #[getter]
    fn jitter_ms(&self) -> Option<(u64, u64)> {
        self.jitter_ms
    }

    #[getter]
    fn stealth_mode(&self) -> bool {
        self.stealth_mode
    }

    #[getter]
    fn exclude_ports(&self) -> Vec<u16> {
        self.exclude_ports.clone()
    }

    #[getter]
    fn exclude_hosts(&self) -> Vec<String> {
        self.exclude_hosts.clone()
    }

    #[getter]
    fn port_timeout_secs(&self) -> u64 {
        self.port_timeout_secs
    }

    #[getter]
    fn save_session(&self) -> bool {
        self.save_session
    }

    fn __repr__(&self) -> String {
        format!(
            "ScanConfig(default_concurrency={}, stealth_mode={})",
            self.default_concurrency, self.stealth_mode
        )
    }
}

impl PyScanConfig {
    pub(crate) fn from_inner(config: &eggsec::config::ScanConfig) -> Self {
        Self {
            default_concurrency: config.default_concurrency,
            rate_limit_per_second: config.rate_limit_per_second,
            jitter_ms: config.jitter_ms,
            stealth_mode: config.stealth_mode,
            exclude_ports: config.exclude_ports.clone(),
            exclude_hosts: config.exclude_hosts.clone(),
            port_timeout_secs: config.port_timeout_secs,
            save_session: config.save_session,
        }
    }
}

/// Python wrapper for `OutputConfig`.
///
/// Report formatting and output settings.
#[pyclass(name = "OutputConfig", frozen)]
#[derive(Clone)]
pub(crate) struct PyOutputConfig {
    format: String,
    verbosity: String,
    color: bool,
    progress_bars: bool,
    save_results: bool,
    results_dir: Option<String>,
    include_timestamp: bool,
}

#[pymethods]
impl PyOutputConfig {
    #[new]
    #[pyo3(signature = (
        format="pretty",
        verbosity="normal",
        color=true,
        progress_bars=true,
        save_results=false,
        results_dir=None,
        include_timestamp=true,
    ))]
    fn new(
        format: &str,
        verbosity: &str,
        color: bool,
        progress_bars: bool,
        save_results: bool,
        results_dir: Option<String>,
        include_timestamp: bool,
    ) -> Self {
        Self {
            format: format.to_string(),
            verbosity: verbosity.to_string(),
            color,
            progress_bars,
            save_results,
            results_dir,
            include_timestamp,
        }
    }

    #[getter]
    fn format(&self) -> String {
        self.format.clone()
    }

    #[getter]
    fn verbosity(&self) -> String {
        self.verbosity.clone()
    }

    #[getter]
    fn color(&self) -> bool {
        self.color
    }

    #[getter]
    fn progress_bars(&self) -> bool {
        self.progress_bars
    }

    #[getter]
    fn save_results(&self) -> bool {
        self.save_results
    }

    #[getter]
    fn results_dir(&self) -> Option<String> {
        self.results_dir.clone()
    }

    #[getter]
    fn include_timestamp(&self) -> bool {
        self.include_timestamp
    }

    fn __repr__(&self) -> String {
        format!(
            "OutputConfig(format='{}', verbosity='{}')",
            self.format, self.verbosity
        )
    }
}

impl PyOutputConfig {
    pub(crate) fn from_inner(config: &eggsec::config::OutputConfig) -> Self {
        Self {
            format: config.format.to_string(),
            verbosity: format!("{:?}", config.verbosity).to_lowercase(),
            color: config.color,
            progress_bars: config.progress_bars,
            save_results: config.save_results,
            results_dir: config
                .results_dir
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            include_timestamp: config.include_timestamp,
        }
    }
}

/// Python wrapper for recon API configuration.
///
/// API keys and settings for reconnaissance data sources.
#[pyclass(name = "ReconApiConfig", frozen)]
#[derive(Clone)]
pub(crate) struct PyReconApiConfig {
    virustotal_enabled: bool,
    virustotal_api_key: Option<PySensitiveString>,
    alienvault_enabled: bool,
    alienvault_api_key: Option<PySensitiveString>,
    shodan_enabled: bool,
    shodan_api_key: Option<PySensitiveString>,
    ipapi_enabled: bool,
    ipapi_api_key: Option<PySensitiveString>,
    maxmind_enabled: bool,
    maxmind_account_id: Option<u32>,
    maxmind_license_key: Option<PySensitiveString>,
    wayback_enabled: bool,
    wayback_api_key: Option<PySensitiveString>,
    nvd_api_key: Option<PySensitiveString>,
}

#[pymethods]
impl PyReconApiConfig {
    #[new]
    #[pyo3(signature = (
        virustotal_enabled=false,
        virustotal_api_key=None,
        alienvault_enabled=false,
        alienvault_api_key=None,
        shodan_enabled=false,
        shodan_api_key=None,
        ipapi_enabled=false,
        ipapi_api_key=None,
        maxmind_enabled=false,
        maxmind_account_id=None,
        maxmind_license_key=None,
        wayback_enabled=false,
        wayback_api_key=None,
        nvd_api_key=None,
    ))]
    fn new(
        virustotal_enabled: bool,
        virustotal_api_key: Option<String>,
        alienvault_enabled: bool,
        alienvault_api_key: Option<String>,
        shodan_enabled: bool,
        shodan_api_key: Option<String>,
        ipapi_enabled: bool,
        ipapi_api_key: Option<String>,
        maxmind_enabled: bool,
        maxmind_account_id: Option<u32>,
        maxmind_license_key: Option<String>,
        wayback_enabled: bool,
        wayback_api_key: Option<String>,
        nvd_api_key: Option<String>,
    ) -> Self {
        Self {
            virustotal_enabled,
            virustotal_api_key: virustotal_api_key.map(|s| PySensitiveString::new(&s)),
            alienvault_enabled,
            alienvault_api_key: alienvault_api_key.map(|s| PySensitiveString::new(&s)),
            shodan_enabled,
            shodan_api_key: shodan_api_key.map(|s| PySensitiveString::new(&s)),
            ipapi_enabled,
            ipapi_api_key: ipapi_api_key.map(|s| PySensitiveString::new(&s)),
            maxmind_enabled,
            maxmind_account_id,
            maxmind_license_key: maxmind_license_key.map(|s| PySensitiveString::new(&s)),
            wayback_enabled,
            wayback_api_key: wayback_api_key.map(|s| PySensitiveString::new(&s)),
            nvd_api_key: nvd_api_key.map(|s| PySensitiveString::new(&s)),
        }
    }

    #[getter]
    fn virustotal_enabled(&self) -> bool {
        self.virustotal_enabled
    }

    #[getter]
    fn has_virustotal_api_key(&self) -> bool {
        self.virustotal_api_key.is_some()
    }

    #[getter]
    fn virustotal_api_key(&self, py: Python) -> PyResult<PyObject> {
        match &self.virustotal_api_key {
            Some(s) => Ok(Py::new(py, s.clone())?.into_any()),
            None => Ok(py.None()),
        }
    }

    #[getter]
    fn alienvault_enabled(&self) -> bool {
        self.alienvault_enabled
    }

    #[getter]
    fn has_alienvault_api_key(&self) -> bool {
        self.alienvault_api_key.is_some()
    }

    #[getter]
    fn alienvault_api_key(&self, py: Python) -> PyResult<PyObject> {
        match &self.alienvault_api_key {
            Some(s) => Ok(Py::new(py, s.clone())?.into_any()),
            None => Ok(py.None()),
        }
    }

    #[getter]
    fn shodan_enabled(&self) -> bool {
        self.shodan_enabled
    }

    #[getter]
    fn has_shodan_api_key(&self) -> bool {
        self.shodan_api_key.is_some()
    }

    #[getter]
    fn shodan_api_key(&self, py: Python) -> PyResult<PyObject> {
        match &self.shodan_api_key {
            Some(s) => Ok(Py::new(py, s.clone())?.into_any()),
            None => Ok(py.None()),
        }
    }

    #[getter]
    fn ipapi_enabled(&self) -> bool {
        self.ipapi_enabled
    }

    #[getter]
    fn has_ipapi_api_key(&self) -> bool {
        self.ipapi_api_key.is_some()
    }

    #[getter]
    fn ipapi_api_key(&self, py: Python) -> PyResult<PyObject> {
        match &self.ipapi_api_key {
            Some(s) => Ok(Py::new(py, s.clone())?.into_any()),
            None => Ok(py.None()),
        }
    }

    #[getter]
    fn maxmind_enabled(&self) -> bool {
        self.maxmind_enabled
    }

    #[getter]
    fn maxmind_account_id(&self) -> Option<u32> {
        self.maxmind_account_id
    }

    #[getter]
    fn has_maxmind_license_key(&self) -> bool {
        self.maxmind_license_key.is_some()
    }

    #[getter]
    fn maxmind_license_key(&self, py: Python) -> PyResult<PyObject> {
        match &self.maxmind_license_key {
            Some(s) => Ok(Py::new(py, s.clone())?.into_any()),
            None => Ok(py.None()),
        }
    }

    #[getter]
    fn wayback_enabled(&self) -> bool {
        self.wayback_enabled
    }

    #[getter]
    fn has_wayback_api_key(&self) -> bool {
        self.wayback_api_key.is_some()
    }

    #[getter]
    fn wayback_api_key(&self, py: Python) -> PyResult<PyObject> {
        match &self.wayback_api_key {
            Some(s) => Ok(Py::new(py, s.clone())?.into_any()),
            None => Ok(py.None()),
        }
    }

    #[getter]
    fn has_nvd_api_key(&self) -> bool {
        self.nvd_api_key.is_some()
    }

    #[getter]
    fn nvd_api_key(&self, py: Python) -> PyResult<PyObject> {
        match &self.nvd_api_key {
            Some(s) => Ok(Py::new(py, s.clone())?.into_any()),
            None => Ok(py.None()),
        }
    }

    fn __repr__(&self) -> String {
        "ReconApiConfig()".to_string()
    }
}

impl PyReconApiConfig {
    pub(crate) fn from_inner(config: &eggsec::config::ApiConfig) -> Self {
        Self {
            virustotal_enabled: config.virustotal.enabled,
            virustotal_api_key: config
                .virustotal
                .api_key
                .as_ref()
                .map(|s| PySensitiveString::new(s.expose_secret())),
            alienvault_enabled: config.alienvault.enabled,
            alienvault_api_key: config
                .alienvault
                .api_key
                .as_ref()
                .map(|s| PySensitiveString::new(s.expose_secret())),
            shodan_enabled: config.shodan.enabled,
            shodan_api_key: config
                .shodan
                .api_key
                .as_ref()
                .map(|s| PySensitiveString::new(s.expose_secret())),
            ipapi_enabled: config.ipapi.enabled,
            ipapi_api_key: config
                .ipapi
                .api_key
                .as_ref()
                .map(|s| PySensitiveString::new(s.expose_secret())),
            maxmind_enabled: config.maxmind.enabled,
            maxmind_account_id: config.maxmind.account_id,
            maxmind_license_key: config
                .maxmind
                .license_key
                .as_ref()
                .map(|s| PySensitiveString::new(s.expose_secret())),
            wayback_enabled: config.wayback_machine.enabled,
            wayback_api_key: config
                .wayback_machine
                .api_key
                .as_ref()
                .map(|s| PySensitiveString::new(s.expose_secret())),
            nvd_api_key: config
                .nvd
                .api_key
                .as_ref()
                .map(|s| PySensitiveString::new(s.expose_secret())),
        }
    }
}

/// Python wrapper for `ReconConfig`.
///
/// Reconnaissance module settings.
#[pyclass(name = "ReconConfig", frozen)]
#[derive(Clone)]
pub(crate) struct PyReconConfig {
    dns_concurrency: usize,
    apis: PyReconApiConfig,
}

#[pymethods]
impl PyReconConfig {
    #[new]
    #[pyo3(signature = (dns_concurrency=10, apis=None))]
    fn new(dns_concurrency: usize, apis: Option<PyReconApiConfig>) -> Self {
        Self {
            dns_concurrency,
            apis: apis.unwrap_or_else(|| PyReconApiConfig {
                virustotal_enabled: false,
                virustotal_api_key: None,
                alienvault_enabled: false,
                alienvault_api_key: None,
                shodan_enabled: false,
                shodan_api_key: None,
                ipapi_enabled: false,
                ipapi_api_key: None,
                maxmind_enabled: false,
                maxmind_account_id: None,
                maxmind_license_key: None,
                wayback_enabled: false,
                wayback_api_key: None,
                nvd_api_key: None,
            }),
        }
    }

    #[getter]
    fn dns_concurrency(&self) -> usize {
        self.dns_concurrency
    }

    #[getter]
    fn apis(&self) -> PyReconApiConfig {
        self.apis.clone()
    }

    fn __repr__(&self) -> String {
        format!("ReconConfig(dns_concurrency={})", self.dns_concurrency)
    }
}

impl PyReconConfig {
    pub(crate) fn from_inner(config: &eggsec::config::ReconConfig) -> Self {
        Self {
            dns_concurrency: config.dns_concurrency,
            apis: PyReconApiConfig::from_inner(&config.apis),
        }
    }
}

/// Python wrapper for `ProxyConfigEntry`.
///
/// Proxy configuration for routing traffic through intermediary servers.
#[pyclass(name = "ProxyConfigEntry", frozen)]
#[derive(Clone)]
pub(crate) struct PyProxyConfigEntry {
    proxy_type: String,
    address: String,
    port: u16,
    username: Option<String>,
    has_password: bool,
    local_addr: Option<String>,
    weight: Option<u32>,
    priority: Option<u32>,
    enabled: bool,
}

#[pymethods]
impl PyProxyConfigEntry {
    #[new]
    #[pyo3(signature = (
        address,
        port,
        proxy_type="socks5",
        username=None,
        password=None,
        local_addr=None,
        weight=None,
        priority=None,
        enabled=true,
    ))]
    fn new(
        address: &str,
        port: u16,
        proxy_type: &str,
        username: Option<String>,
        password: Option<String>,
        local_addr: Option<String>,
        weight: Option<u32>,
        priority: Option<u32>,
        enabled: bool,
    ) -> Self {
        Self {
            proxy_type: proxy_type.to_string(),
            address: address.to_string(),
            port,
            username,
            has_password: password.is_some(),
            local_addr,
            weight,
            priority,
            enabled,
        }
    }

    #[getter]
    fn proxy_type(&self) -> String {
        self.proxy_type.clone()
    }

    #[getter]
    fn address(&self) -> String {
        self.address.clone()
    }

    #[getter]
    fn port(&self) -> u16 {
        self.port
    }

    #[getter]
    fn username(&self) -> Option<String> {
        self.username.clone()
    }

    #[getter]
    fn has_password(&self) -> bool {
        self.has_password
    }

    #[getter]
    fn local_addr(&self) -> Option<String> {
        self.local_addr.clone()
    }

    #[getter]
    fn weight(&self) -> Option<u32> {
        self.weight
    }

    #[getter]
    fn priority(&self) -> Option<u32> {
        self.priority
    }

    #[getter]
    fn enabled(&self) -> bool {
        self.enabled
    }

    fn __repr__(&self) -> String {
        format!(
            "ProxyConfigEntry(type='{}', address='{}', port={})",
            self.proxy_type, self.address, self.port
        )
    }
}

impl PyProxyConfigEntry {
    pub(crate) fn from_inner(config: &eggsec::config::ProxyConfigEntry) -> Self {
        Self {
            proxy_type: config.proxy_type.to_string(),
            address: config.address.clone(),
            port: config.port,
            username: config.username.clone(),
            has_password: config.password.is_some(),
            local_addr: config.local_addr.clone(),
            weight: config.weight,
            priority: config.priority,
            enabled: config.enabled,
        }
    }
}

/// Python wrapper for an allowed remote worker.
#[pyclass(name = "AllowedWorker", frozen)]
#[derive(Clone)]
pub(crate) struct PyAllowedWorker {
    host: String,
    port: Option<u16>,
}

#[pymethods]
impl PyAllowedWorker {
    #[new]
    fn new(host: &str, port: Option<u16>) -> Self {
        Self {
            host: host.to_string(),
            port,
        }
    }

    #[getter]
    fn host(&self) -> String {
        self.host.clone()
    }

    #[getter]
    fn port(&self) -> Option<u16> {
        self.port
    }

    fn __repr__(&self) -> String {
        match self.port {
            Some(p) => format!("AllowedWorker(host='{}', port={})", self.host, p),
            None => format!("AllowedWorker(host='{}')", self.host),
        }
    }
}

/// Python wrapper for `RemoteConfig`.
///
/// Remote worker connection settings.
#[pyclass(name = "RemoteConfig", frozen)]
#[derive(Clone)]
pub(crate) struct PyRemoteConfig {
    psk: Option<PySensitiveString>,
    default_port: u16,
    allowed_workers: Vec<PyAllowedWorker>,
}

#[pymethods]
impl PyRemoteConfig {
    #[new]
    #[pyo3(signature = (psk=None, default_port=5000, allowed_workers=None))]
    fn new(
        psk: Option<String>,
        default_port: u16,
        allowed_workers: Option<Vec<PyAllowedWorker>>,
    ) -> Self {
        Self {
            psk: psk.map(|s| PySensitiveString::new(&s)),
            default_port,
            allowed_workers: allowed_workers.unwrap_or_default(),
        }
    }

    #[getter]
    fn has_psk(&self) -> bool {
        self.psk.is_some()
    }

    #[getter]
    fn psk(&self, py: Python) -> PyResult<PyObject> {
        match &self.psk {
            Some(s) => Ok(Py::new(py, s.clone())?.into_any()),
            None => Ok(py.None()),
        }
    }

    #[getter]
    fn default_port(&self) -> u16 {
        self.default_port
    }

    #[getter]
    fn allowed_workers(&self) -> Vec<PyAllowedWorker> {
        self.allowed_workers.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "RemoteConfig(default_port={}, workers={})",
            self.default_port,
            self.allowed_workers.len()
        )
    }
}

impl PyRemoteConfig {
    pub(crate) fn from_inner(config: &eggsec::config::RemoteConfig) -> Self {
        Self {
            psk: config
                .psk
                .as_ref()
                .map(|s| PySensitiveString::new(s.expose_secret())),
            default_port: config.default_port,
            allowed_workers: config
                .allowed_workers
                .iter()
                .map(|w| PyAllowedWorker {
                    host: w.host.clone(),
                    port: w.port,
                })
                .collect(),
        }
    }
}

/// Python wrapper for `AiConfig`.
///
/// AI/LLM integration settings for adaptive security testing.
#[pyclass(name = "AiConfig", frozen)]
#[derive(Clone)]
pub(crate) struct PyAiConfig {
    provider: String,
    model: Option<String>,
    api_key: Option<PySensitiveString>,
    base_url: Option<String>,
    max_tokens: Option<usize>,
    temperature: Option<f64>,
    max_payloads: usize,
    max_bypasses: usize,
}

#[pymethods]
impl PyAiConfig {
    #[new]
    #[pyo3(signature = (
        provider="openai",
        model=None,
        api_key=None,
        base_url=None,
        max_tokens=None,
        temperature=None,
        max_payloads=50,
        max_bypasses=10,
    ))]
    fn new(
        provider: &str,
        model: Option<String>,
        api_key: Option<String>,
        base_url: Option<String>,
        max_tokens: Option<usize>,
        temperature: Option<f64>,
        max_payloads: usize,
        max_bypasses: usize,
    ) -> Self {
        Self {
            provider: provider.to_string(),
            model,
            api_key: api_key.map(|s| PySensitiveString::new(&s)),
            base_url,
            max_tokens,
            temperature,
            max_payloads,
            max_bypasses,
        }
    }

    #[getter]
    fn provider(&self) -> String {
        self.provider.clone()
    }

    #[getter]
    fn model(&self) -> Option<String> {
        self.model.clone()
    }

    #[getter]
    fn has_api_key(&self) -> bool {
        self.api_key.is_some()
    }

    #[getter]
    fn api_key(&self, py: Python) -> PyResult<PyObject> {
        match &self.api_key {
            Some(s) => Ok(Py::new(py, s.clone())?.into_any()),
            None => Ok(py.None()),
        }
    }

    #[getter]
    fn base_url(&self) -> Option<String> {
        self.base_url.clone()
    }

    #[getter]
    fn max_tokens(&self) -> Option<usize> {
        self.max_tokens
    }

    #[getter]
    fn temperature(&self) -> Option<f64> {
        self.temperature
    }

    #[getter]
    fn max_payloads(&self) -> usize {
        self.max_payloads
    }

    #[getter]
    fn max_bypasses(&self) -> usize {
        self.max_bypasses
    }

    fn __repr__(&self) -> String {
        format!(
            "AiConfig(provider='{}', model={:?})",
            self.provider, self.model
        )
    }
}

impl PyAiConfig {
    pub(crate) fn from_inner(config: &eggsec::config::AiConfig) -> Self {
        Self {
            provider: config.provider.clone(),
            model: config.model.clone(),
            api_key: config
                .api_key
                .as_ref()
                .map(|s| PySensitiveString::new(s.expose_secret())),
            base_url: config.base_url.clone(),
            max_tokens: config.max_tokens,
            temperature: config.temperature,
            max_payloads: config.max_payloads,
            max_bypasses: config.max_bypasses,
        }
    }
}

/// Python wrapper for `SearchConfig`.
///
/// Search engine integration settings for OSINT gathering.
#[pyclass(name = "SearchConfig", frozen)]
#[derive(Clone)]
pub(crate) struct PySearchConfig {
    enabled: bool,
    url: Option<String>,
    engines: Vec<String>,
    cache_ttl_seconds: u64,
}

#[pymethods]
impl PySearchConfig {
    #[new]
    #[pyo3(signature = (enabled=false, url=None, engines=None, cache_ttl_seconds=3600))]
    fn new(
        enabled: bool,
        url: Option<String>,
        engines: Option<Vec<String>>,
        cache_ttl_seconds: u64,
    ) -> Self {
        Self {
            enabled,
            url,
            engines: engines.unwrap_or_default(),
            cache_ttl_seconds,
        }
    }

    #[getter]
    fn enabled(&self) -> bool {
        self.enabled
    }

    #[getter]
    fn url(&self) -> Option<String> {
        self.url.clone()
    }

    #[getter]
    fn engines(&self) -> Vec<String> {
        self.engines.clone()
    }

    #[getter]
    fn cache_ttl_seconds(&self) -> u64 {
        self.cache_ttl_seconds
    }

    fn __repr__(&self) -> String {
        format!("SearchConfig(enabled={}, url={:?})", self.enabled, self.url)
    }
}

impl PySearchConfig {
    pub(crate) fn from_inner(config: &eggsec::config::SearchConfig) -> Self {
        Self {
            enabled: config.enabled,
            url: config.searxng_url.clone(),
            engines: config.engines.clone(),
            cache_ttl_seconds: config.cache_ttl_seconds,
        }
    }
}

/// Python wrapper for `PathsConfig`.
///
/// Filesystem paths for payloads, wordlists, and exports.
#[pyclass(name = "PathsConfig", frozen)]
#[derive(Clone)]
pub(crate) struct PyPathsConfig {
    custom_payloads_dir: Option<String>,
    wordlists_dir: Option<String>,
    export_dir: Option<String>,
}

#[pymethods]
impl PyPathsConfig {
    #[new]
    #[pyo3(signature = (custom_payloads_dir=None, wordlists_dir=None, export_dir=None))]
    fn new(
        custom_payloads_dir: Option<String>,
        wordlists_dir: Option<String>,
        export_dir: Option<String>,
    ) -> Self {
        Self {
            custom_payloads_dir,
            wordlists_dir,
            export_dir,
        }
    }

    #[getter]
    fn custom_payloads_dir(&self) -> Option<String> {
        self.custom_payloads_dir.clone()
    }

    #[getter]
    fn wordlists_dir(&self) -> Option<String> {
        self.wordlists_dir.clone()
    }

    #[getter]
    fn export_dir(&self) -> Option<String> {
        self.export_dir.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "PathsConfig(custom_payloads_dir={:?}, wordlists_dir={:?})",
            self.custom_payloads_dir, self.wordlists_dir
        )
    }
}

impl PyPathsConfig {
    pub(crate) fn from_inner(config: &eggsec::config::PathsConfig) -> Self {
        Self {
            custom_payloads_dir: config
                .custom_payloads_dir
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            wordlists_dir: config
                .wordlists_dir
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            export_dir: config.export_dir.clone(),
        }
    }
}

/// Python wrapper for `CacheConfig`.
///
/// TTL settings for cached reconnaissance data.
#[pyclass(name = "CacheConfig", frozen)]
#[derive(Clone)]
pub(crate) struct PyCacheConfig {
    ttl_secs: u64,
}

#[pymethods]
impl PyCacheConfig {
    #[new]
    #[pyo3(signature = (ttl_secs=3600))]
    fn new(ttl_secs: u64) -> Self {
        Self { ttl_secs }
    }

    #[getter]
    fn ttl_secs(&self) -> u64 {
        self.ttl_secs
    }

    fn __repr__(&self) -> String {
        format!("CacheConfig(ttl_secs={})", self.ttl_secs)
    }
}

impl PyCacheConfig {
    pub(crate) fn from_inner(config: &eggsec::config::CacheConfig) -> Self {
        Self {
            ttl_secs: config.ttl_secs,
        }
    }
}

/// Python wrapper for `AlertChannelConfigEntry`.
///
/// Alert channel configuration. The `channel_type` property indicates the channel kind
/// ("webhook", "email", "slack", "pagerduty"). Use the corresponding constructor or
/// inspect `channel_type` to determine which getters are meaningful.
#[pyclass(name = "AlertChannelConfig", frozen)]
#[derive(Clone)]
pub(crate) struct PyAlertChannelConfig {
    channel_type: String,
    url: Option<String>,
    smtp_host: Option<String>,
    smtp_port: Option<u16>,
    from: Option<String>,
    to: Option<Vec<String>>,
    webhook_url: Option<String>,
    channel: Option<String>,
    routing_key: Option<String>,
    severity: Option<String>,
    has_secret: bool,
    headers: HashMap<String, String>,
}

#[pymethods]
impl PyAlertChannelConfig {
    #[staticmethod]
    fn webhook(url: &str) -> Self {
        Self {
            channel_type: "webhook".to_string(),
            url: Some(url.to_string()),
            smtp_host: None,
            smtp_port: None,
            from: None,
            to: None,
            webhook_url: None,
            channel: None,
            routing_key: None,
            severity: None,
            has_secret: false,
            headers: HashMap::new(),
        }
    }

    #[staticmethod]
    fn email(smtp_host: &str, smtp_port: u16, from: &str, to: Vec<String>) -> Self {
        Self {
            channel_type: "email".to_string(),
            url: None,
            smtp_host: Some(smtp_host.to_string()),
            smtp_port: Some(smtp_port),
            from: Some(from.to_string()),
            to: Some(to),
            webhook_url: None,
            channel: None,
            routing_key: None,
            severity: None,
            has_secret: false,
            headers: HashMap::new(),
        }
    }

    #[staticmethod]
    fn slack(webhook_url: &str, channel: Option<String>) -> Self {
        Self {
            channel_type: "slack".to_string(),
            url: None,
            smtp_host: None,
            smtp_port: None,
            from: None,
            to: None,
            webhook_url: Some(webhook_url.to_string()),
            channel,
            routing_key: None,
            severity: None,
            has_secret: false,
            headers: HashMap::new(),
        }
    }

    #[staticmethod]
    fn pagerduty(routing_key: &str, severity: &str) -> Self {
        Self {
            channel_type: "pagerduty".to_string(),
            url: None,
            smtp_host: None,
            smtp_port: None,
            from: None,
            to: None,
            webhook_url: None,
            channel: None,
            routing_key: Some(routing_key.to_string()),
            severity: Some(severity.to_string()),
            has_secret: true,
            headers: HashMap::new(),
        }
    }

    #[getter]
    fn channel_type(&self) -> String {
        self.channel_type.clone()
    }

    #[getter]
    fn url(&self) -> Option<String> {
        self.url.clone()
    }

    #[getter]
    fn smtp_host(&self) -> Option<String> {
        self.smtp_host.clone()
    }

    #[getter]
    fn smtp_port(&self) -> Option<u16> {
        self.smtp_port
    }

    #[getter]
    fn from(&self) -> Option<String> {
        self.from.clone()
    }

    #[getter]
    fn to(&self) -> Option<Vec<String>> {
        self.to.clone()
    }

    #[getter]
    fn webhook_url(&self) -> Option<String> {
        self.webhook_url.clone()
    }

    #[getter]
    fn channel(&self) -> Option<String> {
        self.channel.clone()
    }

    #[getter]
    fn routing_key(&self) -> Option<String> {
        self.routing_key.clone()
    }

    #[getter]
    fn severity(&self) -> Option<String> {
        self.severity.clone()
    }

    #[getter]
    fn has_secret(&self) -> bool {
        self.has_secret
    }

    #[getter]
    fn headers(&self) -> HashMap<String, String> {
        self.headers.clone()
    }

    fn __repr__(&self) -> String {
        format!("AlertChannelConfig(type='{}')", self.channel_type)
    }
}

impl PyAlertChannelConfig {
    pub(crate) fn from_inner(name: &str, entry: &eggsec::config::AlertChannelConfigEntry) -> Self {
        use eggsec::config::AlertChannelConfigEntry as E;
        match entry {
            E::Webhook(wh) => Self {
                channel_type: "webhook".to_string(),
                url: Some(wh.url.clone()),
                smtp_host: None,
                smtp_port: None,
                from: None,
                to: None,
                webhook_url: None,
                channel: None,
                routing_key: None,
                severity: None,
                has_secret: wh.secret.is_some(),
                headers: wh
                    .headers
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            },
            E::Email(em) => Self {
                channel_type: "email".to_string(),
                url: None,
                smtp_host: Some(em.smtp_host.clone()),
                smtp_port: Some(em.smtp_port),
                from: Some(em.from.clone()),
                to: Some(em.to.clone()),
                webhook_url: None,
                channel: None,
                routing_key: None,
                severity: None,
                has_secret: false,
                headers: HashMap::new(),
            },
            E::Slack(sl) => Self {
                channel_type: "slack".to_string(),
                url: None,
                smtp_host: None,
                smtp_port: None,
                from: None,
                to: None,
                webhook_url: Some(sl.webhook_url.clone()),
                channel: sl.channel.clone(),
                routing_key: None,
                severity: None,
                has_secret: false,
                headers: HashMap::new(),
            },
            E::PagerDuty(pd) => Self {
                channel_type: "pagerduty".to_string(),
                url: None,
                smtp_host: None,
                smtp_port: None,
                from: None,
                to: None,
                webhook_url: None,
                channel: None,
                routing_key: Some(pd.routing_key.expose_secret().to_string()),
                severity: Some(pd.severity.clone()),
                has_secret: true,
                headers: HashMap::new(),
            },
        }
    }
}

/// Python wrapper for `EggsecConfig`.
///
/// Top-level configuration for the Eggsec security assessment engine.
/// Load with `EggsecConfig.load()` or `EggsecConfig.default()`.
#[pyclass(name = "EggsecConfig", frozen)]
#[derive(Clone)]
pub(crate) struct PyEggsecConfig {
    http: PyHttpConfig,
    scan: PyScanConfig,
    output: PyOutputConfig,
    recon: PyReconConfig,
    remote: PyRemoteConfig,
    proxies: Vec<PyProxyConfigEntry>,
    ai: Option<PyAiConfig>,
    search: Option<PySearchConfig>,
    paths: PyPathsConfig,
    alert_channels: HashMap<String, PyAlertChannelConfig>,
    profiles: HashMap<String, String>,
    auto_save_interval_secs: u64,
}

#[pymethods]
impl PyEggsecConfig {
    #[staticmethod]
    fn default() -> Self {
        let inner = eggsec::config::EggsecConfig::default();
        Self::from_inner(&inner)
    }

    #[staticmethod]
    fn load(path: Option<&str>) -> PyResult<Self> {
        let config = match path {
            Some(p) => eggsec::config::EggsecConfig::load(p)
                .map_err(|e| crate::error::ConfigError::new_err(e.to_string()))?,
            None => {
                let default_path =
                    eggsec::config::EggsecConfig::default_path().ok_or_else(|| {
                        crate::error::ConfigError::new_err(
                            "Could not determine default config path".to_string(),
                        )
                    })?;
                eggsec::config::EggsecConfig::load(&default_path)
                    .map_err(|e| crate::error::ConfigError::new_err(e.to_string()))?
            }
        };
        Ok(Self::from_inner(&config))
    }

    fn validate(&self) -> PyResult<()> {
        let inner = self.to_inner();
        inner
            .validate()
            .map_err(|e| crate::error::ConfigError::new_err(e.to_string()))
    }

    #[staticmethod]
    fn default_path() -> Option<String> {
        eggsec::config::EggsecConfig::default_path().map(|p| p.to_string_lossy().to_string())
    }

    fn save(&self, path: &str) -> PyResult<()> {
        let inner = self.to_inner();
        inner
            .save(path)
            .map_err(|e| crate::error::ConfigError::new_err(e.to_string()))
    }

    #[getter]
    fn http(&self) -> PyHttpConfig {
        self.http.clone()
    }

    #[getter]
    fn scan(&self) -> PyScanConfig {
        self.scan.clone()
    }

    #[getter]
    fn output(&self) -> PyOutputConfig {
        self.output.clone()
    }

    #[getter]
    fn recon(&self) -> PyReconConfig {
        self.recon.clone()
    }

    #[getter]
    fn remote(&self) -> PyRemoteConfig {
        self.remote.clone()
    }

    #[getter]
    fn proxies(&self) -> Vec<PyProxyConfigEntry> {
        self.proxies.clone()
    }

    #[getter]
    fn ai(&self) -> Option<PyAiConfig> {
        self.ai.clone()
    }

    #[getter]
    fn search(&self) -> Option<PySearchConfig> {
        self.search.clone()
    }

    #[getter]
    fn paths(&self) -> PyPathsConfig {
        self.paths.clone()
    }

    #[getter]
    fn alert_channels(&self) -> HashMap<String, PyAlertChannelConfig> {
        self.alert_channels.clone()
    }

    #[getter]
    fn profiles(&self) -> HashMap<String, String> {
        self.profiles.clone()
    }

    #[getter]
    fn auto_save_interval_secs(&self) -> u64 {
        self.auto_save_interval_secs
    }

    fn __repr__(&self) -> String {
        format!(
            "EggsecConfig(proxies={}, profiles={}, has_ai={})",
            self.proxies.len(),
            self.profiles.len(),
            self.ai.is_some()
        )
    }
}

impl PyEggsecConfig {
    /// Create a default config (pub(crate) for internal use).
    pub(crate) fn new_default() -> Self {
        let inner = eggsec::config::EggsecConfig::default();
        Self::from_inner(&inner)
    }

    /// Get the default concurrency from scan config.
    pub(crate) fn default_concurrency(&self) -> usize {
        self.scan.default_concurrency
    }

    pub(crate) fn from_inner(config: &eggsec::config::EggsecConfig) -> Self {
        let profiles: HashMap<String, String> = config
            .profiles
            .iter()
            .map(|(k, v)| (k.clone(), v.name.clone()))
            .collect();

        let alert_channels: HashMap<String, PyAlertChannelConfig> = config
            .alert_channels
            .channels
            .iter()
            .map(|(k, v)| (k.clone(), PyAlertChannelConfig::from_inner(k, v)))
            .collect();

        Self {
            http: PyHttpConfig::from_inner(&config.http),
            scan: PyScanConfig::from_inner(&config.scan),
            output: PyOutputConfig::from_inner(&config.output),
            recon: PyReconConfig::from_inner(&config.recon),
            remote: PyRemoteConfig::from_inner(&config.remote),
            proxies: config
                .proxies
                .iter()
                .map(PyProxyConfigEntry::from_inner)
                .collect(),
            ai: config.ai.as_ref().map(PyAiConfig::from_inner),
            search: config.search.as_ref().map(PySearchConfig::from_inner),
            paths: PyPathsConfig::from_inner(&config.paths),
            alert_channels,
            profiles,
            auto_save_interval_secs: config.auto_save_interval_secs,
        }
    }

    pub(crate) fn to_inner(&self) -> eggsec::config::EggsecConfig {
        let http = eggsec::config::HttpConfig {
            timeout_secs: self.http.timeout_secs,
            max_retries: self.http.max_retries,
            retry_delay_ms: self.http.retry_delay_ms,
            verify_tls: self.http.verify_tls,
            follow_redirects: self.http.follow_redirects,
            max_redirects: self.http.max_redirects,
            default_headers: self
                .http
                .default_headers
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            default_user_agent: self.http.default_user_agent.clone(),
            proxy: self.http.proxy.clone(),
            proxy_auth: self
                .http
                .proxy_auth
                .as_ref()
                .map(|s| eggsec_core::types::SensitiveString::new(s.inner.expose_secret())),
        };

        let scan = eggsec::config::ScanConfig {
            default_concurrency: self.scan.default_concurrency,
            rate_limit_per_second: self.scan.rate_limit_per_second,
            jitter_ms: self.scan.jitter_ms,
            stealth_mode: self.scan.stealth_mode,
            exclude_ports: self.scan.exclude_ports.clone(),
            exclude_hosts: self.scan.exclude_hosts.clone(),
            port_timeout_secs: self.scan.port_timeout_secs,
            save_session: self.scan.save_session,
            session_dir: None,
        };

        let output = eggsec::config::OutputConfig {
            format: self.output.format.parse().unwrap_or_default(),
            verbosity: match self.output.verbosity.as_str() {
                "quiet" => eggsec::config::Verbosity::Quiet,
                "verbose" => eggsec::config::Verbosity::Verbose,
                "debug" => eggsec::config::Verbosity::Debug,
                _ => eggsec::config::Verbosity::Normal,
            },
            color: self.output.color,
            progress_bars: self.output.progress_bars,
            save_results: self.output.save_results,
            results_dir: self
                .output
                .results_dir
                .as_ref()
                .map(std::path::PathBuf::from),
            include_timestamp: self.output.include_timestamp,
        };

        let remote = eggsec::config::RemoteConfig {
            psk: self
                .remote
                .psk
                .as_ref()
                .map(|s| eggsec_core::types::SensitiveString::new(s.inner.expose_secret())),
            default_port: self.remote.default_port,
            allowed_workers: self
                .remote
                .allowed_workers
                .iter()
                .map(|w| eggsec::config::AllowedWorker {
                    host: w.host.clone(),
                    port: w.port,
                })
                .collect(),
        };

        let proxies: Vec<eggsec::config::ProxyConfigEntry> = self
            .proxies
            .iter()
            .map(|p| {
                let proxy_type = match p.proxy_type.as_str() {
                    "socks4" => eggsec::proxy::ProxyType::Socks4,
                    "http" => eggsec::proxy::ProxyType::Http,
                    "https" => eggsec::proxy::ProxyType::Https,
                    "tor" => eggsec::proxy::ProxyType::Tor,
                    _ => eggsec::proxy::ProxyType::Socks5,
                };
                eggsec::config::ProxyConfigEntry {
                    proxy_type,
                    address: p.address.clone(),
                    port: p.port,
                    username: p.username.clone(),
                    password: None,
                    local_addr: p.local_addr.clone(),
                    weight: p.weight,
                    priority: p.priority,
                    enabled: p.enabled,
                }
            })
            .collect();

        let ai = self.ai.as_ref().map(|a| eggsec::config::AiConfig {
            provider: a.provider.clone(),
            model: a.model.clone(),
            api_key: a
                .api_key
                .as_ref()
                .map(|s| eggsec_core::types::SensitiveString::new(s.inner.expose_secret())),
            base_url: a.base_url.clone(),
            max_tokens: a.max_tokens,
            temperature: a.temperature,
            max_payloads: a.max_payloads,
            max_bypasses: a.max_bypasses,
        });

        let search = self.search.as_ref().map(|s| eggsec::config::SearchConfig {
            enabled: s.enabled,
            searxng_url: s.url.clone(),
            engines: s.engines.clone(),
            cache_ttl_seconds: s.cache_ttl_seconds,
        });

        let paths = eggsec::config::PathsConfig {
            custom_payloads_dir: self
                .paths
                .custom_payloads_dir
                .as_ref()
                .map(std::path::PathBuf::from),
            wordlists_dir: self
                .paths
                .wordlists_dir
                .as_ref()
                .map(std::path::PathBuf::from),
            export_dir: self.paths.export_dir.clone(),
        };

        let profiles: rustc_hash::FxHashMap<String, eggsec::config::ScanProfile> = self
            .profiles
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    eggsec::config::ScanProfile {
                        name: v.clone(),
                        ..Default::default()
                    },
                )
            })
            .collect();

        eggsec::config::EggsecConfig {
            http,
            scan,
            output,
            notifications: eggsec::config::NotificationConfig::default(),
            profiles,
            paths,
            recon: eggsec::config::ReconConfig {
                dns_concurrency: self.recon.dns_concurrency,
                apis: self.recon.apis.to_inner(),
            },
            schedule: Vec::new(),
            remote,
            proxies,
            ai,
            search,
            alert_channels: eggsec::config::AlertChannelsConfig::default(),
            execution_policy: eggsec::config::ExecutionPolicy::default(),
            auto_save_interval_secs: self.auto_save_interval_secs,
        }
    }
}

impl PyReconApiConfig {
    pub(crate) fn to_inner(&self) -> eggsec::config::ApiConfig {
        use eggsec::config::{ApiKeyConfig, IpApiConfig, MaxMindConfig, NvdConfig, WaybackConfig};

        eggsec::config::ApiConfig {
            virustotal: ApiKeyConfig {
                enabled: self.virustotal_enabled,
                api_key: self
                    .virustotal_api_key
                    .as_ref()
                    .map(|s| eggsec_core::types::SensitiveString::new(s.inner.expose_secret())),
            },
            alienvault: ApiKeyConfig {
                enabled: self.alienvault_enabled,
                api_key: self
                    .alienvault_api_key
                    .as_ref()
                    .map(|s| eggsec_core::types::SensitiveString::new(s.inner.expose_secret())),
            },
            shodan: ApiKeyConfig {
                enabled: self.shodan_enabled,
                api_key: self
                    .shodan_api_key
                    .as_ref()
                    .map(|s| eggsec_core::types::SensitiveString::new(s.inner.expose_secret())),
            },
            ipapi: IpApiConfig {
                enabled: self.ipapi_enabled,
                api_key: self
                    .ipapi_api_key
                    .as_ref()
                    .map(|s| eggsec_core::types::SensitiveString::new(s.inner.expose_secret())),
            },
            maxmind: MaxMindConfig {
                enabled: self.maxmind_enabled,
                account_id: self.maxmind_account_id,
                license_key: self
                    .maxmind_license_key
                    .as_ref()
                    .map(|s| eggsec_core::types::SensitiveString::new(s.inner.expose_secret())),
                edition_ids: Vec::new(),
                data_dir: std::path::PathBuf::from("."),
                auto_update: false,
            },
            wayback_machine: WaybackConfig {
                enabled: self.wayback_enabled,
                api_key: self
                    .wayback_api_key
                    .as_ref()
                    .map(|s| eggsec_core::types::SensitiveString::new(s.inner.expose_secret())),
            },
            nvd: NvdConfig {
                api_key: self
                    .nvd_api_key
                    .as_ref()
                    .map(|s| eggsec_core::types::SensitiveString::new(s.inner.expose_secret())),
            },
        }
    }
}
