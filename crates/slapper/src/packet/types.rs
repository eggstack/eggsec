use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthernetFrame {
    pub dst_mac: String,
    pub src_mac: String,
    pub ether_type: u16,
    pub ether_type_name: String,
}

impl EthernetFrame {
    pub fn header_len() -> usize {
        14
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpPacket {
    pub version: u8,
    pub header_len: u8,
    pub total_len: u16,
    pub ttl: u8,
    pub protocol: u8,
    pub protocol_name: String,
    pub src_ip: String,
    pub dst_ip: String,
    pub payload: Vec<u8>,
    pub options: Vec<IpOption>,
    pub identification: u16,
    pub flags: IpFlags,
    pub checksum: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IpFlags {
    pub reserved: bool,
    pub dont_fragment: bool,
    pub more_fragments: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpOption {
    pub code: u8,
    pub name: String,
    pub length: Option<u8>,
    pub data: Option<Vec<u8>>,
}

impl IpPacket {
    pub fn src_ip(&self) -> &str {
        &self.src_ip
    }

    pub fn dst_ip(&self) -> &str {
        &self.dst_ip
    }

    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        let version = (data[0] >> 4) & 0x0f;
        match version {
            4 => Self::parse_ipv4(data),
            6 => Self::parse_ipv6(data),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TcpFlags {
    pub fin: bool,
    pub syn: bool,
    pub rst: bool,
    pub psh: bool,
    pub ack: bool,
    pub urg: bool,
    pub ece: bool,
    pub cwr: bool,
}

impl TcpFlags {
    pub fn from_bits(bits: u8) -> Self {
        Self {
            fin: (bits & 0x01) != 0,
            syn: (bits & 0x02) != 0,
            rst: (bits & 0x04) != 0,
            psh: (bits & 0x08) != 0,
            ack: (bits & 0x10) != 0,
            urg: (bits & 0x20) != 0,
            ece: (bits & 0x40) != 0,
            cwr: (bits & 0x80) != 0,
        }
    }

    pub fn to_string(&self) -> String {
        let mut flags = Vec::new();
        if self.cwr {
            flags.push("CWR");
        }
        if self.ece {
            flags.push("ECE");
        }
        if self.urg {
            flags.push("URG");
        }
        if self.ack {
            flags.push("ACK");
        }
        if self.psh {
            flags.push("PSH");
        }
        if self.rst {
            flags.push("RST");
        }
        if self.syn {
            flags.push("SYN");
        }
        if self.fin {
            flags.push("FIN");
        }
        if flags.is_empty() {
            "None".to_string()
        } else {
            flags.join(", ")
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub seq_num: u32,
    pub ack_num: u32,
    pub data_offset: u8,
    pub flags: TcpFlags,
    pub window_size: u16,
    pub checksum: u16,
    pub urgent_ptr: u16,
    pub payload: Vec<u8>,
    pub options: Vec<TcpOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpOption {
    pub kind: u8,
    pub name: String,
    pub length: Option<u8>,
    pub data: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub length: u16,
    pub checksum: u16,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcmpHeader {
    pub icmp_type: u8,
    pub icmp_code: u8,
    pub checksum: u16,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransportProtocol {
    Tcp(TcpHeader),
    Udp(UdpHeader),
    Icmp(IcmpHeader),
    Unknown(Vec<u8>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequest {
    pub method: String,
    pub uri: String,
    pub version: String,
    pub headers: Vec<HttpHeader>,
    pub body: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponse {
    pub version: String,
    pub status_code: u16,
    pub reason_phrase: String,
    pub headers: Vec<HttpHeader>,
    pub body: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecord {
    pub transaction_id: u16,
    pub flags: String,
    pub query_type: String,
    pub questions: Vec<DnsQuestion>,
    pub answers: Vec<DnsAnswer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsQuestion {
    pub name: String,
    pub query_type: String,
    pub class: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsAnswer {
    pub name: String,
    pub record_type: String,
    pub ttl: u32,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsHandshake {
    pub handshake_type: String,
    pub version: String,
    pub client_hello: Option<TlsClientHello>,
    pub server_hello: Option<TlsServerHello>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsClientHello {
    pub session_id: Vec<u8>,
    pub cipher_suites: Vec<String>,
    pub compression_methods: Vec<String>,
    pub server_name: Option<String>,
    pub supported_versions: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsServerHello {
    pub version: String,
    pub session_id: Vec<u8>,
    pub cipher_suite: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppLayer {
    Http(HttpRequest),
    Dns(DnsRecord),
    Tls(TlsHandshake),
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPacket {
    pub ethernet: Option<EthernetFrame>,
    pub ip: Option<IpPacket>,
    pub transport: Option<TransportProtocol>,
    pub app: Option<AppLayer>,
}
