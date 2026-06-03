use crate::commands::handlers::CommandContext;
use anyhow::Result;

pub async fn handle_fuzz(ctx: &CommandContext, mut args: crate::cli::FuzzArgs) -> Result<()> {
    ctx.ensure_scope_url(&args.url)?;
    args.json |= ctx.json;
    let target = args.url.clone();
    let scan_id = format!("fuzz-{}", chrono::Utc::now().timestamp());
    ctx.notify_manager.notify_scan_started(&scan_id, &target).await;
    match crate::fuzzer::run_cli(args)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
    {
        Ok(()) => {
            ctx.notify_manager
                .notify_scan_complete(&scan_id, &target, "Fuzz scan completed", None, None)
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

pub async fn handle_waf_stress(
    ctx: &CommandContext,
    mut args: crate::cli::WafStressArgs,
) -> Result<()> {
    ctx.ensure_scope_url(&args.url)?;
    args.json |= ctx.json;
    crate::fuzzer::run_waf_stress(args)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

pub async fn handle_waf(ctx: &CommandContext, mut args: crate::cli::WafArgs) -> Result<()> {
    ctx.ensure_scope_url(&args.url)?;
    args.json |= ctx.json;
    let target = args.url.clone();
    let scan_id = format!("waf-{}", chrono::Utc::now().timestamp());
    ctx.notify_manager.notify_scan_started(&scan_id, &target).await;
    match crate::waf::run_cli(args)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
    {
        Ok(()) => {
            ctx.notify_manager
                .notify_scan_complete(&scan_id, &target, "WAF scan completed", None, None)
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

pub async fn handle_graphql(ctx: &CommandContext, mut args: crate::cli::GraphQlArgs) -> Result<()> {
    ctx.ensure_scope_url(&args.url)?;
    args.json |= ctx.json;
    crate::commands::run_graphql(args).await
}

pub async fn handle_oauth(ctx: &CommandContext, mut args: crate::cli::OAuthArgs) -> Result<()> {
    ctx.ensure_scope_url(&args.url)?;
    args.json |= ctx.json;
    crate::commands::run_oauth(args).await
}
