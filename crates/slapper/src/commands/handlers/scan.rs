use crate::commands::handlers::CommandContext;
use anyhow::Result;

pub async fn handle_scan_ports(
    ctx: &CommandContext,
    mut args: crate::cli::PortScanArgs,
) -> Result<()> {
    ctx.ensure_scope(&args.host)?;
    args.json |= ctx.json;
    crate::scanner::ports::run_cli(args, &ctx.config)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

pub async fn handle_scan_endpoints(
    ctx: &CommandContext,
    mut args: crate::cli::EndpointScanArgs,
) -> Result<()> {
    ctx.ensure_scope_url(&args.url)?;
    args.json |= ctx.json;
    crate::scanner::endpoints::run_cli(args, &ctx.config)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

pub async fn handle_fingerprint(
    ctx: &CommandContext,
    mut args: crate::cli::FingerprintArgs,
) -> Result<()> {
    ctx.ensure_scope(&args.host)?;
    args.json |= ctx.json;
    crate::scanner::fingerprint::run_cli(args, &ctx.config)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

#[cfg(feature = "nse")]
pub async fn handle_nse(ctx: &CommandContext, mut args: crate::cli::NseArgs) -> Result<()> {
    ctx.ensure_scope(&args.target)?;
    args.json |= ctx.json;
    let config = slapper_nse::NseConfig::new(
        &args.target,
        &args.script,
        args.script_args.as_deref(),
        args.script_file.as_deref(),
        args.json,
        args.verbose,
    );
    slapper_nse::run_cli(config).await
}

pub async fn handle_scan(ctx: &CommandContext, mut args: crate::cli::ScanArgs) -> Result<()> {
    ctx.ensure_scope(&args.target)?;
    args.json |= ctx.json;
    crate::pipeline::run_cli(args, &ctx.config)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

pub async fn handle_resume(ctx: &CommandContext, args: crate::cli::ResumeArgs) -> Result<()> {
    let session = crate::pipeline::session::load(&args.session)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    ctx.ensure_scope(&session.target)?;
    crate::pipeline::resume_cli(args)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}
