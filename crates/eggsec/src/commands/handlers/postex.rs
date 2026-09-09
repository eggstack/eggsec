use crate::commands::handlers::CommandContext;
use anyhow::Result;

pub async fn handle_postex(ctx: &CommandContext, args: crate::cli::PostexArgs) -> Result<()> {
    let descriptor = ctx
        .describe_from_registry("postex", args.target.clone())
        .expect("postex should have registry metadata");
    ctx.evaluate_and_enforce_operation(descriptor)?;

    if !args.dry_run && !args.quiet {
        eprintln!("NOTE: Real post-exploitation simulation requires explicit authorization.");
        eprintln!("Running in dry-run mode by default for safety.");
    }

    let postex_args = crate::cli::PostexArgs {
        dry_run: true,
        json: args.json | ctx.json,
        ..args
    };

    crate::postex::run_cli(postex_args, &ctx.config)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}
