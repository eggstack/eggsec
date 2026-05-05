use crate::commands::handlers::CommandContext;
use anyhow::Result;

pub async fn handle_fuzz(ctx: &CommandContext, args: crate::cli::FuzzArgs) -> Result<()> {
    ctx.ensure_scope_url(&args.url)?;
    crate::fuzzer::run_cli(args)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

pub async fn handle_waf_stress(
    ctx: &CommandContext,
    args: crate::cli::WafStressArgs,
) -> Result<()> {
    ctx.ensure_scope_url(&args.url)?;
    crate::fuzzer::run_waf_stress(args)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

pub async fn handle_waf(ctx: &CommandContext, args: crate::cli::WafArgs) -> Result<()> {
    ctx.ensure_scope_url(&args.url)?;
    crate::waf::run_cli(args)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

pub async fn handle_graphql(ctx: &CommandContext, args: crate::cli::GraphQlArgs) -> Result<()> {
    ctx.ensure_scope_url(&args.url)?;
    crate::commands::run_graphql(args).await
}

pub async fn handle_oauth(ctx: &CommandContext, args: crate::cli::OAuthArgs) -> Result<()> {
    ctx.ensure_scope_url(&args.url)?;
    crate::commands::run_oauth(args).await
}
