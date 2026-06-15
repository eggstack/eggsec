use crate::commands::handlers::CommandContext;
use crate::config::OperationDescriptor;
use anyhow::Result;

pub async fn handle_c2(
    ctx: &CommandContext,
    args: crate::cli::C2Args,
) -> Result<()> {
    ctx.evaluate_and_enforce_operation(OperationDescriptor {
        operation: "c2".to_string(),
        mode: crate::config::OperationMode::DefenseLab,
        risk: if args.dry_run {
            crate::config::OperationRisk::SafeActive
        } else {
            crate::config::OperationRisk::C2Operation
        },
        intended_uses: vec![crate::config::IntendedUse::WafRegression],
        target: args.target.clone(),
        required_features: vec!["c2".to_string()],
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: Vec::new(),
    })?;

    if !args.dry_run && !args.quiet {
        eprintln!("NOTE: Real C2 simulation requires explicit authorization.");
        eprintln!("Running in dry-run mode by default for safety.");
    }

    let c2_args = crate::cli::C2Args {
        dry_run: true,
        json: args.json | ctx.json,
        ..args
    };

    crate::c2::run_cli(c2_args, &ctx.config)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}
