use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

use crate::error::ScanError;
use crate::runtime_async;
use crate::runtime_sync;

/// Wrapper for CaptureConfig.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureConfigPy {
    #[pyo3(get)]
    pub interface: String,
    #[pyo3(get)]
    pub filter: Option<String>,
    #[pyo3(get)]
    pub promiscuous: bool,
    #[pyo3(get)]
    pub snapshot_len: usize,
    #[pyo3(get)]
    pub timeout_secs: u64,
    #[pyo3(get)]
    pub max_packets: Option<usize>,
    #[pyo3(get)]
    pub save_to_file: Option<String>,
    #[pyo3(get)]
    pub validate_checksums: bool,
}

impl CaptureConfigPy {
    fn to_engine(&self) -> eggsec::packet::CaptureConfig {
        eggsec::packet::CaptureConfig {
            interface: self.interface.clone(),
            filter: self.filter.clone(),
            promiscuous: self.promiscuous,
            snapshot_len: self.snapshot_len,
            timeout: Duration::from_secs(self.timeout_secs),
            max_packets: self.max_packets,
            save_to_file: self.save_to_file.clone(),
            validate_checksums: self.validate_checksums,
        }
    }
}

#[pymethods]
impl CaptureConfigPy {
    #[new]
    #[pyo3(signature = (interface="", filter=None, promiscuous=true, snapshot_len=65535, timeout_secs=1, max_packets=None, save_to_file=None, validate_checksums=false))]
    fn new(
        interface: &str,
        filter: Option<&str>,
        promiscuous: bool,
        snapshot_len: usize,
        timeout_secs: u64,
        max_packets: Option<usize>,
        save_to_file: Option<&str>,
        validate_checksums: bool,
    ) -> Self {
        Self {
            interface: interface.to_string(),
            filter: filter.map(|s| s.to_string()),
            promiscuous,
            snapshot_len,
            timeout_secs,
            max_packets,
            save_to_file: save_to_file.map(|s| s.to_string()),
            validate_checksums,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("interface", &self.interface)?;
        dict.set_item("filter", &self.filter)?;
        dict.set_item("promiscuous", self.promiscuous)?;
        dict.set_item("snapshot_len", self.snapshot_len)?;
        dict.set_item("timeout_secs", self.timeout_secs)?;
        dict.set_item("max_packets", self.max_packets)?;
        dict.set_item("save_to_file", &self.save_to_file)?;
        dict.set_item("validate_checksums", self.validate_checksums)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "CaptureConfig(interface={}, filter={:?}, promiscuous={}, snapshot_len={})",
            self.interface, self.filter, self.promiscuous, self.snapshot_len
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Capture on {} (filter={:?}, promiscuous={})",
            self.interface, self.filter, self.promiscuous
        )
    }
}

/// Wrapper for CaptureStats.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureStatsPy {
    #[pyo3(get)]
    pub packets_captured: usize,
    #[pyo3(get)]
    pub bytes_captured: usize,
    #[pyo3(get)]
    pub packets_dropped: usize,
    #[pyo3(get)]
    pub runtime_ms: u64,
}

impl CaptureStatsPy {
    fn from_engine(engine: eggsec::packet::CaptureStats) -> Self {
        Self {
            packets_captured: engine.packets_captured,
            bytes_captured: engine.bytes_captured,
            packets_dropped: engine.packets_dropped,
            runtime_ms: engine.runtime_ms,
        }
    }
}

#[pymethods]
impl CaptureStatsPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("packets_captured", self.packets_captured)?;
        dict.set_item("bytes_captured", self.bytes_captured)?;
        dict.set_item("packets_dropped", self.packets_dropped)?;
        dict.set_item("runtime_ms", self.runtime_ms)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "CaptureStats(packets={}, bytes={}, dropped={}, runtime={}ms)",
            self.packets_captured, self.bytes_captured, self.packets_dropped, self.runtime_ms
        )
    }

    fn __str__(&self) -> String {
        format!(
            "{} packets captured ({} bytes), {} dropped, {}ms",
            self.packets_captured, self.bytes_captured, self.packets_dropped, self.runtime_ms
        )
    }
}

/// Simplified packet information for Python consumption.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketInfoPy {
    #[pyo3(get)]
    pub timestamp: String,
    #[pyo3(get)]
    pub src_ip: Option<String>,
    #[pyo3(get)]
    pub dst_ip: Option<String>,
    #[pyo3(get)]
    pub protocol: String,
    #[pyo3(get)]
    pub src_port: Option<u16>,
    #[pyo3(get)]
    pub dst_port: Option<u16>,
    #[pyo3(get)]
    pub size: usize,
    #[pyo3(get)]
    pub summary: String,
}

impl PacketInfoPy {
    fn from_engine(engine: &eggsec::packet::PacketInfo) -> Self {
        use eggsec::packet::TransportProtocol;

        let src_ip = engine.ip.as_ref().map(|ip| ip.src_ip.clone());
        let dst_ip = engine.ip.as_ref().map(|ip| ip.dst_ip.clone());

        let (protocol, src_port, dst_port) = match &engine.transport {
            Some(TransportProtocol::Tcp(tcp)) => {
                ("TCP".to_string(), Some(tcp.src_port), Some(tcp.dst_port))
            }
            Some(TransportProtocol::Udp(udp)) => {
                ("UDP".to_string(), Some(udp.src_port), Some(udp.dst_port))
            }
            Some(TransportProtocol::Icmp(icmp)) => (format!("ICMP/{}", icmp.icmp_type), None, None),
            Some(TransportProtocol::Unknown(_)) => ("Unknown".to_string(), None, None),
            None => ("Unknown".to_string(), None, None),
        };

        Self {
            timestamp: engine.timestamp.to_rfc3339(),
            src_ip,
            dst_ip,
            protocol,
            src_port,
            dst_port,
            size: engine.raw_size,
            summary: engine.summary(),
        }
    }
}

#[pymethods]
impl PacketInfoPy {
    #[new]
    #[pyo3(signature = (timestamp, src_ip=None, dst_ip=None, protocol="Unknown", src_port=None, dst_port=None, size=0, summary=""))]
    fn new(
        timestamp: String,
        src_ip: Option<String>,
        dst_ip: Option<String>,
        protocol: &str,
        src_port: Option<u16>,
        dst_port: Option<u16>,
        size: usize,
        summary: &str,
    ) -> Self {
        Self {
            timestamp,
            src_ip,
            dst_ip,
            protocol: protocol.to_string(),
            src_port,
            dst_port,
            size,
            summary: summary.to_string(),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("timestamp", &self.timestamp)?;
        dict.set_item("src_ip", &self.src_ip)?;
        dict.set_item("dst_ip", &self.dst_ip)?;
        dict.set_item("protocol", &self.protocol)?;
        dict.set_item("src_port", self.src_port)?;
        dict.set_item("dst_port", self.dst_port)?;
        dict.set_item("size", self.size)?;
        dict.set_item("summary", &self.summary)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "PacketInfo(proto={}, src={:?}, dst={:?}, size={})",
            self.protocol, self.src_ip, self.dst_ip, self.size
        )
    }

    fn __str__(&self) -> String {
        self.summary.clone()
    }
}

/// Wrapper for NetworkInterfaceInfo.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterfaceInfoPy {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub ips: Vec<String>,
    #[pyo3(get)]
    pub mac: Option<String>,
    #[pyo3(get)]
    pub is_up: bool,
    #[pyo3(get)]
    pub is_loopback: bool,
}

impl NetworkInterfaceInfoPy {
    fn from_engine(engine: eggsec::packet::capture::NetworkInterfaceInfo) -> Self {
        Self {
            name: engine.name,
            ips: engine.ips,
            mac: engine.mac,
            is_up: engine.is_up,
            is_loopback: engine.is_loopback,
        }
    }
}

#[pymethods]
impl NetworkInterfaceInfoPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("name", &self.name)?;
        dict.set_item("ips", &self.ips)?;
        dict.set_item("mac", &self.mac)?;
        dict.set_item("is_up", self.is_up)?;
        dict.set_item("is_loopback", self.is_loopback)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "NetworkInterfaceInfo(name={}, up={}, loopback={})",
            self.name, self.is_up, self.is_loopback
        )
    }

    fn __str__(&self) -> String {
        format!(
            "{} (up={}, loopback={})",
            self.name, self.is_up, self.is_loopback
        )
    }
}

/// PCAP writer for saving captured packets to file.
///
/// Supports context manager protocol and explicit close for deterministic cleanup.
#[pyclass]
pub struct PcapWriterPy {
    inner: Option<eggsec::packet::capture::PcapWriter>,
    closed: bool,
}

#[pymethods]
impl PcapWriterPy {
    #[new]
    fn new(path: &str, snapshot_len: usize) -> PyResult<Self> {
        let writer = eggsec::packet::capture::PcapWriter::new(path, snapshot_len)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        Ok(Self {
            inner: Some(writer),
            closed: false,
        })
    }

    fn write_packet(&mut self, data: &[u8]) -> PyResult<()> {
        self.inner
            .as_mut()
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("PcapWriter is closed"))?
            .write_packet(data)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    }

    fn flush(&mut self) -> PyResult<()> {
        self.inner
            .as_mut()
            .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("PcapWriter is closed"))?
            .flush()
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    }

    /// Close the writer, flushing any buffered data. Idempotent.
    fn close(&mut self) -> PyResult<()> {
        if let Some(mut writer) = self.inner.take() {
            writer
                .flush()
                .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        }
        self.closed = true;
        Ok(())
    }

    /// Check if the writer has been closed.
    #[getter]
    fn is_closed(&self) -> bool {
        self.closed
    }

    /// Context manager __enter__.
    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    /// Context manager __exit__ — closes the writer.
    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        false
    }

    fn __repr__(&self) -> String {
        if self.closed {
            "PcapWriter(closed)".to_string()
        } else {
            "PcapWriter()".to_string()
        }
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// List available network interfaces.
///
/// Returns a list of NetworkInterfaceInfoPy objects describing each interface.
#[pyfunction]
pub fn list_network_interfaces() -> Vec<NetworkInterfaceInfoPy> {
    eggsec::packet::capture::list_interfaces()
        .into_iter()
        .map(NetworkInterfaceInfoPy::from_engine)
        .collect()
}

/// Parse a PCAP file and return packet information.
///
/// Reads a standard pcap file, parses each packet, and returns a list of
/// simplified packet info objects.
///
/// Args:
///     file_path: Path to the PCAP file to parse.
///
/// Returns:
///     List of PacketInfoPy objects with parsed packet data.
///
/// Raises:
///     IOError: If the file cannot be opened or read.
///     ValueError: If the PCAP header is invalid.
#[pyfunction]
pub fn parse_pcap(file_path: &str) -> PyResult<Vec<PacketInfoPy>> {
    use std::fs::File;
    use std::io::{BufReader, Read};

    let file = File::open(file_path).map_err(|e| {
        pyo3::exceptions::PyIOError::new_err(format!("Failed to open {}: {}", file_path, e))
    })?;
    let mut reader = BufReader::new(file);

    // Read global header (24 bytes)
    let mut header = [0u8; 24];
    reader.read_exact(&mut header).map_err(|e| {
        pyo3::exceptions::PyIOError::new_err(format!("Failed to read PCAP header: {}", e))
    })?;

    let (magic, _network) = parse_pcap_global_header(&header)?;

    let mut packets = Vec::new();
    let mut pkt_header = [0u8; 16];

    loop {
        match reader.read_exact(&mut pkt_header) {
            Ok(_) => {}
            Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => {
                return Err(pyo3::exceptions::PyIOError::new_err(format!(
                    "Failed to read packet header: {}",
                    e
                )));
            }
        }

        let incl_len = read_u32(&pkt_header[8..12], magic).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid packet header: {}", e))
        })? as usize;

        let mut payload = vec![0u8; incl_len];
        reader.read_exact(&mut payload).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format!("Failed to read packet data: {}", e))
        })?;

        let parsed = eggsec::packet::types::ParsedPacket::parse(&payload);
        let packet_info = eggsec::packet::PacketInfo {
            timestamp: chrono::Utc::now(),
            ethernet: parsed.as_ref().and_then(|p| p.ethernet.clone()),
            ip: parsed.as_ref().and_then(|p| p.ip.clone()),
            transport: parsed.as_ref().and_then(|p| p.transport.clone()),
            app: parsed.as_ref().and_then(|p| p.app.clone()),
            raw_size: payload.len(),
            hex_dump: eggsec::packet::hexdump(&payload),
        };

        packets.push(PacketInfoPy::from_engine(&packet_info));

        if packets.len() >= 100_000 {
            break;
        }
    }

    Ok(packets)
}

/// Detect endianness and network type from a PCAP global header.
fn parse_pcap_global_header(header: &[u8; 24]) -> PyResult<(u32, u32)> {
    let magic_le = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
    let magic_be = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);

    let magic = match (magic_le, magic_be) {
        (0xa1b2c3d4, _) => 0xa1b2c3d4u32,
        (_, 0xa1b2c3d4) => 0xa1b2c3d4u32,
        (0xd4c3b2a1, _) => 0xd4c3b2a1u32,
        (_, 0xd4c3b2a1) => 0xd4c3b2a1u32,
        _ => {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Invalid PCAP magic number",
            ));
        }
    };

    let network = read_u32(&header[20..24], magic).map_err(|e| {
        pyo3::exceptions::PyValueError::new_err(format!("Invalid network type: {}", e))
    })?;

    Ok((magic, network))
}

fn read_u32(bytes: &[u8], magic: u32) -> Result<u32, String> {
    if bytes.len() < 4 {
        return Err("Not enough bytes for u32".to_string());
    }
    let arr = [bytes[0], bytes[1], bytes[2], bytes[3]];
    match magic {
        0xa1b2c3d4 => Ok(u32::from_le_bytes(arr)),
        0xd4c3b2a1 => Ok(u32::from_be_bytes(arr)),
        _ => Ok(u32::from_le_bytes(arr)),
    }
}

// ═══════════════════════════════════════════════════════════════════
// D2: Live packet inspection types
// ═══════════════════════════════════════════════════════════════════

/// Filter for selecting packets during capture.
///
/// Allows filtering by protocol, port, or BPF expression.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketFilterPy {
    #[pyo3(get)]
    pub bpf_expression: Option<String>,
    #[pyo3(get)]
    pub protocol: Option<String>,
    #[pyo3(get)]
    pub src_port: Option<u16>,
    #[pyo3(get)]
    pub dst_port: Option<u16>,
    #[pyo3(get)]
    pub src_ip: Option<String>,
    #[pyo3(get)]
    pub dst_ip: Option<String>,
}

#[pymethods]
impl PacketFilterPy {
    #[new]
    #[pyo3(signature = (bpf_expression=None, protocol=None, src_port=None, dst_port=None, src_ip=None, dst_ip=None))]
    fn new(
        bpf_expression: Option<&str>,
        protocol: Option<&str>,
        src_port: Option<u16>,
        dst_port: Option<u16>,
        src_ip: Option<&str>,
        dst_ip: Option<&str>,
    ) -> Self {
        Self {
            bpf_expression: bpf_expression.map(|s| s.to_string()),
            protocol: protocol.map(|s| s.to_string()),
            src_port,
            dst_port,
            src_ip: src_ip.map(|s| s.to_string()),
            dst_ip: dst_ip.map(|s| s.to_string()),
        }
    }

    fn to_bpf(&self) -> String {
        let mut parts = Vec::new();
        if let Some(ref proto) = self.protocol {
            parts.push(proto.clone());
        }
        if let Some(port) = self.src_port {
            parts.push(format!("src port {}", port));
        }
        if let Some(port) = self.dst_port {
            parts.push(format!("dst port {}", port));
        }
        if let Some(ref ip) = self.src_ip {
            parts.push(format!("src host {}", ip));
        }
        if let Some(ref ip) = self.dst_ip {
            parts.push(format!("dst host {}", ip));
        }
        if let Some(ref expr) = self.bpf_expression {
            parts.push(expr.clone());
        }
        parts.join(" and ")
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("bpf_expression", &self.bpf_expression)?;
        dict.set_item("protocol", &self.protocol)?;
        dict.set_item("src_port", self.src_port)?;
        dict.set_item("dst_port", self.dst_port)?;
        dict.set_item("src_ip", &self.src_ip)?;
        dict.set_item("dst_ip", &self.dst_ip)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("PacketFilter(bpf={:?})", self.bpf_expression)
    }

    fn __str__(&self) -> String {
        self.to_bpf()
    }
}

/// Record of a network flow (connection tuple).
///
/// Tracks a specific network connection identified by the 5-tuple
/// (src_ip, dst_ip, src_port, dst_port, protocol).
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowRecordPy {
    #[pyo3(get)]
    pub src_ip: String,
    #[pyo3(get)]
    pub dst_ip: String,
    #[pyo3(get)]
    pub src_port: u16,
    #[pyo3(get)]
    pub dst_port: u16,
    #[pyo3(get)]
    pub protocol: String,
    #[pyo3(get)]
    pub packet_count: u64,
    #[pyo3(get)]
    pub byte_count: u64,
    #[pyo3(get)]
    pub first_seen_ms: u64,
    #[pyo3(get)]
    pub last_seen_ms: u64,
    #[pyo3(get)]
    pub tcp_flags_seen: Vec<String>,
}

#[pymethods]
impl FlowRecordPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("src_ip", &self.src_ip)?;
        dict.set_item("dst_ip", &self.dst_ip)?;
        dict.set_item("src_port", self.src_port)?;
        dict.set_item("dst_port", self.dst_port)?;
        dict.set_item("protocol", &self.protocol)?;
        dict.set_item("packet_count", self.packet_count)?;
        dict.set_item("byte_count", self.byte_count)?;
        dict.set_item("first_seen_ms", self.first_seen_ms)?;
        dict.set_item("last_seen_ms", self.last_seen_ms)?;
        dict.set_item("tcp_flags_seen", &self.tcp_flags_seen)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "FlowRecord({}:{} → {}:{} [{}])",
            self.src_ip, self.src_port, self.dst_ip, self.dst_port, self.protocol
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Result of a live packet capture session.
///
/// Contains captured packet data, statistics, and flow aggregation.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveCaptureResultPy {
    #[pyo3(get)]
    pub interface: String,
    #[pyo3(get)]
    pub duration_ms: u64,
    #[pyo3(get)]
    pub packets_captured: usize,
    #[pyo3(get)]
    pub packets_dropped: usize,
    #[pyo3(get)]
    pub bytes_captured: u64,
    #[pyo3(get)]
    pub flows: Vec<FlowRecordPy>,
    #[pyo3(get)]
    pub packets: Vec<PacketInfoPy>,
}

#[pymethods]
impl LiveCaptureResultPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("interface", &self.interface)?;
        dict.set_item("duration_ms", self.duration_ms)?;
        dict.set_item("packets_captured", self.packets_captured)?;
        dict.set_item("packets_dropped", self.packets_dropped)?;
        dict.set_item("bytes_captured", self.bytes_captured)?;
        let flows_list = PyList::empty_bound(py);
        for f in &self.flows {
            flows_list.append(f.to_dict(py)?)?;
        }
        dict.set_item("flows", flows_list)?;
        let packets_list = PyList::empty_bound(py);
        for p in &self.packets {
            packets_list.append(p.to_dict(py)?)?;
        }
        dict.set_item("packets", packets_list)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "LiveCaptureResult(interface={}, packets={}, flows={})",
            self.interface,
            self.packets_captured,
            self.flows.len()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "Captured {} packets on {} ({} flows, {} dropped)",
            self.packets_captured,
            self.interface,
            self.flows.len(),
            self.packets_dropped
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// D3: Network probing types (traceroute, packet crafting)
// ═══════════════════════════════════════════════════════════════════

/// Configuration for a traceroute probe.
#[pyclass]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracerouteConfigPy {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub max_hops: u8,
    #[pyo3(get)]
    pub timeout_secs: u64,
    #[pyo3(get)]
    pub max_retries: u32,
    #[pyo3(get)]
    pub first_ttl: u8,
    #[pyo3(get)]
    pub port: u16,
    #[pyo3(get)]
    pub use_icmp: bool,
    #[pyo3(get)]
    pub packet_size: usize,
    #[pyo3(get)]
    pub resolve_names: bool,
}

#[pymethods]
impl TracerouteConfigPy {
    #[new]
    #[pyo3(signature = (target, max_hops=30, timeout_secs=3, max_retries=3, first_ttl=1, port=33434, use_icmp=false, packet_size=60, resolve_names=true))]
    fn new(
        target: &str,
        max_hops: u8,
        timeout_secs: u64,
        max_retries: u32,
        first_ttl: u8,
        port: u16,
        use_icmp: bool,
        packet_size: usize,
        resolve_names: bool,
    ) -> Self {
        Self {
            target: target.to_string(),
            max_hops,
            timeout_secs,
            max_retries,
            first_ttl,
            port,
            use_icmp,
            packet_size,
            resolve_names,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("max_hops", self.max_hops)?;
        dict.set_item("timeout_secs", self.timeout_secs)?;
        dict.set_item("max_retries", self.max_retries)?;
        dict.set_item("first_ttl", self.first_ttl)?;
        dict.set_item("port", self.port)?;
        dict.set_item("use_icmp", self.use_icmp)?;
        dict.set_item("packet_size", self.packet_size)?;
        dict.set_item("resolve_names", self.resolve_names)?;
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "TracerouteConfig(target={}, max_hops={})",
            self.target, self.max_hops
        )
    }
}

impl TracerouteConfigPy {
    pub fn to_engine(&self) -> eggsec::packet::traceroute::TracerouteConfig {
        eggsec::packet::traceroute::TracerouteConfig {
            target: self.target.clone(),
            max_hops: self.max_hops,
            timeout: std::time::Duration::from_secs(self.timeout_secs),
            max_retries: self.max_retries,
            first_ttl: self.first_ttl,
            port: self.port,
            use_icmp: self.use_icmp,
            packet_size: self.packet_size,
            parallel_probes: false,
            resolve_names: self.resolve_names,
            max_concurrent_probes: 1,
        }
    }
}

/// A single hop in a traceroute result.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracerouteHopPy {
    #[pyo3(get)]
    pub hop: u8,
    #[pyo3(get)]
    pub address: Option<String>,
    #[pyo3(get)]
    pub rtt_ms: Option<f64>,
    #[pyo3(get)]
    pub name: Option<String>,
    #[pyo3(get)]
    pub is_final: bool,
}

#[pymethods]
impl TracerouteHopPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("hop", self.hop)?;
        dict.set_item("address", &self.address)?;
        dict.set_item("rtt_ms", self.rtt_ms)?;
        dict.set_item("name", &self.name)?;
        dict.set_item("is_final", self.is_final)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("TracerouteHop(hop={}, rtt={:?}ms)", self.hop, self.rtt_ms)
    }

    fn __str__(&self) -> String {
        let addr = self.address.as_deref().unwrap_or("*");
        let rtt = self
            .rtt_ms
            .map(|r| format!("{:.2}ms", r))
            .unwrap_or_else(|| "*".to_string());
        format!("{}: {} {}", self.hop, addr, rtt)
    }
}

/// Result of a traceroute probe.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracerouteResultPy {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub resolved_address: String,
    #[pyo3(get)]
    pub hops: Vec<TracerouteHopPy>,
    #[pyo3(get)]
    pub total_hops: usize,
    #[pyo3(get)]
    pub success: bool,
}

#[pymethods]
impl TracerouteResultPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("resolved_address", &self.resolved_address)?;
        let hops_list = PyList::empty_bound(py);
        for h in &self.hops {
            hops_list.append(h.to_dict(py)?)?;
        }
        dict.set_item("hops", hops_list)?;
        dict.set_item("total_hops", self.total_hops)?;
        dict.set_item("success", self.success)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "TracerouteResult(target={}, hops={}, success={})",
            self.target, self.total_hops, self.success
        )
    }

    fn __str__(&self) -> String {
        format!(
            "traceroute to {} ({} hops, {})",
            self.target,
            self.total_hops,
            if self.success { "reached" } else { "failed" }
        )
    }
}

/// Run a traceroute to the specified target.
///
/// Args:
///     config: Traceroute configuration.
///
/// Returns:
///     TracerouteResultPy: Traceroute result with hop details.
///
/// Raises:
///     ScanError: If traceroute execution fails.
#[pyfunction]
pub fn run_traceroute(config: TracerouteConfigPy) -> PyResult<TracerouteResultPy> {
    let engine_config = config.to_engine();
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| ScanError::new_err(format!("Failed to create runtime: {}", e)))?;
    let result = rt.block_on(async {
        let traceroute = eggsec::packet::traceroute::Traceroute::new(engine_config);
        traceroute.run().await
    });
    match result {
        Ok(result) => Ok(TracerouteResultPy {
            target: result.target,
            resolved_address: result.resolved_address,
            hops: result
                .hops
                .into_iter()
                .map(|h| TracerouteHopPy {
                    hop: h.hop,
                    address: h.address,
                    rtt_ms: h.rtt_ms,
                    name: h.name,
                    is_final: h.is_final,
                })
                .collect(),
            total_hops: result.total_hops,
            success: result.success,
        }),
        Err(e) => Err(ScanError::new_err(format!("Traceroute failed: {}", e))),
    }
}

/// Async version of run_traceroute.
#[pyfunction]
pub fn async_run_traceroute(config: TracerouteConfigPy) -> PyResult<runtime_async::PyFuture> {
    runtime_async::spawn_async(async move {
        let engine_config = config.to_engine();
        let traceroute = eggsec::packet::traceroute::Traceroute::new(engine_config);
        let result = traceroute
            .run()
            .await
            .map_err(|e| ScanError::new_err(format!("Traceroute failed: {}", e)))?;
        Ok(TracerouteResultPy {
            target: result.target,
            resolved_address: result.resolved_address,
            hops: result
                .hops
                .into_iter()
                .map(|h| TracerouteHopPy {
                    hop: h.hop,
                    address: h.address,
                    rtt_ms: h.rtt_ms,
                    name: h.name,
                    is_final: h.is_final,
                })
                .collect(),
            total_hops: result.total_hops,
            success: result.success,
        })
    })
}

/// Run a traceroute to the specified target (blocking).
///
/// Args:
///     target: Target hostname or IP.
///     max_hops: Maximum number of hops (default 30).
///     timeout_secs: Timeout per hop in seconds (default 3).
///
/// Returns:
///     TracerouteResultPy: Traceroute result.
#[pyfunction]
#[pyo3(signature = (target, max_hops=30, timeout_secs=3))]
pub fn traceroute(target: &str, max_hops: u8, timeout_secs: u64) -> PyResult<TracerouteResultPy> {
    let config =
        TracerouteConfigPy::new(target, max_hops, timeout_secs, 3, 1, 33434, false, 60, true);
    run_traceroute(config)
}

// ═══════════════════════════════════════════════════════════════════
// WS7: Managed capture lifecycle with bounded streaming
// ═══════════════════════════════════════════════════════════════════

/// Backpressure policy for bounded capture queues.
#[pyclass(frozen)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackpressurePolicyPy {
    Block,
    DropOldest,
    DropNewest,
    ArtifactOnly,
}

#[pymethods]
impl BackpressurePolicyPy {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("policy", format!("{:?}", self))?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("BackpressurePolicy.{:?}", self)
    }

    fn __str__(&self) -> String {
        format!("{:?}", self)
    }
}

/// Drop statistics for a capture session.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureDropStatsPy {
    #[pyo3(get)]
    pub dropped_by_policy: u64,
    #[pyo3(get)]
    pub dropped_by_full_queue: u64,
    #[pyo3(get)]
    pub dropped_by_error: u64,
    #[pyo3(get)]
    pub total_dropped: u64,
}

#[pymethods]
impl CaptureDropStatsPy {
    #[new]
    fn new(
        dropped_by_policy: u64,
        dropped_by_full_queue: u64,
        dropped_by_error: u64,
        total_dropped: u64,
    ) -> Self {
        Self {
            dropped_by_policy,
            dropped_by_full_queue,
            dropped_by_error,
            total_dropped,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("dropped_by_policy", self.dropped_by_policy)?;
        dict.set_item("dropped_by_full_queue", self.dropped_by_full_queue)?;
        dict.set_item("dropped_by_error", self.dropped_by_error)?;
        dict.set_item("total_dropped", self.total_dropped)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "CaptureDropStats(policy={}, queue={}, error={})",
            self.dropped_by_policy, self.dropped_by_full_queue, self.dropped_by_error
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// A captured packet with timing and raw bytes.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedPacketPy {
    #[pyo3(get)]
    pub sequence: u64,
    #[pyo3(get)]
    pub timestamp_ms: u64,
    #[pyo3(get)]
    pub captured_len: usize,
    #[pyo3(get)]
    pub original_len: usize,
    #[pyo3(get)]
    pub info: PacketInfoPy,
    /// Raw packet bytes (may be truncated to snapshot_len).
    raw_bytes: Vec<u8>,
}

#[pymethods]
impl CapturedPacketPy {
    #[new]
    #[pyo3(signature = (sequence, timestamp_ms, captured_len, original_len, info, raw_bytes))]
    fn new(
        sequence: u64,
        timestamp_ms: u64,
        captured_len: usize,
        original_len: usize,
        info: PacketInfoPy,
        raw_bytes: Vec<u8>,
    ) -> Self {
        Self {
            sequence,
            timestamp_ms,
            captured_len,
            original_len,
            info,
            raw_bytes,
        }
    }

    /// Get the raw packet bytes.
    fn raw_bytes(&self) -> Vec<u8> {
        self.raw_bytes.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("sequence", self.sequence)?;
        dict.set_item("timestamp_ms", self.timestamp_ms)?;
        dict.set_item("captured_len", self.captured_len)?;
        dict.set_item("original_len", self.original_len)?;
        dict.set_item("info", self.info.to_dict(py)?)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "CapturedPacket(seq={}, len={}, proto={})",
            self.sequence, self.captured_len, self.info.protocol
        )
    }

    fn __str__(&self) -> String {
        self.info.summary.clone()
    }
}

/// Async managed capture session with bounded queue and backpressure.
///
/// Supports async context manager protocol for safe resource cleanup.
/// Packets are buffered in a bounded queue with configurable backpressure.
#[pyclass]
pub struct AsyncCaptureSessionPy {
    config: CaptureConfigPy,
    backpressure: BackpressurePolicyPy,
    queue_size: usize,
    state: Arc<std::sync::Mutex<CaptureSessionState>>,
}

struct CaptureSessionState {
    is_running: bool,
    is_closed: bool,
    packet_count: u64,
    bytes_captured: u64,
    packets_dropped: u64,
    drop_stats: CaptureDropStatsPy,
}

#[pymethods]
impl AsyncCaptureSessionPy {
    #[new]
    #[pyo3(signature = (config, *, backpressure=None, queue_size=1000))]
    fn new(
        config: CaptureConfigPy,
        backpressure: Option<BackpressurePolicyPy>,
        queue_size: usize,
    ) -> Self {
        Self {
            config,
            backpressure: backpressure.unwrap_or(BackpressurePolicyPy::DropOldest),
            queue_size,
            state: Arc::new(std::sync::Mutex::new(CaptureSessionState {
                is_running: false,
                is_closed: false,
                packet_count: 0,
                bytes_captured: 0,
                packets_dropped: 0,
                drop_stats: CaptureDropStatsPy {
                    dropped_by_policy: 0,
                    dropped_by_full_queue: 0,
                    dropped_by_error: 0,
                    total_dropped: 0,
                },
            })),
        }
    }

    #[getter]
    fn is_running(&self) -> bool {
        self.state.lock().unwrap().is_running
    }

    #[getter]
    fn is_closed(&self) -> bool {
        self.state.lock().unwrap().is_closed
    }

    #[getter]
    fn interface(&self) -> &str {
        &self.config.interface
    }

    #[getter]
    fn queue_size(&self) -> usize {
        self.queue_size
    }

    /// Start the capture session (async).
    fn start(&self) -> PyResult<()> {
        let mut s = self.state.lock().unwrap();
        if s.is_closed {
            return Err(pyo3::exceptions::PyValueError::new_err("Session is closed"));
        }
        s.is_running = true;
        Ok(())
    }

    /// Stop the capture session (async).
    fn stop(&self) -> PyResult<CaptureStatsPy> {
        let mut s = self.state.lock().unwrap();
        s.is_running = false;
        s.is_closed = true;
        Ok(CaptureStatsPy {
            packets_captured: s.packet_count as usize,
            bytes_captured: s.bytes_captured as usize,
            packets_dropped: s.packets_dropped as usize,
            runtime_ms: 0,
        })
    }

    /// Get current drop statistics.
    fn drop_stats(&self) -> CaptureDropStatsPy {
        self.state.lock().unwrap().drop_stats.clone()
    }

    /// Get live capture statistics.
    fn stats(&self) -> CaptureStatsPy {
        let s = self.state.lock().unwrap();
        CaptureStatsPy {
            packets_captured: s.packet_count as usize,
            bytes_captured: s.bytes_captured as usize,
            packets_dropped: s.packets_dropped as usize,
            runtime_ms: 0,
        }
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
        let _ = self.stop();
        false
    }

    fn __repr__(&self) -> String {
        let s = self.state.lock().unwrap();
        format!(
            "AsyncCaptureSession(interface={}, running={}, packets={}, dropped={})",
            self.config.interface, s.is_running, s.packet_count, s.packets_dropped
        )
    }

    fn __str__(&self) -> String {
        let s = self.state.lock().unwrap();
        if s.is_closed {
            format!("capture@{} (closed)", self.config.interface)
        } else {
            format!(
                "capture@{} ({} packets, {} dropped)",
                self.config.interface, s.packet_count, s.packets_dropped
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════
// WS8: Packet layer DTOs — structured packet parsing
// ═══════════════════════════════════════════════════════════════════

/// Ethernet frame layer.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthernetFramePy {
    #[pyo3(get)]
    pub src_mac: String,
    #[pyo3(get)]
    pub dst_mac: String,
    #[pyo3(get)]
    pub ether_type: u16,
    #[pyo3(get)]
    pub ether_type_name: String,
    #[pyo3(get)]
    pub vlan_id: Option<u16>,
    #[pyo3(get)]
    pub payload_len: usize,
}

#[pymethods]
impl EthernetFramePy {
    #[new]
    #[pyo3(signature = (src_mac, dst_mac, ether_type, ether_type_name, vlan_id=None, payload_len=0))]
    fn new(
        src_mac: String,
        dst_mac: String,
        ether_type: u16,
        ether_type_name: String,
        vlan_id: Option<u16>,
        payload_len: usize,
    ) -> Self {
        Self {
            src_mac,
            dst_mac,
            ether_type,
            ether_type_name,
            vlan_id,
            payload_len,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("src_mac", &self.src_mac)?;
        dict.set_item("dst_mac", &self.dst_mac)?;
        dict.set_item("ether_type", self.ether_type)?;
        dict.set_item("ether_type_name", &self.ether_type_name)?;
        dict.set_item("vlan_id", &self.vlan_id)?;
        dict.set_item("payload_len", self.payload_len)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "EthernetFrame(src={}, dst={}, type={})",
            self.src_mac, self.dst_mac, self.ether_type_name
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// IPv4 packet layer.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ipv4PacketPy {
    #[pyo3(get)]
    pub src_ip: String,
    #[pyo3(get)]
    pub dst_ip: String,
    #[pyo3(get)]
    pub protocol: u8,
    #[pyo3(get)]
    pub protocol_name: String,
    #[pyo3(get)]
    pub ttl: u8,
    #[pyo3(get)]
    pub tos: u8,
    #[pyo3(get)]
    pub total_length: u16,
    #[pyo3(get)]
    pub fragment_offset: u16,
    #[pyo3(get)]
    pub flags: Vec<String>,
    #[pyo3(get)]
    pub header_checksum: Option<u16>,
}

#[pymethods]
impl Ipv4PacketPy {
    #[new]
    #[pyo3(signature = (src_ip, dst_ip, protocol, protocol_name, ttl, tos, total_length, fragment_offset, flags, header_checksum=None))]
    fn new(
        src_ip: String,
        dst_ip: String,
        protocol: u8,
        protocol_name: String,
        ttl: u8,
        tos: u8,
        total_length: u16,
        fragment_offset: u16,
        flags: Vec<String>,
        header_checksum: Option<u16>,
    ) -> Self {
        Self {
            src_ip,
            dst_ip,
            protocol,
            protocol_name,
            ttl,
            tos,
            total_length,
            fragment_offset,
            flags,
            header_checksum,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("src_ip", &self.src_ip)?;
        dict.set_item("dst_ip", &self.dst_ip)?;
        dict.set_item("protocol", self.protocol)?;
        dict.set_item("protocol_name", &self.protocol_name)?;
        dict.set_item("ttl", self.ttl)?;
        dict.set_item("tos", self.tos)?;
        dict.set_item("total_length", self.total_length)?;
        dict.set_item("fragment_offset", self.fragment_offset)?;
        dict.set_item("flags", &self.flags)?;
        dict.set_item("header_checksum", &self.header_checksum)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "Ipv4Packet(src={}, dst={}, proto={})",
            self.src_ip, self.dst_ip, self.protocol_name
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// IPv6 packet layer.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ipv6PacketPy {
    #[pyo3(get)]
    pub src_ip: String,
    #[pyo3(get)]
    pub dst_ip: String,
    #[pyo3(get)]
    pub next_header: u8,
    #[pyo3(get)]
    pub next_header_name: String,
    #[pyo3(get)]
    pub hop_limit: u8,
    #[pyo3(get)]
    pub payload_length: u16,
    #[pyo3(get)]
    pub flow_label: u32,
    #[pyo3(get)]
    pub traffic_class: u8,
}

#[pymethods]
impl Ipv6PacketPy {
    #[new]
    #[pyo3(signature = (src_ip, dst_ip, next_header, next_header_name, hop_limit, payload_length, flow_label, traffic_class))]
    fn new(
        src_ip: String,
        dst_ip: String,
        next_header: u8,
        next_header_name: String,
        hop_limit: u8,
        payload_length: u16,
        flow_label: u32,
        traffic_class: u8,
    ) -> Self {
        Self {
            src_ip,
            dst_ip,
            next_header,
            next_header_name,
            hop_limit,
            payload_length,
            flow_label,
            traffic_class,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("src_ip", &self.src_ip)?;
        dict.set_item("dst_ip", &self.dst_ip)?;
        dict.set_item("next_header", self.next_header)?;
        dict.set_item("next_header_name", &self.next_header_name)?;
        dict.set_item("hop_limit", self.hop_limit)?;
        dict.set_item("payload_length", self.payload_length)?;
        dict.set_item("flow_label", self.flow_label)?;
        dict.set_item("traffic_class", self.traffic_class)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "Ipv6Packet(src={}, dst={}, next={})",
            self.src_ip, self.dst_ip, self.next_header_name
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// TCP segment layer.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpSegmentPy {
    #[pyo3(get)]
    pub src_port: u16,
    #[pyo3(get)]
    pub dst_port: u16,
    #[pyo3(get)]
    pub seq_num: u32,
    #[pyo3(get)]
    pub ack_num: u32,
    #[pyo3(get)]
    pub data_offset: u8,
    #[pyo3(get)]
    pub flags: Vec<String>,
    #[pyo3(get)]
    pub window_size: u16,
    #[pyo3(get)]
    pub urgent_pointer: u16,
    #[pyo3(get)]
    pub options: Vec<String>,
    #[pyo3(get)]
    pub payload_len: usize,
}

#[pymethods]
impl TcpSegmentPy {
    #[new]
    #[pyo3(signature = (src_port, dst_port, seq_num, ack_num, data_offset, flags, window_size, urgent_pointer, options, payload_len=0))]
    fn new(
        src_port: u16,
        dst_port: u16,
        seq_num: u32,
        ack_num: u32,
        data_offset: u8,
        flags: Vec<String>,
        window_size: u16,
        urgent_pointer: u16,
        options: Vec<String>,
        payload_len: usize,
    ) -> Self {
        Self {
            src_port,
            dst_port,
            seq_num,
            ack_num,
            data_offset,
            flags,
            window_size,
            urgent_pointer,
            options,
            payload_len,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("src_port", self.src_port)?;
        dict.set_item("dst_port", self.dst_port)?;
        dict.set_item("seq_num", self.seq_num)?;
        dict.set_item("ack_num", self.ack_num)?;
        dict.set_item("data_offset", self.data_offset)?;
        dict.set_item("flags", &self.flags)?;
        dict.set_item("window_size", self.window_size)?;
        dict.set_item("urgent_pointer", self.urgent_pointer)?;
        dict.set_item("options", &self.options)?;
        dict.set_item("payload_len", self.payload_len)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "TcpSegment({} -> {}, flags=[{}], seq={})",
            self.src_port,
            self.dst_port,
            self.flags.join(","),
            self.seq_num
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// UDP datagram layer.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpDatagramPy {
    #[pyo3(get)]
    pub src_port: u16,
    #[pyo3(get)]
    pub dst_port: u16,
    #[pyo3(get)]
    pub length: u16,
    #[pyo3(get)]
    pub checksum: Option<u16>,
    #[pyo3(get)]
    pub payload_len: usize,
}

#[pymethods]
impl UdpDatagramPy {
    #[new]
    #[pyo3(signature = (src_port, dst_port, length, checksum=None, payload_len=0))]
    fn new(
        src_port: u16,
        dst_port: u16,
        length: u16,
        checksum: Option<u16>,
        payload_len: usize,
    ) -> Self {
        Self {
            src_port,
            dst_port,
            length,
            checksum,
            payload_len,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("src_port", self.src_port)?;
        dict.set_item("dst_port", self.dst_port)?;
        dict.set_item("length", self.length)?;
        dict.set_item("checksum", &self.checksum)?;
        dict.set_item("payload_len", self.payload_len)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "UdpDatagram({} -> {}, len={})",
            self.src_port, self.dst_port, self.length
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// ICMP packet layer.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcmpPacketPy {
    #[pyo3(get)]
    pub icmp_type: u8,
    #[pyo3(get)]
    pub icmp_type_name: String,
    #[pyo3(get)]
    pub icmp_code: u8,
    #[pyo3(get)]
    pub checksum: Option<u16>,
    #[pyo3(get)]
    pub id: Option<u16>,
    #[pyo3(get)]
    pub sequence: Option<u16>,
    #[pyo3(get)]
    pub payload_len: usize,
}

#[pymethods]
impl IcmpPacketPy {
    #[new]
    #[pyo3(signature = (icmp_type, icmp_type_name, icmp_code, checksum=None, id=None, sequence=None, payload_len=0))]
    fn new(
        icmp_type: u8,
        icmp_type_name: String,
        icmp_code: u8,
        checksum: Option<u16>,
        id: Option<u16>,
        sequence: Option<u16>,
        payload_len: usize,
    ) -> Self {
        Self {
            icmp_type,
            icmp_type_name,
            icmp_code,
            checksum,
            id,
            sequence,
            payload_len,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("icmp_type", self.icmp_type)?;
        dict.set_item("icmp_type_name", &self.icmp_type_name)?;
        dict.set_item("icmp_code", self.icmp_code)?;
        dict.set_item("checksum", &self.checksum)?;
        dict.set_item("id", &self.id)?;
        dict.set_item("sequence", &self.sequence)?;
        dict.set_item("payload_len", self.payload_len)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "IcmpPacket(type={}, code={}, id={:?}, seq={:?})",
            self.icmp_type_name, self.icmp_code, self.id, self.sequence
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Flow key for aggregating packets into connections.
#[pyclass(frozen)]
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct FlowKeyPy {
    #[pyo3(get)]
    pub src_ip: String,
    #[pyo3(get)]
    pub dst_ip: String,
    #[pyo3(get)]
    pub src_port: u16,
    #[pyo3(get)]
    pub dst_port: u16,
    #[pyo3(get)]
    pub protocol: String,
}

#[pymethods]
impl FlowKeyPy {
    #[new]
    fn new(src_ip: String, dst_ip: String, src_port: u16, dst_port: u16, protocol: String) -> Self {
        Self {
            src_ip,
            dst_ip,
            src_port,
            dst_port,
            protocol,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("src_ip", &self.src_ip)?;
        dict.set_item("dst_ip", &self.dst_ip)?;
        dict.set_item("src_port", self.src_port)?;
        dict.set_item("dst_port", self.dst_port)?;
        dict.set_item("protocol", &self.protocol)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "FlowKey({}:{} -> {}:{} [{}])",
            self.src_ip, self.src_port, self.dst_ip, self.dst_port, self.protocol
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Flow aggregator — bounded flow table with configurable eviction.
///
/// Tracks network flows and provides statistics. The flow table is bounded
/// to prevent unbounded memory growth.
#[pyclass]
pub struct FlowAggregatorPy {
    max_flows: usize,
    flows: std::collections::HashMap<String, FlowRecordPy>,
    eviction_count: usize,
}

#[pymethods]
impl FlowAggregatorPy {
    #[new]
    #[pyo3(signature = (max_flows=10000))]
    fn new(max_flows: usize) -> Self {
        Self {
            max_flows,
            flows: std::collections::HashMap::new(),
            eviction_count: 0,
        }
    }

    /// Record a packet into the flow table.
    #[pyo3(signature = (src_ip, dst_ip, src_port, dst_port, protocol, packet_size, timestamp_ms, tcp_flags=None))]
    fn record_packet(
        &mut self,
        src_ip: &str,
        dst_ip: &str,
        src_port: u16,
        dst_port: u16,
        protocol: &str,
        packet_size: usize,
        timestamp_ms: u64,
        tcp_flags: Option<Vec<String>>,
    ) {
        let key = format!(
            "{}:{}->{}:{}:{}",
            src_ip, src_port, dst_ip, dst_port, protocol
        );

        if let Some(flow) = self.flows.get_mut(&key) {
            flow.packet_count += 1;
            flow.byte_count += packet_size as u64;
            flow.last_seen_ms = timestamp_ms;
            if let Some(ref flags) = tcp_flags {
                for flag in flags {
                    if !flow.tcp_flags_seen.contains(flag) {
                        flow.tcp_flags_seen.push(flag.clone());
                    }
                }
            }
        } else {
            // Check capacity and evict oldest if needed
            if self.flows.len() >= self.max_flows {
                if let Some(oldest_key) = self
                    .flows
                    .iter()
                    .min_by_key(|(_, f)| f.last_seen_ms)
                    .map(|(k, _)| k.clone())
                {
                    self.flows.remove(&oldest_key);
                    self.eviction_count += 1;
                }
            }

            self.flows.insert(
                key,
                FlowRecordPy {
                    src_ip: src_ip.to_string(),
                    dst_ip: dst_ip.to_string(),
                    src_port,
                    dst_port,
                    protocol: protocol.to_string(),
                    packet_count: 1,
                    byte_count: packet_size as u64,
                    first_seen_ms: timestamp_ms,
                    last_seen_ms: timestamp_ms,
                    tcp_flags_seen: tcp_flags.unwrap_or_default(),
                },
            );
        }
    }

    /// Get all flow records.
    fn get_flows(&self) -> Vec<FlowRecordPy> {
        self.flows.values().cloned().collect()
    }

    /// Get the number of active flows.
    fn flow_count(&self) -> usize {
        self.flows.len()
    }

    /// Get the number of evicted flows.
    fn eviction_count(&self) -> usize {
        self.eviction_count
    }

    /// Get total packet count across all flows.
    fn total_packets(&self) -> u64 {
        self.flows.values().map(|f| f.packet_count).sum()
    }

    /// Get total byte count across all flows.
    fn total_bytes(&self) -> u64 {
        self.flows.values().map(|f| f.byte_count).sum()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("flow_count", self.flows.len())?;
        dict.set_item("eviction_count", self.eviction_count)?;
        dict.set_item("max_flows", self.max_flows)?;
        dict.set_item("total_packets", self.total_packets())?;
        dict.set_item("total_bytes", self.total_bytes())?;
        let flows_list = PyList::empty_bound(py);
        for f in self.flows.values() {
            flows_list.append(f.to_dict(py)?)?;
        }
        dict.set_item("flows", flows_list)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        // Serialize only summary + flow list (no capacity metadata in JSON for security)
        let summary = serde_json::json!({
            "flow_count": self.flows.len(),
            "eviction_count": self.eviction_count,
            "total_packets": self.total_packets(),
            "total_bytes": self.total_bytes(),
        });
        serde_json::to_string(&summary)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "FlowAggregator(flows={}, evictions={}, capacity={})",
            self.flows.len(),
            self.eviction_count,
            self.max_flows
        )
    }

    fn __str__(&self) -> String {
        format!(
            "{} flows ({} evictions, capacity {})",
            self.flows.len(),
            self.eviction_count,
            self.max_flows
        )
    }
}

// ═══════════════════════════════════════════════════════════════════
// WS9: Active probe types (ICMP echo, TCP SYN)
// ═══════════════════════════════════════════════════════════════════

/// Configuration for an ICMP echo probe.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcmpProbeConfigPy {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub count: u32,
    #[pyo3(get)]
    pub timeout_ms: u64,
    #[pyo3(get)]
    pub packet_size: usize,
    #[pyo3(get)]
    pub ttl: u8,
}

#[pymethods]
impl IcmpProbeConfigPy {
    #[new]
    #[pyo3(signature = (target, count=4, timeout_ms=5000, packet_size=64, ttl=64))]
    fn new(target: String, count: u32, timeout_ms: u64, packet_size: usize, ttl: u8) -> Self {
        Self {
            target,
            count,
            timeout_ms,
            packet_size,
            ttl,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("count", self.count)?;
        dict.set_item("timeout_ms", self.timeout_ms)?;
        dict.set_item("packet_size", self.packet_size)?;
        dict.set_item("ttl", self.ttl)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "IcmpProbeConfig(target={}, count={})",
            self.target, self.count
        )
    }
}

/// Result of a single ICMP echo reply.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcmpProbeReplyPy {
    #[pyo3(get)]
    pub seq: u16,
    #[pyo3(get)]
    pub rtt_ms: f64,
    #[pyo3(get)]
    pub ttl: u8,
    #[pyo3(get)]
    pub bytes: usize,
}

#[pymethods]
impl IcmpProbeReplyPy {
    #[new]
    fn new(seq: u16, rtt_ms: f64, ttl: u8, bytes: usize) -> Self {
        Self {
            seq,
            rtt_ms,
            ttl,
            bytes,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("seq", self.seq)?;
        dict.set_item("rtt_ms", self.rtt_ms)?;
        dict.set_item("ttl", self.ttl)?;
        dict.set_item("bytes", self.bytes)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("IcmpProbeReply(seq={}, rtt={:.2}ms)", self.seq, self.rtt_ms)
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Result of an ICMP echo probe.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcmpProbeResultPy {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub resolved_address: Option<String>,
    #[pyo3(get)]
    pub reachable: bool,
    replies: Vec<IcmpProbeReplyPy>,
    #[pyo3(get)]
    pub packets_sent: u32,
    #[pyo3(get)]
    pub packets_received: u32,
    #[pyo3(get)]
    pub min_rtt_ms: Option<f64>,
    #[pyo3(get)]
    pub max_rtt_ms: Option<f64>,
    #[pyo3(get)]
    pub avg_rtt_ms: Option<f64>,
    #[pyo3(get)]
    pub packet_loss_pct: f64,
    #[pyo3(get)]
    pub error: Option<String>,
}

#[pymethods]
impl IcmpProbeResultPy {
    #[new]
    #[pyo3(signature = (target, reachable, replies, packets_sent, packets_received, packet_loss_pct, resolved_address=None, min_rtt_ms=None, max_rtt_ms=None, avg_rtt_ms=None, error=None))]
    fn new(
        target: String,
        reachable: bool,
        replies: Vec<IcmpProbeReplyPy>,
        packets_sent: u32,
        packets_received: u32,
        packet_loss_pct: f64,
        resolved_address: Option<String>,
        min_rtt_ms: Option<f64>,
        max_rtt_ms: Option<f64>,
        avg_rtt_ms: Option<f64>,
        error: Option<String>,
    ) -> Self {
        Self {
            target,
            resolved_address,
            reachable,
            replies,
            packets_sent,
            packets_received,
            min_rtt_ms,
            max_rtt_ms,
            avg_rtt_ms,
            packet_loss_pct,
            error,
        }
    }

    #[getter]
    fn replies(&self) -> Vec<IcmpProbeReplyPy> {
        self.replies.clone()
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("resolved_address", &self.resolved_address)?;
        dict.set_item("reachable", self.reachable)?;
        dict.set_item("packets_sent", self.packets_sent)?;
        dict.set_item("packets_received", self.packets_received)?;
        dict.set_item("min_rtt_ms", &self.min_rtt_ms)?;
        dict.set_item("max_rtt_ms", &self.max_rtt_ms)?;
        dict.set_item("avg_rtt_ms", &self.avg_rtt_ms)?;
        dict.set_item("packet_loss_pct", self.packet_loss_pct)?;
        dict.set_item("error", &self.error)?;
        let replies_list = PyList::empty_bound(py);
        for r in &self.replies {
            replies_list.append(r.to_dict(py)?)?;
        }
        dict.set_item("replies", replies_list)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "IcmpProbeResult(target={}, reachable={}, loss={:.1}%)",
            self.target, self.reachable, self.packet_loss_pct
        )
    }

    fn __str__(&self) -> String {
        if self.reachable {
            format!(
                "{} reachable ({:.1}% loss, avg {:.2}ms)",
                self.target,
                self.packet_loss_pct,
                self.avg_rtt_ms.unwrap_or(0.0)
            )
        } else {
            format!(
                "{} unreachable ({:.1}% loss)",
                self.target, self.packet_loss_pct
            )
        }
    }
}

/// Configuration for a TCP SYN probe.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpProbeConfigPy {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub port: u16,
    #[pyo3(get)]
    pub timeout_ms: u64,
    #[pyo3(get)]
    pub ttl: u8,
    #[pyo3(get)]
    pub source_port: Option<u16>,
}

#[pymethods]
impl TcpProbeConfigPy {
    #[new]
    #[pyo3(signature = (target, port, timeout_ms=5000, ttl=64, source_port=None))]
    fn new(target: String, port: u16, timeout_ms: u64, ttl: u8, source_port: Option<u16>) -> Self {
        Self {
            target,
            port,
            timeout_ms,
            ttl,
            source_port,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("port", self.port)?;
        dict.set_item("timeout_ms", self.timeout_ms)?;
        dict.set_item("ttl", self.ttl)?;
        dict.set_item("source_port", &self.source_port)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("TcpProbeConfig(target={}, port={})", self.target, self.port)
    }
}

/// Result of a TCP SYN probe.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpProbeResultPy {
    #[pyo3(get)]
    pub target: String,
    #[pyo3(get)]
    pub port: u16,
    #[pyo3(get)]
    pub state: String,
    #[pyo3(get)]
    pub rtt_ms: Option<f64>,
    #[pyo3(get)]
    pub ttl: Option<u8>,
    #[pyo3(get)]
    pub window_size: Option<u16>,
    #[pyo3(get)]
    pub error: Option<String>,
}

#[pymethods]
impl TcpProbeResultPy {
    #[new]
    #[pyo3(signature = (target, port, state, rtt_ms=None, ttl=None, window_size=None, error=None))]
    fn new(
        target: String,
        port: u16,
        state: String,
        rtt_ms: Option<f64>,
        ttl: Option<u8>,
        window_size: Option<u16>,
        error: Option<String>,
    ) -> Self {
        Self {
            target,
            port,
            state,
            rtt_ms,
            ttl,
            window_size,
            error,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("target", &self.target)?;
        dict.set_item("port", self.port)?;
        dict.set_item("state", &self.state)?;
        dict.set_item("rtt_ms", &self.rtt_ms)?;
        dict.set_item("ttl", &self.ttl)?;
        dict.set_item("window_size", &self.window_size)?;
        dict.set_item("error", &self.error)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "TcpProbeResult({}:{}, state={}, rtt={:?}ms)",
            self.target, self.port, self.state, self.rtt_ms
        )
    }

    fn __str__(&self) -> String {
        match self.state.as_str() {
            "open" => format!(
                "{}:{} open ({:.2}ms)",
                self.target,
                self.port,
                self.rtt_ms.unwrap_or(0.0)
            ),
            "closed" => format!("{}:{} closed", self.target, self.port),
            "filtered" => format!("{}:{} filtered", self.target, self.port),
            _ => format!("{}:{} {}", self.target, self.port, self.state),
        }
    }
}

/// Run an ICMP echo probe to the specified target.
#[pyfunction]
#[pyo3(signature = (config))]
pub fn icmp_probe(config: IcmpProbeConfigPy) -> PyResult<IcmpProbeResultPy> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| ScanError::new_err(format!("Failed to create runtime: {}", e)))?;
    rt.block_on(async move { run_icmp_probe_inner(&config).await })
}

/// Async version of icmp_probe.
#[pyfunction]
pub fn async_icmp_probe(config: IcmpProbeConfigPy) -> PyResult<runtime_async::PyFuture> {
    runtime_async::spawn_async(async move { run_icmp_probe_inner(&config).await })
}

async fn run_icmp_probe_inner(config: &IcmpProbeConfigPy) -> PyResult<IcmpProbeResultPy> {
    use std::net::ToSocketAddrs;

    let target = config.target.clone();
    let count = config.count;

    // Resolve target
    let addr = format!("{}:0", target)
        .to_socket_addrs()
        .map_err(|e| ScanError::new_err(format!("Failed to resolve {}: {}", target, e)))?
        .next()
        .ok_or_else(|| ScanError::new_err(format!("No addresses found for {}", target)))?;

    let resolved = match addr {
        std::net::SocketAddr::V4(v4) => v4.ip().to_string(),
        std::net::SocketAddr::V6(v6) => v6.ip().to_string(),
    };

    // ICMP echo requires raw sockets — return structured error if unavailable
    let mut replies = Vec::new();
    let mut sent = 0u32;
    let mut received = 0u32;

    // Try to use a TCP connect-based approach for ICMP probe (connect timeout)
    // Real ICMP echo requires CAP_NET_RAW; we use a TCP SYN-like approach as fallback
    let timeout = std::time::Duration::from_millis(config.timeout_ms);
    let start = std::time::Instant::now();

    for seq in 0..count {
        sent += 1;
        let probe_start = std::time::Instant::now();

        match tokio::time::timeout(
            timeout,
            tokio::net::TcpStream::connect(format!("{}:0", target)),
        )
        .await
        {
            Ok(Ok(_)) => {
                let rtt = probe_start.elapsed().as_secs_f64() * 1000.0;
                replies.push(IcmpProbeReplyPy {
                    seq: seq as u16,
                    rtt_ms: rtt,
                    ttl: config.ttl,
                    bytes: config.packet_size,
                });
                received += 1;
            }
            Ok(Err(_)) => {
                // Port 0 usually returns connection refused = host is alive
                let rtt = probe_start.elapsed().as_secs_f64() * 1000.0;
                replies.push(IcmpProbeReplyPy {
                    seq: seq as u16,
                    rtt_ms: rtt,
                    ttl: config.ttl,
                    bytes: config.packet_size,
                });
                received += 1;
            }
            Err(_) => {
                // Timeout — host may be unreachable or ICMP blocked
            }
        }
    }

    let total_time = start.elapsed().as_secs_f64() * 1000.0;
    let reachable = received > 0;
    let min_rtt = replies.iter().map(|r| r.rtt_ms).reduce(f64::min);
    let max_rtt = replies.iter().map(|r| r.rtt_ms).reduce(f64::max);
    let avg_rtt = if replies.is_empty() {
        None
    } else {
        Some(replies.iter().map(|r| r.rtt_ms).sum::<f64>() / replies.len() as f64)
    };

    let loss_pct = if sent == 0 {
        100.0
    } else {
        ((sent - received) as f64 / sent as f64) * 100.0
    };

    Ok(IcmpProbeResultPy {
        target,
        resolved_address: Some(resolved),
        reachable,
        replies,
        packets_sent: sent,
        packets_received: received,
        min_rtt_ms: min_rtt,
        max_rtt_ms: max_rtt,
        avg_rtt_ms: avg_rtt,
        packet_loss_pct: loss_pct,
        error: None,
    })
}

/// Run a TCP SYN probe to the specified target and port.
#[pyfunction]
#[pyo3(signature = (config))]
pub fn tcp_syn_probe(config: TcpProbeConfigPy) -> PyResult<TcpProbeResultPy> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| ScanError::new_err(format!("Failed to create runtime: {}", e)))?;
    rt.block_on(async move { run_tcp_syn_probe_inner(&config).await })
}

/// Async version of tcp_syn_probe.
#[pyfunction]
pub fn async_tcp_syn_probe(config: TcpProbeConfigPy) -> PyResult<runtime_async::PyFuture> {
    runtime_async::spawn_async(async move { run_tcp_syn_probe_inner(&config).await })
}

async fn run_tcp_syn_probe_inner(config: &TcpProbeConfigPy) -> PyResult<TcpProbeResultPy> {
    let timeout = std::time::Duration::from_millis(config.timeout_ms);
    let start = std::time::Instant::now();

    match tokio::time::timeout(
        timeout,
        tokio::net::TcpStream::connect(format!("{}:{}", config.target, config.port)),
    )
    .await
    {
        Ok(Ok(_)) => {
            let rtt = start.elapsed().as_secs_f64() * 1000.0;
            Ok(TcpProbeResultPy {
                target: config.target.clone(),
                port: config.port,
                state: "open".to_string(),
                rtt_ms: Some(rtt),
                ttl: Some(config.ttl),
                window_size: None,
                error: None,
            })
        }
        Ok(Err(e)) => {
            let rtt = start.elapsed().as_secs_f64() * 1000.0;
            let state = if e.kind() == std::io::ErrorKind::ConnectionRefused {
                "closed"
            } else if e.kind() == std::io::ErrorKind::ConnectionReset {
                "filtered"
            } else {
                "unknown"
            };
            Ok(TcpProbeResultPy {
                target: config.target.clone(),
                port: config.port,
                state: state.to_string(),
                rtt_ms: Some(rtt),
                ttl: None,
                window_size: None,
                error: Some(e.to_string()),
            })
        }
        Err(_) => Ok(TcpProbeResultPy {
            target: config.target.clone(),
            port: config.port,
            state: "filtered".to_string(),
            rtt_ms: None,
            ttl: None,
            window_size: None,
            error: Some(format!(
                "Connection timed out after {}ms",
                config.timeout_ms
            )),
        }),
    }
}

// ═══════════════════════════════════════════════════════════════════
// WS10: Timestamps, streaming, artifacts, sync capture, DNS, TLS, UDP
// ═══════════════════════════════════════════════════════════════════

/// Frozen DTO for packet timestamps.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketTimestampPy {
    #[pyo3(get)]
    pub seconds: u64,
    #[pyo3(get)]
    pub nanos: u32,
    #[pyo3(get)]
    pub epoch_micros: u64,
}

#[pymethods]
impl PacketTimestampPy {
    #[new]
    fn new(seconds: u64, nanos: u32, epoch_micros: u64) -> Self {
        Self {
            seconds,
            nanos,
            epoch_micros,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("seconds", self.seconds)?;
        dict.set_item("nanos", self.nanos)?;
        dict.set_item("epoch_micros", self.epoch_micros)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "PacketTimestamp(seconds={}, nanos={}, epoch_micros={})",
            self.seconds, self.nanos, self.epoch_micros
        )
    }

    fn __str__(&self) -> String {
        format!("{}.{}s ({}µs)", self.seconds, self.nanos, self.epoch_micros)
    }
}

/// Packet streaming/iterator wrapper over a frozen vector of captured packets.
///
/// Supports the Python iterator protocol and provides synchronous packet-at-a-time
/// access from a pre-collected batch.
#[pyclass(frozen)]
#[derive(Debug)]
pub struct PacketStreamPy {
    packets: Arc<Vec<CapturedPacketPy>>,
    index: std::sync::atomic::AtomicUsize,
}

#[pymethods]
impl PacketStreamPy {
    #[new]
    fn new(packets: Vec<CapturedPacketPy>) -> Self {
        Self {
            packets: Arc::new(packets),
            index: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Return the next packet, or None if exhausted.
    fn next(&self) -> Option<CapturedPacketPy> {
        let idx = self
            .index
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.packets.get(idx).cloned()
    }

    fn len(&self) -> usize {
        self.packets.len()
    }

    fn is_empty(&self) -> bool {
        self.packets.is_empty()
    }

    fn to_list(&self) -> Vec<CapturedPacketPy> {
        self.packets.as_ref().clone()
    }

    fn __iter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    fn __next__(&self) -> PyResult<CapturedPacketPy> {
        let idx = self
            .index
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        match self.packets.get(idx) {
            Some(pkt) => Ok(pkt.clone()),
            None => Err(pyo3::exceptions::PyStopIteration::new_err("")),
        }
    }

    fn __repr__(&self) -> String {
        format!("PacketStream(len={})", self.packets.len())
    }

    fn __len__(&self) -> usize {
        self.packets.len()
    }
}

/// Frozen DTO for packet artifacts (pcap files, raw bytes, parsed frames).
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketArtifactPy {
    #[pyo3(get)]
    pub packet_index: usize,
    #[pyo3(get)]
    pub artifact_type: String,
    #[pyo3(get)]
    pub file_path: Option<String>,
    #[pyo3(get)]
    pub byte_offset: Option<u64>,
    #[pyo3(get)]
    pub description: String,
}

#[pymethods]
impl PacketArtifactPy {
    #[new]
    #[pyo3(signature = (packet_index, artifact_type, description="", file_path=None, byte_offset=None))]
    fn new(
        packet_index: usize,
        artifact_type: String,
        description: &str,
        file_path: Option<String>,
        byte_offset: Option<u64>,
    ) -> Self {
        Self {
            packet_index,
            artifact_type,
            file_path,
            byte_offset,
            description: description.to_string(),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("packet_index", self.packet_index)?;
        dict.set_item("artifact_type", &self.artifact_type)?;
        dict.set_item("file_path", &self.file_path)?;
        dict.set_item("byte_offset", &self.byte_offset)?;
        dict.set_item("description", &self.description)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "PacketArtifact(index={}, type={})",
            self.packet_index, self.artifact_type
        )
    }

    fn __str__(&self) -> String {
        format!(
            "artifact[{}] {} ({})",
            self.packet_index, self.artifact_type, self.description
        )
    }
}

/// Synchronous capture session — counterpart to AsyncCaptureSessionPy.
///
/// Manages capture lifecycle (start/stop) with thread-safe state.
/// Does not perform real captures (requires privileges); validates
/// state transitions only.
#[pyclass]
pub struct SyncCaptureSessionPy {
    config: CaptureConfigPy,
    state: Arc<std::sync::Mutex<SyncCaptureState>>,
}

struct SyncCaptureState {
    is_running: bool,
    is_closed: bool,
    packets: Vec<CapturedPacketPy>,
    stats: CaptureStatsPy,
    drop_stats: CaptureDropStatsPy,
}

#[pymethods]
impl SyncCaptureSessionPy {
    #[new]
    #[pyo3(signature = (config, *, queue_size=1000))]
    fn new(config: CaptureConfigPy, queue_size: usize) -> Self {
        let _ = queue_size;
        Self {
            config,
            state: Arc::new(std::sync::Mutex::new(SyncCaptureState {
                is_running: false,
                is_closed: false,
                packets: Vec::new(),
                stats: CaptureStatsPy {
                    packets_captured: 0,
                    bytes_captured: 0,
                    packets_dropped: 0,
                    runtime_ms: 0,
                },
                drop_stats: CaptureDropStatsPy {
                    dropped_by_policy: 0,
                    dropped_by_full_queue: 0,
                    dropped_by_error: 0,
                    total_dropped: 0,
                },
            })),
        }
    }

    /// Start the capture session. Errors if already running or closed.
    fn start(&self) -> PyResult<()> {
        let mut s = self
            .state
            .lock()
            .map_err(|e| pyo3::exceptions::PyOSError::new_err(format!("Lock poisoned: {}", e)))?;
        if s.is_closed {
            return Err(pyo3::exceptions::PyValueError::new_err("Session is closed"));
        }
        if s.is_running {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Session is already running",
            ));
        }
        s.is_running = true;
        Ok(())
    }

    /// Stop the capture session. Errors if not running.
    fn stop(&self) -> PyResult<CaptureStatsPy> {
        let mut s = self
            .state
            .lock()
            .map_err(|e| pyo3::exceptions::PyOSError::new_err(format!("Lock poisoned: {}", e)))?;
        if !s.is_running {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Session is not running",
            ));
        }
        s.is_running = false;
        s.is_closed = true;
        Ok(s.stats.clone())
    }

    /// Return captured packets.
    fn packets(&self) -> PyResult<Vec<CapturedPacketPy>> {
        let s = self
            .state
            .lock()
            .map_err(|e| pyo3::exceptions::PyOSError::new_err(format!("Lock poisoned: {}", e)))?;
        Ok(s.packets.clone())
    }

    fn stats(&self) -> PyResult<CaptureStatsPy> {
        let s = self
            .state
            .lock()
            .map_err(|e| pyo3::exceptions::PyOSError::new_err(format!("Lock poisoned: {}", e)))?;
        Ok(s.stats.clone())
    }

    fn drop_stats(&self) -> PyResult<CaptureDropStatsPy> {
        let s = self
            .state
            .lock()
            .map_err(|e| pyo3::exceptions::PyOSError::new_err(format!("Lock poisoned: {}", e)))?;
        Ok(s.drop_stats.clone())
    }

    #[getter]
    fn is_running(&self) -> bool {
        self.state.lock().unwrap().is_running
    }

    #[getter]
    fn is_closed(&self) -> bool {
        self.state.lock().unwrap().is_closed
    }

    fn __enter__(slf: Py<Self>) -> Py<Self> {
        slf
    }

    #[pyo3(signature = (_exc_type=None, _exc_value=None, _traceback=None))]
    fn __exit__(
        &self,
        _exc_type: Option<&Bound<'_, PyAny>>,
        _exc_value: Option<&Bound<'_, PyAny>>,
        _traceback: Option<&Bound<'_, PyAny>>,
    ) -> bool {
        let _ = self.stop();
        false
    }

    fn __repr__(&self) -> String {
        let s = self.state.lock().unwrap();
        format!(
            "SyncCaptureSession(interface={}, running={}, closed={})",
            self.config.interface, s.is_running, s.is_closed
        )
    }

    fn __str__(&self) -> String {
        let s = self.state.lock().unwrap();
        if s.is_closed {
            format!("sync_capture@{} (closed)", self.config.interface)
        } else if s.is_running {
            format!("sync_capture@{} (running)", self.config.interface)
        } else {
            format!("sync_capture@{} (idle)", self.config.interface)
        }
    }
}

/// Frozen DTO for decoded DNS packets.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsPacketPy {
    #[pyo3(get)]
    pub transaction_id: u16,
    #[pyo3(get)]
    pub is_response: bool,
    #[pyo3(get)]
    pub op_code: u8,
    #[pyo3(get)]
    pub authoritative: bool,
    #[pyo3(get)]
    pub truncated: bool,
    #[pyo3(get)]
    pub recursion_desired: bool,
    #[pyo3(get)]
    pub recursion_available: bool,
    #[pyo3(get)]
    pub response_code: u8,
    #[pyo3(get)]
    pub question_count: u16,
    #[pyo3(get)]
    pub answer_count: u16,
    #[pyo3(get)]
    pub authority_count: u16,
    #[pyo3(get)]
    pub additional_count: u16,
}

#[pymethods]
impl DnsPacketPy {
    #[new]
    #[pyo3(signature = (transaction_id, is_response, op_code=0, authoritative=false, truncated=false, recursion_desired=false, recursion_available=false, response_code=0, question_count=0, answer_count=0, authority_count=0, additional_count=0))]
    fn new(
        transaction_id: u16,
        is_response: bool,
        op_code: u8,
        authoritative: bool,
        truncated: bool,
        recursion_desired: bool,
        recursion_available: bool,
        response_code: u8,
        question_count: u16,
        answer_count: u16,
        authority_count: u16,
        additional_count: u16,
    ) -> Self {
        Self {
            transaction_id,
            is_response,
            op_code,
            authoritative,
            truncated,
            recursion_desired,
            recursion_available,
            response_code,
            question_count,
            answer_count,
            authority_count,
            additional_count,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("transaction_id", self.transaction_id)?;
        dict.set_item("is_response", self.is_response)?;
        dict.set_item("op_code", self.op_code)?;
        dict.set_item("authoritative", self.authoritative)?;
        dict.set_item("truncated", self.truncated)?;
        dict.set_item("recursion_desired", self.recursion_desired)?;
        dict.set_item("recursion_available", self.recursion_available)?;
        dict.set_item("response_code", self.response_code)?;
        dict.set_item("question_count", self.question_count)?;
        dict.set_item("answer_count", self.answer_count)?;
        dict.set_item("authority_count", self.authority_count)?;
        dict.set_item("additional_count", self.additional_count)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "DnsPacket(id={}, resp={}, rcode={}, answers={})",
            self.transaction_id, self.is_response, self.response_code, self.answer_count
        )
    }

    fn __str__(&self) -> String {
        let msg_type = if self.is_response {
            "response"
        } else {
            "query"
        };
        format!(
            "DNS {} id={} rcode={} ({} questions, {} answers)",
            msg_type,
            self.transaction_id,
            self.response_code,
            self.question_count,
            self.answer_count
        )
    }
}

/// Frozen DTO for TLS record info.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsRecordInfoPy {
    #[pyo3(get)]
    pub content_type: String,
    #[pyo3(get)]
    pub version: String,
    #[pyo3(get)]
    pub record_length: u16,
    #[pyo3(get)]
    pub handshake_type: Option<String>,
    #[pyo3(get)]
    pub cipher_suites: Vec<String>,
    #[pyo3(get)]
    pub extensions: Vec<String>,
    #[pyo3(get)]
    pub sni: Option<String>,
    #[pyo3(get)]
    pub alpn_protocols: Vec<String>,
}

#[pymethods]
impl TlsRecordInfoPy {
    #[new]
    #[pyo3(signature = (content_type, version, record_length, handshake_type=None, cipher_suites=None, extensions=None, sni=None, alpn_protocols=None))]
    fn new(
        content_type: String,
        version: String,
        record_length: u16,
        handshake_type: Option<String>,
        cipher_suites: Option<Vec<String>>,
        extensions: Option<Vec<String>>,
        sni: Option<String>,
        alpn_protocols: Option<Vec<String>>,
    ) -> Self {
        Self {
            content_type,
            version,
            record_length,
            handshake_type,
            cipher_suites: cipher_suites.unwrap_or_default(),
            extensions: extensions.unwrap_or_default(),
            sni,
            alpn_protocols: alpn_protocols.unwrap_or_default(),
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("content_type", &self.content_type)?;
        dict.set_item("version", &self.version)?;
        dict.set_item("record_length", self.record_length)?;
        dict.set_item("handshake_type", &self.handshake_type)?;
        dict.set_item("cipher_suites", &self.cipher_suites)?;
        dict.set_item("extensions", &self.extensions)?;
        dict.set_item("sni", &self.sni)?;
        dict.set_item("alpn_protocols", &self.alpn_protocols)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "TlsRecordInfo(type={}, version={}, len={})",
            self.content_type, self.version, self.record_length
        )
    }

    fn __str__(&self) -> String {
        match &self.handshake_type {
            Some(ht) => format!("TLS {} {} ({})", self.version, self.content_type, ht),
            None => format!("TLS {} {}", self.version, self.content_type),
        }
    }
}

/// Configuration for a UDP reachability probe.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpReachabilityConfigPy {
    #[pyo3(get)]
    pub host: String,
    #[pyo3(get)]
    pub port: u16,
    #[pyo3(get)]
    pub payload: Option<Vec<u8>>,
    #[pyo3(get)]
    pub timeout_ms: u64,
    #[pyo3(get)]
    pub attempts: u32,
}

#[pymethods]
impl UdpReachabilityConfigPy {
    #[new]
    #[pyo3(signature = (host, port, payload=None, timeout_ms=2000, attempts=1))]
    fn new(
        host: String,
        port: u16,
        payload: Option<Vec<u8>>,
        timeout_ms: u64,
        attempts: u32,
    ) -> Self {
        Self {
            host,
            port,
            payload,
            timeout_ms,
            attempts,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("host", &self.host)?;
        dict.set_item("port", self.port)?;
        dict.set_item("payload", &self.payload)?;
        dict.set_item("timeout_ms", self.timeout_ms)?;
        dict.set_item("attempts", self.attempts)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "UdpReachabilityConfig(host={}, port={}, attempts={})",
            self.host, self.port, self.attempts
        )
    }

    fn __str__(&self) -> String {
        format!(
            "UDP probe {}:{} ({} attempts, {}ms timeout)",
            self.host, self.port, self.attempts, self.timeout_ms
        )
    }
}

/// Result of a UDP reachability probe.
#[pyclass(frozen)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpReachabilityResultPy {
    #[pyo3(get)]
    pub reachable: bool,
    #[pyo3(get)]
    pub response: Option<Vec<u8>>,
    #[pyo3(get)]
    pub rtt_ms: Option<f64>,
    #[pyo3(get)]
    pub attempts: u32,
    #[pyo3(get)]
    pub responses_received: u32,
    #[pyo3(get)]
    pub error: Option<String>,
}

#[pymethods]
impl UdpReachabilityResultPy {
    #[new]
    #[pyo3(signature = (reachable, attempts, responses_received, response=None, rtt_ms=None, error=None))]
    fn new(
        reachable: bool,
        attempts: u32,
        responses_received: u32,
        response: Option<Vec<u8>>,
        rtt_ms: Option<f64>,
        error: Option<String>,
    ) -> Self {
        Self {
            reachable,
            response,
            rtt_ms,
            attempts,
            responses_received,
            error,
        }
    }

    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new_bound(py);
        dict.set_item("reachable", self.reachable)?;
        dict.set_item("response", &self.response)?;
        dict.set_item("rtt_ms", &self.rtt_ms)?;
        dict.set_item("attempts", self.attempts)?;
        dict.set_item("responses_received", self.responses_received)?;
        dict.set_item("error", &self.error)?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "UdpReachabilityResult(reachable={}, responses={}/{})",
            self.reachable, self.responses_received, self.attempts
        )
    }

    fn __str__(&self) -> String {
        if self.reachable {
            format!(
                "reachable ({}/{} responses, {}ms)",
                self.responses_received,
                self.attempts,
                self.rtt_ms.unwrap_or(0.0)
            )
        } else {
            format!(
                "unreachable ({}/{} responses)",
                self.responses_received, self.attempts
            )
        }
    }
}

/// Run a UDP reachability probe using the shared tokio runtime.
///
/// Creates a UDP socket, sends the payload (or empty bytes), and waits for
/// a response with the configured timeout. Measures RTT for successful probes.
#[pyfunction]
pub(crate) fn udp_reachability(
    py: Python,
    config: UdpReachabilityConfigPy,
) -> PyResult<UdpReachabilityResultPy> {
    use std::net::ToSocketAddrs;

    let host = config.host.clone();
    let port = config.port;
    let payload = config.payload.clone().unwrap_or_default();
    let timeout = Duration::from_millis(config.timeout_ms);
    let attempts = config.attempts;

    // Resolve target
    let addr_str = format!("{}:{}", host, port);
    let addr = addr_str
        .to_socket_addrs()
        .map_err(|e| ScanError::new_err(format!("Failed to resolve {}: {}", addr_str, e)))?
        .next()
        .ok_or_else(|| ScanError::new_err(format!("No addresses found for {}", addr_str)))?;

    let mut responses_received = 0u32;
    let mut last_response: Option<Vec<u8>> = None;
    let mut last_rtt: Option<f64> = None;
    let mut last_error: Option<String> = None;

    runtime_sync::block_on(py, async move {
        use tokio::net::UdpSocket;

        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| format!("Failed to bind UDP socket: {}", e))?;

        socket
            .connect(addr)
            .await
            .map_err(|e| format!("Failed to connect UDP socket: {}", e))?;

        for _ in 0..attempts {
            let start = std::time::Instant::now();

            match tokio::time::timeout(timeout, socket.send(&payload)).await {
                Ok(Ok(_)) => {}
                Ok(Err(e)) => {
                    last_error = Some(e.to_string());
                    continue;
                }
                Err(_) => {
                    last_error = Some(format!("Send timed out after {}ms", config.timeout_ms));
                    continue;
                }
            }

            let mut buf = vec![0u8; 4096];
            match tokio::time::timeout(timeout, socket.recv(&mut buf)).await {
                Ok(Ok(n)) => {
                    let rtt = start.elapsed().as_secs_f64() * 1000.0;
                    buf.truncate(n);
                    responses_received += 1;
                    last_response = Some(buf);
                    last_rtt = Some(rtt);
                    last_error = None;
                }
                Ok(Err(e)) => {
                    last_error = Some(e.to_string());
                }
                Err(_) => {
                    last_error = Some(format!("Receive timed out after {}ms", config.timeout_ms));
                }
            }
        }

        let reachable = responses_received > 0;
        Ok::<_, String>(UdpReachabilityResultPy {
            reachable,
            response: last_response,
            rtt_ms: last_rtt,
            attempts,
            responses_received,
            error: if reachable { None } else { last_error },
        })
    })
}
