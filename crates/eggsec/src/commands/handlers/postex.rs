use crate::commands::handlers::CommandContext;
use crate::config::OperationDescriptor;
use anyhow::Result;

pub async fn handle_postex(
    ctx: &CommandContext,
    args: crate::cli::PostexArgs,
) -> Result<()> {
    ctx.evaluate_and_enforce_operation(OperationDescriptor {
        operation: "postex".to_string(),
        mode: crate::config::OperationMode::DefenseLab,
        risk: if args.dry_run {
            crate::config::OperationRisk::SafeActive
        } else {
            crate::config::OperationRisk::ExploitAdjacent
        },
        intended_uses: vec![crate::config::IntendedUse::WafRegression],
        target: args.target.clone(),
        required_features: vec!["postex".to_string()],
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: Vec::new(),
    })?;

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
