use crate::commands::handlers::CommandContext;
use crate::config::OperationDescriptor;
use anyhow::Result;

pub async fn handle_fuzz(ctx: &CommandContext, mut args: crate::cli::FuzzArgs) -> Result<()> {
    let target =
        crate::utils::extract_target_from_url(&args.url).unwrap_or_else(|| args.url.clone());
    ctx.evaluate_and_enforce_operation(OperationDescriptor {
        operation: "fuzz".to_string(),
        mode: crate::config::OperationMode::StandardAssessment,
        risk: crate::config::OperationRisk::Intrusive,
        intended_uses: vec![crate::config::IntendedUse::WebAssessment],
        target: Some(target),
        required_features: Vec::new(),
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
    })?;
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
    ctx.evaluate_and_enforce_operation(OperationDescriptor {
        operation: "waf-stress".to_string(),
        mode: crate::config::OperationMode::StandardAssessment,
        risk: crate::config::OperationRisk::Intrusive,
        intended_uses: vec![crate::config::IntendedUse::WafRegression],
        target: Some(target),
        required_features: Vec::new(),
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
    })?;
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
    ctx.evaluate_and_enforce_operation(OperationDescriptor {
        operation: "waf-detect".to_string(),
        mode: crate::config::OperationMode::StandardAssessment,
        risk: crate::config::OperationRisk::Intrusive,
        intended_uses: vec![crate::config::IntendedUse::WafRegression],
        target: Some(target),
        required_features: Vec::new(),
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
    })?;
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
    ctx.ensure_scope_url(&args.url)?;
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
    ctx.ensure_scope_url(&args.url)?;
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
