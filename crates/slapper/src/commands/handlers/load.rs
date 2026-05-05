use crate::commands::handlers::CommandContext;
use anyhow::Result;

pub async fn handle_load(ctx: &CommandContext, args: crate::cli::LoadArgs) -> Result<()> {
    ctx.ensure_scope_url(&args.url)?;
    crate::loadtest::run_cli(args, &ctx.config)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}
