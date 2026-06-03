#[cfg(all(feature = "stress-testing", unix))]
use crate::error::{Result, SlapperError};
#[cfg(all(feature = "stress-testing", unix))]
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
#[cfg(all(feature = "stress-testing", unix))]
use pnet_packet::ipv4::MutableIpv4Packet;
#[cfg(all(feature = "stress-testing", unix))]
use pnet_packet::ipv6::MutableIpv6Packet;

#[cfg(all(feature = "stress-testing", unix))]
use super::metrics::StressMetrics;
#[cfg(all(feature = "stress-testing", unix))]
use super::utils;
#[cfg(all(feature = "stress-testing", unix))]
use super::{StressConfig, StressStats};

#[cfg(all(feature = "stress-testing", unix))]
const ICMP_HEADER_LEN: usize = 8;
#[cfg(all(feature = "stress-testing", unix))]
const ICMP_PAYLOAD_SIZE: usize = 56;

#[cfg(all(feature = "stress-testing", unix))]
pub async fn run_icmp_flood(config: &StressConfig, metrics: &StressMetrics) -> Result<StressStats> {
    let target_ip = utils::resolve_target(&config.target).await?;
    let target_addr = SocketAddr::new(target_ip, 0);

    let interface = utils::get_network_interface()?;
    let (mut tx, _rx) = utils::create_channel(&interface, "ICMP probe")?;

    let payload_size = config.payload_size.max(ICMP_PAYLOAD_SIZE);
    let payload = utils::generate_payload(payload_size);

    metrics.start();

    let start_time = Instant::now();
    let duration = Duration::from_secs(config.duration_secs);
    let interval = Duration::from_micros(1_000_000 / config.rate_pps.max(1));

    let mut identifier: u16 = 1000;

    while start_time.elapsed() < duration {
        if config.random_source_port {
            identifier = rand::random::<u16>();
        } else {
            identifier = identifier.wrapping_add(1);
        }

        let packet = match target_addr.ip() {
            IpAddr::V4(dst_ip) => {
                let src_ip = if config.spoof_source {
                    utils::get_spoofed_source(&config.spoof_range)?
                } else {
                    utils::get_local_ip(&interface)?
                };
                build_icmp_packet_v4(src_ip, dst_ip, identifier, &payload)?
            }
            IpAddr::V6(dst_ip) => {
                let src_ip = if config.spoof_source {
                    utils::get_spoofed_source_v6(&config.spoof_range)?
                } else {
                    utils::get_local_ip_v6(&interface)?
                };
                build_icmp_packet_v6(src_ip, dst_ip, identifier, &payload)?
            }
        };

        match tx.send_to(&packet, Some(interface.clone())) {
            Some(Ok(_)) => {
                metrics.record_packet((ICMP_HEADER_LEN + payload.len()) as u64);
            }
            Some(Err(e)) => {
                tracing::debug!("Send error: {}", e);
                metrics.record_error();
            }
            None => {
                metrics.record_error();
            }
        }

        if interval > Duration::ZERO {
            tokio::time::sleep(interval).await;
        }
    }

    Ok(metrics.to_stats())
}

#[cfg(all(feature = "stress-testing", unix))]
fn build_icmp_packet_v4(
    src_ip: Ipv4Addr,
    dst_ip: Ipv4Addr,
    identifier: u16,
    payload: &[u8],
) -> Result<Vec<u8>> {
    let icmp_len = ICMP_HEADER_LEN + payload.len();
    let total_len = 20 + icmp_len;

    let mut buffer = vec![0u8; total_len];

    let mut ipv4_packet = MutableIpv4Packet::new(&mut buffer[..20])
        .ok_or_else(|| SlapperError::Runtime("Failed to create IPv4 packet".to_string()))?;

    ipv4_packet.set_version(4);
    ipv4_packet.set_header_length(5);
    ipv4_packet.set_total_length(total_len as u16);
    ipv4_packet.set_ttl(64);
    ipv4_packet.set_flags(0x40);
    ipv4_packet.set_next_level_protocol(IpNextHeaderProtocols::Icmp);
    ipv4_packet.set_source(src_ip);
    ipv4_packet.set_destination(dst_ip);

    let mut icmp_packet = MutableEchoRequestPacket::new(&mut buffer[20..])
        .ok_or_else(|| SlapperError::Runtime("Failed to create ICMP packet".to_string()))?;

    icmp_packet.set_icmp_type(IcmpTypes::EchoRequest);
    icmp_packet.set_icmp_code(IcmpCode(0));
    icmp_packet.set_identifier(identifier);
    icmp_packet.set_sequence_number(1);
    icmp_packet.set_payload(payload.into());
    icmp_packet.set_checksum(0);

    Ok(buffer)
}

#[cfg(all(feature = "stress-testing", unix))]
fn build_icmp_packet_v6(
    src_ip: Ipv6Addr,
    dst_ip: Ipv6Addr,
    identifier: u16,
    payload: &[u8],
) -> Result<Vec<u8>> {
    let icmp_len = ICMP_HEADER_LEN + payload.len();
    let total_len = 40 + icmp_len;

    let mut buffer = vec![0u8; total_len];

    let mut ipv6_packet = MutableIpv6Packet::new(&mut buffer[..40])
        .ok_or_else(|| SlapperError::Runtime("Failed to create IPv6 packet".to_string()))?;

    ipv6_packet.set_version(6);
    ipv6_packet.set_payload_length(icmp_len as u16);
    ipv6_packet.set_hop_limit(64);
    ipv6_packet.set_next_header(IpNextHeaderProtocols::Icmp);
    ipv6_packet.set_source(src_ip);
    ipv6_packet.set_destination(dst_ip);

    let mut icmp_packet = MutableEchoRequestPacket::new(&mut buffer[40..])
        .ok_or_else(|| SlapperError::Runtime("Failed to create ICMPv6 packet".to_string()))?;

    icmp_packet.set_icmp_type(IcmpTypes::EchoRequest);
    icmp_packet.set_icmp_code(IcmpCode(0));
    icmp_packet.set_identifier(identifier);
    icmp_packet.set_sequence_number(1);
    icmp_packet.set_payload(payload.into());
    icmp_packet.set_checksum(0);

    Ok(buffer)
}

#[cfg(not(all(feature = "stress-testing", unix)))]
pub async fn run_icmp_flood(
    _config: &super::StressConfig,
    _metrics: &super::metrics::StressMetrics,
) -> crate::error::Result<super::StressStats> {
    Err(SlapperError::Runtime(
        "ICMP flood requires Unix and 'stress-testing' feature enabled".to_string(),
    ))
}
