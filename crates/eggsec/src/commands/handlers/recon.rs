use crate::commands::handlers::CommandContext;
use anyhow::Result;

pub async fn handle_recon(ctx: &CommandContext, mut args: crate::cli::ReconArgs) -> Result<()> {
    let target = Some(
        crate::utils::extract_target_from_url(&args.target).unwrap_or_else(|| args.target.clone()),
    );
    let descriptor = ctx
        .describe_from_registry("recon", target)
        .expect("recon should have registry metadata");
    ctx.evaluate_and_enforce_operation(descriptor)?;
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
