use crate::commands::handlers::CommandContext;
use anyhow::Result;

pub async fn handle_load(ctx: &CommandContext, mut args: crate::cli::LoadArgs) -> Result<()> {
    let target =
        crate::utils::extract_target_from_url(&args.url).unwrap_or_else(|| args.url.clone());
    let descriptor = ctx
        .describe_from_registry("load", Some(target))
        .expect("load should have registry metadata");
    ctx.evaluate_and_enforce_operation(descriptor)?;
    args.json |= ctx.json;
    let target = args.url.clone();
    let scan_id = format!("load-{}", chrono::Utc::now().timestamp());
    ctx.notify_manager
        .notify_scan_started(&scan_id, &target)
        .await;
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
