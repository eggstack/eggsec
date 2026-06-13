use crate::commands::handlers::CommandContext;
use crate::config::OperationDescriptor;
use anyhow::Result;

pub async fn handle_proxy_intercept(
    ctx: &CommandContext,
    args: crate::cli::ProxyInterceptArgs,
) -> Result<()> {
    let is_real = !args.dry_run;
    let risk = if is_real {
        crate::config::OperationRisk::TrafficInterception
    } else {
        crate::config::OperationRisk::SafeActive
    };

    ctx.evaluate_and_enforce_operation(OperationDescriptor {
        operation: "proxy-intercept".to_string(),
        mode: crate::config::OperationMode::DefenseLab,
        risk,
        intended_uses: vec![crate::config::IntendedUse::WebAssessment],
        target: Some(args.listen.clone()),
        required_features: vec!["web-proxy".to_string()],
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: Vec::new(),
    })?;

    // Extra runtime safety gate
    if is_real && !args.allow_web_proxy {
        anyhow::bail!(
            "Traffic interception requires --allow-web-proxy for non-dry-run operations. \
             Use --dry-run for safe validation, or provide --allow-web-proxy --manual-override-reason \"...\" for authorized lab runs."
        );
    }

    let mut report = crate::proxy::intercept::types::WebProxySessionReport::new(&args.listen, args.dry_run);

    if !args.quiet {
        eprintln!(
            "[web-proxy] Standalone defense-lab mode. Use only on lab systems you own and are authorized to intercept.\n\
             Dry-run is always safe and produces a complete report. Real interception requires --allow-web-proxy."
        );
    }

    if args.dry_run {
        // Dry-run: produce complete report with synthetic data, zero network activity
        report.ca_fingerprint = "dry-run-synthetic-fingerprint".to_string();
        report.manifest_matched = true;
        report.actions_performed.push("dry-run-execution".to_string());
        report.budget = crate::proxy::intercept::types::BudgetUsage {
            max_flows: Some(args.max_flows),
            flows_captured: 0,
            max_bytes_per_flow: Some(args.max_bytes_per_flow),
            max_duration_secs: Some(args.max_duration),
            elapsed_secs: 0,
            max_concurrent: Some(args.max_concurrent),
            peak_concurrent: 0,
        };

        // Add synthetic flows for dry-run
        let synthetic_flows = vec![
            crate::proxy::intercept::types::ProxyFlow {
                index: 1,
                method: "GET".to_string(),
                url: "https://httpbin.org/get".to_string(),
                host: "httpbin.org".to_string(),
                path: "/get".to_string(),
                request_headers: {
                    let mut h = std::collections::HashMap::new();
                    h.insert("User-Agent".to_string(), "eggsec-dry-run/1.0".to_string());
                    h
                },
                request_body: None,
                response_status: 200,
                response_headers: {
                    let mut h = std::collections::HashMap::new();
                    h.insert("Content-Type".to_string(), "application/json".to_string());
                    h
                },
                response_body: Some("{ \"ok\": true }".to_string()),
                is_https: true,
                duration_ms: 45,
                request_body_size: 0,
                response_body_size: 18,
                started_at: chrono::Utc::now().to_rfc3339(),
                completed_at: chrono::Utc::now().to_rfc3339(),
                redaction_applied: None,
                protocol: "http1".to_string(),
            },
            crate::proxy::intercept::types::ProxyFlow {
                index: 2,
                method: "POST".to_string(),
                url: "https://httpbin.org/post".to_string(),
                host: "httpbin.org".to_string(),
                path: "/post".to_string(),
                request_headers: {
                    let mut h = std::collections::HashMap::new();
                    h.insert("Content-Type".to_string(), "application/json".to_string());
                    h
                },
                request_body: Some("{\"token\":\"REDACTED_IN_LIVE\"}".to_string()),
                response_status: 200,
                response_headers: std::collections::HashMap::new(),
                response_body: None,
                is_https: true,
                duration_ms: 62,
                request_body_size: 28,
                response_body_size: 0,
                started_at: chrono::Utc::now().to_rfc3339(),
                completed_at: chrono::Utc::now().to_rfc3339(),
                redaction_applied: Some("header".to_string()),
                protocol: "http1".to_string(),
            },
        ];

        for flow in synthetic_flows {
            report.add_flow(flow);
        }

        report.budget.flows_captured = report.flows.len() as u64;
        report.actions_performed.push("synthetic-flows-generated".to_string());
        report.finalize();

        let output = if args.json || ctx.json {
            serde_json::to_string_pretty(&report)?
        } else {
            format_proxy_report(&report)
        };

        if let Some(ref f) = args.output {
            tokio::fs::write(f, &output).await?;
            println!("Report written to: {}", f);
        } else {
            println!("{}", output);
        }

        return Ok(());
    }

    // Real interception path (Phase 1: not yet implemented - will use ProxyServer)
    anyhow::bail!(
        "Real traffic interception is not yet implemented in Phase 1. \
         Use --dry-run to validate the report structure and policy integration."
    )
}

fn format_proxy_report(report: &crate::proxy::intercept::types::WebProxySessionReport) -> String {
    let mut out = String::new();
    out.push_str("═══════════════════════════════════════════\n");
    out.push_str("  Interactive Web Proxy — Session Report\n");
    out.push_str("═══════════════════════════════════════════\n\n");
    out.push_str(&format!("  Listen:       {}\n", report.listen_addr));
    out.push_str(&format!("  CA Fingerprint: {}\n", &report.ca_fingerprint[..std::cmp::min(16, report.ca_fingerprint.len())]));
    out.push_str(&format!("  Dry Run:      {}\n", report.dry_run));
    out.push_str(&format!("  Duration:     {}ms\n", report.duration_ms));
    out.push_str(&format!("  Flows:        {} (HTTPS: {}, HTTP: {}, Blocked: {}, Redacted: {})\n",
        report.flows.len(), report.https_intercepted, report.http_logged, report.blocked, report.redacted));
    out.push_str(&format!("  Budget:       max_flows={} max_bytes_per_flow={} max_duration={}s max_concurrent={}\n",
        report.budget.max_flows.unwrap_or(0),
        report.budget.max_bytes_per_flow.unwrap_or(0),
        report.budget.max_duration_secs.unwrap_or(0),
        report.budget.max_concurrent.unwrap_or(0)));
    if !report.actions_performed.is_empty() {
        out.push_str(&format!("  Actions:      {}\n", report.actions_performed.join("; ")));
    }
    out.push('\n');

    for flow in &report.flows {
        let proto = if flow.is_https { "HTTPS" } else { "HTTP" };
        out.push_str(&format!("  [{}] {} {} {} → {} ({}ms)\n",
            flow.index, proto, flow.method, flow.url, flow.response_status, flow.duration_ms));
        if let Some(ref body) = flow.response_body {
            let preview = if body.len() > 80 { &body[..80] } else { body };
            out.push_str(&format!("    Response: {}...\n", preview));
        }
        if flow.redaction_applied.is_some() {
            out.push_str("    [Redacted]\n");
        }
    }
    out
}
