#![allow(unused_imports)]

pub mod context;
pub mod executor;
pub mod report;
pub mod session;
pub mod stage;

use anyhow::Result;

use crate::cli::ResumeArgs;
use crate::cli::ScanArgs;
use crate::config::SlapperConfig;

pub use context::PipelineContext;
pub use executor::Pipeline;
pub use report::PipelineReport;
pub use stage::{parse_stages, Stage};

pub async fn run_cli(args: ScanArgs, config: &SlapperConfig) -> Result<()> {
    if args.verbose {
        eprintln!("Starting pipeline scan on {}", args.target);
    }
    
    let pipeline = Pipeline::from_args_with_config(args.clone(), config);
    let report = pipeline.run().await?;

    if args.verbose {
        eprintln!("Pipeline complete: {} stages run", report.stage_results.len());
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("{}", report);
    }

    if let Some(output_path) = args.output {
        match args.format {
            Some(crate::cli::OutputFormat::Html) | None => {
                let html = report::generate_html(&report)?;
                std::fs::write(&output_path, html)?;
            }
            Some(crate::cli::OutputFormat::Json) => {
                let json = serde_json::to_string_pretty(&report)?;
                std::fs::write(&output_path, json)?;
            }
            Some(crate::cli::OutputFormat::Csv) => {
                let csv = report::generate_csv(&report)?;
                std::fs::write(&output_path, csv)?;
            }
            Some(crate::cli::OutputFormat::Sarif) => {
                let sarif = crate::output::SarifBuilder::new()
                    .with_report(&report)
                    .build();
                std::fs::write(&output_path, serde_json::to_string_pretty(&sarif)?)?;
            }
            Some(crate::cli::OutputFormat::Junit) => {
                let junit = crate::output::JUnitBuilder::new("slapper")
                    .with_report(&report)
                    .build();
                std::fs::write(&output_path, junit.to_xml()?)?;
            }
        }
        if args.verbose {
            eprintln!("Results written to {}", output_path);
        }
    }

    Ok(())
}

pub async fn resume_cli(args: ResumeArgs) -> Result<()> {
    let session = session::load(&args.session)?;
    let pipeline = Pipeline::from_session(session);
    let report = pipeline.run().await?;
    
    println!("{}", report);
    
    Ok(())
}
