use crate::commands::handlers::CommandContext;
use crate::config::OperationDescriptor;
use anyhow::Result;

pub async fn handle_wireless(ctx: &CommandContext, args: crate::cli::WirelessArgs) -> Result<()> {
    match args.command {
        #[cfg(feature = "wireless-advanced")]
        Some(crate::cli::WirelessSubcommand::Deauth(deauth_args)) => {
            handle_deauth(ctx, args.interface, deauth_args).await
        }
        Some(crate::cli::WirelessSubcommand::Scan(scan_args)) => {
            handle_scan(ctx, args.interface, scan_args).await
        }
        None => handle_scan(ctx, args.interface, crate::cli::WirelessScanArgs::default()).await,
        #[cfg(not(feature = "wireless-advanced"))]
        Some(crate::cli::WirelessSubcommand::Deauth(_)) => {
            anyhow::bail!(
                "wireless-advanced feature not enabled; rebuild with --features wireless-advanced"
            )
        }
    }
}

async fn handle_scan(
    ctx: &CommandContext,
    interface: String,
    mut scan_args: crate::cli::WirelessScanArgs,
) -> Result<()> {
    ctx.evaluate_and_enforce_operation(OperationDescriptor {
        operation: "wireless".to_string(),
        mode: crate::config::OperationMode::StandardAssessment,
        risk: crate::config::OperationRisk::SafeActive,
        intended_uses: vec![crate::config::IntendedUse::WebAssessment],
        target: Some(interface.clone()),
        required_features: vec!["wireless".to_string()],
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: Vec::new(),
    })?;
    scan_args.json |= ctx.json;
    let target = interface.clone();
    let scan_id = format!("wireless-{}", chrono::Utc::now().timestamp());
    ctx.notify_manager
        .notify_scan_started(&scan_id, &target)
        .await;

    // Rebuild WirelessArgs for backward compatibility with run_cli
    let wireless_args = crate::cli::WirelessArgs {
        interface,
        command: Some(crate::cli::WirelessSubcommand::Scan(scan_args)),
    };

    match crate::wireless::run_cli(wireless_args, &ctx.config).await {
        Ok(()) => {
            ctx.notify_manager
                .notify_scan_complete(&scan_id, &target, "Wireless scan completed", None, None)
                .await;
            Ok(())
        }
        Err(e) => {
            ctx.notify_manager
                .notify_error(&scan_id, &target, &e.to_string())
                .await;
            Err(e.into())
        }
    }
}

#[cfg(feature = "wireless-advanced")]
async fn handle_deauth(
    ctx: &CommandContext,
    interface: String,
    deauth_args: crate::cli::DeauthArgs,
) -> Result<()> {
    // Policy gate: active wireless is high-risk
    ctx.evaluate_and_enforce_operation(OperationDescriptor {
        operation: "wireless-deauth".to_string(),
        mode: crate::config::OperationMode::DefenseLab,
        risk: crate::config::OperationRisk::Intrusive,
        intended_uses: vec![crate::config::IntendedUse::WebAssessment],
        target: Some(deauth_args.bssid.clone()),
        required_features: vec!["wireless-advanced".to_string()],
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: Vec::new(),
    })?;

    // Additional check: non-dry-run requires explicit --allow-active-wireless
    if !deauth_args.dry_run && !deauth_args.allow_active_wireless {
        anyhow::bail!(
            "Active wireless attack requires --allow-active-wireless flag. \
             Use --dry-run for planning without execution."
        );
    }

    let bssid_bytes = crate::wireless::active::ActiveAttackConfig::parse_mac(&deauth_args.bssid)
        .ok_or_else(|| anyhow::anyhow!("Invalid BSSID format: {}", deauth_args.bssid))?;

    let client_bytes = match &deauth_args.client {
        Some(c) => Some(
            crate::wireless::active::ActiveAttackConfig::parse_mac(c)
                .ok_or_else(|| anyhow::anyhow!("Invalid client MAC format: {}", c))?,
        ),
        None => None,
    };

    let config = crate::wireless::active::ActiveAttackConfig {
        interface: interface.clone(),
        bssid: Some(bssid_bytes),
        client: client_bytes,
        reason_code: deauth_args.reason_code,
        max_frames: deauth_args.max_frames.min(1000), // Enforce hard budget
        frames_per_second: deauth_args.fps.min(100),  // Enforce rate limit
        dry_run: deauth_args.dry_run,
    };

    let target = format!(
        "deauth:{}",
        crate::wireless::active::ActiveAttackConfig::format_mac(&bssid_bytes)
    );
    let scan_id = format!("wireless-deauth-{}", chrono::Utc::now().timestamp());
    ctx.notify_manager
        .notify_scan_started(&scan_id, &target)
        .await;

    let result =
        crate::wireless::active::attacks::deauth::run_deauth(&config, deauth_args.broadcast)
            .await?;

    if deauth_args.json || ctx.json {
        let json = serde_json::to_string_pretty(&result)?;
        if let Some(ref output_path) = deauth_args.output {
            tokio::fs::write(output_path, &json).await?;
            eprintln!("Results written to {}", output_path);
        } else {
            println!("{}", json);
        }
    } else {
        // Human-readable output
        println!("\n=== Active Wireless Attack Result ===");
        println!("Interface:    {}", result.interface);
        println!("Attack type:  {}", result.attack_type);
        println!(
            "Target BSSID: {}",
            result.target_bssid.as_deref().unwrap_or("-")
        );
        println!(
            "Target:       {}",
            result.target_client.as_deref().unwrap_or("-")
        );
        println!("Frames sent:  {}", result.frames_sent);
        println!("Duration:     {}s", result.duration_secs);
        println!("Dry run:      {}", result.dry_run);
        println!();
        for finding in &result.findings {
            println!("[{}] {}", finding.severity, finding.description);
            println!("  Evidence: {}", finding.evidence);
            println!("  Fix:      {}", finding.remediation);
        }
        if !result.recommendations.is_empty() {
            println!("\nRecommendations:");
            for rec in &result.recommendations {
                println!("  - {}", rec);
            }
        }
        if let Some(ref output_path) = deauth_args.output {
            let json = serde_json::to_string_pretty(&result)?;
            tokio::fs::write(output_path, &json).await?;
            eprintln!("\nJSON results written to {}", output_path);
        }
    }

    ctx.notify_manager
        .notify_scan_complete(&scan_id, &target, "Deauth attack completed", None, None)
        .await;

    Ok(())
}
