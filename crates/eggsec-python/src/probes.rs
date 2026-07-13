use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use crate::error::{EggsecResultExt, NetworkError, TimeoutError};
use crate::network::ConnectionTimingPy;
use crate::runtime_async;
use crate::runtime_sync;

// ============================================================================
// DNS Probe Types
// ============================================================================

/// Configuration for a DNS query.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsQueryConfigPy {
    #[pyo3(get)]
    pub domain: String,
    pub(crate) record_types: Vec<String>,
    #[pyo3(get)]
    pub resolver: Option<String>,
    #[pyo3(get)]
    pub timeout_ms: u64,
    #[pyo3(get)]
    pub tcp_fallback: bool,
    #[pyo3(get)]
    pub max_retries: u32,
}

#[pymethods]
impl DnsQueryConfigPy {
    #[new]
    #[pyo3(signature = (domain, record_types=None, resolver=None, timeout_ms=5000, tcp_fallback=true, max_retries=2))]
    fn new(
        domain: String,
        record_types: Option<Vec<String>>,
        resolver: Option<String>,
        timeout_ms: u64,
        tcp_fallback: bool,
        max_retries: u32,
    ) -> Self {
        Self {
            domain,
            record_types: record_types.unwrap_or_else(|| vec!["A".into(), "AAAA".into()]),
            resolver,
            timeout_ms,
            tcp_fallback,
            max_retries,
        }
    }

    #[getter]
    fn record_types(&self) -> Vec<String> {
        self.record_types.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("domain", &self.domain)?;
        dict.set_item("record_types", &self.record_types)?;
        dict.set_item("resolver", &self.resolver)?;
        dict.set_item("timeout_ms", self.timeout_ms)?;
        dict.set_item("tcp_fallback", self.tcp_fallback)?;
        dict.set_item("max_retries", self.max_retries)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "DnsQueryConfigPy(domain={}, types={:?}, timeout={}ms)",
            self.domain, self.record_types, self.timeout_ms
        )
    }

    fn __str__(&self) -> String {
        format!(
            "DNS query for {} ({}) timeout={}ms",
            self.domain,
            self.record_types.join(","),
            self.timeout_ms
        )
    }
}

/// A single DNS resource record.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecordPy {
    #[pyo3(get)]
    pub record_type: String,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub data: String,
    #[pyo3(get)]
    pub ttl: u32,
    #[pyo3(get)]
    pub class: String,
}

#[pymethods]
impl DnsRecordPy {
    #[new]
    #[pyo3(signature = (record_type, name, data, ttl=0, class="IN"))]
    fn new(record_type: String, name: String, data: String, ttl: u32, class: &str) -> Self {
        Self {
            record_type,
            name,
            data,
            ttl,
            class: class.to_string(),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("record_type", &self.record_type)?;
        dict.set_item("name", &self.name)?;
        dict.set_item("data", &self.data)?;
        dict.set_item("ttl", self.ttl)?;
        dict.set_item("class", &self.class)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "DnsRecordPy(type={}, name={}, data={})",
            self.record_type, self.name, self.data
        )
    }

    fn __str__(&self) -> String {
        format!(
            "{} {} {} TTL={} {}",
            self.record_type, self.name, self.data, self.ttl, self.class
        )
    }
}

/// Result of a DNS query.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsQueryResultPy {
    #[pyo3(get)]
    pub domain: String,
    pub(crate) records: Vec<DnsRecordPy>,
    #[pyo3(get)]
    pub response_code: String,
    #[pyo3(get)]
    pub authoritative: bool,
    #[pyo3(get)]
    pub truncated: bool,
    #[pyo3(get)]
    pub resolver_used: String,
    #[pyo3(get)]
    pub timing: ConnectionTimingPy,
    #[pyo3(get)]
    pub raw_artifact: Option<String>,
    #[pyo3(get)]
    pub error: Option<String>,
}

#[pymethods]
impl DnsQueryResultPy {
    #[getter]
    fn records(&self) -> Vec<DnsRecordPy> {
        self.records.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("domain", &self.domain)?;

        let records_list = PyList::empty_bound(py);
        for record in &self.records {
            records_list.append(record.to_dict(py)?)?;
        }
        dict.set_item("records", records_list)?;
        dict.set_item("response_code", &self.response_code)?;
        dict.set_item("authoritative", self.authoritative)?;
        dict.set_item("truncated", self.truncated)?;
        dict.set_item("resolver_used", &self.resolver_used)?;
        dict.set_item("timing", crate::network::timing_to_dict(py, &self.timing)?)?;
        dict.set_item("raw_artifact", &self.raw_artifact)?;
        dict.set_item("error", &self.error)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "DnsQueryResultPy(domain={}, records={}, rcode={}, error={:?})",
            self.domain,
            self.records.len(),
            self.response_code,
            self.error
        )
    }

    fn __str__(&self) -> String {
        if let Some(ref err) = self.error {
            format!(
                "DNS {} - {} records - error: {}",
                self.domain,
                self.records.len(),
                err
            )
        } else {
            format!(
                "DNS {} - {} records - {}",
                self.domain,
                self.records.len(),
                self.response_code
            )
        }
    }
}

// ============================================================================
// TLS Probe Types
// ============================================================================

/// Configuration for a TLS inspection probe.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsProbeConfigPy {
    #[pyo3(get)]
    pub host: String,
    #[pyo3(get)]
    pub port: u16,
    #[pyo3(get)]
    pub sni: Option<String>,
    pub(crate) alpn: Vec<String>,
    #[pyo3(get)]
    pub min_version: Option<String>,
    #[pyo3(get)]
    pub max_version: Option<String>,
    #[pyo3(get)]
    pub verify_certificate: bool,
    #[pyo3(get)]
    pub timeout_ms: u64,
    #[pyo3(get)]
    pub include_chain: bool,
}

#[pymethods]
impl TlsProbeConfigPy {
    #[new]
    #[pyo3(signature = (host, port=443, sni=None, alpn=None, min_version=None, max_version=None, verify_certificate=true, timeout_ms=10000, include_chain=true))]
    fn new(
        host: String,
        port: u16,
        sni: Option<String>,
        alpn: Option<Vec<String>>,
        min_version: Option<String>,
        max_version: Option<String>,
        verify_certificate: bool,
        timeout_ms: u64,
        include_chain: bool,
    ) -> Self {
        Self {
            host,
            port,
            sni,
            alpn: alpn.unwrap_or_else(|| vec!["http/1.1".into(), "h2".into()]),
            min_version,
            max_version,
            verify_certificate,
            timeout_ms,
            include_chain,
        }
    }

    #[getter]
    fn alpn(&self) -> Vec<String> {
        self.alpn.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("host", &self.host)?;
        dict.set_item("port", self.port)?;
        dict.set_item("sni", &self.sni)?;
        dict.set_item("alpn", &self.alpn)?;
        dict.set_item("min_version", &self.min_version)?;
        dict.set_item("max_version", &self.max_version)?;
        dict.set_item("verify_certificate", self.verify_certificate)?;
        dict.set_item("timeout_ms", self.timeout_ms)?;
        dict.set_item("include_chain", self.include_chain)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "TlsProbeConfigPy(host={}, port={}, verify={}, timeout={}ms)",
            self.host, self.port, self.verify_certificate, self.timeout_ms
        )
    }

    fn __str__(&self) -> String {
        format!(
            "TLS probe for {}:{} (verify={}, timeout={}ms)",
            self.host, self.port, self.verify_certificate, self.timeout_ms
        )
    }
}

/// Certificate information from a TLS probe.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateInfoPy {
    #[pyo3(get)]
    pub subject: String,
    #[pyo3(get)]
    pub issuer: String,
    #[pyo3(get)]
    pub valid_from: String,
    #[pyo3(get)]
    pub valid_until: String,
    #[pyo3(get)]
    pub serial_number: String,
    #[pyo3(get)]
    pub signature_algorithm: String,
    #[pyo3(get)]
    pub public_key_algorithm: String,
    #[pyo3(get)]
    pub key_size: Option<u32>,
    #[pyo3(get)]
    pub is_expired: bool,
    #[pyo3(get)]
    pub days_until_expiry: Option<i64>,
    pub(crate) subject_alternative_names: Vec<String>,
    pub(crate) chain: Vec<CertificateChainEntryPy>,
}

#[pymethods]
impl CertificateInfoPy {
    #[getter]
    fn subject_alternative_names(&self) -> Vec<String> {
        self.subject_alternative_names.clone()
    }

    #[getter]
    fn chain(&self) -> Vec<CertificateChainEntryPy> {
        self.chain.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("subject", &self.subject)?;
        dict.set_item("issuer", &self.issuer)?;
        dict.set_item("valid_from", &self.valid_from)?;
        dict.set_item("valid_until", &self.valid_until)?;
        dict.set_item("serial_number", &self.serial_number)?;
        dict.set_item("signature_algorithm", &self.signature_algorithm)?;
        dict.set_item("public_key_algorithm", &self.public_key_algorithm)?;
        dict.set_item("key_size", self.key_size)?;
        dict.set_item("is_expired", self.is_expired)?;
        dict.set_item("days_until_expiry", self.days_until_expiry)?;
        dict.set_item("subject_alternative_names", &self.subject_alternative_names)?;

        let chain_list = PyList::empty_bound(py);
        for entry in &self.chain {
            let entry_dict = PyDict::new_bound(py);
            entry_dict.set_item("subject", &entry.subject)?;
            entry_dict.set_item("issuer", &entry.issuer)?;
            entry_dict.set_item("valid_from", &entry.valid_from)?;
            entry_dict.set_item("valid_until", &entry.valid_until)?;
            chain_list.append(entry_dict)?;
        }
        dict.set_item("chain", chain_list)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "CertificateInfoPy(subject={}, issuer={}, expired={})",
            self.subject, self.issuer, self.is_expired
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Certificate '{}' issued by '{}' (expires: {}, {} days left)",
            self.subject,
            self.issuer,
            self.valid_until,
            self.days_until_expiry
                .map(|d| d.to_string())
                .unwrap_or_else(|| "?".into())
        )
    }
}

/// A single entry in a certificate chain.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateChainEntryPy {
    #[pyo3(get)]
    pub subject: String,
    #[pyo3(get)]
    pub issuer: String,
    #[pyo3(get)]
    pub valid_from: String,
    #[pyo3(get)]
    pub valid_until: String,
}

#[pymethods]
impl CertificateChainEntryPy {
    #[new]
    fn new(subject: String, issuer: String, valid_from: String, valid_until: String) -> Self {
        Self {
            subject,
            issuer,
            valid_from,
            valid_until,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("subject", &self.subject)?;
        dict.set_item("issuer", &self.issuer)?;
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
            "CertificateChainEntryPy(subject={}, issuer={})",
            self.subject, self.issuer
        )
    }

    fn __str__(&self) -> String {
        format!("{} -> {}", self.subject, self.issuer)
    }
}

/// Result of a TLS probe.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsProbeResultPy {
    #[pyo3(get)]
    pub host: String,
    #[pyo3(get)]
    pub port: u16,
    #[pyo3(get)]
    pub has_tls: bool,
    #[pyo3(get)]
    pub tls_version: Option<String>,
    #[pyo3(get)]
    pub cipher_suite: Option<String>,
    #[pyo3(get)]
    pub alpn_negotiated: Option<String>,
    pub(crate) certificate: Option<CertificateInfoPy>,
    #[pyo3(get)]
    pub hostname_verified: bool,
    #[pyo3(get)]
    pub certificate_verified: bool,
    pub(crate) supported_versions: Vec<String>,
    pub(crate) supported_ciphers: Vec<String>,
    pub(crate) issues: Vec<TlsIssuePy>,
    #[pyo3(get)]
    pub timing: ConnectionTimingPy,
    #[pyo3(get)]
    pub error: Option<String>,
}

#[pymethods]
impl TlsProbeResultPy {
    #[getter]
    fn certificate(&self) -> Option<CertificateInfoPy> {
        self.certificate.clone()
    }

    #[getter]
    fn supported_versions(&self) -> Vec<String> {
        self.supported_versions.clone()
    }

    #[getter]
    fn supported_ciphers(&self) -> Vec<String> {
        self.supported_ciphers.clone()
    }

    #[getter]
    fn issues(&self) -> Vec<TlsIssuePy> {
        self.issues.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("host", &self.host)?;
        dict.set_item("port", self.port)?;
        dict.set_item("has_tls", self.has_tls)?;
        dict.set_item("tls_version", &self.tls_version)?;
        dict.set_item("cipher_suite", &self.cipher_suite)?;
        dict.set_item("alpn_negotiated", &self.alpn_negotiated)?;

        if let Some(ref cert) = self.certificate {
            dict.set_item("certificate", cert.to_dict(py)?)?;
        } else {
            dict.set_item("certificate", py.None())?;
        }

        dict.set_item("hostname_verified", self.hostname_verified)?;
        dict.set_item("certificate_verified", self.certificate_verified)?;
        dict.set_item("supported_versions", &self.supported_versions)?;
        dict.set_item("supported_ciphers", &self.supported_ciphers)?;

        let issues_list = PyList::empty_bound(py);
        for issue in &self.issues {
            let issue_dict = PyDict::new_bound(py);
            issue_dict.set_item("severity", &issue.severity)?;
            issue_dict.set_item("code", &issue.code)?;
            issue_dict.set_item("description", &issue.description)?;
            issues_list.append(issue_dict)?;
        }
        dict.set_item("issues", issues_list)?;
        dict.set_item("timing", crate::network::timing_to_dict(py, &self.timing)?)?;
        dict.set_item("error", &self.error)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "TlsProbeResultPy(host={}:{}, has_tls={}, version={:?}, errors={:?})",
            self.host, self.port, self.has_tls, self.tls_version, self.error
        )
    }

    fn __str__(&self) -> String {
        if let Some(ref err) = self.error {
            format!("TLS {}:{} - error: {}", self.host, self.port, err)
        } else if self.has_tls {
            format!(
                "TLS {}:{} - {} / {} ({} issues)",
                self.host,
                self.port,
                self.tls_version.as_deref().unwrap_or("?"),
                self.cipher_suite.as_deref().unwrap_or("?"),
                self.issues.len()
            )
        } else {
            format!("TLS {}:{} - no TLS", self.host, self.port)
        }
    }
}

/// A TLS configuration or protocol issue.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsIssuePy {
    #[pyo3(get)]
    pub severity: String,
    #[pyo3(get)]
    pub code: String,
    #[pyo3(get)]
    pub description: String,
}

#[pymethods]
impl TlsIssuePy {
    #[new]
    fn new(severity: String, code: String, description: String) -> Self {
        Self {
            severity,
            code,
            description,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("severity", &self.severity)?;
        dict.set_item("code", &self.code)?;
        dict.set_item("description", &self.description)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("TlsIssuePy(severity={}, code={})", self.severity, self.code)
    }

    fn __str__(&self) -> String {
        format!("[{}] {}: {}", self.severity, self.code, self.description)
    }
}

// ============================================================================
// HTTP Probe Types
// ============================================================================

/// Configuration for an HTTP probe.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpProbeConfigPy {
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub method: String,
    pub(crate) headers: Vec<(String, String)>,
    #[pyo3(get)]
    pub body: Option<String>,
    #[pyo3(get)]
    pub follow_redirects: bool,
    #[pyo3(get)]
    pub max_redirects: u32,
    #[pyo3(get)]
    pub verify_tls: bool,
    #[pyo3(get)]
    pub timeout_ms: u64,
    #[pyo3(get)]
    pub user_agent: Option<String>,
}

#[pymethods]
impl HttpProbeConfigPy {
    #[new]
    #[pyo3(signature = (url, method="GET", headers=None, body=None, follow_redirects=true, max_redirects=10, verify_tls=true, timeout_ms=10000, user_agent=None))]
    fn new(
        url: String,
        method: &str,
        headers: Option<Vec<(String, String)>>,
        body: Option<String>,
        follow_redirects: bool,
        max_redirects: u32,
        verify_tls: bool,
        timeout_ms: u64,
        user_agent: Option<String>,
    ) -> Self {
        Self {
            url,
            method: method.to_uppercase(),
            headers: headers.unwrap_or_default(),
            body,
            follow_redirects,
            max_redirects,
            verify_tls,
            timeout_ms,
            user_agent,
        }
    }

    #[getter]
    fn headers(&self) -> Vec<(String, String)> {
        self.headers.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("url", &self.url)?;
        dict.set_item("method", &self.method)?;
        dict.set_item("headers", &self.headers)?;
        dict.set_item("body", &self.body)?;
        dict.set_item("follow_redirects", self.follow_redirects)?;
        dict.set_item("max_redirects", self.max_redirects)?;
        dict.set_item("verify_tls", self.verify_tls)?;
        dict.set_item("timeout_ms", self.timeout_ms)?;
        dict.set_item("user_agent", &self.user_agent)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "HttpProbeConfigPy(url={}, method={}, timeout={}ms)",
            self.url, self.method, self.timeout_ms
        )
    }

    fn __str__(&self) -> String {
        format!(
            "HTTP {} {} (timeout={}ms)",
            self.method, self.url, self.timeout_ms
        )
    }
}

/// Result of an HTTP probe.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpProbeResultPy {
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub final_url: Option<String>,
    #[pyo3(get)]
    pub status_code: u16,
    #[pyo3(get)]
    pub reason: Option<String>,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body_bytes: Vec<u8>,
    #[pyo3(get)]
    pub body_text: Option<String>,
    #[pyo3(get)]
    pub content_length: Option<u64>,
    #[pyo3(get)]
    pub redirect_count: u32,
    #[pyo3(get)]
    pub timing: ConnectionTimingPy,
    #[pyo3(get)]
    pub tls_metadata: Option<TlsProbeResultPy>,
    #[pyo3(get)]
    pub error: Option<String>,
}

#[pymethods]
impl HttpProbeResultPy {
    #[getter]
    fn headers(&self) -> Vec<(String, String)> {
        self.headers.clone()
    }

    #[getter]
    fn body_bytes(&self) -> Vec<u8> {
        self.body_bytes.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("url", &self.url)?;
        dict.set_item("final_url", &self.final_url)?;
        dict.set_item("status_code", self.status_code)?;
        dict.set_item("reason", &self.reason)?;

        let headers_list = PyList::empty_bound(py);
        for (k, v) in &self.headers {
            let pair = pyo3::types::PyTuple::new_bound(py, &[k, v]);
            headers_list.append(pair)?;
        }
        dict.set_item("headers", headers_list)?;

        dict.set_item("body_bytes", &self.body_bytes)?;
        dict.set_item("body_text", &self.body_text)?;
        dict.set_item("content_length", self.content_length)?;
        dict.set_item("redirect_count", self.redirect_count)?;
        dict.set_item("timing", crate::network::timing_to_dict(py, &self.timing)?)?;

        if let Some(ref tls) = self.tls_metadata {
            dict.set_item("tls_metadata", tls.to_dict(py)?)?;
        } else {
            dict.set_item("tls_metadata", py.None())?;
        }

        dict.set_item("error", &self.error)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "HttpProbeResultPy(url={}, status={}, redirects={}, error={:?})",
            self.url, self.status_code, self.redirect_count, self.error
        )
    }

    fn __str__(&self) -> String {
        if let Some(ref err) = self.error {
            format!("HTTP {} {} - error: {}", self.url, self.status_code, err)
        } else {
            format!(
                "HTTP {} {} - {} ({} bytes)",
                self.url,
                self.status_code,
                self.reason.as_deref().unwrap_or("?"),
                self.body_bytes.len()
            )
        }
    }
}

// ============================================================================
// DNS Probe Functions
// ============================================================================

/// Perform a DNS query for a domain.
///
/// Args:
///     domain: Domain name to query (e.g. "example.com").
///     record_types: DNS record types to query (default: ["A", "AAAA"]).
///     resolver: Custom resolver address (e.g. "8.8.8.8"). Uses system resolver if None.
///     timeout_ms: Query timeout in milliseconds (default: 5000).
///
/// Returns:
///     DnsQueryResultPy with DNS records and metadata.
///
/// Raises:
///     NetworkError: If DNS resolution fails.
#[pyfunction]
#[pyo3(signature = (domain, record_types=None, resolver=None, timeout_ms=5000))]
pub fn dns_query(
    domain: &str,
    record_types: Option<Vec<String>>,
    resolver: Option<&str>,
    timeout_ms: u64,
) -> PyResult<DnsQueryResultPy> {
    let domain_owned = domain.to_string();
    let record_types_owned = record_types.unwrap_or_else(|| vec!["A".into(), "AAAA".into()]);
    let resolver_owned = resolver.map(|s| s.to_string());
    let timeout = std::time::Duration::from_millis(timeout_ms);

    Python::with_gil(|py| {
        runtime_sync::block_on(py, async move {
            dns_query_impl(
                &domain_owned,
                &record_types_owned,
                resolver_owned.as_deref(),
                timeout,
            )
            .await
        })
    })
}

/// Perform an async DNS query.
#[pyfunction]
#[pyo3(signature = (domain, record_types=None, resolver=None, timeout_ms=5000))]
pub fn async_dns_query(
    domain: &str,
    record_types: Option<Vec<String>>,
    resolver: Option<&str>,
    timeout_ms: u64,
) -> PyResult<runtime_async::PyFuture> {
    let domain_owned = domain.to_string();
    let record_types_owned = record_types.unwrap_or_else(|| vec!["A".into(), "AAAA".into()]);
    let resolver_owned = resolver.map(|s| s.to_string());
    let timeout = std::time::Duration::from_millis(timeout_ms);

    runtime_async::spawn_async(async move {
        dns_query_impl(
            &domain_owned,
            &record_types_owned,
            resolver_owned.as_deref(),
            timeout,
        )
        .await
    })
}

async fn dns_query_impl(
    domain: &str,
    record_types: &[String],
    resolver_addr: Option<&str>,
    timeout: std::time::Duration,
) -> PyResult<DnsQueryResultPy> {
    let start = std::time::Instant::now();

    let resolver_source = resolver_addr.unwrap_or("system");

    let mut records = Vec::new();
    let mut response_code = "NOERROR".to_string();
    let mut authoritative = false;
    let mut truncated = false;
    let mut error_msg: Option<String> = None;

    let has_a = record_types.iter().any(|r| r.eq_ignore_ascii_case("A"));
    let has_aaaa = record_types.iter().any(|r| r.eq_ignore_ascii_case("AAAA"));
    let has_mx = record_types.iter().any(|r| r.eq_ignore_ascii_case("MX"));
    let has_txt = record_types.iter().any(|r| r.eq_ignore_ascii_case("TXT"));
    let has_ns = record_types.iter().any(|r| r.eq_ignore_ascii_case("NS"));
    let has_soa = record_types.iter().any(|r| r.eq_ignore_ascii_case("SOA"));
    let has_cname = record_types.iter().any(|r| r.eq_ignore_ascii_case("CNAME"));
    let has_srv = record_types.iter().any(|r| r.eq_ignore_ascii_case("SRV"));
    let has_ptr = record_types.iter().any(|r| r.eq_ignore_ascii_case("PTR"));
    let has_caa = record_types.iter().any(|r| r.eq_ignore_ascii_case("CAA"));

    match eggsec::recon::dns_records::enumerate_dns_records(domain).await {
        Ok(dns_records) => {
            if has_a {
                for ip in &dns_records.a {
                    records.push(DnsRecordPy {
                        record_type: "A".into(),
                        name: domain.to_string(),
                        data: ip.clone(),
                        ttl: 0,
                        class: "IN".into(),
                    });
                }
            }
            if has_aaaa {
                for ip in &dns_records.aaaa {
                    records.push(DnsRecordPy {
                        record_type: "AAAA".into(),
                        name: domain.to_string(),
                        data: ip.clone(),
                        ttl: 0,
                        class: "IN".into(),
                    });
                }
            }
            if has_mx {
                for mx in &dns_records.mx {
                    records.push(DnsRecordPy {
                        record_type: "MX".into(),
                        name: domain.to_string(),
                        data: format!("{} {}", mx.preference, mx.exchange),
                        ttl: 0,
                        class: "IN".into(),
                    });
                }
            }
            if has_txt {
                for txt in &dns_records.txt {
                    records.push(DnsRecordPy {
                        record_type: "TXT".into(),
                        name: domain.to_string(),
                        data: txt.clone(),
                        ttl: 0,
                        class: "IN".into(),
                    });
                }
            }
            if has_ns {
                for ns in &dns_records.ns {
                    records.push(DnsRecordPy {
                        record_type: "NS".into(),
                        name: domain.to_string(),
                        data: ns.clone(),
                        ttl: 0,
                        class: "IN".into(),
                    });
                }
            }
            if has_soa {
                if let Some(ref soa) = dns_records.soa {
                    records.push(DnsRecordPy {
                        record_type: "SOA".into(),
                        name: domain.to_string(),
                        data: format!(
                            "{} {} {} {} {} {} {}",
                            soa.mname,
                            soa.rname,
                            soa.serial,
                            soa.refresh,
                            soa.retry,
                            soa.expire,
                            soa.minimum
                        ),
                        ttl: 0,
                        class: "IN".into(),
                    });
                }
            }
            if has_cname {
                for cname in &dns_records.cname {
                    records.push(DnsRecordPy {
                        record_type: "CNAME".into(),
                        name: domain.to_string(),
                        data: cname.clone(),
                        ttl: 0,
                        class: "IN".into(),
                    });
                }
            }
            if has_caa {
                for caa in &dns_records.caa {
                    records.push(DnsRecordPy {
                        record_type: "CAA".into(),
                        name: domain.to_string(),
                        data: caa.clone(),
                        ttl: 0,
                        class: "IN".into(),
                    });
                }
            }
            if records.is_empty() && !dns_records.a.is_empty() {
                for ip in &dns_records.a {
                    records.push(DnsRecordPy {
                        record_type: "A".into(),
                        name: domain.to_string(),
                        data: ip.clone(),
                        ttl: 0,
                        class: "IN".into(),
                    });
                }
            }
        }
        Err(e) => {
            error_msg = Some(e.to_string());
            response_code = "SERVFAIL".into();
        }
    }

    if has_srv || has_ptr {
        let timeout_fut = tokio::time::timeout(timeout, async {
            use tokio::net::UdpSocket;

            let socket = UdpSocket::bind("0.0.0.0:0")
                .await
                .map_err(|e| NetworkError::new_err(format!("Failed to bind UDP socket: {}", e)))?;

            let target_addr = resolver_addr
                .map(|a| {
                    if a.contains(':') {
                        a.to_string()
                    } else {
                        format!("{}:53", a)
                    }
                })
                .unwrap_or_else(|| "8.8.8.8:53".to_string());

            socket.connect(&target_addr).await.map_err(|e| {
                NetworkError::new_err(format!("Failed to connect to resolver: {}", e))
            })?;

            let query_id: u16 = rand_simple();
            let mut packet = Vec::with_capacity(512);
            packet.extend_from_slice(&query_id.to_be_bytes());
            packet.extend_from_slice(&[0x01, 0x00]); // flags: standard query, recursion desired
            packet.extend_from_slice(&[0x00, 0x01]); // questions: 1
            packet.extend_from_slice(&[0x00, 0x00]); // answer RRs: 0
            packet.extend_from_slice(&[0x00, 0x00]); // authority RRs: 0
            packet.extend_from_slice(&[0x00, 0x00]); // additional RRs: 0

            let qtype = if has_srv {
                33u16
            } else {
                12u16 // PTR
            };

            for label in domain.split('.') {
                packet.push(label.len() as u8);
                packet.extend_from_slice(label.as_bytes());
            }
            packet.push(0);
            packet.extend_from_slice(&qtype.to_be_bytes());
            packet.extend_from_slice(&[0x00, 0x01]); // class IN

            socket
                .send(&packet)
                .await
                .map_err(|e| NetworkError::new_err(format!("Failed to send DNS query: {}", e)))?;

            let mut response_buf = vec![0u8; 1024];
            let n = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                socket.recv(&mut response_buf),
            )
            .await
            .map_err(|_| TimeoutError::new_err("DNS query timed out".to_string()))?
            .map_err(|e| NetworkError::new_err(format!("Failed to receive DNS response: {}", e)))?;

            response_buf.truncate(n);
            Ok::<Vec<u8>, PyErr>(response_buf)
        });

        match timeout_fut.await {
            Ok(Ok(response_buf)) => {
                truncated = (response_buf[2] & 0x02) != 0;
                authoritative = (response_buf[2] & 0x04) != 0;
                let rcode = response_buf[3] & 0x0F;
                response_code = match rcode {
                    0 => "NOERROR".to_string(),
                    1 => "FORMERR".to_string(),
                    2 => "SERVFAIL".to_string(),
                    3 => "NXDOMAIN".to_string(),
                    4 => "NOTIMP".to_string(),
                    5 => "REFUSED".to_string(),
                    _ => format!("RCODE_{}", rcode),
                };

                let an_count = u16::from_be_bytes([response_buf[6], response_buf[7]]) as usize;
                let qd_count = u16::from_be_bytes([response_buf[4], response_buf[5]]) as usize;

                let mut offset = 12;
                for _ in 0..qd_count {
                    while offset < response_buf.len() && response_buf[offset] != 0 {
                        let label_len = response_buf[offset] as usize;
                        if label_len & 0xC0 == 0xC0 {
                            offset += 2;
                            break;
                        }
                        offset += 1 + label_len;
                    }
                    if offset < response_buf.len() && response_buf[offset] == 0 {
                        offset += 1;
                    }
                    offset += 4; // qtype + qclass
                }

                let mut i = 0;
                while i < an_count && offset < response_buf.len() {
                    if response_buf[offset] & 0xC0 == 0xC0 {
                        offset += 2;
                    } else {
                        while offset < response_buf.len() && response_buf[offset] != 0 {
                            let label_len = response_buf[offset] as usize;
                            offset += 1 + label_len;
                        }
                        offset += 1;
                    }

                    if offset + 10 > response_buf.len() {
                        break;
                    }

                    let _at = u16::from_be_bytes([response_buf[offset], response_buf[offset + 1]]);
                    let rdlength =
                        u16::from_be_bytes([response_buf[offset + 8], response_buf[offset + 9]])
                            as usize;
                    offset += 10;

                    if offset + rdlength > response_buf.len() {
                        break;
                    }

                    let rdata = &response_buf[offset..offset + rdlength];
                    let type_name = if has_srv { "SRV" } else { "PTR" };

                    let data_str = if has_srv && rdlength >= 6 && type_name == "SRV" {
                        let priority = u16::from_be_bytes([rdata[0], rdata[1]]);
                        let weight = u16::from_be_bytes([rdata[2], rdata[3]]);
                        let port = u16::from_be_bytes([rdata[4], rdata[5]]);
                        format!("{} {} {} {}", priority, weight, port, "(target)")
                    } else if rdlength == 4 && type_name == "PTR" {
                        format!("{}.{}.{}.{}", rdata[0], rdata[1], rdata[2], rdata[3])
                    } else {
                        String::new()
                    };

                    if !data_str.is_empty() {
                        records.push(DnsRecordPy {
                            record_type: type_name.to_string(),
                            name: domain.to_string(),
                            data: data_str,
                            ttl: 0,
                            class: "IN".into(),
                        });
                    }

                    offset += rdlength;
                    i += 1;
                }
            }
            Ok(Err(e)) => {
                error_msg = Some(e.to_string());
            }
            Err(_) => {
                error_msg = Some("DNS query timed out".to_string());
                response_code = "SERVFAIL".into();
            }
        }
    }

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    let raw_artifact = None;

    Ok(DnsQueryResultPy {
        domain: domain.to_string(),
        records,
        response_code,
        authoritative,
        truncated,
        resolver_used: resolver_source.to_string(),
        timing: ConnectionTimingPy {
            dns_resolution_ms: Some(elapsed_ms),
            tcp_connect_ms: None,
            tls_handshake_ms: None,
            first_byte_ms: None,
            total_ms: elapsed_ms,
            connection_reused: false,
        },
        raw_artifact,
        error: error_msg,
    })
}

fn rand_simple() -> u16 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let s = RandomState::new();
    let mut hasher = s.build_hasher();
    hasher.write_u64(0);
    hasher.finish() as u16
}

// ============================================================================
// TLS Probe Functions
// ============================================================================

/// Perform a TLS inspection probe on a host.
///
/// Args:
///     host: Hostname to probe (e.g. "example.com").
///     port: TLS port (default: 443).
///     sni: Server Name Indication override. Uses host if None.
///     timeout_ms: Probe timeout in milliseconds (default: 10000).
///     verify_certificate: Whether to verify the certificate chain (default: true).
///
/// Returns:
///     TlsProbeResultPy with TLS configuration and certificate details.
///
/// Raises:
///     NetworkError: If the TLS connection fails.
#[pyfunction]
#[pyo3(signature = (host, port=443, sni=None, timeout_ms=10000, verify_certificate=true))]
pub fn tls_probe(
    host: &str,
    port: u16,
    sni: Option<&str>,
    timeout_ms: u64,
    verify_certificate: bool,
) -> PyResult<TlsProbeResultPy> {
    let host_owned = host.to_string();
    let sni_owned = sni.map(|s| s.to_string());
    let timeout = std::time::Duration::from_millis(timeout_ms);

    Python::with_gil(|py| {
        runtime_sync::block_on(py, async move {
            tls_probe_impl(
                &host_owned,
                port,
                sni_owned.as_deref(),
                timeout,
                verify_certificate,
            )
            .await
        })
    })
}

/// Perform an async TLS probe.
#[pyfunction]
#[pyo3(signature = (host, port=443, sni=None, timeout_ms=10000, verify_certificate=true))]
pub fn async_tls_probe(
    host: &str,
    port: u16,
    sni: Option<&str>,
    timeout_ms: u64,
    verify_certificate: bool,
) -> PyResult<runtime_async::PyFuture> {
    let host_owned = host.to_string();
    let sni_owned = sni.map(|s| s.to_string());
    let timeout = std::time::Duration::from_millis(timeout_ms);

    runtime_async::spawn_async(async move {
        tls_probe_impl(
            &host_owned,
            port,
            sni_owned.as_deref(),
            timeout,
            verify_certificate,
        )
        .await
    })
}

async fn tls_probe_impl(
    host: &str,
    port: u16,
    sni: Option<&str>,
    timeout: std::time::Duration,
    verify_certificate: bool,
) -> PyResult<TlsProbeResultPy> {
    let start = std::time::Instant::now();
    let effective_sni = sni.unwrap_or(host);
    let error_prefix = format!("TLS {}:{}", host, port);

    match tokio::time::timeout(timeout, async {
        eggsec::recon::ssl::analyze_ssl(host, port)
            .await
            .map_pyerr()
    })
    .await
    {
        Ok(Ok(analysis)) => {
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

            let certificate = analysis.certificate.map(|c| {
                let chain: Vec<CertificateChainEntryPy> = c
                    .certificate_chain
                    .into_iter()
                    .map(|e| CertificateChainEntryPy {
                        subject: e.subject,
                        issuer: e.issuer,
                        valid_from: e.valid_from,
                        valid_until: e.valid_until,
                    })
                    .collect();

                CertificateInfoPy {
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
                    subject_alternative_names: c.subject_alternative_names,
                    chain,
                }
            });

            let tls_version = analysis.supported_versions.first().cloned();

            let cipher_suite = analysis.supported_cipher_suites.first().cloned();

            let hostname_verified = effective_sni == host;
            let certificate_verified = analysis.has_ssl
                && !analysis
                    .issues
                    .iter()
                    .any(|i| i.severity == "high" && i.code.starts_with("CERT_"));

            let mut issues: Vec<TlsIssuePy> = analysis
                .issues
                .into_iter()
                .map(|i| TlsIssuePy {
                    severity: i.severity,
                    code: i.code,
                    description: i.description,
                })
                .collect();

            let sni_mismatch = effective_sni != host;
            if sni_mismatch {
                issues.push(TlsIssuePy {
                    severity: "info".into(),
                    code: "SNI_MISMATCH".into(),
                    description: format!("SNI '{}' differs from host '{}'", effective_sni, host),
                });
            }

            if !verify_certificate {
                issues.push(TlsIssuePy {
                    severity: "info".into(),
                    code: "CERT_VERIFY_DISABLED".into(),
                    description: "Certificate verification was disabled by caller".into(),
                });
            }

            Ok(TlsProbeResultPy {
                host: host.to_string(),
                port,
                has_tls: analysis.has_ssl,
                tls_version,
                cipher_suite,
                alpn_negotiated: None,
                certificate,
                hostname_verified,
                certificate_verified,
                supported_versions: analysis.supported_versions,
                supported_ciphers: analysis.supported_cipher_suites,
                issues,
                timing: ConnectionTimingPy {
                    dns_resolution_ms: None,
                    tcp_connect_ms: None,
                    tls_handshake_ms: Some(elapsed_ms),
                    first_byte_ms: None,
                    total_ms: elapsed_ms,
                    connection_reused: false,
                },
                error: None,
            })
        }
        Ok(Err(e)) => Ok(TlsProbeResultPy {
            host: host.to_string(),
            port,
            has_tls: false,
            tls_version: None,
            cipher_suite: None,
            alpn_negotiated: None,
            certificate: None,
            hostname_verified: false,
            certificate_verified: false,
            supported_versions: vec![],
            supported_ciphers: vec![],
            issues: vec![TlsIssuePy {
                severity: "high".into(),
                code: "TLS_ERROR".into(),
                description: e.to_string(),
            }],
            timing: ConnectionTimingPy {
                dns_resolution_ms: None,
                tcp_connect_ms: None,
                tls_handshake_ms: None,
                first_byte_ms: None,
                total_ms: start.elapsed().as_secs_f64() * 1000.0,
                connection_reused: false,
            },
            error: Some(format!("{}: {}", error_prefix, e)),
        }),
        Err(_) => Ok(TlsProbeResultPy {
            host: host.to_string(),
            port,
            has_tls: false,
            tls_version: None,
            cipher_suite: None,
            alpn_negotiated: None,
            certificate: None,
            hostname_verified: false,
            certificate_verified: false,
            supported_versions: vec![],
            supported_ciphers: vec![],
            issues: vec![TlsIssuePy {
                severity: "high".into(),
                code: "TIMEOUT".into(),
                description: format!("TLS probe timed out after {}ms", timeout.as_millis()),
            }],
            timing: ConnectionTimingPy {
                dns_resolution_ms: None,
                tcp_connect_ms: None,
                tls_handshake_ms: None,
                first_byte_ms: None,
                total_ms: start.elapsed().as_secs_f64() * 1000.0,
                connection_reused: false,
            },
            error: Some(format!("{}: connection timed out", error_prefix)),
        }),
    }
}

// ============================================================================
// HTTP Probe Functions
// ============================================================================

/// Perform an HTTP probe on a URL.
///
/// Args:
///     url: Full URL to probe (e.g. "https://example.com").
///     method: HTTP method (default: "GET").
///     timeout_ms: Request timeout in milliseconds (default: 10000).
///     follow_redirects: Whether to follow redirects (default: true).
///
/// Returns:
///     HttpProbeResultPy with response details.
///
/// Raises:
///     NetworkError: If the HTTP request fails.
#[pyfunction]
#[pyo3(signature = (url, method="GET", timeout_ms=10000, follow_redirects=true))]
pub fn http_probe(
    url: &str,
    method: &str,
    timeout_ms: u64,
    follow_redirects: bool,
) -> PyResult<HttpProbeResultPy> {
    let url_owned = url.to_string();
    let method_owned = method.to_uppercase();
    let timeout = std::time::Duration::from_millis(timeout_ms);

    Python::with_gil(|py| {
        runtime_sync::block_on(py, async move {
            http_probe_impl(&url_owned, &method_owned, timeout, follow_redirects, None).await
        })
    })
}

/// Perform an async HTTP probe.
#[pyfunction]
#[pyo3(signature = (url, method="GET", timeout_ms=10000, follow_redirects=true))]
pub fn async_http_probe(
    url: &str,
    method: &str,
    timeout_ms: u64,
    follow_redirects: bool,
) -> PyResult<runtime_async::PyFuture> {
    let url_owned = url.to_string();
    let method_owned = method.to_uppercase();
    let timeout = std::time::Duration::from_millis(timeout_ms);

    runtime_async::spawn_async(async move {
        http_probe_impl(&url_owned, &method_owned, timeout, follow_redirects, None).await
    })
}

async fn http_probe_impl(
    url: &str,
    method: &str,
    timeout: std::time::Duration,
    follow_redirects: bool,
    _extra_headers: Option<&[(String, String)]>,
) -> PyResult<HttpProbeResultPy> {
    let start = std::time::Instant::now();

    let mut builder = reqwest::Client::builder()
        .timeout(timeout)
        .danger_accept_invalid_certs(true);

    if !follow_redirects {
        builder = builder.redirect(reqwest::redirect::Policy::none());
    }

    let client = builder
        .build()
        .map_err(|e| NetworkError::new_err(format!("Failed to create HTTP client: {}", e)))?;

    let http_method = reqwest::Method::from_bytes(method.as_bytes())
        .map_err(|e| NetworkError::new_err(format!("Invalid HTTP method '{}': {}", method, e)))?;

    let mut request_builder = client.request(http_method, url);

    request_builder = request_builder.header(reqwest::header::USER_AGENT, "eggsec-probe/1.0");

    let response = tokio::time::timeout(timeout, request_builder.send())
        .await
        .map_err(|_| {
            TimeoutError::new_err(format!(
                "HTTP request to {} timed out after {}ms",
                url,
                timeout.as_millis()
            ))
        })?
        .map_err(|e| NetworkError::new_err(format!("HTTP request failed: {}", e)))?;

    let status_code = response.status().as_u16();
    let reason = response.status().canonical_reason().map(|s| s.to_string());

    let headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(k, v)| {
            (
                k.as_str().to_string(),
                String::from_utf8_lossy(v.as_bytes()).to_string(),
            )
        })
        .collect();

    let content_length = response.content_length();

    let final_url = response.url().clone().to_string();

    let body_bytes = response
        .bytes()
        .await
        .map_err(|e| NetworkError::new_err(format!("Failed to read response body: {}", e)))?;

    let body_text = String::from_utf8(body_bytes.to_vec()).ok();

    let redirect_count = 0u32;

    let tls_metadata = None;

    let timing = ConnectionTimingPy {
        dns_resolution_ms: None,
        tcp_connect_ms: None,
        tls_handshake_ms: None,
        first_byte_ms: None,
        total_ms: start.elapsed().as_secs_f64() * 1000.0,
        connection_reused: false,
    };

    Ok(HttpProbeResultPy {
        url: url.to_string(),
        final_url: Some(final_url),
        status_code,
        reason,
        headers,
        body_bytes: body_bytes.to_vec(),
        body_text,
        content_length,
        redirect_count,
        timing,
        tls_metadata,
        error: None,
    })
}
