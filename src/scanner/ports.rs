#![allow(dead_code)]

use crate::utils::parsing::{parse_ports, resolve_host};
use crate::utils::truncate;
use crate::scanner::spoof::{SpoofConfig, SpoofStats, format_spoof_warning};
use anyhow::Result;
use futures::future::join_all;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::timeout;

use crate::cli::PortScanArgs;
use crate::config::SlapperConfig;

static PACKET_TRACE_FILE: std::sync::OnceLock<std::sync::Mutex<std::fs::File>> = std::sync::OnceLock::new();

fn log_packet_trace(src_ip: &str, src_port: u16, dst_ip: &str, dst_port: u16, scan_type: &str) {
    if let Some(file) = PACKET_TRACE_FILE.get() {
        if let Ok(mut guard) = file.lock() {
            use std::io::Write;
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let _ = writeln!(guard, "{},{},{},{},{},{}", timestamp, src_ip, src_port, dst_ip, dst_port, scan_type);
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

static COMMON_PORTS: &[(u16, &str)] = &[
    (21, "FTP"),
    (22, "SSH"),
    (23, "Telnet"),
    (25, "SMTP"),
    (53, "DNS"),
    (80, "HTTP"),
    (110, "POP3"),
    (143, "IMAP"),
    (443, "HTTPS"),
    (445, "SMB"),
    (993, "IMAPS"),
    (995, "POP3S"),
    (1433, "MSSQL"),
    (1521, "Oracle"),
    (3306, "MySQL"),
    (3389, "RDP"),
    (5432, "PostgreSQL"),
    (5900, "VNC"),
    (6379, "Redis"),
    (8080, "HTTP-Alt"),
    (8443, "HTTPS-Alt"),
    (27017, "MongoDB"),
];

fn get_service_name(port: u16) -> String {
    COMMON_PORTS
        .iter()
        .find(|(p, _)| *p == port)
        .map(|(_, name)| name.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortResult {
    pub port: u16,
    pub status: String,
    pub service: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PortScanResults {
    pub host: String,
    pub ports_scanned: u16,
    pub open_ports: Vec<PortResult>,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spoof_stats: Option<SpoofStats>,
}

impl std::fmt::Display for PortScanResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Port Scan Results")?;
        writeln!(f, "host: {}", truncate(&self.host, 60))?;
        writeln!(f, "scanned: {} ports", self.ports_scanned)?;
        writeln!(f, "open: {} ports", self.open_ports.len())?;
        
        if self.open_ports.is_empty() {
            writeln!(f, "no open ports")?;
        } else {
            writeln!(f, "open ports");
            for port in &self.open_ports {
                writeln!(f, "\t{}/tcp\t{}\t{}", port.port, port.status, port.service)?;
            }
        }
        
        Ok(())
    }
}



pub async fn run_cli(args: PortScanArgs, config: &SlapperConfig) -> Result<()> {
    if args.verbose {
        eprintln!("Starting port scan on {} ports {}", args.host, args.ports);
    }
    
    let ports = parse_ports(&args.ports)?;
    let timeout_secs = if args.timeout == 2 {
        config.scan.port_timeout_secs
    } else {
        args.timeout
    };
    
    let spoof_config = SpoofConfig::from_args(
        args.source_ip.clone(),
        args.spoof_range.clone(),
        false,
        args.decoy.clone(),
        args.decoy_range.clone(),
        args.decoy_count,
        args.decoy_mode.clone(),
        args.include_me,
        args.source_port,
        args.random_source_port,
        args.fragment,
        args.scan_type.clone(),
        args.packet_trace.clone(),
        args.max_rate,
        args.ttl,
    )?;
    
    if let Some(ref trace_path) = spoof_config.packet_trace {
        if let Err(e) = init_packet_trace(trace_path) {
            eprintln!("Warning: Failed to initialize packet trace: {}", e);
        }
    }
    
    if spoof_config.enabled {
        eprintln!("{}", format_spoof_warning(&spoof_config));
    }
    
    if args.dry_run {
        eprintln!("\n=== DRY RUN MODE ===");
        eprintln!("Target: {}", args.host);
        eprintln!("Ports: {}", args.ports);
        eprintln!("Concurrency: {}", args.concurrency);
        eprintln!("Timeout: {}s", timeout_secs);
        if spoof_config.enabled {
            if let Some(ref ip) = spoof_config.source_ip {
                eprintln!("Spoof Source IP: {}", ip);
            }
            if let Some(ref range) = spoof_config.ip_range {
                eprintln!("Spoof IP Range: {}", range);
            }
            if let Some(port) = spoof_config.source_port {
                eprintln!("Source Port: {}", port);
            }
            if spoof_config.random_source_port {
                eprintln!("Source Port: RANDOM");
            }
            if spoof_config.fragment {
                eprintln!("Fragmentation: YES (8-byte fragments)");
            }
            eprintln!("Scan Type: {:?}", spoof_config.scan_type);
            if let Some(ref trace) = spoof_config.packet_trace {
                eprintln!("Packet Trace: {}", trace);
            }
            if let Some(rate) = spoof_config.max_rate {
                eprintln!("Max Rate: {} pps", rate);
            }
            if let Some(ttl) = spoof_config.ttl {
                eprintln!("TTL: {}", ttl);
            }
            if !spoof_config.decoy_ips.is_empty() {
                eprintln!("Decoy IPs: {} total", spoof_config.decoy_ips.len());
                for (_i, ip) in spoof_config.decoy_ips.iter().take(5).enumerate() {
                    eprintln!("  - {}", ip);
                }
                if spoof_config.decoy_ips.len() > 5 {
                    eprintln!("  ... and {} more", spoof_config.decoy_ips.len() - 5);
                }
                eprintln!("Decoy Mode: {:?}", spoof_config.decoy_mode);
                if spoof_config.include_real_ip {
                    eprintln!("Include Real IP: YES");
                }
            }
        }
        eprintln!("===================\n");
        return Ok(());
    }
    
    let ports_count = ports.len();
    
    let results = scan_ports(
        &args.host, 
        ports, 
        args.concurrency, 
        Duration::from_secs(timeout_secs), 
        false,
        spoof_config
    ).await?;

    if args.verbose {
        eprintln!("Scan complete: {} open ports found out of {} scanned", 
            results.open_ports.len(), ports_count);
    }

    let output = if args.json {
        serde_json::to_string_pretty(&results)?
    } else if args.grepable {
        let mut s = String::new();
        s.push_str("# Nmap grepable output\n");
        s.push_str(&format!("Host: {}\n", results.host));
        s.push_str("Status: up\n");
        s.push_str("Ports: ");
        for (i, port) in results.open_ports.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            s.push_str(&format!("{}/open/{}", port.port, port.service));
        }
        s.push_str("\n");
        s
    } else if args.xml {
        let mut s = String::new();
        s.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        s.push_str("<nmaprun>\n");
        s.push_str(&format!("  <host>{}</host>\n", results.host));
        s.push_str("  <ports>\n");
        for port in &results.open_ports {
            s.push_str(&format!("    <port>{}</port>\n", port.port));
            s.push_str("      <state>open</state>\n");
            s.push_str(&format!("      <service>{}</service>\n", port.service));
        }
        s.push_str("  </ports>\n");
        s.push_str("</nmaprun>\n");
        s
    } else {
        format!("{}", results)
    };
    
    if let Some(ref output_file) = args.output {
        std::fs::write(output_file, &output)?;
        if args.verbose {
            eprintln!("Results written to {}", output_file);
        }
    } else {
        println!("{}", output);
    }

    Ok(())
}

pub async fn scan_ports(
    host: &str,
    ports: Vec<u16>,
    concurrency: usize,
    timeout_duration: Duration,
    tui_mode: bool,
    spoof_config: SpoofConfig,
) -> Result<PortScanResults> {
    if spoof_config.enabled && spoof_config.use_raw_sockets {
        return scan_ports_spoofed(host, ports, concurrency, timeout_duration, tui_mode, spoof_config).await;
    }
    
    let addr = resolve_host(host)?;
    let results = Arc::new(Mutex::new(Vec::new()));
    
    let progress = if tui_mode {
        None
    } else {
        let pb = Arc::new(ProgressBar::new(ports.len() as u64));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ports ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );
        Some(pb)
    };

    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));
    let mut handles = Vec::new();
    let start = std::time::Instant::now();
    let ports_count = ports.len();

    for port in ports {
        let permit = semaphore.clone().acquire_owned().await?;
        let addr = addr.clone();
        let results = results.clone();
        let progress = progress.clone();
        let timeout_dur = timeout_duration;

        let handle = tokio::spawn(async move {
            let socket_addr = std::net::SocketAddr::new(addr, port);
            let result = timeout(timeout_dur, TcpStream::connect(&socket_addr)).await;
            
            let mut results = results.lock().await;
            match result {
                Ok(Ok(_)) => {
                    results.push(PortResult {
                        port,
                        status: "open".to_string(),
                        service: get_service_name(port),
                    });
                }
                Ok(Err(_)) | Err(_) => {}
            }
            if let Some(ref pb) = progress {
                pb.inc(1);
            }
            drop(permit);
        });
        
        handles.push(handle);
    }

    join_all(handles).await;
    if let Some(ref pb) = progress {
        pb.finish_and_clear();
    }

    let mut results = results.lock().await.clone();
    results.sort_by_key(|p| p.port);
    
    Ok(PortScanResults {
        host: host.to_string(),
        ports_scanned: ports_count as u16,
        open_ports: results,
        duration_ms: start.elapsed().as_millis() as u64,
        spoof_stats: None,
    })
}

#[cfg(all(feature = "stress-testing", unix))]
async fn scan_ports_spoofed(
    host: &str,
    ports: Vec<u16>,
    concurrency: usize,
    timeout_duration: Duration,
    tui_mode: bool,
    spoof_config: SpoofConfig,
) -> Result<PortScanResults> {
    use crate::scanner::spoof::{build_syn_packet, build_tcp_packet, build_fragmented_packets, get_network_interface, get_local_ip, random_ip_from_cidr};
    use pnet::datalink::{Channel::Ethernet, Config};
    use std::net::Ipv4Addr;
    use rand::Rng;
    use std::sync::Arc as StdArc;
    
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
                .unwrap()
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
            random_ip_from_cidr(range).map_err(|e| anyhow::anyhow!("Invalid spoof range '{}': {}", range, e))?
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
            
            let total_packets = if has_decoys { 1 + decoy_count_for_port(&spoof_config, port) } else { 1 };
            if let Some(rate) = max_rate {
                let delay_ms = (1000 * total_packets as u64) / rate as u64;
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            }
            
            if do_fragment {
                match build_fragmented_packets(src_ip, src_port, target_ipv4, port, seq, scan_type, ttl) {
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
                        
                        let mut all_ips: Vec<(Ipv4Addr, u16, u32)> = decoy_ips.iter()
                            .take(decoy_count)
                            .enumerate()
                            .map(|(i, ip)| (*ip, src_port.wrapping_add(i as u16), seq.wrapping_add(i as u32)))
                            .collect();
                        
                        for i in 0..all_ips.len() {
                            let j = i + rand::random::<usize>() % (all_ips.len() - i);
                            all_ips.swap(i, j);
                        }
                        
                        for (ip, port_offset, seq_offset) in all_ips {
                            if let Ok(packet) = build_tcp_packet(ip, port_offset, target_ipv4, port, seq_offset, scan_type, ttl) {
                                let _ = tx_guard.send_to(&packet, Some(interface.clone()));
                                packets_sent.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            }
                        }
                    }
                    crate::scanner::spoof::DecoyMode::Staggered => {
                        let mut all_ips: Vec<(Ipv4Addr, u16, u32)> = decoy_ips.iter()
                            .take(decoy_count)
                            .enumerate()
                            .map(|(i, ip)| (*ip, src_port.wrapping_add(i as u16), seq.wrapping_add(i as u32)))
                            .collect();
                        
                        for i in 0..all_ips.len() {
                            let j = i + rand::random::<usize>() % (all_ips.len() - i);
                            all_ips.swap(i, j);
                        }
                        
                        for (ip, port_offset, seq_offset) in all_ips {
                            if let Ok(packet) = build_tcp_packet(ip, port_offset, target_ipv4, port, seq_offset, scan_type, ttl) {
                                {
                                    let mut tx_guard = tx.lock();
                                    let _ = tx_guard.send_to(&packet, Some(interface.clone()));
                                    packets_sent.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                }
                                if use_staggered {
                                    let stagger_delay = 10 + rand::random::<u64>() % 100;
                                    tokio::time::sleep(tokio::time::Duration::from_millis(stagger_delay)).await;
                                }
                            }
                        }
                    }
                }
            }
            
            let status = if has_decoys { "decoy" } else { "spoofed" };
            
            if packet_trace.is_some() {
                log_packet_trace(&src_ip.to_string(), src_port, &target_ipv4.to_string(), port, &format!("{:?}", scan_type));
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
    
    let spoof_stats = Some(SpoofStats {
        packets_sent: packets_sent.load(std::sync::atomic::Ordering::Relaxed),
        packets_dropped: 0,
        spoofed_ips_used: if spoof_config.has_decoys() { spoof_config.decoy_ips.len() + 1 } else { 1 },
        decoys_used: if spoof_config.has_decoys() { spoof_config.decoy_count } else { 0 },
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
async fn scan_ports_spoofed(
    _host: &str,
    _ports: Vec<u16>,
    _concurrency: usize,
    _timeout_duration: Duration,
    _tui_mode: bool,
    _spoof_config: SpoofConfig,
) -> Result<PortScanResults> {
    anyhow::bail!("IP spoofing requires 'stress-testing' feature and Unix system");
}


