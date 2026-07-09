use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde::{Deserialize, Serialize};
use std::time::Duration;

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
#[pyclass]
pub struct PcapWriterPy {
    inner: eggsec::packet::capture::PcapWriter,
}

#[pymethods]
impl PcapWriterPy {
    #[new]
    fn new(path: &str, snapshot_len: usize) -> PyResult<Self> {
        let writer = eggsec::packet::capture::PcapWriter::new(path, snapshot_len)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        Ok(Self { inner: writer })
    }

    fn write_packet(&mut self, data: &[u8]) -> PyResult<()> {
        self.inner
            .write_packet(data)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    }

    fn flush(&mut self) -> PyResult<()> {
        self.inner
            .flush()
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        "PcapWriterPy".to_string()
    }

    fn __str__(&self) -> String {
        "PcapWriter".to_string()
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
