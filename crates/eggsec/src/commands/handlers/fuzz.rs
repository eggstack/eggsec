use crate::commands::handlers::CommandContext;
use crate::config::OperationDescriptor;
use anyhow::Result;

pub async fn handle_fuzz(ctx: &CommandContext, mut args: crate::cli::FuzzArgs) -> Result<()> {
    let target =
        crate::utils::extract_target_from_url(&args.url).unwrap_or_else(|| args.url.clone());
    ctx.evaluate_and_enforce_operation(OperationDescriptor::new(
        "fuzz".to_string(),
        crate::config::OperationMode::StandardAssessment,
        crate::config::OperationRisk::Intrusive,
        vec![crate::config::IntendedUse::WebAssessment],
        Some(target),
        Vec::new(),
        Vec::new(),
        false,
        false,
        Vec::new(),
    ))?;
    args.json |= ctx.json;
    let target = args.url.clone();
    let scan_id = format!("fuzz-{}", chrono::Utc::now().timestamp());
    ctx.notify_manager
        .notify_scan_started(&scan_id, &target)
        .await;
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
    let target =
        crate::utils::extract_target_from_url(&args.url).unwrap_or_else(|| args.url.clone());
    ctx.evaluate_and_enforce_operation(OperationDescriptor::new(
        "waf-stress".to_string(),
        crate::config::OperationMode::StandardAssessment,
        crate::config::OperationRisk::Intrusive,
        vec![crate::config::IntendedUse::WafRegression],
        Some(target),
        Vec::new(),
        Vec::new(),
        false,
        false,
        Vec::new(),
    ))?;
    args.json |= ctx.json;
    let target = args.url.clone();
    let scan_id = format!("waf-stress-{}", chrono::Utc::now().timestamp());
    ctx.notify_manager
        .notify_scan_started(&scan_id, &target)
        .await;
    match crate::fuzzer::run_waf_stress(args)
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
    ctx.evaluate_and_enforce_operation(OperationDescriptor::new(
        "waf-detect".to_string(),
        crate::config::OperationMode::StandardAssessment,
        crate::config::OperationRisk::Intrusive,
        vec![crate::config::IntendedUse::WafRegression],
        Some(target),
        Vec::new(),
        Vec::new(),
        false,
        false,
        Vec::new(),
    ))?;
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
    ctx.evaluate_and_enforce_operation(OperationDescriptor::new(
        "graphql".to_string(),
        crate::config::OperationMode::StandardAssessment,
        crate::config::OperationRisk::Intrusive,
        vec![crate::config::IntendedUse::WebAssessment],
        Some(crate::utils::extract_target_from_url(&args.url).unwrap_or_else(|| args.url.clone())),
        Vec::new(),
        Vec::new(),
        false,
        false,
        Vec::new(),
    ))?;
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
    ctx.evaluate_and_enforce_operation(OperationDescriptor::new(
        "oauth".to_string(),
        crate::config::OperationMode::StandardAssessment,
        crate::config::OperationRisk::Intrusive,
        vec![crate::config::IntendedUse::WebAssessment],
        Some(crate::utils::extract_target_from_url(&args.url).unwrap_or_else(|| args.url.clone())),
        Vec::new(),
        Vec::new(),
        false,
        false,
        Vec::new(),
    ))?;
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
