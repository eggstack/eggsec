use crate::tabs::recon::ReconOptions;
use crate::workers::TaskResult;
use eggsec::cli::ScanProfile;

#[allow(unused_variables)]
pub async fn run_pipeline(
    target: String,
    profile: ScanProfile,
    output_file: String,
    output_format: String,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use eggsec::cli::{CommonHttpArgs, ScanArgs};
    use eggsec::pipeline::Pipeline;

    let args = ScanArgs {
        target: target.clone(),
        profile,
        stages: None,
        concurrency: None,
        concurrent_stages: false,
        json: false,
        output: if output_file.is_empty() {
            None
        } else {
            Some(output_file)
        },
        format: output_format.parse::<eggsec::cli::OutputFormat>().ok(),
        web_types: None,
        common: CommonHttpArgs::default(),
        source_ip: None,
        spoof_range: None,
        decoy: None,
        decoy_range: None,
        decoy_count: None,
        decoy_mode: None,
        include_me: false,
        source_port: None,
        random_source_port: false,
        fragment: false,
        scan_type: None,
        packet_trace: None,
        max_rate: None,
        ttl: None,
        verbose: false,
    };

    let pipeline = Pipeline::from_args_with_tui_mode(args, None, true);
    let stages_count = pipeline.get_stages().len() as u64;

    if let Err(e) = progress_tx.send((0, stages_count.max(1))).await {
        tracing::warn!("Failed to send progress: {}", e);
    }

    let report =
        match tokio::time::timeout(std::time::Duration::from_secs(300), pipeline.run()).await {
            Ok(Ok(report)) => report,
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => return Err(anyhow::anyhow!("Pipeline timed out after 300s")),
        };

    if let Err(e) = result_tx.send(TaskResult::Pipeline(report)).await {
        tracing::warn!("Failed to send pipeline result: {}", e);
    }
    if let Err(e) = progress_tx.send((stages_count, stages_count.max(1))).await {
        tracing::warn!("Failed to send progress: {}", e);
    }
    Ok(())
}

pub async fn run_recon(
    target: String,
    concurrency: usize,
    options: ReconOptions,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use eggsec::cli::ReconArgs;
    use eggsec::config::EggsecConfig;
    use eggsec::recon::run_full_recon;

    if let Err(e) = progress_tx.send((0, 100)).await {
        tracing::warn!("Failed to send initial progress: {}", e);
    }

    let args = ReconArgs {
        target: target.clone(),
        concurrency: Some(concurrency),
        no_tech: options.no_tech,
        no_dns: options.no_dns,
        no_geo: options.no_geo,
        no_whois: options.no_whois,
        no_subdomains: options.no_subdomains,
        no_ssl: options.no_ssl,
        no_dns_records: options.no_dns_records,
        no_js: true,
        no_content: options.no_content,
        no_cloud: options.no_cloud,
        no_wayback: options.no_wayback,
        no_cors: options.no_cors,
        no_threat: options.no_threat,
        no_cve: options.no_cve,
        no_email: options.no_email,
        no_takeover: options.no_takeover,
        json: false,
        quiet: false,
        verbose: false,
        output: None,
    };

    let config = EggsecConfig::default();

    if let Err(e) = progress_tx.send((5, 100)).await {
        tracing::warn!("Failed to send progress: {}", e);
    }

    let max_retries = eggsec_core::constants::DEFAULT_MAX_RETRIES;
    let base_delay_secs = 2u64;

    for attempt in 1..=max_retries {
        let stage = std::sync::Arc::new(parking_lot::Mutex::new(String::new()));
        let (stage_tx, mut stage_rx) = tokio::sync::watch::channel("initial".to_string());
        let ptx = progress_tx.clone();
        let progress_tx_for_timeout = progress_tx.clone();

        let progress_handle = tokio::spawn(async move {
            let result = tokio::time::timeout(std::time::Duration::from_secs(300), async move {
                let mut last_stage = stage_rx.borrow().clone();
                let stages = ["resolving", "recon (parallel)", "takeover", "cve", "done"];
                let total_stages = stages.len() as u64;
                let mut stalled_count = 0u32;
                while stage_rx.changed().await.is_ok() {
                    let current = stage_rx.borrow().clone();
                    if current != last_stage {
                        last_stage.clone_from(&current);
                        stalled_count = 0;
                        let completed = stages
                            .iter()
                            .take_while(|&&s| {
                                current.contains(s) || (s == "done" && current.is_empty())
                            })
                            .count() as u64;
                        let total_stages = total_stages.max(1);
                        let pct = (completed.min(total_stages) * 90) / total_stages + 5;
                        if let Err(e) = ptx.send((pct, 100)).await {
                            tracing::warn!("Failed to send progress: {}", e);
                        }
                    } else {
                        stalled_count += 1;
                        if stalled_count > 200 {
                            if let Err(e) = ptx.send((95, 100)).await {
                                tracing::warn!("Failed to send stalled progress: {}", e);
                            }
                            break;
                        }
                    }
                    if current.is_empty() && !last_stage.is_empty() {
                        break;
                    }
                }
            })
            .await;
            if result.is_err() {
                tracing::warn!("Progress monitor task timed out after 300s");
                if let Err(e) = progress_tx_for_timeout.send((95, 100)).await {
                    tracing::warn!("Failed to send timeout progress: {}", e);
                }
            }
        });

        let watch_sender = stage_tx.clone();
        let stage_for_thread = stage.clone();
        let start_time = std::time::Instant::now();
        // NOTE: This OS thread is intentionally not joined. It polls the shared stage
        // Arc<Mutex<String>> and sends updates via a watch channel. On timeout (120s)
        // it exits naturally. On retry, a new stage clone is created, orphaning the old
        // thread — acceptable for short-lived polling that self-terminates.
        std::thread::spawn(move || {
            let mut last = String::new();
            loop {
                std::thread::sleep(std::time::Duration::from_millis(50));
                let current = stage_for_thread.lock().clone();
                if current != last {
                    last = current.clone();
                    if let Err(e) = watch_sender.send(current) {
                        tracing::warn!("Failed to send stage update: {}", e);
                    }
                }
                if start_time.elapsed().as_secs() > 120 {
                    if let Err(e) = watch_sender.send("timeout".to_string()) {
                        tracing::warn!("Failed to send timeout signal: {}", e);
                    }
                    break;
                }
            }
        });

        let timeout_duration = std::time::Duration::from_secs(120);
        let recon_result = tokio::time::timeout(
            timeout_duration,
            run_full_recon(&args, &config, stage, false),
        )
        .await;

        match recon_result {
            Ok(Ok(r)) => {
                progress_handle.abort();
                if let Err(e) = progress_tx.send((100, 100)).await {
                    tracing::warn!("Failed to send progress: {}", e);
                }
                if let Err(e) = result_tx.send(TaskResult::Recon(r)).await {
                    tracing::warn!("Failed to send recon result: {}", e);
                }
                if let Err(e) = progress_handle.await {
                    if e.is_panic() {
                        tracing::warn!("Progress tracking task panicked: {:?}", e);
                    }
                }
                return Ok(());
            }
            Ok(Err(e)) => {
                progress_handle.abort();
                let error_str = e.to_string().to_lowercase();

                let is_retryable = error_str.contains("timeout")
                    || error_str.contains("connection")
                    || error_str.contains("temporary")
                    || error_str.contains("reset")
                    || error_str.contains("broken pipe");

                if is_retryable && attempt < max_retries {
                    let delay = base_delay_secs * 2u64.pow(attempt - 1);
                    tracing::warn!(
                        "Recon attempt {} failed, retrying in {} seconds...",
                        attempt,
                        delay
                    );
                    if let Err(e) = progress_tx.send(((attempt as u64) * 20, 100)).await {
                        tracing::warn!("Failed to send retry progress: {}", e);
                    }
                    tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
                } else {
                    tracing::error!("Recon failed after {} attempts: {:?}", max_retries, e);
                    if let Err(e) = progress_handle.await {
                        if e.is_panic() {
                            tracing::warn!("Progress tracking task panicked: {:?}", e);
                        }
                    }
                    return Err(e.into());
                }
            }
            Err(_) => {
                progress_handle.abort();
                tracing::error!("Recon timed out after 120 seconds");
                if let Err(e) = progress_handle.await {
                    if e.is_panic() {
                        tracing::warn!("Progress tracking task panicked: {:?}", e);
                    }
                }
                return Err(anyhow::anyhow!("Recon timed out after 120 seconds"));
            }
        }
    }

    Err(anyhow::anyhow!("Recon failed after max retries"))
}
