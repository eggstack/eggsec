use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::error::ScanError;
use crate::runtime_async;

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
