use crate::commands::handlers::CommandContext;
use crate::config::OperationDescriptor;
use anyhow::Result;

pub async fn handle_scan_ports(
    ctx: &CommandContext,
    mut args: crate::cli::PortScanArgs,
) -> Result<()> {
    ctx.ensure_scope(&args.host)?;
    args.json |= ctx.json;
    let target = args.host.clone();
    let scan_id = format!("port-{}", chrono::Utc::now().timestamp());
    ctx.notify_manager
        .notify_scan_started(&scan_id, &target)
        .await;
    match crate::scanner::ports::run_cli(args, &ctx.config)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
    {
        Ok(()) => {
            ctx.notify_manager
                .notify_scan_complete(&scan_id, &target, "Port scan completed", None, None)
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

pub async fn handle_scan_endpoints(
    ctx: &CommandContext,
    mut args: crate::cli::EndpointScanArgs,
) -> Result<()> {
    ctx.ensure_scope_url(&args.url)?;
    args.json |= ctx.json;
    let target = args.url.clone();
    let scan_id = format!("endpoint-{}", chrono::Utc::now().timestamp());
    ctx.notify_manager
        .notify_scan_started(&scan_id, &target)
        .await;
    match crate::scanner::endpoints::run_cli(args, &ctx.config)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
    {
        Ok(()) => {
            ctx.notify_manager
                .notify_scan_complete(&scan_id, &target, "Endpoint scan completed", None, None)
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

pub async fn handle_fingerprint(
    ctx: &CommandContext,
    mut args: crate::cli::FingerprintArgs,
) -> Result<()> {
    ctx.ensure_scope(&args.host)?;
    args.json |= ctx.json;
    let target = args.host.clone();
    let scan_id = format!("fingerprint-{}", chrono::Utc::now().timestamp());
    ctx.notify_manager
        .notify_scan_started(&scan_id, &target)
        .await;
    match crate::scanner::fingerprint::run_cli(args, &ctx.config)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
    {
        Ok(()) => {
            ctx.notify_manager
                .notify_scan_complete(&scan_id, &target, "Fingerprint scan completed", None, None)
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

#[cfg(feature = "nse")]
pub async fn handle_nse(ctx: &CommandContext, mut args: crate::cli::NseArgs) -> Result<()> {
    ctx.evaluate_and_enforce_operation(OperationDescriptor {
        operation: "nse".to_string(),
        mode: crate::config::OperationMode::StandardAssessment,
        risk: crate::config::OperationRisk::Intrusive,
        intended_uses: vec![crate::config::IntendedUse::WebAssessment],
        target: Some(args.target.clone()),
        required_features: vec!["nse".to_string()],
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
    })?;
    args.json |= ctx.json;
    let target = args.target.clone();
    let scan_id = format!("nse-{}", chrono::Utc::now().timestamp());
    ctx.notify_manager
        .notify_scan_started(&scan_id, &target)
        .await;
    let config = eggsec_nse::NseConfig::new(
        &args.target,
        &args.script,
        args.script_args.as_deref(),
        args.script_file.as_deref(),
        args.json,
        args.verbose,
    );
    match eggsec_nse::run_cli(config).await {
        Ok(()) => {
            ctx.notify_manager
                .notify_scan_complete(&scan_id, &target, "NSE scan completed", None, None)
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

pub async fn handle_scan(ctx: &CommandContext, mut args: crate::cli::ScanArgs) -> Result<()> {
    ctx.ensure_scope(&args.target)?;
    args.json |= ctx.json;
    let target = args.target.clone();
    let scan_id = format!("scan-{}", chrono::Utc::now().timestamp());
    ctx.notify_manager
        .notify_scan_started(&scan_id, &target)
        .await;
    match crate::pipeline::run_cli(args, &ctx.config)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
    {
        Ok(()) => {
            ctx.notify_manager
                .notify_scan_complete(&scan_id, &target, "Scan completed", None, None)
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

pub async fn handle_resume(ctx: &CommandContext, args: crate::cli::ResumeArgs) -> Result<()> {
    let session = crate::pipeline::session::load(&args.session)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    ctx.ensure_scope(&session.target)?;
    let target = session.target.clone();
    let scan_id = format!("resume-{}", chrono::Utc::now().timestamp());
    ctx.notify_manager
        .notify_scan_started(&scan_id, &target)
        .await;
    match crate::pipeline::resume_cli(args, &ctx.config)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
    {
        Ok(()) => {
            ctx.notify_manager
                .notify_scan_complete(&scan_id, &target, "Resumed scan completed", None, None)
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
