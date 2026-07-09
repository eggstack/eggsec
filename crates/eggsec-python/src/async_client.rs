use pyo3::prelude::*;

use crate::dto::PortScanResult;
use crate::endpoint::EndpointScanResult;
use crate::error::EggsecResultExt;
use crate::fingerprint::FingerprintScanResult;
use crate::recon::{DnsRecordSet, TechDetectionResult, TlsInspectionResult};
use crate::runtime_async;
use crate::scope::Scope;
use crate::waf::WafDetectionResultPy;

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

    /// Perform async passive DNS reconnaissance on a domain.
    fn recon_dns(&self, domain: &str) -> PyResult<crate::runtime_async::PyFuture> {
        self.scope.enforce_target(domain)?;

        let domain_owned = domain.to_string();

        runtime_async::spawn_async(async move {
            let result = eggsec::recon::dns_records::enumerate_dns_records(&domain_owned)
                .await
                .map_pyerr()?;

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
        })
    }

    /// Perform async TLS inspection.
    #[pyo3(signature = (host, *, port=443))]
    fn inspect_tls(&self, host: &str, port: u16) -> PyResult<crate::runtime_async::PyFuture> {
        self.scope.enforce_target(host)?;

        let host_owned = host.to_string();

        runtime_async::spawn_async(async move {
            let result = eggsec::recon::ssl::analyze_ssl(&host_owned, port)
                .await
                .map_pyerr()?;

            Ok(TlsInspectionResult {
                target: result.target,
                has_ssl: result.has_ssl,
                certificate: result.certificate.map(|c| crate::recon::TlsCertificateInfo {
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
        })
    }

    /// Perform async technology detection.
    fn detect_technology(&self, url: &str) -> PyResult<crate::runtime_async::PyFuture> {
        let host = extract_host_from_url(url)?;
        self.scope.enforce_target(&host)?;

        let url_owned = url.to_string();

        runtime_async::spawn_async(async move {
            let result = eggsec::recon::techdetect::detect_tech_stack(&url_owned)
                .await
                .map_pyerr()?;

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
        })
    }

    /// Perform async WAF detection.
    fn detect_waf(&self, url: &str) -> PyResult<crate::runtime_async::PyFuture> {
        let host = extract_host_from_url(url)?;
        self.scope.enforce_target(&host)?;

        let url_owned = url.to_string();

        runtime_async::spawn_async(async move {
            let detector = eggsec::waf::WafDetector::new().map_pyerr()?;
            let result = detector.detect(&url_owned).await.map_pyerr()?;

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
