use pyo3::prelude::*;

use crate::dto::PortScanResult;
use crate::endpoint::EndpointScanResult;
use crate::engine::Engine;
use crate::error::EggsecResultExt;
use crate::fingerprint::FingerprintScanResult;
use crate::recon::{DnsRecordSet, TechDetectionResult, TlsInspectionResult};
use crate::runtime_sync;
use crate::scope::Scope;
use crate::waf::WafDetectionResultPy;

/// Client for performing scoped security scans through the Eggsec engine.
///
/// Delegates scope enforcement and configuration to an internal `Engine`.
/// The GIL is released during network I/O operations.
#[pyclass]
pub struct Client {
    engine: Engine,
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
        Ok(Self {
            engine: Engine::new_inner(scope, mode, concurrency, timeout_ms)?,
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
        self.engine.enforce_target(target)?;

        let effective_concurrency = concurrency.unwrap_or(self.engine.get_concurrency());
        let effective_timeout_ms = timeout_ms.unwrap_or(self.engine.get_timeout_ms());

        for &port in &ports {
            self.engine.enforce_port(port)?;
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
                eggsec::scanner::scan_ports(&target_owned, config)
                    .await
                    .map_pyerr()
            })?;

            Ok(PortScanResult::from_engine(result))
        })
    }

    /// Perform endpoint discovery against a web server.
    ///
    /// Args:
    ///     base_url: Base URL to scan (e.g. "https://example.com").
    ///     endpoints: List of paths to probe (e.g. ["admin", "login"]).
    ///     concurrency: Max concurrent requests (overrides client default).
    ///     timeout_ms: Request timeout in ms (overrides client default).
    ///     include_404: Include 404 responses (default: False).
    ///     verify_tls: Verify TLS certificates (default: True).
    ///
    /// Returns:
    ///     EndpointScanResult: Structured endpoint discovery results.
    ///
    /// Raises:
    ///     EnforcementError: If the target is outside the allowed scope.
    ///     ScanError: If the scan fails.
    #[pyo3(signature = (base_url, endpoints, *, concurrency=None, timeout_ms=None, include_404=false, verify_tls=true))]
    fn scan_endpoints(
        &self,
        py: Python<'_>,
        base_url: &str,
        endpoints: Vec<String>,
        concurrency: Option<usize>,
        timeout_ms: Option<u64>,
        include_404: bool,
        verify_tls: bool,
    ) -> PyResult<EndpointScanResult> {
        // Extract host from URL for scope enforcement
        let host = extract_host_from_url(base_url)?;
        self.engine.enforce_target(&host)?;

        let effective_concurrency = concurrency.unwrap_or(self.engine.get_concurrency());
        let effective_timeout_ms = timeout_ms.unwrap_or(self.engine.get_timeout_ms());

        let config = eggsec::scanner::EndpointScanConfig {
            base_url: base_url.to_string(),
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

        let result = runtime_sync::block_on(py, async move {
            eggsec::scanner::scan_endpoints(config).await.map_pyerr()
        })?;

        Ok(EndpointScanResult::from_engine(result))
    }

    /// Perform service fingerprinting on target ports.
    ///
    /// Args:
    ///     target: Hostname or IP to fingerprint.
    ///     ports: List of port numbers to fingerprint.
    ///     concurrency: Max concurrent connections (overrides client default).
    ///     timeout_ms: Connection timeout in ms (overrides client default).
    ///
    /// Returns:
    ///     FingerprintScanResult: Structured fingerprinting results.
    ///
    /// Raises:
    ///     EnforcementError: If the target is outside the allowed scope.
    ///     ScanError: If the scan fails.
    #[pyo3(signature = (target, ports, *, concurrency=None, timeout_ms=None))]
    fn fingerprint_services(
        &self,
        py: Python<'_>,
        target: &str,
        ports: Vec<u16>,
        concurrency: Option<usize>,
        timeout_ms: Option<u64>,
    ) -> PyResult<FingerprintScanResult> {
        self.engine.enforce_target(target)?;

        let effective_concurrency = concurrency.unwrap_or(self.engine.get_concurrency());
        let effective_timeout_ms = timeout_ms.unwrap_or(self.engine.get_timeout_ms());

        for &port in &ports {
            self.engine.enforce_port(port)?;
        }

        let target_owned = target.to_string();

        let result = runtime_sync::block_on(py, async move {
            eggsec::scanner::fingerprint_services(
                &target_owned,
                ports,
                std::time::Duration::from_millis(effective_timeout_ms),
                false,
                effective_concurrency,
                None,
                None,
            )
            .await
            .map_pyerr()
        })?;

        Ok(FingerprintScanResult::from_engine(result))
    }

    /// Perform passive DNS reconnaissance on a domain.
    ///
    /// Args:
    ///     domain: Domain name to enumerate (e.g. "example.com").
    ///
    /// Returns:
    ///     DnsRecordSet: DNS records for the domain.
    ///
    /// Raises:
    ///     EnforcementError: If the target is outside the allowed scope.
    ///     NetworkError: If DNS resolution fails.
    fn recon_dns(&self, py: Python<'_>, domain: &str) -> PyResult<DnsRecordSet> {
        self.engine.enforce_target(domain)?;

        let domain_owned = domain.to_string();
        let result = runtime_sync::block_on(py, async move {
            eggsec::recon::dns_records::enumerate_dns_records(&domain_owned)
                .await
                .map_pyerr()
        })?;

        Ok(DnsRecordSet {
            domain: result.domain,
            a_records: result.a,
            aaaa_records: result.aaaa,
            cname_records: result.cname,
            mx_records: result
                .mx
                .into_iter()
                .map(|m| crate::recon::MxRecord {
                    preference: m.preference,
                    exchange: m.exchange,
                })
                .collect(),
            txt_records: result.txt,
            ns_records: result.ns,
            soa_record: result.soa.map(|s| crate::recon::SoaRecord {
                mname: s.mname,
                rname: s.rname,
                serial: s.serial,
                refresh: s.refresh,
                retry: s.retry,
                expire: s.expire,
                minimum: s.minimum,
            }),
            caa_records: result.caa,
        })
    }

    /// Inspect TLS certificate and configuration for a host.
    ///
    /// Args:
    ///     host: Hostname to inspect (e.g. "example.com").
    ///     port: TLS port (default: 443).
    ///
    /// Returns:
    ///     TlsInspectionResult: TLS certificate and configuration details.
    ///
    /// Raises:
    ///     EnforcementError: If the target is outside the allowed scope.
    ///     NetworkError: If TLS connection fails.
    #[pyo3(signature = (host, *, port=443))]
    fn inspect_tls(&self, py: Python<'_>, host: &str, port: u16) -> PyResult<TlsInspectionResult> {
        self.engine.enforce_target(host)?;

        let host_owned = host.to_string();
        let result = runtime_sync::block_on(py, async move {
            eggsec::recon::ssl::analyze_ssl(&host_owned, port)
                .await
                .map_pyerr()
        })?;

        Ok(TlsInspectionResult {
            target: result.target,
            has_ssl: result.has_ssl,
            certificate: result
                .certificate
                .map(|c| crate::recon::TlsCertificateInfo {
                    subject: c.subject,
                    issuer: c.issuer,
                    valid_from: c.valid_from,
                    valid_until: c.valid_until,
                    serial_number: c.serial_number,
                    signature_algorithm: c.signature_algorithm,
                    public_key_algorithm: c.public_key_algorithm,
                    key_size: c.key_size,
                    is_expired: c.is_expired,
                    days_until_expiry: c.days_until_expiry,
                    sans: c.subject_alternative_names,
                }),
            supported_versions: result.supported_versions,
            supported_cipher_suites: result.supported_cipher_suites,
            issues: result
                .issues
                .into_iter()
                .map(|i| crate::recon::SslIssue {
                    severity: i.severity,
                    code: i.code,
                    description: i.description,
                })
                .collect(),
        })
    }

    /// Detect technology stack from HTTP response headers and body.
    ///
    /// Args:
    ///     url: Full URL to inspect (e.g. "https://example.com").
    ///
    /// Returns:
    ///     TechDetectionResult: Detected technology stack.
    ///
    /// Raises:
    ///     EnforcementError: If the target is outside the allowed scope.
    ///     NetworkError: If the HTTP request fails.
    fn detect_technology(&self, py: Python<'_>, url: &str) -> PyResult<TechDetectionResult> {
        let host = extract_host_from_url(url)?;
        self.engine.enforce_target(&host)?;

        let url_owned = url.to_string();
        let result = runtime_sync::block_on(py, async move {
            eggsec::recon::techdetect::detect_tech_stack(&url_owned)
                .await
                .map_pyerr()
        })?;

        Ok(TechDetectionResult {
            url: result.url,
            status_code: result.status_code,
            headers: result.headers.into_iter().collect(),
            tech_stack: crate::recon::TechStack {
                servers: result.tech_stack.servers,
                frameworks: result.tech_stack.frameworks,
                languages: result.tech_stack.languages,
                databases: result.tech_stack.databases,
                cdns: result.tech_stack.cdns,
                cms: result.tech_stack.cms,
                javascript: result.tech_stack.javascript,
                other: result.tech_stack.other,
            },
        })
    }

    /// Detect WAF by making an HTTP request to the target URL.
    ///
    /// This performs passive detection only - no bypass or validation testing.
    ///
    /// Args:
    ///     url: Target URL to test (e.g. "https://example.com").
    ///
    /// Returns:
    ///     WafDetectionResultPy: WAF detection result with vendor, confidence, and evidence.
    ///
    /// Raises:
    ///     EnforcementError: If the target is outside the allowed scope.
    ///     NetworkError: If the HTTP request fails.
    fn detect_waf(&self, py: Python<'_>, url: &str) -> PyResult<WafDetectionResultPy> {
        let host = extract_host_from_url(url)?;
        self.engine.enforce_target(&host)?;

        let url_owned = url.to_string();
        let url_clone = url_owned.clone();
        let result = runtime_sync::block_on(py, async move {
            let detector = eggsec::waf::WafDetector::new().map_pyerr()?;
            detector.detect(&url_clone).await.map_pyerr()
        })?;

        Ok(WafDetectionResultPy {
            url: url_owned,
            detected: result.waf_name.is_some(),
            vendor: result.waf_name.clone(),
            waf_name: result.waf_name,
            confidence: result.confidence,
            matched_headers: result.matched_headers,
            matched_cookies: result.matched_cookies,
            matched_patterns: result.matched_patterns,
            server_header: result.server_header,
            status_code: result.status_code,
            request_error: result.request_error,
        })
    }

    /// Run an HTTP load test against a scoped target.
    #[pyo3(signature = (url, total_requests, concurrency, timeout_secs, *, method="GET"))]
    fn load_test_http(
        &self,
        py: Python<'_>,
        url: &str,
        total_requests: u64,
        concurrency: usize,
        timeout_secs: u64,
        method: &str,
    ) -> PyResult<crate::loadtest::LoadTestResultPy> {
        let host = extract_host_from_url(url)?;
        self.engine.enforce_target(&host)?;

        if total_requests == 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "total_requests must be > 0",
            ));
        }
        if concurrency == 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "concurrency must be > 0",
            ));
        }
        if timeout_secs == 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "timeout_secs must be > 0",
            ));
        }

        crate::loadtest::load_test_http(
            py,
            url,
            total_requests,
            concurrency,
            timeout_secs,
            self.engine.scope_ref().clone(),
            method,
        )
    }

    /// Validate WAF protection on a scoped target.
    #[pyo3(signature = (url, *, bypass=false, test_type=None))]
    fn validate_waf(
        &self,
        url: &str,
        bypass: bool,
        test_type: Option<&str>,
    ) -> PyResult<crate::waf_validation::WafScanResultPy> {
        let host = extract_host_from_url(url)?;
        self.engine.enforce_target(&host)?;

        crate::waf_validation::validate_waf(url, self.engine.scope_ref().clone(), bypass, test_type)
    }

    /// Run HTTP fuzzing against a scoped target.
    #[pyo3(signature = (url, payload_type="all", *, method="GET", param=None, concurrency=10, timeout=30))]
    fn fuzz_http(
        &self,
        url: &str,
        payload_type: &str,
        method: &str,
        param: Option<&str>,
        concurrency: usize,
        timeout: u64,
    ) -> PyResult<crate::waf_validation::FuzzSessionPy> {
        let host = extract_host_from_url(url)?;
        self.engine.enforce_target(&host)?;

        if concurrency == 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "concurrency must be > 0",
            ));
        }
        if timeout == 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "timeout must be > 0",
            ));
        }

        crate::waf_validation::fuzz_http(
            url,
            self.engine.scope_ref().clone(),
            payload_type,
            method,
            param,
            concurrency,
            timeout,
        )
    }

    /// Get the client's scope.
    #[getter]
    fn scope(&self) -> Scope {
        self.engine.scope_ref().clone()
    }

    /// Get the client's mode.
    #[getter]
    fn mode(&self) -> String {
        self.engine.get_mode().to_string()
    }

    /// Close the client (no-op for sync client, exists for API consistency).
    fn close(&self) {}

    /// Context manager __enter__.
    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    /// Context manager __exit__.
    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        false // Don't suppress exceptions
    }

    fn __repr__(&self) -> String {
        format!("Client(mode={})", self.engine.get_mode())
    }
}

/// Extract hostname from a URL for scope enforcement.
fn extract_host_from_url(url: &str) -> PyResult<String> {
    // Try to parse as a URL
    let parsed = url::Url::parse(url)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid URL: {}", e)))?;

    parsed
        .host_str()
        .map(|h| h.to_string())
        .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("URL does not contain a valid host"))
}
