use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

use crate::error::{NetworkError, TimeoutError};
use crate::runtime_async;
use crate::runtime_sync::block_on;

// ---------------------------------------------------------------------------
// RedactConfigPy
// ---------------------------------------------------------------------------

/// Configuration for redacting sensitive data from HTTP responses.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactConfigPy {
    pub(crate) redact_headers: Vec<String>,
    pub(crate) redact_query_params: Vec<String>,
    pub(crate) redact_body_fields: Vec<String>,
}

#[pymethods]
impl RedactConfigPy {
    #[new]
    #[pyo3(signature = (redact_headers=None, redact_query_params=None, redact_body_fields=None))]
    fn new(
        redact_headers: Option<Vec<String>>,
        redact_query_params: Option<Vec<String>>,
        redact_body_fields: Option<Vec<String>>,
    ) -> Self {
        Self {
            redact_headers: redact_headers.unwrap_or_else(|| {
                vec![
                    "Authorization".to_string(),
                    "Cookie".to_string(),
                    "Proxy-Authorization".to_string(),
                    "X-API-Key".to_string(),
                ]
            }),
            redact_query_params: redact_query_params.unwrap_or_default(),
            redact_body_fields: redact_body_fields.unwrap_or_default(),
        }
    }

    #[getter]
    fn redact_headers(&self) -> Vec<String> {
        self.redact_headers.clone()
    }

    #[getter]
    fn redact_query_params(&self) -> Vec<String> {
        self.redact_query_params.clone()
    }

    #[getter]
    fn redact_body_fields(&self) -> Vec<String> {
        self.redact_body_fields.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("redact_headers", &self.redact_headers)?;
        dict.set_item("redact_query_params", &self.redact_query_params)?;
        dict.set_item("redact_body_fields", &self.redact_body_fields)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "RedactConfigPy(headers={}, query_params={}, body_fields={})",
            self.redact_headers.len(),
            self.redact_query_params.len(),
            self.redact_body_fields.len()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "redact_headers=[{}], redact_query_params=[{}], redact_body_fields=[{}]",
            self.redact_headers.join(", "),
            self.redact_query_params.join(", "),
            self.redact_body_fields.join(", ")
        )
    }
}

// ---------------------------------------------------------------------------
// HttpRequestPy
// ---------------------------------------------------------------------------

/// HTTP request configuration.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequestPy {
    #[pyo3(get)]
    pub method: String,
    #[pyo3(get)]
    pub url: String,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) query_params: Vec<(String, String)>,
    pub(crate) body_bytes: Option<Vec<u8>>,
    #[pyo3(get)]
    pub body_text: Option<String>,
    #[pyo3(get)]
    pub body_json: Option<String>,
    pub(crate) body_form: Option<Vec<(String, String)>>,
    pub(crate) cookies: Vec<(String, String)>,
    #[pyo3(get)]
    pub follow_redirects: bool,
    #[pyo3(get)]
    pub max_redirects: u32,
    #[pyo3(get)]
    pub verify_tls: bool,
    #[pyo3(get)]
    pub timeout_ms: u64,
    #[pyo3(get)]
    pub connect_timeout_ms: u64,
    #[pyo3(get)]
    pub user_agent: Option<String>,
    #[pyo3(get)]
    pub proxy_url: Option<String>,
    #[pyo3(get)]
    pub response_size_limit: Option<usize>,
}

#[pymethods]
impl HttpRequestPy {
    #[new]
    #[pyo3(signature = (method, url, *, headers=None, query_params=None, body_bytes=None, body_text=None, body_json=None, body_form=None, cookies=None, follow_redirects=true, max_redirects=10, verify_tls=true, timeout_ms=30000, connect_timeout_ms=5000, user_agent=None, proxy_url=None, response_size_limit=None))]
    fn new(
        method: String,
        url: String,
        headers: Option<Vec<(String, String)>>,
        query_params: Option<Vec<(String, String)>>,
        body_bytes: Option<Vec<u8>>,
        body_text: Option<String>,
        body_json: Option<String>,
        body_form: Option<Vec<(String, String)>>,
        cookies: Option<Vec<(String, String)>>,
        follow_redirects: bool,
        max_redirects: u32,
        verify_tls: bool,
        timeout_ms: u64,
        connect_timeout_ms: u64,
        user_agent: Option<String>,
        proxy_url: Option<String>,
        response_size_limit: Option<usize>,
    ) -> Self {
        Self {
            method,
            url,
            headers: headers.unwrap_or_default(),
            query_params: query_params.unwrap_or_default(),
            body_bytes,
            body_text,
            body_json,
            body_form,
            cookies: cookies.unwrap_or_default(),
            follow_redirects,
            max_redirects,
            verify_tls,
            timeout_ms,
            connect_timeout_ms,
            user_agent,
            proxy_url,
            response_size_limit,
        }
    }

    #[getter]
    fn headers(&self) -> Vec<(String, String)> {
        self.headers.clone()
    }

    #[getter]
    fn query_params(&self) -> Vec<(String, String)> {
        self.query_params.clone()
    }

    #[getter]
    fn body_bytes(&self) -> Option<Vec<u8>> {
        self.body_bytes.clone()
    }

    #[getter]
    fn body_form(&self) -> Option<Vec<(String, String)>> {
        self.body_form.clone()
    }

    #[getter]
    fn cookies(&self) -> Vec<(String, String)> {
        self.cookies.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("method", &self.method)?;
        dict.set_item("url", &self.url)?;
        dict.set_item("headers", &self.headers)?;
        dict.set_item("query_params", &self.query_params)?;
        dict.set_item("body_bytes", &self.body_bytes)?;
        dict.set_item("body_text", &self.body_text)?;
        dict.set_item("body_json", &self.body_json)?;
        dict.set_item("body_form", &self.body_form)?;
        dict.set_item("cookies", &self.cookies)?;
        dict.set_item("follow_redirects", self.follow_redirects)?;
        dict.set_item("max_redirects", self.max_redirects)?;
        dict.set_item("verify_tls", self.verify_tls)?;
        dict.set_item("timeout_ms", self.timeout_ms)?;
        dict.set_item("connect_timeout_ms", self.connect_timeout_ms)?;
        dict.set_item("user_agent", &self.user_agent)?;
        dict.set_item("proxy_url", &self.proxy_url)?;
        dict.set_item("response_size_limit", self.response_size_limit)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("HttpRequestPy(method={}, url={})", self.method, self.url)
    }

    fn __str__(&self) -> String {
        format!("{} {}", self.method, self.url)
    }
}

// ---------------------------------------------------------------------------
// HttpHeadersPy
// ---------------------------------------------------------------------------

/// Duplicate-preserving HTTP header container.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpHeadersPy {
    pub(crate) entries: Vec<(String, String)>,
}

#[pymethods]
impl HttpHeadersPy {
    #[new]
    #[pyo3(signature = (entries=None))]
    fn new(entries: Option<Vec<(String, String)>>) -> Self {
        Self {
            entries: entries.unwrap_or_default(),
        }
    }

    /// Get the first value for the given header name (case-insensitive).
    fn get(&self, name: &str) -> Option<String> {
        let lower = name.to_lowercase();
        self.entries
            .iter()
            .find(|(k, _)| k.to_lowercase() == lower)
            .map(|(_, v)| v.clone())
    }

    /// Get all values for the given header name (case-insensitive).
    fn get_all(&self, name: &str) -> Vec<String> {
        let lower = name.to_lowercase();
        self.entries
            .iter()
            .filter(|(k, _)| k.to_lowercase() == lower)
            .map(|(_, v)| v.clone())
            .collect()
    }

    /// Check if a header name exists (case-insensitive).
    fn contains(&self, name: &str) -> bool {
        let lower = name.to_lowercase();
        self.entries.iter().any(|(k, _)| k.to_lowercase() == lower)
    }

    /// Return unique header names in order of first appearance.
    fn names(&self) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();
        for (k, _) in &self.entries {
            let lower = k.to_lowercase();
            if seen.insert(lower) {
                result.push(k.clone());
            }
        }
        result
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn __len__(&self) -> usize {
        self.entries.len()
    }

    fn __bool__(&self) -> bool {
        !self.entries.is_empty()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        let entries_list = PyList::empty_bound(py);
        for (k, v) in &self.entries {
            let pair = PyDict::new_bound(py);
            pair.set_item("name", k)?;
            pair.set_item("value", v)?;
            entries_list.append(pair)?;
        }
        dict.set_item("entries", entries_list)?;
        dict.set_item("len", self.entries.len())?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "HttpHeadersPy(entries={}, names=[{}])",
            self.entries.len(),
            self.names().join(", ")
        )
    }

    fn __str__(&self) -> String {
        self.entries
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

// ---------------------------------------------------------------------------
// HttpCookiePy
// ---------------------------------------------------------------------------

/// An HTTP cookie.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpCookiePy {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub value: String,
    #[pyo3(get)]
    pub domain: Option<String>,
    #[pyo3(get)]
    pub path: Option<String>,
    #[pyo3(get)]
    pub expires: Option<String>,
    #[pyo3(get)]
    pub secure: bool,
    #[pyo3(get)]
    pub http_only: bool,
}

#[pymethods]
impl HttpCookiePy {
    #[new]
    #[pyo3(signature = (name, value, *, domain=None, path=None, expires=None, secure=false, http_only=false))]
    fn new(
        name: String,
        value: String,
        domain: Option<String>,
        path: Option<String>,
        expires: Option<String>,
        secure: bool,
        http_only: bool,
    ) -> Self {
        Self {
            name,
            value,
            domain,
            path,
            expires,
            secure,
            http_only,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("name", &self.name)?;
        dict.set_item("value", &self.value)?;
        dict.set_item("domain", &self.domain)?;
        dict.set_item("path", &self.path)?;
        dict.set_item("expires", &self.expires)?;
        dict.set_item("secure", self.secure)?;
        dict.set_item("http_only", self.http_only)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "HttpCookiePy(name={}, domain={:?}, secure={})",
            self.name, self.domain, self.secure
        )
    }

    fn __str__(&self) -> String {
        format!("{}={}", self.name, self.value)
    }
}

// ---------------------------------------------------------------------------
// RedirectEntryPy
// ---------------------------------------------------------------------------

/// A single redirect entry in the redirect history.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedirectEntryPy {
    #[pyo3(get)]
    pub url: String,
    #[pyo3(get)]
    pub status_code: u16,
    pub(crate) headers: Vec<(String, String)>,
}

#[pymethods]
impl RedirectEntryPy {
    #[new]
    fn new(url: String, status_code: u16, headers: Vec<(String, String)>) -> Self {
        Self {
            url,
            status_code,
            headers,
        }
    }

    #[getter]
    fn headers(&self) -> Vec<(String, String)> {
        self.headers.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("url", &self.url)?;
        dict.set_item("status_code", self.status_code)?;
        dict.set_item("headers", &self.headers)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "RedirectEntryPy(url={}, status={})",
            self.url, self.status_code
        )
    }

    fn __str__(&self) -> String {
        format!("{} -> {}", self.status_code, self.url)
    }
}

// ---------------------------------------------------------------------------
// TlsMetadataPy
// ---------------------------------------------------------------------------

/// TLS connection metadata.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsMetadataPy {
    #[pyo3(get)]
    pub version: Option<String>,
    #[pyo3(get)]
    pub cipher: Option<String>,
    pub(crate) certificate_chain: Vec<String>,
}

#[pymethods]
impl TlsMetadataPy {
    #[new]
    #[pyo3(signature = (version=None, cipher=None, certificate_chain=None))]
    fn new(
        version: Option<String>,
        cipher: Option<String>,
        certificate_chain: Option<Vec<String>>,
    ) -> Self {
        Self {
            version,
            cipher,
            certificate_chain: certificate_chain.unwrap_or_default(),
        }
    }

    #[getter]
    fn certificate_chain(&self) -> Vec<String> {
        self.certificate_chain.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("version", &self.version)?;
        dict.set_item("cipher", &self.cipher)?;
        dict.set_item("certificate_chain", &self.certificate_chain)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "TlsMetadataPy(version={:?}, cipher={:?}, chain_len={})",
            self.version,
            self.cipher,
            self.certificate_chain.len()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "TLS {:?} cipher={:?} certs={}",
            self.version,
            self.cipher,
            self.certificate_chain.len()
        )
    }
}

// ---------------------------------------------------------------------------
// HttpTimingPy
// ---------------------------------------------------------------------------

/// Timing breakdown for an HTTP request lifecycle.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpTimingPy {
    #[pyo3(get)]
    pub dns_ms: Option<f64>,
    #[pyo3(get)]
    pub connect_ms: Option<f64>,
    #[pyo3(get)]
    pub tls_handshake_ms: Option<f64>,
    #[pyo3(get)]
    pub first_byte_ms: Option<f64>,
    #[pyo3(get)]
    pub total_ms: f64,
    #[pyo3(get)]
    pub size_download: u64,
    #[pyo3(get)]
    pub speed_download: f64,
}

#[pymethods]
impl HttpTimingPy {
    #[new]
    #[pyo3(signature = (dns_ms=None, connect_ms=None, tls_handshake_ms=None, first_byte_ms=None, total_ms=0.0, size_download=0, speed_download=0.0))]
    fn new(
        dns_ms: Option<f64>,
        connect_ms: Option<f64>,
        tls_handshake_ms: Option<f64>,
        first_byte_ms: Option<f64>,
        total_ms: f64,
        size_download: u64,
        speed_download: f64,
    ) -> Self {
        Self {
            dns_ms,
            connect_ms,
            tls_handshake_ms,
            first_byte_ms,
            total_ms,
            size_download,
            speed_download,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("dns_ms", &self.dns_ms)?;
        dict.set_item("connect_ms", &self.connect_ms)?;
        dict.set_item("tls_handshake_ms", &self.tls_handshake_ms)?;
        dict.set_item("first_byte_ms", &self.first_byte_ms)?;
        dict.set_item("total_ms", self.total_ms)?;
        dict.set_item("size_download", self.size_download)?;
        dict.set_item("speed_download", self.speed_download)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "HttpTimingPy(total={:.1}ms, download={}B)",
            self.total_ms, self.size_download
        )
    }

    fn __str__(&self) -> String {
        let mut parts = vec![format!("total={:.1}ms", self.total_ms)];
        if let Some(dns) = self.dns_ms {
            parts.push(format!("dns={:.1}ms", dns));
        }
        if let Some(conn) = self.connect_ms {
            parts.push(format!("connect={:.1}ms", conn));
        }
        if let Some(tls) = self.tls_handshake_ms {
            parts.push(format!("tls={:.1}ms", tls));
        }
        if let Some(ttfb) = self.first_byte_ms {
            parts.push(format!("ttfb={:.1}ms", ttfb));
        }
        if self.size_download > 0 {
            parts.push(format!("download={}B", self.size_download));
        }
        parts.join(" ")
    }
}

// ---------------------------------------------------------------------------
// HttpResponsePy
// ---------------------------------------------------------------------------

/// Full HTTP response.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponsePy {
    #[pyo3(get)]
    pub status_code: u16,
    #[pyo3(get)]
    pub reason: Option<String>,
    #[pyo3(get)]
    pub body_text: Option<String>,
    #[pyo3(get)]
    pub content_length: Option<u64>,
    #[pyo3(get)]
    pub final_url: String,
    #[pyo3(get)]
    pub protocol_version: Option<String>,
    #[pyo3(get)]
    pub timing: HttpTimingPy,
    #[pyo3(get)]
    pub bytes_received: u64,
    pub(crate) headers: HttpHeadersPy,
    pub(crate) cookies: Vec<HttpCookiePy>,
    pub(crate) body_bytes: Vec<u8>,
    pub(crate) redirect_history: Vec<RedirectEntryPy>,
    pub(crate) tls_metadata: Option<TlsMetadataPy>,
    pub(crate) redact_config: Option<RedactConfigPy>,
}

#[pymethods]
impl HttpResponsePy {
    #[getter]
    fn headers(&self) -> HttpHeadersPy {
        self.headers.clone()
    }

    #[getter]
    fn cookies(&self) -> Vec<HttpCookiePy> {
        self.cookies.clone()
    }

    #[getter]
    fn body_bytes(&self) -> Vec<u8> {
        self.body_bytes.clone()
    }

    #[getter]
    fn redirect_history(&self) -> Vec<RedirectEntryPy> {
        self.redirect_history.clone()
    }

    #[getter]
    fn tls_metadata(&self) -> Option<TlsMetadataPy> {
        self.tls_metadata.clone()
    }

    /// Return headers with sensitive values masked according to the redact config.
    fn redacted_headers(&self, py: Python) -> PyResult<PyObject> {
        let redact_config = match &self.redact_config {
            Some(c) => c,
            None => {
                let entries_list = PyList::empty_bound(py);
                for (k, v) in &self.headers.entries {
                    let pair = PyDict::new_bound(py);
                    pair.set_item("0", k)?;
                    pair.set_item("1", v)?;
                    entries_list.append(pair)?;
                }
                return Ok(entries_list.into());
            }
        };

        let sensitive: std::collections::HashSet<String> = redact_config
            .redact_headers
            .iter()
            .map(|h| h.to_lowercase())
            .collect();

        let entries_list = PyList::empty_bound(py);
        for (k, v) in &self.headers.entries {
            let pair = PyDict::new_bound(py);
            pair.set_item("0", k)?;
            if sensitive.contains(&k.to_lowercase()) {
                let masked = if v.len() > 4 {
                    format!("{}****{}", &v[..2], &v[v.len() - 2..])
                } else {
                    "****".to_string()
                };
                pair.set_item("1", masked)?;
            } else {
                pair.set_item("1", v)?;
            }
            entries_list.append(pair)?;
        }
        Ok(entries_list.into())
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("status_code", self.status_code)?;
        dict.set_item("reason", &self.reason)?;
        dict.set_item("headers", self.headers.to_dict(py)?)?;

        let cookies_list = PyList::empty_bound(py);
        for cookie in &self.cookies {
            cookies_list.append(cookie.to_dict(py)?)?;
        }
        dict.set_item("cookies", cookies_list)?;

        dict.set_item("body_bytes", &self.body_bytes)?;
        dict.set_item("body_text", &self.body_text)?;
        dict.set_item("content_length", &self.content_length)?;
        dict.set_item("final_url", &self.final_url)?;

        let redirects_list = PyList::empty_bound(py);
        for entry in &self.redirect_history {
            redirects_list.append(entry.to_dict(py)?)?;
        }
        dict.set_item("redirect_history", redirects_list)?;

        dict.set_item("protocol_version", &self.protocol_version)?;
        dict.set_item("timing", self.timing.to_dict(py)?)?;

        if let Some(ref tls) = self.tls_metadata {
            dict.set_item("tls_metadata", tls.to_dict(py)?)?;
        } else {
            dict.set_item("tls_metadata", py.None())?;
        }

        dict.set_item("bytes_received", self.bytes_received)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "HttpResponsePy(status={}, url={}, bytes={})",
            self.status_code, self.final_url, self.bytes_received
        )
    }

    fn __str__(&self) -> String {
        format!(
            "HTTP {} {} ({} bytes, {:.1}ms)",
            self.status_code, self.final_url, self.bytes_received, self.timing.total_ms
        )
    }

    /// Return the body as an iterator of chunks.
    ///
    /// Useful for processing large response bodies without loading
    /// everything into memory at once. Each chunk is a bytes object.
    fn iter_body_chunks(&self, chunk_size: Option<usize>) -> Vec<Vec<u8>> {
        let chunk_size = chunk_size.unwrap_or(8192);
        self.body_bytes
            .chunks(chunk_size)
            .map(|c| c.to_vec())
            .collect()
    }

    /// Return body bytes up to the given limit, truncating if necessary.
    fn body_bytes_limited(&self, max_bytes: usize) -> (Vec<u8>, bool) {
        if self.body_bytes.len() <= max_bytes {
            (self.body_bytes.clone(), false)
        } else {
            let mut data = self.body_bytes[..max_bytes].to_vec();
            data.truncate(max_bytes);
            (data, true)
        }
    }
}

// ---------------------------------------------------------------------------
// HttpClientConfigPy
// ---------------------------------------------------------------------------

/// Configuration for an HTTP client.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpClientConfigPy {
    #[pyo3(get)]
    pub base_url: Option<String>,
    pub(crate) default_headers: Vec<(String, String)>,
    #[pyo3(get)]
    pub timeout_ms: u64,
    #[pyo3(get)]
    pub connect_timeout_ms: u64,
    #[pyo3(get)]
    pub max_redirects: u32,
    #[pyo3(get)]
    pub verify_tls: bool,
    #[pyo3(get)]
    pub proxy_url: Option<String>,
    #[pyo3(get)]
    pub user_agent: Option<String>,
    #[pyo3(get)]
    pub cookie_store: bool,
    #[pyo3(get)]
    pub pool_idle_timeout_ms: u64,
    #[pyo3(get)]
    pub pool_max_idle_per_host: usize,
}

#[pymethods]
impl HttpClientConfigPy {
    #[new]
    #[pyo3(signature = (*, base_url=None, default_headers=None, timeout_ms=30000, connect_timeout_ms=5000, max_redirects=10, verify_tls=true, proxy_url=None, user_agent=None, cookie_store=true, pool_idle_timeout_ms=90000, pool_max_idle_per_host=10))]
    fn new(
        base_url: Option<String>,
        default_headers: Option<Vec<(String, String)>>,
        timeout_ms: u64,
        connect_timeout_ms: u64,
        max_redirects: u32,
        verify_tls: bool,
        proxy_url: Option<String>,
        user_agent: Option<String>,
        cookie_store: bool,
        pool_idle_timeout_ms: u64,
        pool_max_idle_per_host: usize,
    ) -> Self {
        Self {
            base_url,
            default_headers: default_headers.unwrap_or_default(),
            timeout_ms,
            connect_timeout_ms,
            max_redirects,
            verify_tls,
            proxy_url,
            user_agent,
            cookie_store,
            pool_idle_timeout_ms,
            pool_max_idle_per_host,
        }
    }

    #[getter]
    fn default_headers(&self) -> Vec<(String, String)> {
        self.default_headers.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("base_url", &self.base_url)?;
        dict.set_item("default_headers", &self.default_headers)?;
        dict.set_item("timeout_ms", self.timeout_ms)?;
        dict.set_item("connect_timeout_ms", self.connect_timeout_ms)?;
        dict.set_item("max_redirects", self.max_redirects)?;
        dict.set_item("verify_tls", self.verify_tls)?;
        dict.set_item("proxy_url", &self.proxy_url)?;
        dict.set_item("user_agent", &self.user_agent)?;
        dict.set_item("cookie_store", self.cookie_store)?;
        dict.set_item("pool_idle_timeout_ms", self.pool_idle_timeout_ms)?;
        dict.set_item("pool_max_idle_per_host", self.pool_max_idle_per_host)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "HttpClientConfigPy(base_url={:?}, timeout={}ms, tls={})",
            self.base_url, self.timeout_ms, self.verify_tls
        )
    }

    fn __str__(&self) -> String {
        format!(
            "base_url={:?} timeout={}ms connect={}ms redirects={} tls={} cookies={}",
            self.base_url,
            self.timeout_ms,
            self.connect_timeout_ms,
            self.max_redirects,
            self.verify_tls,
            self.cookie_store
        )
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Build a reqwest::Client from the given configuration.
fn build_reqwest_client(config: &HttpClientConfigPy) -> PyResult<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(config.timeout_ms))
        .connect_timeout(std::time::Duration::from_millis(config.connect_timeout_ms))
        .pool_idle_timeout(std::time::Duration::from_millis(
            config.pool_idle_timeout_ms,
        ))
        .pool_max_idle_per_host(config.pool_max_idle_per_host)
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(config.cookie_store);

    if !config.verify_tls {
        builder = builder.danger_accept_invalid_certs(true);
    }

    if let Some(ref ua) = config.user_agent {
        builder = builder.user_agent(ua.as_str());
    }

    if let Some(ref proxy) = config.proxy_url {
        let proxy_url = reqwest::Url::parse(proxy)
            .map_err(|e| NetworkError::new_err(format!("Invalid proxy URL: {}", e)))?;
        let proxy = reqwest::Proxy::all(proxy_url)
            .map_err(|e| NetworkError::new_err(format!("Invalid proxy: {}", e)))?;
        builder = builder.proxy(proxy);
    }

    if !config.default_headers.is_empty() {
        let mut header_map = reqwest::header::HeaderMap::new();
        for (k, v) in &config.default_headers {
            let name = reqwest::header::HeaderName::from_bytes(k.as_bytes()).map_err(|e| {
                NetworkError::new_err(format!("Invalid header name '{}': {}", k, e))
            })?;
            let value = reqwest::header::HeaderValue::from_str(v).map_err(|e| {
                NetworkError::new_err(format!("Invalid header value for '{}': {}", k, e))
            })?;
            header_map.append(name, value);
        }
        builder = builder.default_headers(header_map);
    }

    builder
        .build()
        .map_err(|e| NetworkError::new_err(format!("Failed to build HTTP client: {}", e)))
}

/// Resolve a URL against an optional base URL.
fn resolve_url(base_url: Option<&str>, url: &str) -> PyResult<String> {
    if let Some(base) = base_url {
        let base_parsed = url::Url::parse(base).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid base URL: {}", e))
        })?;
        let resolved = base_parsed.join(url).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Failed to resolve URL: {}", e))
        })?;
        Ok(resolved.to_string())
    } else {
        Ok(url.to_string())
    }
}

/// Resolve the full URL from request config, respecting base_url.
fn request_url(req: &HttpRequestPy, base_url: Option<&str>) -> PyResult<String> {
    resolve_url(base_url, &req.url)
}

/// Execute an HTTP request asynchronously, building the reqwest request from config.
async fn execute_request_async(
    client: &reqwest::Client,
    req: &HttpRequestPy,
    base_url: Option<&str>,
) -> PyResult<HttpResponsePy> {
    let url = request_url(req, base_url)?;
    let start = std::time::Instant::now();

    let http_method = reqwest::Method::from_bytes(req.method.as_bytes()).map_err(|e| {
        NetworkError::new_err(format!("Invalid HTTP method '{}': {}", req.method, e))
    })?;

    let mut request_builder = client.request(http_method, &url);

    for (k, v) in &req.query_params {
        request_builder = request_builder.query(&(k.as_str(), v.as_str()));
    }

    for (k, v) in &req.headers {
        request_builder = request_builder.header(k.as_str(), v.as_str());
    }

    for (k, v) in &req.cookies {
        request_builder = request_builder.header("cookie", format!("{}={}", k, v));
    }

    if let Some(ref ua) = req.user_agent {
        request_builder = request_builder.header(reqwest::header::USER_AGENT, ua.as_str());
    }

    if let Some(ref body) = req.body_text {
        request_builder = request_builder.body(body.clone());
    } else if let Some(ref body) = req.body_json {
        request_builder = request_builder
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.clone());
    } else if let Some(ref body) = req.body_bytes {
        request_builder = request_builder.body(body.clone());
    } else if let Some(ref form) = req.body_form {
        let form_data: Vec<(&str, &str)> =
            form.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        request_builder = request_builder.form(&form_data);
    }

    let response = request_builder.send().await.map_err(|e| {
        if e.is_timeout() {
            TimeoutError::new_err(format!(
                "HTTP request to {} timed out after {}ms",
                url, req.timeout_ms
            ))
        } else if e.is_connect() {
            NetworkError::new_err(format!("Connection to {} failed: {}", url, e))
        } else {
            NetworkError::new_err(format!("HTTP request to {} failed: {}", url, e))
        }
    })?;

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    let status_code = response.status().as_u16();
    let reason = response.status().canonical_reason().map(|s| s.to_string());
    let final_url = response.url().clone().to_string();
    let content_length = response.content_length();
    let protocol_version = format!("{:?}", response.version());

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

    let mut cookies = Vec::new();
    for cookie_result in response.cookies() {
        cookies.push(HttpCookiePy {
            name: cookie_result.name().to_string(),
            value: cookie_result.value().to_string(),
            domain: cookie_result.domain().map(|d| d.to_string()),
            path: cookie_result.path().map(|p| p.to_string()),
            expires: cookie_result.expires().map(|e| format!("{:?}", e)),
            secure: cookie_result.secure(),
            http_only: cookie_result.http_only(),
        });
    }

    let body_bytes_raw = response
        .bytes()
        .await
        .map_err(|e| NetworkError::new_err(format!("Failed to read response body: {}", e)))?;
    let bytes_received = body_bytes_raw.len() as u64;
    let body_bytes = body_bytes_raw.to_vec();
    let body_text = String::from_utf8(body_bytes.clone()).ok();

    let speed_download = if elapsed > 0.0 {
        (bytes_received as f64) / (elapsed / 1000.0)
    } else {
        0.0
    };

    let timing = HttpTimingPy {
        dns_ms: None,
        connect_ms: None,
        tls_handshake_ms: None,
        first_byte_ms: None,
        total_ms: elapsed,
        size_download: bytes_received,
        speed_download,
    };

    Ok(HttpResponsePy {
        status_code,
        reason,
        body_text,
        content_length,
        final_url,
        protocol_version: Some(protocol_version),
        timing,
        bytes_received,
        headers: HttpHeadersPy { entries: headers },
        cookies,
        body_bytes,
        redirect_history: Vec::new(),
        tls_metadata: None,
        redact_config: None,
    })
}

/// Execute a request with manual redirect tracking.
async fn execute_with_redirects_async(
    config: HttpClientConfigPy,
    req: HttpRequestPy,
) -> PyResult<HttpResponsePy> {
    let max_redirects = if req.follow_redirects {
        req.max_redirects
    } else {
        0
    };

    let client = build_reqwest_client(&config)?;

    let mut redirect_history = Vec::new();
    let mut current_url = req.url.clone();
    let mut current_method = req.method.clone();
    let mut final_response: Option<HttpResponsePy> = None;

    for _ in 0..=max_redirects {
        let mut current_req = HttpRequestPy {
            method: current_method.clone(),
            url: current_url.clone(),
            headers: req.headers.clone(),
            query_params: Vec::new(),
            body_bytes: None,
            body_text: None,
            body_json: None,
            body_form: None,
            cookies: req.cookies.clone(),
            follow_redirects: false,
            max_redirects: 0,
            verify_tls: req.verify_tls,
            timeout_ms: req.timeout_ms,
            connect_timeout_ms: req.connect_timeout_ms,
            user_agent: req.user_agent.clone(),
            proxy_url: req.proxy_url.clone(),
            response_size_limit: None,
        };

        let resp = execute_request_async(&client, &mut current_req, None).await?;

        let is_redirect = [301, 302, 307, 308].contains(&resp.status_code);

        if is_redirect {
            redirect_history.push(RedirectEntryPy {
                url: resp.final_url.clone(),
                status_code: resp.status_code,
                headers: resp.headers.entries.clone(),
            });

            let location = resp.headers.get("location").ok_or_else(|| {
                NetworkError::new_err(format!(
                    "Redirect {} missing Location header",
                    resp.status_code
                ))
            })?;

            let resolved = if location.starts_with("http://") || location.starts_with("https://") {
                location
            } else {
                let base = url::Url::parse(&resp.final_url).map_err(|e| {
                    NetworkError::new_err(format!("Invalid redirect base URL: {}", e))
                })?;
                base.join(&location)
                    .map_err(|e| NetworkError::new_err(format!("Invalid redirect URL: {}", e)))?
                    .to_string()
            };

            current_url = resolved;
            current_method = if resp.status_code == 301 || resp.status_code == 308 {
                current_method.clone()
            } else {
                "GET".to_string()
            };
        } else {
            final_response = Some(HttpResponsePy {
                status_code: resp.status_code,
                reason: resp.reason,
                body_text: resp.body_text,
                content_length: resp.content_length,
                final_url: resp.final_url,
                protocol_version: resp.protocol_version,
                timing: resp.timing,
                bytes_received: resp.bytes_received,
                headers: resp.headers,
                cookies: resp.cookies,
                body_bytes: resp.body_bytes,
                redirect_history,
                tls_metadata: None,
                redact_config: None,
            });
            break;
        }
    }

    final_response.ok_or_else(|| NetworkError::new_err("Too many redirects"))
}

// ---------------------------------------------------------------------------
// HttpClientPy
// ---------------------------------------------------------------------------

/// Synchronous HTTP client with connection pooling and configuration.
#[pyclass]
pub struct HttpClientPy {
    client: Option<reqwest::Client>,
    config: HttpClientConfigPy,
}

#[pymethods]
impl HttpClientPy {
    /// Create a new HTTP client with the given configuration.
    #[new]
    fn new(config: HttpClientConfigPy) -> PyResult<Self> {
        let client = build_reqwest_client(&config)?;
        Ok(Self {
            client: Some(client),
            config,
        })
    }

    /// Execute an HTTP request.
    fn request(&self, py: Python<'_>, req: HttpRequestPy) -> PyResult<HttpResponsePy> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| NetworkError::new_err("HTTP client is closed"))?;

        let config = self.config.clone();

        block_on(py, execute_with_redirects_async(config, req))
    }

    /// Convenience GET request.
    #[pyo3(signature = (url, *, headers=None))]
    fn get(
        &self,
        py: Python<'_>,
        url: &str,
        headers: Option<Vec<(String, String)>>,
    ) -> PyResult<HttpResponsePy> {
        let req = HttpRequestPy {
            method: "GET".to_string(),
            url: url.to_string(),
            headers: headers.unwrap_or_default(),
            query_params: Vec::new(),
            body_bytes: None,
            body_text: None,
            body_json: None,
            body_form: None,
            cookies: Vec::new(),
            follow_redirects: true,
            max_redirects: self.config.max_redirects,
            verify_tls: self.config.verify_tls,
            timeout_ms: self.config.timeout_ms,
            connect_timeout_ms: self.config.connect_timeout_ms,
            user_agent: self.config.user_agent.clone(),
            proxy_url: self.config.proxy_url.clone(),
            response_size_limit: None,
        };
        self.request(py, req)
    }

    /// Convenience POST request.
    #[pyo3(signature = (url, body=None, *, headers=None, content_type=None))]
    fn post(
        &self,
        py: Python<'_>,
        url: &str,
        body: Option<&str>,
        headers: Option<Vec<(String, String)>>,
        content_type: Option<&str>,
    ) -> PyResult<HttpResponsePy> {
        let mut hdrs = headers.unwrap_or_default();
        if let Some(ct) = content_type {
            hdrs.push(("Content-Type".to_string(), ct.to_string()));
        } else if body.is_some()
            && !hdrs
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("Content-Type"))
        {
            hdrs.push(("Content-Type".to_string(), "text/plain".to_string()));
        }

        let req = HttpRequestPy {
            method: "POST".to_string(),
            url: url.to_string(),
            headers: hdrs,
            query_params: Vec::new(),
            body_bytes: None,
            body_text: body.map(|s| s.to_string()),
            body_json: None,
            body_form: None,
            cookies: Vec::new(),
            follow_redirects: true,
            max_redirects: self.config.max_redirects,
            verify_tls: self.config.verify_tls,
            timeout_ms: self.config.timeout_ms,
            connect_timeout_ms: self.config.connect_timeout_ms,
            user_agent: self.config.user_agent.clone(),
            proxy_url: self.config.proxy_url.clone(),
            response_size_limit: None,
        };
        self.request(py, req)
    }

    /// Convenience PUT request.
    #[pyo3(signature = (url, body=None, *, headers=None, content_type=None))]
    fn put(
        &self,
        py: Python<'_>,
        url: &str,
        body: Option<&str>,
        headers: Option<Vec<(String, String)>>,
        content_type: Option<&str>,
    ) -> PyResult<HttpResponsePy> {
        let mut hdrs = headers.unwrap_or_default();
        if let Some(ct) = content_type {
            hdrs.push(("Content-Type".to_string(), ct.to_string()));
        } else if body.is_some()
            && !hdrs
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("Content-Type"))
        {
            hdrs.push(("Content-Type".to_string(), "text/plain".to_string()));
        }

        let req = HttpRequestPy {
            method: "PUT".to_string(),
            url: url.to_string(),
            headers: hdrs,
            query_params: Vec::new(),
            body_bytes: None,
            body_text: body.map(|s| s.to_string()),
            body_json: None,
            body_form: None,
            cookies: Vec::new(),
            follow_redirects: true,
            max_redirects: self.config.max_redirects,
            verify_tls: self.config.verify_tls,
            timeout_ms: self.config.timeout_ms,
            connect_timeout_ms: self.config.connect_timeout_ms,
            user_agent: self.config.user_agent.clone(),
            proxy_url: self.config.proxy_url.clone(),
            response_size_limit: None,
        };
        self.request(py, req)
    }

    /// Convenience DELETE request.
    #[pyo3(signature = (url, *, headers=None))]
    fn delete(
        &self,
        py: Python<'_>,
        url: &str,
        headers: Option<Vec<(String, String)>>,
    ) -> PyResult<HttpResponsePy> {
        let req = HttpRequestPy {
            method: "DELETE".to_string(),
            url: url.to_string(),
            headers: headers.unwrap_or_default(),
            query_params: Vec::new(),
            body_bytes: None,
            body_text: None,
            body_json: None,
            body_form: None,
            cookies: Vec::new(),
            follow_redirects: true,
            max_redirects: self.config.max_redirects,
            verify_tls: self.config.verify_tls,
            timeout_ms: self.config.timeout_ms,
            connect_timeout_ms: self.config.connect_timeout_ms,
            user_agent: self.config.user_agent.clone(),
            proxy_url: self.config.proxy_url.clone(),
            response_size_limit: None,
        };
        self.request(py, req)
    }

    /// Close the client, releasing the connection pool.
    fn close(&mut self) {
        self.client.take();
    }

    /// Whether the client has been closed.
    #[getter]
    fn is_closed(&self) -> bool {
        self.client.is_none()
    }

    /// Context manager __enter__.
    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    /// Context manager __exit__: closes the client.
    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __exit__(
        &mut self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        self.close();
        false
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        self.config.to_dict(py)
    }

    fn to_json(&self) -> PyResult<String> {
        self.config.to_json()
    }

    fn __repr__(&self) -> String {
        format!(
            "HttpClientPy(base_url={:?}, closed={})",
            self.config.base_url,
            self.client.is_none()
        )
    }

    fn __str__(&self) -> String {
        format!("HttpClient(base_url={:?})", self.config.base_url)
    }
}

// ---------------------------------------------------------------------------
// AsyncHttpClientPy
// ---------------------------------------------------------------------------

/// Async HTTP client returning PyFuture objects for Python `await`.
#[pyclass]
pub struct AsyncHttpClientPy {
    config: HttpClientConfigPy,
    is_closed: Mutex<bool>,
}

#[pymethods]
impl AsyncHttpClientPy {
    #[new]
    fn new(config: HttpClientConfigPy) -> PyResult<Self> {
        Ok(Self {
            config,
            is_closed: Mutex::new(false),
        })
    }

    /// Whether the client has been closed.
    #[getter]
    fn is_closed(&self) -> bool {
        *self.is_closed.lock().unwrap()
    }

    /// Execute an HTTP request asynchronously.
    fn async_request(&self, req: HttpRequestPy) -> PyResult<runtime_async::PyFuture> {
        if *self.is_closed.lock().unwrap() {
            return Err(NetworkError::new_err("HTTP client is closed"));
        }

        let config = self.config.clone();
        runtime_async::spawn_async(async move { execute_with_redirects_async(config, req).await })
    }

    /// Convenience async GET request.
    #[pyo3(signature = (url, *, headers=None))]
    fn async_get(
        &self,
        url: &str,
        headers: Option<Vec<(String, String)>>,
    ) -> PyResult<runtime_async::PyFuture> {
        let req = HttpRequestPy {
            method: "GET".to_string(),
            url: url.to_string(),
            headers: headers.unwrap_or_default(),
            query_params: Vec::new(),
            body_bytes: None,
            body_text: None,
            body_json: None,
            body_form: None,
            cookies: Vec::new(),
            follow_redirects: true,
            max_redirects: self.config.max_redirects,
            verify_tls: self.config.verify_tls,
            timeout_ms: self.config.timeout_ms,
            connect_timeout_ms: self.config.connect_timeout_ms,
            user_agent: self.config.user_agent.clone(),
            proxy_url: self.config.proxy_url.clone(),
            response_size_limit: None,
        };
        self.async_request(req)
    }

    /// Convenience async POST request.
    #[pyo3(signature = (url, body=None, *, headers=None, content_type=None))]
    fn async_post(
        &self,
        url: &str,
        body: Option<&str>,
        headers: Option<Vec<(String, String)>>,
        content_type: Option<&str>,
    ) -> PyResult<runtime_async::PyFuture> {
        let mut hdrs = headers.unwrap_or_default();
        if let Some(ct) = content_type {
            hdrs.push(("Content-Type".to_string(), ct.to_string()));
        } else if body.is_some()
            && !hdrs
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("Content-Type"))
        {
            hdrs.push(("Content-Type".to_string(), "text/plain".to_string()));
        }

        let req = HttpRequestPy {
            method: "POST".to_string(),
            url: url.to_string(),
            headers: hdrs,
            query_params: Vec::new(),
            body_bytes: None,
            body_text: body.map(|s| s.to_string()),
            body_json: None,
            body_form: None,
            cookies: Vec::new(),
            follow_redirects: true,
            max_redirects: self.config.max_redirects,
            verify_tls: self.config.verify_tls,
            timeout_ms: self.config.timeout_ms,
            connect_timeout_ms: self.config.connect_timeout_ms,
            user_agent: self.config.user_agent.clone(),
            proxy_url: self.config.proxy_url.clone(),
            response_size_limit: None,
        };
        self.async_request(req)
    }

    /// Convenience async PUT request.
    #[pyo3(signature = (url, body=None, *, headers=None, content_type=None))]
    fn async_put(
        &self,
        url: &str,
        body: Option<&str>,
        headers: Option<Vec<(String, String)>>,
        content_type: Option<&str>,
    ) -> PyResult<runtime_async::PyFuture> {
        let mut hdrs = headers.unwrap_or_default();
        if let Some(ct) = content_type {
            hdrs.push(("Content-Type".to_string(), ct.to_string()));
        } else if body.is_some()
            && !hdrs
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case("Content-Type"))
        {
            hdrs.push(("Content-Type".to_string(), "text/plain".to_string()));
        }

        let req = HttpRequestPy {
            method: "PUT".to_string(),
            url: url.to_string(),
            headers: hdrs,
            query_params: Vec::new(),
            body_bytes: None,
            body_text: body.map(|s| s.to_string()),
            body_json: None,
            body_form: None,
            cookies: Vec::new(),
            follow_redirects: true,
            max_redirects: self.config.max_redirects,
            verify_tls: self.config.verify_tls,
            timeout_ms: self.config.timeout_ms,
            connect_timeout_ms: self.config.connect_timeout_ms,
            user_agent: self.config.user_agent.clone(),
            proxy_url: self.config.proxy_url.clone(),
            response_size_limit: None,
        };
        self.async_request(req)
    }

    /// Convenience async DELETE request.
    #[pyo3(signature = (url, *, headers=None))]
    fn async_delete(
        &self,
        url: &str,
        headers: Option<Vec<(String, String)>>,
    ) -> PyResult<runtime_async::PyFuture> {
        let req = HttpRequestPy {
            method: "DELETE".to_string(),
            url: url.to_string(),
            headers: headers.unwrap_or_default(),
            query_params: Vec::new(),
            body_bytes: None,
            body_text: None,
            body_json: None,
            body_form: None,
            cookies: Vec::new(),
            follow_redirects: true,
            max_redirects: self.config.max_redirects,
            verify_tls: self.config.verify_tls,
            timeout_ms: self.config.timeout_ms,
            connect_timeout_ms: self.config.connect_timeout_ms,
            user_agent: self.config.user_agent.clone(),
            proxy_url: self.config.proxy_url.clone(),
            response_size_limit: None,
        };
        self.async_request(req)
    }

    /// Mark the client as closed.
    fn close(&self) {
        *self.is_closed.lock().unwrap() = true;
    }

    /// Async context manager __aenter__.
    fn __aenter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    /// Async context manager __aexit__: marks the client as closed.
    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __aexit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        self.close();
        false
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        self.config.to_dict(py)
    }

    fn to_json(&self) -> PyResult<String> {
        self.config.to_json()
    }

    fn __repr__(&self) -> String {
        format!(
            "AsyncHttpClientPy(base_url={:?}, closed={})",
            self.config.base_url,
            *self.is_closed.lock().unwrap()
        )
    }

    fn __str__(&self) -> String {
        format!("AsyncHttpClient(base_url={:?})", self.config.base_url)
    }
}

// ---------------------------------------------------------------------------
// Factory functions
// ---------------------------------------------------------------------------

/// Create a synchronous HTTP client with the given configuration.
#[pyfunction]
pub fn create_http_client(config: HttpClientConfigPy) -> PyResult<HttpClientPy> {
    HttpClientPy::new(config)
}

/// Create an asynchronous HTTP client with the given configuration.
#[pyfunction]
pub fn async_create_http_client(config: HttpClientConfigPy) -> PyResult<AsyncHttpClientPy> {
    AsyncHttpClientPy::new(config)
}
