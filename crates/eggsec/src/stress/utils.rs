#[cfg(feature = "stress-testing")]
use crate::error::{Result, EggsecError};
#[cfg(feature = "stress-testing")]
use std::net::IpAddr;
#[cfg(all(feature = "stress-testing", unix))]
use std::net::{Ipv4Addr, Ipv6Addr};

#[cfg(all(feature = "stress-testing", unix))]
use pnet::datalink::{self, Channel::Ethernet, Config, NetworkInterface};
#[cfg(feature = "stress-testing")]
use rand::Rng;

#[cfg(feature = "stress-testing")]
pub async fn resolve_target(target: &str) -> Result<IpAddr> {
    if let Ok(ip) = target.parse::<IpAddr>() {
        return Ok(ip);
    }

    let addrs: Vec<_> = tokio::net::lookup_host((target, 0)).await?.collect();

    addrs
        .first()
        .map(|a| a.ip())
        .ok_or_else(|| EggsecError::Runtime(format!("Failed to resolve target: {}", target)))
}

#[cfg(all(feature = "stress-testing", unix))]
pub fn get_network_interface() -> Result<NetworkInterface> {
    let interfaces = datalink::interfaces();

    interfaces
        .into_iter()
        .find(|iface| iface.is_up() && !iface.is_loopback() && !iface.ips.is_empty())
        .ok_or_else(|| EggsecError::Runtime("No suitable network interface found".to_string()))
}

#[cfg(all(feature = "stress-testing", unix))]
pub fn create_channel(
    interface: &NetworkInterface,
    label: &str,
) -> Result<(
    Box<dyn datalink::DataLinkSender>,
    Box<dyn datalink::DataLinkReceiver>,
)> {
    crate::utils::privilege::check_privileged(label)?;
    let config = Config::default();

    match datalink::channel(interface, config) {
        Ok(Ethernet(tx, rx)) => Ok((tx, rx)),
        Ok(_) => Err(EggsecError::Runtime(
            "Unsupported channel type".to_string(),
        )),
        Err(e) => Err(EggsecError::Runtime(format!(
            "Failed to create channel: {}",
            e
        ))),
    }
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
        .ok_or_else(|| EggsecError::Runtime("No IPv4 address found on interface".to_string()))
}

#[cfg(all(feature = "stress-testing", unix))]
pub fn get_local_ip_v6(interface: &NetworkInterface) -> Result<Ipv6Addr> {
    interface
        .ips
        .iter()
        .find_map(|ip| match ip.ip() {
            IpAddr::V6(ip) => Some(ip),
            _ => None,
        })
        .ok_or_else(|| EggsecError::Runtime("No IPv6 address found on interface".to_string()))
}

#[cfg(all(feature = "stress-testing", unix))]
pub fn get_spoofed_source(range: &Option<String>) -> Result<Ipv4Addr> {
    let mut rng = rand::thread_rng();

    if let Some(range_str) = range {
        let parts: Vec<&str> = range_str.split('/').collect();
        if parts.len() == 2 {
            let base: Ipv4Addr = parts[0].parse()?;
            let prefix: u8 = parts[1].parse()?;

            let base_u32 = u32::from(base);
            if prefix == 0 {
                return Ok(Ipv4Addr::from(base_u32 | rng.gen_range(1..=u32::MAX)));
            }
            if prefix >= 32 {
                return Ok(base);
            }
            let host_bits = 32 - prefix;
            let max_offset = (1u32 << host_bits) - 1;
            if max_offset <= 1 {
                return Ok(base);
            }
            let offset = rng.gen_range(1..max_offset);
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
        rng.gen_range(0..255),
        rng.gen_range(0..255),
        rng.gen_range(1..254),
    ))
}

#[cfg(all(feature = "stress-testing", unix))]
pub fn get_spoofed_source_v6(range: &Option<String>) -> Result<Ipv6Addr> {
    let mut rng = rand::thread_rng();

    if let Some(range_str) = range {
        let parts: Vec<&str> = range_str.split('/').collect();
        if parts.len() == 2 {
            let base: Ipv6Addr = parts[0].parse()?;
            let prefix: u8 = parts[1].parse()?;

            let base_segments = base.segments();
            let mut new_segments = [0u16; 8];
            let mut done = false;

            for i in 0..8 {
                let seg_start = i * 16;
                let seg_end = seg_start + 16;

                if seg_end <= prefix as usize {
                    new_segments[i] = base_segments[i];
                } else if seg_start >= prefix as usize {
                    new_segments[i] = rng.gen_range(0..=u16::MAX);
                } else {
                    let host_bits_in_seg = seg_end - prefix as usize;
                    let mask = !((1u16 << host_bits_in_seg) - 1);
                    let network_part = base_segments[i] & mask;
                    let random_part = rng.gen_range(0..=(1u16 << host_bits_in_seg) - 1);
                    new_segments[i] = network_part | random_part;
                    done = true;
                }

                if done {
                    for j in (i + 1)..8 {
                        new_segments[j] = rng.gen_range(0..=u16::MAX);
                    }
                    break;
                }
            }

            return Ok(Ipv6Addr::new(
                new_segments[0],
                new_segments[1],
                new_segments[2],
                new_segments[3],
                new_segments[4],
                new_segments[5],
                new_segments[6],
                new_segments[7],
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
        rng.gen_range(0..0xffff),
    ))
}

#[cfg(feature = "stress-testing")]
pub fn generate_payload(size: usize) -> Vec<u8> {
    let mut rng = rand::thread_rng();
    let mut payload = vec![0u8; size];
    rng.fill(&mut payload[..]);
    payload
}
