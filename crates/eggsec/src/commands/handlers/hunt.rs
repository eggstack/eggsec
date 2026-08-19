use crate::commands::handlers::CommandContext;
use crate::config::OperationDescriptor;
use anyhow::Result;

pub async fn handle_hunt(ctx: &CommandContext, mut args: crate::cli::HuntArgs) -> Result<()> {
    ctx.evaluate_and_enforce_operation(OperationDescriptor::new(
        "hunt".to_string(),
        crate::config::OperationMode::StandardAssessment,
        crate::config::OperationRisk::Intrusive,
        vec![crate::config::IntendedUse::WebAssessment],
        Some(
            crate::utils::extract_target_from_url(&args.target)
                .unwrap_or_else(|| args.target.clone()),
        ),
        vec!["advanced-hunting".to_string()],
        Vec::new(),
        false,
        false,
        Vec::new(),
    ))?;
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
    writeln!(out, "Vulnerability Hunt Report").expect("writing to String cannot fail");
    writeln!(out, "========================").expect("writing to String cannot fail");
    writeln!(out, "Target: {}", report.target).expect("writing to String cannot fail");
    writeln!(out, "Total findings: {}", report.total_findings)
        .expect("writing to String cannot fail");
    writeln!(out).expect("writing to String cannot fail");

    if !report.attack_chains.is_empty() {
        writeln!(out, "Attack Chains ({}):", report.attack_chains.len())
            .expect("writing to String cannot fail");
        for chain in &report.attack_chains {
            writeln!(
                out,
                "  [{}] {} (CVSS: {:?}) - {} steps",
                chain.severity,
                chain.name,
                chain.cvss_score,
                chain.steps.len()
            )
            .expect("writing to String cannot fail");
            writeln!(out, "    {}", chain.description).expect("writing to String cannot fail");
            for step in &chain.steps {
                writeln!(
                    out,
                    "    Step {}: [{}] {} - {}",
                    step.step_number, step.severity, step.vulnerability, step.impact
                )
                .expect("writing to String cannot fail");
            }
            writeln!(out).expect("writing to String cannot fail");
        }
    }

    if !report.business_logic.is_empty() {
        writeln!(
            out,
            "Business Logic Flaws ({}):",
            report.business_logic.len()
        )
        .expect("writing to String cannot fail");
        for flaw in &report.business_logic {
            writeln!(
                out,
                "  [{}] {:?} - {}",
                flaw.severity, flaw.flaw_type, flaw.description
            )
            .expect("writing to String cannot fail");
            writeln!(out, "    Location: {}", flaw.location)
                .expect("writing to String cannot fail");
            writeln!(out, "    Evidence: {}", flaw.evidence)
                .expect("writing to String cannot fail");
            writeln!(out).expect("writing to String cannot fail");
        }
    }

    if !report.race_conditions.is_empty() {
        writeln!(out, "Race Conditions ({}):", report.race_conditions.len())
            .expect("writing to String cannot fail");
        for race in &report.race_conditions {
            writeln!(
                out,
                "  [{}] {:?} - {}",
                race.severity, race.race_type, race.description
            )
            .expect("writing to String cannot fail");
            writeln!(out, "    Endpoint: {}", race.endpoint)
                .expect("writing to String cannot fail");
            writeln!(out, "    Evidence: {}", race.evidence)
                .expect("writing to String cannot fail");
            writeln!(out).expect("writing to String cannot fail");
        }
    }

    if !report.authz_bypasses.is_empty() {
        writeln!(
            out,
            "Authorization Bypasses ({}):",
            report.authz_bypasses.len()
        )
        .expect("writing to String cannot fail");
        for bypass in &report.authz_bypasses {
            writeln!(
                out,
                "  [{}] {:?} - {}",
                bypass.severity, bypass.bypass_type, bypass.description
            )
            .expect("writing to String cannot fail");
            writeln!(out, "    Endpoint: {}", bypass.endpoint)
                .expect("writing to String cannot fail");
            writeln!(out, "    Evidence: {}", bypass.evidence)
                .expect("writing to String cannot fail");
            writeln!(out).expect("writing to String cannot fail");
        }
    }

    if !report.session_issues.is_empty() {
        writeln!(out, "Session Issues ({}):", report.session_issues.len())
            .expect("writing to String cannot fail");
        for issue in &report.session_issues {
            writeln!(
                out,
                "  [{}] {:?} - {}",
                issue.severity, issue.issue_type, issue.description
            )
            .expect("writing to String cannot fail");
            writeln!(out, "    Evidence: {}", issue.evidence)
                .expect("writing to String cannot fail");
            writeln!(out).expect("writing to String cannot fail");
        }
    }

    out
}
