use crate::commands::handlers::CommandContext;
use anyhow::Result;

pub async fn handle_load(ctx: &CommandContext, mut args: crate::cli::LoadArgs) -> Result<()> {
    ctx.ensure_scope_url(&args.url)?;
    args.json |= ctx.json;
    let target = args.url.clone();
    let scan_id = format!("load-{}", chrono::Utc::now().timestamp());
    ctx.notify_manager.notify_scan_started(&scan_id, &target).await;
    match crate::loadtest::run_cli(args, &ctx.config)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
    {
        Ok(()) => {
            ctx.notify_manager
                .notify_scan_complete(&scan_id, &target, "Load test completed", None, None)
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
