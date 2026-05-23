use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

fn calculate_ipv4_checksum(header: &[u8; 20]) -> u16 {
    let mut sum: u32 = 0;
    for i in (0..20).step_by(2) {
        let word = ((header[i] as u32) << 8) | (header[i + 1] as u32);
        sum += word;
    }
    while sum > 0xffff {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !sum as u16
}

fn compute_tcp_checksum(
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
    src_port: u16,
    dst_port: u16,
    seq: u32,
    ack: u32,
    data_offset: u8,
    flags: u8,
    window: u16,
    urgent: u16,
    options: &[u8],
    payload: &[u8],
) -> u16 {
    let tcp_header_len = 20 + options.len();
    let tcp_segment_len = tcp_header_len + payload.len();
    let mut pseudo = vec![0u8; 12 + tcp_segment_len];

    pseudo[0..4].copy_from_slice(&src_ip.octets());
    pseudo[4..8].copy_from_slice(&dst_ip.octets());
    pseudo[8] = 0;
    pseudo[9] = 6;
    pseudo[10] = (tcp_segment_len >> 8) as u8;
    pseudo[11] = (tcp_segment_len & 0xff) as u8;
    pseudo[12..14].copy_from_slice(&src_port.to_be_bytes());
    pseudo[14..16].copy_from_slice(&dst_port.to_be_bytes());
    pseudo[16..20].copy_from_slice(&seq.to_be_bytes());
    pseudo[20..24].copy_from_slice(&ack.to_be_bytes());
    pseudo[24] = data_offset;
    pseudo[25] = flags;
    pseudo[26..28].copy_from_slice(&window.to_be_bytes());
    pseudo[28..30].copy_from_slice(&0u16.to_be_bytes());
    pseudo[30..32].copy_from_slice(&urgent.to_be_bytes());
    pseudo[32..tcp_header_len].copy_from_slice(options);
    pseudo[tcp_header_len..].copy_from_slice(payload);

    let mut sum: u32 = 0;
    for i in (0..pseudo.len()).step_by(2) {
        if i + 1 < pseudo.len() {
            let word = ((pseudo[i] as u32) << 8) | (pseudo[i + 1] as u32);
            sum += word;
        } else {
            sum += (pseudo[i] as u32) << 8;
        }
    }
    while sum > 0xffff {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !sum as u16
}

#[derive(Debug, Clone, PartialEq)]
pub enum PacketValidationError {
    InvalidTtl,
    InvalidHopLimit,
    InvalidTcpOptionsLength(usize),
    PacketTooLarge { size: usize, max: usize },
    PayloadTooLarge { size: usize, max: usize },
}

impl std::fmt::Display for PacketValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PacketValidationError::InvalidTtl => write!(f, "IPv4 TTL cannot be zero"),
            PacketValidationError::InvalidHopLimit => write!(f, "IPv6 hop limit cannot be zero"),
            PacketValidationError::InvalidTcpOptionsLength(len) => {
                write!(f, "TCP options length ({}) is not a multiple of 4", len)
            }
            PacketValidationError::PacketTooLarge { size, max } => {
                write!(f, "Packet size ({}) exceeds maximum ({})", size, max)
            }
            PacketValidationError::PayloadTooLarge { size, max } => {
                write!(f, "Payload size ({}) exceeds maximum ({})", size, max)
            }
        }
    }
}

impl std::error::Error for PacketValidationError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketBuilder {
    pub ethernet: Option<EthernetBuilder>,
    pub ipv4: Option<Ipv4Builder>,
    pub ipv6: Option<Ipv6Builder>,
    pub transport: Option<TransportBuilder>,
    pub payload: Option<Vec<u8>>,
}

impl PacketBuilder {
    pub fn new() -> Self {
        Self {
            ethernet: None,
            ipv4: None,
            ipv6: None,
            transport: None,
            payload: None,
        }
    }

    pub fn ethernet(mut self, src: [u8; 6], dst: [u8; 6], ether_type: u16) -> Self {
        self.ethernet = Some(EthernetBuilder {
            src,
            dst,
            ether_type,
        });
        self
    }

    pub fn ipv4(mut self, src: Ipv4Addr, dst: Ipv4Addr, protocol: u8, ttl: u8) -> Self {
        self.ipv4 = Some(Ipv4Builder {
            src,
            dst,
            protocol,
            ttl,
            id: rand::random(),
            flags: 0,
        });
        self
    }

    pub fn ipv6(mut self, src: Ipv6Addr, dst: Ipv6Addr, next_header: u8, hop_limit: u8) -> Self {
        self.ipv6 = Some(Ipv6Builder {
            src,
            dst,
            next_header,
            hop_limit,
            traffic_class: 0,
            flow_label: 0,
        });
        self
    }

    pub fn tcp(
        mut self,
        src_port: u16,
        dst_port: u16,
        seq: u32,
        ack: u32,
        flags: TcpFlags,
        window: u16,
    ) -> Self {
        self.transport = Some(TransportBuilder::Tcp(TcpBuilder {
            src_port,
            dst_port,
            seq,
            ack,
            flags,
            window,
            urgent: 0,
            options: vec![],
        }));
        self
    }

    pub fn udp(mut self, src_port: u16, dst_port: u16) -> Self {
        self.transport = Some(TransportBuilder::Udp(UdpBuilder { src_port, dst_port }));
        self
    }

    pub fn icmp(mut self, icmp_type: u8, icmp_code: u8, identifier: u16, sequence: u16) -> Self {
        self.transport = Some(TransportBuilder::Icmp(IcmpBuilder {
            icmp_type,
            icmp_code,
            identifier,
            sequence,
        }));
        self
    }

    pub fn payload(mut self, data: Vec<u8>) -> Self {
        self.payload = Some(data);
        self
    }

    pub fn validate(&self) -> Result<(), PacketValidationError> {
        if let Some(ref ip) = self.ipv4 {
            if ip.ttl == 0 {
                return Err(PacketValidationError::InvalidTtl);
            }
            let header_len = 20;
            let payload_len = self.payload.as_ref().map(|p| p.len()).unwrap_or(0);
            if header_len + payload_len > 65535 {
                return Err(PacketValidationError::PacketTooLarge {
                    size: header_len + payload_len,
                    max: 65535,
                });
            }
            if payload_len > 65507 {
                return Err(PacketValidationError::PayloadTooLarge {
                    size: payload_len,
                    max: 65507,
                });
            }
        }

        if let Some(ref ip) = self.ipv6 {
            if ip.hop_limit == 0 {
                return Err(PacketValidationError::InvalidHopLimit);
            }
            let header_len = 40;
            let payload_len = self.payload.as_ref().map(|p| p.len()).unwrap_or(0);
            if header_len + payload_len > 65575 {
                return Err(PacketValidationError::PacketTooLarge {
                    size: header_len + payload_len,
                    max: 65575,
                });
            }
        }

        if let Some(ref trans) = self.transport {
            if let TransportBuilder::Tcp(tcp) = trans {
                let options_len = tcp.options.len();
                if options_len % 4 != 0 {
                    return Err(PacketValidationError::InvalidTcpOptionsLength(options_len));
                }
            }
        }

        Ok(())
    }

    pub fn build(&self) -> Vec<u8> {
        let mut packet = Vec::new();

        if let Some(ref eth) = self.ethernet {
            packet.extend_from_slice(&eth.to_bytes());
        }

        if let Some(ref ip) = self.ipv4 {
            packet.extend_from_slice(&ip.to_bytes());
        } else if let Some(ref ip) = self.ipv6 {
            packet.extend_from_slice(&ip.to_bytes());
        }

        let (src_ip, dst_ip) = self
            .ipv4
            .as_ref()
            .map(|ip| (IpAddr::V4(ip.src), IpAddr::V4(ip.dst)))
            .or_else(|| self.ipv6.as_ref().map(|ip| (IpAddr::V6(ip.src), IpAddr::V6(ip.dst))))
            .unwrap_or((IpAddr::V4(Ipv4Addr::UNSPECIFIED), IpAddr::V4(Ipv4Addr::UNSPECIFIED)));

        if let Some(ref trans) = self.transport {
            match trans {
                TransportBuilder::Tcp(tcp) => {
                    let payload = self.payload.as_deref().unwrap_or(&[]);
                    packet.extend_from_slice(&tcp.to_bytes(src_ip, dst_ip, payload));
                }
                TransportBuilder::Udp(udp) => {
                    packet.extend_from_slice(&udp.to_bytes());
                }
                TransportBuilder::Icmp(icmp) => {
                    packet.extend_from_slice(&icmp.to_bytes());
                }
            }
        }

        if let Some(ref payload) = self.payload {
            packet.extend_from_slice(payload);
        }

        packet
    }
}

impl Default for PacketBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthernetBuilder {
    pub src: [u8; 6],
    pub dst: [u8; 6],
    pub ether_type: u16,
}

impl EthernetBuilder {
    fn to_bytes(&self) -> [u8; 14] {
        let mut bytes = [0u8; 14];
        bytes[0..6].copy_from_slice(&self.dst);
        bytes[6..12].copy_from_slice(&self.src);
        bytes[12..14].copy_from_slice(&self.ether_type.to_be_bytes());
        bytes
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ipv4Builder {
    pub src: Ipv4Addr,
    pub dst: Ipv4Addr,
    pub protocol: u8,
    pub ttl: u8,
    pub id: u16,
    pub flags: u8,
}

impl Ipv4Builder {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = [0u8; 20];
        bytes[0] = 0x45;
        bytes[1] = self.flags << 5;
        bytes[2..4].copy_from_slice(&20u16.to_be_bytes());
        bytes[4..6].copy_from_slice(&self.id.to_be_bytes());
        bytes[6] = 0;
        bytes[7] = 0;
        bytes[8] = self.ttl;
        bytes[9] = self.protocol;
        bytes[12..16].copy_from_slice(&self.src.octets());
        bytes[16..20].copy_from_slice(&self.dst.octets());
        let checksum = calculate_ipv4_checksum(&bytes);
        bytes[10..12].copy_from_slice(&checksum.to_be_bytes());
        bytes.to_vec()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ipv6Builder {
    pub src: Ipv6Addr,
    pub dst: Ipv6Addr,
    pub next_header: u8,
    pub hop_limit: u8,
    pub traffic_class: u8,
    pub flow_label: u32,
}

impl Ipv6Builder {
    fn to_bytes(&self) -> [u8; 40] {
        let mut bytes = [0u8; 40];
        let version_traffic_class_flow =
            (6u32 << 28) | ((self.traffic_class as u32) << 20) | (self.flow_label & 0x000FFFFF);
        bytes[0..4].copy_from_slice(&version_traffic_class_flow.to_be_bytes());
        bytes[4..6].copy_from_slice(&0u16.to_be_bytes());
        bytes[6] = self.next_header;
        bytes[7] = self.hop_limit;
        bytes[8..24].copy_from_slice(&self.src.octets());
        bytes[24..40].copy_from_slice(&self.dst.octets());
        bytes
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpBuilder {
    pub src_port: u16,
    pub dst_port: u16,
    pub seq: u32,
    pub ack: u32,
    pub flags: TcpFlags,
    pub window: u16,
    pub urgent: u16,
    pub options: Vec<u8>,
}

impl TcpBuilder {
    fn to_bytes(&self, src_ip: IpAddr, dst_ip: IpAddr, payload: &[u8]) -> Vec<u8> {
        let header_len = 20 + self.options.len();
        let data_offset = ((header_len / 4) as u8) << 4;
        let mut bytes = vec![0u8; header_len];
        bytes[0..2].copy_from_slice(&self.src_port.to_be_bytes());
        bytes[2..4].copy_from_slice(&self.dst_port.to_be_bytes());
        bytes[4..8].copy_from_slice(&self.seq.to_be_bytes());
        bytes[8..12].copy_from_slice(&self.ack.to_be_bytes());
        bytes[12] = data_offset;
        bytes[13] = self.flags.to_byte();
        bytes[14..16].copy_from_slice(&self.window.to_be_bytes());
        bytes[18..20].copy_from_slice(&self.urgent.to_be_bytes());
        if !self.options.is_empty() {
            bytes[20..].copy_from_slice(&self.options);
        }

        if let (IpAddr::V4(src), IpAddr::V4(dst)) = (src_ip, dst_ip) {
            let checksum = compute_tcp_checksum(
                src,
                dst,
                self.src_port,
                self.dst_port,
                self.seq,
                self.ack,
                data_offset,
                self.flags.to_byte(),
                self.window,
                self.urgent,
                &self.options,
                payload,
            );
            bytes[16..18].copy_from_slice(&checksum.to_be_bytes());
        }

        bytes
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpBuilder {
    pub src_port: u16,
    pub dst_port: u16,
}

impl UdpBuilder {
    fn to_bytes(&self) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        bytes[0..2].copy_from_slice(&self.src_port.to_be_bytes());
        bytes[2..4].copy_from_slice(&self.dst_port.to_be_bytes());
        bytes[4..6].copy_from_slice(&8u16.to_be_bytes());
        bytes[6..8].copy_from_slice(&0u16.to_be_bytes());
        bytes
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcmpBuilder {
    pub icmp_type: u8,
    pub icmp_code: u8,
    pub identifier: u16,
    pub sequence: u16,
}

impl IcmpBuilder {
    fn to_bytes(&self) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        bytes[0] = self.icmp_type;
        bytes[1] = self.icmp_code;
        bytes[2..4].copy_from_slice(&0u16.to_be_bytes());
        bytes[4..6].copy_from_slice(&self.identifier.to_be_bytes());
        bytes[6..8].copy_from_slice(&self.sequence.to_be_bytes());
        bytes
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub fn to_byte(&self) -> u8 {
        let mut byte = 0u8;
        if self.cwr {
            byte |= 0x80;
        }
        if self.ece {
            byte |= 0x40;
        }
        if self.urg {
            byte |= 0x20;
        }
        if self.ack {
            byte |= 0x10;
        }
        if self.psh {
            byte |= 0x08;
        }
        if self.rst {
            byte |= 0x04;
        }
        if self.syn {
            byte |= 0x02;
        }
        if self.fin {
            byte |= 0x01;
        }
        byte
    }

    pub fn syn() -> Self {
        Self {
            fin: false,
            syn: true,
            rst: false,
            psh: false,
            ack: false,
            urg: false,
            ece: false,
            cwr: false,
        }
    }

    pub fn ack() -> Self {
        Self {
            fin: false,
            syn: false,
            rst: false,
            psh: false,
            ack: true,
            urg: false,
            ece: false,
            cwr: false,
        }
    }

    pub fn syn_ack() -> Self {
        Self {
            fin: false,
            syn: true,
            rst: false,
            psh: false,
            ack: true,
            urg: false,
            ece: false,
            cwr: false,
        }
    }

    pub fn fin() -> Self {
        Self {
            fin: true,
            syn: false,
            rst: false,
            psh: false,
            ack: false,
            urg: false,
            ece: false,
            cwr: false,
        }
    }

    pub fn rst() -> Self {
        Self {
            fin: false,
            syn: false,
            rst: true,
            psh: false,
            ack: false,
            urg: false,
            ece: false,
            cwr: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransportBuilder {
    Tcp(TcpBuilder),
    Udp(UdpBuilder),
    Icmp(IcmpBuilder),
}
