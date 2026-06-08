//! Spoofed port scanning implementation.
//!
//! Provides raw socket-based port scanning with IP spoofing, decoy support,
//! and packet fragmentation capabilities.

#[cfg(all(feature = "stress-testing", unix))]
use super::get_service_name;
use super::PortScanResults;
use crate::error::{Result, SlapperError};
use crate::scanner::spoof::SpoofConfig;
#[cfg(all(feature = "stress-testing", unix))]
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

#[cfg(all(feature = "stress-testing", unix))]
fn parse_tcp_response(packet: &[u8]) -> Option<(u32, u16, String)> {
    if packet.len() < 20 {
        return None;
    }

    let ip_header_len = ((packet[0] & 0x0F) as usize) * 4;
    if packet.len() < ip_header_len + 20 {
        return None;
    }

    let src_ip_bytes = &packet[12..16];
    let _dst_ip_bytes = &packet[16..20];
    let src_ip = u32::from_be_bytes([
        src_ip_bytes[0],
        src_ip_bytes[1],
        src_ip_bytes[2],
        src_ip_bytes[3],
    ]);

    let tcp_data = &packet[ip_header_len..];
    if tcp_data.len() < 20 {
        return None;
    }

    let _src_port = u16::from_be_bytes([tcp_data[0], tcp_data[1]]);
    let dst_port = u16::from_be_bytes([tcp_data[2], tcp_data[3]]);
    let flags = u16::from_be_bytes([tcp_data[12], tcp_data[13]]);

    let syn_ack = (flags & 0x12) == 0x12;
    let rst = (flags & 0x04) == 0x04;

    if syn_ack {
        Some((src_ip, dst_port, "open".to_string()))
    } else if rst {
        Some((src_ip, dst_port, "closed".to_string()))
    } else {
        None
    }
}

static PACKET_TRACE_FILE: std::sync::OnceLock<parking_lot::Mutex<std::fs::File>> =
    std::sync::OnceLock::new();

#[cfg(all(feature = "stress-testing", unix))]
fn log_packet_trace(src_ip: &str, src_port: u16, dst_ip: &str, dst_port: u16, scan_type: &str) {
    if let Some(file) = PACKET_TRACE_FILE.get() {
        let mut guard = file.lock();
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

pub fn init_packet_trace(path: &str, include_header: bool) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;

    if include_header {
        use std::io::Write;
        writeln!(file, "timestamp,src_ip,src_port,dst_ip,dst_port,scan_type")?;
    }

    PACKET_TRACE_FILE
        .set(parking_lot::Mutex::new(file))
        .map_err(|_| SlapperError::Runtime("Packet trace file already initialized".to_string()))?;
    Ok(())
}

pub fn shutdown_packet_trace() {
    if let Some(file) = PACKET_TRACE_FILE.get() {
        let mut guard = file.lock();
        use std::io::Write;
        let _ = guard.flush();
    }
}

#[cfg(all(feature = "stress-testing", unix))]
pub(crate) async fn scan_ports_spoofed(
    host: &str,
    ports: Vec<u16>,
    concurrency: usize,
    timeout_duration: Duration,
    tui_mode: bool,
    spoof_config: SpoofConfig,
    progress_tx: Option<tokio::sync::mpsc::Sender<(u64, u64)>>,
) -> Result<PortScanResults> {
    use crate::scanner::spoof::{
        build_fragmented_packets, build_tcp_packet, get_local_ip, get_network_interface,
        random_ip_from_cidr,
    };
    use crate::utils::parsing::resolve_host;
    use dashmap::DashMap;
    use futures::future::join_all;
    use indicatif::{ProgressBar, ProgressStyle};
    use pnet::datalink::Config;
    use std::net::Ipv4Addr;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use super::PortResult;

    let target_ip = resolve_host(host)?;
    let target_ipv4 = match target_ip {
        std::net::IpAddr::V4(ip) => ip,
        std::net::IpAddr::V6(_) => {
            return Err(SlapperError::Runtime(
                "IPv6 not supported for spoofed scanning".to_string(),
            ))
        }
    };

    let interface = get_network_interface()?;
    let local_ip = get_local_ip(&interface)?;

    crate::utils::privilege::check_privileged("IP spoof")?;

    let (tx, rx) = match pnet::datalink::channel(&interface, Config::default()) {
        Ok(pnet::datalink::Channel::Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => {
            return Err(SlapperError::Runtime(
                "Unsupported channel type".to_string(),
            ))
        }
        Err(e) => {
            return Err(SlapperError::Runtime(format!(
                "Failed to create datalink channel: {}",
                e
            )))
        }
    };

    let target_ip_u32: u32 = u32::from(target_ipv4);
    let local_ip_u32: u32 = u32::from(local_ip);

    let sent_packets: Arc<DashMap<u16, u32>> = Arc::new(DashMap::new());
    let responses: Arc<DashMap<u16, String>> = Arc::new(DashMap::new());
    let stop_receiver = Arc::new(AtomicBool::new(false));
    let results: Arc<DashMap<u16, PortResult>> = Arc::new(DashMap::new());
    let scanned_count = Arc::new(AtomicU64::new(0));
    let total_ports = ports.len() as u64;
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

    let tx = Arc::new(parking_lot::Mutex::new(tx));

    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));
    let mut handles = Vec::new();
    let start = std::time::Instant::now();
    let ports_count = ports.len();

    let packets_sent = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));

    let has_decoys = spoof_config.has_decoys();
    let use_staggered = spoof_config.decoy_mode == crate::scanner::spoof::DecoyMode::Staggered;

    std::thread::spawn({
        let rx = Arc::new(parking_lot::Mutex::new(rx));
        let sent_packets = sent_packets.clone();
        let responses = responses.clone();
        let stop_receiver = stop_receiver.clone();
        let target_ip_u32 = target_ip_u32;
        let local_ip_u32 = local_ip_u32;

        move || loop {
            if stop_receiver.load(Ordering::Relaxed) {
                break;
            }

            let packet = {
                let mut rx_guard = rx.lock();
                match rx_guard.next() {
                    Ok(p) => p.to_vec(),
                    Err(_) => {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                        continue;
                    }
                }
            };

            if let Some((src_ip, dst_port, status)) = parse_tcp_response(&packet) {
                if src_ip == target_ip_u32 || src_ip == local_ip_u32 {
                    if sent_packets.contains_key(&dst_port) {
                        if !responses.contains_key(&dst_port) {
                            responses.insert(dst_port, status.clone());
                        }
                    }
                }
            }
        }
    });

    for port in ports {
        let permit = semaphore.clone().acquire_owned().await?;
        let results = results.clone();
        let progress = progress.clone();
        let spoof_config = spoof_config.clone();
        let interface = interface.clone();
        let tx = tx.clone();
        let scanned_count = scanned_count.clone();
        let progress_tx = progress_tx.clone();

        let src_ip: Ipv4Addr = if let Some(ref ip) = spoof_config.source_ip {
            *ip
        } else if let Some(ref range) = spoof_config.ip_range {
            random_ip_from_cidr(range).map_err(|e| {
                SlapperError::Parse(format!("Invalid spoof range '{}': {}", range, e))
            })?
        } else {
            local_ip
        };

        let packets_sent = packets_sent.clone();
        let sent_packets = sent_packets.clone();
        let responses = responses.clone();
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
                            if tx_guard.send_to(pkt, Some(interface.clone())).is_none() {
                                tracing::warn!("Failed to send spoofed packet");
                            } else {
                                packets_sent.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            }
                        }
                        let src_ip_u32: u32 = u32::from(src_ip);
                        sent_packets.insert(port, src_ip_u32);
                    }
                    Err(e) => {
                        tracing::debug!("Failed to build fragmented TCP packets: {}", e);
                        if let Some(ref pb) = progress {
                            pb.inc(1);
                        }
                        if let Some(ref tx) = progress_tx {
                            let count = scanned_count.fetch_add(1, Ordering::Relaxed) + 1;
                            if tx.send((count, total_ports)).await.is_err() {
                                tracing::warn!("Progress receiver dropped");
                            }
                        }
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
                                let src_ip_u32: u32 = u32::from(src_ip);
                                sent_packets.insert(port, src_ip_u32);
                            }
                            _ => {
                                tracing::warn!("Failed to send TCP packet for port {}", port);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::debug!(
                            "Failed to build spoofed TCP packet for port {}: {}",
                            port,
                            e
                        );
                        if let Some(ref pb) = progress {
                            pb.inc(1);
                        }
                        if let Some(ref tx) = progress_tx {
                            let count = scanned_count.fetch_add(1, Ordering::Relaxed) + 1;
                            if tx.send((count, total_ports)).await.is_err() {
                                tracing::warn!("Progress receiver dropped");
                            }
                        }
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
                                let send_result =
                                    tx_guard.send_to(&packet, Some(interface.clone()));
                                if send_result.is_none() {
                                    tracing::warn!("Failed to send simultaneous decoy packet");
                                } else {
                                    packets_sent.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                }
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
                                    let send_result =
                                        tx_guard.send_to(&packet, Some(interface.clone()));
                                    if send_result.is_none() {
                                        tracing::warn!("Failed to send staggered decoy packet");
                                    } else {
                                        packets_sent
                                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                    }
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

            let status = {
                let wait_start = std::time::Instant::now();
                let timeout_ms = timeout_duration.as_millis() as u64;
                let mut status = "filtered".to_string();
                let mut backoff_ms = 1u64;
                let max_backoff_ms = 50u64;

                while (wait_start.elapsed().as_millis() as u64) < timeout_ms {
                    tokio::time::sleep(tokio::time::Duration::from_millis(backoff_ms)).await;

                    if let Some(resp) = responses.get(&port) {
                        status = resp.clone();
                        break;
                    }

                    // Exponential backoff: double the delay each time
                    backoff_ms = std::cmp::min(backoff_ms * 2, max_backoff_ms);
                }

                status
            };

            if packet_trace.is_some() {
                log_packet_trace(
                    &src_ip.to_string(),
                    src_port,
                    &target_ipv4.to_string(),
                    port,
                    &format!("{:?}", scan_type),
                );
            }

            results.insert(
                port,
                PortResult {
                    port,
                    status: status.to_string(),
                    service: get_service_name(port).to_string(),
                },
            );

            if let Some(ref pb) = progress {
                pb.inc(1);
            }
            if let Some(ref tx) = progress_tx {
                let count = scanned_count.fetch_add(1, Ordering::Relaxed) + 1;
                if tx.send((count, total_ports)).await.is_err() {
                    tracing::warn!("Progress receiver dropped");
                }
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

    stop_receiver.store(true, Ordering::Relaxed);

    if let Some(ref pb) = progress {
        pb.finish_and_clear();
    }

    let results_map = Arc::try_unwrap(results).map_err(|_| {
        crate::error::SlapperError::Runtime("Arc ref count non-zero after workers completed".into())
    })?;
    let mut results: Vec<PortResult> = results_map
        .into_iter()
        .map(|(_, v)| v)
        .filter(|p| p.status == "open")
        .collect();
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
        ports_scanned: ports_count as u32,
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
    _progress_tx: Option<tokio::sync::mpsc::Sender<(u64, u64)>>,
) -> Result<PortScanResults> {
    Err(SlapperError::Runtime(
        "IP spoofing requires 'stress-testing' feature and Unix system".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_packet_trace_creates_file() {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join("test_packet_trace.csv");
        let path_str = path.to_str().unwrap();

        let result = init_packet_trace(path_str, true);
        assert!(result.is_ok());

        // Clean up
        let _ = std::fs::remove_file(&path);
    }
}
