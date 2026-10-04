use crate::dispatch::types::{send_progress, TaskResult};
use crate::scanner::spoof::SpoofConfig;

pub async fn run_port_scan(
    target: String,
    ports: String,
    concurrency: usize,
    timeout: std::time::Duration,
    udp: bool,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
) -> anyhow::Result<TaskResult> {
    use crate::scanner::ports::scan_ports;

    send_progress(&progress_tx, 0, 100).await;

    if udp {
        #[cfg(feature = "udp-scan")]
        {
            return scan_ports_udp(target, ports, concurrency, timeout, progress_tx).await;
        }
        // Fail closed rather than silently scanning TCP: a caller that asked
        // for UDP and got TCP results would have no way to tell.
        #[cfg(not(feature = "udp-scan"))]
        {
            return Err(anyhow::anyhow!(
                "UDP scanning requested but the 'udp-scan' feature is not enabled"
            ));
        }
    }

    let port_list = crate::utils::parsing::parse_ports(&ports)?;
    let total_ports = port_list.len() as u64;

    send_progress(&progress_tx, 10, 100).await;

    let results = match tokio::time::timeout(
        std::time::Duration::from_secs(60),
        scan_ports(
            &target,
            crate::scanner::ports::PortScanConfig {
                ports: port_list,
                concurrency,
                timeout_duration: timeout,
                tui_mode: true,
                spoof_config: SpoofConfig::default(),
                progress_tx: Some(progress_tx.clone()),
                max_results: None,
            },
        ),
    )
    .await
    {
        Ok(Ok(results)) => results,
        Ok(Err(e)) => return Err(e.into()),
        Err(_) => return Err(anyhow::anyhow!("Port scan timed out after 60s")),
    };

    let total = results.ports_scanned as u64;
    send_progress(&progress_tx, total.max(1), total_ports.max(1)).await;
    Ok(TaskResult::PortScan(results))
}

pub async fn run_endpoint_scan(
    target: String,
    concurrency: usize,
    timeout: std::time::Duration,
    wordlist: Option<String>,
    include_404: bool,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
) -> anyhow::Result<TaskResult> {
    use crate::scanner::endpoints::{scan_endpoints, EndpointScanConfig, DEFAULT_ENDPOINTS};

    send_progress(&progress_tx, 0, 100).await;

    let endpoints: Vec<String> = if let Some(ref wl) = wordlist {
        crate::scanner::wordlist::Wordlist::from_file(wl)
            .await?
            .into_endpoints()
    } else {
        DEFAULT_ENDPOINTS.iter().map(|s| s.to_string()).collect()
    };
    let total_endpoints = endpoints.len() as u64;

    let results = match tokio::time::timeout(
        std::time::Duration::from_secs(60),
        scan_endpoints(EndpointScanConfig {
            base_url: target,
            endpoints,
            concurrency,
            timeout_duration: timeout,
            include_404,
            tui_mode: true,
            spoof_config: std::sync::Arc::new(SpoofConfig::default()),
            verify_tls: true,
            progress_tx: Some(progress_tx.clone()),
            max_results: None,
        }),
    )
    .await
    {
        Ok(Ok(results)) => results,
        Ok(Err(e)) => return Err(e.into()),
        Err(_) => return Err(anyhow::anyhow!("Endpoint scan timed out after 60s")),
    };

    let total = results.endpoints_scanned as u64;
    send_progress(&progress_tx, total.max(1), total_endpoints.max(1)).await;
    Ok(TaskResult::EndpointScan(results))
}

pub async fn run_fingerprint(
    target: String,
    ports: String,
    timeout: std::time::Duration,
    concurrency: usize,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
) -> anyhow::Result<TaskResult> {
    use crate::scanner::fingerprint::fingerprint_services;

    send_progress(&progress_tx, 0, 100).await;

    let port_list = crate::utils::parsing::parse_ports(&ports)?;
    let total_ports = port_list.len() as u64;

    let results = match tokio::time::timeout(
        std::time::Duration::from_secs(60),
        fingerprint_services(
            &target,
            port_list,
            timeout,
            true,
            concurrency,
            Some(progress_tx.clone()),
            None,
        ),
    )
    .await
    {
        Ok(Ok(results)) => results,
        Ok(Err(e)) => return Err(e.into()),
        Err(_) => return Err(anyhow::anyhow!("Fingerprint timed out after 60s")),
    };

    let total = results.ports_scanned as u64;
    send_progress(&progress_tx, total.max(1), total_ports.max(1)).await;
    Ok(TaskResult::Fingerprint(results))
}

/// UDP range scan, mapped onto the existing [`PortScanResults`] envelope.
///
/// The mapping is deliberately lossy in the *safe* direction: the crate's typed
/// evidence stays in `eggsec-udp-scan`, and this boundary projects it into the
/// string `status` the existing writers already understand. `PortResult` gains
/// no protocol field in this slice -- that refactor reaches the report model,
/// `eggsec-output` and the Python bindings under guards 118-120, and coupling
/// it to the scanner would make the risky wide change depend on the small
/// safe one.
#[cfg(feature = "udp-scan")]
async fn scan_ports_udp(
    target: String,
    ports: String,
    concurrency: usize,
    timeout: std::time::Duration,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
) -> anyhow::Result<TaskResult> {
    use crate::scanner::ports::{PortResult, PortScanResults, UdpEvidenceSummary, UdpHostState};

    let port_list = crate::utils::parsing::parse_ports(&ports)?;
    let request = eggsec_udp_scan::UdpScanRequest {
        target: target
            .parse()
            .map_err(|_| anyhow::anyhow!("UDP scan requires an IPv4 target, got {target:?}"))?,
        start_port: port_list.first().copied().unwrap_or(1),
        end_port: port_list.last().copied().unwrap_or(1),
        timeout,
        concurrency,
        sweeps: 1,
    };
    request
        .validate()
        .map_err(|e| anyhow::anyhow!("invalid UDP scan request: {e}"))?;

    // A missing ICMP receiver is a *reported* condition, not a silent empty
    // result: the crate never acquires privilege, so an operator without it
    // gets an explanation rather than a scan that found nothing.
    let receiver = eggsec_udp_scan::IcmpReceiver::open_unprivileged()
        .or_else(|_| eggsec_udp_scan::IcmpReceiver::open())
        .map_err(|e| {
            anyhow::anyhow!(
                "UDP scanning needs an ICMP error receiver, which is unavailable here: {e}"
            )
        })?;

    // This is a blocking call: the crate enforces its own internal deadline, and
    // the outer timeout bounds the join as well. A cancelled join would not
    // stop the thread inside it, so the bound is not advisory.
    let results = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        tokio::task::spawn_blocking(move || {
            eggsec_udp_scan::scan_with_receiver(&request, &receiver)
        }),
    )
    .await
    .map_err(|_| anyhow::anyhow!("UDP scan timed out after 120s"))?
    .map_err(|e| anyhow::anyhow!("UDP scan worker failed: {e}"))?
    .map_err(|e| anyhow::anyhow!("UDP scan failed: {e}"))?;

    // `open_ports` holds every port with a verdict, not just open ones: a UDP
    // result is mostly `open|filtered`, and dropping those would leave the
    // caller with an empty list that looks like "nothing was listening".
    let open_ports = results
        .ports
        .iter()
        .map(|v| PortResult {
            port: v.port,
            status: match v.state {
                eggsec_udp_scan::UdpPortState::Closed => "closed".to_string(),
                eggsec_udp_scan::UdpPortState::Filtered => "filtered".to_string(),
                eggsec_udp_scan::UdpPortState::OpenFiltered => "open|filtered".to_string(),
                eggsec_udp_scan::UdpPortState::Open => "open".to_string(),
            },
            service: String::new(),
        })
        .collect::<Vec<_>>();

    let total_open = open_ports.len();
    send_progress(&progress_tx, 100, 100).await;

    Ok(TaskResult::PortScan(PortScanResults {
        host: target,
        ports_scanned: results.ports_scanned,
        open_ports,
        total_open_ports: total_open,
        results_truncated: results.truncated,
        duration_ms: results.duration.as_millis() as u64,
        spoof_stats: None,
        udp_host_state: Some(match results.host_state {
            eggsec_udp_scan::HostState::Up => UdpHostState::Up,
            eggsec_udp_scan::HostState::Unresponsive => UdpHostState::Unresponsive,
            eggsec_udp_scan::HostState::Indeterminate => UdpHostState::Indeterminate,
        }),
        udp_evidence: Some(UdpEvidenceSummary {
            correlated: results.evidence.correlated,
            orphans: results.evidence.orphans,
            expired: results.evidence.expired,
            unparseable: results.evidence.unparseable,
            sweeps: results.evidence.sweeps,
        }),
    }))
}
