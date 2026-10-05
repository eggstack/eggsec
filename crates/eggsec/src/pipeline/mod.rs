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

// Unconditional: `write_output` is no longer feature-gated, so the module's
// `Result` alias is needed on every build.
use crate::error::Result;
use crate::output::extensions::{JUnitBuilderExt, SarifBuilderExt};

#[cfg(feature = "cli")]
use crate::cli::ResumeArgs;
#[cfg(feature = "cli")]
use crate::cli::ScanArgs;
#[cfg(any(feature = "tool-api", feature = "cli"))]
use crate::config::EggsecConfig;
use crate::types::OutputFormat;
#[cfg(feature = "cli")]
use crate::utils::sanitize_for_logging;

pub use context::PipelineContext;
pub use executor::Pipeline;
pub use report::PipelineReport;
pub use stage::{parse_stages, Stage};

/// Render `report` into `output_path` in the requested format, plus a sibling
/// `<base>.manifest.json` when the report carries a manifest.
///
/// Not CLI-gated: the canonical pipeline path writes report output too, and
/// every writer below is format-dispatched off the engine's own
/// [`OutputFormat`], so the writers must exist wherever the pipeline runs.
pub async fn write_output(
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
        return Err(crate::error::EggsecError::ScanFailed {
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
        return Err(crate::error::EggsecError::ScanFailed {
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
        return Err(crate::error::EggsecError::ScanFailed {
            stage: failed_stage.stage.to_string(),
            error: failed_stage
                .error
                .clone()
                .unwrap_or_else(|| "unknown pipeline stage failure".to_string()),
        });
    }

    Ok(())
}

/// Resume a saved scan and return its report.
///
/// This is the non-printing half of `resume_cli`. Surfaces that render a
/// session to the user themselves (notably the TUI, which owns the alternate
/// screen and must never write terminal bytes) call this directly; only the CLI
/// wrapper renders the returned report.
pub async fn resume(path: &str, config: &crate::config::EggsecConfig) -> Result<PipelineReport> {
    let session = session::load(path).await?;
    let pipeline = Pipeline::from_session(session).with_config(config.clone());
    let report = pipeline.run().await?;

    if let Some(failed_stage) = report.first_failed_stage() {
        return Err(crate::error::EggsecError::ScanFailed {
            stage: failed_stage.stage.to_string(),
            error: failed_stage
                .error
                .clone()
                .unwrap_or_else(|| "unknown pipeline stage failure".to_string()),
        });
    }

    Ok(report)
}

/// Read the target stored in a checkpoint without running it.
///
/// The TUI needs the target before dispatch so it can build the enforcement
/// descriptor for the resumed run, exactly as the CLI handler does.
pub async fn session_target(path: &str) -> Result<String> {
    Ok(session::load(path).await?.target)
}

#[cfg(feature = "cli")]
pub async fn resume_cli(args: ResumeArgs, config: &EggsecConfig) -> Result<()> {
    let report = resume(&args.session, config).await?;
    println!("{}", report);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::OutputFormat;

    fn sample_report() -> PipelineReport {
        PipelineReport {
            target: "example.com".into(),
            total_duration_ms: 42,
            stage_results: vec![],
            open_ports: vec![],
            services: vec![],
            endpoints: vec![],
            checkpoint_error: None,
            manifest: None,
            vuln_assessment: None,
            load_test_results: None,
        }
    }

    /// Every declared format must produce a non-empty file.
    ///
    /// `write_output` is the only renderer all eight output formats share, and
    /// it previously had no coverage at all, so a writer that panicked or
    /// returned empty would only surface in production.
    #[tokio::test]
    async fn write_output_renders_every_format() {
        let dir = tempfile::tempdir().expect("temp dir");
        let report = sample_report();
        let formats = [
            OutputFormat::Html,
            OutputFormat::Pretty,
            OutputFormat::Compact,
            OutputFormat::Markdown,
            OutputFormat::Json,
            OutputFormat::Csv,
            OutputFormat::Sarif,
            OutputFormat::Junit,
        ];
        for format in formats {
            let path = dir.path().join(format!("report-{format}.out"));
            write_output(&report, &path.to_string_lossy(), Some(format))
                .await
                .unwrap_or_else(|e| panic!("write_output failed for {format:?}: {e}"));
            let written = tokio::fs::read_to_string(&path)
                .await
                .unwrap_or_else(|e| panic!("{format:?} output unreadable: {e}"));
            assert!(
                !written.trim().is_empty(),
                "{format:?} produced an empty report"
            );
        }
    }

    /// `None` is the documented default (HTML), not an error and not a no-op.
    #[tokio::test]
    async fn write_output_defaults_to_html() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("default.out");
        write_output(&sample_report(), &path.to_string_lossy(), None)
            .await
            .expect("default format writes");
        let written = tokio::fs::read_to_string(&path).await.expect("readable");
        assert!(written.contains("<"), "default output is not HTML");
    }

    /// A report carrying a manifest gets a sibling manifest file.
    #[tokio::test]
    async fn write_output_emits_manifest_sibling() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut report = sample_report();
        let now = chrono::Utc::now();
        report.manifest = Some(crate::output::RunManifest {
            schema_version: "1.0.0".into(),
            run_id: "run-1".into(),
            started_at: now,
            ended_at: now,
            eggsec_version: "0.0.0-test".into(),
            target_scope: "example.com".into(),
            profile: "quick".into(),
            probe_intents: vec![],
            risk_budget: crate::probe::ProbeRisk::SafeActive,
            feature_flags: vec![],
            observations: vec![],
            findings: vec![],
            artifacts: vec![],
            baseline_id: None,
            diff_summary: None,
        });
        let path = dir.path().join("manifested.json");
        write_output(&report, &path.to_string_lossy(), Some(OutputFormat::Json))
            .await
            .expect("write with manifest");
        let sibling = dir.path().join("manifested.manifest.json");
        assert!(
            sibling.exists(),
            "expected sibling manifest at {}",
            sibling.display()
        );
    }

    /// A write failure must surface, not be swallowed.
    #[tokio::test]
    async fn write_output_reports_unwritable_destination() {
        let dir = tempfile::tempdir().expect("temp dir");
        // A path whose parent does not exist cannot be created by `fs::write`.
        let path = dir.path().join("missing-subdir").join("report.json");
        let err = write_output(
            &sample_report(),
            &path.to_string_lossy(),
            Some(OutputFormat::Json),
        )
        .await
        .expect_err("unwritable destination must error");
        tracing::debug!("write_output error surfaced: {err}");
    }
}
