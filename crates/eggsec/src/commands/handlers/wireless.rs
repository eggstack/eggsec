use crate::commands::handlers::CommandContext;
use crate::config::OperationDescriptor;
use anyhow::Result;

pub async fn handle_wireless(
    ctx: &CommandContext,
    mut args: crate::cli::WirelessArgs,
) -> Result<()> {
    ctx.evaluate_and_enforce_operation(OperationDescriptor {
        operation: "wireless".to_string(),
        mode: crate::config::OperationMode::StandardAssessment,
        risk: crate::config::OperationRisk::SafeActive,
        intended_uses: vec![crate::config::IntendedUse::WebAssessment],
        target: Some(args.interface.clone()),
        required_features: vec!["wireless".to_string()],
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
    })?;
    args.json |= ctx.json;
    let target = args.interface.clone();
    let scan_id = format!("wireless-{}", chrono::Utc::now().timestamp());
    ctx.notify_manager
        .notify_scan_started(&scan_id, &target)
        .await;
    match crate::wireless::run_cli(args, &ctx.config).await {
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
