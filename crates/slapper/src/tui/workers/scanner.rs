use crate::scanner::spoof::SpoofConfig;
use crate::tui::workers::TaskResult;

pub async fn run_port_scan(
    target: String,
    ports: String,
    concurrency: usize,
    timeout: std::time::Duration,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::scanner::ports::scan_ports;

    let port_list = crate::utils::parsing::parse_ports(&ports)?;
    let results = scan_ports(
        &target,
        port_list,
        concurrency,
        timeout,
        true,
        SpoofConfig::default(),
    )
    .await?;

    let total = results.ports_scanned as u64;
    let _ = result_tx.send(TaskResult::PortScan(results)).await;
    let _ = progress_tx.send((total, total)).await;
    Ok(())
}

pub async fn run_endpoint_scan(
    target: String,
    concurrency: usize,
    timeout: std::time::Duration,
    wordlist: Option<String>,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::scanner::endpoints::{scan_endpoints, DEFAULT_ENDPOINTS};

    let endpoints: Vec<String> = if let Some(ref wl) = wordlist {
        tokio::fs::read_to_string(wl).await?
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        DEFAULT_ENDPOINTS.iter().map(|s| s.to_string()).collect()
    };
    let results = scan_endpoints(
        &target,
        endpoints,
        concurrency,
        timeout,
        false,
        true,
        SpoofConfig::default(),
        true,
    )
    .await?;

    let total = results.endpoints_scanned as u64;
    let _ = result_tx.send(TaskResult::EndpointScan(results)).await;
    let _ = progress_tx.send((total, total)).await;
    Ok(())
}

pub async fn run_fingerprint(
    target: String,
    ports: String,
    timeout: std::time::Duration,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::scanner::fingerprint::fingerprint_services;

    let port_list = crate::utils::parsing::parse_ports(&ports)?;
    let results = fingerprint_services(&target, port_list, timeout, true, 20).await?;

    let total = results.ports_scanned as u64;
    let _ = result_tx.send(TaskResult::Fingerprint(results)).await;
    let _ = progress_tx.send((total, total)).await;
    Ok(())
}