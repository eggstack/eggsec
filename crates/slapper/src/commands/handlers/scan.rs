use crate::commands::handlers::CommandContext;
use anyhow::Result;

pub async fn handle_scan_ports(ctx: &CommandContext, args: crate::cli::PortScanArgs) -> Result<()> {
    ctx.ensure_scope(&args.host)?;
    crate::scanner::ports::run_cli(args, &ctx.config)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

pub async fn handle_scan_endpoints(
    ctx: &CommandContext,
    args: crate::cli::EndpointScanArgs,
) -> Result<()> {
    ctx.ensure_scope_url(&args.url)?;
    crate::scanner::endpoints::run_cli(args, &ctx.config)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

pub async fn handle_fingerprint(
    ctx: &CommandContext,
    args: crate::cli::FingerprintArgs,
) -> Result<()> {
    ctx.ensure_scope(&args.host)?;
    crate::scanner::fingerprint::run_cli(args, &ctx.config)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

#[cfg(feature = "nse")]
pub async fn handle_nse(ctx: &CommandContext, args: crate::cli::NseArgs) -> Result<()> {
    ctx.ensure_scope(&args.target)?;
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

pub async fn handle_scan(ctx: &CommandContext, args: crate::cli::ScanArgs) -> Result<()> {
    ctx.ensure_scope(&args.target)?;
    crate::pipeline::run_cli(args, &ctx.config)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

pub async fn handle_resume(args: crate::cli::ResumeArgs) -> Result<()> {
    crate::pipeline::resume_cli(args)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}
