use crate::commands::handlers::CommandContext;
use anyhow::Result;

pub async fn handle_fuzz(ctx: &CommandContext, mut args: crate::cli::FuzzArgs) -> Result<()> {
    let target =
        crate::utils::extract_target_from_url(&args.url).unwrap_or_else(|| args.url.clone());
    let descriptor = ctx
        .describe_from_registry("fuzz", Some(target))
        .expect("fuzz should have registry metadata");
    ctx.evaluate_and_enforce_operation(descriptor)?;
    args.json |= ctx.json;
    let target = args.url.clone();
    let scan_id = format!("fuzz-{}", chrono::Utc::now().timestamp());
    ctx.notify_manager
        .notify_scan_started(&scan_id, &target)
        .await;
    match crate::fuzzer::run_cli(args.into())
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
    let target =
        crate::utils::extract_target_from_url(&args.url).unwrap_or_else(|| args.url.clone());
    let descriptor = ctx
        .describe_from_registry("waf-stress", Some(target))
        .expect("waf-stress should have registry metadata");
    ctx.evaluate_and_enforce_operation(descriptor)?;
    args.json |= ctx.json;
    let target = args.url.clone();
    let scan_id = format!("waf-stress-{}", chrono::Utc::now().timestamp());
    ctx.notify_manager
        .notify_scan_started(&scan_id, &target)
        .await;
    match crate::fuzzer::run_waf_stress(args.into())
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
    {
        Ok(()) => {
            ctx.notify_manager
                .notify_scan_complete(&scan_id, &target, "WAF stress test completed", None, None)
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

pub async fn handle_waf(ctx: &CommandContext, mut args: crate::cli::WafArgs) -> Result<()> {
    let target =
        crate::utils::extract_target_from_url(&args.url).unwrap_or_else(|| args.url.clone());
    let descriptor = ctx
        .describe_from_registry("waf", Some(target))
        .expect("waf should have registry metadata");
    ctx.evaluate_and_enforce_operation(descriptor)?;
    args.json |= ctx.json;
    let target = args.url.clone();
    let scan_id = format!("waf-{}", chrono::Utc::now().timestamp());
    ctx.notify_manager
        .notify_scan_started(&scan_id, &target)
        .await;
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
    let target =
        crate::utils::extract_target_from_url(&args.url).unwrap_or_else(|| args.url.clone());
    let descriptor = ctx
        .describe_from_registry("graphql", Some(target))
        .expect("graphql should have registry metadata");
    ctx.evaluate_and_enforce_operation(descriptor)?;
    args.json |= ctx.json;
    let target = args.url.clone();
    let scan_id = format!("graphql-{}", chrono::Utc::now().timestamp());
    ctx.notify_manager
        .notify_scan_started(&scan_id, &target)
        .await;
    match crate::commands::run_graphql(args)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
    {
        Ok(()) => {
            ctx.notify_manager
                .notify_scan_complete(&scan_id, &target, "GraphQL scan completed", None, None)
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

pub async fn handle_oauth(ctx: &CommandContext, mut args: crate::cli::OAuthArgs) -> Result<()> {
    let target =
        crate::utils::extract_target_from_url(&args.url).unwrap_or_else(|| args.url.clone());
    let descriptor = ctx
        .describe_from_registry("oauth", Some(target))
        .expect("oauth should have registry metadata");
    ctx.evaluate_and_enforce_operation(descriptor)?;
    args.json |= ctx.json;
    let target = args.url.clone();
    let scan_id = format!("oauth-{}", chrono::Utc::now().timestamp());
    ctx.notify_manager
        .notify_scan_started(&scan_id, &target)
        .await;
    match crate::commands::run_oauth(args)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
    {
        Ok(()) => {
            ctx.notify_manager
                .notify_scan_complete(&scan_id, &target, "OAuth scan completed", None, None)
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
