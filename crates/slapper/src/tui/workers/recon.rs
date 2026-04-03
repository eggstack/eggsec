use crate::cli::ScanProfile;
use crate::tui::tabs::recon::ReconOptions;
use crate::tui::workers::TaskResult;

#[allow(unused_variables)]
pub async fn run_pipeline(
    target: String,
    profile: ScanProfile,
    output_file: String,
    output_format: String,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::cli::{CommonHttpArgs, ScanArgs};
    use crate::pipeline::Pipeline;

    let args = ScanArgs {
        target: target.clone(),
        profile,
        stages: None,
        concurrency: 10,
        json: false,
        output: if output_file.is_empty() {
            None
        } else {
            Some(output_file)
        },
        format: match output_format.as_str() {
            "html" => Some(crate::cli::OutputFormat::Html),
            "csv" => Some(crate::cli::OutputFormat::Csv),
            "pretty" => Some(crate::cli::OutputFormat::Pretty),
            "compact" => Some(crate::cli::OutputFormat::Compact),
            _ => Some(crate::cli::OutputFormat::Json),
        },
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
    let report = pipeline.run().await?;

    let _ = result_tx.send(TaskResult::Pipeline(report)).await;
    let _ = progress_tx.send((stages_count, stages_count)).await;
    Ok(())
}

pub async fn run_recon(
    target: String,
    concurrency: usize,
    options: ReconOptions,
    progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    result_tx: tokio::sync::mpsc::Sender<TaskResult>,
) -> anyhow::Result<()> {
    use crate::cli::ReconArgs;
    use crate::config::SlapperConfig;
    use crate::recon::run_full_recon;

    let _ = progress_tx.send((0, 100)).await;

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

    let config = SlapperConfig::default();

    let _ = progress_tx.send((0, 100)).await;

    let mut last_error = None;
    let max_retries = 3;
    let base_delay_secs = 2u64;

    for attempt in 1..=max_retries {
        let _ = progress_tx.send((5, 100)).await;

        let stage = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        match run_full_recon(&args, &config, stage, false).await {
            Ok(r) => {
                let _ = progress_tx.send((100, 100)).await;
                let _ = result_tx.send(TaskResult::Recon(r)).await;
                return Ok(());
            }
            Err(e) => {
                let error_str = e.to_string().to_lowercase();
                last_error = Some(e);

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
                    let _ = progress_tx.send(((attempt as u64) * 20, 100)).await;
                    tokio::time::sleep(tokio::time::Duration::from_secs(delay)).await;
                } else {
                    break;
                }
            }
        }
    }

    tracing::error!(
        "Recon failed after {} attempts: {:?}",
        max_retries,
        last_error
    );
    Err(last_error.unwrap_or_else(|| crate::error::SlapperError::Runtime("Unknown error".to_string())).into())
}