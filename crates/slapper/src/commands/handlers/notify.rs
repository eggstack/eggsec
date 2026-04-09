use anyhow::Result;
use crate::commands::handlers::CommandContext;

pub async fn handle_notify(_ctx: &CommandContext, args: crate::cli::NotifyArgs) -> Result<()> {
    use crate::cli::NotifyCommand;
    use crate::notify::{NotificationPayload, WebhookEvent, FindingSummary, ScanStats};
    use chrono::Utc;

    match &args.command {
        NotifyCommand::Test(test_args) => {
            let test_payload = NotificationPayload {
                event: WebhookEvent::ScanComplete,
                timestamp: Utc::now(),
                scan_id: "test-scan-id".to_string(),
                target: "test.example.com".to_string(),
                message: "This is a test notification from Slapper".to_string(),
                findings: Some(vec![FindingSummary {
                    severity: "info".to_string(),
                    finding_type: "test".to_string(),
                    description: "Test finding".to_string(),
                    location: "/test".to_string(),
                }]),
                stats: Some(ScanStats {
                    duration_ms: 100,
                    requests_total: 10,
                    requests_success: 9,
                    requests_failed: 1,
                    findings_total: 1,
                }),
            };

            let test_config = crate::commands::webhook::WebhookTestConfig {
                slack: test_args.slack.clone(),
                discord: test_args.discord.clone(),
                teams: test_args.teams.clone(),
                webhook: test_args.webhook.clone(),
                secret: test_args.secret.clone(),
            };

            if !crate::commands::webhook::has_any_webhook(&test_config) {
                crate::commands::webhook::print_webhook_usage();
            } else {
                crate::commands::webhook::send_webhook_notifications(&test_config, &test_payload, None).await;
            }
        }
        NotifyCommand::Send(send_args) => {
            let severity = send_args.severity.clone().unwrap_or_else(|| "info".to_string());
            let target = send_args.target.clone().unwrap_or_else(|| "N/A".to_string());

            let payload = NotificationPayload {
                event: if severity == "critical" || severity == "high" {
                    WebhookEvent::FindingDetected
                } else {
                    WebhookEvent::ScanComplete
                },
                timestamp: Utc::now(),
                scan_id: format!("manual-{}", chrono::Utc::now().timestamp()),
                target: target.clone(),
                message: send_args.message.clone(),
                findings: None,
                stats: None,
            };

            let send_config = crate::commands::webhook::WebhookTestConfig {
                slack: send_args.slack.clone(),
                discord: send_args.discord.clone(),
                teams: send_args.teams.clone(),
                webhook: send_args.webhook.clone(),
                secret: None,
            };

            if !crate::commands::webhook::has_any_webhook(&send_config) {
                println!("\nNo webhook URL provided.");
                println!("Configure webhooks in config file or use:");
                println!("  slapper notify send --slack <url> --message 'your message'");
            } else {
                crate::commands::webhook::send_webhook_notifications(&send_config, &payload, None).await;
            }
        }
    }

    Ok(())
}

#[cfg(feature = "rest-api")]
pub async fn handle_serve(_ctx: &CommandContext, args: crate::cli::ServeArgs) -> Result<()> {
    eprintln!("[STUB] REST API server is not yet implemented.");
    eprintln!("  Bind: {}", args.bind);
    eprintln!("  Port: {}", args.port);
    Ok(())
}

#[cfg(feature = "rest-api")]
pub async fn handle_mcp_serve(_ctx: &CommandContext, args: crate::cli::McpServeArgs) -> Result<()> {
    use std::net::SocketAddr;
    use tokio::net::TcpListener;
    use axum::serve;
    use crate::tool::create_default_registry;
    use crate::tool::protocol::mcp::{create_mcp_router, run_stdio};

    let registry = create_default_registry();

    if args.stdio {
        tracing::info!("Starting MCP server in STDIO mode");
        run_stdio(registry, args.api_key).await;
        Ok(())
    } else {
        let router = create_mcp_router(registry, args.api_key.clone()).await;

        let addr: SocketAddr = format!("{}:{}", args.bind, args.port)
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid address {}:{} - {}", args.bind, args.port, e))?;

        tracing::info!("Starting MCP server on {}", addr);

        let listener = TcpListener::bind(addr).await
            .map_err(|e| anyhow::anyhow!("Failed to bind to {}: {}", addr, e))?;

        serve(listener, router)
            .await
            .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))?;

        Ok(())
    }
}
