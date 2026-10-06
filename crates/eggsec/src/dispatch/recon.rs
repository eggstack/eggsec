use crate::dispatch::types::{send_progress, ReconOptions, TaskResult};
use crate::types::ScanProfile;

async fn stop_stage_monitor(handle: tokio::task::JoinHandle<()>) {
    handle.abort();
    if let Err(e) = handle.await {
        if e.is_panic() {
            tracing::warn!("Stage monitor task panicked: {:?}", e);
        }
    }
}

/// Where a pipeline report should be written, already resolved and contained.
///
/// The path is validated by the caller *before* the scan starts, so an
/// out-of-bounds destination fails the operation in seconds rather than after
/// a multi-minute assessment.
#[derive(Debug, Clone)]
pub struct PipelineOutput {
    pub path: std::path::PathBuf,
    pub format: crate::types::OutputFormat,
}

pub async fn run_pipeline(
    target: String,
    profile: ScanProfile,
    output: Option<PipelineOutput>,
    session_path: Option<String>,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
) -> anyhow::Result<TaskResult> {
    use crate::pipeline::Pipeline;

    let pipeline = Pipeline::from_profile(&target, profile)
        .with_concurrency(10)
        .with_session_path(session_path);
    let stages_count = pipeline.get_stages().len() as u64;

    send_progress(&progress_tx, 0, stages_count.max(1)).await;

    let report =
        match tokio::time::timeout(std::time::Duration::from_secs(300), pipeline.run()).await {
            Ok(Ok(report)) => report,
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => return Err(anyhow::anyhow!("Pipeline timed out after 300s")),
        };

    if let Some(output) = &output {
        // An explicitly requested artifact that cannot be written is a failed
        // operation, not a silent one: the caller asked for a durable file and
        // must learn that it does not exist.
        crate::pipeline::write_output(&report, &output.path.to_string_lossy(), Some(output.format))
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "pipeline report write to {} failed: {e}",
                    output.path.display()
                )
            })?;
        tracing::info!(path = %output.path.display(), "pipeline report written");
    }

    send_progress(&progress_tx, stages_count, stages_count.max(1)).await;
    Ok(TaskResult::Pipeline(report))
}

/// Resume a saved scan checkpoint and return its report.
///
/// Non-printing by contract: the TUI owns the alternate screen and must never
/// write terminal bytes (guard Check 138), so the report is returned to the
/// caller for the existing `"pipeline"` renderer rather than rendered here.
pub async fn run_pipeline_resume(
    session_path: String,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
) -> anyhow::Result<TaskResult> {
    use crate::config::EggsecConfig;

    send_progress(&progress_tx, 0, 1).await;

    // Fail before the long run if the checkpoint is unreadable, so a bad path
    // costs a filesystem error rather than a partial assessment.
    let remaining = crate::pipeline::session::load(&session_path)
        .await
        .map(|s| s.remaining_stages.len() as u64)
        .map_err(|e| anyhow::anyhow!("could not read the session checkpoint: {e}"))?;

    let config = crate::config::load_config(None::<&str>)
        .inspect_err(|e| {
            tracing::warn!(error = %e, "Failed to load config for session resume, using default");
        })
        .unwrap_or_else(|_| EggsecConfig::default());

    let report = match tokio::time::timeout(
        std::time::Duration::from_secs(300),
        crate::pipeline::resume(&session_path, &config),
    )
    .await
    {
        Ok(Ok(report)) => report,
        Ok(Err(e)) => return Err(e.into()),
        Err(_) => return Err(anyhow::anyhow!("Session resume timed out after 300s")),
    };

    send_progress(&progress_tx, remaining.max(1), remaining.max(1)).await;
    Ok(TaskResult::Pipeline(report))
}

pub async fn run_recon(
    target: String,
    concurrency: usize,
    options: ReconOptions,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
) -> anyhow::Result<TaskResult> {
    use crate::config::EggsecConfig;
    use crate::recon::runner::run_full_recon_from_request;
    use crate::recon::ReconRequest;

    send_progress(&progress_tx, 0, 100).await;

    let request = ReconRequest {
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
    };

    let config = EggsecConfig::default();

    send_progress(&progress_tx, 5, 100).await;

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
                        send_progress(&ptx, pct, 100).await;
                    } else {
                        stalled_count += 1;
                        if stalled_count > 200 {
                            send_progress(&ptx, 95, 100).await;
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
                send_progress(&progress_tx_for_timeout, 95, 100).await;
            }
        });

        let watch_sender = stage_tx.clone();
        let stage_for_monitor = stage.clone();
        let stage_handle = tokio::spawn(async move {
            let mut last = String::new();
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(50));
            let timeout = tokio::time::sleep(std::time::Duration::from_secs(120));
            tokio::pin!(timeout);
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let current = stage_for_monitor.lock().clone();
                        if current != last {
                            last = current.clone();
                            if let Err(e) = watch_sender.send(current) {
                                tracing::warn!("Failed to send stage update: {}", e);
                                break;
                            }
                        }
                    }
                    _ = &mut timeout => break,
                }
            }
        });

        let timeout_duration = std::time::Duration::from_secs(120);
        let recon_result = tokio::time::timeout(
            timeout_duration,
            run_full_recon_from_request(&request, &config, stage, false),
        )
        .await;

        match recon_result {
            Ok(Ok(r)) => {
                stop_stage_monitor(stage_handle).await;
                progress_handle.abort();
                send_progress(&progress_tx, 100, 100).await;
                if let Err(e) = progress_handle.await {
                    if e.is_panic() {
                        tracing::warn!("Progress tracking task panicked: {:?}", e);
                    }
                }
                return Ok(TaskResult::Recon(r));
            }
            Ok(Err(e)) => {
                stop_stage_monitor(stage_handle).await;
                progress_handle.abort();
                let error_str = e.to_string().to_lowercase();

                let is_retryable = error_str.contains("timeout")
                    || error_str.contains("connection")
                    || error_str.contains("temporary")
                    || error_str.contains("reset")
                    || error_str.contains("broken pipe");

                if is_retryable && attempt < max_retries {
                    let delay = base_delay_secs.saturating_pow(attempt - 1).min(3600);
                    tracing::warn!(
                        "Recon attempt {} failed, retrying in {} seconds...",
                        attempt,
                        delay
                    );
                    send_progress(&progress_tx, (attempt as u64) * 20, 100).await;
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
                stop_stage_monitor(stage_handle).await;
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
