use crate::commands::handlers::CommandContext;
use anyhow::Result;

pub async fn handle_evasion(ctx: &CommandContext, args: crate::cli::EvasionArgs) -> Result<()> {
    let descriptor = ctx
        .describe_from_registry("evasion", args.target.clone())
        .expect("evasion should have registry metadata");
    ctx.evaluate_and_enforce_operation(descriptor)?;

    if !args.dry_run && !args.quiet {
        eprintln!("NOTE: Real evasion detection requires explicit authorization.");
        eprintln!("Running in dry-run mode by default for safety.");
    }

    let evasion_args = crate::cli::EvasionArgs {
        dry_run: true,
        json: args.json | ctx.json,
        ..args
    };

    crate::evasion::run_cli(evasion_args, &ctx.config)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}
