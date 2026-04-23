//! Port scanning module.
//!
//! Provides TCP port scanning with support for concurrent connections,
//! spoofed scanning (with stress-testing feature), and various output formats.

mod spoofed;

use crate::scanner::spoof::{format_spoof_warning, SpoofConfig, SpoofStats};
use crate::utils::parsing::{parse_ports, resolve_host};
use crate::utils::strip_controls;
use crate::utils::sanitize_for_logging;
use crate::utils::connect_with_nodelay_timeout;
use crate::output::escape::escape_xml;
use crate::error::Result;
use futures::future::join_all;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Duration;
use dashmap::DashMap;
use rustc_hash::FxHashMap;

use crate::cli::PortScanArgs;
use crate::config::SlapperConfig;

pub const MAX_SCAN_RESULTS: usize = 10000;
pub const COMMON_PORTS: &[(u16, &str)] = &[
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

static COMMON_PORTS_MAP: LazyLock<FxHashMap<u16, &'static str>> = LazyLock::new(|| {
    let mut m = FxHashMap::default();
    m.insert(21, "FTP");
    m.insert(22, "SSH");
    m.insert(23, "Telnet");
    m.insert(25, "SMTP");
    m.insert(53, "DNS");
    m.insert(80, "HTTP");
    m.insert(110, "POP3");
    m.insert(143, "IMAP");
    m.insert(443, "HTTPS");
    m.insert(445, "SMB");
    m.insert(993, "IMAPS");
    m.insert(995, "POP3S");
    m.insert(1433, "MSSQL");
    m.insert(1521, "Oracle");
    m.insert(3306, "MySQL");
    m.insert(3389, "RDP");
    m.insert(5432, "PostgreSQL");
    m.insert(5900, "VNC");
    m.insert(6379, "Redis");
    m.insert(8080, "HTTP-Alt");
    m.insert(8443, "HTTPS-Alt");
    m.insert(27017, "MongoDB");
    m
});

fn get_service_name(port: u16) -> &'static str {
    COMMON_PORTS_MAP.get(&port).copied().unwrap_or("unknown")
}

#[derive(Debug, Clone)]
pub struct PortScanConfig {
    pub ports: Vec<u16>,
    pub concurrency: usize,
    pub timeout_duration: Duration,
    pub tui_mode: bool,
    pub spoof_config: SpoofConfig,
    pub progress_tx: Option<tokio::sync::mpsc::Sender<(u64, u64)>>,
    pub max_results: Option<usize>,
}

impl Default for PortScanConfig {
    fn default() -> Self {
        Self {
            ports: Vec::new(),
            concurrency: 100,
            timeout_duration: Duration::from_secs(3),
            tui_mode: false,
            spoof_config: SpoofConfig::default(),
            progress_tx: None,
            max_results: None,
        }
    }
}

impl PortScanConfig {
    pub fn new(ports: Vec<u16>) -> Self {
        Self {
            ports,
            ..Default::default()
        }
    }
}

pub use spoofed::init_packet_trace;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortResult {
    pub port: u16,
    pub status: String,
    pub service: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PortScanResults {
    pub host: String,
    pub ports_scanned: u32,
    pub open_ports: Vec<PortResult>,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spoof_stats: Option<SpoofStats>,
}

impl std::fmt::Display for PortScanResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Port Scan Results")?;
        writeln!(f, "host: {}", strip_controls(&self.host, 60))?;
        writeln!(f, "scanned: {} ports", self.ports_scanned)?;
        writeln!(f, "open: {} ports", self.open_ports.len())?;

        if self.open_ports.is_empty() {
            writeln!(f, "no open ports")?;
        } else {
            let _ = writeln!(f, "open ports");
            for port in &self.open_ports {
                writeln!(f, "\t{}/tcp\t{}\t{}", port.port, port.status, port.service)?;
            }
        }

        Ok(())
    }
}

pub async fn run_cli(args: PortScanArgs, config: &SlapperConfig) -> Result<()> {
    if args.verbose {
        eprintln!("Starting port scan on {} ports {}", sanitize_for_logging(&args.host), args.ports);
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
        eprintln!("Target: {}", sanitize_for_logging(&args.host));
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
                for ip in spoof_config.decoy_ips.iter().take(5) {
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
    let port_args = PortScanConfig {
        ports,
        concurrency: args.concurrency,
        timeout_duration: Duration::from_secs(timeout_secs),
        tui_mode: false,
        spoof_config,
        progress_tx: None,
        max_results: None,
    };

    let results = scan_ports(&args.host, port_args).await?;

    if args.verbose {
        eprintln!(
            "Scan complete: {} open ports found out of {} scanned",
            results.open_ports.len(),
            ports_count
        );
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
        s.push('\n');
        s
    } else if args.xml {
        let mut s = String::new();
        s.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        s.push_str("<nmaprun>\n");
        s.push_str(&format!("  <host>{}</host>\n", escape_xml(&results.host)));
        s.push_str("  <ports>\n");
        for port in &results.open_ports {
            s.push_str(&format!(
                r#"    <port protocol="tcp" portid="{}"><state state="open"/><service name="{}"/></port>"#,
                port.port, port.service
            ));
            s.push('\n');
        }
        s.push_str("  </ports>\n");
        s.push_str("</nmaprun>\n");
        s
    } else {
        format!("{}", results)
    };

    if let Some(ref output_file) = args.output {
        tokio::fs::write(output_file, &output).await?;
        if args.verbose {
            eprintln!("Results written to {}", output_file);
        }
    } else {
        println!("{}", output);
    }

    Ok(())
}

#[cfg(feature = "tool-api")]
pub type PortFindingCallback = Box<dyn FnMut(crate::tool::response::Finding) + Send + 'static>;

#[cfg(feature = "tool-api")]
pub async fn run_cli_with_callback<F>(args: PortScanArgs, config: &SlapperConfig, mut callback: F) -> Result<()>
where
    F: FnMut(crate::tool::response::Finding) + Send + 'static,
{
    if args.verbose {
        eprintln!("Starting port scan on {} ports {}", sanitize_for_logging(&args.host), args.ports);
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
        eprintln!("Target: {}", sanitize_for_logging(&args.host));
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
                for ip in spoof_config.decoy_ips.iter().take(5) {
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
        spoof_config,
        None,
        None,
    )
    .await?;

    if args.verbose {
        eprintln!(
            "Scan complete: {} open ports found out of {} scanned",
            results.open_ports.len(),
            ports_count
        );
    }

    for port_result in &results.open_ports {
        callback(crate::tool::response::Finding::from(port_result.clone()));
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
        s.push('\n');
        s
    } else if args.xml {
        let mut s = String::new();
        s.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        s.push_str("<nmaprun>\n");
        s.push_str(&format!("  <host>{}</host>\n", escape_xml(&results.host)));
        s.push_str("  <ports>\n");
        for port in &results.open_ports {
            s.push_str(&format!(
                r#"    <port protocol="tcp" portid="{}"><state state="open"/><service name="{}"/></port>"#,
                port.port, port.service
            ));
            s.push('\n');
        }
        s.push_str("  </ports>\n");
        s.push_str("</nmaprun>\n");
        s
    } else {
        format!("{}", results)
    };

    if let Some(ref output_file) = args.output {
        tokio::fs::write(output_file, &output).await?;
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
    config: PortScanConfig,
) -> Result<PortScanResults> {
    if config.spoof_config.enabled && config.spoof_config.use_raw_sockets {
        return spoofed::scan_ports_spoofed(
            host,
            config.ports,
            config.concurrency,
            config.timeout_duration,
            config.tui_mode,
            config.spoof_config,
            config.progress_tx,
        )
        .await;
    }

    let addr = resolve_host(host)?;
    let results: Arc<DashMap<u16, PortResult>> = Arc::new(DashMap::new());
    let scanned_count = Arc::new(AtomicU64::new(0));
    let results_count = Arc::new(tokio::sync::Mutex::new(0usize));
    let total_ports = config.ports.len() as u64;

    let progress = if config.tui_mode {
        None
    } else {
        let pb = Arc::new(ProgressBar::new(config.ports.len() as u64));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ports ({eta})")
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars("#>-"),
        );
        Some(pb)
    };

    let semaphore = Arc::new(tokio::sync::Semaphore::new(config.concurrency));
    let mut handles = Vec::with_capacity(config.ports.len());
    let start = std::time::Instant::now();
    let ports_count = config.ports.len();

    for port in config.ports {
        let permit = semaphore.clone().acquire_owned().await?;
        let results = results.clone();
        let progress = progress.clone();
        let timeout_dur = config.timeout_duration;
        let scanned_count = scanned_count.clone();
        let progress_tx = config.progress_tx.clone();
        let results_count = results_count.clone();

        let handle = tokio::spawn(async move {
            let socket_addr = std::net::SocketAddr::new(addr, port);
            let result = connect_with_nodelay_timeout(&socket_addr, timeout_dur).await;

            match result {
                Ok(_) => {
                    let should_insert = match config.max_results {
                        Some(limit) => {
                            let count = *results_count.lock().await;
                            if count >= limit {
                                false
                            } else {
                                *results_count.lock().await += 1;
                                true
                            }
                        }
                        None => true,
                    };
                    if should_insert {
                        results.insert(port, PortResult {
                            port,
                            status: "open".to_string(),
                            service: get_service_name(port).to_string(),
                        });
                    }
                }
                Err(_) => {
                    let should_insert = match config.max_results {
                        Some(limit) => {
                            let count = *results_count.lock().await;
                            if count >= limit {
                                false
                            } else {
                                *results_count.lock().await += 1;
                                true
                            }
                        }
                        None => true,
                    };
                    if should_insert {
                        results.insert(port, PortResult {
                            port,
                            status: "closed".to_string(),
                            service: get_service_name(port).to_string(),
                        });
                    }
                }
            }
            if let Some(ref pb) = progress {
                pb.inc(1);
            }
            if let Some(ref tx) = progress_tx {
                let count = scanned_count.fetch_add(1, Ordering::Relaxed) + 1;
                let _ = tx.send((count, total_ports)).await;
            }
            drop(permit);
        });

        handles.push(handle);
    }

    join_all(handles).await;
    if let Some(ref pb) = progress {
        pb.finish_and_clear();
    }

    let mut results: Vec<PortResult> = DashMap::clone(&results).into_iter().map(|(_, v)| v).collect();
    results.sort_by_key(|p| p.port);

    if results.len() > MAX_SCAN_RESULTS {
        results.truncate(MAX_SCAN_RESULTS);
    }

    Ok(PortScanResults {
        host: host.to_string(),
        ports_scanned: ports_count as u32,
        open_ports: results,
        duration_ms: start.elapsed().as_millis() as u64,
        spoof_stats: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_service_name_known_ports() {
        assert_eq!(get_service_name(21), "FTP");
        assert_eq!(get_service_name(22), "SSH");
        assert_eq!(get_service_name(23), "Telnet");
        assert_eq!(get_service_name(25), "SMTP");
        assert_eq!(get_service_name(53), "DNS");
        assert_eq!(get_service_name(80), "HTTP");
        assert_eq!(get_service_name(110), "POP3");
        assert_eq!(get_service_name(143), "IMAP");
        assert_eq!(get_service_name(443), "HTTPS");
        assert_eq!(get_service_name(445), "SMB");
        assert_eq!(get_service_name(993), "IMAPS");
        assert_eq!(get_service_name(995), "POP3S");
        assert_eq!(get_service_name(1433), "MSSQL");
        assert_eq!(get_service_name(1521), "Oracle");
        assert_eq!(get_service_name(3306), "MySQL");
        assert_eq!(get_service_name(3389), "RDP");
        assert_eq!(get_service_name(5432), "PostgreSQL");
        assert_eq!(get_service_name(5900), "VNC");
        assert_eq!(get_service_name(6379), "Redis");
        assert_eq!(get_service_name(8080), "HTTP-Alt");
        assert_eq!(get_service_name(8443), "HTTPS-Alt");
        assert_eq!(get_service_name(27017), "MongoDB");
    }

    #[test]
    fn test_get_service_name_unknown_port() {
        assert_eq!(get_service_name(12345), "unknown");
        assert_eq!(get_service_name(0), "unknown");
        assert_eq!(get_service_name(65535), "unknown");
    }

    #[test]
    fn test_port_result_serialization() {
        let result = PortResult {
            port: 80,
            status: "open".to_string(),
            service: "HTTP".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: PortResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.port, 80);
        assert_eq!(deserialized.status, "open");
        assert_eq!(deserialized.service, "HTTP");
    }

    #[test]
    fn test_port_scan_results_display_empty() {
        let results = PortScanResults {
            host: "example.com".to_string(),
            ports_scanned: 100,
            open_ports: vec![],
            duration_ms: 5000,
            spoof_stats: None,
        };
        let output = format!("{}", results);
        assert!(output.contains("Port Scan Results"));
        assert!(output.contains("example.com"));
        assert!(output.contains("no open ports"));
    }

    #[test]
    fn test_port_scan_results_display_with_ports() {
        let results = PortScanResults {
            host: "192.168.1.1".to_string(),
            ports_scanned: 1000,
            open_ports: vec![
                PortResult {
                    port: 22,
                    status: "open".to_string(),
                    service: "SSH".to_string(),
                },
                PortResult {
                    port: 80,
                    status: "open".to_string(),
                    service: "HTTP".to_string(),
                },
                PortResult {
                    port: 443,
                    status: "open".to_string(),
                    service: "HTTPS".to_string(),
                },
            ],
            duration_ms: 3000,
            spoof_stats: None,
        };
        let output = format!("{}", results);
        assert!(output.contains("scanned: 1000 ports"));
        assert!(output.contains("open: 3 ports"));
        assert!(output.contains("22/tcp"));
        assert!(output.contains("80/tcp"));
        assert!(output.contains("443/tcp"));
        assert!(output.contains("SSH"));
        assert!(output.contains("HTTP"));
        assert!(output.contains("HTTPS"));
    }

    #[test]
    fn test_common_ports_all_unique() {
        let mut ports: Vec<u16> = COMMON_PORTS.iter().map(|(p, _)| *p).collect();
        let before_len = ports.len();
        ports.sort();
        ports.dedup();
        assert_eq!(ports.len(), before_len, "COMMON_PORTS contains duplicate port numbers");
    }

    #[test]
    fn test_common_ports_in_range() {
        for (port, _) in COMMON_PORTS {
            assert!(*port > 0, "Port 0 should not be in COMMON_PORTS");
            assert!(*port <= 65535, "Port {} exceeds u16 max", port);
        }
    }
}
