use crate::commands::handlers::CommandContext;
use anyhow::Result;

pub async fn handle_recon(ctx: &CommandContext, mut args: crate::cli::ReconArgs) -> Result<()> {
    ctx.ensure_scope_url(&args.target)?;
    args.json |= ctx.json;
    crate::recon::run_cli(args, &ctx.config)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}
