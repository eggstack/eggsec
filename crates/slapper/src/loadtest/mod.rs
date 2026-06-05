//! HTTP load testing module
//!
//! Provides load testing capabilities for measuring server performance
//! and gathering latency metrics.
//!
//! ## Key Components
//!
//! - [`LoadTestRunner`] - Main load test executor
//! - [`LoadTestResults`] - Aggregated test results with percentiles
//!
//! ## Usage
//!
//! ```rust,no_run
//! use slapper::loadtest::LoadTestRunner;
//! use std::time::Duration;
//!
//! # async fn example() -> slapper::error::Result<()> {
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
//! - Error messages

pub mod metrics;
pub mod runner;

use crate::error::Result;

use crate::cli::LoadArgs;
use crate::config::SlapperConfig;

pub use metrics::LoadTestResults;
pub use runner::LoadTestRunner;

/// Run load test from CLI
///
/// # Arguments
///
/// * `args` - Load test arguments from CLI
/// * `config` - Slapper configuration
///
/// # Errors
///
/// Returns error if:
/// - URL is invalid
/// - HTTP client construction fails
/// - Network connectivity issues occur
/// - Output file cannot be written
pub async fn run_cli(args: LoadArgs, config: &SlapperConfig) -> Result<()> {
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

    let runner = LoadTestRunner::from_args_with_config(args, config)?;
    let results = runner.run().await?;

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
