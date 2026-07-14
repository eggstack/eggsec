use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};

use crate::error::{NetworkError, TimeoutError};
use crate::runtime_async;

// ---------------------------------------------------------------------------
// TargetPy
// ---------------------------------------------------------------------------

/// A network target specification.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetPy {
    #[pyo3(get)]
    pub host: String,
    #[pyo3(get)]
    pub port: Option<u16>,
    #[pyo3(get)]
    pub scheme: Option<String>,
    #[pyo3(get)]
    pub url_path: Option<String>,
}

#[pymethods]
impl TargetPy {
    #[new]
    #[pyo3(signature = (host, port=None, scheme=None, url_path=None))]
    fn new(
        host: String,
        port: Option<u16>,
        scheme: Option<String>,
        url_path: Option<String>,
    ) -> Self {
        Self {
            host,
            port,
            scheme,
            url_path,
        }
    }

    /// Returns a normalized target string (scheme://host:port/path).
    fn normalized(&self) -> String {
        let mut s = String::new();
        if let Some(ref scheme) = self.scheme {
            s.push_str(scheme);
            s.push_str("://");
        }
        s.push_str(&self.host);
        if let Some(port) = self.port {
            let default_port = match self.scheme.as_deref() {
                Some("https") | Some("wss") => Some(443),
                Some("http") | Some("ws") => Some(80),
                _ => None,
            };
            if default_port != Some(port) {
                s.push(':');
                s.push_str(&port.to_string());
            }
        }
        if let Some(ref path) = self.url_path {
            if !path.starts_with('/') {
                s.push('/');
            }
            s.push_str(path);
        }
        s
    }

    /// Returns true if the host looks like an IPv4 or IPv6 address.
    fn is_ip(&self) -> bool {
        self.host.parse::<std::net::IpAddr>().is_ok()
    }

    /// Returns true if the host is not a numeric IP address.
    fn is_hostname(&self) -> bool {
        !self.is_ip()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("host", &self.host)?;
        dict.set_item("port", &self.port)?;
        dict.set_item("scheme", &self.scheme)?;
        dict.set_item("url_path", &self.url_path)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "TargetPy(host={}, port={:?}, scheme={:?})",
            self.host, self.port, self.scheme
        )
    }

    fn __str__(&self) -> String {
        self.normalized()
    }
}

// ---------------------------------------------------------------------------
// ResolvedTargetPy
// ---------------------------------------------------------------------------

/// Result of DNS resolution for a target.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedTargetPy {
    #[pyo3(get)]
    pub target: String,
    pub(crate) resolved_ips: Vec<String>,
    #[pyo3(get)]
    pub address_family: String,
    #[pyo3(get)]
    pub resolver_source: String,
    #[pyo3(get)]
    pub canonical_name: Option<String>,
    #[pyo3(get)]
    pub resolution_time_ms: Option<f64>,
}

#[pymethods]
impl ResolvedTargetPy {
    #[getter]
    fn resolved_ips(&self) -> Vec<String> {
        self.resolved_ips.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("resolved_ips", &self.resolved_ips)?;
        dict.set_item("address_family", &self.address_family)?;
        dict.set_item("resolver_source", &self.resolver_source)?;
        dict.set_item("canonical_name", &self.canonical_name)?;
        dict.set_item("resolution_time_ms", &self.resolution_time_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ResolvedTargetPy(target={}, ips={}, family={})",
            self.target,
            self.resolved_ips.len(),
            self.address_family
        )
    }

    fn __str__(&self) -> String {
        format!(
            "{} -> [{}] ({})",
            self.target,
            self.resolved_ips.join(", "),
            self.address_family
        )
    }
}

// ---------------------------------------------------------------------------
// ConnectionConfigPy
// ---------------------------------------------------------------------------

/// Connection configuration with timeout and retry settings.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfigPy {
    #[pyo3(get)]
    pub connect_timeout_ms: u64,
    #[pyo3(get)]
    pub read_timeout_ms: u64,
    #[pyo3(get)]
    pub write_timeout_ms: u64,
    #[pyo3(get)]
    pub handshake_timeout_ms: u64,
    #[pyo3(get)]
    pub idle_timeout_ms: u64,
    #[pyo3(get)]
    pub max_retries: u32,
    #[pyo3(get)]
    pub retry_delay_ms: u64,
}

#[pymethods]
impl ConnectionConfigPy {
    #[new]
    #[pyo3(signature = (connect_timeout_ms=5000, read_timeout_ms=30000, write_timeout_ms=30000, handshake_timeout_ms=10000, idle_timeout_ms=60000, max_retries=0, retry_delay_ms=1000))]
    fn new(
        connect_timeout_ms: u64,
        read_timeout_ms: u64,
        write_timeout_ms: u64,
        handshake_timeout_ms: u64,
        idle_timeout_ms: u64,
        max_retries: u32,
        retry_delay_ms: u64,
    ) -> Self {
        Self {
            connect_timeout_ms,
            read_timeout_ms,
            write_timeout_ms,
            handshake_timeout_ms,
            idle_timeout_ms,
            max_retries,
            retry_delay_ms,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("connect_timeout_ms", self.connect_timeout_ms)?;
        dict.set_item("read_timeout_ms", self.read_timeout_ms)?;
        dict.set_item("write_timeout_ms", self.write_timeout_ms)?;
        dict.set_item("handshake_timeout_ms", self.handshake_timeout_ms)?;
        dict.set_item("idle_timeout_ms", self.idle_timeout_ms)?;
        dict.set_item("max_retries", self.max_retries)?;
        dict.set_item("retry_delay_ms", self.retry_delay_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ConnectionConfigPy(connect={}ms, read={}ms, write={}ms, retries={})",
            self.connect_timeout_ms, self.read_timeout_ms, self.write_timeout_ms, self.max_retries
        )
    }

    fn __str__(&self) -> String {
        format!(
            "connect={}ms read={}ms write={}ms handshake={}ms idle={}ms retries={} delay={}ms",
            self.connect_timeout_ms,
            self.read_timeout_ms,
            self.write_timeout_ms,
            self.handshake_timeout_ms,
            self.idle_timeout_ms,
            self.max_retries,
            self.retry_delay_ms
        )
    }
}

// ---------------------------------------------------------------------------
// TimeoutConfigPy
// ---------------------------------------------------------------------------

/// Distinguished timeout configuration for different phases of a connection.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfigPy {
    #[pyo3(get)]
    pub connect_ms: u64,
    #[pyo3(get)]
    pub read_ms: u64,
    #[pyo3(get)]
    pub write_ms: u64,
    #[pyo3(get)]
    pub handshake_ms: u64,
    #[pyo3(get)]
    pub operation_ms: u64,
    #[pyo3(get)]
    pub idle_ms: u64,
}

#[pymethods]
impl TimeoutConfigPy {
    #[new]
    #[pyo3(signature = (connect_ms=5000, read_ms=30000, write_ms=30000, handshake_ms=10000, operation_ms=60000, idle_ms=60000))]
    fn new(
        connect_ms: u64,
        read_ms: u64,
        write_ms: u64,
        handshake_ms: u64,
        operation_ms: u64,
        idle_ms: u64,
    ) -> Self {
        Self {
            connect_ms,
            read_ms,
            write_ms,
            handshake_ms,
            operation_ms,
            idle_ms,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("connect_ms", self.connect_ms)?;
        dict.set_item("read_ms", self.read_ms)?;
        dict.set_item("write_ms", self.write_ms)?;
        dict.set_item("handshake_ms", self.handshake_ms)?;
        dict.set_item("operation_ms", self.operation_ms)?;
        dict.set_item("idle_ms", self.idle_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "TimeoutConfigPy(connect={}ms, read={}ms, write={}ms, handshake={}ms, operation={}ms, idle={}ms)",
            self.connect_ms,
            self.read_ms,
            self.write_ms,
            self.handshake_ms,
            self.operation_ms,
            self.idle_ms
        )
    }

    fn __str__(&self) -> String {
        format!(
            "connect={}ms read={}ms write={}ms handshake={}ms operation={}ms idle={}ms",
            self.connect_ms,
            self.read_ms,
            self.write_ms,
            self.handshake_ms,
            self.operation_ms,
            self.idle_ms
        )
    }
}

// ---------------------------------------------------------------------------
// RetryPolicyPy
// ---------------------------------------------------------------------------

/// Retry configuration for failed connections.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicyPy {
    #[pyo3(get)]
    pub max_retries: u32,
    #[pyo3(get)]
    pub delay_ms: u64,
    #[pyo3(get)]
    pub backoff_multiplier: f64,
    #[pyo3(get)]
    pub max_delay_ms: u64,
    pub(crate) retryable_errors: Vec<String>,
}

#[pymethods]
impl RetryPolicyPy {
    #[new]
    #[pyo3(signature = (max_retries=0, delay_ms=1000, backoff_multiplier=1.0, max_delay_ms=30000, retryable_errors=None))]
    fn new(
        max_retries: u32,
        delay_ms: u64,
        backoff_multiplier: f64,
        max_delay_ms: u64,
        retryable_errors: Option<Vec<String>>,
    ) -> Self {
        Self {
            max_retries,
            delay_ms,
            backoff_multiplier,
            max_delay_ms,
            retryable_errors: retryable_errors.unwrap_or_default(),
        }
    }

    #[getter]
    fn retryable_errors(&self) -> Vec<String> {
        self.retryable_errors.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("max_retries", self.max_retries)?;
        dict.set_item("delay_ms", self.delay_ms)?;
        dict.set_item("backoff_multiplier", self.backoff_multiplier)?;
        dict.set_item("max_delay_ms", self.max_delay_ms)?;
        dict.set_item("retryable_errors", &self.retryable_errors)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "RetryPolicyPy(max_retries={}, delay={}ms, backoff={}, max_delay={}ms, errors={})",
            self.max_retries,
            self.delay_ms,
            self.backoff_multiplier,
            self.max_delay_ms,
            self.retryable_errors.len()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "retries={} delay={}ms backoff={}x max_delay={}ms",
            self.max_retries, self.delay_ms, self.backoff_multiplier, self.max_delay_ms
        )
    }
}

// ---------------------------------------------------------------------------
// SocketEndpointPy
// ---------------------------------------------------------------------------

/// Socket endpoint information (local or remote).
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocketEndpointPy {
    #[pyo3(get)]
    pub address: String,
    #[pyo3(get)]
    pub port: u16,
    #[pyo3(get)]
    pub address_family: String,
    #[pyo3(get)]
    pub is_loopback: bool,
}

#[pymethods]
impl SocketEndpointPy {
    #[new]
    fn new(address: String, port: u16, address_family: String, is_loopback: bool) -> Self {
        Self {
            address,
            port,
            address_family,
            is_loopback,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("address", &self.address)?;
        dict.set_item("port", self.port)?;
        dict.set_item("address_family", &self.address_family)?;
        dict.set_item("is_loopback", self.is_loopback)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "SocketEndpointPy(address={}, port={}, family={})",
            self.address, self.port, self.address_family
        )
    }

    fn __str__(&self) -> String {
        if self.address_family == "ipv6" {
            format!("[{}]:{}", self.address, self.port)
        } else {
            format!("{}:{}", self.address, self.port)
        }
    }
}

impl std::fmt::Display for SocketEndpointPy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.address_family == "ipv6" {
            write!(f, "[{}]:{}", self.address, self.port)
        } else {
            write!(f, "{}:{}", self.address, self.port)
        }
    }
}

// ---------------------------------------------------------------------------
// ConnectionTimingPy
// ---------------------------------------------------------------------------

/// Timing breakdown for a connection lifecycle.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTimingPy {
    #[pyo3(get)]
    pub dns_resolution_ms: Option<f64>,
    #[pyo3(get)]
    pub tcp_connect_ms: Option<f64>,
    #[pyo3(get)]
    pub tls_handshake_ms: Option<f64>,
    #[pyo3(get)]
    pub first_byte_ms: Option<f64>,
    #[pyo3(get)]
    pub total_ms: f64,
    #[pyo3(get)]
    pub connection_reused: bool,
}

#[pymethods]
impl ConnectionTimingPy {
    #[new]
    #[pyo3(signature = (dns_resolution_ms=None, tcp_connect_ms=None, tls_handshake_ms=None, first_byte_ms=None, total_ms=0.0, connection_reused=false))]
    fn new(
        dns_resolution_ms: Option<f64>,
        tcp_connect_ms: Option<f64>,
        tls_handshake_ms: Option<f64>,
        first_byte_ms: Option<f64>,
        total_ms: f64,
        connection_reused: bool,
    ) -> Self {
        Self {
            dns_resolution_ms,
            tcp_connect_ms,
            tls_handshake_ms,
            first_byte_ms,
            total_ms,
            connection_reused,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("dns_resolution_ms", &self.dns_resolution_ms)?;
        dict.set_item("tcp_connect_ms", &self.tcp_connect_ms)?;
        dict.set_item("tls_handshake_ms", &self.tls_handshake_ms)?;
        dict.set_item("first_byte_ms", &self.first_byte_ms)?;
        dict.set_item("total_ms", self.total_ms)?;
        dict.set_item("connection_reused", self.connection_reused)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ConnectionTimingPy(total={}ms, reused={})",
            self.total_ms, self.connection_reused
        )
    }

    fn __str__(&self) -> String {
        let mut parts = vec![format!("total={:.1}ms", self.total_ms)];
        if let Some(dns) = self.dns_resolution_ms {
            parts.push(format!("dns={:.1}ms", dns));
        }
        if let Some(tcp) = self.tcp_connect_ms {
            parts.push(format!("tcp={:.1}ms", tcp));
        }
        if let Some(tls) = self.tls_handshake_ms {
            parts.push(format!("tls={:.1}ms", tls));
        }
        if let Some(ttfb) = self.first_byte_ms {
            parts.push(format!("ttfb={:.1}ms", ttfb));
        }
        if self.connection_reused {
            parts.push("reused".to_string());
        }
        parts.join(" ")
    }
}

// ---------------------------------------------------------------------------
// ConnectionMetadataPy
// ---------------------------------------------------------------------------

/// Full connection metadata for a network operation.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMetadataPy {
    #[pyo3(get)]
    pub local_endpoint: Option<SocketEndpointPy>,
    #[pyo3(get)]
    pub remote_endpoint: Option<SocketEndpointPy>,
    #[pyo3(get)]
    pub resolved_address: Option<String>,
    #[pyo3(get)]
    pub transport_protocol: String,
    #[pyo3(get)]
    pub negotiated_protocol: Option<String>,
    #[pyo3(get)]
    pub connection_reused: bool,
    #[pyo3(get)]
    pub timing: Option<ConnectionTimingPy>,
    #[pyo3(get)]
    pub tls_version: Option<String>,
    #[pyo3(get)]
    pub tls_cipher: Option<String>,
    #[pyo3(get)]
    pub bytes_sent: u64,
    #[pyo3(get)]
    pub bytes_received: u64,
}

#[pymethods]
impl ConnectionMetadataPy {
    #[new]
    #[pyo3(signature = (local_endpoint=None, remote_endpoint=None, resolved_address=None, transport_protocol="tcp", negotiated_protocol=None, connection_reused=false, timing=None, tls_version=None, tls_cipher=None, bytes_sent=0, bytes_received=0))]
    fn new(
        local_endpoint: Option<SocketEndpointPy>,
        remote_endpoint: Option<SocketEndpointPy>,
        resolved_address: Option<String>,
        transport_protocol: &str,
        negotiated_protocol: Option<String>,
        connection_reused: bool,
        timing: Option<ConnectionTimingPy>,
        tls_version: Option<String>,
        tls_cipher: Option<String>,
        bytes_sent: u64,
        bytes_received: u64,
    ) -> Self {
        Self {
            local_endpoint,
            remote_endpoint,
            resolved_address,
            transport_protocol: transport_protocol.to_string(),
            negotiated_protocol,
            connection_reused,
            timing,
            tls_version,
            tls_cipher,
            bytes_sent,
            bytes_received,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);

        if let Some(ref ep) = self.local_endpoint {
            dict.set_item("local_endpoint", ep.to_dict(py)?)?;
        } else {
            dict.set_item("local_endpoint", py.None())?;
        }

        if let Some(ref ep) = self.remote_endpoint {
            dict.set_item("remote_endpoint", ep.to_dict(py)?)?;
        } else {
            dict.set_item("remote_endpoint", py.None())?;
        }

        dict.set_item("resolved_address", &self.resolved_address)?;
        dict.set_item("transport_protocol", &self.transport_protocol)?;
        dict.set_item("negotiated_protocol", &self.negotiated_protocol)?;
        dict.set_item("connection_reused", self.connection_reused)?;

        if let Some(ref t) = self.timing {
            dict.set_item("timing", t.to_dict(py)?)?;
        } else {
            dict.set_item("timing", py.None())?;
        }

        dict.set_item("tls_version", &self.tls_version)?;
        dict.set_item("tls_cipher", &self.tls_cipher)?;
        dict.set_item("bytes_sent", self.bytes_sent)?;
        dict.set_item("bytes_received", self.bytes_received)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        let remote = self
            .remote_endpoint
            .as_ref()
            .map(|e| e.to_string())
            .unwrap_or_else(|| "None".to_string());
        format!(
            "ConnectionMetadataPy(remote={}, transport={}, reused={})",
            remote, self.transport_protocol, self.connection_reused
        )
    }

    fn __str__(&self) -> String {
        let remote = self
            .remote_endpoint
            .as_ref()
            .map(|e| e.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let proto = self
            .negotiated_protocol
            .as_deref()
            .unwrap_or(&self.transport_protocol);
        format!(
            "{} ({}, sent={}B recv={}B)",
            remote, proto, self.bytes_sent, self.bytes_received
        )
    }
}

// ---------------------------------------------------------------------------
// TranscriptEntryPy
// ---------------------------------------------------------------------------

/// A single entry in a network transcript.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptEntryPy {
    #[pyo3(get)]
    pub sequence: u64,
    #[pyo3(get)]
    pub direction: String,
    #[pyo3(get)]
    pub timestamp_ms: f64,
    #[pyo3(get)]
    pub data_type: String,
    #[pyo3(get)]
    pub size: usize,
    #[pyo3(get)]
    pub summary: Option<String>,
    #[pyo3(get)]
    pub redacted: bool,
}

#[pymethods]
impl TranscriptEntryPy {
    #[new]
    #[pyo3(signature = (sequence, direction, timestamp_ms, data_type, size, summary=None, redacted=false))]
    fn new(
        sequence: u64,
        direction: String,
        timestamp_ms: f64,
        data_type: String,
        size: usize,
        summary: Option<String>,
        redacted: bool,
    ) -> Self {
        Self {
            sequence,
            direction,
            timestamp_ms,
            data_type,
            size,
            summary,
            redacted,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("sequence", self.sequence)?;
        dict.set_item("direction", &self.direction)?;
        dict.set_item("timestamp_ms", self.timestamp_ms)?;
        dict.set_item("data_type", &self.data_type)?;
        dict.set_item("size", self.size)?;
        dict.set_item("summary", &self.summary)?;
        dict.set_item("redacted", self.redacted)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "TranscriptEntryPy(seq={}, dir={}, type={}, size={})",
            self.sequence, self.direction, self.data_type, self.size
        )
    }

    fn __str__(&self) -> String {
        format!(
            "#{} {} {} {}B{}",
            self.sequence,
            self.direction,
            self.data_type,
            self.size,
            if self.redacted { " (redacted)" } else { "" }
        )
    }
}

// ---------------------------------------------------------------------------
// NetworkTranscriptPy
// ---------------------------------------------------------------------------

/// Collection of transcript entries from a network operation.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTranscriptPy {
    pub(crate) entries: Vec<TranscriptEntryPy>,
    #[pyo3(get)]
    pub total_bytes: u64,
    #[pyo3(get)]
    pub truncated: bool,
}

#[pymethods]
impl NetworkTranscriptPy {
    #[new]
    #[pyo3(signature = (entries=None, total_bytes=0, truncated=false))]
    fn new(entries: Option<Vec<TranscriptEntryPy>>, total_bytes: u64, truncated: bool) -> Self {
        Self {
            entries: entries.unwrap_or_default(),
            total_bytes,
            truncated,
        }
    }

    #[getter]
    fn entries(&self) -> Vec<TranscriptEntryPy> {
        self.entries.clone()
    }

    fn __len__(&self) -> usize {
        self.entries.len()
    }

    fn __getitem__(&self, index: usize) -> PyResult<TranscriptEntryPy> {
        self.entries
            .get(index)
            .cloned()
            .ok_or_else(|| pyo3::exceptions::PyIndexError::new_err("transcript index out of range"))
    }

    /// Add a transcript entry.
    fn add_entry(&self, entry: TranscriptEntryPy) -> PyResult<Self> {
        let mut entries = self.entries.clone();
        entries.push(entry);
        let total_bytes = self.total_bytes + entries.last().map_or(0, |e| e.size as u64);
        Ok(Self {
            entries,
            total_bytes,
            truncated: self.truncated,
        })
    }

    /// Return a human-readable summary of the transcript.
    fn summary(&self) -> String {
        let sent: Vec<&TranscriptEntryPy> = self
            .entries
            .iter()
            .filter(|e| e.direction == "sent")
            .collect();
        let received: Vec<&TranscriptEntryPy> = self
            .entries
            .iter()
            .filter(|e| e.direction == "received")
            .collect();
        let sent_bytes: usize = sent.iter().map(|e| e.size).sum();
        let recv_bytes: usize = received.iter().map(|e| e.size).sum();
        format!(
            "{} entries ({} sent={}B, {} recv={}B){}",
            self.entries.len(),
            sent.len(),
            sent_bytes,
            received.len(),
            recv_bytes,
            if self.truncated { " [truncated]" } else { "" }
        )
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);

        let entries_list = PyList::empty_bound(py);
        for entry in &self.entries {
            entries_list.append(entry.to_dict(py)?)?;
        }
        dict.set_item("entries", entries_list)?;
        dict.set_item("total_bytes", self.total_bytes)?;
        dict.set_item("truncated", self.truncated)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "NetworkTranscriptPy(entries={}, total_bytes={}, truncated={})",
            self.entries.len(),
            self.total_bytes,
            self.truncated
        )
    }

    fn __str__(&self) -> String {
        self.summary()
    }
}

// ---------------------------------------------------------------------------
// NetworkEvidencePy
// ---------------------------------------------------------------------------

/// Evidence collected from a network operation.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkEvidencePy {
    #[pyo3(get)]
    pub operation: String,
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub metadata: Option<ConnectionMetadataPy>,
    #[pyo3(get)]
    pub transcript: Option<NetworkTranscriptPy>,
    pub(crate) artifact_refs: Vec<String>,
    #[pyo3(get)]
    pub timing: Option<ConnectionTimingPy>,
}

#[pymethods]
impl NetworkEvidencePy {
    #[new]
    #[pyo3(signature = (operation, target, metadata=None, transcript=None, artifact_refs=None, timing=None))]
    fn new(
        operation: String,
        target: String,
        metadata: Option<ConnectionMetadataPy>,
        transcript: Option<NetworkTranscriptPy>,
        artifact_refs: Option<Vec<String>>,
        timing: Option<ConnectionTimingPy>,
    ) -> Self {
        Self {
            operation,
            target,
            metadata,
            transcript,
            artifact_refs: artifact_refs.unwrap_or_default(),
            timing,
        }
    }

    #[getter]
    fn artifact_refs(&self) -> Vec<String> {
        self.artifact_refs.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("operation", &self.operation)?;
        dict.set_item("target", &self.target)?;

        if let Some(ref m) = self.metadata {
            dict.set_item("metadata", m.to_dict(py)?)?;
        } else {
            dict.set_item("metadata", py.None())?;
        }

        if let Some(ref t) = self.transcript {
            dict.set_item("transcript", t.to_dict(py)?)?;
        } else {
            dict.set_item("transcript", py.None())?;
        }

        dict.set_item("artifact_refs", &self.artifact_refs)?;

        if let Some(ref t) = self.timing {
            dict.set_item("timing", t.to_dict(py)?)?;
        } else {
            dict.set_item("timing", py.None())?;
        }

        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "NetworkEvidencePy(operation={}, target={}, artifacts={})",
            self.operation,
            self.target,
            self.artifact_refs.len()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "{} on {} ({} artifacts)",
            self.operation,
            self.target,
            self.artifact_refs.len()
        )
    }
}

// ---------------------------------------------------------------------------
// ProxyRoutePy
// ---------------------------------------------------------------------------

/// A proxy route configuration for routing network traffic through a proxy.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyRoutePy {
    #[pyo3(get)]
    pub proxy_type: String,
    #[pyo3(get)]
    pub host: String,
    #[pyo3(get)]
    pub port: u16,
    #[pyo3(get)]
    pub username: Option<String>,
    #[pyo3(get)]
    pub password: Option<String>,
    #[pyo3(get)]
    pub no_proxy: Vec<String>,
}

#[pymethods]
impl ProxyRoutePy {
    #[new]
    #[pyo3(signature = (host, port, proxy_type="http", username=None, password=None, no_proxy=None))]
    fn new(
        host: String,
        port: u16,
        proxy_type: &str,
        username: Option<String>,
        password: Option<String>,
        no_proxy: Option<Vec<String>>,
    ) -> Self {
        Self {
            proxy_type: proxy_type.to_string(),
            host,
            port,
            username,
            password,
            no_proxy: no_proxy.unwrap_or_default(),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("proxy_type", &self.proxy_type)?;
        dict.set_item("host", &self.host)?;
        dict.set_item("port", self.port)?;
        dict.set_item("username", &self.username)?;
        dict.set_item("password", &self.password)?;
        dict.set_item("no_proxy", &self.no_proxy)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ProxyRoutePy(type={}, host={}, port={}, user={:?})",
            self.proxy_type, self.host, self.port, self.username
        )
    }

    fn __str__(&self) -> String {
        format!("{}://{}:{}", self.proxy_type, self.host, self.port)
    }

    /// Return the URL string for this proxy (without credentials).
    fn url(&self) -> String {
        format!("{}://{}:{}", self.proxy_type, self.host, self.port)
    }
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Convert a `ConnectionTimingPy` to a Python dict (callable from Rust).
pub(crate) fn timing_to_dict(py: Python, timing: &ConnectionTimingPy) -> PyResult<PyObject> {
    let dict = PyDict::new_bound(py);
    dict.set_item("dns_resolution_ms", &timing.dns_resolution_ms)?;
    dict.set_item("tcp_connect_ms", &timing.tcp_connect_ms)?;
    dict.set_item("tls_handshake_ms", &timing.tls_handshake_ms)?;
    dict.set_item("first_byte_ms", &timing.first_byte_ms)?;
    dict.set_item("total_ms", timing.total_ms)?;
    dict.set_item("connection_reused", timing.connection_reused)?;
    Ok(dict.into())
}

/// Resolve a target using system DNS (synchronous, blocks the GIL).
///
/// Args:
///     target: The target to resolve.
///     timeout_ms: DNS resolution timeout in milliseconds (default: 5000).
///     max_results: Maximum number of results to return (default: 100).
///
/// Returns:
///     ResolvedTargetPy with resolved IP addresses and metadata.
#[pyfunction]
#[pyo3(signature = (target, timeout_ms=5000, max_results=100))]
pub fn resolve_target_sync(
    target: &TargetPy,
    timeout_ms: u64,
    max_results: usize,
) -> PyResult<ResolvedTargetPy> {
    let host = target.host.clone();
    let timeout = std::time::Duration::from_millis(timeout_ms);

    Python::with_gil(|py| {
        crate::runtime_sync::block_on(py, async move {
            resolve_host_async(&host, timeout, max_results).await
        })
    })
}

/// Resolve a target using system DNS (async, returns a PyFuture).
///
/// Args:
///     target: The target to resolve.
///     timeout_ms: DNS resolution timeout in milliseconds (default: 5000).
///     max_results: Maximum number of results to return (default: 100).
///
/// Returns:
///     PyFuture that resolves to ResolvedTargetPy.
#[pyfunction]
#[pyo3(signature = (target, timeout_ms=5000, max_results=100))]
pub fn async_resolve_target(
    target: &TargetPy,
    timeout_ms: u64,
    max_results: usize,
) -> PyResult<runtime_async::PyFuture> {
    let host = target.host.clone();
    let timeout = std::time::Duration::from_millis(timeout_ms);

    runtime_async::spawn_async(async move { resolve_host_async(&host, timeout, max_results).await })
}

/// Internal async DNS resolution implementation.
async fn resolve_host_async(
    host: &str,
    timeout: std::time::Duration,
    max_results: usize,
) -> PyResult<ResolvedTargetPy> {
    use tokio::net::lookup_host;

    let start = std::time::Instant::now();

    let addrs = tokio::time::timeout(timeout, lookup_host(format!("{}:0", host)))
        .await
        .map_err(|_| {
            TimeoutError::new_err(format!(
                "DNS resolution timed out after {}ms",
                timeout.as_millis()
            ))
        })?
        .map_err(|e| NetworkError::new_err(format!("DNS resolution failed for {}: {}", host, e)))?;

    let mut ips = Vec::new();
    let mut has_ipv4 = false;
    let mut has_ipv6 = false;

    for addr in addrs {
        if ips.len() >= max_results {
            break;
        }
        let ip = addr.ip();
        match ip {
            std::net::IpAddr::V4(_) => has_ipv4 = true,
            std::net::IpAddr::V6(_) => has_ipv6 = true,
        }
        ips.push(ip.to_string());
    }

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

    if ips.is_empty() {
        return Err(NetworkError::new_err(format!(
            "No addresses found for {}",
            host
        )));
    }

    let address_family = if has_ipv4 && has_ipv6 {
        "dual"
    } else if has_ipv6 {
        "ipv6"
    } else {
        "ipv4"
    };

    Ok(ResolvedTargetPy {
        target: host.to_string(),
        resolved_ips: ips,
        address_family: address_family.to_string(),
        resolver_source: "system".to_string(),
        canonical_name: None,
        resolution_time_ms: Some(elapsed_ms),
    })
}
