use crate::commands::handlers::CommandContext;
use crate::config::OperationDescriptor;
use anyhow::Result;

pub async fn handle_evasion(
    ctx: &CommandContext,
    args: crate::cli::EvasionArgs,
) -> Result<()> {
    ctx.evaluate_and_enforce_operation(OperationDescriptor {
        operation: "evasion".to_string(),
        mode: crate::config::OperationMode::DefenseLab,
        risk: crate::config::OperationRisk::EvasionTesting,
        intended_uses: vec![crate::config::IntendedUse::WafRegression],
        target: args.target.clone(),
        required_features: vec!["evasion".to_string()],
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: Vec::new(),
    })?;

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
