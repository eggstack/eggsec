//! Port scanning module.
//!
//! Provides TCP port scanning with support for concurrent connections,
//! spoofed scanning (with stress-testing feature), and various output formats.

mod spoofed;

use crate::error::Result;
#[cfg(feature = "cli")]
use crate::output::escape::escape_xml;
#[cfg(feature = "cli")]
use crate::scanner::spoof::format_spoof_warning;
use crate::scanner::spoof::{SpoofConfig, SpoofStats};
use crate::utils::connect_with_nodelay_timeout;
#[cfg(feature = "cli")]
use crate::utils::parsing::parse_ports;
use crate::utils::parsing::resolve_host;
#[cfg(feature = "cli")]
use crate::utils::sanitize_for_logging;
use crate::utils::strip_controls;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
#[cfg(feature = "cli")]
use std::fmt::Write as FmtWrite;
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "cli")]
use crate::cli::PortScanArgs;
#[cfg(any(feature = "tool-api", feature = "cli"))]
use crate::config::EggsecConfig;
use crate::scanner::service_data::get_service_name as get_service_name_from_utils;

pub const MAX_SCAN_RESULTS: usize = 10000;

fn get_service_name(port: u16) -> &'static str {
    get_service_name_from_utils(port)
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

/// Plain port-scan request (no Clap derives).
///
/// This is the engine-facing contract used by the pipeline, Python bindings,
/// and tool/API consumers. CLI parsing converts `PortScanArgs` into this type.
#[derive(Debug, Clone)]
pub struct PortScanRequest {
    pub host: String,
    pub ports: Vec<u16>,
    pub concurrency: usize,
    pub timeout: u64,
    pub spoof_config: SpoofConfig,
    pub dry_run: bool,
}

impl PortScanRequest {
    pub fn new(host: impl Into<String>, ports: Vec<u16>) -> Self {
        Self {
            host: host.into(),
            ports,
            concurrency: 100,
            timeout: 2,
            spoof_config: SpoofConfig::default(),
            dry_run: false,
        }
    }
}

#[cfg(feature = "cli")]
pub fn port_scan_request_from_args(
    args: crate::cli::PortScanArgs,
) -> crate::error::Result<PortScanRequest> {
    let ports = parse_ports(&args.ports)?;
    let spoof_config = SpoofConfig::from_args(
        args.source_ip,
        args.spoof_range,
        false,
        args.decoy,
        args.decoy_range,
        args.decoy_count,
        args.decoy_mode,
        args.include_me,
        args.source_port,
        args.random_source_port,
        args.fragment,
        args.scan_type,
        args.packet_trace,
        args.max_rate,
        args.ttl,
    )?;
    Ok(PortScanRequest {
        host: args.host,
        ports,
        concurrency: args.concurrency,
        timeout: args.timeout,
        spoof_config,
        dry_run: args.dry_run,
    })
}

pub use spoofed::{init_packet_trace, shutdown_packet_trace};

/// Transport a port verdict was reached over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PortProtocol {
    #[default]
    Tcp,
    Udp,
}

impl PortProtocol {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Udp => "udp",
        }
    }
}

impl std::fmt::Display for PortProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A per-port verdict.
///
/// Typed rather than a `String` because the set is closed and the differences
/// carry real meaning: `closed` and `open` are *proofs* (an RST or an ICMP
/// port-unreachable arrived), while `filtered` and `open|filtered` are the
/// *absence* of a proof. A reader cannot tell those apart by string
/// comparison, and collapsing `open|filtered` into `open` is exactly the
/// over-claim the UDP path exists to avoid.
///
/// Deserialization rejects an unrecognized value instead of guessing. A
/// payload from a different build should fail loudly rather than have its
/// verdict silently coerced into a state this enum does not model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PortStatus {
    /// A SYN/ACK arrived: proved listening.
    Open,
    /// An RST or ICMP port-unreachable arrived: proved not listening.
    Closed,
    /// No response for this port, but the host answered other probes. A
    /// firewall drop fits; nothing else was proven either way.
    Filtered,
    /// No response for this port and the host proved nothing at all, so the
    /// port is either open-and-filtered or closed-and-filtered. UDP cannot
    /// tell which, so this scanner does not claim either.
    #[serde(rename = "open|filtered")]
    OpenFiltered,
}

impl PortStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::Filtered => "filtered",
            Self::OpenFiltered => "open|filtered",
        }
    }

    /// Whether this verdict is a proof rather than an absence of one.
    pub const fn is_proof(self) -> bool {
        matches!(self, Self::Open | Self::Closed)
    }
}

impl std::fmt::Display for PortStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortResult {
    pub port: u16,
    pub status: PortStatus,
    /// Transport the verdict was reached over.
    ///
    /// `#[serde(default)]` = TCP, which is what every `PortResult` predating
    /// UDP scanning was, so payloads written by an older build still load.
    /// `PortResult` crosses the daemon protocol, the Python bindings and the
    /// report model, so this field is wire-affecting.
    #[serde(default)]
    pub protocol: PortProtocol,
    pub service: String,
}

/// UDP host-liveness verdict, mirrored into the report envelope.
///
/// An engine-owned type rather than `eggsec_udp_scan::HostState` on purpose:
/// `PortScanResults` crosses the daemon protocol, the Python bindings and the
/// report model, so embedding a scanner-crate type would couple the wire
/// contract to that crate's internals -- and would drag `serde` into a crate
/// that deliberately has none.
#[cfg(feature = "udp-scan")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UdpHostState {
    /// At least one ICMP error was attributed to one of our probes.
    Up,
    /// The window elapsed with no attributable ICMP. Per-port results from
    /// such a run describe nothing: a dead host and a fully-filtered host are
    /// indistinguishable on the wire.
    Unresponsive,
    /// Errors arrived but none could be attributed.
    Indeterminate,
}

#[cfg(feature = "udp-scan")]
impl std::fmt::Display for UdpHostState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Up => "up",
            Self::Unresponsive => "unresponsive (no port verdicts are meaningful)",
            Self::Indeterminate => "indeterminate",
        })
    }
}

/// How much signal backed a UDP run's verdicts.
///
/// Reported so a caller can see evidence density rather than over-claiming: a
/// run with zero correlated errors and a full port list is not a port list.
#[cfg(feature = "udp-scan")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct UdpEvidenceSummary {
    pub correlated: u64,
    pub orphans: u64,
    pub expired: u64,
    pub unparseable: u64,
    pub sweeps: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PortScanResults {
    pub host: String,
    pub ports_scanned: u32,
    pub open_ports: Vec<PortResult>,
    pub total_open_ports: usize,
    pub results_truncated: bool,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spoof_stats: Option<SpoofStats>,
    /// UDP host-liveness verdict. `None` for TCP scans.
    ///
    /// `#[serde(default)]` so a payload written before this field existed still
    /// deserializes -- `PortScanResults` crosses the daemon protocol, the
    /// Python bindings and the report model.
    #[cfg_attr(
        feature = "udp-scan",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    #[cfg(feature = "udp-scan")]
    pub udp_host_state: Option<UdpHostState>,
    /// UDP ICMP evidence density. `None` for TCP scans.
    #[cfg_attr(
        feature = "udp-scan",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    #[cfg(feature = "udp-scan")]
    pub udp_evidence: Option<UdpEvidenceSummary>,
}

impl PortScanResults {
    /// Whether these results came from a UDP run.
    ///
    /// Derived from the port records themselves rather than from
    /// `udp_host_state`, so it stays correct in a build without the
    /// `udp-scan` feature and does not depend on a field a caller may have
    /// dropped in transit.
    pub fn is_udp(&self) -> bool {
        self.open_ports
            .iter()
            .any(|p| p.protocol == PortProtocol::Udp)
    }

    /// Ports whose verdict actually proves something is listening.
    ///
    /// `open_ports` is a misnomer for a UDP run: it holds every probed port,
    /// most of which are `open|filtered`. Count only the proofs.
    pub fn proved_open_ports(&self) -> usize {
        self.open_ports
            .iter()
            .filter(|p| p.status == PortStatus::Open)
            .count()
    }
}

impl std::fmt::Display for PortScanResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Port Scan Results")?;
        writeln!(f, "host: {}", strip_controls(&self.host, 60))?;
        writeln!(f, "scanned: {} ports", self.ports_scanned)?;

        // A UDP run lists a verdict for every probed port, so counting the
        // list as "open" would undercount and imply the rest are closed.
        if self.is_udp() {
            writeln!(f, "verdicts: {} ports", self.open_ports.len())?;
        } else {
            writeln!(f, "open: {} ports", self.open_ports.len())?;
        }

        #[cfg(feature = "udp-scan")]
        if let Some(state) = self.udp_host_state {
            writeln!(f, "host state: {state}")?;
        }

        if self.open_ports.is_empty() {
            if self.is_udp() {
                writeln!(f, "no port verdicts reported")?;
            } else {
                writeln!(f, "no open ports")?;
            }
        } else {
            writeln!(f, "ports")?;
            for port in &self.open_ports {
                writeln!(
                    f,
                    "\t{}/{}\t{}\t{}",
                    port.port, port.protocol, port.status, port.service
                )?;
            }
        }

        Ok(())
    }
}

/// Run a `--udp` scan from the CLI.
///
/// Routes through the same `run_port_scan` entry point every other surface
/// uses, so the CLI cannot drift from the TUI/daemon/REST behavior -- and so
/// a build without the `udp-scan` feature fails closed here with the same
/// message rather than quietly scanning TCP instead.
#[cfg(feature = "cli")]
async fn run_udp_scan_cli(args: &PortScanArgs, timeout_secs: u64) -> Result<PortScanResults> {
    // Spoofing, decoys, fragmentation and dry-run are raw-TCP-SYN concepts.
    // The UDP path sends from a real socket and correlates ICMP, so accepting
    // these flags and ignoring them would be a lie; say so instead.
    if args.source_ip.is_some()
        || args.spoof_range.is_some()
        || args.decoy.is_some()
        || args.decoy_range.is_some()
        || args.decoy_count.is_some()
        || args.include_me
        || args.source_port.is_some()
        || args.random_source_port
        || args.fragment
        || args.packet_trace.is_some()
        || args.max_rate.is_some()
        || args.ttl.is_some()
    {
        eprintln!(
            "Warning: source IP, decoy, source-port, fragment, packet-trace, max-rate and ttl \
             options do not apply to a UDP scan and are ignored."
        );
    }
    if args.scan_type.is_some() {
        eprintln!("Warning: --scan-type selects a TCP technique and is ignored with --udp.");
    }

    // Kept alive for the duration so progress sends do not warn on a
    // disconnected channel.
    let (progress_tx, _progress_rx) = tokio::sync::mpsc::channel(16);

    match crate::dispatch::run_port_scan(
        args.host.clone(),
        args.ports.clone(),
        args.concurrency,
        Duration::from_secs(timeout_secs),
        true,
        progress_tx,
    )
    .await?
    {
        crate::dispatch::TaskResult::PortScan(results) => Ok(results),
        other => Err(crate::error::EggsecError::Internal(format!(
            "UDP scan returned an unexpected result: {other:?}"
        ))),
    }
}

#[cfg(feature = "cli")]
pub async fn run_cli(args: PortScanArgs, config: &EggsecConfig) -> Result<()> {
    if args.verbose {
        eprintln!(
            "Starting port scan on {} ports {}",
            sanitize_for_logging(&args.host),
            args.ports
        );
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
        if let Err(e) = init_packet_trace(trace_path, false) {
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
    let results = if args.udp {
        run_udp_scan_cli(&args, timeout_secs).await?
    } else {
        let port_args = PortScanConfig {
            ports,
            concurrency: args.concurrency,
            timeout_duration: Duration::from_secs(timeout_secs),
            tui_mode: false,
            spoof_config,
            progress_tx: None,
            max_results: None,
        };

        scan_ports(&args.host, port_args).await?
    };

    if args.verbose {
        eprintln!(
            "Scan complete: {} open ports found out of {} scanned",
            results.proved_open_ports(),
            ports_count
        );
    }

    let output = if args.json {
        serde_json::to_string_pretty(&results)?
    } else if args.grepable {
        let mut s = String::new();
        s.push_str("# Nmap grepable output\n");
        writeln!(s, "Host: {}", results.host).unwrap();
        s.push_str("Status: up\n");
        s.push_str("Ports: ");
        for (i, port) in results.open_ports.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            // Nmap grepable form is `port/state/proto//service`. Hardcoding
            // `/open/` would report every UDP port -- most of them
            // `open|filtered` -- as open.
            write!(s, "{}/{}/{}/", port.port, port.status, port.protocol).unwrap();
            s.push_str(&port.service);
        }
        s.push('\n');
        s
    } else if args.xml {
        let mut s = String::new();
        s.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        s.push_str("<nmaprun>\n");
        writeln!(s, "  <host>{}</host>", escape_xml(&results.host)).unwrap();
        s.push_str("  <ports>\n");
        for port in &results.open_ports {
            write!(
                s,
                r#"    <port protocol="{}" portid="{}"><state state="{}"/><service name="{}"/></port>"#,
                port.protocol,
                port.port,
                escape_xml(port.status.as_str()),
                escape_xml(&port.service)
            )
            .unwrap();
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

#[cfg(all(feature = "tool-api", feature = "cli"))]
pub type PortFindingCallback = Box<dyn FnMut(crate::tool::response::Finding) + Send + 'static>;

#[cfg(feature = "tool-api")]
pub async fn run_with_callback<F>(
    request: &PortScanRequest,
    config: &EggsecConfig,
    callback: F,
) -> Result<PortScanResults>
where
    F: FnMut(crate::tool::response::Finding) + Send + 'static,
{
    let timeout_secs = if request.timeout == 2 {
        config.scan.port_timeout_secs
    } else {
        request.timeout
    };

    if let Some(ref trace_path) = request.spoof_config.packet_trace {
        if let Err(e) = init_packet_trace(trace_path, false) {
            eprintln!("Warning: Failed to initialize packet trace: {}", e);
        }
    }

    if request.dry_run {
        return Ok(PortScanResults {
            host: request.host.clone(),
            ports_scanned: request.ports.len() as u32,
            open_ports: Vec::new(),
            total_open_ports: 0,
            results_truncated: false,
            duration_ms: 0,
            spoof_stats: None,
            #[cfg(feature = "udp-scan")]
            udp_host_state: None,
            #[cfg(feature = "udp-scan")]
            udp_evidence: None,
        });
    }

    let config = PortScanConfig {
        ports: request.ports.clone(),
        concurrency: request.concurrency,
        timeout_duration: Duration::from_secs(timeout_secs),
        tui_mode: false,
        spoof_config: request.spoof_config.clone(),
        progress_tx: None,
        max_results: None,
    };
    let results = scan_ports(&request.host, config).await?;

    let mut callback = callback;
    for port_result in &results.open_ports {
        callback(crate::tool::response::Finding::from(port_result.clone()));
    }

    Ok(results)
}

#[cfg(all(feature = "tool-api", feature = "cli"))]
pub async fn run_cli_with_callback<F>(
    args: PortScanArgs,
    config: &EggsecConfig,
    callback: F,
) -> Result<()>
where
    F: FnMut(crate::tool::response::Finding) + Send + 'static,
{
    if args.verbose {
        eprintln!(
            "Starting port scan on {} ports {}",
            sanitize_for_logging(&args.host),
            args.ports
        );
    }

    let mut request = port_scan_request_from_args(args.clone())?;

    if request.spoof_config.enabled {
        eprintln!("{}", format_spoof_warning(&request.spoof_config));
    }

    if args.dry_run {
        eprintln!("\n=== DRY RUN MODE ===");
        eprintln!("Target: {}", sanitize_for_logging(&args.host));
        eprintln!("Ports: {}", args.ports);
        eprintln!("Concurrency: {}", args.concurrency);
        eprintln!("Timeout: {}s", request.timeout);
        if request.spoof_config.enabled {
            if let Some(ref ip) = request.spoof_config.source_ip {
                eprintln!("Spoof Source IP: {}", ip);
            }
            if let Some(ref range) = request.spoof_config.ip_range {
                eprintln!("Spoof IP Range: {}", range);
            }
            if let Some(port) = request.spoof_config.source_port {
                eprintln!("Source Port: {}", port);
            }
            if request.spoof_config.random_source_port {
                eprintln!("Source Port: RANDOM");
            }
            if request.spoof_config.fragment {
                eprintln!("Fragmentation: YES (8-byte fragments)");
            }
            eprintln!("Scan Type: {:?}", request.spoof_config.scan_type);
            if let Some(ref trace) = request.spoof_config.packet_trace {
                eprintln!("Packet Trace: {}", trace);
            }
            if let Some(rate) = request.spoof_config.max_rate {
                eprintln!("Max Rate: {} pps", rate);
            }
            if let Some(ttl) = request.spoof_config.ttl {
                eprintln!("TTL: {}", ttl);
            }
            if !request.spoof_config.decoy_ips.is_empty() {
                eprintln!("Decoy IPs: {} total", request.spoof_config.decoy_ips.len());
                for ip in request.spoof_config.decoy_ips.iter().take(5) {
                    eprintln!("  - {}", ip);
                }
                if request.spoof_config.decoy_ips.len() > 5 {
                    eprintln!(
                        "  ... and {} more",
                        request.spoof_config.decoy_ips.len() - 5
                    );
                }
                eprintln!("Decoy Mode: {:?}", request.spoof_config.decoy_mode);
                if request.spoof_config.include_real_ip {
                    eprintln!("Include Real IP: YES");
                }
            }
        }
        eprintln!("===================\n");
        return Ok(());
    }

    let ports_count = request.ports.len();

    // For callback path, dry_run is already handled by run_with_callback if
    // set. We reset it here to ensure dry-run mode goes through the same
    // presentation flow as before.
    request.dry_run = false;

    let results = run_with_callback(&request, config, callback).await?;

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
        write!(s, "Host: {}\n", results.host).unwrap();
        s.push_str("Status: up\n");
        s.push_str("Ports: ");
        for (i, port) in results.open_ports.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            write!(s, "{}/open/{}", port.port, port.service).unwrap();
        }
        s.push('\n');
        s
    } else if args.xml {
        let mut s = String::new();
        s.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        s.push_str("<nmaprun>\n");
        write!(s, "  <host>{}</host>\n", escape_xml(&results.host)).unwrap();
        s.push_str("  <ports>\n");
        for port in &results.open_ports {
            write!(
                s,
                r#"    <port protocol="tcp" portid="{}"><state state="open"/><service name="{}"/></port>"#,
                port.port, port.service
            )
            .unwrap();
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

pub async fn scan_ports(host: &str, config: PortScanConfig) -> Result<PortScanResults> {
    if config.concurrency == 0 {
        return Err(crate::error::EggsecError::Runtime(
            "concurrency must be greater than zero".to_string(),
        ));
    }
    if config.spoof_config.enabled && config.spoof_config.use_raw_sockets {
        return spoofed::scan_ports_spoofed(
            host,
            config.ports,
            config.concurrency,
            config.timeout_duration,
            config.tui_mode,
            config.spoof_config,
            config.progress_tx,
            config.max_results,
        )
        .await;
    }

    let addr = resolve_host(host)?;
    let total_ports = config.ports.len() as u64;
    let ports = config.ports;
    let concurrency = config.concurrency;
    let timeout_dur = config.timeout_duration;
    let max_results = config.max_results;
    let progress_tx = config.progress_tx.clone();

    let progress = if config.tui_mode {
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

    // Phase B: bounded scheduler. At most `concurrency` connect tasks are in
    // flight; tasks return their result to the parent instead of mutating a
    // shared map, so retained handle state is O(concurrency), not O(ports).
    let start = std::time::Instant::now();
    let ports_count = ports.len();
    let mut open_results: Vec<PortResult> = Vec::new();
    let mut total_matches_count: usize = 0;
    let mut admitted_count: u64 = 0;
    let mut scanned_count: u64 = 0;
    let mut in_flight = tokio::task::JoinSet::new();
    let mut next: usize = 0;

    while next < ports.len() || !in_flight.is_empty() {
        while next < ports.len() && in_flight.len() < concurrency {
            let port = ports[next];
            next += 1;

            // Project invariant: every spawned tokio task carries a timeout
            // wrapper (30-300s).
            in_flight.spawn(tokio::time::timeout(
                std::time::Duration::from_secs(300),
                async move {
                    let socket_addr = std::net::SocketAddr::new(addr, port);
                    if connect_with_nodelay_timeout(&socket_addr, timeout_dur)
                        .await
                        .is_ok()
                    {
                        Some(PortResult {
                            port,
                            status: PortStatus::Open,
                            protocol: PortProtocol::Tcp,
                            service: get_service_name(port).to_string(),
                        })
                    } else {
                        None
                    }
                },
            ));
        }

        if let Some(join_result) = in_flight.join_next().await {
            match join_result {
                Ok(Ok(Some(result))) => {
                    total_matches_count += 1;
                    // Completion-order selection, as before: the first
                    // `limit` completions win.
                    let should_insert = match max_results {
                        Some(limit) => {
                            let old = admitted_count;
                            admitted_count += 1;
                            old < limit as u64
                        }
                        None => true,
                    };
                    if should_insert {
                        open_results.push(result);
                    }
                }
                Ok(Ok(None)) => {}
                Ok(Err(_)) => {
                    tracing::debug!("Port scan worker timed out after 300s");
                }
                Err(e) => {
                    tracing::warn!("Port scan worker task join failure: {}", e);
                }
            }
            if let Some(ref pb) = progress {
                pb.inc(1);
            }
            if let Some(ref tx) = progress_tx {
                scanned_count += 1;
                if tx.send((scanned_count, total_ports)).await.is_err() {
                    tracing::warn!("Progress receiver dropped");
                }
            }
        }
    }
    if let Some(ref pb) = progress {
        pb.finish_and_clear();
    }

    open_results.sort_by_key(|p| p.port);

    let results_truncated = open_results.len() > MAX_SCAN_RESULTS;
    if results_truncated {
        open_results.truncate(MAX_SCAN_RESULTS);
    }

    let open_ports: Vec<PortResult> = open_results
        .into_iter()
        .filter(|p| p.status == PortStatus::Open)
        .collect();

    Ok(PortScanResults {
        host: host.to_string(),
        ports_scanned: u32::try_from(ports_count).map_err(|_| {
            crate::error::EggsecError::Internal("port count exceeds u32::MAX".into())
        })?,
        open_ports,
        total_open_ports: total_matches_count,
        results_truncated,
        duration_ms: start.elapsed().as_millis() as u64,
        spoof_stats: None,
        #[cfg(feature = "udp-scan")]
        udp_host_state: None,
        #[cfg(feature = "udp-scan")]
        udp_evidence: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::service_data::COMMON_PORTS;

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
            status: PortStatus::Open,
            protocol: PortProtocol::Tcp,
            service: "HTTP".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: PortResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.port, 80);
        assert_eq!(deserialized.status, PortStatus::Open);
        assert_eq!(deserialized.service, "HTTP");
    }

    #[test]
    fn test_port_scan_results_display_empty() {
        let results = PortScanResults {
            host: "example.com".to_string(),
            ports_scanned: 100,
            open_ports: vec![],
            total_open_ports: 0,
            results_truncated: false,
            duration_ms: 5000,
            spoof_stats: None,

            #[cfg(feature = "udp-scan")]
            udp_host_state: None,
            #[cfg(feature = "udp-scan")]
            udp_evidence: None,
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
                    status: PortStatus::Open,
                    protocol: PortProtocol::Tcp,
                    service: "SSH".to_string(),
                },
                PortResult {
                    port: 80,
                    status: PortStatus::Open,
                    protocol: PortProtocol::Tcp,
                    service: "HTTP".to_string(),
                },
                PortResult {
                    port: 443,
                    status: PortStatus::Open,
                    protocol: PortProtocol::Tcp,
                    service: "HTTPS".to_string(),
                },
            ],
            total_open_ports: 3,
            results_truncated: false,
            duration_ms: 3000,
            spoof_stats: None,

            #[cfg(feature = "udp-scan")]
            udp_host_state: None,
            #[cfg(feature = "udp-scan")]
            udp_evidence: None,
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
        assert_eq!(
            ports.len(),
            before_len,
            "COMMON_PORTS contains duplicate port numbers"
        );
    }

    #[test]
    fn test_common_ports_in_range() {
        for (port, _) in COMMON_PORTS {
            assert!(*port > 0, "Port 0 should not be in COMMON_PORTS");
        }
    }
}
