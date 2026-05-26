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

    if let Err(e) = progress_tx.send((0, 100)).await {
        tracing::warn!("Failed to send initial progress: {}", e);
    }

    let port_list = crate::utils::parsing::parse_ports(&ports)?;
    let total_ports = port_list.len() as u64;

    if let Err(e) = progress_tx.send((10, 100)).await {
        tracing::warn!("Failed to send progress: {}", e);
    }

    let results = scan_ports(
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
    )
    .await?;

    let total = results.ports_scanned as u64;
    if let Err(e) = result_tx.send(TaskResult::PortScan(results)).await {
        tracing::warn!("Failed to send port scan result: {}", e);
    }
    if let Err(e) = progress_tx.send((total.max(1), total_ports.max(1))).await {
        tracing::warn!("Failed to send progress: {}", e);
    }
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
    use crate::scanner::endpoints::{scan_endpoints, EndpointScanConfig, DEFAULT_ENDPOINTS};

    if let Err(e) = progress_tx.send((0, 100)).await {
        tracing::warn!("Failed to send initial progress: {}", e);
    }

    let endpoints: Vec<String> = if let Some(ref wl) = wordlist {
        tokio::fs::read_to_string(wl)
            .await?
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        DEFAULT_ENDPOINTS.iter().map(|s| s.to_string()).collect()
    };
    let total_endpoints = endpoints.len() as u64;

    let results = scan_endpoints(EndpointScanConfig {
        base_url: target,
        endpoints,
        concurrency,
        timeout_duration: timeout,
        include_404: false,
        tui_mode: true,
        spoof_config: SpoofConfig::default(),
        verify_tls: true,
        progress_tx: Some(progress_tx.clone()),
        max_results: None,
    })
    .await?;

    let total = results.endpoints_scanned as u64;
    if let Err(e) = result_tx.send(TaskResult::EndpointScan(results)).await {
        tracing::warn!("Failed to send endpoint scan result: {}", e);
    }
    if let Err(e) = progress_tx
        .send((total.max(1), total_endpoints.max(1)))
        .await
    {
        tracing::warn!("Failed to send progress: {}", e);
    }
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

    if let Err(e) = progress_tx.send((0, 100)).await {
        tracing::warn!("Failed to send initial progress: {}", e);
    }

    let port_list = crate::utils::parsing::parse_ports(&ports)?;
    let total_ports = port_list.len() as u64;

    let results = fingerprint_services(
        &target,
        port_list,
        timeout,
        true,
        20,
        Some(progress_tx.clone()),
        None,
    )
    .await?;

    let total = results.ports_scanned as u64;
    if let Err(e) = result_tx.send(TaskResult::Fingerprint(results)).await {
        tracing::warn!("Failed to send fingerprint result: {}", e);
    }
    if let Err(e) = progress_tx.send((total.max(1), total_ports.max(1))).await {
        tracing::warn!("Failed to send progress: {}", e);
    }
    Ok(())
}
