use crate::commands::handlers::CommandContext;
use anyhow::Result;

pub async fn handle_recon(ctx: &CommandContext, args: crate::cli::ReconArgs) -> Result<()> {
    ctx.ensure_scope_url(&args.target)?;
    crate::recon::run_cli(args, &ctx.config)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}
