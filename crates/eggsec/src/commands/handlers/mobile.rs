use crate::commands::handlers::CommandContext;
use crate::config::OperationDescriptor;
use anyhow::Result;

pub async fn handle_mobile(
    ctx: &CommandContext,
    mut args: crate::cli::MobileArgs,
) -> Result<()> {
    // Normalize legacy direct path vs subcommand form for dispatch and policy target.
    // Legacy `eggsec mobile <path>` or `mobile static <path>` -> static path.
    // `mobile dynamic ...` -> dynamic path (feature gated in CLI parser + here).
    let (is_dynamic, static_path, dynamic_target) = match &args.command {
        Some(crate::cli::MobileSubcommand::Static(s)) => (false, Some(s.path.clone()), None),
        #[cfg(feature = "mobile-dynamic")]
        Some(crate::cli::MobileSubcommand::Dynamic(d)) => (true, None, Some(d.target.clone())),
        None => {
            // Legacy direct path form (path required in this case)
            if let Some(ref p) = args.path {
                (false, Some(p.clone()), None)
            } else {
                return Err(anyhow::anyhow!("mobile: provide a path for legacy static or use a subcommand (static|dynamic)"));
            }
        }
    };

    if is_dynamic {
        // Dynamic path: DefenseLab + SafeActive + mobile-dynamic feature + explicit allow flag (like wireless deauth allow)
        #[cfg(feature = "mobile-dynamic")]
        {
            ctx.evaluate_and_enforce_operation(OperationDescriptor {
                operation: "mobile-dynamic".to_string(),
                mode: crate::config::OperationMode::DefenseLab,
                risk: crate::config::OperationRisk::SafeActive,
                intended_uses: vec![crate::config::IntendedUse::WebAssessment],
                target: dynamic_target.clone(),
                required_features: vec!["mobile-dynamic".to_string()],
                required_policy_flags: Vec::new(),
                requires_private_or_local_target: false,
                requires_explicit_scope: false,
                required_capabilities: Vec::new(),
            })?;
            // Extra runtime gate for non-dry (audited; same pattern as wireless deauth)
            // Note: the actual DynamicMobileArgs is inside the subcommand; re-fetch for the check
            if let Some(crate::cli::MobileSubcommand::Dynamic(dargs)) = &args.command {
                if !dargs.dry_run && !dargs.allow_dynamic_mobile {
                    anyhow::bail!(
                        "Dynamic mobile execution requires --allow-dynamic-mobile flag. \
                         Use --dry-run for planning without touching devices."
                    );
                }
            }
            // Merge top-level json flag
            if let Some(crate::cli::MobileSubcommand::Dynamic(dargs)) = &mut args.command {
                dargs.json |= ctx.json;
            }
            let target = dynamic_target.clone().unwrap_or_default();
            let scan_id = format!("mobile-dynamic-{}", chrono::Utc::now().timestamp());
            ctx.notify_manager.notify_scan_started(&scan_id, &target).await;

            // Extract owned DynamicMobileArgs for the call (move out of the enum)
            let dyn_args_cli = match args.command.take() {
                Some(crate::cli::MobileSubcommand::Dynamic(da)) => da,
                _ => unreachable!(),
            };
            // Map CLI (clap) args to the internal dynamic API struct (two distinct types to keep clap concerns out of lib surface).
            let dyn_args = crate::mobile::DynamicMobileArgs {
                target: dyn_args_cli.target,
                device: dyn_args_cli.device,
                install: dyn_args_cli.install,
                launch: dyn_args_cli.launch,
                capture_logs: dyn_args_cli.capture_logs,
                duration: Some(dyn_args_cli.duration),
                uninstall_after: dyn_args_cli.uninstall_after,
                dry_run: dyn_args_cli.dry_run,
                json: dyn_args_cli.json,
                output: dyn_args_cli.output,
                quiet: dyn_args_cli.quiet,
                allow_dynamic_mobile: dyn_args_cli.allow_dynamic_mobile,
                lab_manifest: dyn_args_cli.lab_manifest,
            };
            match crate::mobile::run_dynamic_cli(dyn_args, &ctx.config).await {
                Ok(()) => {
                    ctx.notify_manager
                        .notify_scan_complete(&scan_id, &target, "Mobile dynamic completed", None, None)
                        .await;
                    Ok(())
                }
                Err(e) => {
                    ctx.notify_manager.notify_error(&scan_id, &target, &e.to_string()).await;
                    Err(e.into())
                }
            }
        }
        #[cfg(not(feature = "mobile-dynamic"))]
        {
            anyhow::bail!("mobile-dynamic feature not enabled; rebuild with --features mobile-dynamic")
        }
    } else {
        // Static path (legacy or 'static' sub)
        let spath = static_path.expect("static path resolved");
        ctx.evaluate_and_enforce_operation(OperationDescriptor {
            operation: "mobile-static".to_string(),
            mode: crate::config::OperationMode::StandardAssessment,
            risk: crate::config::OperationRisk::SafeActive,
            intended_uses: vec![crate::config::IntendedUse::WebAssessment],
            target: Some(spath.clone()),
            required_features: vec!["mobile".to_string()],
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        })?;
        // Build a legacy-style MobileArgs for the existing run_cli (or map inside run_cli).
        // We reuse the top-level flags; construct a thin static args view.
        let static_args = crate::cli::MobileArgs {
            path: Some(spath.clone()),
            command: Some(crate::cli::MobileSubcommand::Static(crate::cli::MobileStaticArgs {
                path: spath.clone(),
                json: args.json | ctx.json,
                output: args.output.clone(),
                quiet: args.quiet,
            })),
            json: args.json | ctx.json,
            output: args.output.clone(),
            quiet: args.quiet,
        };
        let target = spath.clone();
        let scan_id = format!("mobile-{}", chrono::Utc::now().timestamp());
        ctx.notify_manager.notify_scan_started(&scan_id, &target).await;
        match crate::mobile::run_cli(static_args, &ctx.config).await {
            Ok(()) => {
                ctx.notify_manager
                    .notify_scan_complete(&scan_id, &target, "Mobile scan completed", None, None)
                    .await;
                Ok(())
            }
            Err(e) => {
                ctx.notify_manager.notify_error(&scan_id, &target, &e.to_string()).await;
                Err(e.into())
            }
        }
    }
}
