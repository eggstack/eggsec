use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use crate::error::EggsecResultExt;
use crate::runtime_sync;

/// DNS record set for a domain.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecordSet {
    #[pyo3(get)]
    pub domain: String,
    pub(crate) a_records: Vec<String>,
    pub(crate) aaaa_records: Vec<String>,
    pub(crate) cname_records: Vec<String>,
    pub(crate) mx_records: Vec<MxRecord>,
    pub(crate) txt_records: Vec<String>,
    pub(crate) ns_records: Vec<String>,
    pub(crate) soa_record: Option<SoaRecord>,
    pub(crate) caa_records: Vec<String>,
}

#[pymethods]
impl DnsRecordSet {
    #[getter]
    fn a(&self) -> Vec<String> {
        self.a_records.clone()
    }

    #[getter]
    fn aaaa(&self) -> Vec<String> {
        self.aaaa_records.clone()
    }

    #[getter]
    fn cname(&self) -> Vec<String> {
        self.cname_records.clone()
    }

    #[getter]
    fn mx(&self) -> Vec<MxRecord> {
        self.mx_records.clone()
    }

    #[getter]
    fn txt(&self) -> Vec<String> {
        self.txt_records.clone()
    }

    #[getter]
    fn ns(&self) -> Vec<String> {
        self.ns_records.clone()
    }

    #[getter]
    fn soa(&self) -> Option<SoaRecord> {
        self.soa_record.clone()
    }

    #[getter]
    fn caa(&self) -> Vec<String> {
        self.caa_records.clone()
    }

    /// Convert to a Python dictionary.
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("domain", &self.domain)?;
        dict.set_item("a", &self.a_records)?;
        dict.set_item("aaaa", &self.aaaa_records)?;
        dict.set_item("cname", &self.cname_records)?;

        let mx_list = PyList::empty_bound(py);
        for mx in &self.mx_records {
            let mx_dict = PyDict::new_bound(py);
            mx_dict.set_item("preference", mx.preference)?;
            mx_dict.set_item("exchange", &mx.exchange)?;
            mx_list.append(mx_dict)?;
        }
        dict.set_item("mx", mx_list)?;
        dict.set_item("txt", &self.txt_records)?;
        dict.set_item("ns", &self.ns_records)?;

        if let Some(ref soa) = self.soa_record {
            let soa_dict = PyDict::new_bound(py);
            soa_dict.set_item("mname", &soa.mname)?;
            soa_dict.set_item("rname", &soa.rname)?;
            soa_dict.set_item("serial", soa.serial)?;
            dict.set_item("soa", soa_dict)?;
        } else {
            dict.set_item("soa", py.None())?;
        }

        dict.set_item("caa", &self.caa_records)?;
        Ok(dict.into())
    }

    /// Convert to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "DnsRecordSet(domain={}, a={}, aaaa={}, mx={}, txt={}, ns={})",
            self.domain,
            self.a_records.len(),
            self.aaaa_records.len(),
            self.mx_records.len(),
            self.txt_records.len(),
            self.ns_records.len()
        )
    }
}

/// An MX record.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MxRecord {
    #[pyo3(get)]
    pub preference: u16,
    #[pyo3(get)]
    pub exchange: String,
}

#[pymethods]
impl MxRecord {
    fn __repr__(&self) -> String {
        format!(
            "MxRecord(preference={}, exchange={})",
            self.preference, self.exchange
        )
    }
}

/// A SOA record.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoaRecord {
    #[pyo3(get)]
    pub mname: String,
    #[pyo3(get)]
    pub rname: String,
    #[pyo3(get)]
    pub serial: u32,
    #[pyo3(get)]
    pub refresh: i32,
    #[pyo3(get)]
    pub retry: i32,
    #[pyo3(get)]
    pub expire: i32,
    #[pyo3(get)]
    pub minimum: u32,
}

#[pymethods]
impl SoaRecord {
    fn __repr__(&self) -> String {
        format!("SoaRecord(mname={})", self.mname)
    }
}

/// TLS certificate information.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsCertificateInfo {
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
    pub(crate) sans: Vec<String>,
}

#[pymethods]
impl TlsCertificateInfo {
    #[getter]
    fn subject_alternative_names(&self) -> Vec<String> {
        self.sans.clone()
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
        dict.set_item("subject_alternative_names", &self.sans)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "TlsCertificateInfo(subject={}, issuer={})",
            self.subject, self.issuer
        )
    }
}

/// TLS inspection result.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsInspectionResult {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub has_ssl: bool,
    pub(crate) certificate: Option<TlsCertificateInfo>,
    pub(crate) supported_versions: Vec<String>,
    pub(crate) supported_cipher_suites: Vec<String>,
    pub(crate) issues: Vec<SslIssue>,
}

#[pymethods]
impl TlsInspectionResult {
    #[getter]
    fn certificate(&self) -> Option<TlsCertificateInfo> {
        self.certificate.clone()
    }

    #[getter]
    fn supported_versions(&self) -> Vec<String> {
        self.supported_versions.clone()
    }

    #[getter]
    fn supported_cipher_suites(&self) -> Vec<String> {
        self.supported_cipher_suites.clone()
    }

    #[getter]
    fn issues(&self) -> Vec<SslIssue> {
        self.issues.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("has_ssl", self.has_ssl)?;
        if let Some(ref cert) = self.certificate {
            dict.set_item("certificate", cert.to_dict(py)?)?;
        } else {
            dict.set_item("certificate", py.None())?;
        }
        dict.set_item("supported_versions", &self.supported_versions)?;
        dict.set_item("supported_cipher_suites", &self.supported_cipher_suites)?;

        let issues_list = PyList::empty_bound(py);
        for issue in &self.issues {
            let issue_dict = PyDict::new_bound(py);
            issue_dict.set_item("severity", &issue.severity)?;
            issue_dict.set_item("code", &issue.code)?;
            issue_dict.set_item("description", &issue.description)?;
            issues_list.append(issue_dict)?;
        }
        dict.set_item("issues", issues_list)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "TlsInspectionResult(target={}, has_ssl={})",
            self.target, self.has_ssl
        )
    }
}

/// An SSL/TLS issue.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslIssue {
    #[pyo3(get)]
    pub severity: String,
    #[pyo3(get)]
    pub code: String,
    #[pyo3(get)]
    pub description: String,
}

#[pymethods]
impl SslIssue {
    fn __repr__(&self) -> String {
        format!(
            "SslIssue(severity={}, code={})",
            self.severity, self.code
        )
    }
}

/// Technology stack detected on a target.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechStack {
    pub(crate) servers: Vec<String>,
    pub(crate) frameworks: Vec<String>,
    pub(crate) languages: Vec<String>,
    pub(crate) databases: Vec<String>,
    pub(crate) cdns: Vec<String>,
    pub(crate) cms: Vec<String>,
    pub(crate) javascript: Vec<String>,
    pub(crate) other: Vec<String>,
}

#[pymethods]
impl TechStack {
    #[getter]
    fn servers(&self) -> Vec<String> {
        self.servers.clone()
    }

    #[getter]
    fn frameworks(&self) -> Vec<String> {
        self.frameworks.clone()
    }

    #[getter]
    fn languages(&self) -> Vec<String> {
        self.languages.clone()
    }

    #[getter]
    fn databases(&self) -> Vec<String> {
        self.databases.clone()
    }

    #[getter]
    fn cdns(&self) -> Vec<String> {
        self.cdns.clone()
    }

    #[getter]
    fn cms(&self) -> Vec<String> {
        self.cms.clone()
    }

    #[getter]
    fn javascript(&self) -> Vec<String> {
        self.javascript.clone()
    }

    #[getter]
    fn other(&self) -> Vec<String> {
        self.other.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("servers", &self.servers)?;
        dict.set_item("frameworks", &self.frameworks)?;
        dict.set_item("languages", &self.languages)?;
        dict.set_item("databases", &self.databases)?;
        dict.set_item("cdns", &self.cdns)?;
        dict.set_item("cms", &self.cms)?;
        dict.set_item("javascript", &self.javascript)?;
        dict.set_item("other", &self.other)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "TechStack(servers={}, frameworks={}, languages={})",
            self.servers.len(),
            self.frameworks.len(),
            self.languages.len()
        )
    }
}

/// Technology detection result.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechDetectionResult {
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub status_code: u16,
    pub(crate) headers: std::collections::HashMap<String, String>,
    #[pyo3(get)]
    pub tech_stack: TechStack,
}

#[pymethods]
impl TechDetectionResult {
    #[getter]
    fn headers(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        for (k, v) in &self.headers {
            dict.set_item(k, v)?;
        }
        Ok(dict.into())
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("url", &self.url)?;
        dict.set_item("status_code", self.status_code)?;
        let headers_dict = PyDict::new_bound(py);
        for (k, v) in &self.headers {
            headers_dict.set_item(k, v)?;
        }
        dict.set_item("headers", headers_dict)?;
        dict.set_item("tech_stack", self.tech_stack.to_dict(py)?)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "TechDetectionResult(url={}, status={})",
            self.url, self.status_code
        )
    }
}

/// Perform DNS resolution and record enumeration for a domain.
///
/// Args:
///     domain: Domain name to look up (e.g. "example.com").
///
/// Returns:
///     DnsRecordSet: DNS records for the domain.
///
/// Raises:
///     NetworkError: If DNS resolution fails.
#[pyfunction]
pub fn recon_dns(domain: &str) -> PyResult<DnsRecordSet> {
    let domain_owned = domain.to_string();
    Python::with_gil(|py| {
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
                .map(|m| MxRecord {
                    preference: m.preference,
                    exchange: m.exchange,
                })
                .collect(),
            txt_records: result.txt,
            ns_records: result.ns,
            soa_record: result.soa.map(|s| SoaRecord {
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

/// Perform async DNS resolution and record enumeration.
#[pyfunction]
pub fn async_recon_dns(domain: &str) -> PyResult<crate::runtime_async::PyFuture> {
    let domain_owned = domain.to_string();

    crate::runtime_async::spawn_async(async move {
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
                .map(|m| MxRecord {
                    preference: m.preference,
                    exchange: m.exchange,
                })
                .collect(),
            txt_records: result.txt,
            ns_records: result.ns,
            soa_record: result.soa.map(|s| SoaRecord {
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
///     NetworkError: If TLS connection fails.
#[pyfunction]
#[pyo3(signature = (host, *, port=443))]
pub fn inspect_tls(host: &str, port: u16) -> PyResult<TlsInspectionResult> {
    let host_owned = host.to_string();
    Python::with_gil(|py| {
        let result = runtime_sync::block_on(py, async move {
            eggsec::recon::ssl::analyze_ssl(&host_owned, port)
                .await
                .map_pyerr()
        })?;

        Ok(TlsInspectionResult {
            target: result.target,
            has_ssl: result.has_ssl,
            certificate: result.certificate.map(|c| TlsCertificateInfo {
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
                .map(|i| SslIssue {
                    severity: i.severity,
                    code: i.code,
                    description: i.description,
                })
                .collect(),
        })
    })
}

/// Perform async TLS inspection.
#[pyfunction]
#[pyo3(signature = (host, *, port=443))]
pub fn async_inspect_tls(host: &str, port: u16) -> PyResult<crate::runtime_async::PyFuture> {
    let host_owned = host.to_string();

    crate::runtime_async::spawn_async(async move {
        let result = eggsec::recon::ssl::analyze_ssl(&host_owned, port)
            .await
            .map_pyerr()?;

        Ok(TlsInspectionResult {
            target: result.target,
            has_ssl: result.has_ssl,
            certificate: result.certificate.map(|c| TlsCertificateInfo {
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
                .map(|i| SslIssue {
                    severity: i.severity,
                    code: i.code,
                    description: i.description,
                })
                .collect(),
        })
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
///     NetworkError: If the HTTP request fails.
#[pyfunction]
pub fn detect_technology(url: &str) -> PyResult<TechDetectionResult> {
    let url_owned = url.to_string();
    Python::with_gil(|py| {
        let result = runtime_sync::block_on(py, async move {
            eggsec::recon::techdetect::detect_tech_stack(&url_owned)
                .await
                .map_pyerr()
        })?;

        Ok(TechDetectionResult {
            url: result.url,
            status_code: result.status_code,
            headers: result.headers.into_iter().collect(),
            tech_stack: TechStack {
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

/// Perform async technology detection.
#[pyfunction]
pub fn async_detect_technology(url: &str) -> PyResult<crate::runtime_async::PyFuture> {
    let url_owned = url.to_string();

    crate::runtime_async::spawn_async(async move {
        let result = eggsec::recon::techdetect::detect_tech_stack(&url_owned)
            .await
            .map_pyerr()?;

        Ok(TechDetectionResult {
            url: result.url,
            status_code: result.status_code,
            headers: result.headers.into_iter().collect(),
            tech_stack: TechStack {
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
