#[cfg(all(feature = "stress-testing", unix))]
use crate::error::{Result, SlapperError};
#[cfg(all(feature = "stress-testing", unix))]
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
#[cfg(all(feature = "stress-testing", unix))]
use std::time::{Duration, Instant};

#[cfg(all(feature = "stress-testing", unix))]
use pnet::datalink::{self, Channel::Ethernet, Config, NetworkInterface};
#[cfg(all(feature = "stress-testing", unix))]
use pnet::packet::icmp::echo_request::MutableEchoRequestPacket;
#[cfg(all(feature = "stress-testing", unix))]
use pnet::packet::icmp::{IcmpCode, IcmpTypes};
#[cfg(all(feature = "stress-testing", unix))]
use pnet::packet::ip::IpNextHeaderProtocols;

#[cfg(all(feature = "stress-testing", unix))]
use pnet_packet::ipv4::MutableIpv4Packet;

#[cfg(all(feature = "stress-testing", unix))]
use rand::Rng;

#[cfg(all(feature = "stress-testing", unix))]
use super::metrics::StressMetrics;
#[cfg(all(feature = "stress-testing", unix))]
use super::{StressConfig, StressStats};

#[cfg(all(feature = "stress-testing", unix))]
const ICMP_HEADER_LEN: usize = 8;
#[cfg(all(feature = "stress-testing", unix))]
const ICMP_PAYLOAD_SIZE: usize = 56;

#[cfg(all(feature = "stress-testing", unix))]
pub async fn run_icmp_flood(config: &StressConfig, metrics: &StressMetrics) -> Result<StressStats> {
    let target_ip = resolve_target(&config.target).await?;
    let target_addr = SocketAddr::new(target_ip, 0);

    let interface = get_network_interface()?;
    let (mut tx, _rx) = create_channel(&interface)?;

    let src_ip = if config.spoof_source {
        get_spoofed_source(&config.spoof_range)?
    } else {
        get_local_ip(&interface)?
    };

    let payload_size = config.payload_size.max(ICMP_PAYLOAD_SIZE);
    let payload = generate_payload(payload_size);

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

        let packet = build_icmp_packet(
            src_ip,
            match target_addr.ip() {
                IpAddr::V4(ip) => ip,
                IpAddr::V6(_) => {
                    return Err(SlapperError::Runtime(
                        "IPv6 not supported for ICMP flood".to_string(),
                    ))
                }
            },
            identifier,
            &payload,
        )?;

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
fn build_icmp_packet(
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
async fn resolve_target(target: &str) -> Result<IpAddr> {
    if let Ok(ip) = target.parse::<IpAddr>() {
        return Ok(ip);
    }

    let addrs: Vec<_> = tokio::net::lookup_host((target, 0))
        .await?
        .collect();

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
    let config = Config::default();

    match datalink::channel(interface, config) {
        Ok(Ethernet(tx, rx)) => Ok((tx, rx)),
        Ok(_) => Err(SlapperError::Runtime("Unsupported channel type".to_string())),
        Err(e) => Err(SlapperError::Runtime(format!("Failed to create channel: {}", e))),
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
    }

    Ok(Ipv4Addr::new(
        rng.gen_range(1..254),
        rng.gen_range(0..254),
        rng.gen_range(0..254),
        rng.gen_range(1..254),
    ))
}

#[cfg(all(feature = "stress-testing", unix))]
fn generate_payload(size: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let mut payload = vec![0u8; size];
    rng.fill(&mut payload[..]);
    payload
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
