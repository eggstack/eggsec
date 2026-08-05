use crate::commands::handlers::CommandContext;
use crate::config::OperationDescriptor;
use anyhow::Result;

pub async fn handle_evasion(ctx: &CommandContext, args: crate::cli::EvasionArgs) -> Result<()> {
    ctx.evaluate_and_enforce_operation(OperationDescriptor::new(
        "evasion".to_string(),
        crate::config::OperationMode::DefenseLab,
        crate::config::OperationRisk::EvasionTesting,
        vec![crate::config::IntendedUse::WafRegression],
        args.target.clone(),
        vec!["evasion".to_string()],
        Vec::new(),
        false,
        false,
        Vec::new(),
    ))?;

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
