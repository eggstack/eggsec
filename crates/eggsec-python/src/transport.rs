use std::sync::Arc;
use std::time::Instant;

use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::{NetworkError, TimeoutError};
use crate::network::{
    ConnectionMetadataPy, ConnectionTimingPy, NetworkTranscriptPy, SocketEndpointPy,
    TranscriptEntryPy,
};
use crate::runtime_async;
use crate::runtime_sync;

// ═══════════════════════════════════════════════════════════════════
// TCP Configuration
// ═══════════════════════════════════════════════════════════════════

/// TCP connection configuration with timeouts and socket options.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpConfigPy {
    #[pyo3(get)]
    pub host: String,
    #[pyo3(get)]
    pub port: u16,
    #[pyo3(get)]
    pub connect_timeout_ms: u64,
    #[pyo3(get)]
    pub read_timeout_ms: u64,
    #[pyo3(get)]
    pub write_timeout_ms: u64,
    #[pyo3(get)]
    pub idle_timeout_ms: u64,
    #[pyo3(get)]
    pub nodelay: bool,
}

#[pymethods]
impl TcpConfigPy {
    #[new]
    #[pyo3(signature = (host, port, connect_timeout_ms=5000, read_timeout_ms=30000, write_timeout_ms=30000, idle_timeout_ms=60000, nodelay=true))]
    fn new(
        host: String,
        port: u16,
        connect_timeout_ms: u64,
        read_timeout_ms: u64,
        write_timeout_ms: u64,
        idle_timeout_ms: u64,
        nodelay: bool,
    ) -> Self {
        Self {
            host,
            port,
            connect_timeout_ms,
            read_timeout_ms,
            write_timeout_ms,
            idle_timeout_ms,
            nodelay,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("host", &self.host)?;
        dict.set_item("port", self.port)?;
        dict.set_item("connect_timeout_ms", self.connect_timeout_ms)?;
        dict.set_item("read_timeout_ms", self.read_timeout_ms)?;
        dict.set_item("write_timeout_ms", self.write_timeout_ms)?;
        dict.set_item("idle_timeout_ms", self.idle_timeout_ms)?;
        dict.set_item("nodelay", self.nodelay)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "TcpConfig(host={}, port={}, connect={}ms, nodelay={})",
            self.host, self.port, self.connect_timeout_ms, self.nodelay
        )
    }

    fn __str__(&self) -> String {
        format!(
            "tcp://{}:{} connect={}ms read={}ms write={}ms nodelay={}",
            self.host,
            self.port,
            self.connect_timeout_ms,
            self.read_timeout_ms,
            self.write_timeout_ms,
            self.nodelay
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// TCP Connect Result
// ═══════════════════════════════════════════════════════════════════

/// Result of a TCP connect operation.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpConnectResultPy {
    #[pyo3(get)]
    pub local_endpoint: SocketEndpointPy,
    #[pyo3(get)]
    pub remote_endpoint: SocketEndpointPy,
    #[pyo3(get)]
    pub timing: ConnectionTimingPy,
    #[pyo3(get)]
    pub metadata: ConnectionMetadataPy,
}

#[pymethods]
impl TcpConnectResultPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        let local_dict = PyDict::new_bound(py);
        local_dict.set_item("address", &self.local_endpoint.address)?;
        local_dict.set_item("port", self.local_endpoint.port)?;
        local_dict.set_item("address_family", &self.local_endpoint.address_family)?;
        local_dict.set_item("is_loopback", self.local_endpoint.is_loopback)?;
        dict.set_item("local_endpoint", local_dict)?;
        let remote_dict = PyDict::new_bound(py);
        remote_dict.set_item("address", &self.remote_endpoint.address)?;
        remote_dict.set_item("port", self.remote_endpoint.port)?;
        remote_dict.set_item("address_family", &self.remote_endpoint.address_family)?;
        remote_dict.set_item("is_loopback", self.remote_endpoint.is_loopback)?;
        dict.set_item("remote_endpoint", remote_dict)?;
        let timing_dict = PyDict::new_bound(py);
        timing_dict.set_item("dns_resolution_ms", &self.timing.dns_resolution_ms)?;
        timing_dict.set_item("tcp_connect_ms", &self.timing.tcp_connect_ms)?;
        timing_dict.set_item("tls_handshake_ms", &self.timing.tls_handshake_ms)?;
        timing_dict.set_item("first_byte_ms", &self.timing.first_byte_ms)?;
        timing_dict.set_item("total_ms", self.timing.total_ms)?;
        timing_dict.set_item("connection_reused", self.timing.connection_reused)?;
        dict.set_item("timing", timing_dict)?;
        let meta_dict = PyDict::new_bound(py);
        meta_dict.set_item("transport_protocol", &self.metadata.transport_protocol)?;
        meta_dict.set_item("connection_reused", self.metadata.connection_reused)?;
        meta_dict.set_item("bytes_sent", self.metadata.bytes_sent)?;
        meta_dict.set_item("bytes_received", self.metadata.bytes_received)?;
        dict.set_item("metadata", meta_dict)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "TcpConnectResult(local={}, remote={})",
            self.local_endpoint, self.remote_endpoint
        )
    }

    fn __str__(&self) -> String {
        format!(
            "{} -> {} ({:.1}ms)",
            self.local_endpoint, self.remote_endpoint, self.timing.total_ms
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// TCP Read Result
// ═══════════════════════════════════════════════════════════════════

/// Result of a TCP read operation.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpReadResultPy {
    #[pyo3(get)]
    pub data: Vec<u8>,
    #[pyo3(get)]
    pub bytes_read: usize,
    #[pyo3(get)]
    pub eof: bool,
    #[pyo3(get)]
    pub duration_ms: f64,
}

#[pymethods]
impl TcpReadResultPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("data", &self.data)?;
        dict.set_item("bytes_read", self.bytes_read)?;
        dict.set_item("eof", self.eof)?;
        dict.set_item("duration_ms", self.duration_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "TcpReadResult(bytes={}, eof={}, duration={:.1}ms)",
            self.bytes_read, self.eof, self.duration_ms
        )
    }

    fn __str__(&self) -> String {
        if self.eof {
            format!("{} bytes (EOF)", self.bytes_read)
        } else {
            format!("{} bytes", self.bytes_read)
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// TCP Write Result
// ═══════════════════════════════════════════════════════════════════

/// Result of a TCP write operation.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpWriteResultPy {
    #[pyo3(get)]
    pub bytes_written: usize,
    #[pyo3(get)]
    pub duration_ms: f64,
}

#[pymethods]
impl TcpWriteResultPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("bytes_written", self.bytes_written)?;
        dict.set_item("duration_ms", self.duration_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "TcpWriteResult(bytes={}, duration={:.1}ms)",
            self.bytes_written, self.duration_ms
        )
    }

    fn __str__(&self) -> String {
        format!("{} bytes in {:.1}ms", self.bytes_written, self.duration_ms)
    }
}

// ═══════════════════════════════════════════════════════════════════
// TCP Session (mutable)
// ═══════════════════════════════════════════════════════════════════

/// Internal state for a managed TCP session.
struct TcpSessionState {
    stream: Option<tokio::net::TcpStream>,
    is_closed: bool,
    transcript: Vec<TranscriptEntryPy>,
    bytes_sent: u64,
    bytes_received: u64,
    sequence: u64,
}

/// Managed TCP session with transcript tracking and deterministic close.
///
/// Supports context manager protocol for safe resource cleanup.
#[pyclass]
pub struct TcpSessionPy {
    config: TcpConfigPy,
    state: Arc<std::sync::Mutex<TcpSessionState>>,
}

fn endpoint_from_socket_addr(addr: std::net::SocketAddr) -> SocketEndpointPy {
    let (address, port, address_family) = match addr {
        std::net::SocketAddr::V4(a) => (a.ip().to_string(), a.port(), "ipv4".to_string()),
        std::net::SocketAddr::V6(a) => (a.ip().to_string(), a.port(), "ipv6".to_string()),
    };
    let is_loopback = addr.ip().is_loopback();
    SocketEndpointPy {
        address,
        port,
        address_family,
        is_loopback,
    }
}

#[pymethods]
impl TcpSessionPy {
    #[new]
    #[pyo3(signature = (config))]
    fn new(config: TcpConfigPy) -> Self {
        Self {
            config,
            state: Arc::new(std::sync::Mutex::new(TcpSessionState {
                stream: None,
                is_closed: false,
                transcript: Vec::new(),
                bytes_sent: 0,
                bytes_received: 0,
                sequence: 0,
            })),
        }
    }

    #[getter]
    fn is_closed(&self) -> bool {
        self.state.lock().unwrap().is_closed
    }

    #[getter]
    fn config(&self) -> TcpConfigPy {
        self.config.clone()
    }

    /// Return the transcript of all read/write operations.
    #[getter]
    fn transcript(&self) -> NetworkTranscriptPy {
        let s = self.state.lock().unwrap();
        let total_bytes = s.bytes_sent + s.bytes_received;
        NetworkTranscriptPy {
            entries: s.transcript.clone(),
            total_bytes,
            truncated: false,
        }
    }

    /// Return bytes sent counter.
    #[getter]
    fn bytes_sent(&self) -> u64 {
        self.state.lock().unwrap().bytes_sent
    }

    /// Return bytes received counter.
    #[getter]
    fn bytes_received(&self) -> u64 {
        self.state.lock().unwrap().bytes_received
    }

    /// Establish a TCP connection to the configured host:port.
    fn connect(&self, py: Python) -> PyResult<TcpConnectResultPy> {
        let host = self.config.host.clone();
        let port = self.config.port;
        let connect_timeout = std::time::Duration::from_millis(self.config.connect_timeout_ms);
        let nodelay = self.config.nodelay;
        let state = Arc::clone(&self.state);

        {
            let s = state.lock().unwrap();
            if s.is_closed {
                return Err(NetworkError::new_err("Session is closed"));
            }
            if s.stream.is_some() {
                return Err(NetworkError::new_err("Already connected"));
            }
        }

        let connect_start = Instant::now();

        let result = runtime_sync::block_on(py, async move {
            let addr = format!("{}:{}", host, port);
            let stream =
                tokio::time::timeout(connect_timeout, tokio::net::TcpStream::connect(&addr))
                    .await
                    .map_err(|_| {
                        TimeoutError::new_err(format!(
                            "TCP connect timed out after {}ms to {}:{}",
                            connect_timeout.as_millis(),
                            host,
                            port
                        ))
                    })?
                    .map_err(|e| {
                        NetworkError::new_err(format!(
                            "TCP connect failed to {}:{}: {}",
                            host, port, e
                        ))
                    })?;

            stream
                .set_nodelay(nodelay)
                .map_err(|e| NetworkError::new_err(format!("Failed to set TCP_NODELAY: {}", e)))?;

            Ok::<_, PyErr>(stream)
        })?;

        let connect_duration_ms = connect_start.elapsed().as_secs_f64() * 1000.0;

        let local_addr = result
            .local_addr()
            .map_err(|e| NetworkError::new_err(format!("Failed to get local address: {}", e)))?;
        let remote_addr = result
            .peer_addr()
            .map_err(|e| NetworkError::new_err(format!("Failed to get peer address: {}", e)))?;

        let local_endpoint = endpoint_from_socket_addr(local_addr);
        let remote_endpoint = endpoint_from_socket_addr(remote_addr);

        let timing = ConnectionTimingPy {
            dns_resolution_ms: None,
            tcp_connect_ms: Some(connect_duration_ms),
            tls_handshake_ms: None,
            first_byte_ms: None,
            total_ms: connect_duration_ms,
            connection_reused: false,
        };

        let metadata = ConnectionMetadataPy {
            local_endpoint: Some(local_endpoint.clone()),
            remote_endpoint: Some(remote_endpoint.clone()),
            resolved_address: None,
            transport_protocol: "tcp".to_string(),
            negotiated_protocol: None,
            connection_reused: false,
            timing: Some(timing.clone()),
            tls_version: None,
            tls_cipher: None,
            bytes_sent: 0,
            bytes_received: 0,
        };

        {
            let mut s = state.lock().unwrap();
            s.stream = Some(result);
        }

        Ok(TcpConnectResultPy {
            local_endpoint,
            remote_endpoint,
            timing,
            metadata,
        })
    }

    /// Read up to max_bytes from the stream.
    fn read(&self, py: Python, max_bytes: Option<usize>) -> PyResult<TcpReadResultPy> {
        let max_bytes = max_bytes.unwrap_or(4096);
        let state = Arc::clone(&self.state);
        let read_timeout = std::time::Duration::from_millis(self.config.read_timeout_ms);

        {
            let s = state.lock().unwrap();
            if s.is_closed || s.stream.is_none() {
                return Err(NetworkError::new_err("Not connected"));
            }
        }

        let start = Instant::now();
        let (result, buf) = {
            let mut s = state.lock().unwrap();
            let stream = s.stream.as_mut().unwrap();
            let stream_ref = unsafe { &mut *(stream as *mut tokio::net::TcpStream) };

            runtime_sync::block_on(py, async move {
                let mut buf = vec![0u8; max_bytes];
                let n = tokio::time::timeout(read_timeout, stream_ref.read(&mut buf))
                    .await
                    .map_err(|_| {
                        TimeoutError::new_err(format!(
                            "TCP read timed out after {}ms",
                            read_timeout.as_millis()
                        ))
                    })?
                    .map_err(|e| NetworkError::new_err(format!("TCP read error: {}", e)))?;
                Ok::<_, PyErr>((n, buf))
            })?
        };

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        let eof = result == 0;
        let bytes_read = result;

        let mut data = buf;
        data.truncate(bytes_read);

        let mut s = state.lock().unwrap();
        s.bytes_received += bytes_read as u64;
        s.sequence += 1;
        let seq = s.sequence;
        let now_ms = chrono::Utc::now().timestamp_millis() as f64;
        s.transcript.push(TranscriptEntryPy {
            sequence: seq,
            direction: "received".to_string(),
            timestamp_ms: now_ms,
            data_type: "data".to_string(),
            size: bytes_read,
            summary: None,
            redacted: false,
        });

        Ok(TcpReadResultPy {
            data,
            bytes_read,
            eof,
            duration_ms,
        })
    }

    /// Read exactly n bytes from the stream.
    fn read_exact(&self, py: Python, n: usize) -> PyResult<TcpReadResultPy> {
        let state = Arc::clone(&self.state);
        let read_timeout = std::time::Duration::from_millis(self.config.read_timeout_ms);

        {
            let s = state.lock().unwrap();
            if s.is_closed || s.stream.is_none() {
                return Err(NetworkError::new_err("Not connected"));
            }
        }

        let start = Instant::now();
        let buf = {
            let mut s = state.lock().unwrap();
            let stream = s.stream.as_mut().unwrap();
            let stream_ref = unsafe { &mut *(stream as *mut tokio::net::TcpStream) };

            runtime_sync::block_on(py, async move {
                let mut buf = vec![0u8; n];
                tokio::time::timeout(read_timeout, stream_ref.read_exact(&mut buf))
                    .await
                    .map_err(|_| {
                        TimeoutError::new_err(format!(
                            "TCP read_exact timed out after {}ms",
                            read_timeout.as_millis()
                        ))
                    })?
                    .map_err(|e| NetworkError::new_err(format!("TCP read_exact error: {}", e)))?;
                Ok::<_, PyErr>(buf)
            })?
        };

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

        let mut s = state.lock().unwrap();
        s.bytes_received += n as u64;
        s.sequence += 1;
        let seq = s.sequence;
        let now_ms = chrono::Utc::now().timestamp_millis() as f64;
        s.transcript.push(TranscriptEntryPy {
            sequence: seq,
            direction: "received".to_string(),
            timestamp_ms: now_ms,
            data_type: "data".to_string(),
            size: n,
            summary: None,
            redacted: false,
        });

        Ok(TcpReadResultPy {
            data: buf,
            bytes_read: n,
            eof: false,
            duration_ms,
        })
    }

    /// Read until the delimiter byte is encountered or max_len is reached.
    fn read_until(
        &self,
        py: Python,
        delimiter: Option<u8>,
        max_len: Option<usize>,
    ) -> PyResult<TcpReadResultPy> {
        let delimiter = delimiter.unwrap_or(0x0A);
        let max_len = max_len.unwrap_or(65536);
        let state = Arc::clone(&self.state);
        let read_timeout = std::time::Duration::from_millis(self.config.read_timeout_ms);

        {
            let s = state.lock().unwrap();
            if s.is_closed || s.stream.is_none() {
                return Err(NetworkError::new_err("Not connected"));
            }
        }

        let start = Instant::now();
        let (result, buf) = {
            let mut s = state.lock().unwrap();
            let stream = s.stream.as_mut().unwrap();
            let stream_ref = unsafe { &mut *(stream as *mut tokio::net::TcpStream) };

            runtime_sync::block_on(py, async move {
                let mut buf = vec![0u8; max_len];
                let total = tokio::time::timeout(read_timeout, async {
                    let mut total = 0;
                    for byte in buf.iter_mut().take(max_len) {
                        match stream_ref.read(std::slice::from_mut(byte)).await {
                            Ok(0) => break,
                            Ok(_) => {
                                total += 1;
                                if *byte == delimiter {
                                    break;
                                }
                            }
                            Err(e) => return Err(e),
                        }
                    }
                    Ok::<usize, std::io::Error>(total)
                })
                .await
                .map_err(|_| {
                    TimeoutError::new_err(format!(
                        "TCP read_until timed out after {}ms",
                        read_timeout.as_millis()
                    ))
                })?
                .map_err(|e| NetworkError::new_err(format!("TCP read_until error: {}", e)))?;
                Ok::<_, PyErr>((total, buf))
            })?
        };

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        let bytes_read = result;

        let mut data = buf;
        data.truncate(bytes_read);

        let mut s = state.lock().unwrap();
        s.bytes_received += bytes_read as u64;
        s.sequence += 1;
        let seq = s.sequence;
        let now_ms = chrono::Utc::now().timestamp_millis() as f64;
        s.transcript.push(TranscriptEntryPy {
            sequence: seq,
            direction: "received".to_string(),
            timestamp_ms: now_ms,
            data_type: "data".to_string(),
            size: bytes_read,
            summary: None,
            redacted: false,
        });

        Ok(TcpReadResultPy {
            data,
            bytes_read,
            eof: bytes_read == 0,
            duration_ms,
        })
    }

    /// Write data to the stream.
    fn write(&self, py: Python, data: Vec<u8>) -> PyResult<TcpWriteResultPy> {
        let state = Arc::clone(&self.state);
        let write_timeout = std::time::Duration::from_millis(self.config.write_timeout_ms);

        let mut s = state.lock().unwrap();
        let stream = s
            .stream
            .as_mut()
            .ok_or_else(|| NetworkError::new_err("Not connected"))?;
        let stream_ref = unsafe { &mut *(stream as *mut tokio::net::TcpStream) };

        let start = Instant::now();
        let result = runtime_sync::block_on(py, async move {
            tokio::time::timeout(write_timeout, stream_ref.write(&data))
                .await
                .map_err(|_| {
                    TimeoutError::new_err(format!(
                        "TCP write timed out after {}ms",
                        write_timeout.as_millis()
                    ))
                })?
                .map_err(|e| NetworkError::new_err(format!("TCP write error: {}", e)))
        })?;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

        let mut s = state.lock().unwrap();
        s.bytes_sent += result as u64;
        s.sequence += 1;
        let seq = s.sequence;
        let now_ms = chrono::Utc::now().timestamp_millis() as f64;
        s.transcript.push(TranscriptEntryPy {
            sequence: seq,
            direction: "sent".to_string(),
            timestamp_ms: now_ms,
            data_type: "data".to_string(),
            size: result,
            summary: None,
            redacted: false,
        });

        Ok(TcpWriteResultPy {
            bytes_written: result,
            duration_ms,
        })
    }

    /// Write all data to the stream, blocking until complete.
    fn write_all(&self, py: Python, data: Vec<u8>) -> PyResult<TcpWriteResultPy> {
        let state = Arc::clone(&self.state);
        let write_timeout = std::time::Duration::from_millis(self.config.write_timeout_ms);
        let len = data.len();

        let mut s = state.lock().unwrap();
        let stream = s
            .stream
            .as_mut()
            .ok_or_else(|| NetworkError::new_err("Not connected"))?;
        let stream_ref = unsafe { &mut *(stream as *mut tokio::net::TcpStream) };

        let start = Instant::now();
        runtime_sync::block_on(py, async move {
            tokio::time::timeout(write_timeout, stream_ref.write_all(&data))
                .await
                .map_err(|_| {
                    TimeoutError::new_err(format!(
                        "TCP write_all timed out after {}ms",
                        write_timeout.as_millis()
                    ))
                })?
                .map_err(|e| NetworkError::new_err(format!("TCP write_all error: {}", e)))
        })?;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

        let mut s = state.lock().unwrap();
        s.bytes_sent += len as u64;
        s.sequence += 1;
        let seq = s.sequence;
        let now_ms = chrono::Utc::now().timestamp_millis() as f64;
        s.transcript.push(TranscriptEntryPy {
            sequence: seq,
            direction: "sent".to_string(),
            timestamp_ms: now_ms,
            data_type: "data".to_string(),
            size: len,
            summary: None,
            redacted: false,
        });

        Ok(TcpWriteResultPy {
            bytes_written: len,
            duration_ms,
        })
    }

    /// Close the TCP session deterministically.
    fn close(&self) -> PyResult<()> {
        let mut s = self.state.lock().unwrap();
        if s.is_closed {
            return Ok(());
        }
        s.stream.take();
        s.is_closed = true;
        Ok(())
    }

    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        let _ = self.close();
        false
    }

    fn __repr__(&self) -> String {
        let s = self.state.lock().unwrap();
        format!(
            "TcpSession(host={}, port={}, closed={}, sent={}, received={})",
            self.config.host, self.config.port, s.is_closed, s.bytes_sent, s.bytes_received
        )
    }

    fn __str__(&self) -> String {
        let s = self.state.lock().unwrap();
        if s.is_closed {
            format!(
                "tcp://{}:{} (closed, {} transcripts)",
                self.config.host,
                self.config.port,
                s.transcript.len()
            )
        } else {
            format!(
                "tcp://{}:{} (open, {} transcripts, sent={}B recv={}B)",
                self.config.host,
                self.config.port,
                s.transcript.len(),
                s.bytes_sent,
                s.bytes_received
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// UDP Configuration
// ═══════════════════════════════════════════════════════════════════

/// UDP socket configuration.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpConfigPy {
    #[pyo3(get)]
    pub host: String,
    #[pyo3(get)]
    pub port: u16,
    #[pyo3(get)]
    pub timeout_ms: u64,
    #[pyo3(get)]
    pub max_datagram_size: usize,
    #[pyo3(get)]
    pub bind_address: Option<String>,
}

#[pymethods]
impl UdpConfigPy {
    #[new]
    #[pyo3(signature = (host, port, timeout_ms=5000, max_datagram_size=65535, bind_address=None))]
    fn new(
        host: String,
        port: u16,
        timeout_ms: u64,
        max_datagram_size: usize,
        bind_address: Option<String>,
    ) -> Self {
        Self {
            host,
            port,
            timeout_ms,
            max_datagram_size,
            bind_address,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("host", &self.host)?;
        dict.set_item("port", self.port)?;
        dict.set_item("timeout_ms", self.timeout_ms)?;
        dict.set_item("max_datagram_size", self.max_datagram_size)?;
        dict.set_item("bind_address", &self.bind_address)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "UdpConfig(host={}, port={}, timeout={}ms, max_dgram={})",
            self.host, self.port, self.timeout_ms, self.max_datagram_size
        )
    }

    fn __str__(&self) -> String {
        format!(
            "udp://{}:{} timeout={}ms max_datagram={}B",
            self.host, self.port, self.timeout_ms, self.max_datagram_size
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// UDP Send Result
// ═══════════════════════════════════════════════════════════════════

/// Result of a UDP send operation.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpSendResultPy {
    #[pyo3(get)]
    pub bytes_sent: usize,
    #[pyo3(get)]
    pub duration_ms: f64,
}

#[pymethods]
impl UdpSendResultPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("bytes_sent", self.bytes_sent)?;
        dict.set_item("duration_ms", self.duration_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "UdpSendResult(bytes={}, duration={:.1}ms)",
            self.bytes_sent, self.duration_ms
        )
    }

    fn __str__(&self) -> String {
        format!("{} bytes in {:.1}ms", self.bytes_sent, self.duration_ms)
    }
}

// ═══════════════════════════════════════════════════════════════════
// UDP Recv Result
// ═══════════════════════════════════════════════════════════════════

/// Result of a UDP receive operation.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpRecvResultPy {
    #[pyo3(get)]
    pub data: Vec<u8>,
    #[pyo3(get)]
    pub bytes_received: usize,
    #[pyo3(get)]
    pub truncated: bool,
    #[pyo3(get)]
    pub duration_ms: f64,
}

#[pymethods]
impl UdpRecvResultPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("data", &self.data)?;
        dict.set_item("bytes_received", self.bytes_received)?;
        dict.set_item("truncated", self.truncated)?;
        dict.set_item("duration_ms", self.duration_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "UdpRecvResult(bytes={}, truncated={}, duration={:.1}ms)",
            self.bytes_received, self.truncated, self.duration_ms
        )
    }

    fn __str__(&self) -> String {
        if self.truncated {
            format!("{} bytes (truncated)", self.bytes_received)
        } else {
            format!("{} bytes", self.bytes_received)
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// UDP Recv From Result
// ═══════════════════════════════════════════════════════════════════

/// Result of a UDP receive-from operation (includes source address).
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpRecvFromResultPy {
    #[pyo3(get)]
    pub data: Vec<u8>,
    #[pyo3(get)]
    pub bytes_received: usize,
    #[pyo3(get)]
    pub source_address: String,
    #[pyo3(get)]
    pub source_port: u16,
    #[pyo3(get)]
    pub truncated: bool,
    #[pyo3(get)]
    pub duration_ms: f64,
}

#[pymethods]
impl UdpRecvFromResultPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("data", &self.data)?;
        dict.set_item("bytes_received", self.bytes_received)?;
        dict.set_item("source_address", &self.source_address)?;
        dict.set_item("source_port", self.source_port)?;
        dict.set_item("truncated", self.truncated)?;
        dict.set_item("duration_ms", self.duration_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "UdpRecvFromResult(bytes={}, from={}:{}, truncated={}, duration={:.1}ms)",
            self.bytes_received,
            self.source_address,
            self.source_port,
            self.truncated,
            self.duration_ms
        )
    }

    fn __str__(&self) -> String {
        format!(
            "{} bytes from {}:{}{}",
            self.bytes_received,
            self.source_address,
            self.source_port,
            if self.truncated { " (truncated)" } else { "" }
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// UDP Socket (mutable)
// ═══════════════════════════════════════════════════════════════════

struct UdpSocketState {
    socket: Option<tokio::net::UdpSocket>,
    is_closed: bool,
    bytes_sent: u64,
    bytes_received: u64,
    transcript: Vec<TranscriptEntryPy>,
    sequence: u64,
}

/// Managed UDP socket with send/recv operations.
///
/// Supports context manager protocol for safe resource cleanup.
#[pyclass]
pub struct UdpSocketPy {
    config: UdpConfigPy,
    state: Arc<std::sync::Mutex<UdpSocketState>>,
}

#[pymethods]
impl UdpSocketPy {
    #[new]
    #[pyo3(signature = (config))]
    fn new(config: UdpConfigPy) -> Self {
        Self {
            config,
            state: Arc::new(std::sync::Mutex::new(UdpSocketState {
                socket: None,
                is_closed: false,
                bytes_sent: 0,
                bytes_received: 0,
                transcript: Vec::new(),
                sequence: 0,
            })),
        }
    }

    #[getter]
    fn is_closed(&self) -> bool {
        self.state.lock().unwrap().is_closed
    }

    /// Return bytes sent counter.
    #[getter]
    fn bytes_sent(&self) -> u64 {
        self.state.lock().unwrap().bytes_sent
    }

    /// Return bytes received counter.
    #[getter]
    fn bytes_received(&self) -> u64 {
        self.state.lock().unwrap().bytes_received
    }

    /// Return the transcript of all send/recv operations.
    #[getter]
    fn transcript(&self) -> NetworkTranscriptPy {
        let s = self.state.lock().unwrap();
        let total_bytes = s.bytes_sent + s.bytes_received;
        NetworkTranscriptPy {
            entries: s.transcript.clone(),
            total_bytes,
            truncated: false,
        }
    }

    /// Connect the UDP socket to the target host:port.
    fn connect(&self, py: Python) -> PyResult<()> {
        let state = Arc::clone(&self.state);
        let bind_addr = self.config.bind_address.clone();
        let target_addr = format!("{}:{}", self.config.host, self.config.port);
        let timeout = std::time::Duration::from_millis(self.config.timeout_ms);

        {
            let s = state.lock().unwrap();
            if s.is_closed {
                return Err(NetworkError::new_err("Socket is closed"));
            }
            if s.socket.is_some() {
                return Err(NetworkError::new_err("Already connected"));
            }
        }

        let socket = runtime_sync::block_on(py, async move {
            let socket = if let Some(ref bind) = bind_addr {
                tokio::net::UdpSocket::bind(bind).await.map_err(|e| {
                    NetworkError::new_err(format!("UDP bind failed to {}: {}", bind, e))
                })?
            } else {
                tokio::net::UdpSocket::bind("0.0.0.0:0")
                    .await
                    .map_err(|e| NetworkError::new_err(format!("UDP bind failed: {}", e)))?
            };

            tokio::time::timeout(timeout, socket.connect(&target_addr))
                .await
                .map_err(|_| {
                    TimeoutError::new_err(format!(
                        "UDP connect timed out after {}ms to {}",
                        timeout.as_millis(),
                        target_addr
                    ))
                })?
                .map_err(|e| {
                    NetworkError::new_err(format!("UDP connect failed to {}: {}", target_addr, e))
                })?;

            Ok::<_, PyErr>(socket)
        })?;

        let mut s = state.lock().unwrap();
        s.socket = Some(socket);
        Ok(())
    }

    /// Send a datagram to the connected target.
    fn send(&self, py: Python, data: Vec<u8>) -> PyResult<UdpSendResultPy> {
        let state = Arc::clone(&self.state);
        let timeout = std::time::Duration::from_millis(self.config.timeout_ms);

        let s = state.lock().unwrap();
        let socket = s
            .socket
            .as_ref()
            .ok_or_else(|| NetworkError::new_err("Not connected"))?;
        let socket_ref = unsafe { &*(socket as *const tokio::net::UdpSocket) };

        let start = Instant::now();
        let result = runtime_sync::block_on(py, async move {
            tokio::time::timeout(timeout, socket_ref.send(&data))
                .await
                .map_err(|_| {
                    TimeoutError::new_err(format!(
                        "UDP send timed out after {}ms",
                        timeout.as_millis()
                    ))
                })?
                .map_err(|e| NetworkError::new_err(format!("UDP send error: {}", e)))
        })?;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

        let mut s = state.lock().unwrap();
        s.bytes_sent += result as u64;
        s.sequence += 1;
        let seq = s.sequence;
        let now_ms = chrono::Utc::now().timestamp_millis() as f64;
        s.transcript.push(TranscriptEntryPy {
            sequence: seq,
            direction: "sent".to_string(),
            timestamp_ms: now_ms,
            data_type: "datagram".to_string(),
            size: result,
            summary: None,
            redacted: false,
        });

        Ok(UdpSendResultPy {
            bytes_sent: result,
            duration_ms,
        })
    }

    /// Send a datagram to a specific address.
    fn send_to(&self, py: Python, data: Vec<u8>, addr: &str) -> PyResult<UdpSendResultPy> {
        let state = Arc::clone(&self.state);
        let timeout = std::time::Duration::from_millis(self.config.timeout_ms);
        let addr_owned = addr.to_string();

        let s = state.lock().unwrap();
        let socket = s
            .socket
            .as_ref()
            .ok_or_else(|| NetworkError::new_err("Not connected"))?;
        let socket_ref = unsafe { &*(socket as *const tokio::net::UdpSocket) };

        let start = Instant::now();
        let result = runtime_sync::block_on(py, async move {
            tokio::time::timeout(timeout, socket_ref.send_to(&data, &addr_owned))
                .await
                .map_err(|_| {
                    TimeoutError::new_err(format!(
                        "UDP send_to timed out after {}ms",
                        timeout.as_millis()
                    ))
                })?
                .map_err(|e| NetworkError::new_err(format!("UDP send_to error: {}", e)))
        })?;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

        let mut s = state.lock().unwrap();
        s.bytes_sent += result as u64;
        s.sequence += 1;
        let seq = s.sequence;
        let now_ms = chrono::Utc::now().timestamp_millis() as f64;
        s.transcript.push(TranscriptEntryPy {
            sequence: seq,
            direction: "sent".to_string(),
            timestamp_ms: now_ms,
            data_type: "datagram".to_string(),
            size: result,
            summary: None,
            redacted: false,
        });

        Ok(UdpSendResultPy {
            bytes_sent: result,
            duration_ms,
        })
    }

    /// Receive a datagram from the connected target.
    fn recv(&self, py: Python) -> PyResult<UdpRecvResultPy> {
        let state = Arc::clone(&self.state);
        let timeout = std::time::Duration::from_millis(self.config.timeout_ms);
        let max_size = self.config.max_datagram_size;

        let s = state.lock().unwrap();
        let socket = s
            .socket
            .as_ref()
            .ok_or_else(|| NetworkError::new_err("Not connected"))?;
        let socket_ref = unsafe { &*(socket as *const tokio::net::UdpSocket) };

        let start = Instant::now();
        let (result, buf) = runtime_sync::block_on(py, async move {
            let mut buf = vec![0u8; max_size];
            let n = tokio::time::timeout(timeout, socket_ref.recv(&mut buf))
                .await
                .map_err(|_| {
                    TimeoutError::new_err(format!(
                        "UDP recv timed out after {}ms",
                        timeout.as_millis()
                    ))
                })?
                .map_err(|e| NetworkError::new_err(format!("UDP recv error: {}", e)))?;
            Ok::<_, PyErr>((n, buf))
        })?;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        let truncated = result == max_size;

        let mut data = buf;
        data.truncate(result);

        let mut s = state.lock().unwrap();
        s.bytes_received += result as u64;
        s.sequence += 1;
        let seq = s.sequence;
        let now_ms = chrono::Utc::now().timestamp_millis() as f64;
        s.transcript.push(TranscriptEntryPy {
            sequence: seq,
            direction: "received".to_string(),
            timestamp_ms: now_ms,
            data_type: "datagram".to_string(),
            size: result,
            summary: None,
            redacted: false,
        });

        Ok(UdpRecvResultPy {
            data,
            bytes_received: result,
            truncated,
            duration_ms,
        })
    }

    /// Receive a datagram and return the source address.
    fn recv_from(&self, py: Python) -> PyResult<UdpRecvFromResultPy> {
        let state = Arc::clone(&self.state);
        let timeout = std::time::Duration::from_millis(self.config.timeout_ms);
        let max_size = self.config.max_datagram_size;

        let s = state.lock().unwrap();
        let socket = s
            .socket
            .as_ref()
            .ok_or_else(|| NetworkError::new_err("Not connected"))?;
        let socket_ref = unsafe { &*(socket as *const tokio::net::UdpSocket) };

        let start = Instant::now();
        let (result, buf, from_addr) = runtime_sync::block_on(py, async move {
            let mut buf = vec![0u8; max_size];
            let (n, from) = tokio::time::timeout(timeout, socket_ref.recv_from(&mut buf))
                .await
                .map_err(|_| {
                    TimeoutError::new_err(format!(
                        "UDP recv_from timed out after {}ms",
                        timeout.as_millis()
                    ))
                })?
                .map_err(|e| NetworkError::new_err(format!("UDP recv_from error: {}", e)))?;
            Ok::<_, PyErr>((n, buf, from))
        })?;

        let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
        let truncated = result == max_size;

        let mut data = buf;
        data.truncate(result);

        let source_address = from_addr.ip().to_string();
        let source_port = from_addr.port();

        let mut s = state.lock().unwrap();
        s.bytes_received += result as u64;
        s.sequence += 1;
        let seq = s.sequence;
        let now_ms = chrono::Utc::now().timestamp_millis() as f64;
        s.transcript.push(TranscriptEntryPy {
            sequence: seq,
            direction: "received".to_string(),
            timestamp_ms: now_ms,
            data_type: "datagram".to_string(),
            size: result,
            summary: Some(format!("from {}:{}", source_address, source_port)),
            redacted: false,
        });

        Ok(UdpRecvFromResultPy {
            data,
            bytes_received: result,
            source_address,
            source_port,
            truncated,
            duration_ms,
        })
    }

    /// Close the UDP socket deterministically.
    fn close(&self) -> PyResult<()> {
        let mut s = self.state.lock().unwrap();
        if s.is_closed {
            return Ok(());
        }
        s.socket.take();
        s.is_closed = true;
        Ok(())
    }

    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        let _ = self.close();
        false
    }

    fn __repr__(&self) -> String {
        let s = self.state.lock().unwrap();
        format!(
            "UdpSocket(host={}, port={}, closed={})",
            self.config.host, self.config.port, s.is_closed
        )
    }

    fn __str__(&self) -> String {
        let s = self.state.lock().unwrap();
        if s.is_closed {
            format!("udp://{}:{} (closed)", self.config.host, self.config.port)
        } else {
            format!("udp://{}:{} (open)", self.config.host, self.config.port)
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Async TCP Session (mutable, GIL-releasing)
// ═══════════════════════════════════════════════════════════════════

/// Internal state for an async TCP session.
struct AsyncTcpSessionState {
    stream: Option<tokio::net::TcpStream>,
    is_closed: bool,
    transcript: Vec<TranscriptEntryPy>,
    bytes_sent: u64,
    bytes_received: u64,
    sequence: u64,
}

/// Async managed TCP session with transcript tracking.
///
/// Uses `Arc<tokio::sync::Mutex<>>` for internal state so the mutex
/// can be held across `.await` points. All I/O methods return a
/// `PyFuture` that can be `await`-ed from Python and release the GIL
/// during the async operation.
#[pyclass]
pub struct AsyncTcpSessionPy {
    config: TcpConfigPy,
    state: Arc<tokio::sync::Mutex<AsyncTcpSessionState>>,
}

#[pymethods]
impl AsyncTcpSessionPy {
    #[new]
    #[pyo3(signature = (config))]
    fn new(config: TcpConfigPy) -> Self {
        Self {
            config,
            state: Arc::new(tokio::sync::Mutex::new(AsyncTcpSessionState {
                stream: None,
                is_closed: false,
                transcript: Vec::new(),
                bytes_sent: 0,
                bytes_received: 0,
                sequence: 0,
            })),
        }
    }

    #[getter]
    fn is_closed(&self) -> bool {
        // Blocking lock is fine here — we never hold it across .await
        self.state.blocking_lock().is_closed
    }

    #[getter]
    fn config(&self) -> TcpConfigPy {
        self.config.clone()
    }

    #[getter]
    fn transcript(&self) -> NetworkTranscriptPy {
        let s = self.state.blocking_lock();
        let total_bytes = s.bytes_sent + s.bytes_received;
        NetworkTranscriptPy {
            entries: s.transcript.clone(),
            total_bytes,
            truncated: false,
        }
    }

    #[getter]
    fn bytes_sent(&self) -> u64 {
        self.state.blocking_lock().bytes_sent
    }

    #[getter]
    fn bytes_received(&self) -> u64 {
        self.state.blocking_lock().bytes_received
    }

    /// Establish a TCP connection to the configured host:port.
    fn connect(&self, py: Python) -> PyResult<runtime_async::PyFuture> {
        let state = Arc::clone(&self.state);
        let host = self.config.host.clone();
        let port = self.config.port;
        let connect_timeout = std::time::Duration::from_millis(self.config.connect_timeout_ms);
        let nodelay = self.config.nodelay;

        {
            let s = state.blocking_lock();
            if s.is_closed {
                return Err(NetworkError::new_err("Session is closed"));
            }
            if s.stream.is_some() {
                return Err(NetworkError::new_err("Already connected"));
            }
        }

        py.allow_threads(|| {
            runtime_async::spawn_async(async move {
                let addr = format!("{}:{}", host, port);
                let connect_start = Instant::now();

                let stream =
                    tokio::time::timeout(connect_timeout, tokio::net::TcpStream::connect(&addr))
                        .await
                        .map_err(|_| {
                            TimeoutError::new_err(format!(
                                "TCP connect timed out after {}ms to {}:{}",
                                connect_timeout.as_millis(),
                                host,
                                port
                            ))
                        })?
                        .map_err(|e| {
                            NetworkError::new_err(format!(
                                "TCP connect failed to {}:{}: {}",
                                host, port, e
                            ))
                        })?;

                stream.set_nodelay(nodelay).map_err(|e| {
                    NetworkError::new_err(format!("Failed to set TCP_NODELAY: {}", e))
                })?;

                let connect_duration_ms = connect_start.elapsed().as_secs_f64() * 1000.0;

                let local_addr = stream.local_addr().map_err(|e| {
                    NetworkError::new_err(format!("Failed to get local address: {}", e))
                })?;
                let remote_addr = stream.peer_addr().map_err(|e| {
                    NetworkError::new_err(format!("Failed to get peer address: {}", e))
                })?;

                let local_endpoint = endpoint_from_socket_addr(local_addr);
                let remote_endpoint = endpoint_from_socket_addr(remote_addr);

                let timing = ConnectionTimingPy {
                    dns_resolution_ms: None,
                    tcp_connect_ms: Some(connect_duration_ms),
                    tls_handshake_ms: None,
                    first_byte_ms: None,
                    total_ms: connect_duration_ms,
                    connection_reused: false,
                };

                let metadata = ConnectionMetadataPy {
                    local_endpoint: Some(local_endpoint.clone()),
                    remote_endpoint: Some(remote_endpoint.clone()),
                    resolved_address: None,
                    transport_protocol: "tcp".to_string(),
                    negotiated_protocol: None,
                    connection_reused: false,
                    timing: Some(timing.clone()),
                    tls_version: None,
                    tls_cipher: None,
                    bytes_sent: 0,
                    bytes_received: 0,
                };

                {
                    let mut s = state.lock().await;
                    s.stream = Some(stream);
                }

                Ok(TcpConnectResultPy {
                    local_endpoint,
                    remote_endpoint,
                    timing,
                    metadata,
                })
            })
        })
    }

    /// Read up to max_bytes from the stream.
    fn read(&self, py: Python, max_bytes: Option<usize>) -> PyResult<runtime_async::PyFuture> {
        let max_bytes = max_bytes.unwrap_or(4096);
        let state = Arc::clone(&self.state);
        let read_timeout = std::time::Duration::from_millis(self.config.read_timeout_ms);

        {
            let s = state.blocking_lock();
            if s.is_closed || s.stream.is_none() {
                return Err(NetworkError::new_err("Not connected"));
            }
        }

        py.allow_threads(|| {
            runtime_async::spawn_async(async move {
                let start = Instant::now();
                let mut s = state.lock().await;
                let stream = s
                    .stream
                    .as_mut()
                    .ok_or_else(|| NetworkError::new_err("Not connected"))?;

                let mut buf = vec![0u8; max_bytes];
                let n = tokio::time::timeout(read_timeout, stream.read(&mut buf))
                    .await
                    .map_err(|_| {
                        TimeoutError::new_err(format!(
                            "TCP read timed out after {}ms",
                            read_timeout.as_millis()
                        ))
                    })?
                    .map_err(|e| NetworkError::new_err(format!("TCP read error: {}", e)))?;

                let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
                let eof = n == 0;
                let bytes_read = n;

                buf.truncate(bytes_read);
                s.bytes_received += bytes_read as u64;
                s.sequence += 1;
                let seq = s.sequence;
                let now_ms = chrono::Utc::now().timestamp_millis() as f64;
                s.transcript.push(TranscriptEntryPy {
                    sequence: seq,
                    direction: "received".to_string(),
                    timestamp_ms: now_ms,
                    data_type: "data".to_string(),
                    size: bytes_read,
                    summary: None,
                    redacted: false,
                });

                Ok(TcpReadResultPy {
                    data: buf,
                    bytes_read,
                    eof,
                    duration_ms,
                })
            })
        })
    }

    /// Read exactly n bytes from the stream.
    fn read_exact(&self, py: Python, n: usize) -> PyResult<runtime_async::PyFuture> {
        let state = Arc::clone(&self.state);
        let read_timeout = std::time::Duration::from_millis(self.config.read_timeout_ms);

        {
            let s = state.blocking_lock();
            if s.is_closed || s.stream.is_none() {
                return Err(NetworkError::new_err("Not connected"));
            }
        }

        py.allow_threads(|| {
            runtime_async::spawn_async(async move {
                let start = Instant::now();
                let mut s = state.lock().await;
                let stream = s
                    .stream
                    .as_mut()
                    .ok_or_else(|| NetworkError::new_err("Not connected"))?;

                let mut buf = vec![0u8; n];
                tokio::time::timeout(read_timeout, stream.read_exact(&mut buf))
                    .await
                    .map_err(|_| {
                        TimeoutError::new_err(format!(
                            "TCP read_exact timed out after {}ms",
                            read_timeout.as_millis()
                        ))
                    })?
                    .map_err(|e| NetworkError::new_err(format!("TCP read_exact error: {}", e)))?;

                let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

                s.bytes_received += n as u64;
                s.sequence += 1;
                let seq = s.sequence;
                let now_ms = chrono::Utc::now().timestamp_millis() as f64;
                s.transcript.push(TranscriptEntryPy {
                    sequence: seq,
                    direction: "received".to_string(),
                    timestamp_ms: now_ms,
                    data_type: "data".to_string(),
                    size: n,
                    summary: None,
                    redacted: false,
                });

                Ok(TcpReadResultPy {
                    data: buf,
                    bytes_read: n,
                    eof: false,
                    duration_ms,
                })
            })
        })
    }

    /// Read until the delimiter byte is encountered or max_len is reached.
    fn read_until(
        &self,
        py: Python,
        delimiter: Option<u8>,
        max_len: Option<usize>,
    ) -> PyResult<runtime_async::PyFuture> {
        let delimiter = delimiter.unwrap_or(0x0A);
        let max_len = max_len.unwrap_or(65536);
        let state = Arc::clone(&self.state);
        let read_timeout = std::time::Duration::from_millis(self.config.read_timeout_ms);

        {
            let s = state.blocking_lock();
            if s.is_closed || s.stream.is_none() {
                return Err(NetworkError::new_err("Not connected"));
            }
        }

        py.allow_threads(|| {
            runtime_async::spawn_async(async move {
                let start = Instant::now();
                let mut s = state.lock().await;
                let stream = s
                    .stream
                    .as_mut()
                    .ok_or_else(|| NetworkError::new_err("Not connected"))?;

                let mut buf = vec![0u8; max_len];
                let total = tokio::time::timeout(read_timeout, async {
                    let mut total = 0;
                    for byte in buf.iter_mut().take(max_len) {
                        match stream.read(std::slice::from_mut(byte)).await {
                            Ok(0) => break,
                            Ok(_) => {
                                total += 1;
                                if *byte == delimiter {
                                    break;
                                }
                            }
                            Err(e) => return Err(e),
                        }
                    }
                    Ok::<usize, std::io::Error>(total)
                })
                .await
                .map_err(|_| {
                    TimeoutError::new_err(format!(
                        "TCP read_until timed out after {}ms",
                        read_timeout.as_millis()
                    ))
                })?
                .map_err(|e| NetworkError::new_err(format!("TCP read_until error: {}", e)))?;

                let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
                let bytes_read = total;

                buf.truncate(bytes_read);
                s.bytes_received += bytes_read as u64;
                s.sequence += 1;
                let seq = s.sequence;
                let now_ms = chrono::Utc::now().timestamp_millis() as f64;
                s.transcript.push(TranscriptEntryPy {
                    sequence: seq,
                    direction: "received".to_string(),
                    timestamp_ms: now_ms,
                    data_type: "data".to_string(),
                    size: bytes_read,
                    summary: None,
                    redacted: false,
                });

                Ok(TcpReadResultPy {
                    data: buf,
                    bytes_read,
                    eof: bytes_read == 0,
                    duration_ms,
                })
            })
        })
    }

    /// Write data to the stream.
    fn write(&self, py: Python, data: Vec<u8>) -> PyResult<runtime_async::PyFuture> {
        let state = Arc::clone(&self.state);
        let write_timeout = std::time::Duration::from_millis(self.config.write_timeout_ms);

        {
            let s = state.blocking_lock();
            if s.is_closed || s.stream.is_none() {
                return Err(NetworkError::new_err("Not connected"));
            }
        }

        py.allow_threads(|| {
            runtime_async::spawn_async(async move {
                let start = Instant::now();
                let mut s = state.lock().await;
                let stream = s
                    .stream
                    .as_mut()
                    .ok_or_else(|| NetworkError::new_err("Not connected"))?;

                let result = tokio::time::timeout(write_timeout, stream.write(&data))
                    .await
                    .map_err(|_| {
                        TimeoutError::new_err(format!(
                            "TCP write timed out after {}ms",
                            write_timeout.as_millis()
                        ))
                    })?
                    .map_err(|e| NetworkError::new_err(format!("TCP write error: {}", e)))?;

                let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

                s.bytes_sent += result as u64;
                s.sequence += 1;
                let seq = s.sequence;
                let now_ms = chrono::Utc::now().timestamp_millis() as f64;
                s.transcript.push(TranscriptEntryPy {
                    sequence: seq,
                    direction: "sent".to_string(),
                    timestamp_ms: now_ms,
                    data_type: "data".to_string(),
                    size: result,
                    summary: None,
                    redacted: false,
                });

                Ok(TcpWriteResultPy {
                    bytes_written: result,
                    duration_ms,
                })
            })
        })
    }

    /// Write all data to the stream.
    fn write_all(&self, py: Python, data: Vec<u8>) -> PyResult<runtime_async::PyFuture> {
        let state = Arc::clone(&self.state);
        let write_timeout = std::time::Duration::from_millis(self.config.write_timeout_ms);
        let len = data.len();

        {
            let s = state.blocking_lock();
            if s.is_closed || s.stream.is_none() {
                return Err(NetworkError::new_err("Not connected"));
            }
        }

        py.allow_threads(|| {
            runtime_async::spawn_async(async move {
                let start = Instant::now();
                let mut s = state.lock().await;
                let stream = s
                    .stream
                    .as_mut()
                    .ok_or_else(|| NetworkError::new_err("Not connected"))?;

                tokio::time::timeout(write_timeout, stream.write_all(&data))
                    .await
                    .map_err(|_| {
                        TimeoutError::new_err(format!(
                            "TCP write_all timed out after {}ms",
                            write_timeout.as_millis()
                        ))
                    })?
                    .map_err(|e| NetworkError::new_err(format!("TCP write_all error: {}", e)))?;

                let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

                s.bytes_sent += len as u64;
                s.sequence += 1;
                let seq = s.sequence;
                let now_ms = chrono::Utc::now().timestamp_millis() as f64;
                s.transcript.push(TranscriptEntryPy {
                    sequence: seq,
                    direction: "sent".to_string(),
                    timestamp_ms: now_ms,
                    data_type: "data".to_string(),
                    size: len,
                    summary: None,
                    redacted: false,
                });

                Ok(TcpWriteResultPy {
                    bytes_written: len,
                    duration_ms,
                })
            })
        })
    }

    /// Close the TCP session.
    fn close(&self) -> PyResult<()> {
        let mut s = self.state.blocking_lock();
        if s.is_closed {
            return Ok(());
        }
        s.stream.take();
        s.is_closed = true;
        Ok(())
    }

    fn aclose(&self) -> PyResult<()> {
        self.close()
    }

    fn __aenter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __aexit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        let _ = self.close();
        false
    }

    fn __enter__(_slf: Py<Self>) -> PyResult<Py<Self>> {
        Err(pyo3::exceptions::PyTypeError::new_err(
            "Use 'async with' for AsyncTcpSession",
        ))
    }

    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        Err(pyo3::exceptions::PyTypeError::new_err(
            "Use 'async with' for AsyncTcpSession",
        ))
    }

    fn __repr__(&self) -> String {
        let s = self.state.blocking_lock();
        format!(
            "AsyncTcpSession(host={}, port={}, closed={}, sent={}, received={})",
            self.config.host, self.config.port, s.is_closed, s.bytes_sent, s.bytes_received
        )
    }

    fn __str__(&self) -> String {
        let s = self.state.blocking_lock();
        if s.is_closed {
            format!(
                "async tcp://{}:{} (closed, {} transcripts)",
                self.config.host,
                self.config.port,
                s.transcript.len()
            )
        } else {
            format!(
                "async tcp://{}:{} (open, {} transcripts, sent={}B recv={}B)",
                self.config.host,
                self.config.port,
                s.transcript.len(),
                s.bytes_sent,
                s.bytes_received
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Async UDP Socket (mutable, GIL-releasing)
// ═══════════════════════════════════════════════════════════════════

/// Internal state for an async UDP socket.
struct AsyncUdpSocketState {
    socket: Option<tokio::net::UdpSocket>,
    is_closed: bool,
    bytes_sent: u64,
    bytes_received: u64,
    transcript: Vec<TranscriptEntryPy>,
    sequence: u64,
}

/// Async managed UDP socket with send/recv operations.
///
/// Uses `Arc<tokio::sync::Mutex<>>` for internal state so the mutex
/// can be held across `.await` points. All I/O methods return a
/// `PyFuture` that can be `await`-ed from Python and release the GIL
/// during the async operation.
#[pyclass]
pub struct AsyncUdpSocketPy {
    config: UdpConfigPy,
    state: Arc<tokio::sync::Mutex<AsyncUdpSocketState>>,
}

#[pymethods]
impl AsyncUdpSocketPy {
    #[new]
    #[pyo3(signature = (config))]
    fn new(config: UdpConfigPy) -> Self {
        Self {
            config,
            state: Arc::new(tokio::sync::Mutex::new(AsyncUdpSocketState {
                socket: None,
                is_closed: false,
                bytes_sent: 0,
                bytes_received: 0,
                transcript: Vec::new(),
                sequence: 0,
            })),
        }
    }

    #[getter]
    fn is_closed(&self) -> bool {
        self.state.blocking_lock().is_closed
    }

    #[getter]
    fn bytes_sent(&self) -> u64 {
        self.state.blocking_lock().bytes_sent
    }

    #[getter]
    fn bytes_received(&self) -> u64 {
        self.state.blocking_lock().bytes_received
    }

    #[getter]
    fn transcript(&self) -> NetworkTranscriptPy {
        let s = self.state.blocking_lock();
        let total_bytes = s.bytes_sent + s.bytes_received;
        NetworkTranscriptPy {
            entries: s.transcript.clone(),
            total_bytes,
            truncated: false,
        }
    }

    /// Connect the UDP socket to the target host:port.
    fn connect(&self, py: Python) -> PyResult<runtime_async::PyFuture> {
        let state = Arc::clone(&self.state);
        let bind_addr = self.config.bind_address.clone();
        let target_addr = format!("{}:{}", self.config.host, self.config.port);
        let timeout = std::time::Duration::from_millis(self.config.timeout_ms);

        {
            let s = state.blocking_lock();
            if s.is_closed {
                return Err(NetworkError::new_err("Socket is closed"));
            }
            if s.socket.is_some() {
                return Err(NetworkError::new_err("Already connected"));
            }
        }

        py.allow_threads(|| {
            runtime_async::spawn_async(async move {
                let socket = if let Some(ref bind) = bind_addr {
                    tokio::net::UdpSocket::bind(bind).await.map_err(|e| {
                        NetworkError::new_err(format!("UDP bind failed to {}: {}", bind, e))
                    })?
                } else {
                    tokio::net::UdpSocket::bind("0.0.0.0:0")
                        .await
                        .map_err(|e| NetworkError::new_err(format!("UDP bind failed: {}", e)))?
                };

                tokio::time::timeout(timeout, socket.connect(&target_addr))
                    .await
                    .map_err(|_| {
                        TimeoutError::new_err(format!(
                            "UDP connect timed out after {}ms to {}",
                            timeout.as_millis(),
                            target_addr
                        ))
                    })?
                    .map_err(|e| {
                        NetworkError::new_err(format!(
                            "UDP connect failed to {}: {}",
                            target_addr, e
                        ))
                    })?;

                let mut s = state.lock().await;
                s.socket = Some(socket);
                Ok(())
            })
        })
    }

    /// Send a datagram to the connected target.
    fn send(&self, py: Python, data: Vec<u8>) -> PyResult<runtime_async::PyFuture> {
        let state = Arc::clone(&self.state);
        let timeout = std::time::Duration::from_millis(self.config.timeout_ms);

        {
            let s = state.blocking_lock();
            if s.is_closed || s.socket.is_none() {
                return Err(NetworkError::new_err("Not connected"));
            }
        }

        py.allow_threads(|| {
            runtime_async::spawn_async(async move {
                let start = Instant::now();
                let mut s = state.lock().await;
                let socket = s
                    .socket
                    .as_ref()
                    .ok_or_else(|| NetworkError::new_err("Not connected"))?;

                let result = tokio::time::timeout(timeout, socket.send(&data))
                    .await
                    .map_err(|_| {
                        TimeoutError::new_err(format!(
                            "UDP send timed out after {}ms",
                            timeout.as_millis()
                        ))
                    })?
                    .map_err(|e| NetworkError::new_err(format!("UDP send error: {}", e)))?;

                let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

                s.bytes_sent += result as u64;
                s.sequence += 1;
                let seq = s.sequence;
                let now_ms = chrono::Utc::now().timestamp_millis() as f64;
                s.transcript.push(TranscriptEntryPy {
                    sequence: seq,
                    direction: "sent".to_string(),
                    timestamp_ms: now_ms,
                    data_type: "datagram".to_string(),
                    size: result,
                    summary: None,
                    redacted: false,
                });

                Ok(UdpSendResultPy {
                    bytes_sent: result,
                    duration_ms,
                })
            })
        })
    }

    /// Send a datagram to a specific address.
    fn send_to(
        &self,
        py: Python,
        data: Vec<u8>,
        addr: String,
    ) -> PyResult<runtime_async::PyFuture> {
        let state = Arc::clone(&self.state);
        let timeout = std::time::Duration::from_millis(self.config.timeout_ms);

        {
            let s = state.blocking_lock();
            if s.is_closed || s.socket.is_none() {
                return Err(NetworkError::new_err("Not connected"));
            }
        }

        py.allow_threads(|| {
            runtime_async::spawn_async(async move {
                let start = Instant::now();
                let mut s = state.lock().await;
                let socket = s
                    .socket
                    .as_ref()
                    .ok_or_else(|| NetworkError::new_err("Not connected"))?;

                let result = tokio::time::timeout(timeout, socket.send_to(&data, &addr))
                    .await
                    .map_err(|_| {
                        TimeoutError::new_err(format!(
                            "UDP send_to timed out after {}ms",
                            timeout.as_millis()
                        ))
                    })?
                    .map_err(|e| NetworkError::new_err(format!("UDP send_to error: {}", e)))?;

                let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

                s.bytes_sent += result as u64;
                s.sequence += 1;
                let seq = s.sequence;
                let now_ms = chrono::Utc::now().timestamp_millis() as f64;
                s.transcript.push(TranscriptEntryPy {
                    sequence: seq,
                    direction: "sent".to_string(),
                    timestamp_ms: now_ms,
                    data_type: "datagram".to_string(),
                    size: result,
                    summary: None,
                    redacted: false,
                });

                Ok(UdpSendResultPy {
                    bytes_sent: result,
                    duration_ms,
                })
            })
        })
    }

    /// Receive a datagram from the connected target.
    fn recv(&self, py: Python, max_size: Option<usize>) -> PyResult<runtime_async::PyFuture> {
        let state = Arc::clone(&self.state);
        let timeout = std::time::Duration::from_millis(self.config.timeout_ms);
        let max_datagram_size = self.config.max_datagram_size;
        let max_size = max_size.unwrap_or(max_datagram_size);

        {
            let s = state.blocking_lock();
            if s.is_closed || s.socket.is_none() {
                return Err(NetworkError::new_err("Not connected"));
            }
        }

        py.allow_threads(|| {
            runtime_async::spawn_async(async move {
                let start = Instant::now();
                let mut s = state.lock().await;
                let socket = s
                    .socket
                    .as_ref()
                    .ok_or_else(|| NetworkError::new_err("Not connected"))?;

                let mut buf = vec![0u8; max_size];
                let n = tokio::time::timeout(timeout, socket.recv(&mut buf))
                    .await
                    .map_err(|_| {
                        TimeoutError::new_err(format!(
                            "UDP recv timed out after {}ms",
                            timeout.as_millis()
                        ))
                    })?
                    .map_err(|e| NetworkError::new_err(format!("UDP recv error: {}", e)))?;

                let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
                let truncated = n == max_size;

                buf.truncate(n);

                s.bytes_received += n as u64;
                s.sequence += 1;
                let seq = s.sequence;
                let now_ms = chrono::Utc::now().timestamp_millis() as f64;
                s.transcript.push(TranscriptEntryPy {
                    sequence: seq,
                    direction: "received".to_string(),
                    timestamp_ms: now_ms,
                    data_type: "datagram".to_string(),
                    size: n,
                    summary: None,
                    redacted: false,
                });

                Ok(UdpRecvResultPy {
                    data: buf,
                    bytes_received: n,
                    truncated,
                    duration_ms,
                })
            })
        })
    }

    /// Receive a datagram and return the source address.
    fn recv_from(&self, py: Python, max_size: Option<usize>) -> PyResult<runtime_async::PyFuture> {
        let state = Arc::clone(&self.state);
        let timeout = std::time::Duration::from_millis(self.config.timeout_ms);
        let max_datagram_size = self.config.max_datagram_size;
        let max_size = max_size.unwrap_or(max_datagram_size);

        {
            let s = state.blocking_lock();
            if s.is_closed || s.socket.is_none() {
                return Err(NetworkError::new_err("Not connected"));
            }
        }

        py.allow_threads(|| {
            runtime_async::spawn_async(async move {
                let start = Instant::now();
                let mut s = state.lock().await;
                let socket = s
                    .socket
                    .as_ref()
                    .ok_or_else(|| NetworkError::new_err("Not connected"))?;

                let mut buf = vec![0u8; max_size];
                let (n, from) = tokio::time::timeout(timeout, socket.recv_from(&mut buf))
                    .await
                    .map_err(|_| {
                        TimeoutError::new_err(format!(
                            "UDP recv_from timed out after {}ms",
                            timeout.as_millis()
                        ))
                    })?
                    .map_err(|e| NetworkError::new_err(format!("UDP recv_from error: {}", e)))?;

                let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
                let truncated = n == max_size;

                buf.truncate(n);

                let source_address = from.ip().to_string();
                let source_port = from.port();

                s.bytes_received += n as u64;
                s.sequence += 1;
                let seq = s.sequence;
                let now_ms = chrono::Utc::now().timestamp_millis() as f64;
                s.transcript.push(TranscriptEntryPy {
                    sequence: seq,
                    direction: "received".to_string(),
                    timestamp_ms: now_ms,
                    data_type: "datagram".to_string(),
                    size: n,
                    summary: Some(format!("from {}:{}", source_address, source_port)),
                    redacted: false,
                });

                Ok(UdpRecvFromResultPy {
                    data: buf,
                    bytes_received: n,
                    source_address,
                    source_port,
                    truncated,
                    duration_ms,
                })
            })
        })
    }

    /// Close the UDP socket.
    fn close(&self) -> PyResult<()> {
        let mut s = self.state.blocking_lock();
        if s.is_closed {
            return Ok(());
        }
        s.socket.take();
        s.is_closed = true;
        Ok(())
    }

    fn aclose(&self) -> PyResult<()> {
        self.close()
    }

    fn __aenter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __aexit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        let _ = self.close();
        false
    }

    fn __enter__(_slf: Py<Self>) -> PyResult<Py<Self>> {
        Err(pyo3::exceptions::PyTypeError::new_err(
            "Use 'async with' for AsyncUdpSocket",
        ))
    }

    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<bool> {
        Err(pyo3::exceptions::PyTypeError::new_err(
            "Use 'async with' for AsyncUdpSocket",
        ))
    }

    fn __repr__(&self) -> String {
        let s = self.state.blocking_lock();
        format!(
            "AsyncUdpSocket(host={}, port={}, closed={})",
            self.config.host, self.config.port, s.is_closed
        )
    }

    fn __str__(&self) -> String {
        let s = self.state.blocking_lock();
        if s.is_closed {
            format!(
                "async udp://{}:{} (closed)",
                self.config.host, self.config.port
            )
        } else {
            format!(
                "async udp://{}:{} (open)",
                self.config.host, self.config.port
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Banner Probe Result
// ═══════════════════════════════════════════════════════════════════

/// Result of a banner probe operation.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BannerProbeResultPy {
    #[pyo3(get)]
    pub host: String,
    #[pyo3(get)]
    pub port: u16,
    #[pyo3(get)]
    pub banner_bytes: Vec<u8>,
    #[pyo3(get)]
    pub banner_text: Option<String>,
    #[pyo3(get)]
    pub encoding: Option<String>,
    #[pyo3(get)]
    pub timeout: bool,
    #[pyo3(get)]
    pub connection_error: Option<String>,
    #[pyo3(get)]
    pub timing: ConnectionTimingPy,
}

#[pymethods]
impl BannerProbeResultPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("host", &self.host)?;
        dict.set_item("port", self.port)?;
        dict.set_item("banner_bytes", &self.banner_bytes)?;
        dict.set_item("banner_text", &self.banner_text)?;
        dict.set_item("encoding", &self.encoding)?;
        dict.set_item("timeout", self.timeout)?;
        dict.set_item("connection_error", &self.connection_error)?;
        let timing_dict = PyDict::new_bound(py);
        timing_dict.set_item("dns_resolution_ms", &self.timing.dns_resolution_ms)?;
        timing_dict.set_item("tcp_connect_ms", &self.timing.tcp_connect_ms)?;
        timing_dict.set_item("tls_handshake_ms", &self.timing.tls_handshake_ms)?;
        timing_dict.set_item("first_byte_ms", &self.timing.first_byte_ms)?;
        timing_dict.set_item("total_ms", self.timing.total_ms)?;
        timing_dict.set_item("connection_reused", self.timing.connection_reused)?;
        dict.set_item("timing", timing_dict)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "BannerProbeResult(host={}, port={}, banner_bytes={}, timeout={})",
            self.host,
            self.port,
            self.banner_bytes.len(),
            self.timeout
        )
    }

    fn __str__(&self) -> String {
        if let Some(ref text) = self.banner_text {
            format!("{}:{} banner: {}", self.host, self.port, text)
        } else if self.timeout {
            format!("{}:{} timeout", self.host, self.port)
        } else if let Some(ref err) = self.connection_error {
            format!("{}:{} error: {}", self.host, self.port, err)
        } else {
            format!(
                "{}:{} {} banner bytes",
                self.host,
                self.port,
                self.banner_bytes.len()
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// Probe Functions
// ═══════════════════════════════════════════════════════════════════

/// Perform a TCP connect probe to check if a port is open.
///
/// Args:
///     host: Target hostname or IP.
///     port: Target port.
///     timeout_ms: Connection timeout in milliseconds (default: 5000).
///
/// Returns:
///     TcpConnectResultPy with connection metadata and timing.
///
/// Raises:
///     NetworkError: If the connection fails.
///     TimeoutError: If the connection times out.
#[pyfunction]
#[pyo3(signature = (host, port, timeout_ms=5000))]
pub fn tcp_connect_probe(
    py: Python,
    host: &str,
    port: u16,
    timeout_ms: u64,
) -> PyResult<TcpConnectResultPy> {
    let host_owned = host.to_string();
    let connect_timeout = std::time::Duration::from_millis(timeout_ms);

    let connect_start = Instant::now();

    let stream = runtime_sync::block_on(py, async move {
        let addr = format!("{}:{}", host_owned, port);
        let stream = tokio::time::timeout(connect_timeout, tokio::net::TcpStream::connect(&addr))
            .await
            .map_err(|_| {
                TimeoutError::new_err(format!(
                    "TCP connect probe timed out after {}ms to {}:{}",
                    connect_timeout.as_millis(),
                    host_owned,
                    port
                ))
            })?
            .map_err(|e| {
                NetworkError::new_err(format!(
                    "TCP connect probe failed to {}:{}: {}",
                    host_owned, port, e
                ))
            })?;

        Ok::<_, PyErr>(stream)
    })?;

    let connect_duration_ms = connect_start.elapsed().as_secs_f64() * 1000.0;

    let local_addr = stream
        .local_addr()
        .map_err(|e| NetworkError::new_err(format!("Failed to get local address: {}", e)))?;
    let remote_addr = stream
        .peer_addr()
        .map_err(|e| NetworkError::new_err(format!("Failed to get peer address: {}", e)))?;

    let local_endpoint = endpoint_from_socket_addr(local_addr);
    let remote_endpoint = endpoint_from_socket_addr(remote_addr);

    let timing = ConnectionTimingPy {
        dns_resolution_ms: None,
        tcp_connect_ms: Some(connect_duration_ms),
        tls_handshake_ms: None,
        first_byte_ms: None,
        total_ms: connect_duration_ms,
        connection_reused: false,
    };

    let metadata = ConnectionMetadataPy {
        local_endpoint: Some(local_endpoint.clone()),
        remote_endpoint: Some(remote_endpoint.clone()),
        resolved_address: None,
        transport_protocol: "tcp".to_string(),
        negotiated_protocol: None,
        connection_reused: false,
        timing: Some(timing.clone()),
        tls_version: None,
        tls_cipher: None,
        bytes_sent: 0,
        bytes_received: 0,
    };

    Ok(TcpConnectResultPy {
        local_endpoint,
        remote_endpoint,
        timing,
        metadata,
    })
}

/// Async TCP connect probe (returns a PyFuture).
#[pyfunction]
#[pyo3(signature = (host, port, timeout_ms=5000))]
pub fn async_tcp_connect_probe(
    host: &str,
    port: u16,
    timeout_ms: u64,
) -> PyResult<runtime_async::PyFuture> {
    let host_owned = host.to_string();
    let connect_timeout = std::time::Duration::from_millis(timeout_ms);

    runtime_async::spawn_async(async move {
        let connect_start = Instant::now();

        let addr = format!("{}:{}", host_owned, port);
        let stream = tokio::time::timeout(connect_timeout, tokio::net::TcpStream::connect(&addr))
            .await
            .map_err(|_| {
                TimeoutError::new_err(format!(
                    "TCP connect probe timed out after {}ms to {}:{}",
                    connect_timeout.as_millis(),
                    host_owned,
                    port
                ))
            })?
            .map_err(|e| {
                NetworkError::new_err(format!(
                    "TCP connect probe failed to {}:{}: {}",
                    host_owned, port, e
                ))
            })?;

        let connect_duration_ms = connect_start.elapsed().as_secs_f64() * 1000.0;

        let local_addr = stream
            .local_addr()
            .map_err(|e| NetworkError::new_err(format!("Failed to get local address: {}", e)))?;
        let remote_addr = stream
            .peer_addr()
            .map_err(|e| NetworkError::new_err(format!("Failed to get peer address: {}", e)))?;

        let local_endpoint = endpoint_from_socket_addr(local_addr);
        let remote_endpoint = endpoint_from_socket_addr(remote_addr);

        let timing = ConnectionTimingPy {
            dns_resolution_ms: None,
            tcp_connect_ms: Some(connect_duration_ms),
            tls_handshake_ms: None,
            first_byte_ms: None,
            total_ms: connect_duration_ms,
            connection_reused: false,
        };

        let metadata = ConnectionMetadataPy {
            local_endpoint: Some(local_endpoint.clone()),
            remote_endpoint: Some(remote_endpoint.clone()),
            resolved_address: None,
            transport_protocol: "tcp".to_string(),
            negotiated_protocol: None,
            connection_reused: false,
            timing: Some(timing.clone()),
            tls_version: None,
            tls_cipher: None,
            bytes_sent: 0,
            bytes_received: 0,
        };

        Ok(TcpConnectResultPy {
            local_endpoint,
            remote_endpoint,
            timing,
            metadata,
        })
    })
}

/// Perform a banner probe: connect, read an optional banner, and return.
///
/// Args:
///     host: Target hostname or IP.
///     port: Target port.
///     timeout_ms: Connection and read timeout in milliseconds (default: 5000).
///     max_banner_bytes: Maximum bytes to read for the banner (default: 4096).
///
/// Returns:
///     BannerProbeResultPy with the banner data and timing.
#[pyfunction]
#[pyo3(signature = (host, port, timeout_ms=5000, max_banner_bytes=4096))]
pub fn banner_probe(
    py: Python,
    host: &str,
    port: u16,
    timeout_ms: u64,
    max_banner_bytes: usize,
) -> PyResult<BannerProbeResultPy> {
    let host_owned = host.to_string();
    let host_for_result = host_owned.clone();
    let connect_timeout = std::time::Duration::from_millis(timeout_ms);

    let connect_start = Instant::now();

    let (banner_bytes, banner_text, encoding, timeout_flag, error_msg, timing) =
        runtime_sync::block_on(py, async move {
            let addr = format!("{}:{}", host_owned, port);

            let mut stream =
                match tokio::time::timeout(connect_timeout, tokio::net::TcpStream::connect(&addr))
                    .await
                {
                    Ok(Ok(s)) => s,
                    Ok(Err(e)) => {
                        let connect_ms = connect_start.elapsed().as_secs_f64() * 1000.0;
                        let timing = ConnectionTimingPy {
                            dns_resolution_ms: None,
                            tcp_connect_ms: Some(connect_ms),
                            tls_handshake_ms: None,
                            first_byte_ms: None,
                            total_ms: connect_ms,
                            connection_reused: false,
                        };
                        return Ok::<_, PyErr>((
                            Vec::new(),
                            None,
                            None,
                            false,
                            Some(format!("Connection failed: {}", e)),
                            timing,
                        ));
                    }
                    Err(_) => {
                        let connect_ms = connect_start.elapsed().as_secs_f64() * 1000.0;
                        let timing = ConnectionTimingPy {
                            dns_resolution_ms: None,
                            tcp_connect_ms: Some(connect_ms),
                            tls_handshake_ms: None,
                            first_byte_ms: None,
                            total_ms: connect_ms,
                            connection_reused: false,
                        };
                        return Ok::<_, PyErr>((Vec::new(), None, None, true, None, timing));
                    }
                };

            let mut buf = vec![0u8; max_banner_bytes];
            let read_result = tokio::time::timeout(connect_timeout, stream.read(&mut buf)).await;

            let total_ms = connect_start.elapsed().as_secs_f64() * 1000.0;
            let timing = ConnectionTimingPy {
                dns_resolution_ms: None,
                tcp_connect_ms: None,
                tls_handshake_ms: None,
                first_byte_ms: None,
                total_ms,
                connection_reused: false,
            };

            match read_result {
                Ok(Ok(0)) => Ok((Vec::new(), None, None, false, None, timing)),
                Ok(Ok(n)) => {
                    buf.truncate(n);
                    let (text, enc) = decode_banner_best_effort(&buf);
                    Ok((buf, text, enc, false, None, timing))
                }
                Ok(Err(e)) => Ok((
                    Vec::new(),
                    None,
                    None,
                    false,
                    Some(format!("Read error: {}", e)),
                    timing,
                )),
                Err(_) => Ok((Vec::new(), None, None, true, None, timing)),
            }
        })?;

    Ok(BannerProbeResultPy {
        host: host_for_result,
        port,
        banner_bytes,
        banner_text,
        encoding,
        timeout: timeout_flag,
        connection_error: error_msg,
        timing,
    })
}

/// Async banner probe (returns a PyFuture).
#[pyfunction]
#[pyo3(signature = (host, port, timeout_ms=5000, max_banner_bytes=4096))]
pub fn async_banner_probe(
    host: &str,
    port: u16,
    timeout_ms: u64,
    max_banner_bytes: usize,
) -> PyResult<runtime_async::PyFuture> {
    let host_owned = host.to_string();
    let connect_timeout = std::time::Duration::from_millis(timeout_ms);

    runtime_async::spawn_async(async move {
        let connect_start = Instant::now();
        let addr = format!("{}:{}", host_owned, port);

        let mut stream = match tokio::time::timeout(
            connect_timeout,
            tokio::net::TcpStream::connect(&addr),
        )
        .await
        {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => {
                let connect_ms = connect_start.elapsed().as_secs_f64() * 1000.0;
                let timing = ConnectionTimingPy {
                    dns_resolution_ms: None,
                    tcp_connect_ms: Some(connect_ms),
                    tls_handshake_ms: None,
                    first_byte_ms: None,
                    total_ms: connect_ms,
                    connection_reused: false,
                };
                return Ok(BannerProbeResultPy {
                    host: host_owned,
                    port,
                    banner_bytes: Vec::new(),
                    banner_text: None,
                    encoding: None,
                    timeout: false,
                    connection_error: Some(format!("Connection failed: {}", e)),
                    timing,
                });
            }
            Err(_) => {
                let connect_ms = connect_start.elapsed().as_secs_f64() * 1000.0;
                let timing = ConnectionTimingPy {
                    dns_resolution_ms: None,
                    tcp_connect_ms: Some(connect_ms),
                    tls_handshake_ms: None,
                    first_byte_ms: None,
                    total_ms: connect_ms,
                    connection_reused: false,
                };
                return Ok(BannerProbeResultPy {
                    host: host_owned,
                    port,
                    banner_bytes: Vec::new(),
                    banner_text: None,
                    encoding: None,
                    timeout: true,
                    connection_error: None,
                    timing,
                });
            }
        };

        let mut buf = vec![0u8; max_banner_bytes];
        let read_result = tokio::time::timeout(connect_timeout, stream.read(&mut buf)).await;

        let total_ms = connect_start.elapsed().as_secs_f64() * 1000.0;
        let timing = ConnectionTimingPy {
            dns_resolution_ms: None,
            tcp_connect_ms: None,
            tls_handshake_ms: None,
            first_byte_ms: None,
            total_ms,
            connection_reused: false,
        };

        match read_result {
            Ok(Ok(0)) => Ok(BannerProbeResultPy {
                host: host_owned,
                port,
                banner_bytes: Vec::new(),
                banner_text: None,
                encoding: None,
                timeout: false,
                connection_error: None,
                timing,
            }),
            Ok(Ok(n)) => {
                buf.truncate(n);
                let (text, enc) = decode_banner_best_effort(&buf);
                Ok(BannerProbeResultPy {
                    host: host_owned,
                    port,
                    banner_bytes: buf,
                    banner_text: text,
                    encoding: enc,
                    timeout: false,
                    connection_error: None,
                    timing,
                })
            }
            Ok(Err(e)) => Ok(BannerProbeResultPy {
                host: host_owned,
                port,
                banner_bytes: Vec::new(),
                banner_text: None,
                encoding: None,
                timeout: false,
                connection_error: Some(format!("Read error: {}", e)),
                timing,
            }),
            Err(_) => Ok(BannerProbeResultPy {
                host: host_owned,
                port,
                banner_bytes: Vec::new(),
                banner_text: None,
                encoding: None,
                timeout: true,
                connection_error: None,
                timing,
            }),
        }
    })
}

// ═══════════════════════════════════════════════════════════════════
// Internal Helpers
// ═══════════════════════════════════════════════════════════════════

/// Best-effort decode of banner bytes to text.
fn decode_banner_best_effort(data: &[u8]) -> (Option<String>, Option<String>) {
    // Try UTF-8 first
    if let Ok(text) = std::str::from_utf8(data) {
        let trimmed = text.trim_matches(|c: char| c == '\0' || c == '\r' || c == '\n');
        if !trimmed.is_empty() {
            return (Some(trimmed.to_string()), Some("utf-8".to_string()));
        }
    }

    // Try ASCII
    if data.iter().all(|&b| b.is_ascii() || b == 0) {
        let text: String = data
            .iter()
            .filter(|&&b| b != 0)
            .map(|&b| b as char)
            .collect();
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return (Some(trimmed.to_string()), Some("ascii".to_string()));
        }
    }

    // Try Latin-1 (ISO-8859-1) — maps all byte values directly
    if data
        .iter()
        .all(|&b| b.is_ascii() || (0xA0..=0xFF).contains(&b))
    {
        let text: String = data.iter().map(|&b| b as char).collect();
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return (Some(trimmed.to_string()), Some("latin-1".to_string()));
        }
    }

    (None, None)
}
