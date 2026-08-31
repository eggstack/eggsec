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

    checksum_data(&pseudo)
}

fn compute_tcp_checksum_v6(
    src_ip: Ipv6Addr,
    dst_ip: Ipv6Addr,
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
    let mut pseudo = vec![0u8; 40 + tcp_segment_len];

    pseudo[0..16].copy_from_slice(&src_ip.octets());
    pseudo[16..32].copy_from_slice(&dst_ip.octets());
    pseudo[32..36].copy_from_slice(&(tcp_segment_len as u32).to_be_bytes());
    pseudo[36] = 0;
    pseudo[37] = 0;
    pseudo[38] = 0;
    pseudo[39] = 6; // TCP next header

    pseudo[40..42].copy_from_slice(&src_port.to_be_bytes());
    pseudo[42..44].copy_from_slice(&dst_port.to_be_bytes());
    pseudo[44..48].copy_from_slice(&seq.to_be_bytes());
    pseudo[48..52].copy_from_slice(&ack.to_be_bytes());
    pseudo[52] = data_offset;
    pseudo[53] = flags;
    pseudo[54..56].copy_from_slice(&window.to_be_bytes());
    pseudo[56..58].copy_from_slice(&0u16.to_be_bytes());
    pseudo[58..60].copy_from_slice(&urgent.to_be_bytes());
    pseudo[60..tcp_header_len + 40].copy_from_slice(options);
    pseudo[tcp_header_len + 40..].copy_from_slice(payload);

    checksum_data(&pseudo)
}

fn compute_udp_checksum(
    src_ip: IpAddr,
    dst_ip: IpAddr,
    src_port: u16,
    dst_port: u16,
    payload: &[u8],
) -> u16 {
    let segment_len = 8 + payload.len();
    let mut pseudo = match (src_ip, dst_ip) {
        (IpAddr::V4(_), IpAddr::V4(_)) => vec![0u8; 12 + segment_len],
        (IpAddr::V6(_), IpAddr::V6(_)) => vec![0u8; 40 + segment_len],
        _ => return 0,
    };
    let segment_offset = match (src_ip, dst_ip) {
        (IpAddr::V4(src), IpAddr::V4(dst)) => {
            pseudo[0..4].copy_from_slice(&src.octets());
            pseudo[4..8].copy_from_slice(&dst.octets());
            pseudo[8] = 0;
            pseudo[9] = 17;
            pseudo[10..12].copy_from_slice(&(segment_len as u16).to_be_bytes());
            12
        }
        (IpAddr::V6(src), IpAddr::V6(dst)) => {
            pseudo[0..16].copy_from_slice(&src.octets());
            pseudo[16..32].copy_from_slice(&dst.octets());
            pseudo[32..36].copy_from_slice(&(segment_len as u32).to_be_bytes());
            pseudo[39] = 17;
            40
        }
        _ => return 0,
    };
    pseudo[segment_offset..segment_offset + 2].copy_from_slice(&src_port.to_be_bytes());
    pseudo[segment_offset + 2..segment_offset + 4].copy_from_slice(&dst_port.to_be_bytes());
    pseudo[segment_offset + 4..segment_offset + 6]
        .copy_from_slice(&(segment_len as u16).to_be_bytes());
    pseudo[segment_offset + 8..].copy_from_slice(payload);
    let checksum = checksum_data(&pseudo);
    if checksum == 0 {
        0xffff
    } else {
        checksum
    }
}

fn checksum_data(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    for i in (0..data.len()).step_by(2) {
        if i + 1 < data.len() {
            let word = ((data[i] as u32) << 8) | (data[i + 1] as u32);
            sum += word;
        } else {
            sum += (data[i] as u32) << 8;
        }
    }
    while sum > 0xffff {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !sum as u16
}

#[derive(Debug, Clone, PartialEq)]
pub enum PacketValidationError {
    AddressFamilyMismatch,
    InvalidTtl,
    InvalidHopLimit,
    InvalidTcpOptionsLength(usize),
    PacketTooLarge { size: usize, max: usize },
    PayloadTooLarge { size: usize, max: usize },
}

impl std::fmt::Display for PacketValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PacketValidationError::AddressFamilyMismatch => {
                write!(f, "IPv4 and IPv6 headers cannot be combined")
            }
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
        if self.ipv4.is_some() && self.ipv6.is_some() {
            return Err(PacketValidationError::AddressFamilyMismatch);
        }

        let payload_len = self.payload.as_ref().map_or(0, Vec::len);
        let transport_len = self.transport_header_len();
        let transport_and_payload_len = transport_len + payload_len;

        if transport_and_payload_len > u16::MAX as usize {
            return Err(PacketValidationError::PacketTooLarge {
                size: transport_and_payload_len,
                max: u16::MAX as usize,
            });
        }

        if let Some(ref trans) = self.transport {
            if let TransportBuilder::Tcp(tcp) = trans {
                let options_len = tcp.options.len();
                if options_len % 4 != 0 {
                    return Err(PacketValidationError::InvalidTcpOptionsLength(options_len));
                }
            }
        }

        if let Some(ref ip) = self.ipv4 {
            if ip.ttl == 0 {
                return Err(PacketValidationError::InvalidTtl);
            }
            let total_len = 20 + transport_and_payload_len;
            if total_len > u16::MAX as usize {
                return Err(PacketValidationError::PacketTooLarge {
                    size: total_len,
                    max: u16::MAX as usize,
                });
            }
            let max_payload_len = u16::MAX as usize - 20 - transport_len;
            if payload_len > max_payload_len {
                return Err(PacketValidationError::PayloadTooLarge {
                    size: payload_len,
                    max: max_payload_len,
                });
            }
        }

        if let Some(ref ip) = self.ipv6 {
            if ip.hop_limit == 0 {
                return Err(PacketValidationError::InvalidHopLimit);
            }
            if transport_and_payload_len > u16::MAX as usize {
                return Err(PacketValidationError::PacketTooLarge {
                    size: 40 + transport_and_payload_len,
                    max: 40 + u16::MAX as usize,
                });
            }
        }

        Ok(())
    }

    fn transport_header_len(&self) -> usize {
        match self.transport.as_ref() {
            Some(TransportBuilder::Tcp(tcp)) => 20 + tcp.options.len(),
            Some(TransportBuilder::Udp(_)) | Some(TransportBuilder::Icmp(_)) => 8,
            None => 0,
        }
    }

    pub fn build(&self) -> Result<Vec<u8>, PacketValidationError> {
        self.validate()?;
        let payload = self.payload.as_deref().unwrap_or(&[]);
        let transport_len = self.transport_header_len();
        let ip_payload_len = transport_len + payload.len();
        let mut packet = Vec::with_capacity(
            self.ethernet.as_ref().map_or(0, |_| 14)
                + if self.ipv4.is_some() { 20 } else { 0 }
                + if self.ipv6.is_some() { 40 } else { 0 }
                + ip_payload_len,
        );

        if let Some(ref eth) = self.ethernet {
            packet.extend_from_slice(&eth.to_bytes());
        }

        if let Some(ref ip) = self.ipv4 {
            packet.extend_from_slice(&ip.to_bytes(u16::try_from(20 + ip_payload_len).map_err(
                |_| PacketValidationError::PacketTooLarge {
                    size: 20 + ip_payload_len,
                    max: u16::MAX as usize,
                },
            )?));
        } else if let Some(ref ip) = self.ipv6 {
            packet.extend_from_slice(&ip.to_bytes(u16::try_from(ip_payload_len).map_err(
                |_| PacketValidationError::PacketTooLarge {
                    size: 40 + ip_payload_len,
                    max: 40 + u16::MAX as usize,
                },
            )?));
        }

        let (src_ip, dst_ip) = self
            .ipv4
            .as_ref()
            .map(|ip| (IpAddr::V4(ip.src), IpAddr::V4(ip.dst)))
            .or_else(|| {
                self.ipv6
                    .as_ref()
                    .map(|ip| (IpAddr::V6(ip.src), IpAddr::V6(ip.dst)))
            })
            .unwrap_or((
                IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            ));

        if let Some(ref trans) = self.transport {
            match trans {
                TransportBuilder::Tcp(tcp) => {
                    packet.extend_from_slice(&tcp.to_bytes(src_ip, dst_ip, payload)?);
                }
                TransportBuilder::Udp(udp) => {
                    packet.extend_from_slice(&udp.to_bytes(src_ip, dst_ip, payload)?);
                }
                TransportBuilder::Icmp(icmp) => {
                    packet.extend_from_slice(&icmp.to_bytes(payload));
                }
            }
        } else {
            packet.extend_from_slice(payload);
        }

        Ok(packet)
    }
}

impl Default for PacketBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn udp_checksum_is_present_for_ipv4_and_ipv6() {
        let ipv4 = PacketBuilder::new()
            .ipv4(Ipv4Addr::LOCALHOST, Ipv4Addr::LOCALHOST, 17, 64)
            .udp(1000, 1001)
            .payload(vec![1, 2, 3])
            .build()
            .unwrap();
        let ipv6 = PacketBuilder::new()
            .ipv6(Ipv6Addr::LOCALHOST, Ipv6Addr::LOCALHOST, 17, 64)
            .udp(1000, 1001)
            .payload(vec![1, 2, 3])
            .build()
            .unwrap();

        assert_ne!(&ipv4[26..28], &[0, 0]);
        assert_ne!(&ipv6[46..48], &[0, 0]);
    }

    #[test]
    fn mixed_address_families_are_rejected() {
        let builder = PacketBuilder::new()
            .ipv4(Ipv4Addr::LOCALHOST, Ipv4Addr::LOCALHOST, 6, 64)
            .ipv6(Ipv6Addr::LOCALHOST, Ipv6Addr::LOCALHOST, 6, 64);
        assert_eq!(
            builder.build(),
            Err(PacketValidationError::AddressFamilyMismatch)
        );
    }

    #[test]
    fn ipv4_total_length_includes_transport_and_payload() {
        let packet = PacketBuilder::new()
            .ipv4(Ipv4Addr::LOCALHOST, Ipv4Addr::LOCALHOST, 17, 64)
            .udp(1000, 1001)
            .payload(vec![1, 2, 3])
            .build()
            .unwrap();

        assert_eq!(u16::from_be_bytes([packet[2], packet[3]]), 31);
        assert_eq!(u16::from_be_bytes([packet[24], packet[25]]), 11);
        assert_eq!(&packet[28..], &[1, 2, 3]);
    }

    #[test]
    fn transport_header_is_counted_when_validating_packet_size() {
        let result = PacketBuilder::new()
            .ipv4(Ipv4Addr::LOCALHOST, Ipv4Addr::LOCALHOST, 6, 64)
            .tcp(1000, 1001, 0, 0, TcpFlags::syn(), 65535)
            .payload(vec![0; 65_507])
            .build();

        assert_eq!(
            result,
            Err(PacketValidationError::PacketTooLarge {
                size: 65_547,
                max: 65_535,
            })
        );
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
    fn to_bytes(&self, total_len: u16) -> [u8; 20] {
        let mut bytes = [0u8; 20];
        bytes[0] = 0x45;
        bytes[1] = (self.flags & 0x07) << 5;
        bytes[2..4].copy_from_slice(&total_len.to_be_bytes());
        bytes[4..6].copy_from_slice(&self.id.to_be_bytes());
        bytes[6] = 0;
        bytes[7] = 0;
        bytes[8] = self.ttl;
        bytes[9] = self.protocol;
        bytes[12..16].copy_from_slice(&self.src.octets());
        bytes[16..20].copy_from_slice(&self.dst.octets());
        let checksum = calculate_ipv4_checksum(&bytes);
        bytes[10..12].copy_from_slice(&checksum.to_be_bytes());
        bytes
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
    fn to_bytes(&self, payload_len: u16) -> [u8; 40] {
        let mut bytes = [0u8; 40];
        let version_traffic_class_flow =
            (6u32 << 28) | ((self.traffic_class as u32) << 20) | (self.flow_label & 0x000FFFFF);
        bytes[0..4].copy_from_slice(&version_traffic_class_flow.to_be_bytes());
        bytes[4..6].copy_from_slice(&payload_len.to_be_bytes());
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
    fn to_bytes(
        &self,
        src_ip: IpAddr,
        dst_ip: IpAddr,
        payload: &[u8],
    ) -> Result<Vec<u8>, PacketValidationError> {
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

        let checksum = match (src_ip, dst_ip) {
            (IpAddr::V4(src), IpAddr::V4(dst)) => compute_tcp_checksum(
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
            ),
            (IpAddr::V6(src), IpAddr::V6(dst)) => compute_tcp_checksum_v6(
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
            ),
            _ => return Err(PacketValidationError::AddressFamilyMismatch),
        };
        bytes[16..18].copy_from_slice(&checksum.to_be_bytes());

        Ok(bytes)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpBuilder {
    pub src_port: u16,
    pub dst_port: u16,
}

impl UdpBuilder {
    fn to_bytes(
        &self,
        src_ip: IpAddr,
        dst_ip: IpAddr,
        payload: &[u8],
    ) -> Result<Vec<u8>, PacketValidationError> {
        let len = u16::try_from(8 + payload.len()).map_err(|_| {
            PacketValidationError::PacketTooLarge {
                size: 8 + payload.len(),
                max: u16::MAX as usize,
            }
        })?;
        let mut bytes = vec![0u8; 8 + payload.len()];
        bytes[0..2].copy_from_slice(&self.src_port.to_be_bytes());
        bytes[2..4].copy_from_slice(&self.dst_port.to_be_bytes());
        bytes[4..6].copy_from_slice(&len.to_be_bytes());
        let checksum = compute_udp_checksum(src_ip, dst_ip, self.src_port, self.dst_port, payload);
        bytes[6..8].copy_from_slice(&checksum.to_be_bytes());
        bytes[8..].copy_from_slice(payload);
        Ok(bytes)
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
    fn to_bytes(&self, payload: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0u8; 8 + payload.len()];
        bytes[0] = self.icmp_type;
        bytes[1] = self.icmp_code;
        bytes[4..6].copy_from_slice(&self.identifier.to_be_bytes());
        bytes[6..8].copy_from_slice(&self.sequence.to_be_bytes());
        bytes[8..].copy_from_slice(payload);

        let checksum = icmp_checksum(&bytes);
        bytes[2..4].copy_from_slice(&checksum.to_be_bytes());

        bytes
    }
}

fn icmp_checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    for i in (0..data.len()).step_by(2) {
        if i + 1 < data.len() {
            let word = ((data[i] as u32) << 8) | (data[i + 1] as u32);
            sum += word;
        } else {
            sum += (data[i] as u32) << 8;
        }
    }
    while sum > 0xffff {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !sum as u16
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
