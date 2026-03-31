use crate::error::{Result, SlapperError};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

#[cfg(all(feature = "stress-testing", unix))]
use std::net::IpAddr;

#[cfg(all(feature = "stress-testing", unix))]
use pnet::datalink::{self, NetworkInterface};
#[cfg(all(feature = "stress-testing", unix))]
use pnet::packet::ip::IpNextHeaderProtocols;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecoyMode {
    Simultaneous,
    Staggered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScanType {
    Syn,
    Null,
    Fin,
    Xmas,
}

impl Default for ScanType {
    fn default() -> Self {
        Self::Syn
    }
}

impl Default for DecoyMode {
    fn default() -> Self {
        Self::Simultaneous
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpoofConfig {
    pub enabled: bool,
    pub source_ip: Option<Ipv4Addr>,
    pub ip_range: Option<String>,
    pub use_raw_sockets: bool,
    pub decoy_ips: Vec<Ipv4Addr>,
    pub decoy_count: usize,
    pub decoy_mode: DecoyMode,
    pub include_real_ip: bool,
    pub source_port: Option<u16>,
    pub random_source_port: bool,
    pub fragment: bool,
    pub scan_type: ScanType,
    pub packet_trace: Option<String>,
    pub max_rate: Option<u32>,
    pub ttl: Option<u8>,
}

impl Default for SpoofConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            source_ip: None,
            ip_range: None,
            use_raw_sockets: false,
            decoy_ips: Vec::new(),
            decoy_count: 0,
            decoy_mode: DecoyMode::Simultaneous,
            include_real_ip: false,
            source_port: None,
            random_source_port: false,
            fragment: false,
            scan_type: ScanType::Syn,
            packet_trace: None,
            max_rate: None,
            ttl: None,
        }
    }
}

impl SpoofConfig {
    pub fn from_args(
        source_ip: Option<String>,
        spoof_range: Option<String>,
        require_raw: bool,
        decoy: Option<String>,
        decoy_range: Option<String>,
        decoy_count: Option<usize>,
        decoy_mode: Option<String>,
        include_real_ip: bool,
        source_port: Option<u16>,
        random_source_port: bool,
        fragment: bool,
        scan_type: Option<String>,
        packet_trace: Option<String>,
        max_rate: Option<u32>,
        ttl: Option<u8>,
    ) -> Result<Self> {
        let enabled = source_ip.is_some()
            || spoof_range.is_some()
            || decoy.is_some()
            || decoy_range.is_some()
            || source_port.is_some()
            || random_source_port
            || fragment
            || scan_type.as_ref().map(|s| s != "syn").unwrap_or(false)
            || packet_trace.is_some()
            || max_rate.is_some()
            || ttl.is_some();

        let source_ip = if let Some(ip_str) = source_ip {
            Some(ip_str.parse::<Ipv4Addr>()?)
        } else {
            None
        };

        let use_raw_sockets = require_raw
            || source_ip.is_some()
            || decoy.is_some()
            || decoy_range.is_some()
            || packet_trace.is_some()
            || fragment;

        let scan_type = match scan_type.as_deref() {
            Some("null") | Some("n") => ScanType::Null,
            Some("fin") | Some("f") => ScanType::Fin,
            Some("xmas") | Some("x") => ScanType::Xmas,
            _ => ScanType::Syn,
        };

        if let Some(t) = ttl {
            if t == 0 {
                return Err(SlapperError::Validation(
                    "TTL must be between 1 and 255".to_string(),
                ));
            }
        }

        let mut decoy_ips = Vec::new();

        if let Some(decoy_str) = decoy {
            let parts: Vec<&str> = decoy_str.split(',').collect();
            for part in parts {
                let part = part.trim();
                if part.to_uppercase().starts_with("RANDOM") {
                    let count = if let Some(colon_pos) = part.find(':') {
                        part[colon_pos + 1..]
                            .parse()
                            .unwrap_or(decoy_count.unwrap_or(5))
                    } else {
                        decoy_count.unwrap_or(5)
                    };
                    for _ in 0..count {
                        decoy_ips.push(generate_random_ip());
                    }
                } else if part.eq_ignore_ascii_case("ME") || part.eq_ignore_ascii_case("RAND") {
                    continue;
                } else if let Ok(ip) = part.parse::<Ipv4Addr>() {
                    decoy_ips.push(ip);
                } else {
                    return Err(SlapperError::Validation(format!(
                        "Invalid decoy IP: {}",
                        part
                    )));
                }
            }
        }

        if let Some(ref range) = decoy_range {
            let count = decoy_count.unwrap_or(5);
            for _ in 0..count {
                if let Ok(ip) = random_ip_from_cidr(range) {
                    decoy_ips.push(ip);
                }
            }
        }

        let decoy_mode = match decoy_mode.as_deref() {
            Some("staggered") | Some("s") => DecoyMode::Staggered,
            _ => DecoyMode::Simultaneous,
        };

        let final_decoy_count = decoy_count.unwrap_or(decoy_ips.len());

        Ok(Self {
            enabled,
            source_ip,
            ip_range: spoof_range,
            use_raw_sockets,
            decoy_ips,
            decoy_count: final_decoy_count,
            decoy_mode,
            include_real_ip,
            source_port,
            random_source_port,
            fragment,
            scan_type,
            packet_trace,
            max_rate,
            ttl,
        })
    }

    pub fn header_value(&self) -> Result<Option<String>> {
        if self.enabled {
            if !self.decoy_ips.is_empty() {
                let idx = rand::random::<usize>() % (self.decoy_ips.len() + 1);
                if idx < self.decoy_ips.len() {
                    return Ok(Some(self.decoy_ips[idx].to_string()));
                }
            }
            if let Some(ip) = self.source_ip {
                return Ok(Some(ip.to_string()));
            }
            if let Some(range) = &self.ip_range {
                let ip = random_ip_from_cidr(range)?;
                return Ok(Some(ip.to_string()));
            }
        }
        Ok(None)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpoofStats {
    pub packets_sent: u64,
    pub packets_dropped: u64,
    pub spoofed_ips_used: usize,
    pub decoys_used: usize,
    pub unique_decoy_ips: usize,
    pub decoy_mode: String,
}

impl Default for SpoofStats {
    fn default() -> Self {
        Self {
            packets_sent: 0,
            packets_dropped: 0,
            spoofed_ips_used: 0,
            decoys_used: 0,
            unique_decoy_ips: 0,
            decoy_mode: "simultaneous".to_string(),
        }
    }
}

pub fn random_ip_from_cidr(cidr: &str) -> Result<Ipv4Addr> {
    let parts: Vec<&str> = cidr.split('/').collect();
    if parts.len() != 2 {
        return Err(SlapperError::Parse(
            "Invalid CIDR format. Use: x.x.x.x/nn".to_string(),
        ));
    }

    let base: Ipv4Addr = parts[0]
        .parse()
        .map_err(|_| SlapperError::Parse("Invalid IP address in CIDR".to_string()))?;
    let prefix: u8 = parts[1]
        .parse()
        .map_err(|_| SlapperError::Parse("Invalid prefix length".to_string()))?;

    if prefix > 32 {
        return Err(SlapperError::Parse(
            "Prefix must be between 0 and 32".to_string(),
        ));
    }

    let mut rng = rand::thread_rng();
    let base_u32 = u32::from(base);
    let host_bits = 32 - prefix;

    if host_bits == 0 {
        return Ok(base);
    }

    let min_offset: u32;
    let max_offset: u32;

    if prefix < 31 {
        min_offset = 1;
        max_offset = (1u32 << host_bits) - 2;
    } else {
        min_offset = 1;
        max_offset = 1;
    }

    if max_offset < min_offset {
        return Err(SlapperError::Parse(
            "CIDR range too small to generate valid IPs".to_string(),
        ));
    }

    let offset = rng.gen_range(min_offset..=max_offset);

    Ok(Ipv4Addr::from(base_u32 | offset))
}

pub fn generate_random_ip() -> Ipv4Addr {
    let mut rng = rand::thread_rng();
    Ipv4Addr::new(
        rng.gen_range(1..224),
        rng.gen_range(0..=255),
        rng.gen_range(0..=255),
        rng.gen_range(1..254),
    )
}

impl SpoofConfig {
    pub fn has_decoys(&self) -> bool {
        !self.decoy_ips.is_empty()
    }

    pub fn get_all_source_ips(&self, real_ip: Ipv4Addr) -> Vec<Ipv4Addr> {
        let mut ips = self.decoy_ips.clone();
        if self.include_real_ip {
            ips.insert(0, real_ip);
        }
        ips
    }
}

#[cfg(all(feature = "stress-testing", unix))]
pub fn build_tcp_packet(
    src_ip: Ipv4Addr,
    src_port: u16,
    dst_ip: Ipv4Addr,
    dst_port: u16,
    seq: u32,
    scan_type: ScanType,
    ttl: Option<u8>,
) -> Result<Vec<u8>> {
    use pnet_packet::ipv4::{checksum as ipv4_checksum, MutableIpv4Packet};
    use pnet_packet::tcp::{ipv4_checksum as tcp_ipv4_checksum, MutableTcpPacket};

    let mut buffer = vec![0u8; 20 + 20];

    let mut ipv4_packet = MutableIpv4Packet::new(&mut buffer[..20])
        .ok_or_else(|| SlapperError::Runtime("Failed to create IPv4 packet".to_string()))?;

    ipv4_packet.set_version(4);
    ipv4_packet.set_header_length(5);
    ipv4_packet.set_total_length(40);
    ipv4_packet.set_ttl(ttl.unwrap_or(64));
    ipv4_packet.set_identification(rand::thread_rng().r#gen());
    ipv4_packet.set_flags(pnet::packet::ipv4::Ipv4Flags::DontFragment);
    ipv4_packet.set_next_level_protocol(IpNextHeaderProtocols::Tcp);
    ipv4_packet.set_source(src_ip);
    ipv4_packet.set_destination(dst_ip);
    // Compute IPv4 header checksum
    let ipv4_checksum = ipv4_checksum(&ipv4_packet.to_immutable());
    ipv4_packet.set_checksum(ipv4_checksum);

    let mut tcp_packet = MutableTcpPacket::new(&mut buffer[20..])
        .ok_or_else(|| SlapperError::Runtime("Failed to create TCP packet".to_string()))?;

    tcp_packet.set_source(src_port);
    tcp_packet.set_destination(dst_port);
    tcp_packet.set_sequence(seq);
    tcp_packet.set_acknowledgement(0);
    tcp_packet.set_data_offset(5);

    let flags = match scan_type {
        ScanType::Syn => 0x02,  // SYN
        ScanType::Null => 0x00, // No flags
        ScanType::Fin => 0x01,  // FIN
        ScanType::Xmas => 0x29, // FIN + PSH + URG
    };
    tcp_packet.set_flags(flags);
    tcp_packet.set_window(65535);
    // Compute TCP checksum using IPv4 pseudo-header
    let tcp_checksum = tcp_ipv4_checksum(&tcp_packet.to_immutable(), &src_ip, &dst_ip);
    tcp_packet.set_checksum(tcp_checksum);

    Ok(buffer)
}

#[cfg(all(feature = "stress-testing", unix))]
pub fn build_syn_packet(
    src_ip: Ipv4Addr,
    src_port: u16,
    dst_ip: Ipv4Addr,
    dst_port: u16,
    seq: u32,
    ttl: Option<u8>,
) -> Result<Vec<u8>> {
    build_tcp_packet(src_ip, src_port, dst_ip, dst_port, seq, ScanType::Syn, ttl)
}

#[cfg(all(feature = "stress-testing", unix))]
pub fn build_fragmented_packets(
    src_ip: Ipv4Addr,
    src_port: u16,
    dst_ip: Ipv4Addr,
    dst_port: u16,
    seq: u32,
    scan_type: ScanType,
    ttl: Option<u8>,
) -> Result<Vec<Vec<u8>>> {
    use pnet_packet::ipv4::{checksum as ipv4_checksum, MutableIpv4Packet};
    use pnet_packet::tcp::{ipv4_checksum as tcp_ipv4_checksum, MutableTcpPacket};

    // Build TCP header with checksum zero, then compute checksum over whole header
    let tcp_data = {
        let mut buffer = vec![0u8; 20];
        let mut tcp_packet = MutableTcpPacket::new(&mut buffer[..])
            .ok_or_else(|| SlapperError::Runtime("Failed to create TCP packet".to_string()))?;

        tcp_packet.set_source(src_port);
        tcp_packet.set_destination(dst_port);
        tcp_packet.set_sequence(seq);
        tcp_packet.set_acknowledgement(0);
        tcp_packet.set_data_offset(5);

        let flags = match scan_type {
            ScanType::Syn => 0x02,  // SYN
            ScanType::Null => 0x00, // No flags
            ScanType::Fin => 0x01,  // FIN
            ScanType::Xmas => 0x29, // FIN + PSH + URG
        };
        tcp_packet.set_flags(flags);
        tcp_packet.set_window(65535);
        // Compute TCP checksum using IPv4 pseudo-header
        let tcp_checksum = tcp_ipv4_checksum(&tcp_packet.to_immutable(), &src_ip, &dst_ip);
        tcp_packet.set_checksum(tcp_checksum);
        // Now buffer contains the TCP header with correct checksum
        buffer
    };

    let mut packets = Vec::new();
    let fragment_size = 8;
    let ttl_val = ttl.unwrap_or(64);
    // Calculate total number of fragments
    let total_chunks = tcp_data.chunks(fragment_size).count();

    for (i, chunk) in tcp_data.chunks(fragment_size).enumerate() {
        let mut buffer = vec![0u8; 20 + fragment_size];

        let mut ipv4_packet = MutableIpv4Packet::new(&mut buffer[..20])
            .ok_or_else(|| SlapperError::Runtime("Failed to create IPv4 packet".to_string()))?;

        ipv4_packet.set_version(4);
        ipv4_packet.set_header_length(5);
        ipv4_packet.set_total_length((20 + chunk.len()) as u16);
        ipv4_packet.set_ttl(ttl_val);
        ipv4_packet.set_identification(rand::thread_rng().r#gen());

        // Set MoreFragments flag for all but the last fragment
        if i < total_chunks - 1 {
            ipv4_packet.set_flags(pnet::packet::ipv4::Ipv4Flags::MoreFragments);
        } else {
            ipv4_packet.set_flags(0);
        }

        ipv4_packet.set_fragment_offset(i as u16);
        ipv4_packet.set_next_level_protocol(IpNextHeaderProtocols::Tcp);
        ipv4_packet.set_source(src_ip);
        ipv4_packet.set_destination(dst_ip);
        // Compute IPv4 header checksum
        let ipv4_checksum = ipv4_checksum(&ipv4_packet.to_immutable());
        ipv4_packet.set_checksum(ipv4_checksum);

        buffer[20..20 + chunk.len()].copy_from_slice(chunk);
        packets.push(buffer);
    }

    Ok(packets)
}

#[cfg(all(feature = "stress-testing", unix))]
pub fn get_network_interface() -> Result<NetworkInterface> {
    let interfaces = datalink::interfaces();

    interfaces
        .into_iter()
        .find(|iface| {
            iface.is_up()
                && !iface.is_loopback()
                && !iface.ips.is_empty()
                && iface.ips.iter().any(|ip| ip.is_ipv4())
        })
        .ok_or_else(|| {
            SlapperError::Network(
                "No suitable network interface found for raw packet sending".to_string(),
            )
        })
}

#[cfg(all(feature = "stress-testing", unix))]
pub fn get_local_ip(interface: &NetworkInterface) -> Result<Ipv4Addr> {
    interface
        .ips
        .iter()
        .find_map(|ip| match ip.ip() {
            IpAddr::V4(ip) => Some(ip),
            _ => None,
        })
        .ok_or_else(|| SlapperError::Network("No IPv4 address found on interface".to_string()))
}

pub fn format_spoof_warning(config: &SpoofConfig) -> String {
    let mut warning = String::new();

    warning.push_str("⚠️  IP SPOOFING ENABLED  ⚠️\n");
    warning.push_str("Source IP spoofing is active for this scan.\n");

    if let Some(ip) = &config.source_ip {
        warning.push_str(&format!("Spoofed Source IP: {}\n", ip));
    }

    if let Some(range) = &config.ip_range {
        warning.push_str(&format!("Spoof IP Range: {}\n", range));
    }

    if let Some(port) = config.source_port {
        warning.push_str(&format!("Source Port: {}\n", port));
    }

    if config.fragment {
        warning.push_str("Fragmentation: YES (8-byte fragments)\n");
    }

    warning.push_str(&format!("Scan Type: {:?}\n", config.scan_type));

    if let Some(rate) = config.max_rate {
        warning.push_str(&format!("Max Rate: {} pps\n", rate));
    }

    if let Some(ttl) = config.ttl {
        warning.push_str(&format!("TTL: {}\n", ttl));
    }

    if !config.decoy_ips.is_empty() {
        warning.push_str(&format!("Decoy IPs: {}\n", config.decoy_ips.len()));
        warning.push_str(&format!("Decoy Mode: {:?}\n", config.decoy_mode));
        if config.include_real_ip {
            warning.push_str("Include Real IP: YES\n");
        }
    }

    warning.push_str("\nRequirements:\n");
    warning.push_str("  • Root/sudo privileges required for raw socket spoofing\n");
    warning.push_str("  • May not work through NAT without proper configuration\n");
    warning.push_str("  • Some networks/firewalls may drop spoofed packets\n");
    warning.push_str("\nThis feature is intended for authorized security testing only.\n");

    warning
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_random_ip_from_cidr_24() {
        let ip = random_ip_from_cidr("10.0.0.0/24").unwrap();
        let ip_u32 = u32::from(ip);
        assert!(ip_u32 >= u32::from(Ipv4Addr::new(10, 0, 0, 1)));
        assert!(ip_u32 <= u32::from(Ipv4Addr::new(10, 0, 0, 254)));
    }

    #[test]
    fn test_random_ip_from_cidr_32() {
        let ip = random_ip_from_cidr("192.168.1.1/32").unwrap();
        assert_eq!(ip, Ipv4Addr::new(192, 168, 1, 1));
    }

    #[test]
    fn test_random_ip_from_cidr_invalid() {
        assert!(random_ip_from_cidr("invalid").is_err());
        assert!(random_ip_from_cidr("10.0.0.0/33").is_err());
        assert!(random_ip_from_cidr("10.0.0.0").is_err());
    }

    #[test]
    fn test_spoof_config_default() {
        let config = SpoofConfig::default();
        assert!(!config.enabled);
        assert!(!config.use_raw_sockets);
    }

    proptest! {
        #[test]
        fn test_random_ip_in_cidr_range(prefix in 24u8..=30) {
            let cidr = format!("10.0.0.0/{}", prefix);
            let ip = random_ip_from_cidr(&cidr).unwrap();
            let ip_u32 = u32::from(ip);
            let base_u32 = u32::from(Ipv4Addr::new(10, 0, 0, 0));
            let host_bits = 32 - prefix;
            let max_network = base_u32 | ((1u32 << host_bits) - 1);
            prop_assert!(ip_u32 >= base_u32);
            prop_assert!(ip_u32 <= max_network);
        }
    }
}
