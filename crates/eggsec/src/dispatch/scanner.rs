use crate::dispatch::types::{send_progress, send_result, TaskResult};
use crate::scanner::spoof::SpoofConfig;

pub async fn run_port_scan(
    target: String,
    ports: String,
    concurrency: usize,
    timeout: std::time::Duration,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::scanner::ports::scan_ports;

    send_progress(&progress_tx, 0, 100).await;

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
    send_result(&result_tx, TaskResult::PortScan(results)).await;
    send_progress(&progress_tx, total.max(1), total_ports.max(1)).await;
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
            include_404: false,
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
    send_result(&result_tx, TaskResult::EndpointScan(results)).await;
    send_progress(&progress_tx, total.max(1), total_endpoints.max(1)).await;
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
            20,
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
    send_result(&result_tx, TaskResult::Fingerprint(results)).await;
    send_progress(&progress_tx, total.max(1), total_ports.max(1)).await;
    Ok(())
}
