use crate::cli::vuln::{VulnArgs, VulnCommand};
use crate::commands::handlers::CommandContext;
use crate::vuln::{CvssScore, ExploitInfo, Remediation, RiskScore, TriageResult};
use anyhow::Result;

pub async fn handle_vuln(_ctx: &CommandContext, args: VulnArgs) -> Result<()> {
    match args.command {
        VulnCommand::Score(args) => handle_vuln_score(args).await,
        VulnCommand::Exploitability(args) => handle_vuln_exploitability(args).await,
        VulnCommand::Prioritize(args) => handle_vuln_prioritize(args).await,
        VulnCommand::Triage(args) => handle_vuln_triage(args).await,
        VulnCommand::Remediate(args) => handle_vuln_remediate(args).await,
    }
}

async fn handle_vuln_score(args: crate::cli::vuln::VulnScoreArgs) -> Result<()> {
    match CvssScore::from_vector(&args.vector) {
        Ok(score) => {
            println!("CVSS Score for: {}", args.vector);
            println!("  Base Score: {}", score.base_score());
            println!("  Severity: {:?}", score.severity());
            println!("  Temporal Score: {:.2}", score.temporal_score());
        }
        Err(e) => {
            eprintln!("Error parsing CVSS vector: {}", e);
        }
    }
    Ok(())
}

async fn handle_vuln_exploitability(args: crate::cli::vuln::VulnExploitArgs) -> Result<()> {
    println!("Exploitability assessment for: {}", args.cve_id);
    if let Ok(info) = ExploitInfo::for_cve(&args.cve_id) {
        println!("  Exploitability Score: {:.1}", info.exploitability_score());
        println!("  Has Public Exploit: {}", info.has_public_exploit());
        if let Some(edp) = info.exploit_pipeline_score() {
            println!("  EDP Score: {}", edp);
        }
    } else {
        println!("  Note: Exploit data lookup failed");
    }
    Ok(())
}

async fn handle_vuln_prioritize(args: crate::cli::vuln::VulnPrioritizeArgs) -> Result<()> {
    println!("Priority assessment for: {}", args.title);
    println!("  Severity: {}", args.severity);
    if let Some(cvss) = args.cvss {
        println!("  CVSS: {}", cvss);
    }
    if let Some(ac) = args.asset_criticality {
        println!("  Asset Criticality: {}", ac);
    }
    if let Some(exp) = args.exploitability {
        println!("  Exploitability: {}", exp);
    }
    
    let risk = RiskScore::new(
        args.cvss.unwrap_or(0.0),
        args.asset_criticality.unwrap_or(0.0),
        args.exploitability.unwrap_or(0.0),
    );
    println!("  Calculated Risk Score: {:.2}", risk.total());
    println!("  Priority: {:?}", risk.priority());
    Ok(())
}

async fn handle_vuln_triage(args: crate::cli::vuln::VulnTriageArgs) -> Result<()> {
    println!("Triage for: {}", args.title);
    if let Some(id) = &args.id {
        println!("  ID: {}", id);
    }
    println!("  Severity: {}", args.severity);
    if let Some(cvss) = args.cvss {
        println!("  CVSS: {}", cvss);
    }
    let result = TriageResult::new(args.id.clone(), crate::vuln::triage::TriageStatus::New);
    println!("  Triage Status: {:?}", result.status());
    Ok(())
}

async fn handle_vuln_remediate(args: crate::cli::vuln::VulnRemediateArgs) -> Result<()> {
    println!("Remediation guidance for: {}", args.title);
    if let Some(id) = &args.id {
        println!("  ID: {}", id);
    }
    println!("  Severity: {}", args.severity);
    
    let remediation = Remediation::for_finding(
        args.id.as_deref().unwrap_or("default"),
        &args.title,
        args.severity,
    );
    println!("  Priority: {:?}", remediation.priority());
    println!("  Effort: {:?}", remediation.effort());
    if !remediation.steps().is_empty() {
        println!("  Remediation Steps:");
        for (i, step) in remediation.steps().iter().enumerate() {
            println!("    {}. {}", i + 1, step);
        }
    }
    Ok(())
}