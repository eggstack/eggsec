use crate::commands::handlers::CommandContext;
use crate::config::OperationDescriptor;
use anyhow::Result;

pub async fn handle_hunt(ctx: &CommandContext, mut args: crate::cli::HuntArgs) -> Result<()> {
    ctx.evaluate_and_enforce_operation(OperationDescriptor {
        operation: "hunt".to_string(),
        mode: crate::config::OperationMode::StandardAssessment,
        risk: crate::config::OperationRisk::Intrusive,
        intended_uses: vec![crate::config::IntendedUse::WebAssessment],
        target: Some(
            crate::utils::extract_target_from_url(&args.target)
                .unwrap_or_else(|| args.target.clone()),
        ),
        required_features: vec!["advanced-hunting".to_string()],
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: Vec::new(),
    })?;
    args.json |= ctx.json;
    let target = args.target.clone();
    let scan_id = format!("hunt-{}", chrono::Utc::now().timestamp());
    ctx.notify_manager
        .notify_scan_started(&scan_id, &target)
        .await;

    let config = crate::hunt::HuntConfig {
        check_attack_chains: !args.skip_chains,
        check_business_logic: !args.skip_business,
        check_race_conditions: !args.skip_race,
        check_authz_bypass: !args.skip_authz,
        check_session: !args.skip_session,
        concurrency: args.concurrency,
        timeout_ms: args.timeout * 1000,
    };

    match crate::hunt::run_hunt(&args.target, config)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
    {
        Ok(report) => {
            let output = match args.format.as_deref() {
                Some("json") | None => serde_json::to_string_pretty(&report)
                    .map_err(|e| anyhow::anyhow!("JSON serialization failed: {}", e))?,
                Some("pretty") => format_hunt_report(&report),
                Some(other) => {
                    anyhow::bail!("Unsupported format: {}. Use json, pretty.", other);
                }
            };

            if let Some(path) = &args.output {
                std::fs::write(path, &output)
                    .map_err(|e| anyhow::anyhow!("Failed to write output to {}: {}", path, e))?;
                println!("Results written to {}", path);
            } else {
                println!("{}", output);
            }

            ctx.notify_manager
                .notify_scan_complete(&scan_id, &target, "Hunt completed", None, None)
                .await;
            Ok(())
        }
        Err(e) => {
            ctx.notify_manager
                .notify_error(&scan_id, &target, &e.to_string())
                .await;
            Err(e)
        }
    }
}

fn format_hunt_report(report: &crate::hunt::HuntReport) -> String {
    use std::fmt::Write;

    let mut out = String::new();
    let _ = writeln!(out, "Vulnerability Hunt Report");
    let _ = writeln!(out, "========================");
    let _ = writeln!(out, "Target: {}", report.target);
    let _ = writeln!(out, "Total findings: {}", report.total_findings);
    let _ = writeln!(out);

    if !report.attack_chains.is_empty() {
        let _ = writeln!(out, "Attack Chains ({}):", report.attack_chains.len());
        for chain in &report.attack_chains {
            let _ = writeln!(
                out,
                "  [{}] {} (CVSS: {:?}) - {} steps",
                chain.severity,
                chain.name,
                chain.cvss_score,
                chain.steps.len()
            );
            let _ = writeln!(out, "    {}", chain.description);
            for step in &chain.steps {
                let _ = writeln!(
                    out,
                    "    Step {}: [{}] {} - {}",
                    step.step_number, step.severity, step.vulnerability, step.impact
                );
            }
            let _ = writeln!(out);
        }
    }

    if !report.business_logic.is_empty() {
        let _ = writeln!(
            out,
            "Business Logic Flaws ({}):",
            report.business_logic.len()
        );
        for flaw in &report.business_logic {
            let _ = writeln!(
                out,
                "  [{}] {:?} - {}",
                flaw.severity, flaw.flaw_type, flaw.description
            );
            let _ = writeln!(out, "    Location: {}", flaw.location);
            let _ = writeln!(out, "    Evidence: {}", flaw.evidence);
            let _ = writeln!(out);
        }
    }

    if !report.race_conditions.is_empty() {
        let _ = writeln!(out, "Race Conditions ({}):", report.race_conditions.len());
        for race in &report.race_conditions {
            let _ = writeln!(
                out,
                "  [{}] {:?} - {}",
                race.severity, race.race_type, race.description
            );
            let _ = writeln!(out, "    Endpoint: {}", race.endpoint);
            let _ = writeln!(out, "    Evidence: {}", race.evidence);
            let _ = writeln!(out);
        }
    }

    if !report.authz_bypasses.is_empty() {
        let _ = writeln!(
            out,
            "Authorization Bypasses ({}):",
            report.authz_bypasses.len()
        );
        for bypass in &report.authz_bypasses {
            let _ = writeln!(
                out,
                "  [{}] {:?} - {}",
                bypass.severity, bypass.bypass_type, bypass.description
            );
            let _ = writeln!(out, "    Endpoint: {}", bypass.endpoint);
            let _ = writeln!(out, "    Evidence: {}", bypass.evidence);
            let _ = writeln!(out);
        }
    }

    if !report.session_issues.is_empty() {
        let _ = writeln!(out, "Session Issues ({}):", report.session_issues.len());
        for issue in &report.session_issues {
            let _ = writeln!(
                out,
                "  [{}] {:?} - {}",
                issue.severity, issue.issue_type, issue.description
            );
            let _ = writeln!(out, "    Evidence: {}", issue.evidence);
            let _ = writeln!(out);
        }
    }

    out
}
