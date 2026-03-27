//! Spoofed port scanning implementation.
//!
//! Provides raw socket-based port scanning with IP spoofing, decoy support,
//! and packet fragmentation capabilities.

use anyhow::Result;
use std::time::Duration;

use crate::scanner::spoof::SpoofConfig;
use super::PortScanResults;

static PACKET_TRACE_FILE: std::sync::OnceLock<std::sync::Mutex<std::fs::File>> =
    std::sync::OnceLock::new();

#[cfg(all(feature = "stress-testing", unix))]
fn log_packet_trace(src_ip: &str, src_port: u16, dst_ip: &str, dst_port: u16, scan_type: &str) {
    if let Some(file) = PACKET_TRACE_FILE.get() {
        if let Ok(mut guard) = file.lock() {
            use std::io::Write;
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let _ = writeln!(
                guard,
                "{},{},{},{},{},{}",
                timestamp, src_ip, src_port, dst_ip, dst_port, scan_type
            );
        }
    }
}

pub fn init_packet_trace(path: &str) -> Result<()> {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;

    let mut header = std::fs::OpenOptions::new()
        .create_new(true)
        .append(true)
        .open(path);

    if let Ok(ref mut f) = header {
        use std::io::Write;
        let _ = writeln!(f, "timestamp,src_ip,src_port,dst_ip,dst_port,scan_type");
    }

    PACKET_TRACE_FILE.set(std::sync::Mutex::new(file)).ok();
    Ok(())
}

#[cfg(all(feature = "stress-testing", unix))]
pub(crate) async fn scan_ports_spoofed(
    host: &str,
    ports: Vec<u16>,
    concurrency: usize,
    timeout_duration: Duration,
    tui_mode: bool,
    spoof_config: SpoofConfig,
) -> Result<PortScanResults> {
    use crate::scanner::spoof::{
        build_fragmented_packets, build_tcp_packet, get_local_ip,
        get_network_interface, random_ip_from_cidr,
    };
    use crate::utils::parsing::resolve_host;
    use futures::future::join_all;
    use indicatif::{ProgressBar, ProgressStyle};
    use pnet::datalink::Config;
    use rand::Rng;
    use std::net::Ipv4Addr;
    use std::sync::Arc;
    use std::sync::Arc as StdArc;
    use tokio::sync::Mutex;

    use super::{get_service_name, PortResult};

    let target_ip = resolve_host(host)?;
    let target_ipv4 = match target_ip {
        std::net::IpAddr::V4(ip) => ip,
        std::net::IpAddr::V6(_) => anyhow::bail!("IPv6 not supported for spoofed scanning"),
    };

    let interface = get_network_interface()?;
    let local_ip = get_local_ip(&interface)?;

    let (tx, _rx) = match pnet::datalink::channel(&interface, Config::default()) {
        Ok(pnet::datalink::Channel::Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => anyhow::bail!("Unsupported channel type"),
        Err(e) => anyhow::bail!("Failed to create datalink channel: {}", e),
    };

    let tx = StdArc::new(parking_lot::Mutex::new(tx));
    let results = Arc::new(Mutex::new(Vec::new()));
    let progress = if tui_mode {
        None
    } else {
        let pb = Arc::new(ProgressBar::new(ports.len() as u64));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ports ({eta})")
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars("#>-"),
        );
        Some(pb)
    };

    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));
    let mut handles = Vec::new();
    let start = std::time::Instant::now();
    let ports_count = ports.len();

    let packets_sent = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));

    let has_decoys = spoof_config.has_decoys();
    let use_staggered = spoof_config.decoy_mode == crate::scanner::spoof::DecoyMode::Staggered;

    for port in ports {
        let permit = semaphore.clone().acquire_owned().await?;
        let results = results.clone();
        let progress = progress.clone();
        let spoof_config = spoof_config.clone();
        let interface = interface.clone();
        let tx = tx.clone();

        let src_ip: Ipv4Addr = if let Some(ref ip) = spoof_config.source_ip {
            *ip
        } else if let Some(ref range) = spoof_config.ip_range {
            random_ip_from_cidr(range)
                .map_err(|e| anyhow::anyhow!("Invalid spoof range '{}': {}", range, e))?
        } else {
            local_ip
        };

        let packets_sent = packets_sent.clone();
        let scan_type = spoof_config.scan_type;
        let do_fragment = spoof_config.fragment;
        let packet_trace = spoof_config.packet_trace.clone();
        let max_rate = spoof_config.max_rate;
        let ttl = spoof_config.ttl;
        let random_source_port = spoof_config.random_source_port;

        let handle = tokio::spawn(async move {
            let src_port: u16 = if let Some(port) = spoof_config.source_port {
                port
            } else if random_source_port {
                rand::random::<u16>() % 32768 + 32768
            } else {
                rand::random::<u16>() % 20000 + 40000
            };
            let seq: u32 = rand::random::<u32>();

            let total_packets = if has_decoys {
                1 + decoy_count_for_port(&spoof_config, port)
            } else {
                1
            };
            if let Some(rate) = max_rate {
                let delay_ms = (1000 * total_packets as u64) / rate as u64;
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            }

            if do_fragment {
                match build_fragmented_packets(
                    src_ip,
                    src_port,
                    target_ipv4,
                    port,
                    seq,
                    scan_type,
                    ttl,
                ) {
                    Ok(packets) => {
                        let mut tx_guard = tx.lock();
                        for pkt in &packets {
                            let _ = tx_guard.send_to(pkt, Some(interface.clone()));
                            packets_sent.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                    Err(_) => {
                        drop(permit);
                        return;
                    }
                }
            } else {
                match build_tcp_packet(src_ip, src_port, target_ipv4, port, seq, scan_type, ttl) {
                    Ok(packet) => {
                        let mut tx_guard = tx.lock();
                        match tx_guard.send_to(&packet, Some(interface.clone())) {
                            Some(Ok(_)) => {
                                packets_sent.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            }
                            _ => {}
                        }
                    }
                    Err(_) => {
                        drop(permit);
                        return;
                    }
                }
            }

            if has_decoys {
                let decoy_ips = spoof_config.get_all_source_ips(local_ip);
                let decoy_count = decoy_count_for_port(&spoof_config, port);

                match spoof_config.decoy_mode {
                    crate::scanner::spoof::DecoyMode::Simultaneous => {
                        let mut tx_guard = tx.lock();

                        let mut all_ips: Vec<(Ipv4Addr, u16, u32)> = decoy_ips
                            .iter()
                            .take(decoy_count)
                            .enumerate()
                            .map(|(i, ip)| {
                                (
                                    *ip,
                                    src_port.wrapping_add(i as u16),
                                    seq.wrapping_add(i as u32),
                                )
                            })
                            .collect();

                        for i in 0..all_ips.len() {
                            let j = i + rand::random::<usize>() % (all_ips.len() - i);
                            all_ips.swap(i, j);
                        }

                        for (ip, port_offset, seq_offset) in all_ips {
                            if let Ok(packet) = build_tcp_packet(
                                ip,
                                port_offset,
                                target_ipv4,
                                port,
                                seq_offset,
                                scan_type,
                                ttl,
                            ) {
                                let _ = tx_guard.send_to(&packet, Some(interface.clone()));
                                packets_sent.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            }
                        }
                    }
                    crate::scanner::spoof::DecoyMode::Staggered => {
                        let mut all_ips: Vec<(Ipv4Addr, u16, u32)> = decoy_ips
                            .iter()
                            .take(decoy_count)
                            .enumerate()
                            .map(|(i, ip)| {
                                (
                                    *ip,
                                    src_port.wrapping_add(i as u16),
                                    seq.wrapping_add(i as u32),
                                )
                            })
                            .collect();

                        for i in 0..all_ips.len() {
                            let j = i + rand::random::<usize>() % (all_ips.len() - i);
                            all_ips.swap(i, j);
                        }

                        for (ip, port_offset, seq_offset) in all_ips {
                            if let Ok(packet) = build_tcp_packet(
                                ip,
                                port_offset,
                                target_ipv4,
                                port,
                                seq_offset,
                                scan_type,
                                ttl,
                            ) {
                                {
                                    let mut tx_guard = tx.lock();
                                    let _ = tx_guard.send_to(&packet, Some(interface.clone()));
                                    packets_sent.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                }
                                if use_staggered {
                                    let stagger_delay = 10 + rand::random::<u64>() % 100;
                                    tokio::time::sleep(tokio::time::Duration::from_millis(
                                        stagger_delay,
                                    ))
                                    .await;
                                }
                            }
                        }
                    }
                }
            }

            let status = if has_decoys { "decoy" } else { "spoofed" };

            if packet_trace.is_some() {
                log_packet_trace(
                    &src_ip.to_string(),
                    src_port,
                    &target_ipv4.to_string(),
                    port,
                    &format!("{:?}", scan_type),
                );
            }

            let mut results = results.lock().await;
            results.push(PortResult {
                port,
                status: status.to_string(),
                service: get_service_name(port),
            });

            if let Some(ref pb) = progress {
                pb.inc(1);
            }
            drop(permit);
        });

        handles.push(handle);
    }

    fn decoy_count_for_port(config: &crate::scanner::spoof::SpoofConfig, port: u16) -> usize {
        let base_count = config.decoy_count.max(1);
        let variation = (port as usize) % 3;
        base_count + variation
    }

    join_all(handles).await;
    if let Some(ref pb) = progress {
        pb.finish_and_clear();
    }

    let mut results = results.lock().await.clone();
    results.sort_by_key(|p| p.port);

    let spoof_stats = Some(crate::scanner::spoof::SpoofStats {
        packets_sent: packets_sent.load(std::sync::atomic::Ordering::Relaxed),
        packets_dropped: 0,
        spoofed_ips_used: if spoof_config.has_decoys() {
            spoof_config.decoy_ips.len() + 1
        } else {
            1
        },
        decoys_used: if spoof_config.has_decoys() {
            spoof_config.decoy_count
        } else {
            0
        },
        unique_decoy_ips: spoof_config.decoy_ips.len(),
        decoy_mode: format!("{:?}", spoof_config.decoy_mode).to_lowercase(),
    });

    Ok(PortScanResults {
        host: host.to_string(),
        ports_scanned: ports_count as u16,
        open_ports: results,
        duration_ms: start.elapsed().as_millis() as u64,
        spoof_stats,
    })
}

#[cfg(not(all(feature = "stress-testing", unix)))]
pub(crate) async fn scan_ports_spoofed(
    _host: &str,
    _ports: Vec<u16>,
    _concurrency: usize,
    _timeout_duration: Duration,
    _tui_mode: bool,
    _spoof_config: SpoofConfig,
) -> Result<PortScanResults> {
    anyhow::bail!("IP spoofing requires 'stress-testing' feature and Unix system");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_packet_trace_creates_file() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_packet_trace.csv");
        let path_str = path.to_str().unwrap();

        let result = init_packet_trace(path_str);
        assert!(result.is_ok());

        // Clean up
        let _ = std::fs::remove_file(&path);
    }
}
