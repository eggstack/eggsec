use crate::cli::VulnArgs;
use crate::commands::handlers::CommandContext;
use crate::types::Severity;
use anyhow::Result;

pub async fn handle_vuln(_ctx: &CommandContext, args: VulnArgs) -> Result<()> {
    use crate::cli::VulnCommand;

    match args.command {
        VulnCommand::Score(score_args) => {
            let score = crate::vuln::CvssScore::from_vector(&score_args.vector)?;
            println!("CVSS Score from vector: {}", score_args.vector);
            println!("  Base Score:       {:.1}", score.base_score);
            println!("  Temporal Score:   {:.1}", score.temporal_score);
            println!("  Environmental:    {:.1}", score.environmental_score);

            let severity = match score.base_score {
                s if s >= 9.0 => "CRITICAL",
                s if s >= 7.0 => "HIGH",
                s if s >= 4.0 => "MEDIUM",
                s if s >= 0.1 => "LOW",
                _ => "NONE",
            };
            println!("  Severity:         {}", severity);
        }
        VulnCommand::Exploitability(exploit_args) => {
            let info = crate::vuln::ExploitInfo::assess(&exploit_args.cve_id);
            println!("Exploitability Assessment for {}", exploit_args.cve_id);
            println!("  Has Public Exploit: {}", info.has_public_exploit);
            println!("  In CISA KEV:        {}", info.in_cisa_kev);
            println!("  Actively Exploited: {}", info.is_actively_exploited);
            println!("  Exploit Score:      {:.1}", info.exploit_score);
            if let Some(ref edb_id) = info.exploit_db_id {
                println!("  Exploit DB ID:      {}", edb_id);
            }
            if let Some(ref msf_module) = info.metasploit_module {
                println!("  Metasploit Module:  {}", msf_module);
            }
        }
        VulnCommand::Prioritize(prioritize_args) => {
            let findings = vec![(
                "finding-1".to_string(),
                prioritize_args.title.clone(),
                prioritize_args.severity,
                prioritize_args.cvss,
            )];

            let prioritized = crate::vuln::prioritize_findings(&findings);
            if let Some(finding) = prioritized.first() {
                println!("Priority Assessment");
                println!("  Finding: {}", prioritize_args.title);
                println!("  Severity: {}", prioritize_args.severity);
                if let Some(cvss) = prioritize_args.cvss {
                    println!("  CVSS Score: {:.1}", cvss);
                }
                println!("  Priority Level: {:?}", finding.risk_score.priority_level);
                println!("  Combined Score: {:.2}", finding.risk_score.combined_score);
                println!("  CVSS Score:     {:.2}", finding.risk_score.cvss_score);
                println!("  Exploitability: {:.2}", finding.risk_score.exploitability_score);
                if let Some(ac) = prioritize_args.asset_criticality {
                    println!("  Asset Criticality: {:.2}", ac);
                } else {
                    println!("  Asset Criticality: {:.2}", finding.risk_score.asset_criticality);
                }
            }
        }
        VulnCommand::Triage(triage_args) => {
            let result = crate::vuln::triage_finding(
                triage_args.id.as_deref().unwrap_or("unknown"),
                &triage_args.title,
                triage_args.description.as_deref().unwrap_or(""),
                triage_args.severity,
                triage_args.cvss,
            );

            println!("Triage Result");
            println!("  Finding ID:  {}", result.finding_id);
            println!("  Status:      {:?}", result.triage_status);
            println!("  Confidence:  {:.0}%", result.confidence * 100.0);
            println!("  Reason:      {}", result.reason);
        }
        VulnCommand::Remediate(remediate_args) => {
            let rem = crate::vuln::Remediation::for_finding(
                remediate_args.id.as_deref().unwrap_or("unknown"),
                &remediate_args.title,
                remediate_args.severity,
            );

            println!("Remediation Guidance");
            println!("  Finding: {}", remediate_args.title);
            println!("  Severity: {}", remediate_args.severity);
            println!("  Priority: {:?}", rem.priority);
            println!("  Estimated Effort: {:.1} hours", rem.effort_hours);
            println!("\nSteps:");
            for (i, step) in rem.steps.iter().enumerate() {
                println!("  {}. {}", i + 1, step);
            }
            if !rem.references.is_empty() {
                println!("\nReferences:");
                for reference in &rem.references {
                    println!("  - {}", reference);
                }
            }
        }
    }

    Ok(())
}