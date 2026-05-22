//! Security assessment pipeline module
//!
//! Orchestrates multiple security scanning stages in sequence for
//! comprehensive target assessment.
//!
//! ## Key Components
//!
//! - [`Pipeline`] - Main pipeline executor
//! - [`Stage`] - Individual scanning stages (PortScan, Fingerprint, Fuzz, etc.)
//! - [`PipelineContext`] - Shared context between pipeline stages
//! - [`PipelineReport`] - Aggregated results from all stages
//!
//! ## Usage
//!
//! ```rust,compile_fail
//! use slapper::pipeline::{Pipeline, Stage};
//! use slapper::cli::ScanArgs;
//!
//! # async fn example() -> slapper::error::Result<()> {
//! let args = ScanArgs {
//!     target: "example.com".to_string(),
//!     stages: Some("port,fingerprint,endpoint,fuzz".to_string()),
//!     concurrency: 20,
//!     ..Default::default()
//! };
//!
//! let pipeline = Pipeline::from_args(args);
//! let report = pipeline.run().await?;
//!
//! println!("Completed {} stages", report.stage_results.len());
//! # Ok(())
//! # }
//! ```
//!
//! ## Available Stages
//!
//! - `PortScan` - TCP port scanning
//! - `Fingerprint` - Service identification
//! - `EndpointScan` - HTTP endpoint discovery
//! - `Fuzz` - Security payload fuzzing
//! - `LoadTest` - HTTP load testing
//! - `Waf` - WAF detection and bypass
//! - `Recon` - Reconnaissance gathering

pub mod context;
pub mod executor;
pub mod report;
pub mod session;
pub mod stage;

use crate::error::{Result, SlapperError};

use crate::cli::ResumeArgs;
use crate::cli::ScanArgs;
use crate::config::SlapperConfig;
use crate::utils::sanitize_for_logging;

pub use context::PipelineContext;
pub use executor::Pipeline;
pub use report::PipelineReport;
pub use stage::{parse_stages, Stage};

async fn write_output(report: &PipelineReport, output_path: &str, format: Option<crate::cli::OutputFormat>) -> Result<()> {
    match format {
        Some(crate::cli::OutputFormat::Html)
        | Some(crate::cli::OutputFormat::Pretty)
        | Some(crate::cli::OutputFormat::Compact)
        | Some(crate::cli::OutputFormat::Markdown)
        | None => {
            let html = report::generate_html(report)?;
            tokio::fs::write(output_path, html).await?;
        }
        Some(crate::cli::OutputFormat::Json) => {
            let json = serde_json::to_string_pretty(report)?;
            tokio::fs::write(output_path, json).await?;
        }
        Some(crate::cli::OutputFormat::Csv) => {
            let csv = report::generate_csv(report)?;
            tokio::fs::write(output_path, csv).await?;
        }
        Some(crate::cli::OutputFormat::Sarif) => {
            let sarif = crate::output::SarifBuilder::new()
                .with_report(report)
                .build();
            tokio::fs::write(output_path, serde_json::to_string_pretty(&sarif)?).await?;
        }
        Some(crate::cli::OutputFormat::Junit) => {
            let junit = crate::output::JUnitBuilder::new("slapper")
                .with_report(report)
                .build();
            tokio::fs::write(output_path, junit.to_xml()?).await?;
        }
    }
    Ok(())
}

/// Run security assessment pipeline from CLI
///
/// # Arguments
///
/// * `args` - Pipeline arguments from CLI
/// * `config` - Slapper configuration
///
/// # Errors
///
/// Returns error if:
/// - Target is invalid
/// - Any stage fails to execute
/// - Output file cannot be written
#[cfg(feature = "tool-api")]
pub async fn run_cli_with_callback<F>(
    args: ScanArgs,
    config: &SlapperConfig,
    mut callback: F,
) -> Result<()>
where
    F: FnMut(crate::tool::response::Finding) + Send + 'static,
{
    if args.verbose {
        eprintln!(
            "Starting pipeline scan on {}",
            sanitize_for_logging(&args.target)
        );
    }

    let pipeline = Pipeline::from_args_with_config(args.clone(), config);
    let report = pipeline.run().await?;

    if args.verbose {
        eprintln!(
            "Pipeline complete: {} stages run",
            report.stage_results.len()
        );
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", report);
    }

    for port in &report.open_ports {
        callback(port.clone().into());
    }
    for service in &report.services {
        callback(service.clone().into());
    }
    for endpoint in &report.endpoints {
        callback(endpoint.clone().into());
    }

    if let Some(ref output_path) = args.output {
        write_output(&report, output_path, args.format).await?;
        if args.verbose {
            eprintln!("Results written to {}", output_path);
        }
    }

    if let Some(failed_stage) = report.first_failed_stage() {
        return Err(SlapperError::ScanFailed {
            stage: failed_stage.stage.to_string(),
            error: failed_stage
                .error
                .clone()
                .unwrap_or_else(|| "unknown pipeline stage failure".to_string()),
        });
    }

    Ok(())
}

pub async fn run_cli(args: ScanArgs, config: &SlapperConfig) -> Result<()> {
    if args.verbose {
        eprintln!(
            "Starting pipeline scan on {}",
            sanitize_for_logging(&args.target)
        );
    }

    let pipeline = Pipeline::from_args_with_config(args.clone(), config);
    let report = pipeline.run().await?;

    if args.verbose {
        eprintln!(
            "Pipeline complete: {} stages run",
            report.stage_results.len()
        );
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", report);
    }

    if let Some(ref output_path) = args.output {
        write_output(&report, output_path, args.format).await?;
        if args.verbose {
            eprintln!("Results written to {}", output_path);
        }
    }

    if let Some(failed_stage) = report.first_failed_stage() {
        return Err(SlapperError::ScanFailed {
            stage: failed_stage.stage.to_string(),
            error: failed_stage
                .error
                .clone()
                .unwrap_or_else(|| "unknown pipeline stage failure".to_string()),
        });
    }

    Ok(())
}

pub async fn resume_cli(args: ResumeArgs) -> Result<()> {
    let session = session::load(&args.session)?;
    let pipeline = Pipeline::from_session(session);
    let report = pipeline.run().await?;

    println!("{}", report);

    if let Some(failed_stage) = report.first_failed_stage() {
        return Err(SlapperError::ScanFailed {
            stage: failed_stage.stage.to_string(),
            error: failed_stage
                .error
                .clone()
                .unwrap_or_else(|| "unknown pipeline stage failure".to_string()),
        });
    }

    Ok(())
}
