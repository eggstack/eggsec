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
//! use eggsec::pipeline::{Pipeline, Stage};
//! use eggsec::cli::ScanArgs;
//!
//! # async fn example() -> eggsec::error::Result<()> {
//! let args = ScanArgs {
//!     target: "example.com".to_string(),
//!     stages: Some("port,fingerprint,endpoint,fuzz".to_string()),
//!     concurrency: Some(20),
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

use crate::error::{EggsecError, Result};
use crate::output::extensions::{JUnitBuilderExt, SarifBuilderExt};

#[cfg(feature = "cli")]
use crate::cli::ResumeArgs;
#[cfg(feature = "cli")]
use crate::cli::ScanArgs;
use crate::config::EggsecConfig;
use crate::types::OutputFormat;
use crate::utils::sanitize_for_logging;

pub use context::PipelineContext;
pub use executor::Pipeline;
pub use report::PipelineReport;
pub use stage::{parse_stages, Stage};

async fn write_output(
    report: &PipelineReport,
    output_path: &str,
    format: Option<OutputFormat>,
) -> Result<()> {
    match format {
        Some(OutputFormat::Html) | None => {
            let html = report::generate_html(report)?;
            tokio::fs::write(output_path, html).await?;
        }
        Some(OutputFormat::Pretty) => {
            let json = serde_json::to_string_pretty(report)?;
            tokio::fs::write(output_path, json).await?;
        }
        Some(OutputFormat::Compact) => {
            let json = serde_json::to_string(report)?;
            tokio::fs::write(output_path, json).await?;
        }
        Some(OutputFormat::Markdown) => {
            let md = report::generate_markdown(report)?;
            tokio::fs::write(output_path, md).await?;
        }
        Some(OutputFormat::Json) => {
            let json = serde_json::to_string_pretty(report)?;
            tokio::fs::write(output_path, json).await?;
        }
        Some(OutputFormat::Csv) => {
            let csv = report::generate_csv(report)?;
            tokio::fs::write(output_path, csv).await?;
        }
        Some(OutputFormat::Sarif) => {
            let sarif = crate::output::SarifBuilder::new()
                .with_report(report)
                .build();
            tokio::fs::write(output_path, serde_json::to_string_pretty(&sarif)?).await?;
        }
        Some(OutputFormat::Junit) => {
            let junit = crate::output::JUnitBuilder::new("eggsec")
                .with_report(report)
                .build();
            tokio::fs::write(output_path, junit.to_xml()?).await?;
        }
    }

    if let Some(ref manifest) = report.manifest {
        let base = std::path::Path::new(output_path)
            .with_extension("")
            .to_string_lossy()
            .to_string();
        let manifest_path = format!("{}.manifest.json", base);
        let manifest_json = serde_json::to_string_pretty(manifest)?;
        tokio::fs::write(&manifest_path, manifest_json).await?;
    }

    Ok(())
}

/// Run security assessment pipeline from CLI
///
/// # Arguments
///
/// * `args` - Pipeline arguments from CLI
/// * `config` - Eggsec configuration
///
/// # Errors
///
/// Returns error if:
/// - Target is invalid
/// - Any stage fails to execute
/// - Output file cannot be written
#[cfg(feature = "tool-api")]
pub async fn run_with_callback<F>(target: &str, config: &EggsecConfig, callback: F) -> Result<()>
where
    F: FnMut(crate::tool::response::Finding) + Send + 'static,
{
    run_with_callback_for_profile(target, crate::types::ScanProfile::Quick, config, callback).await
}

/// Run security assessment pipeline for a specific profile with a finding callback.
///
/// This is the profile-aware entry point for the tool-API. It constructs the
/// pipeline through the canonical [`Pipeline::from_profile`] path so that the
/// requested profile's stages, risk budget, and validation are all honoured.
#[cfg(feature = "tool-api")]
pub async fn run_with_callback_for_profile<F>(
    target: &str,
    profile: crate::types::ScanProfile,
    config: &EggsecConfig,
    mut callback: F,
) -> Result<()>
where
    F: FnMut(crate::tool::response::Finding) + Send + 'static,
{
    let pipeline = Pipeline::from_profile(target, profile).with_config(config.clone());
    let report = pipeline.run().await?;

    for port in &report.open_ports {
        callback(port.clone().into());
    }
    for service in &report.services {
        callback(service.clone().into());
    }
    for endpoint in &report.endpoints {
        callback(endpoint.clone().into());
    }

    if let Some(failed_stage) = report.first_failed_stage() {
        return Err(EggsecError::ScanFailed {
            stage: failed_stage.stage.to_string(),
            error: failed_stage
                .error
                .clone()
                .unwrap_or_else(|| "unknown pipeline stage failure".to_string()),
        });
    }

    Ok(())
}

#[cfg(all(feature = "tool-api", feature = "cli"))]
pub async fn run_cli_with_callback<F>(
    args: ScanArgs,
    config: &EggsecConfig,
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
        return Err(EggsecError::ScanFailed {
            stage: failed_stage.stage.to_string(),
            error: failed_stage
                .error
                .clone()
                .unwrap_or_else(|| "unknown pipeline stage failure".to_string()),
        });
    }

    Ok(())
}

#[cfg(feature = "cli")]
pub async fn run_cli(args: ScanArgs, config: &EggsecConfig) -> Result<()> {
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
        return Err(EggsecError::ScanFailed {
            stage: failed_stage.stage.to_string(),
            error: failed_stage
                .error
                .clone()
                .unwrap_or_else(|| "unknown pipeline stage failure".to_string()),
        });
    }

    Ok(())
}

#[cfg(feature = "cli")]
pub async fn resume_cli(args: ResumeArgs, config: &EggsecConfig) -> Result<()> {
    let session = session::load(&args.session).await?;
    let pipeline = Pipeline::from_session(session).with_config(config.clone());
    let report = pipeline.run().await?;

    println!("{}", report);

    if let Some(failed_stage) = report.first_failed_stage() {
        return Err(EggsecError::ScanFailed {
            stage: failed_stage.stage.to_string(),
            error: failed_stage
                .error
                .clone()
                .unwrap_or_else(|| "unknown pipeline stage failure".to_string()),
        });
    }

    Ok(())
}
