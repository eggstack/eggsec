use crate::commands::handlers::CommandContext;
use crate::config::OperationDescriptor;
use anyhow::Result;

pub async fn handle_recon(ctx: &CommandContext, mut args: crate::cli::ReconArgs) -> Result<()> {
    ctx.evaluate_and_enforce_operation(OperationDescriptor {
        operation: "recon".to_string(),
        mode: crate::config::OperationMode::StandardAssessment,
        risk: crate::config::OperationRisk::SafeActive,
        intended_uses: vec![crate::config::IntendedUse::WebAssessment],
        target: Some(
            crate::utils::extract_target_from_url(&args.target)
                .unwrap_or_else(|| args.target.clone()),
        ),
        required_features: Vec::new(),
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: Vec::new(),
    })?;
    args.json |= ctx.json;
    let target = args.target.clone();
    let scan_id = format!("recon-{}", chrono::Utc::now().timestamp());
    ctx.notify_manager
        .notify_scan_started(&scan_id, &target)
        .await;
    match crate::recon::run_cli(args, &ctx.config)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
    {
        Ok(()) => {
            ctx.notify_manager
                .notify_scan_complete(&scan_id, &target, "Recon completed", None, None)
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
