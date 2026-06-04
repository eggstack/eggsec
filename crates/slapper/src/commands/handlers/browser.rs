use crate::commands::handlers::CommandContext;
use anyhow::Result;

pub async fn handle_browser(ctx: &CommandContext, mut args: crate::cli::BrowserArgs) -> Result<()> {
    ctx.ensure_scope(&args.target)?;
    args.json |= ctx.json;

    let config = crate::browser::BrowserConfig {
        check_dom_xss: !args.no_dom_xss,
        discover_spa_routes: !args.no_spa,
        check_client_security: !args.no_client_checks,
        timeout_ms: args.timeout,
        xss_payload: args.xss_payload
            .unwrap_or_else(|| crate::browser::BrowserConfig::default().xss_payload),
    };

    let target = args.target.clone();
    let scan_id = format!("browser-{}", chrono::Utc::now().timestamp());
    ctx.notify_manager.notify_scan_started(&scan_id, &target).await;

    let report = match tokio::time::timeout(
        std::time::Duration::from_millis(args.timeout + 10000),
        crate::browser::run_browser_scan(&target, config),
    )
    .await
    {
        Ok(Ok(report)) => report,
        Ok(Err(e)) => {
            ctx.notify_manager
                .notify_error(&scan_id, &target, &e.to_string())
                .await;
            return Err(e.into());
        }
        Err(_) => {
            let msg = "Browser scan timed out".to_string();
            ctx.notify_manager
                .notify_error(&scan_id, &target, &msg)
                .await;
            anyhow::bail!(msg);
        }
    };

    if args.json {
        let json = serde_json::to_string_pretty(&report)?;
        if let Some(path) = &args.output {
            std::fs::write(path, &json)?;
            if !args.quiet {
                println!("Results written to {}", path);
            }
        } else {
            println!("{}", json);
        }
    } else {
        if !args.quiet {
            println!("Browser Scan Complete: {}", report.target);
            println!("Total findings: {}", report.total_findings);
            println!();

            if !report.dom_xss.is_empty() {
                println!("DOM XSS Findings ({}):", report.dom_xss.len());
                for finding in &report.dom_xss {
                    println!(
                        "  [{}] {} -> {} at {}",
                        finding.severity, finding.source, finding.sink, finding.location
                    );
                }
                println!();
            }

            if !report.spa_routes.is_empty() {
                println!("SPA Routes Discovered ({}):", report.spa_routes.len());
                for route in &report.spa_routes {
                    println!("  {} (via: {})", route.path, route.discovered_via);
                }
                println!();
            }

            if !report.client_issues.is_empty() {
                println!("Client Issues ({}):", report.client_issues.len());
                for issue in &report.client_issues {
                    println!(
                        "  [{}] {} - {}",
                        issue.severity, issue.issue_type, issue.description
                    );
                }
            }

            if report.total_findings == 0 {
                println!("No issues found.");
            }
        }

        if let Some(path) = &args.output {
            let json = serde_json::to_string_pretty(&report)?;
            std::fs::write(path, &json)?;
            if !args.quiet {
                println!("\nResults written to {}", path);
            }
        }
    }

    ctx.notify_manager
        .notify_scan_complete(
            &scan_id,
            &target,
            &format!("Browser scan completed with {} findings", report.total_findings),
            None,
            None,
        )
        .await;

    Ok(())
}
