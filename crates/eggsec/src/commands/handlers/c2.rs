use crate::commands::handlers::CommandContext;
use crate::config::OperationDescriptor;
use anyhow::Result;

pub async fn handle_c2(
    ctx: &CommandContext,
    args: crate::cli::C2Args,
) -> Result<()> {
    let is_real = !args.dry_run;

    let risk = if is_real {
        crate::config::OperationRisk::C2Operation
    } else {
        crate::config::OperationRisk::SafeActive
    };

    ctx.evaluate_and_enforce_operation(OperationDescriptor {
        operation: "c2".to_string(),
        mode: crate::config::OperationMode::DefenseLab,
        risk,
        intended_uses: vec![crate::config::IntendedUse::WafRegression],
        target: args.target.clone(),
        required_features: vec!["c2".to_string()],
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: Vec::new(),
    })?;

    // Gate real mode behind --allow-c2 (same pattern as db-pentest / wireless active)
    if is_real && !args.allow_c2 {
        anyhow::bail!(
            "Real C2 simulation requires --allow-c2 flag. \
             Use --dry-run for safe validation, or provide --allow-c2 for authorized lab runs."
        );
    }

    if !args.quiet {
        if is_real {
            eprintln!("NOTE: Defense-lab only. Performing real C2 simulation.");
        } else {
            eprintln!("DRY-RUN: planning mode (no real C2 operations performed).");
        }
    }

    let c2_args = crate::cli::C2Args {
        dry_run: args.dry_run,
        json: args.json | ctx.json,
        ..args
    };

    crate::c2::run_cli(c2_args, &ctx.config)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}
