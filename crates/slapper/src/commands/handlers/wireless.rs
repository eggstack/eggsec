use crate::commands::handlers::CommandContext;
use anyhow::Result;

pub async fn handle_wireless(ctx: &CommandContext, mut args: crate::cli::WirelessArgs) -> Result<()> {
    ctx.ensure_scope(&args.interface)?;
    args.json |= ctx.json;
    let target = args.interface.clone();
    let scan_id = format!("wireless-{}", chrono::Utc::now().timestamp());
    ctx.notify_manager.notify_scan_started(&scan_id, &target).await;
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
