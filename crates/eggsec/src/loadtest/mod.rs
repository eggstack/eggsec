//! HTTP load testing module (Phase D: transport-neutral core).
//!
//! Provides load testing capabilities for measuring server performance
//! and gathering latency metrics.
//!
//! ## Key Components
//!
//! - [`LoadTestPlan`](plan::LoadTestPlan) - transport-neutral plan (no Clap/config/Reqwest/indicatif)
//! - [`LoadTestExecutor`](executor::LoadTestExecutor) - generic executor over `HttpTransport`
//! - [`LoadTestResults`] - Aggregated test results with percentiles
//! - [`RequestTemplate`](adapter::RequestTemplate) - engine adaptation above the core
//!
//! ## Usage
//!
//! ```rust,no_run
//! use eggsec::loadtest::LoadTestRunner;
//! use std::time::Duration;
//!
//! # async fn example() -> eggsec::error::Result<()> {
//! let runner = LoadTestRunner::new(
//!     "https://example.com".to_string(),
//!     1000,  // total requests
//!     50,    // concurrency
//!     Duration::from_secs(30),
//! )?;
//!
//! let results = runner.run().await?;
//! println!("Average latency: {:.2}ms", results.latency_mean_ms);
//! println!("P95 latency: {:.2}ms", results.latency_p95_ms);
//! # Ok(())
//! # }
//! ```
//!
//! ## Metrics Collected
//!
//! - Total/successful/failed requests
//! - Requests per second
//! - Latency percentiles (p50, p90, p95, p99)
//! - Status code distribution
//! - Transport-error categories (`error_kinds`)
//! - Error messages

pub mod adapter;
pub mod backend;
pub mod executor;
pub mod metrics;
pub mod plan;
pub mod progress;
pub mod runner;

#[cfg(feature = "cli")]
use crate::error::Result;

pub use adapter::RequestTemplate;
pub use backend::{OwnedScopeAuthority, ReqwestTransport};
pub use executor::LoadTestExecutor;
pub use metrics::{LoadTestErrorKind, LoadTestResults};
pub use plan::{LoadTestPlan, RatePolicy};
pub use progress::{ChannelSink, FnSink, LoadTestEvent, LoadTestProgress, NoopSink, ProgressSink};
pub use runner::{LoadTestRunConfig, LoadTestRunner};

/// Run load test from CLI
///
/// # Arguments
///
/// * `args` - Load test arguments from CLI
/// * `config` - Eggsec configuration
///
/// # Errors
///
/// Returns error if:
/// - URL is invalid
/// - HTTP client construction fails
/// - Network connectivity issues occur
/// - Output file cannot be written
#[cfg(feature = "cli")]
pub async fn run_cli(
    args: crate::cli::LoadArgs,
    config: &crate::config::EggsecConfig,
) -> Result<()> {
    run_cli_with_scope(args, config, crate::config::Scope::new()).await
}

/// Run load test from CLI with an explicit scope.
///
/// The scope authorizes every request through the transport seam
/// (per-hop checkpoints, re-authorized redirects). Callers that already
/// enforce operation policy (e.g. `handle_load` with `ctx.scope`) pass that
/// scope here so per-request authorization matches the pre-dispatch verdict.
#[cfg(feature = "cli")]
pub async fn run_cli_with_scope(
    args: crate::cli::LoadArgs,
    config: &crate::config::EggsecConfig,
    scope: crate::config::Scope,
) -> Result<()> {
    use std::sync::Arc;
    use tokio_util::sync::CancellationToken;

    let verbose = args.verbose;
    let quiet = args.quiet;
    let json = args.json;
    let output_file = args.output.clone();
    let url = args.url.clone();
    let concurrency = args.concurrency;

    if verbose && !quiet {
        eprintln!(
            "Starting load test against {} with {} concurrent connections",
            url, concurrency
        );
    }

    let run_config: LoadTestRunConfig = args.into();
    let mut runner = LoadTestRunner::from_config_with_engine(run_config, config)?;
    runner.set_scope(scope);

    let (plan, template) = runner.plan()?;
    let transport = Arc::new(ReqwestTransport::with_system_resolver());
    let authority: Arc<dyn eggsec_transport::NetworkAuthority> =
        Arc::new(OwnedScopeAuthority::new(runner.scope().clone()));
    let executor = LoadTestExecutor::new(
        plan.clone(),
        template,
        transport,
        authority,
        CancellationToken::new(),
    );

    // Process-host presentation: indicatif renderer over structured events.
    // The core never prints; this sink owns the only progress widget.
    let total = plan.total_requests;
    let progress_bar = if quiet {
        None
    } else {
        let pb = indicatif::ProgressBar::new(total);
        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})",
                )
                .unwrap_or_else(|_| indicatif::ProgressStyle::default_bar())
                .progress_chars("#>-"),
        );
        Some(pb)
    };
    let sink = FnSink(|event| {
        let _ = event;
    });
    // Drive progress manually: poll via a channel sink when a bar exists,
    // otherwise run silently. The executor emits per-request events; the
    // bar increments here, in CLI code — never in the core.
    let results = if let Some(pb) = progress_bar.clone() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<LoadTestProgress>(1024);
        let channel_sink = ChannelSink::new(tx);
        let run_fut = executor.run(&channel_sink);
        tokio::pin!(run_fut);
        loop {
            tokio::select! {
                r = &mut run_fut => break r.map_err(crate::error::EggsecError::Validation)?,
                update = rx.recv() => {
                    if let Some(p) = update {
                        pb.set_position(p.completed.min(total));
                    }
                }
            }
        }
    } else {
        executor
            .run(&sink)
            .await
            .map_err(crate::error::EggsecError::Validation)?
    };
    if let Some(pb) = progress_bar {
        pb.finish_and_clear();
    }

    let output_str = if json {
        serde_json::to_string_pretty(&results)?
    } else {
        format!("\n{}", results)
    };

    if let Some(ref path) = output_file {
        tokio::fs::write(path, &output_str).await?;
        if verbose && !quiet {
            eprintln!("Results written to {}", path);
        }
    } else if !quiet {
        println!("{}", output_str);
    }

    if verbose && !quiet {
        eprintln!(
            "Load test complete: {} requests, {} errors",
            results.total_requests, results.failed_requests
        );
    }

    Ok(())
}
