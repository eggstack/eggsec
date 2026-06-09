use crate::cli::ci::CiArgs;
use crate::commands::handlers::CommandContext;
use crate::output::agent::{AgentFinding, FindingSummary};
use crate::output::baseline::BaselineComparison;
use crate::types::Severity;
use anyhow::Result;

#[derive(Debug)]
pub enum CiError {
    NewFindingsDetected,
    FindingsExceedMaximum(usize, usize),
    SeverityThresholdExceeded(usize),
}

impl std::fmt::Display for CiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CiError::NewFindingsDetected => write!(f, "New findings detected"),
            CiError::FindingsExceedMaximum(actual, max) => {
                write!(f, "{} findings exceed maximum of {}", actual, max)
            }
            CiError::SeverityThresholdExceeded(count) => {
                write!(f, "{} findings at or above severity threshold", count)
            }
        }
    }
}

impl std::error::Error for CiError {}

pub async fn handle_ci(_ctx: &CommandContext, args: CiArgs) -> Result<()> {
    let fail_severity = Severity::parse_or_default(&args.fail_on);

    if !args.quiet {
        eprintln!("Running CI checks against target: {:?}", args.target);
        eprintln!("Fail threshold: {}", args.fail_on);
    }

    // Read findings from stdin or generate empty set for demo
    let findings: Vec<AgentFinding> = read_findings()?;

    let summary = FindingSummary::from_findings(&findings);

    // Check baseline if provided
    if let Some(ref baseline_path) = args.baseline {
        let baseline_data = std::fs::read_to_string(baseline_path)?;
        let baseline_findings: Vec<AgentFinding> = serde_json::from_str(&baseline_data)?;
        let comparison = BaselineComparison::compare(&findings, &baseline_findings);

        if !args.quiet {
            eprintln!("Baseline comparison:");
            eprintln!("  New findings: {}", comparison.new_finding_count());
            eprintln!("  Resolved: {}", comparison.resolved_findings.len());
            eprintln!("  Unchanged: {}", comparison.unchanged_findings.len());
        }

        if comparison.has_new_findings() {
            if !args.quiet {
                eprintln!("FAIL: New findings detected");
            }
            return Err(anyhow::anyhow!(CiError::NewFindingsDetected));
        }
    }

    // Check severity threshold
    let findings_above_threshold: Vec<_> = findings
        .iter()
        .filter(|f| f.severity.as_int() >= fail_severity.as_int())
        .collect();

    // Check max findings
    if let Some(max) = args.max_findings {
        if findings.len() > max {
            if !args.quiet {
                eprintln!(
                    "FAIL: {} findings exceed maximum of {}",
                    findings.len(),
                    max
                );
            }
            return Err(anyhow::anyhow!(CiError::FindingsExceedMaximum(
                findings.len(),
                max
            )));
        }
    }

    // Output results
    match args.format.parse::<crate::types::OutputFormat>() {
        Ok(crate::types::OutputFormat::Json) => {
            let output = serde_json::to_string_pretty(&findings)?;
            if let Some(ref output_path) = args.output {
                std::fs::write(output_path, &output)?;
            } else {
                println!("{}", output);
            }
        }
        Ok(crate::types::OutputFormat::Sarif) => {
            let mut builder = crate::output::sarif::SarifBuilder::new();
            for f in &findings {
                let level = match f.severity {
                    Severity::Critical | Severity::High => "error",
                    Severity::Medium => "warning",
                    _ => "note",
                };
                builder = builder.add_result(&f.vulnerability_type, level, &f.title, &f.endpoint);
            }
            let sarif = builder.build();
            let output = serde_json::to_string_pretty(&sarif)?;
            if let Some(ref output_path) = args.output {
                std::fs::write(output_path, &output)?;
            } else {
                println!("{}", output);
            }
        }
        _ => {
            if !args.quiet {
                println!("Total findings: {}", summary.total);
                println!("Risk score: {:.1}", summary.risk_score());
            }
        }
    }

    if !findings_above_threshold.is_empty() {
        if !args.quiet {
            eprintln!(
                "FAIL: {} findings at or above {} severity",
                findings_above_threshold.len(),
                args.fail_on
            );
        }
        return Err(anyhow::anyhow!(CiError::SeverityThresholdExceeded(
            findings_above_threshold.len()
        )));
    }

    if !args.quiet {
        eprintln!("PASS: All checks passed");
    }
    Ok(())
}

fn read_findings() -> Result<Vec<AgentFinding>> {
    use std::io::Read;
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;

    if input.trim().is_empty() {
        return Ok(Vec::new());
    }

    serde_json::from_str(&input).map_err(|e| anyhow::anyhow!("Failed to parse findings: {}", e))
}
