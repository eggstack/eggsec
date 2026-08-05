use crate::commands::handlers::CommandContext;
use crate::config::OperationDescriptor;
use anyhow::Result;

pub async fn handle_postex(ctx: &CommandContext, args: crate::cli::PostexArgs) -> Result<()> {
    ctx.evaluate_and_enforce_operation(OperationDescriptor::new(
        "postex".to_string(),
        crate::config::OperationMode::DefenseLab,
        if args.dry_run {
            crate::config::OperationRisk::SafeActive
        } else {
            crate::config::OperationRisk::ExploitAdjacent
        },
        vec![crate::config::IntendedUse::WafRegression],
        args.target.clone(),
        vec!["postex".to_string()],
        Vec::new(),
        false,
        false,
        Vec::new(),
    ))?;

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
